use std::sync::Mutex;

use aurora_core::LevelState;

use super::*;

fn level(peak: f32, state: LevelState) -> AudioLevel {
    AudioLevel { peak, rms: peak / 3.0, clipped: if peak >= 0.98 { 0.01 } else { 0.0 }, state }
}

/// Una serie di misure ogni 250 ms con lo stesso picco; restituisce l'ultimo volume deciso, applicandolo man mano.
fn run(gain: &mut AutoGain, mut volume: f32, peak: f32, state: LevelState, from_ms: u64, seconds: u64) -> (f32, Vec<f32>) {
    let mut changes = Vec::new();
    for i in 0..seconds * 4 {
        if let Some(v) = gain.on_level(&level(peak, state), volume, from_ms + i * 250, false) {
            volume = v;
            changes.push(v);
        }
    }
    (volume, changes)
}

#[test]
fn falls_at_once_when_the_signal_clips() {
    let mut gain = AutoGain::new();
    let v = gain.on_level(&level(1.0, LevelState::High), 0.9, 0, false).expect("scende");
    assert!((0.45..0.9).contains(&v), "{v}");
}

#[test]
fn keeps_falling_while_it_still_clips_but_waits_a_second_between_steps() {
    let mut gain = AutoGain::new();
    let (v, changes) = run(&mut gain, 1.0, 1.0, LevelState::Overdrive, 0, 4);
    assert!((3..=5).contains(&changes.len()), "{changes:?}");
    assert!(v < 0.65, "{v}");
}

#[test]
fn rises_slowly_after_three_seconds_of_weak_signal() {
    let mut gain = AutoGain::new();
    let (_, none_yet) = run(&mut gain, 0.5, 0.1, LevelState::Good, 0, 2);
    assert!(none_yet.is_empty(), "troppo presto: {none_yet:?}");
    let (v, changes) = run(&mut gain, 0.5, 0.1, LevelState::Good, 2_000, 4);
    assert!(!changes.is_empty());
    assert!(changes[0] <= 0.5 * 1.12 + 1e-6, "al massimo il 12 %: {}", changes[0]);
    assert!(v > 0.5);
}

#[test]
fn never_goes_above_full_volume() {
    let mut gain = AutoGain::new();
    let (v, _) = run(&mut gain, 0.99, 0.05, LevelState::Low, 0, 30);
    assert!(v <= 1.0);
}

#[test]
fn leaves_a_good_level_alone() {
    let mut gain = AutoGain::new();
    let (v, changes) = run(&mut gain, 0.6, 0.6, LevelState::Good, 0, 20);
    assert_eq!(v, 0.6);
    assert!(changes.is_empty());
}

#[test]
fn does_nothing_while_transmitting_or_when_the_input_is_silent() {
    let mut gain = AutoGain::new();
    assert!(gain.on_level(&level(1.0, LevelState::Overdrive), 0.9, 0, true).is_none());
    for i in 0..60 {
        assert!(gain.on_level(&level(0.0, LevelState::Silent), 0.3, i * 250, false).is_none(), "su un ingresso muto non si alza il volume");
    }
}

struct FakeVolume(Mutex<f32>);

impl VolumeControl for FakeVolume {
    fn name(&self) -> String {
        "Ingresso di prova".into()
    }

    fn volume(&self) -> Result<f32, String> {
        Ok(*self.0.lock().unwrap())
    }

    fn set_volume(&self, value: f32) -> Result<(), String> {
        *self.0.lock().unwrap() = value;
        Ok(())
    }
}

fn service(name: &str, volume: f32) -> (Arc<InputLevelService>, Arc<FakeVolume>, PathBuf) {
    let path = std::env::temp_dir().join(format!("aurora-input-level-{}-{name}.json", std::process::id()));
    std::fs::remove_file(&path).ok();
    let fake = Arc::new(FakeVolume(Mutex::new(volume)));
    (InputLevelService::open(fake.clone(), &path), fake, path)
}

#[test]
fn reports_the_device_and_the_volume_in_percent() {
    let (s, _, path) = service("status", 0.964);
    assert_eq!(s.status(), Ok(InputStatus { device: "Ingresso di prova".into(), volume_pct: 96, auto: false }));
    std::fs::remove_file(path).ok();
}

#[test]
fn a_manual_change_sets_the_volume_and_turns_auto_off() {
    let (s, fake, path) = service("manual", 0.5);
    s.set_auto(true);
    s.set_volume_pct(35.0).unwrap();
    assert!((*fake.0.lock().unwrap() - 0.35).abs() < 1e-6);
    assert!(!s.status().unwrap().auto);
    std::fs::remove_file(path).ok();
}

#[test]
fn refuses_a_volume_outside_the_scale() {
    let (s, fake, path) = service("bad", 0.5);
    for bad in [-1.0, 100.5, f64::NAN, f64::INFINITY] {
        assert_eq!(s.set_volume_pct(bad), Err(InputLevelError::BadValue));
    }
    assert_eq!(*fake.0.lock().unwrap(), 0.5);
    std::fs::remove_file(path).ok();
}

#[test]
fn auto_changes_the_volume_only_when_on_and_remembers_the_choice() {
    let (s, fake, path) = service("auto", 0.9);
    let clipping = level(1.0, LevelState::Overdrive);
    assert!(s.on_level(&clipping, 0).is_none(), "spento: non tocca nulla");
    assert_eq!(*fake.0.lock().unwrap(), 0.9);
    s.set_auto(true);
    assert!(s.on_level(&clipping, 0).is_some());
    assert!(*fake.0.lock().unwrap() < 0.9);
    // un nuovo servizio sullo stesso file ritrova la scelta
    let again = InputLevelService::open(fake.clone(), &path);
    assert!(again.status().unwrap().auto);
    std::fs::remove_file(path).ok();
}

#[test]
fn auto_stays_still_while_the_station_transmits() {
    let (s, fake, path) = service("tx", 0.9);
    s.set_auto(true);
    s.set_transmitting(true);
    assert!(s.on_level(&level(1.0, LevelState::Overdrive), 0).is_none());
    assert_eq!(*fake.0.lock().unwrap(), 0.9);
    std::fs::remove_file(path).ok();
}

#[test]
fn the_service_follows_the_device_when_the_input_is_changed() {
    let (service, first, _) = service("follows", 0.4);
    assert_eq!(service.status().expect("stato").volume_pct, 40);
    let second = Arc::new(FakeVolume(Mutex::new(0.8)));
    service.set_control(second.clone());
    assert_eq!(service.status().expect("stato").volume_pct, 80, "ora legge il dispositivo nuovo");
    service.set_volume_pct(30.0).expect("volume");
    assert!((*second.0.lock().unwrap() - 0.3).abs() < 1e-6);
    assert!((*first.0.lock().unwrap() - 0.4).abs() < 1e-6, "il vecchio non si tocca piu'");
}
