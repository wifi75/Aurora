//! Registra ogni messaggio decodificato nello storico (`decode-history`) e toglie i piu' vecchi di qualche settimana.

use std::{sync::Arc, time::Duration};

use aurora_core::{DecodedMessage, Event};
use decode_history::{Entry, HistoryStore};
use tokio::sync::broadcast;

use crate::wanted::sender_and_grid;

/// Ogni quanto i messaggi raccolti si scrivono su disco.
const FLUSH: Duration = Duration::from_secs(2);
/// Ogni quanto si tolgono i messaggi troppo vecchi.
const PRUNE: Duration = Duration::from_secs(3600);
const DAY_MS: u64 = 24 * 3600 * 1000;

/// La riga dello storico per un messaggio ricevuto con la radio su `dial_hz` in `mode`.
pub fn entry_for(d: &DecodedMessage, dial_hz: u64, mode: &str, now_ms: u64) -> Entry {
    let tag = serde_json::to_value(d.tag).ok().and_then(|v| v.as_str().map(str::to_string));
    Entry {
        ts_ms: now_ms,
        band: engine_jtdx::map::band_of(dial_hz).map(str::to_string),
        mode: mode.to_string(),
        dial_hz,
        freq_hz: d.freq_hz,
        snr: d.snr,
        dt: d.delta_time,
        text: d.text.clone(),
        call: sender_and_grid(&d.text).map(|(call, _)| call),
        country: d.country.clone(),
        continent: d.continent.clone(),
        tag,
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Raccoglie i messaggi dal bus e li scrive a gruppi; `days` e' quanto a lungo si ricordano (0 = per sempre).
pub fn spawn_recorder(store: Arc<HistoryStore>, events: broadcast::Receiver<Event>, days: u32) {
    spawn_recorder_every(store, events, days, FLUSH);
}

fn spawn_recorder_every(store: Arc<HistoryStore>, mut events: broadcast::Receiver<Event>, days: u32, every: Duration) {
    tokio::spawn(async move {
        let mut dial_hz = 0u64;
        let mut mode = String::new();
        let mut pending: Vec<Entry> = Vec::new();
        let mut flush = tokio::time::interval(every);
        let mut prune = tokio::time::interval(PRUNE);
        loop {
            tokio::select! {
                received = events.recv() => match received {
                    Ok(Event::Status(s)) => {
                        dial_hz = s.dial_hz;
                        mode = s.mode;
                    }
                    Ok(Event::Decode(d)) => pending.push(entry_for(&d, dial_hz, &mode, now_ms())),
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => {
                        write(&store, std::mem::take(&mut pending)).await;
                        return;
                    }
                },
                _ = flush.tick() => write(&store, std::mem::take(&mut pending)).await,
                _ = prune.tick() => {
                    if days > 0 {
                        let store = Arc::clone(&store);
                        let cutoff = now_ms().saturating_sub(u64::from(days) * DAY_MS);
                        let _ = tokio::task::spawn_blocking(move || match store.prune_before(cutoff) {
                            Ok(0) => {}
                            Ok(n) => tracing::info!("storico dei messaggi: tolti {n} messaggi piu' vecchi di {days} giorni"),
                            Err(e) => tracing::warn!("{e}"),
                        })
                        .await;
                    }
                }
            }
        }
    });
}

async fn write(store: &Arc<HistoryStore>, entries: Vec<Entry>) {
    if entries.is_empty() {
        return;
    }
    let store = Arc::clone(store);
    let _ = tokio::task::spawn_blocking(move || {
        if let Err(e) = store.insert_many(&entries) {
            tracing::warn!("{e}");
        }
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_core::{DecodeTag, StationStatus};
    use decode_history::Query;

    fn decode(text: &str) -> DecodedMessage {
        DecodedMessage { id: 1, time_ms: 0, snr: -7, delta_time: 0.3, freq_hz: 1234, text: text.into(), tag: Some(DecodeTag::NewDxcc), country: Some("Japan".into()), continent: Some("AS".into()) }
    }

    #[test]
    fn the_entry_carries_band_mode_sender_and_tag() {
        let e = entry_for(&decode("CQ DX JA1XYZ PM95"), 14_074_000, "FT8", 99);
        assert_eq!((e.band.as_deref(), e.mode.as_str(), e.call.as_deref(), e.tag.as_deref()), (Some("20m"), "FT8", Some("JA1XYZ"), Some("new_dxcc")));
        assert_eq!((e.ts_ms, e.freq_hz, e.snr), (99, 1234, -7));
    }

    #[test]
    fn without_a_known_band_or_sender_the_fields_stay_empty() {
        let mut d = decode("TNX 73");
        d.tag = None;
        let e = entry_for(&d, 0, "", 1);
        assert_eq!((e.band, e.call, e.tag), (None, None, None));
    }

    #[tokio::test]
    async fn records_decodes_with_the_band_and_mode_of_the_moment_and_flushes_them() {
        let store = Arc::new(HistoryStore::open_in_memory().unwrap());
        let (tx, rx) = broadcast::channel(16);
        spawn_recorder_every(Arc::clone(&store), rx, 30, Duration::from_millis(50));
        tx.send(Event::Status(StationStatus { dial_hz: 7_074_000, mode: "FT8".into(), ..StationStatus::default() })).unwrap();
        tx.send(Event::Decode(decode("CQ K1ABC FN42"))).unwrap();
        tokio::time::sleep(Duration::from_millis(400)).await;
        let page = store.query(&Query::default()).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!((page.rows[0].entry.band.as_deref(), page.rows[0].entry.mode.as_str()), (Some("40m"), "FT8"));
    }
}
