//! I file di diagnostica del pacchetto di assistenza.
//!
//! Il pacchetto che parte da Aurora (scheda Assistenza, o dopo un crash) deve bastare per capire un guaio senza altre domande all'utente:
//! oltre al registro (`logs/aurora.log`) porta, sotto `diagnostica/`,
//! - `cat-comandi.log`: i comandi mandati alla radio e le risposte, con l'ora e il tempo di ognuno (l'inizio dell'ultimo collegamento e gli ultimi);
//! - `radio.json`: come e' impostata la radio (modello, porta, velocita', PTT, split...), che stato ha adesso e su quale VFO sta;
//! - `ambiente.json`: il PC, Aurora, l'audio, il motore e le variabili `AURORA_*`;
//! - `interfaccia-*.json`: la finestra, lo schermo, il tema, le preferenze e gli ultimi errori della pagina, come li vede la pagina stessa.
//!
//! Niente dati personali: nominativi, locatori, percorsi con il nome utente, indirizzi e codici li toglie `rapporti::pulisci` sul testo finito;
//! qui non si leggono comunque ne' nominativo ne' locatore, ne' le chiavi dei servizi.

use std::time::Instant;

use serde_json::{Value, json};

/// Il testo di un valore JSON con la forma di un file.
fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

/// Le variabili d'ambiente di Aurora (`AURORA_*`): scelte di avvio che cambiano il comportamento (radio, cartelle, rete).
fn variabili_aurora() -> Value {
    let mut vars: Vec<(String, String)> = std::env::vars().filter(|(k, _)| k.starts_with("AURORA_")).collect();
    vars.sort();
    // i segreti (chiavi, codici, password) non si dicono nemmeno nel nome del valore
    let segreto = |k: &str| {
        let k = k.to_ascii_uppercase();
        ["KEY", "TOKEN", "SECRET", "PASS", "CODE"].iter().any(|s| k.contains(s))
    };
    Value::Object(vars.into_iter().map(|(k, v)| { let valore = if segreto(&k) { "***".to_string() } else { v }; (k, Value::String(valore)) }).collect())
}

/// La versione di Hamlib inclusa (il file `VERSIONE.txt` accanto a `rigctld`), se si trova.
fn versione_hamlib() -> Option<String> {
    let rigctld = rig_control::hamlib::locate_rigctld()?;
    let text = std::fs::read_to_string(rigctld.parent()?.join("VERSIONE.txt")).ok()?;
    let first = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(first.chars().take(120).collect())
}

/// `radio.json`: impostazioni e stato della radio.
pub fn radio(cat: &rig_control::cat::CatService) -> Value {
    let mut config = serde_json::to_value(cat.config()).unwrap_or(Value::Null);
    // le chiavi dei programmi collegati non sono diagnostica
    if let Some(map) = config.as_object_mut() {
        for k in ["cat4om_key"] {
            if map.get(k).and_then(Value::as_str).is_some_and(|v| !v.is_empty()) {
                map.insert(k.into(), Value::String("***".into()));
            }
        }
    }
    let stato = cat.status();
    let mut stato_json = serde_json::to_value(&stato).unwrap_or(Value::Null);
    // il VFO si chiede alla radio adesso (un comando in piu' nella traccia): se Hamlib non lo sa dire, si scrive «non letto»
    let vfo = cat.hamlib().ok().and_then(|h| h.current_vfo());
    if let Some(map) = stato_json.as_object_mut() {
        map.insert("vfo_secondo_hamlib".into(), vfo.map_or(Value::String("non letto".into()), Value::String));
    }
    json!({
        "nota": "Impostazioni (configurazione) e stato della radio nel momento in cui si e' preparato il pacchetto. «vfo_secondo_hamlib» e' il VFO che Hamlib dice di usare: con le Icom piu' vecchie se lo ricorda da solo, non lo legge dalla radio.",
        "hamlib": versione_hamlib(),
        "configurazione": config,
        "stato": stato_json,
    })
}

/// `ambiente.json`: il PC, Aurora, l'audio, il motore.
pub fn ambiente(state: &remote_api::AppState, config: &crate::Config, avvio: Instant) -> Value {
    let pc = crate::rapporti::pc();
    let audio = state.audio.as_ref().map(|a| a.devices());
    let dispositivi = |elenco: &[remote_api::audio_api::AudioDevice]| -> Value {
        Value::Array(elenco.iter().map(|d| json!({ "nome": d.name, "canali": d.channels, "predefinito": d.is_default })).collect())
    };
    let stazione = state.current_status().map(|s| {
        // senza nominativo e locatore: modo, frequenza e cosa sta facendo il motore
        json!({ "modo": s.mode, "dial_hz": s.dial_hz, "tx_abilitata": s.tx_enabled, "trasmette": s.transmitting, "decodifica": s.decoding, "periodo_s": s.tr_period, "audio_rx_hz": s.rx_df, "audio_tx_hz": s.tx_df })
    });
    json!({
        "aurora": {
            "versione": aurora_core::version::APP_VERSION,
            "acceso_da_secondi": avvio.elapsed().as_secs(),
            "motore": match config.engine { crate::EngineKind::Native => "native", crate::EngineKind::Sim => "sim" },
            "porta_web": config.bind.port(),
            "trasmissione_consentita": config.allow_tx,
        },
        "pc": { "processore": pc.cpu, "core": pc.core, "ram_gb": pc.ram_gb, "sistema": std::env::consts::OS, "architettura": std::env::consts::ARCH },
        "stazione": stazione,
        "audio": audio.map(|a| json!({
            "ingressi": dispositivi(&a.inputs),
            "uscite": dispositivi(&a.outputs),
            "ingresso_in_uso": a.input_in_use,
            "uscita_in_uso": a.output_in_use,
            "errore_ingresso": a.input_error,
            "errore_uscita": a.output_error,
        })),
        "variabili_aurora": variabili_aurora(),
    })
}

/// Tutti i file di diagnostica (nome, testo).
pub fn raccogli(state: &remote_api::AppState, config: &crate::Config, cat: &rig_control::cat::CatService, avvio: Instant) -> Vec<(String, String)> {
    let mut file = vec![
        ("cat-comandi.log".to_string(), rig_control::trace::dump()),
        ("radio.json".to_string(), pretty(&radio(cat))),
        ("ambiente.json".to_string(), pretty(&ambiente(state, config, avvio))),
    ];
    for (origine, testo) in remote_api::ui_snapshots() {
        // il testo e' di una pagina: si rilegge come JSON e si riscrive in ordine; se non lo e' si tiene com'e'
        let testo = serde_json::from_str::<Value>(&testo).map(|v| pretty(&v)).unwrap_or(testo);
        file.push((format!("interfaccia-{origine}.json"), testo));
    }
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_environment_variables_are_hidden_and_the_others_kept() {
        // SAFETY: la prova e' la sola che tocca queste due variabili
        unsafe {
            std::env::set_var("AURORA_PROVA_RIG", "hamlib");
            std::env::set_var("AURORA_PROVA_TOKEN", "segretissimo");
        }
        let v = variabili_aurora();
        assert_eq!(v["AURORA_PROVA_RIG"], "hamlib");
        assert_eq!(v["AURORA_PROVA_TOKEN"], "***");
        unsafe {
            std::env::remove_var("AURORA_PROVA_RIG");
            std::env::remove_var("AURORA_PROVA_TOKEN");
        }
    }
}
