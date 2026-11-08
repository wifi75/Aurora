//! Messaggi di contest a 77 bit («Special operating activity» di WSJT-X 3.0.1).
//!
//! Riscrittura di `MainWindow::genStdMsgs` e `genCQMsg` (`widgets/mainwindow.cpp`, righe 9480-9640) e della parte di `processMessage`
//! che legge lo scambio ricevuto (righe 9030-9160). La sequenza di Aurora resta quella di JTDX: lo scambio di contest ricevuto si
//! **traduce** in un rapporto («-12» o «R-12», con l'SNR misurato) prima di entrare nella sequenza, cosi' gli stati di `QsoHistory`
//! restano gli stessi; in uscita i messaggi Tx1, Tx2, Tx3 e il CQ si compongono nella forma del contest.
//!
//! | Attivita'          | Tx2 (scambio)             | Tx3 (R + scambio)            | CQ        | si parte da |
//! |--------------------|---------------------------|------------------------------|-----------|-------------|
//! | NA VHF, ARRL Digi  | `DX MIA GRID4`            | `DX MIA R GRID4`             | `CQ TEST` | Tx2         |
//! | WW Digi            | `DX MIA GRID4`            | `DX MIA R GRID4`             | `CQ WW`   | Tx2         |
//! | Field Day          | `DX MIA 1D DX`            | `DX MIA R 1D DX`             | `CQ FD`   | Tx2         |
//! | RTTY Roundup (RU)  | `DX MIA 559 WI`/`559 0013`| `DX MIA R 559 0013`          | `CQ RU`   | Tx2         |
//! | EU VHF             | `<DX> <MIA> 570123 IO91NP`| `<DX> <MIA> R 570123 IO91NP` | `CQ TEST` | Tx1         |
//!
//! Nell'attivita' «standard» (FT Challenge, ARI, DARC, IARU MGM...) i messaggi sono quelli di tutti i giorni: niente traduzione.

use crate::message::{is_grid, is_std_call, is_valid_locator, same_station};
use crate::report::{format_r_report, format_report};
use crate::stdmsg::StdMessages;

/// L'attivita' di contest (la «Special operating activity» di WSJT-X).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// Messaggi normali (rapporto e locatore).
    Standard,
    NaVhf,
    ArrlDigi,
    WwDigi,
    EuVhf,
    FieldDay,
    /// Formato «RU» (RTTY Roundup): rapporto 5n9 e stato/provincia o seriale.
    Roundup,
}

impl Activity {
    /// Dal nome usato nei file di regole (`ww-digi`, `na-vhf`...). Sconosciuto = `None`.
    pub fn from_name(name: &str) -> Option<Activity> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "standard" | "" => Activity::Standard,
            "na-vhf" => Activity::NaVhf,
            "arrl-digi" => Activity::ArrlDigi,
            "ww-digi" => Activity::WwDigi,
            "eu-vhf" => Activity::EuVhf,
            "field-day" => Activity::FieldDay,
            "roundup" => Activity::Roundup,
            _ => return None,
        })
    }

    /// La parola del CQ di WSJT-X per l'attivita' (`m_cqStr`).
    pub fn default_cq_word(self) -> &'static str {
        match self {
            Activity::Standard => "",
            Activity::NaVhf | Activity::ArrlDigi | Activity::EuVhf => "TEST",
            Activity::WwDigi => "WW",
            Activity::FieldDay => "FD",
            Activity::Roundup => "RU",
        }
    }

    /// Con queste attivita' chi risponde a un CQ parte da Tx2 (WSJT-X disabilita Tx1: «start QSO with Tx2»); EU VHF parte da Tx1.
    pub fn starts_with_tx2(self) -> bool {
        matches!(self, Activity::NaVhf | Activity::ArrlDigi | Activity::WwDigi | Activity::FieldDay | Activity::Roundup)
    }

    /// I messaggi sono diversi da quelli normali.
    pub fn is_special(self) -> bool {
        self != Activity::Standard
    }
}

/// La sequenza di contest scelta: attivita', parola del CQ, scambio da inviare e seriale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContestSeq {
    pub activity: Activity,
    /// Parola del CQ (`TEST`, `WW`, `FD`, `RU`, o quella scelta dall'operatore); vuota = quella dell'attivita'.
    pub cq_word: String,
    /// Scambio fisso da inviare: Field Day `1D DX`; Roundup lo stato o `#` per il seriale. Per le altre attivita' non serve.
    pub exchange: String,
    /// Il prossimo numero progressivo da inviare (EU VHF e Roundup con seriale).
    pub serial: u32,
    /// Locatore a 6 caratteri (EU VHF); se manca si usa quello della stazione.
    pub grid6: String,
    /// Chiusura con `RRR` invece di `RR73`.
    pub rrr: bool,
}

impl ContestSeq {
    pub fn new(activity: Activity) -> ContestSeq {
        ContestSeq { activity, cq_word: String::new(), exchange: String::new(), serial: 1, grid6: String::new(), rrr: false }
    }

    /// La parola del CQ in uso.
    pub fn cq_word(&self) -> String {
        let w = self.cq_word.trim().to_ascii_uppercase();
        if w.is_empty() { self.activity.default_cq_word().to_string() } else { w }
    }
}

/// `rst` del Roundup e di EU VHF dal rapporto in dB, come WSJT-X: `nn=(n+36)/6` limitato a 2..9, `5{nn}9`.
pub fn rst_from_snr(snr: i32) -> String {
    let nn = ((snr + 36) / 6).clamp(2, 9);
    format!("5{nn}9")
}

/// Lo scambio inviato (la parte dopo i nominativi, senza «R»), per il log e per i messaggi.
pub fn sent_exchange(c: &ContestSeq, my_grid: &str, report: i32) -> String {
    let grid4: String = my_grid.chars().take(4).collect::<String>().to_ascii_uppercase();
    match c.activity {
        Activity::Standard => format_report(report),
        Activity::NaVhf | Activity::ArrlDigi | Activity::WwDigi => grid4,
        Activity::FieldDay => tidy(&c.exchange.to_ascii_uppercase()),
        Activity::Roundup => {
            let x = c.exchange.trim().to_ascii_uppercase();
            let tail = if x.is_empty() || x == "DX" || x == "#" { format!("{:04}", c.serial.min(9999)) } else { x };
            format!("{} {tail}", rst_from_snr(report))
        }
        Activity::EuVhf => {
            let g6 = if is_valid_locator(&c.grid6) && c.grid6.len() == 6 { c.grid6.to_ascii_uppercase() } else { my_grid.to_ascii_uppercase() };
            let rs: String = rst_from_snr(report).chars().take(2).collect();
            format!("{rs}{:04} {g6}", c.serial.min(9999))
        }
    }
}

fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// I messaggi Tx1-Tx5 del contest a partire da quelli standard (che restano per Tx1, Tx4 e Tx5). `None` se l'attivita' e' standard o se
/// uno dei due nominativi non e' standard: i messaggi di contest non li ammettono (solo `/P` e `/R`), e si resta ai messaggi normali.
pub fn contest_msgs(c: &ContestSeq, std: &StdMessages, my: &str, my_grid: &str, his: &str, report: i32) -> Option<StdMessages> {
    if !c.activity.is_special() || !is_std_call(my) || !is_std_call(his) {
        return None;
    }
    let my = my.to_ascii_uppercase();
    let his = his.to_ascii_uppercase();
    let sent = sent_exchange(c, my_grid, report);
    let t = if c.activity == Activity::EuVhf { format!("<{his}> <{my}> ") } else { format!("{his} {my} ") };
    let mut out = std.clone();
    out.tx2 = tidy(&format!("{t}{sent}"));
    out.tx3 = tidy(&format!("{t}R {sent}"));
    out.tx4 = format!("{his} {my} {}", if c.rrr { "RRR" } else { "RR73" });
    Some(out)
}

/// Il CQ del contest: `CQ TEST IU3ABC IO91`. `None` con un nominativo non standard (WSJT-X lo vuole standard).
pub fn contest_cq(c: &ContestSeq, my: &str, my_grid: &str) -> Option<String> {
    if !c.activity.is_special() || !is_std_call(my) {
        return None;
    }
    let grid4: String = my_grid.chars().take(4).collect::<String>().to_ascii_uppercase();
    Some(tidy(&format!("CQ {} {} {grid4}", c.cq_word(), my.to_ascii_uppercase())))
}

/// Un messaggio di contest diretto a noi, tradotto per la sequenza.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Translated {
    /// Il messaggio nella forma standard: `MIA DX -12` o `MIA DX R-12`.
    pub text: String,
    /// Chi lo manda.
    pub from: String,
    /// Lo scambio ricevuto, come scritto (`FN42`, `2A EMA`, `559 0013`, `570123 JN65AB`).
    pub exchange: String,
    /// Con la «R» davanti.
    pub roger: bool,
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn letters(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.chars().all(|c| c.is_ascii_uppercase())
}

/// Classe del Field Day: 1-32 trasmettitori e una lettera A-F (`processMessage`: `t1>="A" and t1<="F" and t0.size()<=3`).
fn fd_class(s: &str) -> bool {
    let n = s.len();
    (2..=3).contains(&n) && digits(&s[..n - 1]) && s[..n - 1].parse::<u32>().is_ok_and(|v| v >= 1) && s.ends_with(|c: char| ('A'..='F').contains(&c))
}

/// Rapporto del Roundup: da 529 a 599 (`bRTTY`).
fn ru_rst(s: &str) -> bool {
    s.len() == 3 && digits(s) && s.parse::<u32>().is_ok_and(|v| (529..=599).contains(&v))
}

/// Rapporto+seriale di EU VHF: 6 cifre da 520001 a 594000 (`bEU_VHF_w2`).
fn eu_rs_serial(s: &str) -> bool {
    s.len() == 6 && digits(s) && s.parse::<u32>().is_ok_and(|v| (520_001..=594_000).contains(&v))
}

/// Lo scambio di contest dopo i nominativi (`body` senza «R»), se ha la forma giusta per l'attivita'.
pub fn exchange_of(activity: Activity, body: &[&str]) -> Option<String> {
    match (activity, body) {
        (Activity::NaVhf | Activity::ArrlDigi | Activity::WwDigi, [g]) if is_grid(g) => Some((*g).to_string()),
        (Activity::FieldDay, [class, sect]) if fd_class(class) && letters(sect, 4) => Some(format!("{class} {sect}")),
        (Activity::Roundup, [rst, x]) if ru_rst(rst) && (letters(x, 3) || (digits(x) && x.len() <= 4)) => Some(format!("{rst} {x}")),
        (Activity::EuVhf, [rs, g6]) if eu_rs_serial(rs) && g6.len() == 6 && is_valid_locator(g6) => Some(format!("{rs} {g6}")),
        _ => None,
    }
}

/// Traduce un messaggio di contest diretto a noi (`MIA DX [R] scambio`) nella forma standard con il rapporto misurato (`snr`).
/// Tutto il resto (CQ, messaggi per altri, rapporti in dB, RR73, 73) resta com'e' (`None`).
pub fn translate(activity: Activity, text: &str, my_call: &str, snr: i32) -> Option<Translated> {
    if !activity.is_special() {
        return None;
    }
    let clean = text.replace(['<', '>'], "").to_ascii_uppercase();
    let w: Vec<&str> = clean.split_whitespace().collect();
    if w.len() < 3 || !same_station(w[0], my_call) {
        return None;
    }
    let roger = w[2] == "R" && w.len() > 3;
    let body: &[&str] = if roger { &w[3..] } else { &w[2..] };
    let exchange = exchange_of(activity, body)?;
    let rpt = if roger { format_r_report(snr) } else { format_report(snr) };
    Some(Translated { text: format!("{} {} {rpt}", w[0], w[1]), from: w[1].to_string(), exchange, roger })
}

/// Un rapporto in dB ricevuto mentre il contest chiede uno scambio diverso: chi risponde non sta facendo il contest (lo scambio non vale).
pub fn is_plain_report_reply(activity: Activity, text: &str, my_call: &str) -> bool {
    if !activity.is_special() {
        return false;
    }
    let w: Vec<String> = text.replace(['<', '>'], "").split_whitespace().map(str::to_ascii_uppercase).collect();
    w.len() == 3 && same_station(&w[0], my_call) && (crate::report::parse_report(&w[2]).is_some() || crate::report::parse_r_report(&w[2]).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdmsg::gen_std_msgs;

    fn seq(a: Activity) -> ContestSeq {
        ContestSeq::new(a)
    }

    /// WW Digi: `CQ WW`, scambio e R+scambio con il locatore a 4 caratteri.
    #[test]
    fn ww_digi_messages() {
        let c = seq(Activity::WwDigi);
        let std = gen_std_msgs("IU3ABC", "IO91wm", "K1ABC", -12, false);
        let m = contest_msgs(&c, &std, "IU3ABC", "IO91wm", "K1ABC", -12).expect("contest");
        assert_eq!(m.tx1, "K1ABC IU3ABC IO91");
        assert_eq!(m.tx2, "K1ABC IU3ABC IO91");
        assert_eq!(m.tx3, "K1ABC IU3ABC R IO91");
        assert_eq!(m.tx4, "K1ABC IU3ABC RR73");
        assert_eq!(contest_cq(&c, "IU3ABC", "IO91wm").as_deref(), Some("CQ WW IU3ABC IO91"));
    }

    /// Field Day e Roundup con il seriale a 4 cifre e il rapporto 5n9 dall'SNR.
    #[test]
    fn field_day_and_roundup() {
        let mut c = seq(Activity::FieldDay);
        c.exchange = "1d  dx".into();
        let std = gen_std_msgs("IU3ABC", "IO91", "K1ABC", -12, false);
        let m = contest_msgs(&c, &std, "IU3ABC", "IO91", "K1ABC", -12).expect("fd");
        assert_eq!(m.tx2, "K1ABC IU3ABC 1D DX");
        assert_eq!(m.tx3, "K1ABC IU3ABC R 1D DX");
        let mut r = seq(Activity::Roundup);
        r.exchange = "#".into();
        r.serial = 13;
        let m = contest_msgs(&r, &std, "IU3ABC", "IO91", "K1ABC", -12).expect("ru");
        assert_eq!(m.tx2, "K1ABC IU3ABC 549 0013");
        assert_eq!(rst_from_snr(-30), "529");
        assert_eq!(rst_from_snr(20), "599");
    }

    /// EU VHF: due nominativi tra parentesi, rapporto a 2 cifre, seriale a 4 e locatore a 6.
    #[test]
    fn eu_vhf_messages() {
        let mut c = seq(Activity::EuVhf);
        c.serial = 123;
        c.grid6 = "IO91wm".into();
        let std = gen_std_msgs("G4ABC", "IO91wm", "PA9XYZ", 6, false);
        let m = contest_msgs(&c, &std, "G4ABC", "IO91wm", "PA9XYZ", 6).expect("eu");
        assert_eq!(m.tx1, "PA9XYZ G4ABC IO91");
        assert_eq!(m.tx2, "<PA9XYZ> <G4ABC> 570123 IO91WM");
        assert_eq!(m.tx3, "<PA9XYZ> <G4ABC> R 570123 IO91WM");
        assert_eq!(m.tx4, "PA9XYZ G4ABC RR73");
    }

    /// Lo scambio ricevuto diventa un rapporto con l'SNR misurato; RR73 e i messaggi per altri restano com'erano.
    #[test]
    fn received_exchanges_are_translated() {
        let t = translate(Activity::WwDigi, "IU3ABC K1ABC FN42", "IU3ABC", -9).expect("grid");
        assert_eq!(t.text, "IU3ABC K1ABC -09");
        assert_eq!(t.exchange, "FN42");
        let t = translate(Activity::WwDigi, "IU3ABC K1ABC R FN42", "IU3ABC", 3).expect("r grid");
        assert_eq!(t.text, "IU3ABC K1ABC R+03");
        assert!(t.roger);
        assert_eq!(translate(Activity::WwDigi, "IU3ABC K1ABC RR73", "IU3ABC", 0), None);
        assert_eq!(translate(Activity::WwDigi, "W9XYZ K1ABC FN42", "IU3ABC", 0), None);
        let fd = translate(Activity::FieldDay, "IU3ABC K1ABC R 2A EMA", "IU3ABC", -1).expect("fd");
        assert_eq!(fd.exchange, "2A EMA");
        let eu = translate(Activity::EuVhf, "<IU3ABC> <PA9XYZ> 570123 JO22DB", "IU3ABC", -5).expect("eu");
        assert_eq!(eu.text, "IU3ABC PA9XYZ -05");
        assert_eq!(translate(Activity::EuVhf, "IU3ABC PA9XYZ JO22", "IU3ABC", -5), None, "in EU VHF il locatore e' il Tx1");
        let ru = translate(Activity::Roundup, "IU3ABC K1ABC 579 WI", "IU3ABC", -5).expect("ru");
        assert_eq!(ru.exchange, "579 WI");
        assert_eq!(translate(Activity::Standard, "IU3ABC K1ABC FN42", "IU3ABC", 0), None);
    }

    /// Chi risponde con il rapporto in dB a un CQ di contest non sta facendo il contest.
    #[test]
    fn plain_reports_are_flagged() {
        assert!(is_plain_report_reply(Activity::WwDigi, "IU3ABC K1ABC -12", "IU3ABC"));
        assert!(!is_plain_report_reply(Activity::WwDigi, "IU3ABC K1ABC FN42", "IU3ABC"));
        assert!(!is_plain_report_reply(Activity::Standard, "IU3ABC K1ABC -12", "IU3ABC"));
    }

    /// Con un nominativo composto i messaggi di contest non si possono comporre: restano quelli normali.
    #[test]
    fn compound_calls_fall_back() {
        let c = seq(Activity::WwDigi);
        let std = gen_std_msgs("DL/IU3ABC", "IO91", "K1ABC", -12, false);
        assert_eq!(contest_msgs(&c, &std, "DL/IU3ABC", "IO91", "K1ABC", -12), None);
        assert_eq!(contest_cq(&c, "DL/IU3ABC", "IO91"), None);
    }
}
