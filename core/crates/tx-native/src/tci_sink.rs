//! L'uscita «TCI (alla radio)»: l'onda da trasmettere va alla SDR sul collegamento TCI invece che a una scheda audio.
//!
//! Con TCI e' il server a chiedere l'audio: manda un «chrono» con la lunghezza del blocco che vuole e la radio TCI (in `rig-control`)
//! chiama la sorgente restituita da [`TciSink::new`] con un buffer di quella lunghezza. La sorgente lo riempie con l'onda dal punto
//! in cui era rimasta (silenzio quando non c'e' niente da trasmettere), con il guadagno del cursore Pwr applicato al momento:
//! cosi' il blocco segue sempre il ritmo della SDR e l'onda esce intera, un chrono alla volta.
//!
//! L'onda e' fatta a 48 kHz. Se il server chiede un'altra frequenza (`audio_samplerate`, es. 24, 12 o 8 kHz: lo dice l'intestazione del
//! chrono) la si porta a quella con un filtro sinc a finestra che tiene lo stato da un chrono all'altro: il tono resta dov'e'.

use std::{
    f64::consts::PI,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::{audio::Playback, error::TxError, lock, AudioSink};

/// Il nome dell'uscita nella scheda Audio.
pub const TCI_OUTPUT_NAME: &str = "TCI (alla radio)";
/// La frequenza dell'audio TCI di base (ExpertSDR, Thetis; WSJT-X non la cambia).
pub const TCI_RATE: u32 = 48_000;
/// Discesa a fine trasmissione interrotta, come per la scheda audio.
const STOP_FADE: Duration = Duration::from_millis(5);
/// Con un'onda in corso e nessuna richiesta del server per tanto tempo, il collegamento TCI si considera perso.
const STALL: Duration = Duration::from_secs(3);

/// Da 48 kHz (l'onda) alla frequenza chiesta dal server, a blocchi: filtro sinc a finestra di Hann (passa-basso al 90% della meta'
/// della frequenza piu' bassa), la stessa forma di `engine-native/src/resample.rs`. La posizione resta tra un blocco e l'altro.
struct RateConverter {
    out_rate: u32,
    /// Campioni a 48 kHz per campione in uscita.
    ratio: f64,
    /// Meta' larghezza del filtro, in campioni a 48 kHz.
    half: usize,
    /// Frequenza di taglio in cicli per campione a 48 kHz.
    cutoff: f64,
    /// Indice assoluto (nell'onda a 48 kHz) del primo campione di `buf`.
    base: u64,
    /// Posizione assoluta del prossimo campione da produrre.
    next: f64,
    buf: Vec<f32>,
}

impl RateConverter {
    fn new(out_rate: u32) -> RateConverter {
        let ratio = f64::from(TCI_RATE) / f64::from(out_rate.max(1));
        let cutoff = 0.5 / ratio.max(1.0) * 0.9;
        let half = (16.0 * ratio.max(1.0)).ceil() as usize;
        RateConverter { out_rate, ratio, half, cutoff, base: 0, next: 0.0, buf: Vec::new() }
    }

    /// Riempie `out` alla frequenza del server; `pull` da' i campioni a 48 kHz che servono (l'onda, poi silenzio).
    fn fill(&mut self, out: &mut [f32], mut pull: impl FnMut(&mut [f32])) {
        let half = self.half as f64;
        for o in out.iter_mut() {
            // servono i campioni fino a `next + half`
            let need_end = self.next.floor() as u64 + self.half as u64 + 1;
            let have_end = self.base + self.buf.len() as u64;
            if need_end > have_end {
                let start = self.buf.len();
                self.buf.resize(start + (need_end - have_end) as usize, 0.0);
                pull(&mut self.buf[start..]);
            }
            let centre = self.next;
            let first = (centre.floor() as i64 - self.half as i64 + 1).max(self.base as i64) as u64;
            let last = centre.floor() as u64 + self.half as u64;
            let mut acc = 0.0f64;
            for i in first..=last {
                let t = i as f64 - centre;
                let window = 0.5 * (1.0 + (PI * t / half).cos());
                let x = 2.0 * PI * self.cutoff * t;
                let sinc = if x.abs() < 1e-12 { 1.0 } else { x.sin() / x };
                acc += f64::from(self.buf[(i - self.base) as usize]) * 2.0 * self.cutoff * sinc * window;
            }
            *o = acc as f32;
            self.next += self.ratio;
        }
        // si tengono solo i campioni che servono ancora al filtro
        let keep_from = (self.next.floor() as i64 - self.half as i64).max(self.base as i64) as u64;
        let drop = ((keep_from - self.base) as usize).min(self.buf.len());
        self.buf.drain(..drop);
        self.base += drop as u64;
    }
}

/// Fornisce l'audio da trasmettere: riempie il buffer mono (alla frequenza indicata) e dice quanti campioni ha scritto.
pub type TciTxSource = Arc<dyn Fn(&mut [f32], u32) -> usize + Send + Sync>;

struct Shared {
    playback: Mutex<Playback>,
    gain: Arc<AtomicU32>,
    /// Millisecondi (dal 1970) dell'ultima richiesta del server o dell'inizio dell'onda.
    last_pull_ms: AtomicU64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub struct TciSink {
    shared: Arc<Shared>,
}

impl TciSink {
    /// L'uscita e la sorgente da dare alla radio TCI (`CatService::set_tci_tx_audio`). `gain` e' il cursore Pwr (bit di un `f32`).
    pub fn new(gain: Arc<AtomicU32>) -> (TciSink, TciTxSource) {
        let shared = Arc::new(Shared { playback: Mutex::new(Playback::new()), gain, last_pull_ms: AtomicU64::new(now_ms()) });
        let source_shared = Arc::clone(&shared);
        let converter: Mutex<Option<RateConverter>> = Mutex::new(None);
        let source: TciTxSource = Arc::new(move |buf: &mut [f32], rate: u32| {
            source_shared.last_pull_ms.store(now_ms(), Ordering::SeqCst);
            let mut playback = lock(&source_shared.playback);
            let mut conv = lock(&converter);
            if !playback.is_playing() {
                // la prossima onda riparte con il filtro vuoto
                *conv = None;
                return 0;
            }
            if rate == TCI_RATE || rate == 0 {
                *conv = None;
                playback.fill(buf);
            } else {
                let c = match &mut *conv {
                    Some(c) if c.out_rate == rate => c,
                    slot => {
                        tracing::info!("audio TCI da trasmettere a {rate} Hz: l'onda a {TCI_RATE} Hz viene ricampionata");
                        slot.insert(RateConverter::new(rate))
                    }
                };
                c.fill(buf, |chunk| playback.fill(chunk));
            }
            let g = f32::from_bits(source_shared.gain.load(Ordering::Relaxed));
            for s in buf.iter_mut() {
                *s = (*s * g).clamp(-1.0, 1.0);
            }
            buf.len()
        });
        (TciSink { shared }, source)
    }
}

impl AudioSink for TciSink {
    fn sample_rate(&self) -> u32 {
        TCI_RATE
    }

    fn start(&mut self, samples: Vec<f32>) -> Result<(), TxError> {
        self.shared.last_pull_ms.store(now_ms(), Ordering::SeqCst);
        lock(&self.shared.playback).start(samples);
        Ok(())
    }

    fn is_playing(&self) -> bool {
        lock(&self.shared.playback).is_playing()
    }

    fn stop(&mut self) {
        let n = (STOP_FADE.as_secs_f64() * f64::from(TCI_RATE)).round() as usize;
        lock(&self.shared.playback).begin_stop(n);
    }

    fn live_gain(&self) -> Option<Arc<AtomicU32>> {
        Some(Arc::clone(&self.shared.gain))
    }

    /// Un'onda in corso che il server non chiede piu' (SDR chiusa, TCI scollegato): la trasmissione non deve restare appesa.
    fn has_failed(&self) -> bool {
        self.is_playing() && now_ms().saturating_sub(self.shared.last_pull_ms.load(Ordering::SeqCst)) > STALL.as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wave_leaves_one_chrono_at_a_time() {
        let gain = Arc::new(AtomicU32::new(0.5f32.to_bits()));
        let (mut sink, source) = TciSink::new(gain);
        assert_eq!(sink.sample_rate(), 48_000);
        let mut buf = vec![9.0f32; 4];
        assert_eq!(source(&mut buf, 48_000), 0, "niente da trasmettere: il blocco resta silenzio");
        sink.start((1..=10).map(|v| v as f32 / 10.0).collect()).expect("start");
        assert!(sink.is_playing());
        let mut out = Vec::new();
        for len in [4usize, 3, 4] {
            let mut buf = vec![0.0f32; len];
            assert_eq!(source(&mut buf, 48_000), len);
            out.extend(buf);
        }
        let expected: Vec<f32> = (1..=10).map(|v| v as f32 / 20.0).chain([0.0]).collect();
        assert_eq!(out, expected, "onda continua tra i chrono, con il guadagno, poi silenzio");
        assert!(!sink.is_playing());
    }

    #[test]
    fn a_lower_server_rate_keeps_the_tone_where_it_is() {
        for rate in [24_000u32, 12_000, 8_000] {
            let (mut sink, source) = TciSink::new(Arc::new(AtomicU32::new(1.0f32.to_bits())));
            // un secondo di tono a 1000 Hz fatto a 48 kHz
            sink.start((0..48_000).map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48_000.0).sin() * 0.5).collect()).expect("start");
            let mut out = Vec::new();
            // chrono di 512 campioni alla frequenza del server, fino a 0,9 s
            while out.len() < (rate as usize * 9) / 10 {
                let mut buf = vec![0.0f32; 512];
                assert_eq!(source(&mut buf, rate), 512, "{rate} Hz");
                out.extend(buf);
            }
            let body = &out[rate as usize / 10..(rate as usize * 9) / 10];
            // 1000 Hz = 2000 passaggi per lo zero al secondo: su 0,8 s sono 1600
            let crossings = body.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
            assert!(crossings.abs_diff(1600) <= 4, "{rate} Hz: {crossings} passaggi per lo zero");
            let rms = (body.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / body.len() as f64).sqrt();
            assert!((rms - 0.5 * std::f64::consts::FRAC_1_SQRT_2).abs() < 0.01, "{rate} Hz: rms {rms}");
        }
    }

    #[test]
    fn stop_fades_out_quickly() {
        let (mut sink, source) = TciSink::new(Arc::new(AtomicU32::new(1.0f32.to_bits())));
        sink.start(vec![1.0; 48_000]).expect("start");
        sink.stop();
        let mut buf = vec![0.0f32; 1024];
        source(&mut buf, 48_000);
        assert!(!sink.is_playing(), "5 ms di discesa e poi basta");
        assert!(buf[0] > 0.9 && buf[500] == 0.0);
    }

    #[test]
    fn a_silent_server_is_a_lost_output() {
        let (mut sink, _source) = TciSink::new(Arc::new(AtomicU32::new(1.0f32.to_bits())));
        sink.start(vec![1.0; 10]).expect("start");
        assert!(!sink.has_failed());
        sink.shared.last_pull_ms.store(now_ms() - 10_000, Ordering::SeqCst);
        assert!(sink.has_failed());
    }
}
