//! Le impostazioni del core nel programma Windows. I valori vengono, in ordine di precedenza:
//!
//! 1. dalle variabili d'ambiente `AURORA_*` del processo;
//! 2. dal file `aurora.env` nella cartella dei dati (`CHIAVE=valore`, `#` per i commenti);
//! 3. dal file dei paesi incluso in Aurora e, se c'e', dall'archivio di Log4OM2 (facoltativo). Aurora non dipende da nessun altro programma.
//!
//! Tutto cio' che non si trova resta al valore predefinito del core. La trasmissione nel programma Windows e' **aperta** (scelta del proprietario); per chiuderla
//! senza opzioni. STOP, tetti, dead-man e automazioni spente di default restano.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub type Values = BTreeMap<String, String>;

/// Dove si cerca: le cartelle di Windows, passate dal chiamante cosi' le prove usano cartelle finte.
pub struct Places {
    /// La cartella Documenti: li' Log4OM2 tiene il suo archivio.
    pub documents: Option<PathBuf>,
    /// Il file dei paesi `cty.dat` incluso in Aurora (cartella delle risorse del programma): Aurora non lo prende da nessun altro programma.
    pub bundled_cty: Option<PathBuf>,
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

/// Cio' che Aurora trova da sola: il suo file dei paesi e, se c'e', l'archivio di Log4OM2 (solo per sapere chi e' gia' stato lavorato).
/// **Nessun altro programma serve**: Aurora non legge piu' JTDX (ne' `JTDX.ini` ne' il suo log) e non ne prende audio, PTT o radio.
pub fn detect(places: &Places) -> Values {
    let mut v = Values::new();
    if let Some(cty) = places.bundled_cty.as_deref().filter(|p| p.is_file()) {
        v.insert("AURORA_CTY_PATH".into(), text(cty));
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
    // il motore e' quello proprio di Aurora: ascolta l'ingresso audio e decodifica da solo, senza JTDX (`AURORA_ENGINE=jtdx` per usare JTDX)
    v.insert("AURORA_ENGINE".into(), "native".into());
    v.insert("AURORA_DATA_DIR".into(), text(data_dir));
    v.insert("AURORA_UI_DIR".into(), text(ui_dir));
    // raggiungibile dal telefono sulla rete di casa; l'accesso richiede comunque il codice di associazione
    v.insert("AURORA_BIND".into(), "0.0.0.0:8787".into());
    // la finestra del programma entra senza codice (solo da questo PC); i telefoni lo chiedono sempre
    v.insert("AURORA_TRUST_LOCAL".into(), "1".into());
    // il motore proprio decodifica (`AURORA_NATIVE_DECODE=0` lo spegne)
    v.insert("AURORA_NATIVE_DECODE".into(), "1".into());
    // le registrazioni WAV di ogni periodo (circa 144 MB su disco e una scrittura ogni 15 s) restano spente: servono solo a chi misura il
    // motore e si accendono con `AURORA_NATIVE_RECORD=1` in aurora.env
    v.insert("AURORA_NATIVE_RECORD".into(), "0".into());
    if let Some(file) = env_file {
        v.extend(parse_env_file(file));
    }
    v.extend(process_env.into_iter().filter(|(k, _)| k.starts_with("AURORA_")));
    // il secondo server UDP (Log4OM2) e' una impostazione di Aurora (`settings-motore.json`); `AURORA_EXPORT_UDP` resta solo per destinatari in piu'.
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
     # Accesso dal telefono sulla rete di casa senza codice di associazione (solo per le prove):\n\
     # AURORA_OPEN_ACCESS=1\n\
     #\n\
     # La finestra del programma sul PC entra senza codice (i telefoni lo chiedono sempre). Per toglierlo:\n\
     # AURORA_TRUST_LOCAL=0\n\
     #\n\
     # Porta e indirizzo (predefinito 0.0.0.0:8787, raggiungibile dal telefono):\n\
     # AURORA_BIND=0.0.0.0:8787\n\
     #\n\
     # Ingresso audio per la cascata (parte del nome); di norma si sceglie in Impostazioni:\n\
     # AURORA_AUDIO_DEVICE=USB Audio\n\
     #\n\
     # Non mettere Aurora tra i programmi che partono con Windows (al primo avvio lo e' di norma):\n\
     # AURORA_AUTOSTART=0\n\
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

    fn places(docs: &Path, bundled_cty: Option<PathBuf>) -> Places {
        Places { documents: Some(docs.to_path_buf()), bundled_cty }
    }

    #[test]
    fn reads_the_env_file_skipping_comments_blank_and_broken_lines() {
        let v = parse_env_file("# commento\n\nAURORA_ALLOW_TX=1\n  AURORA_AUDIO_DEVICE = \"USB Audio\" \nsenza uguale\nBAD KEY=1\n=vuota\nAURORA_X='a=b'\n");
        assert_eq!(v.get("AURORA_AUDIO_DEVICE").map(String::as_str), Some("USB Audio"));
        assert_eq!(v.get("AURORA_X").map(String::as_str), Some("a=b"));
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn on_an_empty_pc_it_finds_nothing_and_needs_nothing() {
        let docs = temp("empty-docs");
        assert!(detect(&places(&docs, None)).is_empty());
    }

    #[test]
    fn it_finds_its_own_countries_file_and_the_log4om_archive() {
        let docs = temp("found-docs");
        let res = temp("found-res");
        touch(&res.join("cty").join("cty.dat"), "x");
        touch(&docs.join("LOG4OM2").join("piccolo.sqlite"), "1");
        touch(&docs.join("LOG4OM2").join("grande.sqlite"), "123456789");
        touch(&docs.join("LOG4OM2").join("altro.txt"), "123456789012345");
        let v = detect(&places(&docs, Some(res.join("cty").join("cty.dat"))));
        assert!(v["AURORA_CTY_PATH"].ends_with("cty.dat"), "il file incluso in Aurora: {}", v["AURORA_CTY_PATH"]);
        assert!(v["AURORA_LOG4OM_DB"].ends_with("grande.sqlite"));
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn jtdx_installed_on_the_pc_changes_nothing() {
        // Aurora e' indipendente: un JTDX presente non le cambia motore, radio, PTT, audio, UDP o log
        let docs = temp("jtdx-docs");
        let local = temp("jtdx-local");
        touch(&local.join("JTDX").join("JTDX.ini"), "[Configuration]\nSoundInName=USB\nPTTMethod=@Variant(xPTT_method_RTS\0)\nPTTport=COM9\n");
        touch(&local.join("JTDX").join("wsjtx_log.adi"), "<eoh>");
        let v = build(&places(&docs, None), Path::new("d"), Path::new("u"), None, vec![]);
        assert_eq!(v["AURORA_ENGINE"], "native");
        for key in ["AURORA_JTDX_INI", "AURORA_RIG", "AURORA_PTT_METHOD", "AURORA_PTT_PORT", "AURORA_AUDIO_DEVICE", "AURORA_AUDIO_OUT_DEVICE", "AURORA_WORKED_ADIF", "AURORA_UDP_LISTEN", "AURORA_UDP_FORWARD"] {
            assert!(!v.contains_key(key), "{key} non deve venire da JTDX");
        }
    }

    #[test]
    fn a_missing_countries_file_is_just_left_out() {
        let res = temp("cty-res");
        let v = detect(&Places { documents: None, bundled_cty: Some(res.join("manca.dat")) });
        assert!(!v.contains_key("AURORA_CTY_PATH"));
    }

    #[test]
    fn the_file_beats_the_detection_and_the_environment_beats_the_file() {
        let p = Places { documents: None, bundled_cty: None };
        let env = vec![("AURORA_BIND".to_string(), "127.0.0.1:9000".to_string()), ("PATH".to_string(), "non conta".to_string())];
        let v = build(&p, Path::new("C:/dati"), Path::new("C:/ui"), Some("AURORA_AUDIO_DEVICE=Dal file\nAURORA_BIND=1.2.3.4:1\n"), env);
        assert_eq!(v["AURORA_AUDIO_DEVICE"], "Dal file");
        assert_eq!(v["AURORA_BIND"], "127.0.0.1:9000");
        assert!(!v.contains_key("PATH"), "solo le variabili AURORA_");
        assert_eq!(v["AURORA_DATA_DIR"], "C:/dati");
        assert_eq!(v["AURORA_UI_DIR"], "C:/ui");
    }

    #[test]
    fn the_defaults_reach_the_phone_and_open_transmission() {
        let p = Places { documents: None, bundled_cty: None };
        let v = build(&p, Path::new("C:/dati"), Path::new("C:/ui"), None, vec![]);
        assert_eq!(v["AURORA_BIND"], "0.0.0.0:8787");
        assert!(!v.contains_key("AURORA_ALLOW_TX"), "la trasmissione non ha opzioni");
        assert_eq!(v["AURORA_ENGINE"], "native", "il motore proprio, non JTDX");
        assert!(!v.contains_key("AURORA_OPEN_ACCESS"));
        let config = config_from(&v).expect("configurazione valida");
        assert!(config.local_trust && !config.open_access, "il PC entra senza codice, la rete di casa no");
        assert!(config.allow_tx);
        assert_eq!(config.bind.port(), 8787);
    }

    #[test]
    fn the_second_udp_server_comes_only_from_aurora_settings() {
        let p = Places { documents: None, bundled_cty: None };
        let native = build(&p, Path::new("d"), Path::new("u"), None, vec![]);
        assert!(!native.contains_key("AURORA_EXPORT_UDP"), "il secondo server e' una impostazione di Aurora: si puo' spegnere dalla scheda Segnalazioni");
        let own = build(&p, Path::new("d"), Path::new("u"), None, vec![("AURORA_EXPORT_UDP".to_string(), "127.0.0.1:2240".to_string())]);
        assert_eq!(own["AURORA_EXPORT_UDP"], "127.0.0.1:2240", "un destinatario in piu' scelto dall'ambiente resta valido");
    }

    #[test]
    fn the_sample_file_changes_nothing_until_edited() {
        assert!(parse_env_file(sample_env_file()).is_empty());
        assert!(!sample_env_file().contains("ALLOW_TX"));
    }

    #[test]
    fn a_bad_value_in_the_file_is_reported_not_ignored() {
        let p = Places { documents: None, bundled_cty: None };
        let v = build(&p, Path::new("d"), Path::new("u"), Some("AURORA_BIND=non un indirizzo"), vec![]);
        assert!(config_from(&v).is_err());
    }
}
