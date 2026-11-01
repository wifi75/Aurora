//! Band Hopping FT8/FT4, come `widgets/mainwindow_bandhopping.cpp` di WSJT-X 3.3 (GPL): ogni due minuti la radio passa alla voce scelta successiva
//! (bande FT8 o FT4 della tabella delle frequenze, oppure fino a 8 frequenze proprie in kHz, in FT8). Il salto non si fa con «Abilita Tx» acceso o in
//! trasmissione (`schedule_blocked`): la voce si salta e si riprova al giro dopo. L'elenco vuoto non fa niente.

use std::{sync::Arc, time::Duration};

use rig_control::Rig;

/// Quante frequenze proprie al massimo (`cbQRG1`..`cbQRG8`).
pub const MAX_CUSTOM: usize = 8;

/// Una voce dell'elenco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HopEntry {
    /// Una banda nel modo dato: la frequenza viene dalla tabella (regione IARU compresa).
    Band { mode: String, band: String },
    /// Una frequenza propria, in Hz (sempre FT8, come `on_actionFT8_triggered` per le QRG di WSJT-X).
    Custom(u64),
}

/// L'elenco delle impostazioni (`BandHopList`): voci separate da virgole, `FT8 20m`, `FT4 40m` o una frequenza in kHz (`14090`). Le voci non valide si scartano.
pub fn parse_list(text: &str) -> Vec<HopEntry> {
    let mut out = Vec::new();
    let mut custom = 0;
    for item in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let words: Vec<&str> = item.split_whitespace().collect();
        match words.as_slice() {
            [mode, band] if matches!(mode.to_uppercase().as_str(), "FT8" | "FT4") && rig_control::bands::canonical_band(band).is_some() => {
                out.push(HopEntry::Band { mode: mode.to_uppercase(), band: band.to_lowercase() });
            }
            [khz] if custom < MAX_CUSTOM => {
                if let Ok(khz) = khz.replace(',', ".").parse::<f64>()
                    && (1_800.0..=450_000.0).contains(&khz)
                {
                    custom += 1;
                    out.push(HopEntry::Custom((khz * 1000.0).round() as u64));
                }
            }
            _ => {}
        }
    }
    out
}

/// La voce successiva da `start` (compreso), girando in tondo; `None` se l'elenco e' vuoto (`next_band_hop_index`).
pub fn next_index(len: usize, start: usize) -> Option<usize> {
    (len > 0).then(|| start % len)
}

/// Il giro delle voci: a ogni minuto pari si passa alla successiva (WSJT-X: `bandHoppingTimer`, un salto ogni due chiamate da un minuto).
#[derive(Debug, Default)]
pub struct Hopper {
    next: usize,
    last_minute: Option<u64>,
}

impl Hopper {
    /// L'ora attuale (secondi UTC): la voce da raggiungere, se e' il momento. Con `blocked` (Tx abilitato o in trasmissione) la voce si salta.
    pub fn tick(&mut self, now_s: u64, len: usize, blocked: bool) -> Option<usize> {
        let minute = now_s / 60;
        if self.last_minute == Some(minute) || !minute.is_multiple_of(2) {
            return None;
        }
        self.last_minute = Some(minute);
        let index = next_index(len, self.next)?;
        self.next = index + 1;
        (!blocked).then_some(index)
    }

    /// Il clic destro sulla barra delle bande: la voce successiva subito, senza aspettare il minuto pari.
    pub fn skip(&mut self, len: usize) -> Option<usize> {
        let index = next_index(len, self.next)?;
        self.next = index + 1;
        Some(index)
    }
}

/// Fa girare il Band Hopping: le impostazioni si rileggono a ogni controllo, cosi' accenderlo o cambiarne l'elenco vale subito.
pub fn spawn(state: remote_api::AppState, rig: Arc<dyn Rig>) {
    tokio::spawn(async move {
        let mut hopper = Hopper::default();
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        loop {
            // il giro ogni 5 secondi, oppure il salto chiesto dal clic destro sulla barra delle bande
            let manual = tokio::select! {
                _ = tick.tick() => false,
                () = remote_api::BAND_HOP_NEXT.notified() => true,
            };
            let Some(settings) = state.engine_settings.as_ref() else { continue };
            let r = settings.reports();
            if !r.band_hopping && !manual {
                continue;
            }
            let entries = parse_list(&r.band_hop_list);
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let blocked = state.current_status().is_some_and(|s| s.tx_enabled || s.transmitting);
            let index = if manual {
                if state.current_status().is_some_and(|s| s.transmitting) {
                    continue;
                }
                hopper.skip(entries.len())
            } else {
                hopper.tick(now, entries.len(), blocked)
            };
            let Some(index) = index else { continue };
            let Some(entry) = entries.get(index).cloned() else { continue };
            hop_to(&state, &rig, &entry).await;
        }
    });
}

async fn hop_to(state: &remote_api::AppState, rig: &Arc<dyn Rig>, entry: &HopEntry) {
    let (mode, target) = match entry {
        HopEntry::Band { mode, band } => {
            let found = {
                let table = state.frequencies.lock().unwrap_or_else(|p| p.into_inner());
                table.frequency_in(state.frequency_region(), band, mode)
            };
            let Some(hz) = found else {
                tracing::info!("Band Hopping: {mode} {band} non e' nella tabella delle frequenze, si salta");
                return;
            };
            (mode.clone(), hz)
        }
        HopEntry::Custom(hz) => ("FT8".to_string(), *hz),
    };
    if state.current_mode() != mode
        && let Err(e) = remote_api::switch_mode(state, &mode).await
    {
        tracing::warn!("Band Hopping: cambio di modo a {mode} non riuscito: {e}");
        return;
    }
    let rig = Arc::clone(rig);
    match tokio::task::spawn_blocking(move || rig.set_frequency(target)).await {
        Ok(Ok(())) => tracing::info!("Band Hopping: radio su {target} Hz in {mode}"),
        other => tracing::warn!("Band Hopping: cambio di frequenza non riuscito: {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_takes_bands_and_up_to_eight_custom_frequencies() {
        let list = parse_list("FT8 20m, ft4 40M, 14090, boh, FT8 99m, 1,2,3");
        assert_eq!(
            list,
            vec![HopEntry::Band { mode: "FT8".into(), band: "20m".into() }, HopEntry::Band { mode: "FT4".into(), band: "40m".into() }, HopEntry::Custom(14_090_000)]
        );
        let many = (0..10).map(|i| (14_000 + i).to_string()).collect::<Vec<_>>().join(",");
        assert_eq!(parse_list(&many).len(), MAX_CUSTOM);
        assert!(parse_list("").is_empty());
    }

    #[test]
    fn it_hops_every_other_minute_in_turn_and_skips_while_transmitting() {
        let mut h = Hopper::default();
        let t0 = 1_800_000_000 / 120 * 120; // un minuto pari
        assert_eq!(h.tick(t0, 3, false), Some(0));
        assert_eq!(h.tick(t0 + 30, 3, false), None, "una sola volta nel minuto");
        assert_eq!(h.tick(t0 + 60, 3, false), None, "i minuti dispari si salta");
        // in trasmissione la voce 1 si salta
        assert_eq!(h.tick(t0 + 120, 3, true), None);
        assert_eq!(h.tick(t0 + 240, 3, false), Some(2));
        assert_eq!(h.tick(t0 + 360, 3, false), Some(0), "si ricomincia dal primo");
        assert_eq!(h.tick(t0 + 480, 0, false), None, "elenco vuoto");
    }

    #[test]
    fn the_right_click_skips_to_the_next_entry_at_once() {
        let mut h = Hopper::default();
        assert_eq!(h.skip(3), Some(0));
        assert_eq!(h.skip(3), Some(1));
        let t0 = 1_800_000_000 / 120 * 120;
        assert_eq!(h.tick(t0, 3, false), Some(2), "il giro automatico continua dalla voce dopo");
        assert_eq!(h.skip(3), Some(0));
        assert_eq!(h.skip(0), None);
    }
}
