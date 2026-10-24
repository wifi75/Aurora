//! Le impostazioni del core nel programma Windows. I valori vengono, in ordine di precedenza:
//!
//! 1. dalle variabili d'ambiente `AURORA_*` del processo;
//! 2. dal file `aurora.env` nella cartella dei dati (`CHIAVE=valore`, `#` per i commenti);
//! 3. dal rilevamento automatico di JTDX, di Log4OM2 e del file dei paesi.
//!
//! Tutto cio' che non si trova resta al valore predefinito del core. La trasmissione resta **chiusa** finche' non la si apre con
//! `AURORA_ALLOW_TX=1` nel file: nessun rilevamento la accende.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub type Values = BTreeMap<String, String>;

/// Dove si cerca: le cartelle di Windows, passate dal chiamante cosi' le prove usano cartelle finte.
pub struct Places {
    /// `%LOCALAPPDATA%`: li' JTDX tiene `JTDX.ini` e il suo log.
    pub local_app_data: Option<PathBuf>,
    /// La cartella Documenti: li' Log4OM2 tiene il suo archivio.
    pub documents: Option<PathBuf>,
    /// Dove puo' essere installato JTDX (per trovare `cty.dat`).
    pub jtdx_roots: Vec<PathBuf>,
}

impl Places {
    /// Le cartelle abituali di JTDX su Windows.
    pub fn standard_jtdx_roots() -> Vec<PathBuf> {
        vec![PathBuf::from(r"C:\JTDX64"), PathBuf::from(r"C:\JTDX"), PathBuf::from(r"C:\Program Files\JTDX"), PathBuf::from(r"C:\Program Files (x86)\JTDX")]
    }
}

/// Legge un file `CHIAVE=valore`: righe vuote e commenti (`#`) si saltano, gli apici attorno al valore si tolgono.
pub fn parse_env_file(text: &str) -> Values {
    let mut out = Values::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let value = value.trim();
        let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\''))).unwrap_or(value);
        out.insert(key.to_string(), value.to_string());
    }
    out
}

/// `cty.dat` dentro una cartella di JTDX: `<radice>/share/jtdx/cty.dat` o `<radice>/<versione>/share/jtdx/cty.dat`.
fn find_cty(roots: &[PathBuf], local_app_data: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for root in roots {
        candidates.push(root.join("share").join("jtdx").join("cty.dat"));
        if let Ok(entries) = std::fs::read_dir(root) {
            let mut versions: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
            // la versione piu' recente (nome piu' alto) per prima
            versions.sort();
            versions.reverse();
            candidates.extend(versions.into_iter().map(|v| v.join("share").join("jtdx").join("cty.dat")));
        }
    }
    if let Some(local) = local_app_data {
        candidates.push(local.join("JTDX").join("cty.dat"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Il piu' grande archivio `*.sqlite` di Log4OM2 nei Documenti.
fn find_log4om(documents: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(documents.join("LOG4OM2")).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sqlite")))
        .max_by_key(|p| std::fs::metadata(p).map_or(0, |m| m.len()))
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

/// Cio' che si ricava da JTDX e dagli altri programmi installati. Senza `JTDX.ini` resta il simulatore.
pub fn detect(places: &Places) -> Values {
    let mut v = Values::new();
    if let Some(local) = &places.local_app_data {
        let ini = local.join("JTDX").join("JTDX.ini");
        if ini.is_file() {
            v.insert("AURORA_ENGINE".into(), "jtdx".into());
            v.insert("AURORA_JTDX_INI".into(), text(&ini));
            v.insert("AURORA_UDP_LISTEN".into(), "127.0.0.1:2334".into());
            // il flusso identico per GridTracker2, come in JTDX
            v.insert("AURORA_UDP_FORWARD".into(), "127.0.0.1:2333".into());
            v.insert("AURORA_RIG".into(), "omnirig".into());
            let log = local.join("JTDX").join("wsjtx_log.adi");
            if log.is_file() {
                v.insert("AURORA_WORKED_ADIF".into(), text(&log));
            }
            // l'ingresso audio e' quello scelto in JTDX
            if let Some(device) = engine_jtdx::ini::read_shown(&ini).ok().and_then(|m| m.get("SoundInName").cloned()).filter(|d| !d.trim().is_empty()) {
                v.insert("AURORA_AUDIO_DEVICE".into(), device);
            }
        }
    }
    if let Some(cty) = find_cty(&places.jtdx_roots, places.local_app_data.as_deref()) {
        v.insert("AURORA_CTY_PATH".into(), text(&cty));
    }
    if let Some(db) = places.documents.as_deref().and_then(find_log4om) {
        v.insert("AURORA_LOG4OM_DB".into(), text(&db));
    }
    v
}

/// I valori finali: rilevamento, poi il file, poi l'ambiente del processo (il piu' forte). Il programma Windows imposta anche
/// la cartella dei dati, quella dell'interfaccia e l'indirizzo, se nessuno li ha scelti.
pub fn build(places: &Places, data_dir: &Path, ui_dir: &Path, env_file: Option<&str>, process_env: impl IntoIterator<Item = (String, String)>) -> Values {
    let mut v = detect(places);
    v.insert("AURORA_DATA_DIR".into(), text(data_dir));
    v.insert("AURORA_UI_DIR".into(), text(ui_dir));
    // raggiungibile dal telefono sulla rete di casa; l'accesso richiede comunque il codice di associazione
    v.insert("AURORA_BIND".into(), "0.0.0.0:8787".into());
    if let Some(file) = env_file {
        v.extend(parse_env_file(file));
    }
    v.extend(process_env.into_iter().filter(|(k, _)| k.starts_with("AURORA_")));
    v
}

pub fn config_from(values: &Values) -> Result<aurora::Config, aurora::StartError> {
    aurora::Config::from_lookup(|key| values.get(key).cloned())
}

/// Il file di esempio che si crea al primo avvio: tutto commentato, cosi' non cambia nulla finche' non lo si modifica.
pub fn sample_env_file() -> &'static str {
    "# Impostazioni di Aurora. Ogni riga e' CHIAVE=valore; le righe con # sono commenti.\n\
     # Si applicano al prossimo avvio. Le variabili d'ambiente AURORA_* hanno la precedenza su questo file.\n\
     #\n\
     # La trasmissione e' CHIUSA. Per aprirla (CQ, risposte con doppio clic, automazione in modo reale):\n\
     # AURORA_ALLOW_TX=1\n\
     #\n\
     # Accesso dal telefono sulla rete di casa senza codice di associazione (solo per le prove):\n\
     # AURORA_OPEN_ACCESS=1\n\
     #\n\
     # Porta e indirizzo (predefinito 0.0.0.0:8787, raggiungibile dal telefono):\n\
     # AURORA_BIND=0.0.0.0:8787\n\
     #\n\
     # Ingresso audio per la cascata (parte del nome); di norma e' quello scelto in JTDX:\n\
     # AURORA_AUDIO_DEVICE=USB Audio\n\
     #\n\
     # Giorni di storico dei messaggi ricevuti (0 = per sempre):\n\
     # AURORA_HISTORY_DAYS=30\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-desktop-{}-{name}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn places(local: &Path, docs: &Path, roots: Vec<PathBuf>) -> Places {
        Places { local_app_data: Some(local.to_path_buf()), documents: Some(docs.to_path_buf()), jtdx_roots: roots }
    }

    #[test]
    fn reads_the_env_file_skipping_comments_blank_and_broken_lines() {
        let v = parse_env_file("# commento\n\nAURORA_ALLOW_TX=1\n  AURORA_AUDIO_DEVICE = \"USB Audio\" \nsenza uguale\nBAD KEY=1\n=vuota\nAURORA_X='a=b'\n");
        assert_eq!(v.get("AURORA_ALLOW_TX").map(String::as_str), Some("1"));
        assert_eq!(v.get("AURORA_AUDIO_DEVICE").map(String::as_str), Some("USB Audio"));
        assert_eq!(v.get("AURORA_X").map(String::as_str), Some("a=b"));
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn without_jtdx_it_stays_the_simulator_and_finds_nothing() {
        let local = temp("nojtdx-local");
        let docs = temp("nojtdx-docs");
        assert!(detect(&places(&local, &docs, vec![])).is_empty());
    }

    #[test]
    fn finds_jtdx_its_log_the_audio_input_the_countries_file_and_the_log4om_archive() {
        let local = temp("found-local");
        let docs = temp("found-docs");
        let root = temp("found-root");
        touch(&local.join("JTDX").join("JTDX.ini"), "[Configuration]\nSoundInName=Microfono (2- USB Audio Device)\nMyCall=AA1AA\n");
        touch(&local.join("JTDX").join("wsjtx_log.adi"), "<eoh>");
        touch(&root.join("159").join("share").join("jtdx").join("cty.dat"), "x");
        touch(&root.join("158").join("share").join("jtdx").join("cty.dat"), "old");
        touch(&docs.join("LOG4OM2").join("piccolo.sqlite"), "1");
        touch(&docs.join("LOG4OM2").join("grande.sqlite"), "123456789");
        touch(&docs.join("LOG4OM2").join("altro.txt"), "123456789012345");
        let v = detect(&places(&local, &docs, vec![root.clone()]));
        assert_eq!(v["AURORA_ENGINE"], "jtdx");
        assert_eq!(v["AURORA_RIG"], "omnirig");
        assert_eq!(v["AURORA_UDP_LISTEN"], "127.0.0.1:2334");
        assert_eq!(v["AURORA_UDP_FORWARD"], "127.0.0.1:2333");
        assert_eq!(v["AURORA_AUDIO_DEVICE"], "Microfono (2- USB Audio Device)");
        assert!(v["AURORA_JTDX_INI"].ends_with("JTDX.ini"));
        assert!(v["AURORA_WORKED_ADIF"].ends_with("wsjtx_log.adi"));
        assert!(v["AURORA_CTY_PATH"].contains("159"), "la versione piu' recente: {}", v["AURORA_CTY_PATH"]);
        assert!(v["AURORA_LOG4OM_DB"].ends_with("grande.sqlite"));
    }

    #[test]
    fn a_missing_audio_device_or_adif_just_leaves_those_values_out() {
        let local = temp("partial-local");
        touch(&local.join("JTDX").join("JTDX.ini"), "[Configuration]\nMyCall=AA1AA\n");
        let v = detect(&Places { local_app_data: Some(local), documents: None, jtdx_roots: vec![] });
        assert_eq!(v["AURORA_ENGINE"], "jtdx");
        assert!(!v.contains_key("AURORA_AUDIO_DEVICE") && !v.contains_key("AURORA_WORKED_ADIF") && !v.contains_key("AURORA_LOG4OM_DB"));
    }

    #[test]
    fn the_file_beats_the_detection_and_the_environment_beats_the_file() {
        let local = temp("prec-local");
        touch(&local.join("JTDX").join("JTDX.ini"), "[Configuration]\nSoundInName=Rilevato\n");
        let p = Places { local_app_data: Some(local), documents: None, jtdx_roots: vec![] };
        let env = vec![("AURORA_BIND".to_string(), "127.0.0.1:9000".to_string()), ("PATH".to_string(), "non conta".to_string())];
        let v = build(&p, Path::new("C:/dati"), Path::new("C:/ui"), Some("AURORA_AUDIO_DEVICE=Dal file\nAURORA_BIND=1.2.3.4:1\n"), env);
        assert_eq!(v["AURORA_AUDIO_DEVICE"], "Dal file");
        assert_eq!(v["AURORA_BIND"], "127.0.0.1:9000");
        assert!(!v.contains_key("PATH"), "solo le variabili AURORA_");
        assert_eq!(v["AURORA_DATA_DIR"], "C:/dati");
        assert_eq!(v["AURORA_UI_DIR"], "C:/ui");
    }

    #[test]
    fn the_defaults_reach_the_phone_and_keep_transmission_closed() {
        let p = Places { local_app_data: None, documents: None, jtdx_roots: vec![] };
        let v = build(&p, Path::new("C:/dati"), Path::new("C:/ui"), None, vec![]);
        assert_eq!(v["AURORA_BIND"], "0.0.0.0:8787");
        assert!(!v.contains_key("AURORA_ALLOW_TX"), "la trasmissione non si apre mai da sola");
        assert!(!v.contains_key("AURORA_OPEN_ACCESS"));
        let config = config_from(&v).expect("configurazione valida");
        assert!(!config.allow_tx);
        assert_eq!(config.bind.port(), 8787);
    }

    #[test]
    fn the_sample_file_changes_nothing_until_edited() {
        assert!(parse_env_file(sample_env_file()).is_empty());
        assert!(sample_env_file().contains("# AURORA_ALLOW_TX=1"));
    }

    #[test]
    fn a_bad_value_in_the_file_is_reported_not_ignored() {
        let p = Places { local_app_data: None, documents: None, jtdx_roots: vec![] };
        let v = build(&p, Path::new("d"), Path::new("u"), Some("AURORA_BIND=non un indirizzo"), vec![]);
        assert!(config_from(&v).is_err());
    }
}
