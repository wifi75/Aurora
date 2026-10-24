use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

const PSK_BODY: &str = r#"<receptionReports><receptionReport receiverCallsign="K1ABC" receiverLocator="FN42" frequency="14075500" flowStartSeconds="100" mode="FT8" sNR="-9"/></receptionReports>"#;
const SOLAR_BODY: &str = "<solar><solardata><solarflux>120</solarflux></solardata></solar>";

/// Una rete finta: risponde in base all'indirizzo e conta le richieste.
struct Fake {
    psk: Mutex<Result<String, String>>,
    solar: Mutex<Result<String, String>>,
    psk_calls: AtomicUsize,
    solar_calls: AtomicUsize,
    last_url: Mutex<String>,
}

impl Fake {
    fn new() -> Arc<Fake> {
        Arc::new(Fake {
            psk: Mutex::new(Ok(PSK_BODY.into())),
            solar: Mutex::new(Ok(SOLAR_BODY.into())),
            psk_calls: AtomicUsize::new(0),
            solar_calls: AtomicUsize::new(0),
            last_url: Mutex::new(String::new()),
        })
    }
}

impl Fetch for Fake {
    fn get(&self, url: &str) -> Result<String, String> {
        *self.last_url.lock().unwrap() = url.to_string();
        if url.contains("pskreporter") {
            self.psk_calls.fetch_add(1, Ordering::SeqCst);
            self.psk.lock().unwrap().clone()
        } else {
            self.solar_calls.fetch_add(1, Ordering::SeqCst);
            self.solar.lock().unwrap().clone()
        }
    }
}

fn service(name: &str, fake: &Arc<Fake>) -> (Arc<OnlineService>, PathBuf) {
    let path = std::env::temp_dir().join(format!("aurora-online-{}-{name}.json", std::process::id()));
    std::fs::remove_file(&path).ok();
    (OnlineService::open(&path, fake.clone()), path)
}

fn on(psk: bool, solar: bool) -> Settings {
    Settings { psk, solar }
}

#[test]
fn everything_is_off_until_it_is_turned_on_and_nothing_is_asked() {
    let fake = Fake::new();
    let (s, path) = service("off", &fake);
    assert_eq!(s.settings(), on(false, false));
    assert!(!s.refresh_due(1_000_000, "IU3ABC"));
    assert!(!s.refresh_now(1_000_000, "IU3ABC"));
    assert_eq!((fake.psk_calls.load(Ordering::SeqCst), fake.solar_calls.load(Ordering::SeqCst)), (0, 0));
    std::fs::remove_file(path).ok();
}

#[test]
fn the_choice_is_remembered_across_restarts() {
    let fake = Fake::new();
    let (s, path) = service("persist", &fake);
    s.set_settings(on(true, false)).unwrap();
    assert_eq!(OnlineService::open(&path, fake.clone()).settings(), on(true, false));
    std::fs::remove_file(path).ok();
}

#[test]
fn asks_psk_reporter_with_the_callsign_and_keeps_the_reports() {
    let fake = Fake::new();
    let (s, path) = service("psk", &fake);
    s.set_settings(on(true, false)).unwrap();
    assert!(s.refresh_due(1_000_000, "iu3abc"));
    assert!(fake.last_url.lock().unwrap().contains("senderCallsign=IU3ABC"));
    let snap = s.psk();
    assert_eq!((snap.fetched_ms, snap.callsign.as_str(), snap.reports.len(), snap.error.clone()), (1_000_000, "IU3ABC", 1, None));
    assert_eq!(fake.solar_calls.load(Ordering::SeqCst), 0, "il servizio spento non si chiede");
    std::fs::remove_file(path).ok();
}

#[test]
fn does_not_ask_again_before_the_interval_and_asks_again_after() {
    let fake = Fake::new();
    let (s, path) = service("interval", &fake);
    s.set_settings(on(true, true)).unwrap();
    assert!(s.refresh_due(1_000_000, "IU3ABC"));
    assert!(!s.refresh_due(1_000_000 + PSK_EVERY_MS - 1, "IU3ABC"));
    assert!(s.refresh_due(1_000_000 + PSK_EVERY_MS, "IU3ABC"), "PSK scaduto, la propagazione no");
    assert_eq!((fake.psk_calls.load(Ordering::SeqCst), fake.solar_calls.load(Ordering::SeqCst)), (2, 1));
    assert!(s.refresh_due(1_000_000 + SOLAR_EVERY_MS, "IU3ABC"));
    assert_eq!(fake.solar_calls.load(Ordering::SeqCst), 2);
    std::fs::remove_file(path).ok();
}

#[test]
fn a_failure_keeps_the_old_reports_says_why_and_waits_before_retrying() {
    let fake = Fake::new();
    let (s, path) = service("fail", &fake);
    s.set_settings(on(true, false)).unwrap();
    s.refresh_due(1_000_000, "IU3ABC");
    *fake.psk.lock().unwrap() = Err("il servizio ha risposto 503".into());
    assert!(s.refresh_due(1_000_000 + PSK_EVERY_MS, "IU3ABC"));
    let snap = s.psk();
    assert_eq!((snap.reports.len(), snap.error.as_deref()), (1, Some("il servizio ha risposto 503")));
    assert_eq!(snap.fetched_ms, 1_000_000, "l'ultimo scaricato bene resta quello");
    assert!(!s.refresh_due(1_000_000 + PSK_EVERY_MS + 1000, "IU3ABC"), "dopo un errore non si insiste");
    std::fs::remove_file(path).ok();
}

#[test]
fn an_unrecognised_answer_is_an_error_not_an_empty_list() {
    let fake = Fake::new();
    *fake.psk.lock().unwrap() = Ok("<html>429</html>".into());
    *fake.solar.lock().unwrap() = Ok("<html>oops</html>".into());
    let (s, path) = service("garbage", &fake);
    s.set_settings(on(true, true)).unwrap();
    s.refresh_due(1_000_000, "IU3ABC");
    assert!(s.psk().error.is_some() && s.psk().reports.is_empty());
    assert!(s.solar().error.is_some() && s.solar().data.is_none());
    std::fs::remove_file(path).ok();
}

#[test]
fn an_empty_but_valid_answer_is_zero_reports_without_an_error() {
    let fake = Fake::new();
    *fake.psk.lock().unwrap() = Ok("<receptionReports></receptionReports>".into());
    let (s, path) = service("empty", &fake);
    s.set_settings(on(true, false)).unwrap();
    s.refresh_due(1_000_000, "IU3ABC");
    assert_eq!((s.psk().reports.len(), s.psk().error), (0, None));
    std::fs::remove_file(path).ok();
}

#[test]
fn without_a_usable_callsign_psk_is_not_asked() {
    let fake = Fake::new();
    let (s, path) = service("nocall", &fake);
    s.set_settings(on(true, false)).unwrap();
    assert!(!s.refresh_due(1_000_000, ""));
    assert!(!s.refresh_due(1_000_000, "a&b"));
    assert_eq!(fake.psk_calls.load(Ordering::SeqCst), 0);
    std::fs::remove_file(path).ok();
}

#[test]
fn a_changed_callsign_is_looked_up_at_once_but_not_more_than_once_a_minute() {
    let fake = Fake::new();
    let (s, path) = service("changed", &fake);
    s.set_settings(on(true, false)).unwrap();
    s.refresh_due(1_000_000, "IU3ABC");
    assert!(!s.refresh_due(1_000_000 + 30_000, "IU3XYZ"), "troppo presto");
    assert!(s.refresh_due(1_000_000 + MANUAL_MIN_MS, "IU3XYZ"));
    assert_eq!(s.psk().callsign, "IU3XYZ");
    std::fs::remove_file(path).ok();
}

#[test]
fn the_manual_refresh_is_limited_to_once_a_minute() {
    let fake = Fake::new();
    let (s, path) = service("manual", &fake);
    s.set_settings(on(true, true)).unwrap();
    assert!(s.refresh_now(1_000_000, "IU3ABC"));
    assert!(!s.refresh_now(1_000_000 + MANUAL_MIN_MS - 1, "IU3ABC"));
    assert!(s.refresh_now(1_000_000 + MANUAL_MIN_MS, "IU3ABC"));
    assert_eq!((fake.psk_calls.load(Ordering::SeqCst), fake.solar_calls.load(Ordering::SeqCst)), (2, 2));
    std::fs::remove_file(path).ok();
}

#[test]
fn turning_a_service_off_forgets_what_it_had_downloaded() {
    let fake = Fake::new();
    let (s, path) = service("forget", &fake);
    s.set_settings(on(true, true)).unwrap();
    s.refresh_due(1_000_000, "IU3ABC");
    assert!(!s.psk().reports.is_empty() && s.solar().data.is_some());
    s.set_settings(on(false, true)).unwrap();
    assert_eq!(s.psk(), PskSnapshot::default());
    assert!(s.solar().data.is_some());
    std::fs::remove_file(path).ok();
}
