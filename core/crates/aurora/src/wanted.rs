//! L'osservatore della lista dei desiderati: guarda i messaggi decodificati e gli spot del cluster, e quando compare una stazione
//! voluta manda l'avviso (evento per la pagina, ntfy per il telefono). Gira nel programma, non nella pagina: l'avviso scatta anche a
//! pagina chiusa.

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aurora_core::{DecodedMessage, Event, WantedAlert};
use dx_cluster::{ClusterService, Spot};
use engine_jtdx::map::band_of;
use tokio::sync::broadcast;
use wishlist::{alert_body, alert_title, send_notification, WishlistService};

const SPOT_POLL: Duration = Duration::from_secs(3);

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn is_grid4(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()
}

fn is_call(s: &str) -> bool {
    (3..=15).contains(&s.len()) && s.chars().any(|c| c.is_ascii_digit()) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
}

/// Chi trasmette un messaggio e, se lo dice, il suo locatore. «CQ DX K1ABC FN42» e «K1ABC G0XYZ IO91» danno chi sta prima del locatore;
/// «CQ K1ABC» da' K1ABC; «K1ABC G0XYZ -12» da' G0XYZ (il secondo nominativo e' chi trasmette). «RR73» ha la forma di un locatore ma non lo e'.
pub fn sender_and_grid(text: &str) -> Option<(String, Option<String>)> {
    let tokens: Vec<String> = text.split_whitespace().map(str::to_uppercase).collect();
    if tokens.len() < 2 {
        return None;
    }
    let last = tokens.last()?;
    if tokens.len() >= 3 && last != "RR73" && is_grid4(last) {
        let call = tokens[tokens.len() - 2].clone();
        return is_call(&call).then(|| (call, Some(last.clone())));
    }
    let call = if matches!(tokens[0].as_str(), "CQ" | "QRZ" | "DE") {
        // dopo CQ puo' esserci una direzione («DX», «NA», «POTA»): niente cifre e corta
        let t1 = &tokens[1];
        if t1.len() <= 4 && !t1.chars().any(|c| c.is_ascii_digit()) { tokens.get(2).cloned()? } else { t1.clone() }
    } else {
        tokens[1].clone()
    };
    is_call(&call).then_some((call, None))
}

/// L'avviso per un messaggio decodificato, se la stazione e' desiderata.
pub fn alert_for_decode(list: &wishlist::Wishlist, d: &DecodedMessage, dial_hz: Option<u64>, now_ms: u64) -> Option<WantedAlert> {
    if !list.from_decodes {
        return None;
    }
    let (call, grid) = sender_and_grid(&d.text)?;
    let band = dial_hz.and_then(band_of);
    let reason = list.reason(&call, grid.as_deref(), d.country.as_deref(), band)?;
    Some(WantedAlert {
        call,
        reason: reason.as_str().to_string(),
        source: "decode".into(),
        text: d.text.trim().to_string(),
        freq_hz: dial_hz.map(|f| f + u64::from(d.freq_hz)),
        band: band.map(str::to_string),
        snr: Some(d.snr),
        country: d.country.clone(),
        grid,
        time_ms: now_ms,
    })
}

/// L'avviso per uno spot del cluster, se il DX e' desiderato.
pub fn alert_for_spot(list: &wishlist::Wishlist, spot: &Spot, now_ms: u64) -> Option<WantedAlert> {
    if !list.from_cluster {
        return None;
    }
    let reason = list.reason(&spot.dx_call, spot.grid.as_deref(), spot.country.as_deref(), spot.band)?;
    Some(WantedAlert {
        call: spot.dx_call.clone(),
        reason: reason.as_str().to_string(),
        source: "cluster".into(),
        text: spot.comment.trim().to_string(),
        freq_hz: Some((spot.freq_khz * 1000.0).round() as u64),
        band: spot.band.map(str::to_string),
        snr: None,
        country: spot.country.clone(),
        grid: spot.grid.clone(),
        time_ms: now_ms,
    })
}

/// Pubblica l'avviso (evento per la pagina), lo ricorda e, se c'e' un indirizzo, lo manda al telefono. Non avvisa due volte per la stessa
/// stazione entro l'attesa scelta.
pub fn announce(service: &Arc<WishlistService>, events: &broadcast::Sender<Event>, alert: WantedAlert) {
    if !service.should_alert(&alert.call, alert.time_ms) {
        return;
    }
    service.remember(alert.clone());
    let url = service.config().notify_url;
    let _ = events.send(Event::Wanted(alert.clone()));
    if !url.is_empty() {
        // la rete puo' essere lenta o assente: fuori dal thread asincrono, e un errore si scrive e basta
        std::thread::spawn(move || {
            if let Err(e) = send_notification(&url, &alert_title(&alert), &alert_body(&alert)) {
                tracing::warn!("avviso al telefono non inviato: {e}");
            }
        });
    }
}

/// Avvia l'osservatore: legge gli eventi del bus e, se c'e', gli spot del cluster.
pub fn spawn_watcher(service: Arc<WishlistService>, events: broadcast::Sender<Event>, cluster: Option<Arc<ClusterService>>) {
    let mut receiver = events.subscribe();
    let decode_service = Arc::clone(&service);
    let decode_events = events.clone();
    tokio::spawn(async move {
        let mut dial_hz: Option<u64> = None;
        loop {
            match receiver.recv().await {
                Ok(Event::Status(s)) => dial_hz = (s.dial_hz > 0).then_some(s.dial_hz),
                Ok(Event::Decode(d)) => {
                    let list = decode_service.config();
                    if let Some(alert) = alert_for_decode(&list, &d, dial_hz, unix_ms()) {
                        announce(&decode_service, &decode_events, alert);
                    }
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(n)) => tracing::warn!("lista dei desiderati: {n} eventi persi"),
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });
    if let Some(cluster) = cluster {
        tokio::spawn(async move {
            // si parte dagli spot gia' presenti: quelli vecchi non fanno scattare nulla
            let mut seen = cluster.spots_since(0, 1).first().map_or(0, |s| s.id);
            loop {
                tokio::time::sleep(SPOT_POLL).await;
                let mut fresh = cluster.spots_since(seen, 500);
                if fresh.is_empty() {
                    continue;
                }
                fresh.sort_by_key(|s| s.id);
                seen = fresh.last().map_or(seen, |s| s.id);
                let list = service.config();
                for spot in &fresh {
                    if let Some(alert) = alert_for_spot(&list, spot, unix_ms()) {
                        announce(&service, &events, alert);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wishlist::Wishlist;

    fn list() -> Wishlist {
        Wishlist { enabled: true, calls: vec!["K1ABC".into()], prefixes: vec!["VP8".into()], countries: vec!["Bulgaria".into()], grids: vec!["JN45".into()], ..Wishlist::default() }.normalized()
    }

    fn decode(text: &str, country: Option<&str>) -> DecodedMessage {
        DecodedMessage { id: 1, time_ms: 0, snr: -12, delta_time: 0.1, freq_hz: 1500, text: text.into(), tag: None, country: country.map(str::to_string), continent: None, hint: false }
    }

    #[test]
    fn reads_who_transmits_from_a_message() {
        assert_eq!(sender_and_grid("CQ K1ABC FN42"), Some(("K1ABC".into(), Some("FN42".into()))));
        assert_eq!(sender_and_grid("CQ DX K1ABC FN42"), Some(("K1ABC".into(), Some("FN42".into()))));
        assert_eq!(sender_and_grid("K1ABC G0XYZ IO91"), Some(("G0XYZ".into(), Some("IO91".into()))));
        assert_eq!(sender_and_grid("CQ K1ABC"), Some(("K1ABC".into(), None)));
        assert_eq!(sender_and_grid("CQ DX K1ABC"), Some(("K1ABC".into(), None)));
        assert_eq!(sender_and_grid("K1ABC G0XYZ -12"), Some(("G0XYZ".into(), None)));
        assert_eq!(sender_and_grid("K1ABC G0XYZ RR73"), Some(("G0XYZ".into(), None)));
        assert_eq!(sender_and_grid("cq vp8lp gd18"), Some(("VP8LP".into(), Some("GD18".into()))));
        assert_eq!(sender_and_grid("TNX 73"), None);
        assert_eq!(sender_and_grid("CQ"), None);
        assert_eq!(sender_and_grid("CQ <...> JN45"), None);
    }

    #[test]
    fn a_wanted_call_in_a_decode_gives_an_alert_with_the_band_and_the_frequency() {
        let a = alert_for_decode(&list(), &decode("CQ K1ABC FN42", Some("United States")), Some(14_074_000), 5).unwrap();
        assert_eq!((a.call.as_str(), a.reason.as_str(), a.source.as_str()), ("K1ABC", "call", "decode"));
        assert_eq!(a.band.as_deref(), Some("20m"));
        assert_eq!(a.freq_hz, Some(14_075_500));
        assert_eq!(a.snr, Some(-12));
    }

    #[test]
    fn country_and_grid_and_prefix_also_alert_and_other_stations_do_not() {
        let w = list();
        assert_eq!(alert_for_decode(&w, &decode("CQ LZ1AB KN22", Some("Bulgaria")), None, 0).unwrap().reason, "country");
        assert_eq!(alert_for_decode(&w, &decode("CQ IU1XYZ JN45", Some("Italy")), None, 0).unwrap().reason, "grid");
        assert_eq!(alert_for_decode(&w, &decode("CQ VP8LP GD18", None), None, 0).unwrap().reason, "prefix");
        assert!(alert_for_decode(&w, &decode("CQ W1AW FN31", Some("United States")), None, 0).is_none());
    }

    #[test]
    fn decodes_and_spots_can_be_switched_off_separately() {
        let mut w = list();
        w.from_decodes = false;
        assert!(alert_for_decode(&w, &decode("CQ K1ABC FN42", None), None, 0).is_none());
        let spot = Spot { id: 1, received_ms: 0, server: "x".into(), spotter: "W3LPL".into(), freq_khz: 14_195.0, dx_call: "K1ABC".into(), comment: "up 5".into(), time_utc: "1200".into(), grid: None, band: Some("20m"), mode: None, country: None, continent: None };
        assert_eq!(alert_for_spot(&w, &spot, 7).unwrap().freq_hz, Some(14_195_000));
        w.from_cluster = false;
        assert!(alert_for_spot(&w, &spot, 7).is_none());
    }

    #[tokio::test]
    async fn the_watcher_announces_a_wanted_station_once_and_ignores_the_others() {
        let dir = std::env::temp_dir().join(format!("aurora-wanted-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let service = WishlistService::open(&dir.join("wishlist.json"));
        service.set_config(list()).unwrap();
        let (events, _) = broadcast::channel::<Event>(64);
        let mut seen = events.subscribe();
        spawn_watcher(Arc::clone(&service), events.clone(), None);
        tokio::time::sleep(Duration::from_millis(50)).await;
        events.send(Event::Decode(decode("CQ W1AW FN31", None))).unwrap();
        events.send(Event::Decode(decode("CQ K1ABC FN42", None))).unwrap();
        events.send(Event::Decode(decode("CQ K1ABC FN42", None))).unwrap();
        let mut alerts = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_millis(600);
        while let Ok(Ok(e)) = tokio::time::timeout_at(deadline, seen.recv()).await {
            if let Event::Wanted(a) = e {
                alerts.push(a);
            }
        }
        assert_eq!(alerts.len(), 1, "una sola volta: la seconda e' dentro l'attesa");
        assert_eq!(alerts[0].call, "K1ABC");
        assert_eq!(service.recent().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
