//! Lista dei desiderati: nominativi, prefissi, paesi e griglie che si vogliono collegare. Quando una di queste stazioni compare in un
//! messaggio decodificato o in uno spot del cluster scatta un avviso (nella pagina e, se si vuole, sul telefono con ntfy).
//!
//! Questo crate sa solo riconoscere, ricordare e avvisare; chi guarda i messaggi e gli spot e' il programma principale.

use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use serde::{Deserialize, Serialize};

pub use aurora_core::WantedAlert;

/// Quante voci al massimo per elenco.
pub const MAX_ITEMS: usize = 200;
/// Quanti avvisi recenti si tengono in memoria.
const MAX_RECENT: usize = 100;

/// Perche' una stazione e' desiderata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Call,
    Prefix,
    Country,
    Grid,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::Call => "call",
            Reason::Prefix => "prefix",
            Reason::Country => "country",
            Reason::Grid => "grid",
        }
    }
}

/// Le impostazioni della lista dei desiderati, salvate su disco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Wishlist {
    pub enabled: bool,
    /// Nominativi voluti («K1ABC»): vale anche «K1ABC/P» e «F/K1ABC».
    pub calls: Vec<String>,
    /// Prefissi voluti («3Y», «VP8»): il nominativo comincia cosi'.
    pub prefixes: Vec<String>,
    /// Paesi voluti, col nome di `cty.dat` come lo mostra l'interfaccia («Bulgaria»).
    pub countries: Vec<String>,
    /// Quadrati locatore voluti, di 4 caratteri («JN45»).
    pub grids: Vec<String>,
    /// Solo su queste bande («20m»); vuoto = tutte.
    pub bands: Vec<String>,
    /// Dopo un avviso per una stazione, quanti minuti prima di avvisare ancora per la stessa (1-240).
    pub cooldown_min: u32,
    pub from_decodes: bool,
    pub from_cluster: bool,
    /// Indirizzo di un server ntfy (`https://ntfy.sh/il-tuo-argomento`) per avvisare anche il telefono a pagina chiusa; vuoto = nessuno.
    pub notify_url: String,
}

impl Default for Wishlist {
    fn default() -> Self {
        Wishlist {
            enabled: false,
            calls: Vec::new(),
            prefixes: Vec::new(),
            countries: Vec::new(),
            grids: Vec::new(),
            bands: Vec::new(),
            cooldown_min: 15,
            from_decodes: true,
            from_cluster: true,
            notify_url: String::new(),
        }
    }
}

fn clean(list: &[String], upper: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in list {
        let t = item.trim();
        if t.is_empty() {
            continue;
        }
        let t = if upper { t.to_uppercase() } else { t.to_string() };
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
    }
    out
}

/// La parte che e' il nominativo vero: «K1ABC/P» e «F/K1ABC» danno «K1ABC» (il pezzo piu' lungo con una cifra).
pub fn base_call(call: &str) -> &str {
    call.split('/').filter(|p| p.chars().any(|c| c.is_ascii_digit())).max_by_key(|p| p.len()).unwrap_or(call)
}

fn valid_call_like(s: &str) -> bool {
    !s.is_empty() && s.len() <= 15 && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '/')
}

fn valid_grid4(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()
}

fn valid_band(s: &str) -> bool {
    (2..=5).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric())
}

impl Wishlist {
    /// Toglie spazi e doppioni e scrive in maiuscolo nominativi, prefissi, griglie e bande (i paesi restano come sono).
    pub fn normalized(mut self) -> Wishlist {
        self.calls = clean(&self.calls, true);
        self.prefixes = clean(&self.prefixes, true);
        self.grids = clean(&self.grids, true);
        self.countries = clean(&self.countries, false);
        self.bands = clean(&self.bands, false).into_iter().map(|b| b.to_lowercase()).collect();
        self.notify_url = self.notify_url.trim().to_string();
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        for (name, list) in [("nominativi", &self.calls), ("prefissi", &self.prefixes), ("paesi", &self.countries), ("griglie", &self.grids), ("bande", &self.bands)] {
            if list.len() > MAX_ITEMS {
                return Err(format!("troppe voci tra i {name} (al massimo {MAX_ITEMS})"));
            }
        }
        if let Some(c) = self.calls.iter().find(|c| !valid_call_like(c)) {
            return Err(format!("nominativo non valido: {c}"));
        }
        if let Some(p) = self.prefixes.iter().find(|p| !valid_call_like(p)) {
            return Err(format!("prefisso non valido: {p}"));
        }
        if let Some(g) = self.grids.iter().find(|g| !valid_grid4(g)) {
            return Err(format!("griglia non valida (servono 4 caratteri, per esempio JN45): {g}"));
        }
        if let Some(b) = self.bands.iter().find(|b| !valid_band(b)) {
            return Err(format!("banda non valida: {b}"));
        }
        if let Some(c) = self.countries.iter().find(|c| c.len() > 60 || c.chars().any(char::is_control)) {
            return Err(format!("paese non valido: {c}"));
        }
        if !(1..=240).contains(&self.cooldown_min) {
            return Err("l'attesa tra due avvisi va da 1 a 240 minuti".into());
        }
        if !self.notify_url.is_empty() {
            let u = &self.notify_url;
            if u.len() > 300 || !(u.starts_with("http://") || u.starts_with("https://")) || u.chars().any(|c| c.is_whitespace() || c.is_control()) {
                return Err("l'indirizzo di avviso deve cominciare con http:// o https://, senza spazi".into());
            }
        }
        Ok(())
    }

    /// La stazione e' desiderata? `grid`, `country` e `band` si danno se si conoscono. Vale la prima ragione trovata: nominativo, prefisso,
    /// paese, griglia. Con un filtro di banda, una banda nota fuori dall'elenco esclude; una banda ignota no.
    pub fn reason(&self, call: &str, grid: Option<&str>, country: Option<&str>, band: Option<&str>) -> Option<Reason> {
        if !self.enabled {
            return None;
        }
        if !self.bands.is_empty() && band.is_some_and(|b| !self.bands.iter().any(|x| x.eq_ignore_ascii_case(b))) {
            return None;
        }
        let call = call.trim().to_uppercase();
        if call.is_empty() {
            return None;
        }
        let base = base_call(&call);
        if self.calls.iter().any(|c| *c == call || c == base) {
            return Some(Reason::Call);
        }
        if self.prefixes.iter().any(|p| call.starts_with(p.as_str()) || base.starts_with(p.as_str())) {
            return Some(Reason::Prefix);
        }
        if let Some(country) = country
            && self.countries.iter().any(|c| c.eq_ignore_ascii_case(country))
        {
            return Some(Reason::Country);
        }
        if let Some(grid) = grid
            && grid.len() >= 4
            && grid.is_char_boundary(4)
            && self.grids.iter().any(|g| g.eq_ignore_ascii_case(&grid[..4]))
        {
            return Some(Reason::Grid);
        }
        None
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// La lista in uso, il suo file e gli avvisi recenti.
pub struct WishlistService {
    path: PathBuf,
    config: Mutex<Wishlist>,
    recent: Mutex<VecDeque<WantedAlert>>,
    last_alert: Mutex<HashMap<String, u64>>,
}

impl WishlistService {
    /// Legge la lista salvata; se manca o e' rovinata parte spenta e vuota.
    pub fn open(path: &Path) -> Arc<WishlistService> {
        let config = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Wishlist>(&text).ok())
            .map(Wishlist::normalized)
            .filter(|w| w.validate().is_ok())
            .unwrap_or_default();
        Arc::new(WishlistService { path: path.to_path_buf(), config: Mutex::new(config), recent: Mutex::new(VecDeque::new()), last_alert: Mutex::new(HashMap::new()) })
    }

    pub fn config(&self) -> Wishlist {
        lock(&self.config).clone()
    }

    /// Valida, salva su disco e usa subito la lista nuova.
    pub fn set_config(&self, config: Wishlist) -> Result<Wishlist, String> {
        let config = config.normalized();
        config.validate()?;
        let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &self.path)).map_err(|e| format!("salvataggio: {e}"))?;
        *lock(&self.config) = config.clone();
        Ok(config)
    }

    /// Si deve avvisare per questa stazione? Falso se e' gia' stata avvisata da meno di `cooldown_min` minuti; altrimenti segna l'ora.
    pub fn should_alert(&self, call: &str, now_ms: u64) -> bool {
        let wait = u64::from(lock(&self.config).cooldown_min) * 60_000;
        let mut last = lock(&self.last_alert);
        let key = base_call(&call.to_uppercase()).to_string();
        if last.get(&key).is_some_and(|t| now_ms.saturating_sub(*t) < wait) {
            return false;
        }
        last.insert(key, now_ms);
        // la tabella non cresce senza limite: si buttano le voci scadute quando diventa grande
        if last.len() > 2000 {
            last.retain(|_, t| now_ms.saturating_sub(*t) < wait);
        }
        true
    }

    pub fn remember(&self, alert: WantedAlert) {
        let mut recent = lock(&self.recent);
        recent.push_front(alert);
        recent.truncate(MAX_RECENT);
    }

    /// Gli avvisi recenti, dal piu' nuovo.
    pub fn recent(&self) -> Vec<WantedAlert> {
        lock(&self.recent).iter().cloned().collect()
    }
}

/// Il testo breve di un avviso, per il telefono.
pub fn alert_title(alert: &WantedAlert) -> String {
    let band = alert.band.as_deref().map(|b| format!(" {b}")).unwrap_or_default();
    format!("{} desiderata{band}", alert.call)
}

pub fn alert_body(alert: &WantedAlert) -> String {
    let mut parts = vec![alert.text.clone()];
    if let Some(c) = &alert.country {
        parts.push(c.clone());
    }
    if let Some(f) = alert.freq_hz {
        parts.push(format!("{:.1} kHz", f as f64 / 1000.0));
    }
    if let Some(s) = alert.snr {
        parts.push(format!("{s} dB"));
    }
    parts.push(format!("({})", match alert.reason.as_str() {
        "call" => "nominativo",
        "prefix" => "prefisso",
        "country" => "paese",
        "grid" => "griglia",
        other => other,
    }));
    parts.into_iter().filter(|p| !p.trim().is_empty()).collect::<Vec<_>>().join(" · ")
}

/// Manda l'avviso a un server ntfy (o a qualunque indirizzo che accetti un POST di testo). Bloccante: da un thread dedicato.
pub fn send_notification(url: &str, title: &str, body: &str) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(8))).build().into();
    agent
        .post(url)
        .header("Title", title)
        .header("Tags", "satellite_antenna")
        .header("Priority", "high")
        .send(body)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Wishlist {
        Wishlist {
            enabled: true,
            calls: vec!["K1ABC".into(), "3Y0J".into()],
            prefixes: vec!["VP8".into(), "ZL9".into()],
            countries: vec!["Bulgaria".into()],
            grids: vec!["JN45".into()],
            ..Wishlist::default()
        }
        .normalized()
    }

    #[test]
    fn matches_calls_with_prefix_or_suffix_and_in_the_order_call_prefix_country_grid() {
        let w = list();
        assert_eq!(w.reason("K1ABC", None, None, None), Some(Reason::Call));
        assert_eq!(w.reason("k1abc/p", None, None, None), Some(Reason::Call));
        assert_eq!(w.reason("F/K1ABC", None, None, None), Some(Reason::Call));
        assert_eq!(w.reason("VP8LP", None, None, None), Some(Reason::Prefix));
        assert_eq!(w.reason("LZ1AB", None, Some("bulgaria"), None), Some(Reason::Country));
        assert_eq!(w.reason("IU1XYZ", Some("JN45ab"), None, None), Some(Reason::Grid));
        // il nominativo vince sul paese
        assert_eq!(w.reason("K1ABC", Some("JN45"), Some("Bulgaria"), None), Some(Reason::Call));
        assert_eq!(w.reason("W1AW", Some("FN31"), Some("United States"), None), None);
    }

    #[test]
    fn a_call_is_not_matched_by_part_of_another_one() {
        let w = list();
        assert_eq!(w.reason("K1ABCD", None, None, None), None);
        assert_eq!(w.reason("AK1ABC", None, None, None), None);
        assert_eq!(w.reason("", None, None, None), None);
    }

    #[test]
    fn nothing_matches_when_it_is_off_and_the_band_filter_only_excludes_known_bands() {
        let mut w = list();
        w.enabled = false;
        assert_eq!(w.reason("K1ABC", None, None, None), None);
        w.enabled = true;
        w.bands = vec!["20m".into()];
        assert_eq!(w.reason("K1ABC", None, None, Some("20m")), Some(Reason::Call));
        assert_eq!(w.reason("K1ABC", None, None, Some("40m")), None);
        assert_eq!(w.reason("K1ABC", None, None, None), Some(Reason::Call));
    }

    #[test]
    fn normalization_trims_uppercases_and_drops_duplicates() {
        let w = Wishlist { calls: vec![" k1abc ".into(), "K1ABC".into(), "".into()], bands: vec![" 20M".into()], grids: vec!["jn45".into()], ..Wishlist::default() }.normalized();
        assert_eq!(w.calls, ["K1ABC"]);
        assert_eq!(w.bands, ["20m"]);
        assert_eq!(w.grids, ["JN45"]);
    }

    #[test]
    fn validation_refuses_what_cannot_work() {
        let ok = list();
        assert!(ok.validate().is_ok());
        for bad in [
            Wishlist { calls: vec!["K1 ABC".into()], ..ok.clone() },
            Wishlist { calls: vec!["A".repeat(16)], ..ok.clone() },
            Wishlist { prefixes: vec!["V;P".into()], ..ok.clone() },
            Wishlist { grids: vec!["JN4".into()], ..ok.clone() },
            Wishlist { grids: vec!["ZZ99".into()], ..ok.clone() },
            Wishlist { cooldown_min: 0, ..ok.clone() },
            Wishlist { cooldown_min: 241, ..ok.clone() },
            Wishlist { notify_url: "ftp://x".into(), ..ok.clone() },
            Wishlist { notify_url: "https://ntfy.sh/a b".into(), ..ok.clone() },
            Wishlist { calls: (0..=MAX_ITEMS).map(|i| format!("K{i}A")).collect(), ..ok.clone() },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
        assert!(Wishlist { notify_url: "https://ntfy.sh/aurora-prova".into(), ..ok }.validate().is_ok());
    }

    #[test]
    fn the_same_station_is_not_announced_again_before_the_cooldown() {
        let dir = std::env::temp_dir().join(format!("aurora-wish-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let svc = WishlistService::open(&dir.join("wishlist.json"));
        svc.set_config(Wishlist { cooldown_min: 10, ..Wishlist::default() }).unwrap();
        assert!(svc.should_alert("K1ABC", 1_000));
        assert!(!svc.should_alert("K1ABC/P", 1_000 + 9 * 60_000), "stessa stazione, anche col suffisso");
        assert!(svc.should_alert("K1ABC", 1_000 + 10 * 60_000));
        assert!(svc.should_alert("W1AW", 2_000), "un'altra stazione avvisa subito");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_list_is_saved_and_read_back_and_a_bad_file_gives_an_empty_list() {
        let dir = std::env::temp_dir().join(format!("aurora-wish2-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wishlist.json");
        let svc = WishlistService::open(&path);
        assert_eq!(svc.config(), Wishlist::default());
        svc.set_config(list()).unwrap();
        assert_eq!(WishlistService::open(&path).config(), list());
        assert!(svc.set_config(Wishlist { grids: vec!["no".into()], ..list() }).is_err());
        std::fs::write(&path, "non json").unwrap();
        assert_eq!(WishlistService::open(&path).config(), Wishlist::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recent_alerts_are_newest_first_and_capped() {
        let dir = std::env::temp_dir().join(format!("aurora-wish3-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let svc = WishlistService::open(&dir.join("wishlist.json"));
        for i in 0..(MAX_RECENT + 5) {
            svc.remember(WantedAlert { call: format!("K{i}A"), reason: "call".into(), source: "decode".into(), text: String::new(), freq_hz: None, band: None, snr: None, country: None, grid: None, time_ms: i as u64 });
        }
        let recent = svc.recent();
        assert_eq!(recent.len(), MAX_RECENT);
        assert_eq!(recent[0].call, format!("K{}A", MAX_RECENT + 4));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_notification_text_is_short_and_says_why() {
        let a = WantedAlert { call: "3Y0J".into(), reason: "call".into(), source: "cluster".into(), text: "CQ DX up".into(), freq_hz: Some(14_195_000), band: Some("20m".into()), snr: None, country: Some("Bouvet".into()), grid: None, time_ms: 0 };
        assert_eq!(alert_title(&a), "3Y0J desiderata 20m");
        assert_eq!(alert_body(&a), "CQ DX up · Bouvet · 14195.0 kHz · (nominativo)");
    }
}
