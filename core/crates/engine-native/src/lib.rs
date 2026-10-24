//! Il motore proprio di Aurora, parte di ricezione. Prende l'audio dell'ingresso (lo stesso della cascata), lo porta a 12 kHz, lo divide nei
//! periodi FT8 di 15 secondi e lo decodifica con il crate `ft8`. Per ora lavora «in ombra» accanto a JTDX: i messaggi si scrivono in
//! `native-decodes.log` (nella cartella dei dati) e si confrontano con quelli di JTDX (`GET /api/v1/engine/native`), senza toccare l'elenco
//! che vede l'utente. Quando il confronto e' abbastanza buono il motore proprio prendera' il posto di JTDX.

mod parity;
mod resample;
mod slots;

use std::{
    io::Write,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use audio_spectrum::AudioChunk;
use aurora_core::Event;
use ft8::{decode::Options, message::CallHashes};
use serde_json::{json, Value};
use tokio::sync::broadcast;

pub use parity::{compare, Parity, ParitySummary, SlotSummary};

/// Campioni al secondo del decoder.
pub const RATE: usize = ft8::SAMPLE_RATE;

const DAY_MS: u64 = 86_400_000;

#[derive(Default)]
struct Stats {
    last_slot_unix_ms: u64,
    last_count: usize,
    last_millis: u64,
    slots_decoded: u64,
}

/// Il motore proprio in funzione: il confronto con JTDX e le ultime misure.
pub struct NativeEngine {
    parity: Arc<Parity>,
    stats: Arc<Mutex<Stats>>,
}

impl NativeEngine {
    /// Stato e confronto, in forma di JSON per l'API.
    pub fn status(&self) -> Value {
        let stats = self.stats.lock().map(|s| (s.last_slot_unix_ms, s.last_count, s.last_millis, s.slots_decoded)).unwrap_or_default();
        json!({
            "enabled": true,
            "mode": "shadow",
            "slots_decoded": stats.3,
            "last_slot_unix_ms": stats.0,
            "last_slot_messages": stats.1,
            "last_decode_ms": stats.2,
            "comparison": serde_json::to_value(self.parity.summary()).unwrap_or(Value::Null),
        })
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Il periodo cui appartiene un messaggio di JTDX: `time_ms` sono i millisecondi dalla mezzanotte UTC.
pub fn slot_of_time_ms(time_ms: u32, now_unix_ms: u64) -> u64 {
    let day_start = now_unix_ms - now_unix_ms % DAY_MS;
    let mut slot = day_start + u64::from(time_ms) / slots::SLOT_MS * slots::SLOT_MS;
    if slot > now_unix_ms + 5_000 {
        slot -= DAY_MS;
    }
    slot
}

fn format_line(slot_unix_ms: u64, d: &ft8::decode::Decoded) -> String {
    let secs = slot_unix_ms / 1000 % 86_400;
    format!("{:02}{:02}{:02} {:>4} {:>5.1} {:>4} ~  {}\n", secs / 3600, secs / 60 % 60, secs % 60, d.snr_db, d.dt_s, d.freq_hz.round() as i64, d.text)
}

/// Avvia il motore proprio: `tap` porta l'audio della scheda, `events` e' il bus da cui si leggono i messaggi di JTDX per il confronto.
/// Va chiamato dentro il runtime di Tokio.
pub fn start(events: broadcast::Sender<Event>, tap: mpsc::Receiver<AudioChunk>, data_dir: PathBuf) -> Arc<NativeEngine> {
    let engine = Arc::new(NativeEngine { parity: Arc::new(Parity::default()), stats: Arc::new(Mutex::new(Stats::default())) });

    // i messaggi di JTDX, per il confronto
    let parity = Arc::clone(&engine.parity);
    let mut rx = events.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::Decode(m)) => parity.add_jtdx(slot_of_time_ms(m.time_ms, now_unix_ms()), &m.text, i64::from(m.freq_hz)),
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // l'audio: resampling, periodi, decodifica
    let (parity, stats) = (Arc::clone(&engine.parity), Arc::clone(&engine.stats));
    std::thread::spawn(move || {
        let log_path = data_dir.join("native-decodes.log");
        let comparison_path = data_dir.join("native-parity.log");
        let mut resampler: Option<(u32, resample::Resampler)> = None;
        let mut assembler = slots::SlotAssembler::default();
        let mut hashes = CallHashes::default();
        let options = Options::default();
        while let Ok(chunk) = tap.recv() {
            let rs = match &mut resampler {
                Some((rate, rs)) if *rate == chunk.rate => rs,
                slot => {
                    // la scheda ha cambiato frequenza: si riparte da capo anche con i periodi
                    assembler = slots::SlotAssembler::default();
                    &mut slot.insert((chunk.rate, resample::Resampler::new(chunk.rate, RATE as u32))).1
                }
            };
            let samples = rs.push(&chunk.samples);
            for slot in assembler.push(&samples, chunk.received_unix_ms) {
                let started = Instant::now();
                let decoded = ft8::decode::decode_slot(&slot.samples, &options, &mut hashes);
                let millis = started.elapsed().as_millis() as u64;
                tracing::info!("motore proprio: {} messaggi nel periodo {} in {} ms", decoded.len(), slot.start_unix_ms, millis);
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                    for d in &decoded {
                        let _ = file.write_all(format_line(slot.start_unix_ms, d).as_bytes());
                    }
                }
                parity.add_native(slot.start_unix_ms, decoded.iter().map(|d| (d.text.clone(), d.freq_hz.round() as i64)).collect());
                if let Ok(mut s) = stats.lock() {
                    *s = Stats { last_slot_unix_ms: slot.start_unix_ms, last_count: decoded.len(), last_millis: millis, slots_decoded: s.slots_decoded + 1 };
                }
            }
            for summary in parity.finalize(now_unix_ms()) {
                tracing::info!(
                    "confronto con JTDX, periodo {}: JTDX {}, proprio {}, in comune {}",
                    summary.slot_unix_ms,
                    summary.jtdx,
                    summary.native,
                    summary.both
                );
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&comparison_path) {
                    let _ = writeln!(
                        file,
                        "{} jtdx={} proprio={} comuni={} solo_jtdx={:?} solo_proprio={:?}",
                        summary.slot_unix_ms, summary.jtdx, summary.native, summary.both, summary.only_jtdx, summary.only_native
                    );
                }
            }
        }
    });
    engine
}

#[cfg(test)]
mod tests {
    use super::*;
    use ft8::{modem, SAMPLE_RATE};

    #[test]
    fn jtdx_time_maps_to_the_slot_of_today() {
        // 12:00:30 UTC del giorno 20000 dal 1970
        let now = 20_000 * DAY_MS + 12 * 3_600_000 + 30_000 + 4_000;
        let slot = slot_of_time_ms(12 * 3_600_000 + 30_000, now);
        assert_eq!(slot, 20_000 * DAY_MS + 12 * 3_600_000 + 30_000);
        // appena dopo mezzanotte, un messaggio delle 23:59:45 e' di ieri
        let after_midnight = 20_001 * DAY_MS + 10_000;
        assert_eq!(slot_of_time_ms(23 * 3_600_000 + 59 * 60_000 + 45_000, after_midnight), 20_001 * DAY_MS - 15_000);
    }

    /// Un periodo FT8 vero (messaggio sintetico dentro rumore) passato dalla scheda finta fino ai risultati.
    #[test]
    fn audio_in_decoded_messages_out() {
        let (tx, rx) = mpsc::channel();
        let (events, _keep) = broadcast::channel(16);
        let dir = std::env::temp_dir().join(format!("aurora-native-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let engine = runtime.block_on(async { start(events, rx, dir.clone()) });

        // 12 kHz, due periodi interi a partire da un multiplo di 15 s: il messaggio sta nel secondo
        let t0 = 1_000_005_000u64 - 15_000;
        let mut wave = modem::encode_message("CQ K1ABC FN42", 1500.0, &mut CallHashes::default()).expect("onda");
        let mut audio = vec![0.0f32; 3 * SAMPLE_RATE * 15];
        let mut seed = 7u64;
        for a in audio.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *a = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1;
        }
        let start = (15_000 + 500 + 300) * SAMPLE_RATE / 1000;
        for (i, w) in wave.drain(..).enumerate() {
            audio[start + i] += 0.02 * w;
        }
        for (n, chunk) in audio.chunks(480).enumerate() {
            let end_ms = t0 + ((n + 1) * 480 * 1000 / SAMPLE_RATE) as u64;
            tx.send(AudioChunk { rate: 12_000, samples: chunk.to_vec(), received_unix_ms: end_ms }).expect("invio");
        }
        drop(tx);
        // il thread finisce quando il canale si chiude
        for _ in 0..200 {
            if engine.status()["slots_decoded"].as_u64().unwrap_or(0) >= 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let status = engine.status();
        assert!(status["slots_decoded"].as_u64().unwrap_or(0) >= 2, "{status}");
        let log = std::fs::read_to_string(dir.join("native-decodes.log")).unwrap_or_default();
        assert!(log.contains("CQ K1ABC FN42"), "{log}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
