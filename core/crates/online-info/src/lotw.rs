//! Chi usa LoTW: l'elenco pubblico dell'ARRL (`lotw-user-activity.csv`), una riga per nominativo con la data dell'ultimo invio di log:
//! `NOMINATIVO,AAAA-MM-GG,hh:mm:ss`. Si scarica all'avvio se la copia nella cartella dei dati ha piu' di 24 ore e poi una volta al giorno.
//! Se il download fallisce resta la copia vecchia e l'errore va nel registro: non blocca mai nulla. Il sito vede solo l'indirizzo IP.

use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

/// L'elenco pubblico dell'ARRL.
pub const URL: &str = "https://lotw.arrl.org/lotw-user-activity.csv";
/// Nome della copia nella cartella dei dati.
pub const CACHE_FILE: &str = "lotw-user-activity.csv";
/// Dopo quanto la copia si considera vecchia e si riscarica.
pub const MAX_AGE: Duration = Duration::from_secs(24 * 3600);
/// Sotto questo numero di righe il file scaricato si considera sbagliato (l'elenco ne ha oltre centomila).
const MIN_ENTRIES: usize = 1000;

/// Nominativi che usano LoTW con la data dell'ultimo invio (come numero `AAAAMMGG`, per stare in poca memoria).
#[derive(Debug, Default)]
pub struct LotwUsers {
    map: HashMap<String, u32>,
}

/// L'elenco condiviso: chi lo scarica lo sostituisce, chi decodifica lo interroga.
pub type SharedLotw = Arc<RwLock<LotwUsers>>;

impl LotwUsers {
    /// Legge il CSV; le righe che non tornano si saltano.
    pub fn parse(csv: &str) -> LotwUsers {
        let mut map = HashMap::new();
        for line in csv.lines() {
            let mut fields = line.split(',');
            let (Some(call), Some(date)) = (fields.next(), fields.next()) else { continue };
            let call = call.trim().to_ascii_uppercase();
            if call.is_empty() {
                continue;
            }
            if let Some(ymd) = parse_date(date.trim()) {
                map.insert(call, ymd);
            }
        }
        LotwUsers { map }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Data dell'ultimo invio (`AAAA-MM-GG`) di un nominativo. Un nominativo con prefisso o suffisso (`IU3ABC/P`, `EA/IU3ABC`) se non c'e'
    /// intero si cerca per la parte che sembra il nominativo di base.
    pub fn last_upload(&self, call: &str) -> Option<String> {
        let call = call.trim().to_ascii_uppercase();
        let found = self.map.get(&call).or_else(|| {
            let mut parts: Vec<&str> = call.split('/').filter(|p| p.len() >= 3 && p.chars().any(|c| c.is_ascii_digit())).collect();
            parts.sort_by_key(|p| std::cmp::Reverse(p.len()));
            parts.into_iter().find_map(|p| self.map.get(p))
        })?;
        Some(format!("{:04}-{:02}-{:02}", found / 10_000, found / 100 % 100, found % 100))
    }
}

/// `AAAA-MM-GG` in `AAAAMMGG`.
fn parse_date(s: &str) -> Option<u32> {
    let mut it = s.split('-');
    let (y, m, d) = (it.next()?.parse::<u32>().ok()?, it.next()?.parse::<u32>().ok()?, it.next()?.parse::<u32>().ok()?);
    if it.next().is_some() || !(1990..=2200).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(y * 10_000 + m * 100 + d)
}

fn download(app_version: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(90))).http_status_as_error(false).build().into();
    let mut response = agent.get(URL).header("User-Agent", &format!("Aurora/{app_version}")).call().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("il servizio ha risposto {status}"));
    }
    // l'elenco pesa qualche megabyte: il limite di lettura di fabbrica (10 MB) e' troppo stretto per gli anni a venire
    response.body_mut().with_config().limit(100 * 1024 * 1024).read_to_string().map_err(|e| e.to_string())
}

fn cache_is_fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < MAX_AGE)
}

/// Legge la copia nella cartella dei dati, se c'e' e se l'elenco e' vuoto (file locale: si puo' chiamare in modo sincrono all'avvio,
/// cosi' le prime decodifiche gia' hanno l'elenco). Restituisce vero se l'elenco e' stato caricato.
pub fn load_cache(shared: &SharedLotw, data_dir: &Path) -> bool {
    if !shared.read().map(|u| u.is_empty()).unwrap_or(true) {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(data_dir.join(CACHE_FILE)) else { return false };
    let users = LotwUsers::parse(&text);
    if users.len() < MIN_ENTRIES {
        return false;
    }
    tracing::info!("elenco LoTW letto dalla copia: {} nominativi", users.len());
    match shared.write() {
        Ok(mut w) => {
            *w = users;
            true
        }
        Err(_) => false,
    }
}

/// Carica la copia nella cartella dei dati (se c'e') e, se ha piu' di 24 ore o manca, prova a riscaricarla. Bloccante: va in un thread.
/// Un errore di rete lascia la copia vecchia.
pub fn refresh(shared: &SharedLotw, data_dir: &Path, app_version: &str) {
    let path: PathBuf = data_dir.join(CACHE_FILE);
    load_cache(shared, data_dir);
    if cache_is_fresh(&path) {
        return;
    }
    match download(app_version) {
        Ok(text) => {
            let users = LotwUsers::parse(&text);
            if users.len() < MIN_ENTRIES {
                tracing::warn!("elenco LoTW scaricato ma troppo corto ({} righe): resta la copia vecchia", users.len());
                return;
            }
            tracing::info!("elenco LoTW scaricato: {} nominativi", users.len());
            let tmp = path.with_extension("csv.tmp");
            let saved = std::fs::File::create(&tmp).and_then(|mut f| f.write_all(text.as_bytes())).and_then(|()| std::fs::rename(&tmp, &path));
            if let Err(e) = saved {
                tracing::warn!("copia dell'elenco LoTW non salvata: {e}");
            }
            if let Ok(mut w) = shared.write() {
                *w = users;
            }
        }
        Err(e) => tracing::warn!("elenco LoTW non scaricato, resta la copia vecchia: {e}"),
    }
}

/// Avvia il lavoro in un thread: subito all'avvio e poi ogni ora controlla se la copia ha piu' di 24 ore.
pub fn spawn(shared: SharedLotw, data_dir: PathBuf, app_version: String) {
    // la copia su disco si legge subito, prima che il motore pubblichi le prime decodifiche; nel thread resta solo il download
    load_cache(&shared, &data_dir);
    let spawned = std::thread::Builder::new().name("lotw".into()).spawn(move || loop {
        refresh(&shared, &data_dir, &app_version);
        std::thread::sleep(Duration::from_secs(3600));
    });
    if let Err(e) = spawned {
        tracing::warn!("elenco LoTW non avviato: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &str = "IU3ABC,2026-10-28,14:03:11\nea4xyz,2024-01-05,00:00:01\nPORCARIA\nK1ABC,data-sbagliata,12:00:00\nIO91,2026-13-40,00:00:00\n\nDL1ABC,2020-02-29,23:59:59\n";

    #[test]
    fn parses_lines_and_skips_bad_ones() {
        let u = LotwUsers::parse(CSV);
        assert_eq!(u.len(), 3);
        assert_eq!(u.last_upload("IU3ABC").as_deref(), Some("2026-10-28"));
        assert_eq!(u.last_upload("EA4XYZ").as_deref(), Some("2024-01-05"));
        assert_eq!(u.last_upload("K1ABC"), None);
        assert_eq!(u.last_upload("W9ZZZ"), None);
    }

    #[test]
    fn load_cache_reads_the_local_copy_synchronously() {
        let dir = std::env::temp_dir().join(format!("aurora-lotw-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let shared: SharedLotw = Arc::default();
        assert!(!load_cache(&shared, &dir), "senza file non carica");
        let mut csv = String::from(CSV);
        for i in 0..MIN_ENTRIES {
            csv.push_str(&format!("N{i}ZZ,2025-01-01,00:00:00\n"));
        }
        std::fs::write(dir.join(CACHE_FILE), csv).unwrap();
        assert!(load_cache(&shared, &dir));
        assert_eq!(shared.read().unwrap().last_upload("IU3ABC").as_deref(), Some("2026-10-28"));
        assert!(!load_cache(&shared, &dir), "gia' carico: non rilegge");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn portable_calls_fall_back_to_the_base_call() {
        let u = LotwUsers::parse(CSV);
        assert_eq!(u.last_upload("IU3ABC/P").as_deref(), Some("2026-10-28"));
        assert_eq!(u.last_upload("EA/IU3ABC").as_deref(), Some("2026-10-28"));
        assert_eq!(u.last_upload("dl1abc/QRP").as_deref(), Some("2020-02-29"));
        assert_eq!(u.last_upload("/P"), None);
    }
}
