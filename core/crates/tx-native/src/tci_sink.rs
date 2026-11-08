//! L'uscita «TCI (alla radio)»: l'onda da trasmettere va alla SDR sul collegamento TCI invece che a una scheda audio.
//!
//! Con TCI e' il server a chiedere l'audio: manda un «chrono» con la lunghezza del blocco che vuole e la radio TCI (in `rig-control`)
//! chiama la sorgente restituita da [`TciSink::new`] con un buffer di quella lunghezza. La sorgente lo riempie con l'onda dal punto
//! in cui era rimasta (silenzio quando non c'e' niente da trasmettere), con il guadagno del cursore Pwr applicato al momento:
//! cosi' il blocco segue sempre il ritmo della SDR e l'onda esce intera, un chrono alla volta.

use std::{
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
        let warned = AtomicU32::new(0);
        let source: TciTxSource = Arc::new(move |buf: &mut [f32], rate: u32| {
            source_shared.last_pull_ms.store(now_ms(), Ordering::SeqCst);
            if rate != TCI_RATE && warned.swap(rate, Ordering::SeqCst) != rate {
                tracing::warn!("il server TCI chiede l'audio a {rate} Hz invece di {TCI_RATE}: il tono uscirebbe spostato");
            }
            let mut playback = lock(&source_shared.playback);
            if !playback.is_playing() {
                return 0;
            }
            playback.fill(buf);
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
