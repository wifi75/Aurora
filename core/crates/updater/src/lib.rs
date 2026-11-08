//! Controllo delle nuove versioni di Aurora: legge l'ultima release sul repository (GitHub), la confronta con la versione in uso e
//! ricorda l'esito. Sa anche preparare l'aggiornamento: scarica in anticipo il pacchetto e la sua firma, li controlla e tiene il file
//! pronto; l'avvio dell'installazione resta a chi sa farlo (il programma Windows con l'installatore NSIS, il programma macOS sostituendo
//! Aurora.app).
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
        UpdateConfig { enabled: true, repo: "wifi75/Aurora".into(), interval_hours: 3, prereleases: false, token: String::new() }
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
    /// Indirizzo pubblico di scaricamento (non funziona per un repository privato).
    pub url: String,
    /// Indirizzo dell'API di GitHub per lo stesso file: con un token scarica anche da un repository privato.
    #[serde(skip_serializing)]
    pub api_url: String,
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

/// A che punto e' lo scaricamento del pacchetto d'installazione (per la barra di «Installa e riavvia»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DownloadProgress {
    pub file: String,
    /// Byte gia' arrivati.
    pub done: u64,
    /// Byte in tutto (0 se non si sa).
    pub total: u64,
}

/// A che punto e' la preparazione dell'aggiornamento (scaricamento in anticipo, controllo della firma, installazione).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Niente in corso.
    #[default]
    Idle,
    /// Si scarica il pacchetto.
    Downloading,
    /// Si controlla la firma.
    Verifying,
    /// Pacchetto completo e firmato: «Installa» non riscarica.
    Ready,
    /// L'installatore sta partendo (Aurora si chiude).
    Installing,
    /// Scaricamento o firma non riusciti: si puo' riprovare.
    Error,
}

/// Lo stato della preparazione come lo vede l'interfaccia (barra della testata, cartolina del tray).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Stage {
    pub phase: Phase,
    /// La versione che si sta preparando.
    pub version: String,
    pub file: String,
    /// Byte gia' arrivati.
    pub done: u64,
    /// Byte in tutto (0 se non si sa).
    pub total: u64,
    /// Percentuale scaricata (`None` se il totale non si sa).
    pub percent: Option<u8>,
    /// Velocita' media dello scaricamento in corso (byte al secondo).
    pub speed_bps: u64,
    /// Secondi che mancano alla fine dello scaricamento (`None` se non si sa).
    pub eta_s: Option<u64>,
    /// Il motivo, in fase `error`.
    pub error: Option<String>,
}

/// Il pacchetto pronto da installare: completo e con la firma controllata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub version: String,
    pub path: PathBuf,
}

/// Il pacchetto di aggiornamento della versione macOS: Aurora.app compressa (come gli «updater artifacts» di Tauri), con nome fisso.
pub const MAC_PACKAGE: &str = "Aurora-macOS.app.tar.gz";

/// Il tipo di pacchetto di aggiornamento che un sistema sa installare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKind {
    /// L'installatore NSIS di Windows (`*-setup.exe`).
    WindowsSetup,
    /// Aurora.app compressa per macOS (`*.app.tar.gz`).
    MacApp,
}

impl PackageKind {
    /// Il pacchetto di questo sistema: su macOS l'app compressa, altrove l'installatore di Windows (Linux non installa niente).
    pub const fn current() -> PackageKind {
        if cfg!(target_os = "macos") { PackageKind::MacApp } else { PackageKind::WindowsSetup }
    }

    /// Come finisce il nome del file.
    pub const fn suffix(self) -> &'static str {
        match self {
            PackageKind::WindowsSetup => "-setup.exe",
            PackageKind::MacApp => ".app.tar.gz",
        }
    }
}

/// Il nome del file da salvare: solo la parte finale, senza percorsi, e solo se sembra un pacchetto di aggiornamento
/// (installatore di Windows o app compressa per macOS).
pub fn safe_file_name(asset_name: &str) -> Option<String> {
    let name = asset_name.rsplit(['/', '\\']).next()?.trim();
    let known = [PackageKind::WindowsSetup, PackageKind::MacApp].iter().any(|k| name.len() > k.suffix().len() && name.ends_with(k.suffix()));
    let ok = name.len() <= 100 && known && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    ok.then(|| name.to_string())
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
    /// Lo scaricamento del pacchetto in corso (o appena finito), se c'e'.
    pub download: Option<DownloadProgress>,
    /// La preparazione dell'aggiornamento (scaricamento in anticipo, firma, installazione).
    pub stage: Stage,
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
                    Some(Asset { name: a["name"].as_str()?.to_string(), size: a["size"].as_u64().unwrap_or(0), url: a["browser_download_url"].as_str()?.to_string(), api_url: a["url"].as_str().unwrap_or("").to_string() })
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

/// Il pacchetto di aggiornamento di una release per questo sistema e la sua firma (`<pacchetto>.sig`), se ci sono tutti e due:
/// su Windows `*-setup.exe`, su macOS `Aurora-macOS.app.tar.gz`.
pub fn installer_assets(release: &Release) -> Option<(&Asset, &Asset)> {
    installer_assets_for(release, PackageKind::current())
}

/// Come `installer_assets`, per un tipo di pacchetto scelto (le prove guardano tutti e due i sistemi).
pub fn installer_assets_for(release: &Release, kind: PackageKind) -> Option<(&Asset, &Asset)> {
    let with_sig = |a: &&Asset| release.assets.iter().any(|s| s.name == format!("{}.sig", a.name));
    let setup = match kind {
        PackageKind::WindowsSetup => release.assets.iter().find(|a| a.name.ends_with("-setup.exe")),
        // il nome fisso per primo; altrimenti un'altra app compressa che abbia la sua firma
        PackageKind::MacApp => release.assets.iter().find(|a| a.name == MAC_PACKAGE).or_else(|| release.assets.iter().filter(|a| a.name.ends_with(".app.tar.gz")).find(with_sig)),
    }?;
    let sig = release.assets.iter().find(|a| a.name == format!("{}.sig", setup.name))?;
    Some((setup, sig))
}

/// Controlla la firma minisign di un pacchetto. `signature_b64` e `pubkey_b64` sono come li scrive Tauri (testo minisign in base64).
/// Un pacchetto senza firma valida non si installa mai.
pub fn verify_signature(data: &[u8], signature_b64: &str, pubkey_b64: &str) -> Result<(), String> {
    use base64_lite::decode;
    let key_text = decode(pubkey_b64.trim()).and_then(|b| String::from_utf8(b).ok()).ok_or("chiave pubblica non valida")?;
    let sig_text = decode(signature_b64.trim()).and_then(|b| String::from_utf8(b).ok()).ok_or("firma non valida")?;
    let key = minisign_verify::PublicKey::decode(&key_text).map_err(|e| format!("chiave pubblica non valida: {e}"))?;
    let signature = minisign_verify::Signature::decode(&sig_text).map_err(|e| format!("firma non valida: {e}"))?;
    key.verify(data, &signature, false).map_err(|_| "la firma del pacchetto non corrisponde: aggiornamento rifiutato".to_string())
}

/// Base64 standard, quanto basta per le chiavi e le firme (evita una dipendenza per poche righe).
mod base64_lite {
    pub fn decode(text: &str) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(text.len() * 3 / 4);
        let (mut acc, mut bits) = (0u32, 0u32);
        for c in text.bytes().filter(|c| !c.is_ascii_whitespace()) {
            let v = match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => break,
                _ => return None,
            };
            acc = (acc << 6) | u32::from(v);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
                acc &= (1 << bits) - 1;
            }
        }
        Some(out)
    }
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
    download: Mutex<Option<DownloadProgress>>,
    stage: Mutex<StageState>,
    /// Una sola preparazione alla volta: chi arriva dopo aspetta e trova il pacchetto pronto.
    preparing: Mutex<()>,
}

/// La preparazione con quanto non si mostra: il file pronto e la sua firma (per ricontrollarla prima di installare).
#[derive(Default)]
struct StageState {
    stage: Stage,
    path: Option<PathBuf>,
    signature: String,
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
        Arc::new(UpdateService { path: path.to_path_buf(), current: current.to_string(), api: api.trim_end_matches('/').to_string(), config: Mutex::new(config), state: Mutex::new(State { notified: std::fs::read_to_string(path.with_extension("notified")).unwrap_or_default().trim().to_string(), checked_at_ms: 0, latest: None, error: None }), notified_path: path.with_extension("notified"), download: Mutex::new(None), stage: Mutex::new(StageState::default()), preparing: Mutex::new(()) })
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
            download: lock(&self.download).clone(),
            stage: lock(&self.stage).stage.clone(),
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

    /// La release piu' nuova della versione in uso trovata dall'ultimo controllo, se c'e'.
    pub fn available(&self) -> Option<Release> {
        self.status().available
    }

    /// Scarica un file allegato alla release (al massimo `max_bytes`). Con un token usa l'API di GitHub, che serve per un repository
    /// privato; senza, l'indirizzo pubblico. Bloccante: da un thread dedicato.
    pub fn download(&self, asset: &Asset, max_bytes: u64) -> Result<Vec<u8>, String> {
        let config = self.config();
        // Strade possibili per lo stesso file: con il token l'API (repository privato); senza, l'indirizzo pubblico e, se non risponde,
        // l'API di GitHub, che per un repository pubblico scarica anche senza token e passa da server diversi. Un 5xx o una rete che
        // cade si riprovano (tre giri, una pausa crescente): un utente ha avuto per un quarto d'ora solo «504» sull'indirizzo pubblico.
        let with_token = !config.token.is_empty() && !asset.api_url.is_empty();
        let mut ways: Vec<(&str, bool)> = Vec::new();
        if with_token {
            ways.push((&asset.api_url, true));
        } else {
            ways.push((&asset.url, false));
            if !asset.api_url.is_empty() {
                ways.push((&asset.api_url, true));
            }
        }
        let tracked = max_bytes > 1024 * 1024;
        if tracked {
            *lock(&self.download) = Some(DownloadProgress { file: asset.name.clone(), done: 0, total: asset.size });
        }
        let mut last = String::new();
        for round in 0..3u64 {
            if round > 0 {
                std::thread::sleep(Duration::from_secs(2 * round));
            }
            for &(url, api) in &ways {
                match self.download_once(url, api, &config, asset, max_bytes) {
                    Ok(data) => return Ok(data),
                    Err((message, retry)) => {
                        tracing::info!("scaricamento di {} non riuscito ({}): {message}", asset.name, if api { "API" } else { "indirizzo pubblico" });
                        if !retry {
                            if tracked {
                                *lock(&self.download) = None;
                            }
                            return Err(message);
                        }
                        last = message;
                    }
                }
            }
        }
        if tracked {
            *lock(&self.download) = None;
        }
        Err(last)
    }

    /// Un tentativo: `Err((messaggio, si_puo_riprovare))`.
    fn download_once(&self, url: &str, api: bool, config: &UpdateConfig, asset: &Asset, max_bytes: u64) -> Result<Vec<u8>, (String, bool)> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(600))).http_status_as_error(false).build().into();
        let mut request = agent.get(url).header("User-Agent", "Aurora-updater");
        if api {
            request = request.header("Accept", "application/octet-stream");
            if !config.token.is_empty() {
                request = request.header("Authorization", &format!("Bearer {}", config.token));
            }
        }
        let mut response = request.call().map_err(|e| (e.to_string(), true))?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(match status {
                401 | 403 | 404 if config.token.is_empty() => ("file non scaricabile: se il repository e' privato serve un token".to_string(), false),
                401 | 403 => ("accesso rifiutato da GitHub (token scaduto o senza accesso)".to_string(), false),
                other => (format!("GitHub ha risposto {other} scaricando {}", asset.name), other >= 500 || other == 429),
            });
        }
        let tracked = max_bytes > 1024 * 1024;
        let length = response.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(asset.size);
        let mut reader = response.body_mut().with_config().limit(max_bytes).reader();
        let mut data = Vec::with_capacity(length.min(max_bytes) as usize);
        let mut buf = vec![0u8; 64 * 1024];
        let started = std::time::Instant::now();
        loop {
            let n = std::io::Read::read(&mut reader, &mut buf).map_err(|e| (format!("scaricamento di {} interrotto: {e}", asset.name), true))?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            if tracked {
                *lock(&self.download) = Some(DownloadProgress { file: asset.name.clone(), done: data.len() as u64, total: length });
                self.stage_progress(&asset.name, data.len() as u64, length, started.elapsed());
            }
        }
        Ok(data)
    }

    /// L'avanzamento dello scaricamento nella preparazione (solo se si sta preparando).
    fn stage_progress(&self, file: &str, done: u64, total: u64, elapsed: Duration) {
        let mut st = lock(&self.stage);
        if st.stage.phase != Phase::Downloading {
            return;
        }
        let secs = elapsed.as_secs_f64();
        // nel primo mezzo secondo la media non dice ancora niente
        let speed = if secs >= 0.5 { (done as f64 / secs) as u64 } else { 0 };
        st.stage.file = file.to_string();
        st.stage.done = done;
        st.stage.total = total;
        st.stage.percent = (total > 0).then(|| ((done.min(total) * 100) / total) as u8);
        st.stage.speed_bps = speed;
        st.stage.eta_s = (total > 0 && speed > 0).then(|| total.saturating_sub(done).div_ceil(speed));
    }

    fn set_phase(&self, phase: Phase) {
        let mut st = lock(&self.stage);
        st.stage.phase = phase;
        if phase != Phase::Error {
            st.stage.error = None;
        }
    }

    /// La preparazione o l'avvio dell'installatore sono andati male: lo stato dice perche' (si puo' riprovare).
    pub fn set_error(&self, message: &str) {
        let mut st = lock(&self.stage);
        st.stage.phase = Phase::Error;
        st.stage.error = Some(message.to_string());
        st.stage.speed_bps = 0;
        st.stage.eta_s = None;
    }

    /// L'installatore sta per partire.
    pub fn set_installing(&self) {
        self.set_phase(Phase::Installing);
    }

    /// Lo stato attuale della preparazione.
    pub fn stage(&self) -> Stage {
        lock(&self.stage).stage.clone()
    }

    /// Dice subito che lo scaricamento sta per partire (prima che ci arrivi il thread che scarica), cosi' la pagina che rilegge lo stato
    /// vede gia' «downloading». Non tocca una preparazione in corso o gia' pronta per quella versione.
    pub fn begin(&self, version: &str) {
        let mut st = lock(&self.stage);
        let busy = matches!(st.stage.phase, Phase::Downloading | Phase::Verifying | Phase::Installing) || (st.stage.phase == Phase::Ready && st.stage.version == version);
        if !busy {
            st.stage = Stage { phase: Phase::Downloading, version: version.to_string(), ..Stage::default() };
            st.path = None;
        }
    }

    /// Conviene preparare adesso la versione nuova? Si' se c'e', ha pacchetto e firma, e non e' gia' pronta, in corso o fallita
    /// (una preparazione fallita si riprova a mano, non a ogni controllo).
    pub fn should_prepare(&self) -> Option<String> {
        let release = self.available()?;
        installer_assets(&release)?;
        let st = lock(&self.stage);
        let busy = matches!(st.stage.phase, Phase::Downloading | Phase::Verifying | Phase::Installing);
        let same = st.stage.version == release.version;
        (st.stage.phase == Phase::Idle || (!busy && !same)).then_some(release.version)
    }

    /// Prepara l'aggiornamento: scarica pacchetto e firma della versione nuova (con le ripetizioni di `download`), controlla la firma con
    /// `pubkey` e salva il pacchetto in `dir`. Se e' gia' pronto non riscarica: rilegge il file dal disco e ne ricontrolla la firma.
    /// Un file della grandezza giusta gia' sul disco (da un avvio precedente) si tiene se la firma torna. Ritorna il pacchetto solo se
    /// completo e firmato; altrimenti lo stato passa a `error` con il motivo. Bloccante: da un thread dedicato.
    pub fn prepare(&self, pubkey: &str, dir: &Path, max_bytes: u64) -> Result<Prepared, String> {
        let _one = lock(&self.preparing);
        let result = self.prepare_locked(pubkey, dir, max_bytes);
        if let Err(e) = &result {
            tracing::warn!("preparazione dell'aggiornamento non riuscita: {e}");
            self.set_error(e);
        }
        result
    }

    fn prepare_locked(&self, pubkey: &str, dir: &Path, max_bytes: u64) -> Result<Prepared, String> {
        // l'ultimo controllo puo' essere di prima della pubblicazione: se non si conosce una versione nuova, si chiede subito a GitHub
        let release = match self.available() {
            Some(r) => r,
            None => self.check().available.ok_or("nessuna versione nuova trovata")?,
        };
        // gia' pronta: si ricontrolla il file sul disco (qualcuno puo' averlo toccato) e non si riscarica
        let ready = {
            let st = lock(&self.stage);
            (st.stage.phase == Phase::Ready && st.stage.version == release.version).then(|| (st.path.clone(), st.signature.clone()))
        };
        if let Some((Some(path), signature)) = ready {
            self.set_phase(Phase::Verifying);
            let checked = std::fs::read(&path).map_err(|e| format!("pacchetto pronto non leggibile: {e}")).and_then(|data| verify_signature(&data, &signature, pubkey));
            match checked {
                Ok(()) => {
                    self.set_phase(Phase::Ready);
                    return Ok(Prepared { version: release.version, path });
                }
                Err(e) => tracing::warn!("il pacchetto pronto non va piu' ({e}): si riscarica"),
            }
        }
        let (setup, sig) = installer_assets(&release).ok_or("la release non ha il pacchetto di installazione con la sua firma")?;
        let file_name = safe_file_name(&setup.name).ok_or("nome del pacchetto non valido")?;
        {
            let mut st = lock(&self.stage);
            st.stage = Stage { phase: Phase::Downloading, version: release.version.clone(), file: setup.name.clone(), total: setup.size, percent: Some(0), ..Stage::default() };
            st.path = None;
            st.signature.clear();
        }
        std::fs::create_dir_all(dir).map_err(|e| format!("cartella temporanea: {e}"))?;
        let path = dir.join(&file_name);
        // la firma per prima: e' piccola, e senza di lei il pacchetto non servirebbe
        let signature = String::from_utf8(self.download(sig, 16 * 1024)?).map_err(|_| "firma illeggibile".to_string())?;
        // un pacchetto della grandezza giusta gia' sul disco (avvio precedente): se la firma torna, non si riscarica
        let kept = std::fs::metadata(&path).is_ok_and(|m| setup.size > 0 && m.len() == setup.size) && {
            self.set_phase(Phase::Verifying);
            std::fs::read(&path).is_ok_and(|data| verify_signature(&data, &signature, pubkey).is_ok())
        };
        if !kept {
            self.set_phase(Phase::Downloading);
            let data = self.download(setup, max_bytes)?;
            self.set_phase(Phase::Verifying);
            verify_signature(&data, &signature, pubkey)?;
            // i pacchetti di versioni passate nella stessa cartella non servono piu'
            let suffix = if file_name.ends_with(PackageKind::MacApp.suffix()) { PackageKind::MacApp.suffix() } else { PackageKind::WindowsSetup.suffix() };
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    let p = entry.path();
                    if p != path && p.is_file() && p.to_string_lossy().ends_with(suffix) {
                        let _ = std::fs::remove_file(p);
                    }
                }
            }
            std::fs::write(&path, &data).map_err(|e| format!("pacchetto non salvato: {e}"))?;
        }
        {
            let mut st = lock(&self.stage);
            st.stage.phase = Phase::Ready;
            st.stage.error = None;
            st.stage.done = setup.size.max(st.stage.done);
            st.stage.total = setup.size.max(st.stage.total);
            st.stage.percent = Some(100);
            st.stage.speed_bps = 0;
            st.stage.eta_s = Some(0);
            st.path = Some(path.clone());
            st.signature = signature;
        }
        tracing::info!("aggiornamento {} pronto in {}", release.version, path.display());
        Ok(Prepared { version: release.version, path })
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
        assert_eq!(r.assets, vec![Asset { name: "Aurora-setup.exe".into(), size: 1234, url: "https://example.test/Aurora-setup.exe".into(), api_url: String::new() }]);
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


    // Chiave e firma di prova (non segrete): il file `sample.bin` firmato con una chiave generata solo per questo.
    const SAMPLE: &[u8] = include_bytes!("../tests/fixtures/sample.bin");
    const SAMPLE_SIG: &str = include_str!("../tests/fixtures/sample.bin.sig");
    const TEST_KEY: &str = include_str!("../tests/fixtures/test.key.pub");

    #[test]
    fn a_package_signed_with_the_right_key_is_accepted() {
        assert_eq!(verify_signature(SAMPLE, SAMPLE_SIG, TEST_KEY), Ok(()));
    }

    #[test]
    fn a_changed_package_is_refused() {
        let mut data = SAMPLE.to_vec();
        data[0] ^= 1;
        assert!(verify_signature(&data, SAMPLE_SIG, TEST_KEY).unwrap_err().contains("non corrisponde"));
        assert!(verify_signature(b"", SAMPLE_SIG, TEST_KEY).is_err());
    }

    #[test]
    fn another_key_or_garbage_is_refused() {
        // una chiave pubblica valida ma di un'altra coppia (cambia un carattere del corpo)
        let other = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDAwMDAwMDAwMDAwMDAwMDAKUldRQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=";
        assert!(verify_signature(SAMPLE, SAMPLE_SIG, other).is_err());
        assert!(verify_signature(SAMPLE, "non e' base64 !!", TEST_KEY).is_err());
        assert!(verify_signature(SAMPLE, SAMPLE_SIG, "").is_err());
        assert!(verify_signature(SAMPLE, "", TEST_KEY).is_err());
    }

    fn release_with(names: &[&str]) -> Release {
        let assets = names.iter().map(|n| Asset { name: (*n).into(), size: 1, url: format!("https://example.test/{n}"), api_url: format!("https://api.example.test/{n}") }).collect();
        Release { tag: "v1.0.0".into(), version: "1.0.0".into(), name: "x".into(), notes: String::new(), url: String::new(), published_at: String::new(), prerelease: false, assets }
    }

    #[test]
    fn finds_the_installer_and_its_signature_only_when_both_are_there() {
        let both = release_with(&["Aurora_1.0.0_x64-setup.exe", "Aurora_1.0.0_x64-setup.exe.sig", "notes.txt"]);
        let (setup, sig) = installer_assets(&both).expect("pacchetto e firma");
        assert_eq!((setup.name.as_str(), sig.name.as_str()), ("Aurora_1.0.0_x64-setup.exe", "Aurora_1.0.0_x64-setup.exe.sig"));
        assert!(installer_assets(&release_with(&["Aurora_1.0.0_x64-setup.exe"])).is_none(), "senza firma non si installa");
        assert!(installer_assets(&release_with(&["Aurora_1.0.0_x64-setup.exe.sig"])).is_none());
        assert!(installer_assets(&release_with(&["Aurora_1.0.0_x64-setup.exe", "Altro-setup.exe.sig"])).is_none(), "la firma deve essere di quel pacchetto");
    }

    #[test]
    fn on_macos_the_app_package_with_its_signature_is_chosen() {
        let all = release_with(&["Aurora_1.0.0_x64-setup.exe", "Aurora_1.0.0_x64-setup.exe.sig", "Aurora-macOS.dmg", "Aurora-macOS.app.tar.gz", "Aurora-macOS.app.tar.gz.sig"]);
        let (pkg, sig) = installer_assets_for(&all, PackageKind::MacApp).expect("app compressa e firma");
        assert_eq!((pkg.name.as_str(), sig.name.as_str()), ("Aurora-macOS.app.tar.gz", "Aurora-macOS.app.tar.gz.sig"));
        // Windows continua a scegliere l'installatore anche se la release ha il pacchetto del Mac
        let (setup, _) = installer_assets_for(&all, PackageKind::WindowsSetup).expect("installatore");
        assert_eq!(setup.name, "Aurora_1.0.0_x64-setup.exe");
        assert!(installer_assets_for(&release_with(&["Aurora-macOS.app.tar.gz", "Aurora-macOS.dmg"]), PackageKind::MacApp).is_none(), "senza firma non si installa");
        assert!(installer_assets_for(&release_with(&["Aurora-macOS.dmg", "Aurora_1.0.0_x64-setup.exe", "Aurora_1.0.0_x64-setup.exe.sig"]), PackageKind::MacApp).is_none(), "il .dmg non e' un pacchetto di aggiornamento");
        let other = release_with(&["Aurora_universal.app.tar.gz", "Aurora_universal.app.tar.gz.sig"]);
        assert_eq!(installer_assets_for(&other, PackageKind::MacApp).map(|(p, _)| p.name.as_str()), Some("Aurora_universal.app.tar.gz"));
    }

    #[test]
    fn safe_names_cover_both_packages() {
        assert_eq!(safe_file_name("Aurora-macOS.app.tar.gz").as_deref(), Some("Aurora-macOS.app.tar.gz"));
        assert_eq!(safe_file_name("../../Aurora-macOS.app.tar.gz").as_deref(), Some("Aurora-macOS.app.tar.gz"));
        assert_eq!(safe_file_name("Aurora_1.0.0_x64-setup.exe").as_deref(), Some("Aurora_1.0.0_x64-setup.exe"));
        assert!(safe_file_name(".app.tar.gz").is_none());
        assert!(safe_file_name("Aurora.tar.gz").is_none());
        assert!(safe_file_name("Aurora-macOS.dmg").is_none());
    }

    #[test]
    fn downloads_a_public_asset_without_credentials() {
        let (base, seen) = serve("200 OK", "contenuto".into());
        let svc = UpdateService::open(&temp("dl-public"), "0.9.0");
        let asset = Asset { name: "a".into(), size: 9, url: format!("{base}/a"), api_url: String::new() };
        assert_eq!(svc.download(&asset, 1000).unwrap(), b"contenuto");
        let request = seen.recv().unwrap().to_lowercase();
        assert!(request.starts_with("get /a"), "{request}");
        assert!(!request.contains("authorization"), "{request}");
    }

    #[test]
    fn downloads_from_a_private_repository_with_the_token_through_the_api_address() {
        let (base, seen) = serve("200 OK", "contenuto".into());
        let path = temp("dl-private");
        let svc = UpdateService::open(&path, "0.9.0");
        svc.set_config(UpdateConfig { repo: "owner/repo".into(), ..UpdateConfig::default() }, Some("ghp_segreto".into())).unwrap();
        let asset = Asset { name: "a".into(), size: 9, url: "http://127.0.0.1:1/pubblico".into(), api_url: format!("{base}/api/a") };
        assert_eq!(svc.download(&asset, 1000).unwrap(), b"contenuto");
        let request = seen.recv().unwrap().to_lowercase();
        assert!(request.starts_with("get /api/a"), "{request}");
        assert!(request.contains("authorization: bearer ghp_segreto") && request.contains("accept: application/octet-stream"), "{request}");
    }

    #[test]
    fn a_download_over_the_limit_or_refused_is_an_error() {
        let (base, _) = serve("200 OK", "x".repeat(500));
        let svc = UpdateService::open(&temp("dl-big"), "0.9.0");
        let asset = Asset { name: "a".into(), size: 500, url: format!("{base}/a"), api_url: String::new() };
        assert!(svc.download(&asset, 100).is_err(), "oltre il limite");
        let (base, _) = serve("404 Not Found", String::new());
        let asset = Asset { name: "a".into(), size: 0, url: format!("{base}/a"), api_url: String::new() };
        assert!(svc.download(&asset, 100).unwrap_err().contains("serve un token"));
    }
}
