//! La porta seriale della radio che non si apre perche' la tiene un altro programma («Accesso negato», os error 5 su Windows).
//!
//! Rapporti del 2026-11-11 (0.11.236/0.11.237): «Accendi»: porta COM8 non aperta: Accesso negato (os error 5)», poi nemmeno Hamlib apriva la
//! porta. Chi scriveva aveva usato OmniRig, rimasto aperto con la COM8. Su Windows una porta seriale si apre una volta sola: finche' un altro
//! programma la tiene, ogni apertura risponde «accesso negato». Non e' un guasto di Aurora ma della configurazione della stazione: invece di
//! un rapporto d'errore l'operatore riceve un avviso che dice quale programma la tiene (dall'elenco dei processi di Windows, Toolhelp32, senza
//! PowerShell ne' tasklist) e cosa fare. Se a tenerla e' un `rigctld` rimasto orfano da un Aurora chiuso male, lo si chiude e si riprova.

use crate::RigError;

/// I programmi che di solito tengono la porta della radio: nome del file (senza `.exe`, minuscolo), se basta l'inizio del nome, e il nome
/// da mostrare. L'ordine e' quello di precedenza quando ne girano piu' d'uno: OmniRig per primo (ha un consiglio suo).
const KNOWN: &[(&str, bool, &str)] = &[
    ("omnirig", true, OMNIRIG),
    ("wsjtx", true, "WSJT-X"),
    ("jt9", false, "WSJT-X"),
    ("jtdx", true, "JTDX"),
    ("mshv", true, "MSHV"),
    ("decodium", true, "Decodium"),
    ("js8call", true, "JS8Call"),
    ("cat4om", true, "Cat4OM"),
    ("l4ong", true, "Log4OM"),
    ("log4om", true, "Log4OM"),
    ("hamradiodeluxe", true, "Ham Radio Deluxe"),
    ("hrdlogbook", true, "Ham Radio Deluxe"),
    ("n1mmlogger", true, "N1MM Logger+"),
    ("fldigi", true, "fldigi"),
    ("flrig", true, "flrig"),
    ("commander", false, "DXLab Commander"),
    ("logger32", true, "Logger32"),
    ("rigctld", false, RIGCTLD),
];

const OMNIRIG: &str = "OmniRig";
const RIGCTLD: &str = "rigctld (Hamlib)";

/// Un processo dell'elenco di sistema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub parent: u32,
    /// Il nome del file del programma (`OmniRig.exe`).
    pub name: String,
}

/// Chi tiene la porta, per quanto si puo' sapere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holder {
    /// Un `rigctld` il cui programma padre non c'e' piu' (un Aurora chiuso male): si puo' chiudere e riprovare.
    OrphanRigctld,
    /// Un programma noto che di solito tiene la porta della radio.
    Program(&'static str),
    /// Nessun programma noto in esecuzione.
    Unknown,
}

/// Il nome del file senza `.exe`, in minuscolo.
fn stem(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    lower.strip_suffix(".exe").map(str::to_string).unwrap_or(lower)
}

/// Il programma noto con questo nome di file, se e' tra quelli che tengono la porta della radio.
pub fn known_program(file_name: &str) -> Option<&'static str> {
    let s = stem(file_name);
    KNOWN.iter().find(|(key, prefix, _)| if *prefix { s.starts_with(key) } else { s == *key }).map(|(_, _, name)| *name)
}

fn is_rigctld(p: &Process) -> bool {
    stem(&p.name) == "rigctld"
}

/// Chi tiene la porta, dall'elenco dei processi (`own` = il processo di Aurora, che non conta): i programmi noti nell'ordine di [`KNOWN`].
/// Un `rigctld` lanciato da Aurora stessa non conta, uno lanciato da un altro programma noto vale quel programma, uno orfano o di un
/// programma sconosciuto vale «rigctld». Gli orfani di Aurora non arrivano qui: li chiude [`diagnose_open_error`] prima della diagnosi (un
/// orfano qualunque non deve passare davanti al programma che tiene davvero la porta).
pub fn holder(processes: &[Process], own: u32) -> Holder {
    let mut found: Option<(usize, &'static str)> = None;
    for p in processes.iter().filter(|p| p.pid != own) {
        let name = if is_rigctld(p) {
            if p.parent == own {
                continue;
            }
            // il rigctld di un altro programma: conta il programma che l'ha lanciato, se e' noto
            processes.iter().find(|q| q.pid == p.parent).and_then(|q| known_program(&q.name)).unwrap_or(RIGCTLD)
        } else {
            match known_program(&p.name) {
                Some(name) => name,
                None => continue,
            }
        };
        let rank = KNOWN.iter().position(|(_, _, n)| *n == name).unwrap_or(KNOWN.len());
        if found.is_none_or(|(r, _)| rank < r) {
            found = Some((rank, name));
        }
    }
    found.map_or(Holder::Unknown, |(_, name)| Holder::Program(name))
}

/// L'errore di apertura dice che la porta e' presa da un altro programma (accesso negato, condivisione violata, risorsa occupata), non che
/// manca o che la radio non risponde.
pub fn is_access_denied(message: &str) -> bool {
    let m = message.to_lowercase();
    const WINDOWS: &[&str] = &[
        "(os error 5)",
        "(os error 32)",
        "accesso negato",
        "access denied",
        "access is denied",
        "permission denied",
        "0x80070005",
        "0x80070020",
        "sharing violation",
        "utilizzato da un altro processo",
        "used by another process",
    ];
    // EBUSY (16) e i testi di porta gia' aperta valgono ovunque
    const ANYWHERE: &[&str] = &["already open", "resource busy"];
    let unix_busy = !cfg!(windows) && m.contains("(os error 16)");
    unix_busy || ANYWHERE.iter().any(|k| m.contains(k)) || (cfg!(windows) && WINDOWS.iter().any(|k| m.contains(k)))
}

/// Il nome della porta da mostrare: `\\.\COM12` diventa `COM12`.
pub fn port_label(port: &str) -> String {
    let p = port.trim().trim_start_matches(r"\\.\");
    if p.len() > 3 && p.get(..3).is_some_and(|s| s.eq_ignore_ascii_case("com")) { p.to_ascii_uppercase() } else { p.to_string() }
}

/// Il testo per l'operatore (l'interfaccia lo traduce: `app/src/lib/coreText.ts`, chiavi `core.portHeld*`).
pub fn busy_message(port: &str, holder: &Holder) -> String {
    match holder {
        Holder::Program(name) if *name == OMNIRIG => {
            format!("La porta {} e' occupata da OmniRig: chiudi OmniRig oppure scegli OmniRig come sistema di controllo in Impostazioni › Stazione › Radio", port_label(port))
        }
        _ => held_message(port, holder, "premi Collega"),
    }
}

/// Lo stesso testo per la porta del PTT su seriale (RTS/DTR): quella porta si apre solo all'avvio di Aurora, e «Collega» ricollega il CAT,
/// non il PTT; chiuso l'altro programma si riavvia Aurora (chiavi `core.pttPortHeld*`).
pub fn ptt_busy_message(port: &str, holder: &Holder) -> String {
    held_message(port, holder, "riavvia Aurora")
}

/// «La porta X e' occupata da Y: chiudi Y e `then`».
fn held_message(port: &str, holder: &Holder, then: &str) -> String {
    let port = port_label(port);
    match holder {
        Holder::Program(name) => format!("La porta {port} e' occupata da {name}: chiudi {name} e {then}"),
        // un rigctld orfano che non si e' potuto chiudere (non di Aurora, o senza permessi)
        Holder::OrphanRigctld => format!("La porta {port} e' occupata da {RIGCTLD}: chiudi {RIGCTLD} e {then}"),
        Holder::Unknown => format!("La porta {port} e' occupata da un altro programma (WSJT-X, JTDX, il log...): chiudilo e {then}"),
    }
}

/// Il messaggio e' una diagnosi di porta occupata scritta da [`busy_message`] (va lasciato com'e', non riscritto da `friendly_error`).
pub fn is_busy_diagnosis(message: &str) -> bool {
    // «La porta <nome> e' occupata da ...»: il nome e' una parola sola (COM8, /dev/ttyUSB0), mai «seriale» (il testo generico di prima)
    message
        .strip_prefix("La porta ")
        .and_then(|rest| rest.split_once(" e' occupata da "))
        .is_some_and(|(port, _)| !port.is_empty() && !port.contains(' ') && port != "seriale")
}

/// Come `cat_config::friendly_error`, ma i messaggi gia' scritti per l'operatore restano come sono: la diagnosi di porta occupata (dice gia'
/// chi tiene la porta) e i tempi scaduti di OmniRig (sostituiti dal consiglio generico di Hamlib dicevano «controlla baud e cavo»).
pub fn friendly_error(raw: &str) -> String {
    if is_busy_diagnosis(raw) || crate::omnirig::is_timeout(raw) { raw.to_string() } else { crate::cat_config::friendly_error(raw) }
}

/// Chi tiene la porta adesso (elenco dei processi di sistema).
pub fn diagnose() -> Holder {
    holder(&processes(), std::process::id())
}

/// Dopo un'apertura negata: chi tiene la porta e il messaggio per l'operatore. Un `rigctld` orfano si chiude: `retry` dice allora di
/// riprovare una volta.
pub struct Diagnosis {
    pub holder: Holder,
    pub message: String,
    pub retry: bool,
}

/// La diagnosi di una porta che non si apre per accesso negato; `None` se l'errore e' un altro (porta assente, radio muta...).
pub fn diagnose_open_error(port: &str, error: &str) -> Option<Diagnosis> {
    if !is_access_denied(error) {
        return None;
    }
    // prima si chiudono gli orfani di Aurora (rimasti da un Aurora chiuso male): se ce n'era uno, era lui a tenere la porta e si riprova
    let retry = close_orphan_rigctld();
    let holder = if retry { Holder::OrphanRigctld } else { diagnose() };
    let message = busy_message(port, &holder);
    tracing::info!("porta {} non aperta ({error}): {message}", port_label(port));
    Some(Diagnosis { holder, message, retry })
}

/// Apre la porta con `open`; se l'apertura e' negata dice chi la tiene e, se era un `rigctld` orfano di Aurora (chiuso), riprova una volta.
/// L'errore torna con la sua diagnosi (`None` se non e' una porta occupata). `describe` scrive l'errore per [`is_access_denied`].
pub fn open_with_diagnosis<T, E>(port: &str, open: impl Fn() -> Result<T, E>, describe: impl Fn(&E) -> String) -> Result<T, (E, Option<Diagnosis>)> {
    let e = match open() {
        Ok(t) => return Ok(t),
        Err(e) => e,
    };
    let Some(d) = diagnose_open_error(port, &describe(&e)) else { return Err((e, None)) };
    if !d.retry {
        return Err((e, Some(d)));
    }
    open().map_err(|again| {
        let d = diagnose_open_error(port, &describe(&again));
        (again, d)
    })
}

/// Manda l'avviso a schermo (non un rapporto d'errore: e' la configurazione della stazione).
pub fn notify(message: &str) {
    aurora_core::notice::notice("port_busy", message);
}

impl Diagnosis {
    /// L'errore per chi chiama, con il testo della diagnosi.
    pub fn error(&self) -> RigError {
        RigError(self.message.clone())
    }
}

/// L'elenco dei processi in esecuzione (Windows: Toolhelp32; Linux: `/proc`; altrove vuoto).
pub fn processes() -> Vec<Process> {
    #[cfg(windows)]
    {
        sys::processes()
    }
    #[cfg(target_os = "linux")]
    {
        linux_processes()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
fn linux_processes() -> Vec<Process> {
    let Ok(dir) = std::fs::read_dir("/proc") else { return Vec::new() };
    dir.filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()))
        .filter_map(|pid| {
            // «pid (nome) stato ppid ...»: il nome puo' contenere spazi e parentesi, si prende fino all'ultima «)»
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            let (open, close) = (stat.find('(')?, stat.rfind(')')?);
            let name = stat.get(open + 1..close)?.to_string();
            let parent = stat.get(close + 1..)?.split_whitespace().nth(1)?.parse().ok()?;
            Some(Process { pid, parent, name })
        })
        .collect()
}

/// Chiude i `rigctld` rimasti orfani di Aurora (il programma che li aveva avviati non c'e' piu', per esempio dopo un crash). Solo quelli
/// del pacchetto di Aurora (percorso con «aurora»): un `rigctld` avviato a mano dall'operatore non si tocca. Dice se ne ha chiuso almeno
/// uno. Solo Windows; altrove non fa nulla (su Linux gli orfani passano a init e non si riconoscono).
pub fn close_orphan_rigctld() -> bool {
    #[cfg(windows)]
    {
        let list = sys::processes();
        let own = std::process::id();
        let alive = |pid: u32| pid != 0 && list.iter().any(|p| p.pid == pid);
        let mut closed = 0;
        let ours = aurora_rigctld_dirs();
        for p in list.iter().filter(|p| is_rigctld(p) && p.parent != own && !alive(p.parent)) {
            match sys::terminate_if_aurora(p.pid, &ours) {
                Ok(true) => closed += 1,
                Ok(false) => tracing::info!("rigctld orfano {} non di Aurora: lasciato", p.pid),
                Err(e) => tracing::info!("rigctld orfano {} non chiuso: {e}", p.pid),
            }
        }
        // niente attesa fissa: `terminate_if_aurora` aspetta che ogni processo sia uscito davvero (porta libera)
        if closed > 0 {
            tracing::info!("chiusi {closed} rigctld rimasti orfani");
        }
        closed > 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Le cartelle dove Aurora cerca il suo `rigctld` (accanto al programma, la sua sottocartella `hamlib`, l'Hamlib ripristinato), in minuscolo
/// e con la barra finale: un `rigctld` orfano si chiude solo se il suo programma sta qui.
#[cfg(windows)]
fn aurora_rigctld_dirs() -> Vec<String> {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(std::path::Path::to_path_buf)) {
        dirs.push(dir);
    }
    dirs.extend(crate::hamlib_restore::restored_dir());
    dirs.iter()
        .map(|d| {
            let mut s = d.to_string_lossy().to_lowercase().replace('/', "\\");
            if !s.ends_with('\\') {
                s.push('\\');
            }
            s
        })
        .collect()
}

#[cfg(windows)]
mod sys {
    use windows::{
        Win32::{
            Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
            System::{
                Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS},
                Threading::{
                    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess,
                    WaitForSingleObject,
                },
            },
        },
        core::PWSTR,
    };

    use super::Process;

    /// Quanto si aspetta al massimo che un `rigctld` chiuso sia uscito davvero (e abbia lasciato la porta): di solito pochi millisecondi.
    const EXIT_WAIT_MS: u32 = 2000;

    /// Una maniglia che si chiude da sola.
    struct Owned(HANDLE);

    impl Drop for Owned {
        fn drop(&mut self) {
            // SAFETY: la maniglia e' stata aperta qui e si chiude una volta sola.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn processes() -> Vec<Process> {
        // SAFETY: fotografia dei processi del sistema, nessun puntatore passato.
        let Ok(snap) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else { return Vec::new() };
        let snap = Owned(snap);
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut out = Vec::new();
        // SAFETY: `entry` e' inizializzata con la sua dimensione e vive per tutte le chiamate.
        let mut more = unsafe { Process32FirstW(snap.0, &mut entry) }.is_ok();
        while more {
            let len = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
            out.push(Process { pid: entry.th32ProcessID, parent: entry.th32ParentProcessID, name: String::from_utf16_lossy(&entry.szExeFile[..len]) });
            // SAFETY: come sopra.
            more = unsafe { Process32NextW(snap.0, &mut entry) }.is_ok();
        }
        out
    }

    /// Chiude il processo se il suo programma sta in una delle cartelle di Aurora (`ours`, in minuscolo con la barra finale); `Ok(false)` se
    /// e' un altro. Il percorso si legge dalla maniglia gia' aperta: un numero di processo riusato nel frattempo non inganna il controllo.
    pub(super) fn terminate_if_aurora(pid: u32, ours: &[String]) -> Result<bool, String> {
        // SAFETY: apertura di un processo per numero; la maniglia si chiude con `Owned`.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) }.map_err(|e| e.message())?;
        let handle = Owned(handle);
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        // SAFETY: `buf` e `len` descrivono lo stesso spazio, valido per tutta la chiamata.
        unsafe { QueryFullProcessImageNameW(handle.0, PROCESS_NAME_FORMAT(0), PWSTR(buf.as_mut_ptr()), &mut len) }.map_err(|e| e.message())?;
        let path = String::from_utf16_lossy(&buf[..(len as usize).min(buf.len())]).to_lowercase();
        if !ours.iter().any(|dir| path.starts_with(dir.as_str())) {
            return Ok(false);
        }
        // SAFETY: maniglia aperta con PROCESS_TERMINATE.
        unsafe { TerminateProcess(handle.0, 1) }.map_err(|e| e.message())?;
        // Windows libera la porta quando il processo e' uscito (le sue maniglie si chiudono prima che diventi segnalato): si aspetta quello,
        // con un tempo massimo, invece di una pausa fissa
        // SAFETY: maniglia aperta con PROCESS_SYNCHRONIZE, valida finche' vive `handle`.
        if unsafe { WaitForSingleObject(handle.0, EXIT_WAIT_MS) } != WAIT_OBJECT_0 {
            tracing::info!("rigctld orfano {pid} chiuso ma non ancora uscito dopo {EXIT_WAIT_MS} ms");
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, parent: u32, name: &str) -> Process {
        Process { pid, parent, name: name.into() }
    }

    #[test]
    fn process_names_map_to_programs() {
        assert_eq!(known_program("OmniRig.exe"), Some("OmniRig"));
        assert_eq!(known_program("wsjtx.exe"), Some("WSJT-X"));
        assert_eq!(known_program("jt9.exe"), Some("WSJT-X"));
        assert_eq!(known_program("jt9_helper.exe"), None, "jt9 solo con il nome esatto");
        assert_eq!(known_program("JTDX.EXE"), Some("JTDX"));
        assert_eq!(known_program("L4ONG.exe"), Some("Log4OM"));
        assert_eq!(known_program("Log4OM_Communicator.exe"), Some("Log4OM"));
        assert_eq!(known_program("HamRadioDeluxe.exe"), Some("Ham Radio Deluxe"));
        assert_eq!(known_program("N1MMLogger.net.exe"), Some("N1MM Logger+"));
        assert_eq!(known_program("js8call"), Some("JS8Call"), "su Linux senza .exe");
        assert_eq!(known_program("explorer.exe"), None);
        assert_eq!(known_program("TOTALCMD64.EXE"), None);
    }

    #[test]
    fn omnirig_wins_over_the_others_and_aurora_itself_does_not_count() {
        let list = [p(10, 1, "wsjtx.exe"), p(11, 1, "OmniRig.exe"), p(4, 1, "explorer.exe"), p(99, 4, "aurora.exe")];
        assert_eq!(holder(&list, 99), Holder::Program("OmniRig"));
        assert_eq!(holder(&list[..1], 99), Holder::Program("WSJT-X"));
        assert_eq!(holder(&[p(4, 1, "explorer.exe")], 99), Holder::Unknown);
    }

    #[test]
    fn rigctld_orphan_ours_or_of_another_program() {
        // padre morto: un orfano qualunque vale «rigctld» (quelli di Aurora li chiude diagnose_open_error prima della diagnosi)
        assert_eq!(holder(&[p(20, 777, "rigctld.exe")], 99), Holder::Program("rigctld (Hamlib)"));
        // e non passa davanti al programma noto che tiene la porta
        assert_eq!(holder(&[p(20, 777, "rigctld.exe"), p(11, 1, "OmniRig.exe")], 99), Holder::Program("OmniRig"));
        // lanciato da Aurora stessa: non conta
        assert_eq!(holder(&[p(99, 4, "aurora.exe"), p(20, 99, "rigctld.exe"), p(4, 1, "explorer.exe")], 99), Holder::Unknown);
        // lanciato da un programma noto: conta quel programma
        assert_eq!(holder(&[p(30, 4, "flrig.exe"), p(20, 30, "rigctld.exe"), p(4, 1, "explorer.exe")], 99), Holder::Program("flrig"));
        // lanciato da un programma sconosciuto ancora vivo
        assert_eq!(holder(&[p(30, 4, "cmd.exe"), p(20, 30, "rigctld.exe"), p(4, 1, "explorer.exe")], 99), Holder::Program("rigctld (Hamlib)"));
    }

    #[test]
    fn only_access_denied_is_a_busy_port() {
        // i codici e i testi di Windows valgono solo su Windows: su Linux «Permission denied» e' il gruppo dialout, 5 e' EIO
        assert_eq!(is_access_denied("Accesso negato. (os error 5)"), cfg!(windows));
        assert_eq!(is_access_denied("Access is denied. (os error 5)"), cfg!(windows));
        assert_eq!(is_access_denied("rigctld si e' fermato (exit code: 1): serial_open: Unable to open COM8 - Permission denied"), cfg!(windows));
        assert_eq!(is_access_denied("rig_open: Access denied"), cfg!(windows));
        assert_eq!(is_access_denied("porta \\\\.\\COM8 non apribile (occupata o assente): Accesso negato. (0x80070005)"), cfg!(windows));
        assert!(is_access_denied("serial_open: Device or resource busy"));
        assert!(is_access_denied("Device or resource busy (os error 16)"));
        assert!(!is_access_denied("Impossibile trovare il file specificato. (os error 2)"));
        assert!(!is_access_denied("rigctld si e' fermato (exit code: 1): rig_open: error = Communication timed out"));
        assert!(!is_access_denied("(os error 50)"), "solo il codice 5, non quelli che iniziano per 5");
    }

    #[test]
    fn the_message_names_the_holder_and_what_to_do() {
        assert_eq!(
            busy_message(r"\\.\COM8", &Holder::Program("OmniRig")),
            "La porta COM8 e' occupata da OmniRig: chiudi OmniRig oppure scegli OmniRig come sistema di controllo in Impostazioni › Stazione › Radio"
        );
        assert_eq!(busy_message("com12", &Holder::Program("JTDX")), "La porta COM12 e' occupata da JTDX: chiudi JTDX e premi Collega");
        let generic = busy_message("COM3", &Holder::Unknown);
        assert_eq!(generic, "La porta COM3 e' occupata da un altro programma (WSJT-X, JTDX, il log...): chiudilo e premi Collega");
        assert_eq!(busy_message("/dev/ttyUSB0", &Holder::Unknown), generic.replace("COM3", "/dev/ttyUSB0"));
        assert_eq!(busy_message("COM4", &Holder::OrphanRigctld), "La porta COM4 e' occupata da rigctld (Hamlib): chiudi rigctld (Hamlib) e premi Collega");
    }

    /// La porta del PTT su seriale si apre all'avvio: il consiglio e' riavviare Aurora, non premere «Collega» (che ricollega solo il CAT).
    #[test]
    fn the_serial_ptt_message_says_to_restart_aurora() {
        assert_eq!(ptt_busy_message(r"\\.\COM5", &Holder::Program("JTDX")), "La porta COM5 e' occupata da JTDX: chiudi JTDX e riavvia Aurora");
        assert_eq!(ptt_busy_message("COM5", &Holder::Program("OmniRig")), "La porta COM5 e' occupata da OmniRig: chiudi OmniRig e riavvia Aurora");
        assert_eq!(ptt_busy_message("COM5", &Holder::OrphanRigctld), "La porta COM5 e' occupata da rigctld (Hamlib): chiudi rigctld (Hamlib) e riavvia Aurora");
        let generic = ptt_busy_message("COM5", &Holder::Unknown);
        assert_eq!(generic, "La porta COM5 e' occupata da un altro programma (WSJT-X, JTDX, il log...): chiudilo e riavvia Aurora");
        assert!(!generic.contains("Collega"));
        assert!(is_busy_diagnosis(&generic), "resta una diagnosi di porta occupata (avviso a schermo, non rapporto d'errore)");
    }

    #[test]
    fn a_diagnosis_survives_the_friendly_rewrite() {
        let d = busy_message("COM8", &Holder::Program("OmniRig"));
        assert!(is_busy_diagnosis(&d));
        assert_eq!(friendly_error(&d), d);
        assert_eq!(friendly_error("rig_open: Access denied"), crate::cat_config::friendly_error("rig_open: Access denied"));
        assert!(!is_busy_diagnosis("La porta seriale e' occupata da un altro programma: chiudilo (o scegli un'altra porta) e riprova."));
    }

    #[test]
    fn an_open_that_is_not_denied_has_no_diagnosis() {
        let missing = open_with_diagnosis("COM99", || Err::<(), _>("Impossibile trovare il file specificato. (os error 2)"), |e| e.to_string());
        assert!(matches!(missing, Err((_, None))));
        assert_eq!(open_with_diagnosis("COM1", || Ok::<_, String>(7), |e| e.clone()).ok(), Some(7));
    }

    #[test]
    fn the_process_list_is_readable_here() {
        let own = std::process::id();
        let list = processes();
        if cfg!(any(windows, target_os = "linux")) {
            assert!(list.iter().any(|p| p.pid == own), "il processo delle prove deve esserci");
        }
        // la diagnosi gira senza errori, qualunque cosa ci sia sul PC
        let _ = diagnose();
    }
}
