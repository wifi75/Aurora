//! Controllo delle nuove versioni di Aurora: legge l'ultima release sul repository (GitHub), la confronta con la versione in uso e
//! ricorda l'esito. Non scarica ne' installa nulla: chi vuole aggiornare usa l'indirizzo della release.
//!
//! Con un repository privato serve un token di sola lettura (creato da chi usa Aurora): si scrive nelle impostazioni, si tiene solo nel
//! file di dati del programma, non si restituisce mai all'interfaccia e non si scrive nei registri.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use serde::{Deserialize, Serialize};

const DEFAULT_API: &str = "https://api.github.com";

/// Le impostazioni degli aggiornamenti, salvate su disco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    pub enabled: bool,
    /// `proprietario/nome` del repository su GitHub.
    pub repo: String,
    /// Ogni quante ore controllare (1-168).
    pub interval_hours: u32,
    /// Anche le versioni di prova (`-rc`, `-beta`).
    pub prereleases: bool,
    /// Token di sola lettura per un repository privato. Non esce mai dal programma: l'interfaccia vede solo se c'e'.
    #[serde(skip_serializing)]
    pub token: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        UpdateConfig { enabled: true, repo: "wifi75/Aurora".into(), interval_hours: 6, prereleases: false, token: String::new() }
    }
}

fn valid_repo(repo: &str) -> bool {
    let Some((owner, name)) = repo.split_once('/') else { return false };
    let ok = |s: &str| !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    ok(owner) && ok(name) && !name.contains('/')
}

impl UpdateConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_repo(&self.repo) {
            return Err("il repository va scritto proprietario/nome".into());
        }
        if !(1..=168).contains(&self.interval_hours) {
            return Err("il controllo va da 1 a 168 ore".into());
        }
        if self.token.len() > 255 || self.token.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("token non valido".into());
        }
        Ok(())
    }
}

/// Un file allegato a una release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub url: String,
}

/// Una versione pubblicata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Release {
    pub tag: String,
    pub version: String,
    pub name: String,
    /// Le note della release, accorciate.
    pub notes: String,
    pub url: String,
    pub published_at: String,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

/// Esito dell'ultimo controllo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    /// La versione in uso.
    pub current: String,
    pub enabled: bool,
    pub repo: String,
    pub interval_hours: u32,
    pub prereleases: bool,
    /// C'e' un token salvato (il valore non si restituisce).
    pub has_token: bool,
    /// Quando e' stato l'ultimo controllo (millisecondi Unix), 0 se mai.
    pub checked_at_ms: u64,
    /// L'ultima release trovata, se e' piu' nuova della versione in uso.
    pub available: Option<Release>,
    /// L'ultima release trovata in assoluto.
    pub latest: Option<Release>,
    pub error: Option<String>,
}

/// «v1.2.3», «1.2.3» o «1.2.3-rc1» in (maggiore, minore, patch, prova).
pub fn parse_version(text: &str) -> Option<(u64, u64, u64, bool)> {
    let t = text.trim().trim_start_matches(['v', 'V']);
    let (core, pre) = match t.split_once(['-', '+']) {
        Some((core, rest)) => (core, t.contains('-') && !rest.is_empty() && t.find('-').is_some_and(|d| t.find('+').is_none_or(|p| d < p))),
        None => (t, false),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch, pre))
}

/// `latest` e' una versione piu' nuova di `current`? A parita' di numeri una versione di prova viene prima di quella finale.
pub fn is_newer(latest: &str, current: &str) -> bool {
    let (Some(l), Some(c)) = (parse_version(latest), parse_version(current)) else { return false };
    if (l.0, l.1, l.2) != (c.0, c.1, c.2) {
        return (l.0, l.1, l.2) > (c.0, c.1, c.2);
    }
    // stessi numeri: la finale batte la prova in uso, non il contrario
    c.3 && !l.3
}

fn shorten(text: &str, max: usize) -> String {
    let t = text.trim();
    if t.chars().count() <= max {
        return t.to_string();
    }
    let cut: String = t.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

/// Legge una release dalla risposta dell'API di GitHub. `None` se manca il numero di versione o e' una bozza.
pub fn parse_release(json: &serde_json::Value) -> Option<Release> {
    if json["draft"].as_bool() == Some(true) {
        return None;
    }
    let tag = json["tag_name"].as_str()?.to_string();
    parse_version(&tag)?;
    let assets = json["assets"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|a| {
                    Some(Asset { name: a["name"].as_str()?.to_string(), size: a["size"].as_u64().unwrap_or(0), url: a["browser_download_url"].as_str()?.to_string() })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Release {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        name: json["name"].as_str().unwrap_or(&tag).to_string(),
        notes: shorten(json["body"].as_str().unwrap_or(""), 1500),
        url: json["html_url"].as_str().unwrap_or("").to_string(),
        published_at: json["published_at"].as_str().unwrap_or("").to_string(),
        prerelease: json["prerelease"].as_bool().unwrap_or(false),
        tag,
        assets,
    })
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

struct State {
    /// L'ultima versione per cui si e' gia' avvisato (salvata a parte), per non ripetere l'avviso a ogni riavvio.
    notified: String,
    checked_at_ms: u64,
    latest: Option<Release>,
    error: Option<String>,
}

/// Il servizio: le impostazioni, il file dove si salvano e l'esito dell'ultimo controllo.
pub struct UpdateService {
    path: PathBuf,
    current: String,
    api: String,
    config: Mutex<UpdateConfig>,
    state: Mutex<State>,
    notified_path: PathBuf,
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

impl UpdateService {
    /// Legge le impostazioni salvate; se mancano o sono rovinate parte con quelle predefinite. `current` e' la versione in uso.
    pub fn open(path: &Path, current: &str) -> Arc<UpdateService> {
        Self::open_with_api(path, current, DEFAULT_API)
    }

    /// Come `open` ma con un altro indirizzo per l'API (le prove usano un server locale).
    pub fn open_with_api(path: &Path, current: &str, api: &str) -> Arc<UpdateService> {
        let mut config = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .map(|v| {
                let mut c: UpdateConfig = serde_json::from_value(v.clone()).unwrap_or_default();
                // il token si salva nel file ma non esce mai dall'API: `skip_serializing` lo toglie solo in uscita, qui si rilegge a mano
                c.token = v["token"].as_str().unwrap_or("").to_string();
                c
            })
            .filter(|c| c.validate().is_ok())
            .unwrap_or_default();
        config.repo = config.repo.trim().to_string();
        Arc::new(UpdateService { path: path.to_path_buf(), current: current.to_string(), api: api.trim_end_matches('/').to_string(), config: Mutex::new(config), state: Mutex::new(State { notified: std::fs::read_to_string(path.with_extension("notified")).unwrap_or_default().trim().to_string(), checked_at_ms: 0, latest: None, error: None }), notified_path: path.with_extension("notified") })
    }

    pub fn config(&self) -> UpdateConfig {
        lock(&self.config).clone()
    }

    /// Salva le impostazioni nuove. `token`: `None` lascia quello che c'e', `Some("")` lo toglie.
    pub fn set_config(&self, mut config: UpdateConfig, token: Option<String>) -> Result<(), String> {
        config.repo = config.repo.trim().to_string();
        config.token = match token {
            Some(t) => t.trim().to_string(),
            None => lock(&self.config).token.clone(),
        };
        config.validate()?;
        // il file contiene il token (serde lo salta in uscita, quindi lo si scrive a mano)
        let mut value = serde_json::to_value(&config).map_err(|e| e.to_string())?;
        value["token"] = serde_json::Value::String(config.token.clone());
        let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &self.path)).map_err(|e| format!("salvataggio: {e}"))?;
        *lock(&self.config) = config;
        Ok(())
    }

    /// Si e' gia' avvisato per questa versione? Se no la ricorda (anche su disco) e dice `true`: va avvisato adesso.
    pub fn first_notice(&self, version: &str) -> bool {
        let mut state = lock(&self.state);
        if state.notified == version {
            return false;
        }
        state.notified = version.to_string();
        if let Err(e) = std::fs::write(&self.notified_path, version) {
            tracing::warn!("versione avvisata non salvata: {e}");
        }
        true
    }

    /// Lo stato da mostrare: impostazioni (senza il token) e ultimo esito.
    pub fn status(&self) -> Status {
        let config = lock(&self.config).clone();
        let state = lock(&self.state);
        let available = state.latest.clone().filter(|r| is_newer(&r.version, &self.current));
        Status {
            current: self.current.clone(),
            enabled: config.enabled,
            repo: config.repo,
            interval_hours: config.interval_hours,
            prereleases: config.prereleases,
            has_token: !config.token.is_empty(),
            checked_at_ms: state.checked_at_ms,
            available,
            latest: state.latest.clone(),
            error: state.error.clone(),
        }
    }

    /// Chiede a GitHub l'ultima release e ricorda l'esito. Bloccante: da un thread dedicato.
    pub fn check(&self) -> Status {
        let config = self.config();
        let result = self.fetch(&config);
        {
            let mut state = lock(&self.state);
            state.checked_at_ms = unix_ms();
            match result {
                Ok(release) => {
                    state.latest = release;
                    state.error = None;
                }
                Err(e) => {
                    tracing::warn!("controllo degli aggiornamenti non riuscito: {e}");
                    state.error = Some(e);
                }
            }
        }
        self.status()
    }

    fn fetch(&self, config: &UpdateConfig) -> Result<Option<Release>, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).http_status_as_error(false).build().into();
        // con le versioni di prova si guarda l'elenco (la piu' recente per prima); altrimenti basta «latest»
        let url = if config.prereleases { format!("{}/repos/{}/releases?per_page=5", self.api, config.repo) } else { format!("{}/repos/{}/releases/latest", self.api, config.repo) };
        let mut request = agent.get(&url).header("User-Agent", "Aurora-updater").header("Accept", "application/vnd.github+json");
        if !config.token.is_empty() {
            request = request.header("Authorization", &format!("Bearer {}", config.token));
        }
        let mut response = request.call().map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        match status {
            200 => {}
            // nessuna release pubblicata ancora (o repository privato senza token): non e' un errore del programma
            404 => return Err(if config.token.is_empty() { "nessuna release trovata (se il repository e' privato serve un token)".into() } else { "nessuna release trovata o token senza accesso al repository".into() }),
            401 | 403 => return Err("accesso rifiutato da GitHub (token scaduto o limite di richieste)".into()),
            other => return Err(format!("GitHub ha risposto {other}")),
        }
        let text = response.body_mut().read_to_string().map_err(|e| format!("risposta non leggibile: {e}"))?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("risposta non leggibile: {e}"))?;
        let releases: Vec<&serde_json::Value> = match json.as_array() {
            Some(list) => list.iter().collect(),
            None => vec![&json],
        };
        Ok(releases.into_iter().filter_map(parse_release).find(|r| config.prereleases || !r.prerelease))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-upd-{}-{name}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("update.json")
    }

    /// Un server finto che risponde una volta con la risposta data e ricorda la richiesta.
    fn serve(status: &'static str, body: String) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                tx.send(String::from_utf8_lossy(&buf[..n]).to_string()).ok();
                let reply = format!("HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                stream.write_all(reply.as_bytes()).ok();
            }
        });
        (format!("http://{addr}"), rx)
    }

    fn release_json(tag: &str) -> String {
        serde_json::json!({
            "tag_name": tag, "name": format!("Aurora {tag}"), "body": "Novita' della versione", "html_url": "https://github.com/x/y/releases/tag/t",
            "published_at": "2026-10-24T10:00:00Z", "prerelease": false, "draft": false,
            "assets": [{ "name": "Aurora-setup.exe", "size": 1234, "browser_download_url": "https://example.test/Aurora-setup.exe" }]
        })
        .to_string()
    }

    #[test]
    fn versions_compare_by_number_and_a_release_candidate_comes_before_the_final() {
        assert_eq!(parse_version("v0.9.1"), Some((0, 9, 1, false)));
        assert_eq!(parse_version("1.2.3-rc1"), Some((1, 2, 3, true)));
        assert_eq!(parse_version("1.2.3+build5"), Some((1, 2, 3, false)));
        for bad in ["", "1.2", "1.2.3.4", "a.b.c", "v", "latest"] {
            assert_eq!(parse_version(bad), None, "{bad}");
        }
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("0.9.1", "0.9.0"));
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("0.8.9", "0.9.0"));
        assert!(is_newer("0.9.0", "0.9.0-rc1"));
        assert!(!is_newer("0.9.0-rc1", "0.9.0"));
        assert!(!is_newer("boh", "0.9.0"));
    }

    #[test]
    fn reads_a_github_release_and_skips_drafts_and_untagged_ones() {
        let r = parse_release(&serde_json::from_str(&release_json("v0.9.1")).unwrap()).unwrap();
        assert_eq!((r.version.as_str(), r.tag.as_str()), ("0.9.1", "v0.9.1"));
        assert_eq!(r.assets, vec![Asset { name: "Aurora-setup.exe".into(), size: 1234, url: "https://example.test/Aurora-setup.exe".into() }]);
        assert!(parse_release(&serde_json::json!({ "tag_name": "v1.0.0", "draft": true })).is_none());
        assert!(parse_release(&serde_json::json!({ "name": "senza tag" })).is_none());
        assert!(parse_release(&serde_json::json!({ "tag_name": "nightly" })).is_none());
    }

    #[test]
    fn long_notes_are_shortened() {
        let r = parse_release(&serde_json::json!({ "tag_name": "v1.0.0", "body": "x".repeat(5000) })).unwrap();
        assert!(r.notes.chars().count() <= 1501);
        assert!(r.notes.ends_with('…'));
    }

    #[test]
    fn config_validation_and_the_token_never_comes_out() {
        assert!(UpdateConfig::default().validate().is_ok());
        for bad in [
            UpdateConfig { repo: "senza-barra".into(), ..Default::default() },
            UpdateConfig { repo: "a/b/c".into(), ..Default::default() },
            UpdateConfig { repo: "a b/c".into(), ..Default::default() },
            UpdateConfig { repo: "a/..;rm".into(), ..Default::default() },
            UpdateConfig { interval_hours: 0, ..Default::default() },
            UpdateConfig { interval_hours: 169, ..Default::default() },
            UpdateConfig { token: "con spazio".into(), ..Default::default() },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
        let json = serde_json::to_string(&UpdateConfig { token: "ghp_segreto".into(), ..Default::default() }).unwrap();
        assert!(!json.contains("ghp_segreto") && !json.contains("token"), "{json}");
    }

    #[test]
    fn saves_the_settings_keeps_the_token_when_not_given_and_never_shows_it() {
        let path = temp("config");
        let svc = UpdateService::open(&path, "0.9.0");
        svc.set_config(UpdateConfig { interval_hours: 12, ..Default::default() }, Some("ghp_segreto".into())).unwrap();
        assert!(svc.status().has_token);
        assert!(!serde_json::to_string(&svc.status()).unwrap().contains("ghp_segreto"));
        // un salvataggio senza token lascia quello che c'e'
        svc.set_config(UpdateConfig { interval_hours: 24, ..Default::default() }, None).unwrap();
        assert_eq!(svc.config().token, "ghp_segreto");
        // si rilegge dal file, token compreso
        let again = UpdateService::open(&path, "0.9.0");
        assert_eq!((again.config().interval_hours, again.config().token.as_str()), (24, "ghp_segreto"));
        // e si toglie con una stringa vuota
        svc.set_config(UpdateConfig::default(), Some(String::new())).unwrap();
        assert!(!svc.status().has_token);
        assert!(svc.set_config(UpdateConfig { repo: "no".into(), ..Default::default() }, None).is_err());
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn a_damaged_file_gives_the_defaults() {
        let path = temp("bad");
        std::fs::write(&path, "non json").unwrap();
        assert_eq!(UpdateService::open(&path, "0.9.0").config(), UpdateConfig::default());
        std::fs::write(&path, r#"{"repo":"a/b/c"}"#).unwrap();
        assert_eq!(UpdateService::open(&path, "0.9.0").config(), UpdateConfig::default());
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn finds_a_newer_release_on_the_server_and_sends_the_token_and_the_repo() {
        let (api, request) = serve("200 OK", release_json("v0.10.0"));
        let svc = UpdateService::open_with_api(&temp("newer"), "0.9.0", &api);
        svc.set_config(UpdateConfig { repo: "wifi75/Aurora".into(), ..Default::default() }, Some("ghp_x".into())).unwrap();
        let status = svc.check();
        assert_eq!(status.error, None);
        assert_eq!(status.available.as_ref().map(|r| r.version.as_str()), Some("0.10.0"));
        assert!(status.checked_at_ms > 0);
        let raw = request.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(raw.starts_with("GET /repos/wifi75/Aurora/releases/latest "), "{raw}");
        assert!(raw.to_lowercase().contains("authorization: bearer ghp_x"), "{raw}");
    }

    #[test]
    fn the_same_or_an_older_version_is_not_an_update() {
        let (api, _) = serve("200 OK", release_json("v0.9.0"));
        let svc = UpdateService::open_with_api(&temp("same"), "0.9.0", &api);
        let status = svc.check();
        assert_eq!(status.error, None);
        assert!(status.available.is_none());
        assert_eq!(status.latest.as_ref().map(|r| r.version.as_str()), Some("0.9.0"));
    }

    #[test]
    fn explains_a_missing_release_and_a_refused_token() {
        let (api, _) = serve("404 Not Found", "{}".into());
        let svc = UpdateService::open_with_api(&temp("404"), "0.9.0", &api);
        let status = svc.check();
        assert!(status.error.as_deref().is_some_and(|e| e.contains("privato")), "{:?}", status.error);
        let (api, _) = serve("403 Forbidden", "{}".into());
        let svc = UpdateService::open_with_api(&temp("403"), "0.9.0", &api);
        assert!(svc.check().error.as_deref().is_some_and(|e| e.contains("rifiutato")));
    }

    #[test]
    fn prereleases_are_only_taken_when_asked() {
        let list = serde_json::json!([
            { "tag_name": "v0.10.0-rc1", "prerelease": true, "html_url": "u", "assets": [] },
            { "tag_name": "v0.9.5", "prerelease": false, "html_url": "u", "assets": [] }
        ])
        .to_string();
        let (api, _) = serve("200 OK", list.clone());
        let svc = UpdateService::open_with_api(&temp("pre"), "0.9.0", &api);
        svc.set_config(UpdateConfig { prereleases: true, ..Default::default() }, None).unwrap();
        assert_eq!(svc.check().latest.map(|r| r.version), Some("0.10.0-rc1".to_string()));
    }

    #[test]
    fn an_unreachable_server_is_an_error_not_a_crash() {
        let svc = UpdateService::open_with_api(&temp("down"), "0.9.0", "http://127.0.0.1:1");
        let status = svc.check();
        assert!(status.error.is_some());
        assert!(status.available.is_none());
    }
}
