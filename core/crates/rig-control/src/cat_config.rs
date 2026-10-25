//! Configurazione del CAT e profili salvati: il file `radio.json` nella cartella dei dati. Niente segreti ne' nominativi: solo
//! scelte tecniche (modello, porta, velocita', PTT). Schema e regole nell'analisi `docs/analisi-decodium-cat.md`.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

use crate::{hamlib::RigctldConfig, RigError};

/// Come si comanda la radio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// Hamlib tramite `rigctld`: oltre 300 modelli.
    #[default]
    Hamlib,
    /// OmniRig (Windows), il percorso storico di Aurora.
    Omnirig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Handshake {
    /// Quello predefinito del modello.
    #[default]
    Default,
    None,
    Xonxoff,
    Hardware,
}

/// Metodo del PTT: via CAT, linee DTR o RTS di una porta, oppure VOX (nessun comando).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PttMethod {
    #[default]
    Cat,
    Dtr,
    Rts,
    Vox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitMode {
    /// La radio resta in simplex.
    #[default]
    None,
    /// Split vero della radio (VFO di trasmissione spostato).
    Rig,
    /// Simplex con la frequenza spostata solo durante la trasmissione.
    Emulate,
}

/// Il modo che Aurora scrive nella radio a ogni cambio di frequenza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CatMode {
    /// Non si tocca il modo scelto sulla radio.
    #[default]
    None,
    Usb,
    /// USB dati (`PKTUSB` per Hamlib).
    Data,
}

impl CatMode {
    /// Il nome del modo per Hamlib.
    pub fn hamlib_name(self) -> Option<&'static str> {
        match self {
            CatMode::None => None,
            CatMode::Usb => Some("USB"),
            CatMode::Data => Some("PKTUSB"),
        }
    }
}

/// Tutto cio' che serve per collegare una radio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RigConfig {
    pub backend: Backend,
    /// Numero del modello di Hamlib (0 = nessuna scelta ancora).
    pub model: u32,
    /// Nome mostrato («Yaesu FT-2000»).
    pub rig_name: String,
    /// Porta seriale (`COM3`) oppure `host:porta` per le radio di rete.
    pub port: String,
    /// Velocita' in baud; 0 = quella del modello.
    pub baud: u32,
    /// 0 = predefiniti del modello, altrimenti 7 o 8.
    pub data_bits: u8,
    /// 0 = predefiniti del modello, altrimenti 1 o 2.
    pub stop_bits: u8,
    pub handshake: Handshake,
    /// Forza la linea DTR (`Some(true)` alta, `Some(false)` bassa); `None` = regola di sicurezza.
    pub force_dtr: Option<bool>,
    pub force_rts: Option<bool>,
    pub ptt: PttMethod,
    /// Porta del PTT per DTR/RTS; vuota = la stessa del CAT.
    pub ptt_port: String,
    pub split: SplitMode,
    pub cat_mode: CatMode,
    /// Indirizzo CI-V delle Icom; 0 = quello del modello.
    pub civ_address: u8,
    /// Intervallo di lettura (s); per le seriali non scende sotto 2.
    pub poll_secs: u32,
    /// Collega la radio da solo all'avvio di Aurora (spento di base: la trasmissione non parte mai senza che lo si sia scelto).
    pub auto_connect: bool,
    /// «Condividi la radio»: `rigctld` ascolta (solo su questo PC, 127.0.0.1) sulla porta fissa `share_port` invece che su una a caso,
    /// cosi' Log4OM2, GridTracker, JTDX e gli altri si collegano ad Aurora come a una radio «Hamlib NET rigctl».
    pub share_enabled: bool,
    pub share_port: u16,
}

/// Porta TCP di `rigctld` di base (quella che i programmi propongono per «Hamlib NET rigctl»).
pub const DEFAULT_SHARE_PORT: u16 = 4532;

impl Default for RigConfig {
    fn default() -> Self {
        RigConfig {
            backend: Backend::Hamlib,
            model: 0,
            rig_name: String::new(),
            port: String::new(),
            baud: 0,
            data_bits: 0,
            stop_bits: 0,
            handshake: Handshake::Default,
            force_dtr: None,
            force_rts: None,
            ptt: PttMethod::Cat,
            ptt_port: String::new(),
            split: SplitMode::None,
            cat_mode: CatMode::None,
            civ_address: 0,
            poll_secs: 2,
            auto_connect: false,
            share_enabled: false,
            share_port: DEFAULT_SHARE_PORT,
        }
    }
}

/// Le velocita' offerte dalla schermata.
pub const BAUDS: [u32; 8] = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200];

/// Una porta seriale (`COM3`, `/dev/ttyUSB0`) e non un indirizzo di rete.
pub fn is_serial_port(port: &str) -> bool {
    let up = port.to_ascii_uppercase();
    if let Some(rest) = up.strip_prefix("COM") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    up.starts_with("/DEV/") || up.starts_with("\\\\.\\")
}

/// Il nome della porta come lo vuole Hamlib su Windows: da COM10 in su serve il prefisso `\\.\`.
pub fn hamlib_port(port: &str) -> String {
    let up = port.to_ascii_uppercase();
    match up.strip_prefix("COM").and_then(|n| n.parse::<u32>().ok()) {
        Some(n) if n >= 10 => format!("\\\\.\\COM{n}"),
        _ => port.to_string(),
    }
}

fn token_ok(text: &str) -> bool {
    !text.is_empty() && text.len() <= 64 && text.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '/' | '\\'))
}

impl RigConfig {
    /// Controlla i valori; l'errore e' un codice breve che l'interfaccia traduce.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.rig_name.chars().count() > 80 || self.rig_name.chars().any(char::is_control) {
            return Err("bad_rig_name");
        }
        if self.model > 99_999 {
            return Err("bad_model");
        }
        if !self.port.is_empty() && !token_ok(&self.port) {
            return Err("bad_port");
        }
        if !self.ptt_port.is_empty() && !token_ok(&self.ptt_port) {
            return Err("bad_port");
        }
        if self.baud != 0 && !BAUDS.contains(&self.baud) {
            return Err("bad_baud");
        }
        if !matches!(self.data_bits, 0 | 7 | 8) || !matches!(self.stop_bits, 0..=2) {
            return Err("bad_serial");
        }
        if !(1..=99).contains(&self.poll_secs) {
            return Err("bad_poll");
        }
        if matches!(self.ptt, PttMethod::Dtr | PttMethod::Rts) && self.port.is_empty() && self.ptt_port.is_empty() {
            return Err("bad_ptt");
        }
        if !(1024..=65535).contains(&self.share_port) {
            return Err("bad_share_port");
        }
        Ok(())
    }

    /// L'intervallo di lettura in uso: le seriali non scendono sotto 2 s.
    pub fn effective_poll_secs(&self) -> u32 {
        if is_serial_port(&self.port) { self.poll_secs.max(2) } else { self.poll_secs.max(1) }
    }

    /// Gli argomenti per `rigctld`. Le linee DTR e RTS si tengono basse all'apertura (molte interfacce hanno una delle due sul PTT e
    /// un adattatore le alza per un istante): solo se la porta e' seriale, l'utente non ha chiesto altro, la linea non e' il PTT di questa porta
    /// e, per RTS, non c'e' handshake hardware (con «predefinito» Hamlib potrebbe sceglierlo e rifiutare la scrittura).
    pub fn to_rigctld(&self, exe: &Path) -> Result<RigctldConfig, RigError> {
        self.validate().map_err(|code| RigError(format!("impostazione radio non valida ({code})")))?;
        if self.model == 0 {
            return Err(RigError("scegli prima il modello della radio".into()));
        }
        let serial = is_serial_port(&self.port);
        let mut conf: Vec<(String, String)> = Vec::new();
        if self.data_bits != 0 {
            conf.push(("data_bits".into(), self.data_bits.to_string()));
        }
        if self.stop_bits != 0 {
            conf.push(("stop_bits".into(), self.stop_bits.to_string()));
        }
        match self.handshake {
            Handshake::Default => {}
            Handshake::None => conf.push(("serial_handshake".into(), "None".into())),
            Handshake::Xonxoff => conf.push(("serial_handshake".into(), "XONXOFF".into())),
            Handshake::Hardware => conf.push(("serial_handshake".into(), "Hardware".into())),
        }
        if self.civ_address != 0 {
            conf.push(("civaddr".into(), self.civ_address.to_string()));
        }
        let ptt_on_cat_port = |same: bool| same && matches!(self.ptt, PttMethod::Dtr | PttMethod::Rts);
        let ptt_same_port = self.ptt_port.is_empty() || self.ptt_port == self.port;
        if serial {
            let dtr_is_ptt = ptt_on_cat_port(ptt_same_port) && self.ptt == PttMethod::Dtr;
            let rts_is_ptt = ptt_on_cat_port(ptt_same_port) && self.ptt == PttMethod::Rts;
            let line = |high: bool| if high { "ON" } else { "OFF" }.to_string();
            match self.force_dtr {
                Some(high) => conf.push(("dtr_state".into(), line(high))),
                None if !dtr_is_ptt => conf.push(("dtr_state".into(), "OFF".into())),
                None => {}
            }
            let handshake_explicit = matches!(self.handshake, Handshake::None | Handshake::Xonxoff);
            match self.force_rts {
                Some(high) if self.handshake != Handshake::Hardware => conf.push(("rts_state".into(), line(high))),
                None if !rts_is_ptt && handshake_explicit => conf.push(("rts_state".into(), "OFF".into())),
                _ => {}
            }
        }
        match self.ptt {
            PttMethod::Cat => {}
            PttMethod::Vox => conf.push(("ptt_type".into(), "None".into())),
            PttMethod::Dtr | PttMethod::Rts => {
                conf.push(("ptt_type".into(), if self.ptt == PttMethod::Dtr { "DTR" } else { "RTS" }.into()));
                if !ptt_same_port {
                    conf.push(("ptt_pathname".into(), hamlib_port(&self.ptt_port)));
                }
                conf.push(("ptt_share".into(), "1".into()));
            }
        }
        // «meno scatti» negli split delle Icom
        conf.push(("no_xchg".into(), "1".into()));
        Ok(RigctldConfig {
            exe: exe.to_path_buf(),
            model: self.model,
            port: (!self.port.is_empty()).then(|| hamlib_port(&self.port)),
            baud: (self.baud != 0).then_some(self.baud),
            conf,
            tcp_port: self.share_enabled.then_some(self.share_port),
        })
    }
}

/// Frequenza di spostamento (XIT) per lo split: l'audio resta tra 1500 e 2000 Hz.
pub fn split_xit_hz(tx_audio_hz: u32) -> i64 {
    i64::from(tx_audio_hz / 500) * 500 - 1500
}

/// Frequenza di trasmissione della radio (dial) per una frequenza audio di trasmissione. Un valore a piu' di 1 MHz dalla ricezione e' un dato
/// sporco (sembra un salto di banda): si ripiega su RX + 4 kHz.
pub fn split_tx_dial_hz(rx_dial_hz: u64, tx_audio_hz: u32) -> u64 {
    let tx = (rx_dial_hz as i64 + split_xit_hz(tx_audio_hz)).max(0) as u64;
    if tx.abs_diff(rx_dial_hz) > 1_000_000 { rx_dial_hz + 4000 } else { tx }
}

/// Se un messaggio di errore e' un timeout o un guasto di ingresso/uscita (la radio o il cavo, non un rifiuto del comando).
pub fn is_io_failure(message: &str) -> bool {
    let m = message.to_lowercase();
    ["timeout", "ingresso/uscita", "non risponde", "chiuso la connessione", "lettura:", "scrittura:", "terminato", "si e' fermato"].iter().any(|k| m.contains(k))
}

/// Traduce l'errore tecnico in un testo comprensibile: Hamlib mette nel messaggio l'ultima riga di debug invece del testo dell'errore.
pub fn friendly_error(raw: &str) -> String {
    let m = raw.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| m.contains(w));
    if has(&["porta di condivisione"]) {
        raw.to_string()
    } else if has(&["rigctld non trovato"]) {
        "Manca rigctld (Hamlib): mettilo accanto ad Aurora o indica il percorso nella variabile AURORA_RIGCTLD.".into()
    } else if has(&["already open", "access denied", "accesso negato", "occupata", "in uso", "sharing violation"]) {
        "La porta seriale e' occupata da un altro programma: chiudilo (o scegli un'altra porta) e riprova.".into()
    } else if has(&["no such file", "cannot find", "non trova", "invalid port", "does not exist", "specificato"]) {
        "La porta seriale non esiste: controlla il numero (Windows puo' rinumerarla) e che la radio sia accesa e collegata.".into()
    } else if has(&["timeout", "non risponde"]) {
        "La radio non risponde: controlla accensione, cavo, porta, velocita' (baud) e, per le Yaesu, le impostazioni CAT della radio.".into()
    } else if has(&["ingresso/uscita", "lettura:", "scrittura:", "chiuso la connessione"]) {
        "Errore sulla porta seriale: la connessione con la radio si e' interrotta.".into()
    } else if has(&["si e' fermato", "terminato"]) {
        format!("rigctld si e' fermato: controlla modello, porta e velocita'. ({raw})")
    } else if has(&["non implementata", "non disponibile"]) {
        "La radio non supporta questo comando.".into()
    } else if has(&["rifiutato"]) {
        "La radio ha rifiutato il comando.".into()
    } else {
        raw.to_string()
    }
}

/// Un profilo salvato: nome e configurazione.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RigProfile {
    pub name: String,
    pub config: RigConfig,
}

/// Il contenuto di `radio.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioFile {
    /// La configurazione viva.
    pub config: RigConfig,
    pub profiles: Vec<RigProfile>,
    /// Il profilo caricato per ultimo (solo per mostrarlo: non si riapplica all'avvio).
    pub active_profile: Option<String>,
}

/// Errori delle operazioni sui profili e sul file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    BadName,
    NotFound,
    Invalid(&'static str),
    Write(String),
}

impl StoreError {
    pub fn code(&self) -> &'static str {
        match self {
            StoreError::BadName => "bad_name",
            StoreError::NotFound => "no_such_profile",
            StoreError::Invalid(code) => code,
            StoreError::Write(_) => "write_failed",
        }
    }
}

/// Il nome di un profilo: spazi compressi, non vuoto, al massimo 80 caratteri, senza caratteri di controllo.
pub fn normalize_profile_name(name: &str) -> Result<String, StoreError> {
    let clean = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() || clean.chars().count() > 80 || clean.chars().any(char::is_control) {
        return Err(StoreError::BadName);
    }
    Ok(clean)
}

/// Il file dei profili e della configurazione viva.
pub struct RadioStore {
    path: PathBuf,
    file: Mutex<RadioFile>,
}

impl RadioStore {
    /// Apre il file; se manca o non e' leggibile si parte dai valori di base (un file rovinato si mette da parte come `.bad`).
    pub fn open(path: &Path) -> RadioStore {
        let file = match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str::<RadioFile>(&text).unwrap_or_else(|e| {
                tracing::warn!("{} non leggibile ({e}): parto dai valori di base", path.display());
                let _ = std::fs::rename(path, path.with_extension("json.bad"));
                RadioFile::default()
            }),
            Err(_) => RadioFile::default(),
        };
        RadioStore { path: path.to_path_buf(), file: Mutex::new(file) }
    }

    pub fn snapshot(&self) -> RadioFile {
        self.file.lock().map(|f| f.clone()).unwrap_or_default()
    }

    pub fn config(&self) -> RigConfig {
        self.snapshot().config
    }

    /// Cambia la configurazione viva senza salvarla (la scelta di `AURORA_RIG` all'avvio).
    pub fn adopt_backend(&self, backend: Backend) {
        if let Ok(mut f) = self.file.lock() {
            f.config.backend = backend;
        }
    }

    fn change(&self, edit: impl FnOnce(&mut RadioFile) -> Result<(), StoreError>) -> Result<RadioFile, StoreError> {
        let mut guard = self.file.lock().map_err(|_| StoreError::Write("file della radio non disponibile".into()))?;
        let mut next = guard.clone();
        edit(&mut next)?;
        write_atomic(&self.path, &next).map_err(StoreError::Write)?;
        *guard = next.clone();
        Ok(next)
    }

    /// Salva la configurazione viva.
    pub fn set_config(&self, config: RigConfig) -> Result<RadioFile, StoreError> {
        config.validate().map_err(StoreError::Invalid)?;
        self.change(|f| {
            f.config = config;
            Ok(())
        })
    }

    /// Salva la configurazione viva come profilo `name` (se esiste lo sostituisce).
    pub fn save_profile(&self, name: &str) -> Result<RadioFile, StoreError> {
        let name = normalize_profile_name(name)?;
        self.change(|f| {
            let config = f.config.clone();
            match f.profiles.iter_mut().find(|p| p.name.eq_ignore_ascii_case(&name)) {
                Some(p) => {
                    p.name = name.clone();
                    p.config = config;
                }
                None => f.profiles.push(RigProfile { name: name.clone(), config }),
            }
            f.active_profile = Some(name);
            Ok(())
        })
    }

    /// Porta il profilo nella configurazione viva. Non collega nulla.
    pub fn load_profile(&self, name: &str) -> Result<RadioFile, StoreError> {
        self.change(|f| {
            let p = f.profiles.iter().find(|p| p.name.eq_ignore_ascii_case(name.trim())).cloned().ok_or(StoreError::NotFound)?;
            f.config = p.config;
            f.active_profile = Some(p.name);
            Ok(())
        })
    }

    /// Un profilo nuovo, con le impostazioni di base, che diventa quello vivo. Senza nome ne propone uno libero.
    pub fn new_profile(&self, name: Option<&str>) -> Result<RadioFile, StoreError> {
        self.change(|f| {
            let name = match name.map(str::trim).filter(|n| !n.is_empty()) {
                Some(n) => normalize_profile_name(n)?,
                None => suggest_name(&f.config, &f.profiles),
            };
            if f.profiles.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
                return Err(StoreError::BadName);
            }
            let config = RigConfig { backend: f.config.backend, ..RigConfig::default() };
            f.profiles.push(RigProfile { name: name.clone(), config: config.clone() });
            f.config = config;
            f.active_profile = Some(name);
            Ok(())
        })
    }

    pub fn delete_profile(&self, name: &str) -> Result<RadioFile, StoreError> {
        self.change(|f| {
            let before = f.profiles.len();
            f.profiles.retain(|p| !p.name.eq_ignore_ascii_case(name.trim()));
            if f.profiles.len() == before {
                return Err(StoreError::NotFound);
            }
            if f.active_profile.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(name.trim())) {
                f.active_profile = None;
            }
            Ok(())
        })
    }
}

/// «<radio> <backend>», con un numero se esiste gia'.
fn suggest_name(config: &RigConfig, profiles: &[RigProfile]) -> String {
    let rig = if config.rig_name.trim().is_empty() { "Radio" } else { config.rig_name.trim() };
    let backend = match config.backend {
        Backend::Hamlib => "Hamlib",
        Backend::Omnirig => "OmniRig",
    };
    let base: String = format!("{rig} {backend}").chars().take(76).collect();
    let taken = |n: &str| profiles.iter().any(|p| p.name.eq_ignore_ascii_case(n));
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| !taken(n))
        .unwrap_or(base)
}

fn write_atomic(path: &Path, file: &RadioFile) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-radio-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("radio.json")
    }

    fn ft2000() -> RigConfig {
        RigConfig { model: 1029, rig_name: "Yaesu FT-2000".into(), port: "COM3".into(), baud: 38400, data_bits: 8, stop_bits: 2, handshake: Handshake::Hardware, ..RigConfig::default() }
    }

    #[test]
    fn serial_ports_are_told_from_network_addresses() {
        assert!(is_serial_port("COM3") && is_serial_port("com12") && is_serial_port("/dev/ttyUSB0"));
        assert!(!is_serial_port("localhost:4532") && !is_serial_port("COM") && !is_serial_port(""));
        assert_eq!(hamlib_port("COM3"), "COM3");
        assert_eq!(hamlib_port("COM10"), "\\\\.\\COM10");
        assert_eq!(hamlib_port("192.168.1.5:4532"), "192.168.1.5:4532");
    }

    #[test]
    fn the_ft2000_maps_to_rigctld_arguments() {
        let r = ft2000().to_rigctld(Path::new("rigctld.exe")).expect("config");
        assert_eq!((r.model, r.port.as_deref(), r.baud), (1029, Some("COM3"), Some(38400)));
        let conf: Vec<String> = r.conf.iter().map(|(k, v)| format!("{k}={v}")).collect();
        assert!(conf.contains(&"data_bits=8".to_string()) && conf.contains(&"stop_bits=2".to_string()) && conf.contains(&"serial_handshake=Hardware".to_string()));
        // DTR basso per sicurezza; RTS non si forza con l'handshake hardware
        assert!(conf.contains(&"dtr_state=OFF".to_string()));
        assert!(!conf.iter().any(|c| c.starts_with("rts_state")));
        assert!(!conf.iter().any(|c| c.starts_with("ptt_type")));
        assert!(crate::hamlib::rigctld_args(&r, 4532).is_ok(), "gli argomenti passano i controlli di rigctld");
    }

    #[test]
    fn dtr_and_rts_are_kept_low_unless_they_are_the_ptt() {
        let mut c = RigConfig { model: 1, port: "COM4".into(), handshake: Handshake::None, ..RigConfig::default() };
        let conf = |c: &RigConfig| c.to_rigctld(Path::new("x")).expect("config").conf.into_iter().collect::<std::collections::BTreeMap<_, _>>();
        let m = conf(&c);
        assert_eq!((m.get("dtr_state").map(String::as_str), m.get("rts_state").map(String::as_str)), (Some("OFF"), Some("OFF")));
        c.ptt = PttMethod::Rts;
        let m = conf(&c);
        assert_eq!(m.get("rts_state"), None, "RTS e' il PTT: non si forza");
        assert_eq!((m.get("ptt_type").map(String::as_str), m.get("ptt_share").map(String::as_str)), (Some("RTS"), Some("1")));
        c.ptt = PttMethod::Dtr;
        c.ptt_port = "COM12".into();
        let m = conf(&c);
        assert_eq!(m.get("ptt_pathname").map(String::as_str), Some("\\\\.\\COM12"));
        assert_eq!(m.get("dtr_state").map(String::as_str), Some("OFF"), "il DTR del PTT e' sull'altra porta: quello del CAT resta basso");
        c.force_dtr = Some(true);
        assert_eq!(conf(&c).get("dtr_state").map(String::as_str), Some("ON"));
    }

    #[test]
    fn sharing_the_radio_fixes_the_rigctld_port_and_old_files_still_load() {
        let mut c = ft2000();
        assert_eq!(c.to_rigctld(Path::new("x")).expect("config").tcp_port, None, "senza condivisione la porta e' a caso");
        c.share_enabled = true;
        assert_eq!(c.to_rigctld(Path::new("x")).expect("config").tcp_port, Some(4532));
        c.share_port = 4600;
        assert_eq!(c.to_rigctld(Path::new("x")).expect("config").tcp_port, Some(4600));
        let old: RigConfig = serde_json::from_str(r#"{"model":1029,"port":"COM3"}"#).expect("file vecchio");
        assert!(!old.share_enabled && old.share_port == DEFAULT_SHARE_PORT);
    }

    #[test]
    fn invalid_values_are_refused_with_a_code() {
        let bad = |edit: fn(&mut RigConfig)| {
            let mut c = ft2000();
            edit(&mut c);
            c.validate().expect_err("doveva rifiutare")
        };
        assert_eq!(bad(|c| c.port = "COM3 & calc".into()), "bad_port");
        assert_eq!(bad(|c| c.baud = 12345), "bad_baud");
        assert_eq!(bad(|c| c.poll_secs = 0), "bad_poll");
        assert_eq!(bad(|c| c.poll_secs = 100), "bad_poll");
        assert_eq!(bad(|c| c.data_bits = 9), "bad_serial");
        assert_eq!(bad(|c| { c.ptt = PttMethod::Dtr; c.port.clear(); c.ptt_port.clear(); }), "bad_ptt");
        assert_eq!(bad(|c| c.rig_name = "x".repeat(81)), "bad_rig_name");
        assert_eq!(bad(|c| c.share_port = 80), "bad_share_port");
        assert_eq!(bad(|c| c.share_port = 0), "bad_share_port");
        assert!(RigConfig::default().to_rigctld(Path::new("x")).is_err(), "senza modello non si collega");
        assert_eq!(ft2000().effective_poll_secs(), 2);
        let mut fast = ft2000();
        fast.poll_secs = 1;
        assert_eq!(fast.effective_poll_secs(), 2, "le seriali non scendono sotto 2 s");
        fast.port = "localhost:4532".into();
        assert_eq!(fast.effective_poll_secs(), 1);
    }

    #[test]
    fn split_follows_the_decodium_rule() {
        assert_eq!(split_xit_hz(1500), 0);
        assert_eq!(split_xit_hz(2200), 500);
        assert_eq!(split_xit_hz(300), -1500);
        // l'audio che esce dalla radio resta tra 1500 e 2000
        for f in [200u32, 750, 1499, 1500, 2199, 2999] {
            let audio = i64::from(f) - split_xit_hz(f);
            assert!((1500..2000).contains(&audio), "{f} -> {audio}");
        }
        assert_eq!(split_tx_dial_hz(14_074_000, 2200), 14_074_500);
        assert_eq!(split_tx_dial_hz(14_074_000, 300), 14_072_500);
    }

    #[test]
    fn errors_are_translated_and_classified() {
        assert!(friendly_error("rigctld si e' fermato (exit code: 1): rig_open: Access denied").contains("occupata"));
        assert!(friendly_error("f: timeout: la radio non risponde (-5)").contains("non risponde"));
        assert!(friendly_error("rigctld non trovato: C:/x").contains("AURORA_RIGCTLD"));
        assert!(is_io_failure("f: timeout: la radio non risponde (-5)"));
        assert!(is_io_failure("rigctld: lettura: connessione interrotta"));
        assert!(!is_io_failure("M: comando rifiutato dalla radio (-9)"));
    }

    #[test]
    fn profiles_are_saved_loaded_renamed_and_deleted() {
        let path = temp("profiles");
        let store = RadioStore::open(&path);
        store.set_config(ft2000()).expect("salva");
        store.save_profile("  FT-2000   casa ").expect("profilo");
        let f = store.snapshot();
        assert_eq!(f.profiles.len(), 1);
        assert_eq!(f.profiles[0].name, "FT-2000 casa");
        assert_eq!(f.active_profile.as_deref(), Some("FT-2000 casa"));
        // un'altra radio nella configurazione viva, poi il profilo la riporta com'era
        store.set_config(RigConfig { model: 3085, ..RigConfig::default() }).expect("salva");
        assert_eq!(store.config().model, 3085);
        store.load_profile("ft-2000 CASA").expect("carica");
        assert_eq!(store.config().model, 1029);
        // salvare con lo stesso nome sostituisce
        store.set_config(RigConfig { model: 1, ..ft2000() }).expect("salva");
        store.save_profile("FT-2000 casa").expect("rimpiazza");
        assert_eq!(store.snapshot().profiles.len(), 1);
        assert_eq!(store.snapshot().profiles[0].config.model, 1);
        // nuovo: nome proposto e configurazione di base
        let f = store.new_profile(None).expect("nuovo");
        assert_eq!(f.profiles.len(), 2);
        assert_eq!(f.config.model, 0);
        assert!(f.active_profile.as_deref().is_some_and(|n| n.ends_with("Hamlib")));
        assert!(store.new_profile(Some("FT-2000 casa")).is_err(), "nome gia' usato");
        store.delete_profile("FT-2000 casa").expect("elimina");
        assert_eq!(store.snapshot().profiles.len(), 1);
        assert_eq!(store.delete_profile("non c'e'").expect_err("manca"), StoreError::NotFound);
        assert_eq!(store.load_profile("non c'e'").expect_err("manca"), StoreError::NotFound);
        assert_eq!(store.save_profile("   ").expect_err("vuoto"), StoreError::BadName);
        assert_eq!(store.save_profile(&"x".repeat(81)).expect_err("lungo"), StoreError::BadName);
        // il file sopravvive alla riapertura
        let again = RadioStore::open(&path);
        assert_eq!(again.snapshot(), store.snapshot());
    }

    #[test]
    fn a_broken_file_does_not_stop_the_program_and_nothing_secret_is_written() {
        let path = temp("broken");
        std::fs::create_dir_all(path.parent().expect("cartella")).expect("cartella");
        std::fs::write(&path, "{ non e' json").expect("scrittura");
        let store = RadioStore::open(&path);
        assert_eq!(store.snapshot(), RadioFile::default());
        assert!(path.with_extension("json.bad").is_file());
        store.set_config(ft2000()).expect("salva");
        let text = std::fs::read_to_string(&path).expect("lettura");
        for secret in ["password", "token", "secret", "call"] {
            assert!(!text.to_lowercase().contains(secret), "{secret}");
        }
        // i campi mancanti prendono i valori di base (file di una versione piu' vecchia)
        std::fs::write(&path, r#"{"config":{"model":1029}}"#).expect("scrittura");
        let old = RadioStore::open(&path).config();
        assert_eq!((old.model, old.poll_secs, old.backend), (1029, 2, Backend::Hamlib));
    }

    #[test]
    fn the_saved_config_rejects_invalid_values() {
        let store = RadioStore::open(&temp("invalid"));
        let mut c = ft2000();
        c.baud = 7;
        assert_eq!(store.set_config(c).expect_err("baud"), StoreError::Invalid("bad_baud"));
        assert_eq!(store.config(), RigConfig::default());
    }
}
