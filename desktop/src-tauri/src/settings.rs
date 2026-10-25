//! Le impostazioni del core nel programma Windows. I valori vengono, in ordine di precedenza:
//!
//! 1. dalle variabili d'ambiente `AURORA_*` del processo;
//! 2. dal file `aurora.env` nella cartella dei dati (`CHIAVE=valore`, `#` per i commenti);
//! 3. dal rilevamento automatico di JTDX, di Log4OM2 e del file dei paesi.
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

/// Il metodo del PTT e la porta come li scrive `JTDX.ini` (`PTTMethod=@Variant(...PTT_method_RTS...)`, `PTTport=COM9`).
pub fn ptt_from_ini(raw: &str) -> (Option<&'static str>, Option<String>) {
    let value = |key: &str| raw.lines().find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix('='))).map(str::trim);
    let method = value("PTTMethod").and_then(|v| {
        let v = v.to_ascii_uppercase();
        if v.contains("PTT_METHOD_RTS") {
            Some("rts")
        } else if v.contains("PTT_METHOD_DTR") {
            Some("dtr")
        } else if v.contains("PTT_METHOD_CAT") {
            Some("omnirig")
        } else if v.contains("PTT_METHOD_VOX") {
            Some("vox")
        } else {
            None
        }
    });
    let port = value("PTTport").filter(|p| !p.is_empty()).map(str::to_string);
    (method, port)
}

/// Le opzioni del primo server UDP di JTDX che contano per il flusso verso GridTracker: `AcceptUDPRequests`, `AcceptUDPReplyMessages` (1-3, di fabbrica 1)
/// e `EnableUDP1adifSending`. Con il motore proprio le usa l'emettitore UDP di Aurora (`AURORA_UDP_ACCEPT`, `AURORA_UDP_ACCEPT_REPLY`, `AURORA_UDP_ADIF`).
pub fn udp1_options_from_ini(raw: &str) -> (bool, u8, bool) {
    let value = |key: &str| raw.lines().find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix('='))).map(str::trim);
    let on = |key: &str| value(key).is_some_and(|v| v.eq_ignore_ascii_case("true") || v == "1");
    let reply = value("AcceptUDPReplyMessages").and_then(|v| v.parse::<u8>().ok()).filter(|n| (1..=3).contains(n)).unwrap_or(1);
    (on("AcceptUDPRequests"), reply, on("EnableUDP1adifSending"))
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
            // uscita audio e PTT come li ha impostati JTDX (PTT sulla linea RTS o DTR di una COM, CAT/OmniRig, VOX)
            if let Ok(raw) = std::fs::read_to_string(&ini) {
                let (accept, reply, adif) = udp1_options_from_ini(&raw);
                v.insert("AURORA_UDP_ACCEPT".into(), u8::from(accept).to_string());
                v.insert("AURORA_UDP_ACCEPT_REPLY".into(), reply.to_string());
                v.insert("AURORA_UDP_ADIF".into(), u8::from(adif).to_string());
                let (method, port) = ptt_from_ini(&raw);
                if let Some(method) = method {
                    v.insert("AURORA_PTT_METHOD".into(), method.into());
                }
                if let Some(port) = port {
                    v.insert("AURORA_PTT_PORT".into(), port);
                }
            }
            if let Some(device) = engine_jtdx::ini::read_shown(&ini).ok().and_then(|m| m.get("SoundOutName").cloned()).filter(|d| !d.trim().is_empty()) {
                v.insert("AURORA_AUDIO_OUT_DEVICE".into(), device);
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
    // il motore e' quello proprio di Aurora: ascolta l'ingresso audio e decodifica da solo, senza JTDX (`AURORA_ENGINE=jtdx` per usare JTDX)
    v.insert("AURORA_ENGINE".into(), "native".into());
    v.insert("AURORA_DATA_DIR".into(), text(data_dir));
    v.insert("AURORA_UI_DIR".into(), text(ui_dir));
    // raggiungibile dal telefono sulla rete di casa; l'accesso richiede comunque il codice di associazione
    v.insert("AURORA_BIND".into(), "0.0.0.0:8787".into());
    // la finestra del programma entra senza codice (solo da questo PC); i telefoni lo chiedono sempre
    v.insert("AURORA_TRUST_LOCAL".into(), "1".into());
    // il motore proprio decodifica in ombra accanto a JTDX e si confronta con lui (`AURORA_NATIVE_DECODE=0` lo spegne)
    v.insert("AURORA_NATIVE_DECODE".into(), "1".into());
    // gli ultimi 400 periodi (circa 144 MB) si tengono in WAV nella cartella dei dati: servono a misurare il motore proprio sulle stesse registrazioni
    v.insert("AURORA_NATIVE_RECORD".into(), "1".into());
    if let Some(file) = env_file {
        v.extend(parse_env_file(file));
    }
    v.extend(process_env.into_iter().filter(|(k, _)| k.starts_with("AURORA_")));
    // il secondo server UDP (Log4OM2) non si decide piu' qui: con il motore proprio e' una impostazione di Aurora (`settings-motore.json`),
    // importata una volta dal `JTDX.ini`; `AURORA_EXPORT_UDP` resta solo per destinatari in piu'.
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
     # Ingresso audio per la cascata (parte del nome); di norma e' quello scelto in JTDX:\n\
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

    fn places(local: &Path, docs: &Path, roots: Vec<PathBuf>) -> Places {
        Places { local_app_data: Some(local.to_path_buf()), documents: Some(docs.to_path_buf()), jtdx_roots: roots }
    }

    #[test]
    fn reads_the_env_file_skipping_comments_blank_and_broken_lines() {
        let v = parse_env_file("# commento\n\nAURORA_ALLOW_TX=1\n  AURORA_AUDIO_DEVICE = \"USB Audio\" \nsenza uguale\nBAD KEY=1\n=vuota\nAURORA_X='a=b'\n");
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
    fn the_defaults_reach_the_phone_and_open_transmission() {
        let p = Places { local_app_data: None, documents: None, jtdx_roots: vec![] };
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
    fn the_second_udp_server_is_not_injected_from_the_ini_any_more() {
        let local = temp("udp2-local");
        touch(&local.join("JTDX").join("JTDX.ini"), "[Configuration]\nUDP2Server=127.0.0.1\nUDP2ServerPort=2237\nEnableUDP2adifBroadcast=true\n");
        let p = Places { local_app_data: Some(local), documents: None, jtdx_roots: vec![] };
        let native = build(&p, Path::new("d"), Path::new("u"), None, vec![]);
        assert!(!native.contains_key("AURORA_EXPORT_UDP"), "il secondo server e' una impostazione di Aurora: si puo' spegnere dalla scheda Segnalazioni");
        let own = build(&p, Path::new("d"), Path::new("u"), None, vec![("AURORA_EXPORT_UDP".to_string(), "127.0.0.1:2240".to_string())]);
        assert_eq!(own["AURORA_EXPORT_UDP"], "127.0.0.1:2240", "un destinatario in piu' scelto dall'ambiente resta valido");
    }

    #[test]
    fn the_first_udp_server_options_of_jtdx_drive_the_gridtracker_flow() {
        let ini = "AcceptUDPReplyMessages=2
AcceptUDPRequests=true
EnableUDP1adifSending=true
";
        assert_eq!(udp1_options_from_ini(ini), (true, 2, true));
        // di fabbrica: niente comandi, solo CQ, niente ADIF; un valore fuori scala torna a 1
        assert_eq!(udp1_options_from_ini(""), (false, 1, false));
        assert_eq!(udp1_options_from_ini("AcceptUDPReplyMessages=9
AcceptUDPRequests=false
"), (false, 1, false));
    }

    #[test]
    fn the_ptt_method_and_port_are_read_from_the_jtdx_ini() {
        let rts = "PTTMethod=@Variant(\0\0\0\x7f\0\0\0\x1eTransceiverFactory::PTTMethod\0\0\0\0\x0fPTT_method_RTS\0)\nPTTport=COM9\n";
        assert_eq!(ptt_from_ini(rts), (Some("rts"), Some("COM9".to_string())));
        assert_eq!(ptt_from_ini("PTTMethod=@Variant(xPTT_method_CAT\0)\n").0, Some("omnirig"));
        assert_eq!(ptt_from_ini("PTTMethod=@Variant(xPTT_method_VOX\0)\n").0, Some("vox"));
        assert_eq!(ptt_from_ini("niente"), (None, None));
    }

    #[test]
    fn the_sample_file_changes_nothing_until_edited() {
        assert!(parse_env_file(sample_env_file()).is_empty());
        assert!(!sample_env_file().contains("ALLOW_TX"));
    }

    #[test]
    fn a_bad_value_in_the_file_is_reported_not_ignored() {
        let p = Places { local_app_data: None, documents: None, jtdx_roots: vec![] };
        let v = build(&p, Path::new("d"), Path::new("u"), Some("AURORA_BIND=non un indirizzo"), vec![]);
        assert!(config_from(&v).is_err());
    }
}
