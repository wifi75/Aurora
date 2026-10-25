//! Elenco delle porte seriali e rilevamento **passivo** della radio, come in Decodium: si legge quello che Windows sa gia' (nome della porta,
//! identita' USB, numero di serie, nomi delle schede audio). Non si apre nessuna porta e non si manda nessun comando alla radio: una sonda
//! attiva troppo insistente ha gia' lasciato un PTT incollato (vedi `docs/analisi-decodium-cat.md`, sezione 4).
//!
//! La fiducia segue Decodium: nome per esteso 90, firma USB 60-75 (+15 se e' la porta CAT «enhanced»), interfaccia generica 35, porta qualunque 15.
//! Una firma sbagliata e' peggio di nessuna: dove il modello non e' certo si propone la famiglia e si lascia la scelta all'utente.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::RigError;

/// Una porta seriale vista dal sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SerialPortInfo {
    /// `COM3`.
    pub port: String,
    /// Nome mostrato da Windows, senza il numero della porta.
    pub name: String,
    /// Identificatore PnP (`USB\VID_10C4&PID_EA70&MI_00\...`).
    pub instance_id: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub serial: Option<String>,
    pub manufacturer: String,
    /// Porta virtuale (com0com, Bluetooth...): non e' una radio.
    pub virtual_port: bool,
}

/// Un candidato: una radio (o un'interfaccia) riconosciuta dalle identita' del sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// Identifica l'apparato fisico (non cambia se Windows rinumera la COM).
    pub device_key: String,
    pub is_radio: bool,
    /// `newcat`, `civ`, `kenwood`, `interface`, `serial`...
    pub protocol: String,
    /// Come la chiameremmo («Yaesu FT-2000»).
    pub rig_label: String,
    /// Testo da cercare nell'elenco dei modelli di Hamlib («FT-2000»), vuoto se il modello va scelto a mano.
    pub rig_token: String,
    /// Altri modelli della stessa famiglia, se la firma non basta a distinguerli.
    pub alternatives: Vec<String>,
    pub cat_port: String,
    pub other_ports: Vec<String>,
    pub baud: Option<u32>,
    pub civ_address: Option<u8>,
    pub audio_input: Option<String>,
    pub audio_output: Option<String>,
    /// 0-100.
    pub confidence: u8,
    /// Perche' si pensa questo, in chiaro.
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetectResult {
    pub candidates: Vec<Candidate>,
    pub audio_devices: Vec<String>,
    pub ports_seen: usize,
}

/// Cio' che si riconosce da un nome per esteso (nome della porta, costruttore, numero di serie).
struct NameSig {
    /// Frammenti da cercare nel testo normalizzato (minuscolo, senza spazi, trattini e sottolineature).
    needles: &'static [&'static str],
    label: &'static str,
    token: &'static str,
    protocol: &'static str,
    baud: Option<u32>,
    civ: Option<u8>,
}

macro_rules! name_sig {
    ($needles:expr, $label:expr, $token:expr, $protocol:expr, $baud:expr, $civ:expr) => {
        NameSig { needles: $needles, label: $label, token: $token, protocol: $protocol, baud: $baud, civ: $civ }
    };
}

/// Le firme per nome. L'ordine conta: la prima che scatta vince, quindi le piu' specifiche vengono prima.
static NAME_SIGS: &[NameSig] = &[
    // Yaesu: la FT-2000 (4800-38400 baud, 8N2, handshake hardware, modello 1029 di Hamlib) e le sorelle
    name_sig!(&["ft2000"], "Yaesu FT-2000", "FT-2000", "newcat", Some(38400), None),
    name_sig!(&["ftdx2000"], "Yaesu FT-2000", "FT-2000", "newcat", Some(38400), None),
    name_sig!(&["ft991a"], "Yaesu FT-991A", "FT-991A", "newcat", Some(38400), None),
    name_sig!(&["ft991"], "Yaesu FT-991", "FT-991", "newcat", Some(38400), None),
    name_sig!(&["ftdx101"], "Yaesu FT-DX101", "FT-DX101", "newcat", Some(38400), None),
    name_sig!(&["ftdx10", "ftdx-10"], "Yaesu FT-DX10", "FT-DX10", "newcat", Some(38400), None),
    name_sig!(&["ftdx3000"], "Yaesu FT-DX3000", "FT-DX3000", "newcat", Some(38400), None),
    name_sig!(&["ftdx5000"], "Yaesu FT-DX5000", "FT-DX5000", "newcat", Some(38400), None),
    name_sig!(&["ftdx1200"], "Yaesu FT-DX1200", "FT-DX1200", "newcat", Some(38400), None),
    name_sig!(&["ft710"], "Yaesu FT-710", "FT-710", "newcat", Some(38400), None),
    name_sig!(&["ft891"], "Yaesu FT-891", "FT-891", "newcat", Some(38400), None),
    name_sig!(&["ft450"], "Yaesu FT-450", "FT-450", "newcat", Some(38400), None),
    name_sig!(&["ft897"], "Yaesu FT-897", "FT-897", "newcat", Some(9600), None),
    name_sig!(&["ft817"], "Yaesu FT-817", "FT-817", "newcat", Some(9600), None),
    // Icom (CI-V): gli indirizzi sono quelli di fabbrica
    name_sig!(&["ic7300mk2", "ic-7300mk2"], "Icom IC-7300 MK2", "IC-7300MK2", "civ", Some(115200), Some(0xB6)),
    name_sig!(&["ic7300"], "Icom IC-7300", "IC-7300", "civ", Some(19200), Some(0x94)),
    name_sig!(&["ic7610"], "Icom IC-7610", "IC-7610", "civ", Some(19200), Some(0x98)),
    name_sig!(&["ic9700"], "Icom IC-9700", "IC-9700", "civ", Some(19200), Some(0xA2)),
    name_sig!(&["ic705"], "Icom IC-705", "IC-705", "civ", Some(19200), Some(0xA4)),
    name_sig!(&["ic7600"], "Icom IC-7600", "IC-7600", "civ", Some(19200), Some(0x7A)),
    name_sig!(&["ic7851"], "Icom IC-7851", "IC-7851", "civ", Some(19200), Some(0x8E)),
    name_sig!(&["ic7100"], "Icom IC-7100", "IC-7100", "civ", Some(19200), Some(0x88)),
    name_sig!(&["ic7410"], "Icom IC-7410", "IC-7410", "civ", Some(19200), Some(0x80)),
    // Kenwood
    name_sig!(&["ts890"], "Kenwood TS-890S", "TS-890S", "kenwood", Some(115200), None),
    name_sig!(&["ts590"], "Kenwood TS-590S", "TS-590S", "kenwood", Some(57600), None),
    name_sig!(&["ts2000"], "Kenwood TS-2000", "TS-2000", "kenwood", Some(57600), None),
    name_sig!(&["ts480"], "Kenwood TS-480", "TS-480", "kenwood", Some(57600), None),
    // Elecraft
    name_sig!(&["kx3"], "Elecraft KX3", "KX3", "kenwood", Some(38400), None),
    name_sig!(&["kx2"], "Elecraft KX2", "KX2", "kenwood", Some(38400), None),
    name_sig!(&["elecraftk4"], "Elecraft K4", "K4", "kenwood", Some(38400), None),
    name_sig!(&["elecraftk3", "k3s"], "Elecraft K3/K3S", "K3", "kenwood", Some(38400), None),
    // Flex (CAT virtuale di SmartSDR)
    name_sig!(&["flex", "smartsdr"], "FlexRadio (CAT di SmartSDR)", "Flex", "kenwood", Some(115200), None),
    // altre radio comuni in digitale
    name_sig!(&["xiegug90", "g90"], "Xiegu G90", "G90", "civ", Some(19200), Some(0x70)),
    name_sig!(&["xiegux6100", "x6100"], "Xiegu X6100", "X6100", "civ", Some(19200), Some(0xA4)),
    name_sig!(&["qmx"], "QRP Labs QMX", "QMX", "kenwood", Some(38400), None),
    name_sig!(&["qdx"], "QRP Labs QDX", "TS-480", "kenwood", Some(38400), None),
];

/// Firma per identita' USB.
struct UsbSig {
    vid: u16,
    pid: Option<u16>,
    label: &'static str,
    /// Il modello piu' probabile (si propone, ma non e' certo) e gli altri della famiglia.
    token: &'static str,
    alternatives: &'static [&'static str],
    protocol: &'static str,
    baud: Option<u32>,
    confidence: u8,
}

static USB_SIGS: &[UsbSig] = &[
    // CP2105 doppio: Yaesu FT-991A, FT-DX10, FT-710, interfaccia SCU-17 (e con questa anche la FT-2000)
    UsbSig {
        vid: 0x10C4,
        pid: Some(0xEA70),
        label: "Yaesu (CP2105 doppio: FT-991A, FT-DX10, FT-710, SCU-17)",
        token: "FT-991A",
        alternatives: &["FT-DX10", "FT-710", "FT-DX101", "FT-2000"],
        protocol: "newcat",
        baud: Some(38400),
        confidence: 75,
    },
    // Icom con interfaccia USB propria
    UsbSig { vid: 0x0C26, pid: None, label: "Icom (interfaccia USB propria)", token: "", alternatives: &["IC-7300", "IC-7610", "IC-9700", "IC-705"], protocol: "civ", baud: Some(19200), confidence: 60 },
];

/// Interfacce USB-seriale generiche: dicono come e' collegata la radio, non quale sia.
static INTERFACES: &[(u16, Option<u16>, &str)] = &[
    (0x10C4, Some(0xEA60), "Silicon Labs CP210x"),
    (0x10C4, None, "Silicon Labs"),
    (0x1A86, None, "WCH CH340/CH341"),
    (0x0403, None, "FTDI"),
    (0x067B, None, "Prolific PL2303"),
];

/// Schede audio USB tipiche delle interfacce radio; sono escluse quelle virtuali.
const AUDIO_WANTED: &[&str] = &["usbaudiocodec", "usbpnpsound", "usbaudiodevice", "signalink", "digirig", "usbaudio"];
const AUDIO_EXCLUDED: &[&str] = &["vb-audio", "cable", "virtualaudio", "voicemeeter", "womic", "virtual"];

fn norm(text: &str) -> String {
    text.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

/// Il numero della porta per ordinare `COM3` prima di `COM10`.
fn com_number(port: &str) -> u32 {
    port.to_ascii_uppercase().strip_prefix("COM").and_then(|n| n.parse().ok()).unwrap_or(u32::MAX)
}

/// Il nome dell'apparato fisico: non cambia se Windows rinumera la COM.
fn device_key(p: &SerialPortInfo) -> String {
    if let (Some(vid), Some(pid)) = (p.vid, p.pid) {
        if let Some(serial) = &p.serial {
            return format!("{vid:04x}:{pid:04x}:{serial}");
        }
        // interfacce dello stesso apparato composito: stessa istanza padre, senza l'ultimo gruppo
        let tail = p.instance_id.split('\\').nth(2).unwrap_or("");
        if p.instance_id.to_ascii_uppercase().contains("&MI_") && let Some((parent, _)) = tail.rsplit_once('&') {
            return format!("{vid:04x}:{pid:04x}:{parent}");
        }
        if !tail.is_empty() {
            return format!("{vid:04x}:{pid:04x}:{tail}");
        }
    }
    format!("porta:{}", p.port)
}

fn is_enhanced(p: &SerialPortInfo) -> bool {
    let n = p.name.to_ascii_lowercase();
    n.contains("enhanced") || p.instance_id.to_ascii_uppercase().contains("&MI_00")
}

fn recognise(group: &[&SerialPortInfo]) -> Candidate {
    let mut ports: Vec<&SerialPortInfo> = group.to_vec();
    ports.sort_by_key(|p| (!is_enhanced(p), com_number(&p.port)));
    let first = ports[0];
    let text = norm(&ports.iter().map(|p| format!("{} {} {} {}", p.name, p.manufacturer, p.serial.as_deref().unwrap_or(""), p.instance_id)).collect::<Vec<_>>().join(" "));
    let mut c = Candidate {
        device_key: device_key(first),
        is_radio: false,
        protocol: "serial".into(),
        rig_label: "Porta seriale".into(),
        rig_token: String::new(),
        alternatives: Vec::new(),
        cat_port: first.port.clone(),
        other_ports: ports[1..].iter().map(|p| p.port.clone()).collect(),
        baud: None,
        civ_address: None,
        audio_input: None,
        audio_output: None,
        confidence: 15,
        evidence: format!("porta seriale {}", first.name),
    };
    // 1) il nome per esteso: la firma piu' specifica nel testo
    if let Some(sig) = NAME_SIGS.iter().find(|s| s.needles.iter().any(|n| text.contains(&norm(n)))) {
        c.is_radio = true;
        c.protocol = sig.protocol.into();
        c.rig_label = sig.label.into();
        c.rig_token = sig.token.into();
        c.baud = sig.baud;
        c.civ_address = sig.civ;
        c.confidence = 90;
        c.evidence = format!("il nome \"{}\" contiene il modello", first.name);
        return c;
    }
    // 2) l'identita' USB
    if let (Some(vid), pid) = (first.vid, first.pid)
        && let Some(sig) = USB_SIGS.iter().find(|s| s.vid == vid && (s.pid.is_none() || s.pid == pid))
    {
        c.is_radio = true;
        c.protocol = sig.protocol.into();
        c.rig_label = sig.label.into();
        c.rig_token = sig.token.into();
        c.alternatives = sig.alternatives.iter().map(|s| (*s).to_string()).collect();
        c.baud = sig.baud;
        // la porta «enhanced» del CP2105 e' quella del CAT
        let bonus = if is_enhanced(first) { 15 } else { 0 };
        c.confidence = (sig.confidence + bonus).min(89);
        c.evidence = format!("identita' USB {vid:04X}:{:04X}{}", pid.unwrap_or(0), if bonus > 0 { ", porta CAT (Enhanced)" } else { "" });
        return c;
    }
    // 3) un'interfaccia generica: il modello va scelto a mano
    if let Some(vid) = first.vid
        && let Some((_, _, name)) = INTERFACES.iter().find(|(v, p, _)| *v == vid && (p.is_none() || *p == first.pid))
    {
        c.protocol = "interface".into();
        c.rig_label = format!("Adattatore USB-seriale ({name}): il modello va scelto a mano");
        c.confidence = 35;
        c.evidence = format!("identita' USB {vid:04X}:{:04X}", first.pid.unwrap_or(0));
        return c;
    }
    // interfacce con il nome nel testo (Digirig, RIGblaster, SignaLink)
    if ["digirig", "rigblaster", "signalink"].iter().any(|k| text.contains(k)) {
        c.protocol = "interface".into();
        c.rig_label = "Interfaccia radio: il modello va scelto a mano".into();
        c.confidence = 35;
        c.evidence = format!("il nome \"{}\" e' quello di un'interfaccia radio", first.name);
    }
    c
}

/// Il riconoscimento: dalle porte e dai nomi delle schede audio ai candidati, in ordine di fiducia. Nessuna porta e' aperta.
pub fn detect(ports: &[SerialPortInfo], audio: &[String]) -> DetectResult {
    let real: Vec<&SerialPortInfo> = ports.iter().filter(|p| !p.virtual_port).collect();
    let mut groups: Vec<(String, Vec<&SerialPortInfo>)> = Vec::new();
    for p in &real {
        let key = device_key(p);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, g)) => g.push(p),
            None => groups.push((key, vec![p])),
        }
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    for (_, group) in &groups {
        let one = recognise(group);
        if !one.is_radio && group.len() > 1 {
            // porte multiple di un apparato non riconosciuto (CP2108, FT4232...): ognuna e' un candidato a se'
            candidates.extend(group.iter().map(|p| recognise(&[*p])));
        } else {
            candidates.push(one);
        }
    }
    let audio_pick = audio.iter().find(|name| {
        let n = norm(name);
        AUDIO_WANTED.iter().any(|w| n.contains(w)) && !AUDIO_EXCLUDED.iter().any(|x| n.contains(&norm(x)))
    });
    for c in &mut candidates {
        c.audio_input = audio_pick.cloned();
        c.audio_output = audio_pick.cloned();
    }
    candidates.sort_by(|a, b| b.confidence.cmp(&a.confidence).then(com_number(&a.cat_port).cmp(&com_number(&b.cat_port))));
    DetectResult { candidates, audio_devices: audio.to_vec(), ports_seen: ports.len() }
}

#[derive(Debug, Deserialize)]
struct RawPort {
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "PNPDeviceID", default)]
    pnp: Option<String>,
    #[serde(rename = "Manufacturer", default)]
    manufacturer: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawSystem {
    #[serde(default)]
    ports: serde_json::Value,
    #[serde(default)]
    audio: serde_json::Value,
}

fn hex4(text: &str, key: &str) -> Option<u16> {
    let at = text.to_ascii_uppercase().find(key)? + key.len();
    u16::from_str_radix(text.get(at..at + 4)?, 16).ok()
}

/// Il numero di serie dall'identificatore PnP: terzo pezzo di `USB\VID_..&PID_..\<serie>` se non e' un identificatore generato dal sistema (contiene `&`).
fn serial_of(pnp: &str) -> Option<String> {
    let upper = pnp.to_ascii_uppercase();
    if upper.starts_with("FTDIBUS\\") {
        // FTDIBUS\VID_0403+PID_6001+A50285BIA\0000: la serie e' dopo il secondo `+`, con una `A` finale aggiunta da FTDI
        let id = pnp.split('\\').nth(1)?;
        let serial = id.split('+').nth(2)?;
        let serial = serial.strip_suffix('A').unwrap_or(serial);
        return (!serial.is_empty()).then(|| serial.to_string());
    }
    let tail = pnp.split('\\').nth(2)?;
    (!tail.is_empty() && !tail.contains('&')).then(|| tail.to_string())
}

fn one_or_many<T: serde::de::DeserializeOwned>(value: &serde_json::Value) -> Vec<T> {
    match value {
        serde_json::Value::Array(items) => items.iter().filter_map(|i| serde_json::from_value(i.clone()).ok()).collect(),
        serde_json::Value::Null => Vec::new(),
        single => serde_json::from_value(single.clone()).map(|one| vec![one]).unwrap_or_default(),
    }
}

/// Legge il JSON di `Get-CimInstance` (porte e schede audio). PowerShell da' un oggetto, non una lista, se ce n'e' uno solo.
pub fn parse_system(json: &str) -> Result<(Vec<SerialPortInfo>, Vec<String>), RigError> {
    let raw: RawSystem = serde_json::from_str(json.trim_start_matches('\u{feff}')).map_err(|e| RigError(format!("risposta di sistema illeggibile: {e}")))?;
    let mut ports = Vec::new();
    for p in one_or_many::<RawPort>(&raw.ports) {
        let full = p.name.unwrap_or_default();
        // «Silicon Labs Dual CP210x USB to UART Bridge: Enhanced COM Port (COM5)»
        let Some(open) = full.rfind("(COM") else { continue };
        let port = full[open + 1..].trim_end_matches(')').trim().to_string();
        if com_number(&port) == u32::MAX {
            continue;
        }
        let pnp = p.pnp.unwrap_or_default();
        let upper = pnp.to_ascii_uppercase();
        let virtual_port = ["ROOT\\", "COM0COM", "VSP", "VIRTUAL", "BTHENUM"].iter().any(|k| upper.starts_with(k) || upper.contains(k)) || full.to_ascii_lowercase().contains("virtual");
        ports.push(SerialPortInfo {
            port,
            name: full[..open].trim().to_string(),
            vid: hex4(&pnp, "VID_"),
            pid: hex4(&pnp, "PID_"),
            serial: serial_of(&pnp),
            manufacturer: p.manufacturer.unwrap_or_default(),
            instance_id: pnp,
            virtual_port,
        });
    }
    ports.sort_by_key(|p| com_number(&p.port));
    let audio = one_or_many::<serde_json::Value>(&raw.audio)
        .into_iter()
        .filter_map(|v| v.get("Name").and_then(|n| n.as_str()).map(str::to_string).or_else(|| v.as_str().map(str::to_string)))
        .collect();
    Ok((ports, audio))
}

/// Lo script: porte COM e schede audio in un colpo solo, come JSON. Solo letture.
const SYSTEM_SCRIPT: &str = "$ErrorActionPreference='SilentlyContinue'; \
    $p = @(Get-CimInstance Win32_PnPEntity | Where-Object { $_.Name -match '\\(COM\\d+\\)' } | Select-Object Name, PNPDeviceID, Manufacturer); \
    $a = @(Get-CimInstance Win32_SoundDevice | Select-Object Name); \
    [pscustomobject]@{ ports = $p; audio = $a } | ConvertTo-Json -Depth 4 -Compress";

/// Interroga Windows (PowerShell nascosto, sola lettura).
pub fn read_system() -> Result<(Vec<SerialPortInfo>, Vec<String>), RigError> {
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SYSTEM_SCRIPT]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // niente finestra nera
        command.creation_flags(0x0800_0000);
    }
    let out = command.output().map_err(|e| RigError(format!("PowerShell non avviabile: {e}")))?;
    if !out.status.success() {
        return Err(RigError("PowerShell non ha risposto".into()));
    }
    parse_system(String::from_utf8_lossy(&out.stdout).trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(port: &str, name: &str, pnp: &str) -> SerialPortInfo {
        SerialPortInfo {
            port: port.into(),
            name: name.into(),
            instance_id: pnp.into(),
            vid: hex4(pnp, "VID_"),
            pid: hex4(pnp, "PID_"),
            serial: serial_of(pnp),
            manufacturer: String::new(),
            virtual_port: false,
        }
    }

    const SAMPLE: &str = r#"{"ports":[
        {"Name":"Silicon Labs Dual CP210x USB to UART Bridge: Enhanced COM Port (COM5)","PNPDeviceID":"USB\\VID_10C4&PID_EA70&MI_00\\7&2A8AB6E4&0&0000","Manufacturer":"Silicon Labs"},
        {"Name":"Silicon Labs Dual CP210x USB to UART Bridge: Standard COM Port (COM6)","PNPDeviceID":"USB\\VID_10C4&PID_EA70&MI_01\\7&2A8AB6E4&0&0001","Manufacturer":"Silicon Labs"},
        {"Name":"USB Serial Port (COM3)","PNPDeviceID":"FTDIBUS\\VID_0403+PID_6001+A50285BIA\\0000","Manufacturer":"FTDI"},
        {"Name":"com0com - serial port emulator (COM20)","PNPDeviceID":"COM0COM\\PORT\\CNCA0","Manufacturer":"com0com"},
        {"Name":"Standard Serial over Bluetooth link (COM7)","PNPDeviceID":"BTHENUM\\{00001101-0000-1000-8000-00805F9B34FB}_LOCALMFG&0000\\7&1&0&000000000000_00000000","Manufacturer":"Microsoft"}
      ],
      "audio":[{"Name":"USB Audio CODEC"},{"Name":"VB-Audio Virtual Cable"},{"Name":"Realtek High Definition Audio"}]}"#;

    #[test]
    fn it_reads_the_windows_answer() {
        let (ports, audio) = parse_system(SAMPLE).expect("lettura");
        assert_eq!(ports.len(), 5);
        let enhanced = &ports[1];
        assert_eq!((enhanced.port.as_str(), enhanced.vid, enhanced.pid), ("COM5", Some(0x10C4), Some(0xEA70)));
        assert!(enhanced.name.contains("Enhanced") && !enhanced.name.contains("COM5"));
        let ftdi = ports.iter().find(|p| p.port == "COM3").expect("FTDI");
        assert_eq!(ftdi.serial.as_deref(), Some("A50285BI"));
        assert!(ports.iter().find(|p| p.port == "COM20").expect("com0com").virtual_port);
        assert!(ports.iter().find(|p| p.port == "COM7").expect("bluetooth").virtual_port);
        assert_eq!(audio, ["USB Audio CODEC", "VB-Audio Virtual Cable", "Realtek High Definition Audio"]);
    }

    #[test]
    fn a_single_port_is_not_a_list_for_powershell() {
        let one = r#"{"ports":{"Name":"USB-SERIAL CH340 (COM4)","PNPDeviceID":"USB\\VID_1A86&PID_7523\\5&1&0&2","Manufacturer":"wch.cn"},"audio":{"Name":"USB PnP Sound Device"}}"#;
        let (ports, audio) = parse_system(one).expect("lettura");
        assert_eq!((ports.len(), audio.len()), (1, 1));
        assert!(parse_system("non e' json").is_err());
        assert_eq!(parse_system(r#"{"ports":null,"audio":null}"#).expect("vuoto"), (Vec::new(), Vec::new()));
    }

    #[test]
    fn the_dual_cp2105_is_one_yaesu_with_the_enhanced_port_as_cat() {
        let (ports, audio) = parse_system(SAMPLE).expect("lettura");
        let result = detect(&ports, &audio);
        let yaesu = result.candidates.iter().find(|c| c.is_radio).expect("Yaesu");
        assert_eq!((yaesu.cat_port.as_str(), yaesu.other_ports.as_slice()), ("COM5", ["COM6".to_string()].as_slice()));
        assert_eq!((yaesu.protocol.as_str(), yaesu.baud, yaesu.confidence), ("newcat", Some(38400), 89));
        assert!(yaesu.alternatives.iter().any(|a| a == "FT-2000"), "la famiglia comprende la FT-2000 (SCU-17)");
        assert_eq!(yaesu.audio_input.as_deref(), Some("USB Audio CODEC"), "la scheda audio virtuale e quella della scheda madre sono escluse");
        // le porte virtuali non sono mai candidate
        assert!(result.candidates.iter().all(|c| c.cat_port != "COM20" && c.cat_port != "COM7"));
        // l'adattatore FTDI e' solo un'interfaccia: fiducia 35, modello a mano
        let ftdi = result.candidates.iter().find(|c| c.cat_port == "COM3").expect("FTDI");
        assert!(!ftdi.is_radio && ftdi.confidence == 35 && ftdi.rig_label.contains("a mano"));
        assert_eq!(result.candidates[0].cat_port, "COM5", "ordine di fiducia");
    }

    #[test]
    fn the_ft2000_is_recognised_by_name() {
        let ports = [port("COM9", "Yaesu FT-2000 CAT", "USB\\VID_067B&PID_2303\\5&9&0&1")];
        let c = &detect(&ports, &[]).candidates[0];
        assert!(c.is_radio);
        assert_eq!((c.rig_label.as_str(), c.rig_token.as_str(), c.protocol.as_str(), c.confidence), ("Yaesu FT-2000", "FT-2000", "newcat", 90));
        // anche dal numero di serie, come lascia scritto il driver di certe interfacce
        let mut p = port("COM2", "USB Serial Port", "USB\\VID_0403&PID_6001\\FT2000A");
        p.manufacturer = "FTDI".into();
        let c = &detect(&[p], &[]).candidates[0];
        assert_eq!(c.rig_token, "FT-2000");
    }

    #[test]
    fn the_common_models_are_recognised_by_name() {
        let cases = [
            ("Icom IC-7300 USB", "IC-7300", Some(0x94)),
            ("IC-7610 Serial Port A", "IC-7610", Some(0x98)),
            ("Icom IC-9700", "IC-9700", Some(0xA2)),
            ("IC-705 CI-V", "IC-705", Some(0xA4)),
            ("FT-991A CAT", "FT-991A", None),
            ("FTDX10 Enhanced", "FT-DX10", None),
            ("FT-DX101D", "FT-DX101", None),
            ("FT-710", "FT-710", None),
            ("Kenwood TS-590SG", "TS-590S", None),
            ("TS-890S", "TS-890S", None),
            ("Elecraft KX3", "KX3", None),
            ("Xiegu G90", "G90", Some(0x70)),
            ("QRP Labs QMX", "QMX", None),
            ("FlexRadio SmartSDR CAT", "Flex", None),
        ];
        for (name, token, civ) in cases {
            let c = &detect(&[port("COM4", name, "USB\\VID_10C4&PID_EA60\\0001")], &[]).candidates[0];
            assert_eq!((c.rig_token.as_str(), c.civ_address, c.is_radio), (token, civ, true), "{name}");
        }
    }

    #[test]
    fn an_unknown_device_is_only_listed_and_a_uncertain_model_is_not_invented() {
        let ports = [port("COM1", "Communications Port", "ACPI\\PNP0501\\1")];
        let c = &detect(&ports, &[]).candidates[0];
        assert!(!c.is_radio && c.confidence == 15 && c.rig_token.is_empty());
        // Icom con interfaccia propria: la famiglia, non il modello
        let c = &detect(&[port("COM8", "Icom CI-V port", "USB\\VID_0C26&PID_0020\\IC1")], &[]).candidates[0];
        assert!(c.is_radio && c.rig_token.is_empty() && c.confidence == 60 && c.alternatives.len() >= 3);
        assert!(detect(&[], &[]).candidates.is_empty());
    }

    #[test]
    fn several_unknown_ports_of_one_device_are_separate_candidates() {
        let ports = [
            port("COM11", "Quad serial (A)", "USB\\VID_0403&PID_6011&MI_00\\6&1A2B&0&0000"),
            port("COM12", "Quad serial (B)", "USB\\VID_0403&PID_6011&MI_01\\6&1A2B&0&0001"),
        ];
        let result = detect(&ports, &[]);
        assert_eq!(result.candidates.len(), 2);
        assert_eq!(device_key(&ports[0]), device_key(&ports[1]), "stesso apparato fisico");
    }

    #[test]
    fn the_device_key_survives_a_com_renumbering() {
        let a = port("COM5", "x", "USB\\VID_10C4&PID_EA70\\SER123");
        let b = port("COM9", "x", "USB\\VID_10C4&PID_EA70\\SER123");
        assert_eq!(device_key(&a), device_key(&b));
        assert_eq!(device_key(&port("COM1", "x", "ACPI\\PNP0501\\1")), "porta:COM1");
    }

    /// Sul PC vero: l'elenco si legge senza errori (porte e schede audio qualunque siano). Si salta fuori da Windows.
    #[test]
    fn the_real_system_can_be_read() {
        if !cfg!(windows) {
            return;
        }
        let (ports, _audio) = read_system().expect("lettura del sistema");
        for p in &ports {
            assert!(p.port.starts_with("COM"), "{p:?}");
        }
    }
}
