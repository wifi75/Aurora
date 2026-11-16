//! L'antivirus del PC e i segni che ha tolto dei file ad Aurora: gli antivirus (Avast Free e altri) mettono in quarantena o cancellano
//! `aurora-desktop.exe` o `hamlib\rigctld.exe` e senza questi segni non lo sapremmo.
//!
//! Il nome dell'antivirus si legge dal registro, dove Windows tiene l'elenco dei prodotti registrati nel Centro sicurezza
//! (`HKLM\SOFTWARE\Microsoft\Security Center\Provider\Av\<GUID>`, valori `DISPLAYNAME` e `STATE`): lo stesso elenco di WMI
//! `root\SecurityCenter2` ma con `reg.exe` senza finestra (come il resto di `support.rs`), senza COM e senza dipendenze nuove, e senza
//! PowerShell nascosto che gli antivirus segnalano. Solo lettura, nessun dato personale: e' il nome commerciale del prodotto.

use std::{
    path::Path,
    sync::OnceLock,
    time::Duration,
};

/// Il segno che la copia di aiuto dell'aggiornamento lascia in `%APPDATA%\Aurora` quando, finita l'installazione, manca qualche file:
/// Aurora lo trova all'avvio, lo manda come errore di area `antivirus` e lo toglie.
pub const SEGNO: &str = "antivirus-sospetto.txt";
/// Un segno piu' grande di cosi' non e' nostro: non si legge.
const MAX_SEGNO: u64 = 8 * 1024;
/// Quanto dopo l'installatore si aspetta prima di controllare i file: l'antivirus li esamina appena scritti e li toglie nei secondi dopo.
pub const ATTESA_DOPO_INSTALLAZIONE: Duration = Duration::from_secs(8);
const MAX_NOME: usize = 120;

/// I file che devono esserci accanto al programma dopo un'installazione (gli stessi che l'installatore controlla dopo la copia, vedi
/// `AuroraDopoLaCopia` in `desktop/src-tauri/installer/hooks.nsh`), oltre al programma stesso. Un componente per voce.
const FILE_PRINCIPALI: [&[&str]; 5] = [&["dist", "index.html"], &["hamlib", "rigctld.exe"], &["hamlib", "libhamlib-4.dll"], &["hamlib", "rotctld.exe"], &["cty", "cty.dat"]];

static NOME: OnceLock<String> = OnceLock::new();

/// L'antivirus installato, letto una sola volta (aspetta la lettura se e' in corso). Vuoto se non si legge o fuori da Windows.
pub fn nome() -> &'static str {
    NOME.get_or_init(leggi_nome).as_str()
}

/// Come [`nome`] ma senza aspettare: vuoto finche' la lettura non e' finita (per chi risponde a una richiesta e non deve bloccarsi).
pub fn nome_se_pronto() -> String {
    NOME.get().cloned().unwrap_or_default()
}

/// Avvia la lettura in un thread, cosi' al primo rapporto il nome e' gia' pronto.
pub fn avvia_lettura() {
    let _ = std::thread::Builder::new().name("antivirus".into()).spawn(|| {
        nome();
    });
}

#[cfg(windows)]
fn leggi_nome() -> String {
    let mut command = std::process::Command::new("reg");
    command.args(["query", r"HKLM\SOFTWARE\Microsoft\Security Center\Provider\Av", "/s"]);
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let Ok(out) = command.output() else { return String::new() };
    unisci(&scegli(&leggi_prodotti(&String::from_utf8_lossy(&out.stdout))))
}

#[cfg(not(windows))]
fn leggi_nome() -> String {
    String::new()
}

/// Un prodotto antivirus registrato in Windows: il nome e, se c'e', lo stato (`STATE`).
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Prodotto {
    nome: String,
    stato: Option<u32>,
}

/// I prodotti nell'uscita di `reg query ... /s`: un blocco per chiave (riga `HKEY_...`), con i valori `DISPLAYNAME` e `STATE`.
#[cfg_attr(not(windows), allow(dead_code))]
fn leggi_prodotti(testo: &str) -> Vec<Prodotto> {
    let mut prodotti = Vec::new();
    let mut nome = String::new();
    let mut stato = None;
    let mut chiudi = |nome: &mut String, stato: &mut Option<u32>| {
        if !nome.trim().is_empty() {
            prodotti.push(Prodotto { nome: std::mem::take(nome), stato: stato.take() });
        }
        nome.clear();
        *stato = None;
    };
    for riga in testo.lines() {
        if riga.starts_with("HKEY_") {
            chiudi(&mut nome, &mut stato);
            continue;
        }
        let mut parti = riga.trim().splitn(3, "    ");
        let (Some(valore), Some(_tipo), Some(dato)) = (parti.next(), parti.next(), parti.next()) else { continue };
        if valore.eq_ignore_ascii_case("DISPLAYNAME") {
            nome = dato.trim().to_string();
        } else if valore.eq_ignore_ascii_case("STATE") {
            stato = dato.trim().strip_prefix("0x").and_then(|h| u32::from_str_radix(h, 16).ok());
        }
    }
    chiudi(&mut nome, &mut stato);
    prodotti
}

/// I prodotti acceso: nello `STATE` (come in `productState` di WMI) il nibble `0x1000` dice «attivo» (`0x41000` Avast acceso,
/// `0x60100` Defender spento perche' c'e' un altro antivirus). Senza stato si conta come acceso. Se nessuno e' acceso si elencano tutti.
#[cfg_attr(not(windows), allow(dead_code))]
fn scegli(prodotti: &[Prodotto]) -> Vec<String> {
    let accesi: Vec<String> = prodotti.iter().filter(|p| p.stato.is_none_or(|s| s & 0xF000 == 0x1000)).map(|p| p.nome.clone()).collect();
    if accesi.is_empty() { prodotti.iter().map(|p| p.nome.clone()).collect() } else { accesi }
}

/// I nomi uniti da «, »: senza vuoti, caratteri di controllo e doppioni, al massimo [`MAX_NOME`] caratteri. Vuoto se non ce ne sono.
pub fn unisci(nomi: &[String]) -> String {
    let mut visti: Vec<String> = Vec::new();
    let mut elenco: Vec<String> = Vec::new();
    for n in nomi {
        let pulito: String = n.chars().filter(|c| !c.is_control()).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
        if pulito.is_empty() || visti.contains(&pulito.to_lowercase()) {
            continue;
        }
        visti.push(pulito.to_lowercase());
        elenco.push(pulito);
    }
    elenco.join(", ").chars().take(MAX_NOME).collect()
}

/// Quali dei file principali mancano in `dir` (la cartella del programma): il programma stesso (`programma`, solo il nome) e i
/// [`FILE_PRINCIPALI`], scritti col percorso di Windows. Con `hamlib_altrove` (un rigctld funzionante c'e', per esempio quello
/// ripristinato nella cartella dei dati) i file di Hamlib non contano: Aurora non ne ha bisogno li' dentro.
pub fn file_mancanti(dir: &Path, programma: &str, hamlib_altrove: bool) -> Vec<String> {
    let mut mancanti = Vec::new();
    if !programma.is_empty() && !dir.join(programma).is_file() {
        mancanti.push(programma.to_string());
    }
    for componenti in FILE_PRINCIPALI {
        if hamlib_altrove && componenti[0] == "hamlib" {
            continue;
        }
        let mut percorso = dir.to_path_buf();
        percorso.extend(componenti);
        if !percorso.is_file() {
            mancanti.push(componenti.join("\\"));
        }
    }
    mancanti
}

/// All'avvio di Aurora installata: i file principali accanto al programma che mancano. Vuoto fuori da Windows, per le prove dalla
/// cartella di sviluppo (`target`) e per chi non e' il programma installato (`aurora.exe` da solo).
pub fn mancanti_all_avvio() -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let Ok(exe) = std::env::current_exe() else { return Vec::new() };
    let Some(programma) = exe.file_name().and_then(|n| n.to_str()).filter(|n| n.eq_ignore_ascii_case("aurora-desktop.exe")) else { return Vec::new() };
    let Some(dir) = exe.parent() else { return Vec::new() };
    if dir.components().any(|c| c.as_os_str().eq_ignore_ascii_case("target")) {
        return Vec::new();
    }
    file_mancanti(dir, programma, rig_control::hamlib::locate_rigctld().is_some())
}

/// Cosa dice il segno lasciato dalla copia di aiuto dell'aggiornamento.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Segno {
    pub data: String,
    /// La versione di Aurora che ha fatto l'aggiornamento (quella vecchia: la copia di aiuto e' una copia sua).
    pub versione: String,
    pub mancava: Vec<String>,
    pub antivirus: String,
}

/// Il testo del segno: righe `chiave=valore`.
pub fn testo_segno(s: &Segno) -> String {
    format!("data={}\nversione={}\nmancava={}\nantivirus={}\n", s.data, s.versione, s.mancava.join("|"), s.antivirus)
}

/// Legge il testo di un segno; `None` se non dice quale file mancava.
pub fn leggi_segno(testo: &str) -> Option<Segno> {
    let mut s = Segno::default();
    for riga in testo.lines() {
        let Some((chiave, valore)) = riga.split_once('=') else { continue };
        let valore = valore.trim();
        match chiave.trim() {
            "data" => s.data = valore.chars().take(10).collect(),
            "versione" => s.versione = valore.chars().take(20).collect(),
            "mancava" => s.mancava = valore.split('|').map(|f| f.trim().chars().take(60).collect::<String>()).filter(|f| !f.is_empty()).take(10).collect(),
            "antivirus" => s.antivirus = valore.chars().take(MAX_NOME).collect(),
            _ => {}
        }
    }
    (!s.mancava.is_empty()).then_some(s)
}

/// Il segno in `dir` come testo, se c'e' ed e' ragionevole.
pub fn leggi_segno_in(dir: &Path) -> Option<Segno> {
    let file = dir.join(SEGNO);
    if std::fs::metadata(&file).ok()?.len() > MAX_SEGNO {
        return None;
    }
    leggi_segno(&std::fs::read_to_string(&file).ok()?)
}

/// Scrive il segno in `dir` (la cartella dei dati): la data e' quella di oggi in UTC, l'antivirus quello letto adesso.
pub fn scrivi_segno(dir: &Path, versione: &str, mancava: &[String]) -> std::io::Result<()> {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (y, m, d, ..) = crate::support::civil(secs);
    let segno = Segno { data: format!("{y:04}-{m:02}-{d:02}"), versione: versione.to_string(), mancava: mancava.to_vec(), antivirus: nome().to_string() };
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(SEGNO), testo_segno(&segno))
}

/// Il messaggio del rapporto per un segno (ancora da ripulire): prima la situazione, poi i dettagli, come fa il registro dell'installatore.
pub fn messaggio_segno(s: &Segno) -> String {
    let da = if s.versione.is_empty() { String::new() } else { format!("; aggiornamento da {}", s.versione) };
    format!("file tolti dopo l'aggiornamento (antivirus?): {} (antivirus: {}{da})", s.mancava.join(", "), if s.antivirus.is_empty() { "non letto" } else { s.antivirus.as_str() })
}

/// Il messaggio del rapporto per i file che mancano all'avvio.
pub fn messaggio_mancanti(mancanti: &[String], antivirus: &str) -> String {
    format!("file di Aurora mancanti all'avvio (antivirus?): {} (antivirus: {})", mancanti.join(", "), if antivirus.is_empty() { "non letto" } else { antivirus })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REG: &str = "
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Security Center\\Provider\\Av
    DataMigrated    REG_DWORD    0x1

HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Security Center\\Provider\\Av\\{B81B202D-B515-3D48-B28C-10C266DF0ED7}
    GUID    REG_SZ    {B81B202D-B515-3D48-B28C-10C266DF0ED7}
    DISPLAYNAME    REG_SZ    Avast Free Antivirus
    STATE    REG_DWORD    0x41000
    PRODUCTEXE    REG_SZ    C:\\Program Files\\Avast\\x.exe

HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Security Center\\Provider\\Av\\{D68DDC3A-831F-4fae-9E44-DA132C1ACF46}
    GUID    REG_SZ    {D68DDC3A-831F-4fae-9E44-DA132C1ACF46}
    DISPLAYNAME    REG_SZ    Microsoft Defender Antivirus
    STATE    REG_DWORD    0x60100
";

    #[test]
    fn i_prodotti_si_leggono_dal_registro() {
        let p = leggi_prodotti(REG);
        assert_eq!(p.len(), 2, "{p:?}");
        assert_eq!(p[0], Prodotto { nome: "Avast Free Antivirus".into(), stato: Some(0x41000) });
        assert_eq!(p[1].nome, "Microsoft Defender Antivirus");
        assert_eq!(p[1].stato, Some(0x60100));
        assert!(leggi_prodotti("").is_empty());
    }

    #[test]
    fn si_sceglie_chi_e_acceso() {
        assert_eq!(scegli(&leggi_prodotti(REG)), vec!["Avast Free Antivirus".to_string()]);
        // nessuno acceso: si elencano tutti; senza stato conta come acceso
        let spenti = vec![Prodotto { nome: "A".into(), stato: Some(0x40000) }, Prodotto { nome: "B".into(), stato: Some(0x60100) }];
        assert_eq!(scegli(&spenti), vec!["A".to_string(), "B".to_string()]);
        let senza = vec![Prodotto { nome: "A".into(), stato: None }, Prodotto { nome: "B".into(), stato: Some(0x60100) }];
        assert_eq!(scegli(&senza), vec!["A".to_string()]);
    }

    #[test]
    fn i_nomi_si_uniscono_senza_doppioni_e_senza_vuoti() {
        let n = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(unisci(&n(&["Avast Free Antivirus", "  ", "avast free antivirus", "Windows  Defender"])), "Avast Free Antivirus, Windows Defender");
        assert_eq!(unisci(&n(&[])), "");
        assert_eq!(unisci(&n(&["A\nB\u{7}"])), "AB");
        assert_eq!(unisci(&[ "x".repeat(300) ]).chars().count(), MAX_NOME);
    }

    #[test]
    fn i_file_mancanti_si_trovano_in_una_cartella() {
        let dir = std::env::temp_dir().join(format!("aurora-av-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("hamlib")).expect("cartella");
        std::fs::create_dir_all(dir.join("dist")).expect("cartella");
        std::fs::create_dir_all(dir.join("cty")).expect("cartella");
        for f in ["aurora-desktop.exe", "dist/index.html", "hamlib/libhamlib-4.dll", "hamlib/rotctld.exe", "cty/cty.dat"] {
            std::fs::write(dir.join(f), b"x").expect("file");
        }
        // manca rigctld.exe (il caso tipico dell'antivirus)
        assert_eq!(file_mancanti(&dir, "aurora-desktop.exe", false), vec!["hamlib\\rigctld.exe".to_string()]);
        // un Hamlib funzionante altrove (ripristinato): non e' piu' un problema
        assert!(file_mancanti(&dir, "aurora-desktop.exe", true).is_empty());
        // il programma tolto
        std::fs::remove_file(dir.join("aurora-desktop.exe")).expect("rimozione");
        assert_eq!(file_mancanti(&dir, "aurora-desktop.exe", true), vec!["aurora-desktop.exe".to_string()]);
        // il segno scritto si rilegge
        scrivi_segno(&dir, "0.11.275", &["aurora-desktop.exe".to_string(), "hamlib\\rigctld.exe".to_string()]).expect("segno");
        let s = leggi_segno_in(&dir).expect("segno letto");
        assert_eq!(s.mancava, vec!["aurora-desktop.exe".to_string(), "hamlib\\rigctld.exe".to_string()]);
        assert_eq!(s.versione, "0.11.275");
        assert_eq!(s.data.len(), 10);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn il_segno_si_legge_solo_se_dice_cosa_mancava() {
        let s = Segno { data: "2026-11-16".into(), versione: "0.11.275".into(), mancava: vec!["hamlib\\rigctld.exe".into()], antivirus: "Avast Free Antivirus".into() };
        assert_eq!(leggi_segno(&testo_segno(&s)), Some(s.clone()));
        assert_eq!(leggi_segno("data=2026-11-16\nversione=1\nmancava=\nantivirus=x\n"), None);
        assert_eq!(leggi_segno("rumore"), None);
        let m = messaggio_segno(&s);
        assert!(m.contains("hamlib\\rigctld.exe") && m.contains("Avast Free Antivirus") && m.contains("0.11.275"), "{m}");
        assert!(messaggio_mancanti(&["cty\\cty.dat".into()], "").contains("antivirus: non letto"));
    }
}
