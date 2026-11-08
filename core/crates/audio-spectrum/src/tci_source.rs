//! L'ingresso «TCI (dalla radio)»: l'audio ricevuto arriva dalla SDR sul collegamento TCI invece che da una scheda audio.
//!
//! La radio TCI (in `rig-control`) chiama il ricevitore restituito da [`run_tci_feed`] con blocchi mono alla frequenza del server
//! (di base 48 kHz, quella scritta nell'intestazione di ogni blocco). Qui i blocchi passano a un thread che fa le stesse cose di
//! [`crate::run_capture_with`]: righe della cascata, livello d'ingresso e copia al decodificatore. Il decodificatore riceve i blocchi
//! alla frequenza del server con l'ora di arrivo, come da una scheda audio: e' lui a portarli a 12 kHz (filtro e decimazione di
//! `engine-native/src/resample.rs`), cosi' ricezione da scheda e da TCI seguono la stessa strada.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use aurora_core::{Event, SpectrumRow};
use tokio::sync::broadcast;

use crate::{AudioChunk, LevelMeter, SpectrumAnalyzer, F_MAX_HZ};

/// Il nome dell'ingresso nella scheda Audio.
pub const TCI_INPUT_NAME: &str = "TCI (dalla radio)";

/// Riceve l'audio della radio TCI: campioni mono e la loro frequenza di campionamento.
pub type TciRxSink = Arc<dyn Fn(&[f32], u32) + Send + Sync>;

/// L'ingresso TCI in corso; si ferma con `stop` o quando viene lasciato cadere.
pub struct TciFeedHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl TciFeedHandle {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for TciFeedHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Prepara l'ingresso TCI: ritorna il controllo e il ricevitore da dare alla radio (`CatService::set_tci_rx_audio`).
pub fn run_tci_feed(rows: broadcast::Sender<SpectrumRow>, events: broadcast::Sender<Event>, tap: Option<mpsc::Sender<AudioChunk>>) -> (TciFeedHandle, TciRxSink) {
    let (chunks_tx, chunks_rx) = mpsc::channel::<(Vec<f32>, u32, u64)>();
    // il ricevitore e' chiamato dal thread del collegamento TCI: solo una copia e un invio, niente calcoli
    let sender = Mutex::new(chunks_tx);
    let sink: TciRxSink = Arc::new(move |samples: &[f32], rate: u32| {
        if samples.is_empty() || rate == 0 {
            return;
        }
        if let Ok(tx) = sender.lock() {
            let _ = tx.send((samples.to_vec(), rate, now_ms()));
        }
    });
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let thread = std::thread::Builder::new()
        .name("tci-audio".into())
        .spawn(move || {
            let mut current: Option<(u32, SpectrumAnalyzer, LevelMeter)> = None;
            let mut seq = 0u64;
            while !stop_thread.load(Ordering::SeqCst) {
                match chunks_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok((samples, rate, received_unix_ms)) => {
                        // la frequenza del server puo' cambiare (`audio_samplerate`): analizzatore e livello si rifanno
                        if current.as_ref().is_none_or(|(r, _, _)| *r != rate) {
                            match SpectrumAnalyzer::new(rate) {
                                Ok(analyzer) => {
                                    tracing::info!("cascata dall'audio TCI a {rate} Hz");
                                    current = Some((rate, analyzer, LevelMeter::new(rate)));
                                }
                                Err(e) => {
                                    tracing::warn!("audio TCI a {rate} Hz non utilizzabile: {e}");
                                    current = None;
                                    continue;
                                }
                            }
                        }
                        let Some((_, analyzer, meter)) = current.as_mut() else { continue };
                        for level in meter.push(&samples) {
                            let _ = events.send(Event::AudioLevel(level));
                        }
                        for bins in analyzer.push(&samples) {
                            seq += 1;
                            let _ = rows.send(SpectrumRow { seq, f_min_hz: 0, f_max_hz: F_MAX_HZ, bins });
                        }
                        if let Some(tap) = &tap {
                            let _ = tap.send(AudioChunk { rate, samples, received_unix_ms });
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        })
        .ok();
    (TciFeedHandle { stop, thread }, sink)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tci_blocks_reach_the_decoder_and_the_waterfall() {
        let (rows, mut rows_rx) = broadcast::channel(64);
        let (events, _) = broadcast::channel(64);
        let (tap_tx, tap_rx) = mpsc::channel();
        let (mut handle, sink) = run_tci_feed(rows, events, Some(tap_tx));
        // due secondi di tono a 1500 Hz (la FFT della cascata e' di 1,365 s) in blocchi da 2048 campioni a 48 kHz, come li manda ExpertSDR
        let rate = 48_000;
        let tone: Vec<f32> = (0..2 * rate).map(|i| (2.0 * std::f32::consts::PI * 1500.0 * i as f32 / rate as f32).sin() * 0.5).collect();
        for block in tone.chunks(2048) {
            sink(block, rate);
        }
        sink(&[], rate);
        let mut total = 0;
        while total < tone.len() {
            let chunk = tap_rx.recv_timeout(Duration::from_secs(5)).expect("blocco al decodificatore");
            assert_eq!(chunk.rate, rate, "il decodificatore riceve la frequenza del server e la porta lui a 12 kHz");
            assert!(chunk.received_unix_ms > 0);
            assert_eq!(chunk.samples[..], tone[total..total + chunk.samples.len()]);
            total += chunk.samples.len();
        }
        assert_eq!(total, tone.len(), "nessun campione perso o aggiunto");
        // le righe si mandano prima della copia al decodificatore: ora ci sono tutte; vale l'ultima, con la finestra piena di tono
        let mut last = None;
        while let Ok(row) = rows_rx.try_recv() {
            last = Some(row);
        }
        let row = last.expect("righe della cascata");
        let peak = row.bins.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).unwrap();
        let hz = peak as f32 * F_MAX_HZ as f32 / row.bins.len() as f32;
        assert!((hz - 1500.0).abs() < 40.0, "picco a {hz} Hz");
        handle.stop();
    }
}
