//! I file da mandare allo sponsor: Cabrillo 3.0, EDI (REG1TEST, VHF IARU Regione 1) e ADIF con `CONTEST_ID`, piu' il controllo prima
//! dell'invio. Formati in `docs/piano-contest.md`, sezione 9.

use crate::calendario::{data, hhmm};
use crate::geo;
use crate::punteggio::{Stato, Totali, Valutato};
use crate::regole::Regole;

/// I dati dell'intestazione: stazione e scelte della scheda «Impostazioni del contest».
#[derive(Clone, Debug, Default)]
pub struct Intestazione {
    pub call: String,
    pub grid: String,
    pub operatore: String,
    pub potenza: String,
    pub trasmettitori: String,
    pub bande: String,
    pub assistenza: String,
    pub operatori: String,
    pub club: String,
    pub nome: String,
    pub location: String,
    pub soapbox: String,
    /// Antenna (EDI: `SAnte`).
    pub antenna: String,
    /// `CATEGORY-STATION` (`FIXED`, `PORTABLE`, `MOBILE`, `ROVER`).
    pub stazione: String,
    /// `ADDRESS` (piu' righe) ed `EMAIL`.
    pub indirizzo: String,
    pub email: String,
    /// Nome del contest nel file (vuoto = quello delle regole).
    pub cabrillo_contest: String,
    pub versione: String,
}

/// Il valore di una colonna dello scambio inviato.
fn colonna_inviata(c: &str, v: &Valutato, h: &Intestazione) -> String {
    let x: Vec<&str> = v.qso.exch_sent.split_whitespace().collect();
    match c {
        "grid4" => h.grid.chars().take(4).collect::<String>().to_ascii_uppercase(),
        "grid6" => h.grid.to_ascii_uppercase(),
        "snr" => non_vuoto(&v.qso.rst_sent, "-10"),
        "rst" => x.first().map_or("59".into(), |s| s.chars().take(if s.len() == 6 { 2 } else { 3 }).collect()),
        "seriale" => x.first().map_or("001".into(), |s| if s.len() == 6 { s[2..].to_string() } else { x.get(1).copied().unwrap_or("001").to_string() }),
        "classe" => x.first().copied().unwrap_or("1D").to_string(),
        "sezione" | "stato" | "provincia" => x.last().copied().unwrap_or("DX").to_string(),
        _ => non_vuoto(&v.qso.exch_sent, "-"),
    }
}

/// Il valore di una colonna dello scambio ricevuto.
fn colonna_ricevuta(c: &str, v: &Valutato) -> String {
    let x: Vec<&str> = v.qso.exch_rcvd.split_whitespace().collect();
    let grid = crate::punteggio::grid_ricevuto(&v.qso);
    match c {
        "grid4" => non_vuoto(&grid.chars().take(4).collect::<String>(), "ZZ00"),
        "grid6" => non_vuoto(&grid, "ZZ00"),
        "snr" => non_vuoto(&v.qso.rst_rcvd, "-10"),
        "rst" => x.first().map_or("59".into(), |s| s.chars().take(if s.len() == 6 { 2 } else { 3 }).collect()),
        "seriale" => x.first().map_or("000".into(), |s| if s.len() == 6 { s[2..].to_string() } else { x.get(1).copied().unwrap_or("000").to_string() }),
        "classe" => x.first().copied().unwrap_or("-").to_string(),
        "sezione" | "stato" | "provincia" => x.last().copied().unwrap_or("-").to_string(),
        _ => non_vuoto(&v.qso.exch_rcvd, "-"),
    }
}

fn non_vuoto(s: &str, altrimenti: &str) -> String {
    let t = s.trim();
    if t.is_empty() { altrimenti.to_string() } else { t.to_ascii_uppercase() }
}

/// Il file Cabrillo 3.0.
pub fn cabrillo(r: &Regole, h: &Intestazione, log: &[Valutato], tot: &Totali) -> String {
    let mut out = String::new();
    let mut riga = |k: &str, v: &str| {
        if !v.trim().is_empty() {
            out.push_str(&format!("{k}: {}\r\n", v.trim()));
        }
    };
    riga("START-OF-LOG", "3.0");
    let nome = if h.cabrillo_contest.trim().is_empty() { r.export.cabrillo_contest.clone() } else { h.cabrillo_contest.clone() };
    riga("CONTEST", &nome);
    riga("CALLSIGN", &h.call.to_ascii_uppercase());
    riga("CATEGORY-OPERATOR", &h.operatore);
    riga("CATEGORY-ASSISTED", &h.assistenza);
    riga("CATEGORY-BAND", if h.bande.is_empty() { "ALL" } else { &h.bande });
    riga("CATEGORY-MODE", "DIGI");
    riga("CATEGORY-POWER", &h.potenza);
    riga("CATEGORY-STATION", if h.stazione.is_empty() { "FIXED" } else { &h.stazione });
    riga("CATEGORY-TRANSMITTER", &h.trasmettitori);
    riga("GRID-LOCATOR", &h.grid.to_ascii_uppercase());
    riga("LOCATION", &h.location);
    riga("CLAIMED-SCORE", &tot.punteggio.to_string());
    riga("CLUB", &h.club);
    riga("OPERATORS", &h.operatori);
    riga("NAME", &h.nome);
    for a in h.indirizzo.lines().take(6) {
        riga("ADDRESS", a);
    }
    riga("EMAIL", &h.email);
    riga("CREATED-BY", &format!("Aurora {}", h.versione));
    for s in h.soapbox.lines() {
        riga("SOAPBOX", s);
    }
    let modo = if r.export.cabrillo_modo.is_empty() { "DG" } else { r.export.cabrillo_modo.as_str() };
    for v in log.iter().filter(|v| v.qso.stato != Stato::Errore) {
        let freq = if r.export.frequenza == "banda" {
            geo::banda_cabrillo(&v.qso.band)
        } else {
            let khz = v.qso.freq_hz / 1000;
            if khz > 0 { khz.to_string() } else { geo::banda_cabrillo(&v.qso.band) }
        };
        let inviato: Vec<String> = r.export.colonne_inviate.iter().map(|c| colonna_inviata(c, v, h)).collect();
        let ricevuto: Vec<String> = r.export.colonne_ricevute.iter().map(|c| colonna_ricevuta(c, v)).collect();
        let tx = if h.trasmettitori == "TWO" { " 0" } else { "" };
        out.push_str(&format!(
            "QSO: {freq:>5} {modo} {} {} {:<13} {} {:<13} {}{tx}\r\n",
            data(v.qso.time_ms),
            hhmm(v.qso.time_ms),
            h.call.to_ascii_uppercase(),
            inviato.join(" "),
            v.qso.call.to_ascii_uppercase(),
            ricevuto.join(" ")
        ));
    }
    out.push_str("END-OF-LOG:\r\n");
    out
}

/// La sezione dell'EDI (`PSect`): `SINGLE` o `MULTI` come chiede il formato REG1TEST (IARU Regione 1, VHF Managers Handbook).
fn psect(h: &Intestazione) -> String {
    if h.operatore.eq_ignore_ascii_case("MULTI-OP") { "MULTI".into() } else { "SINGLE".into() }
}

/// Il numero progressivo inviato (`RRNNNN` nello scambio, come in [`edi`]).
fn seriale_inviato(v: &Valutato) -> Option<u32> {
    let x = v.qso.exch_sent.split_whitespace().next()?;
    if x.len() == 6 { x[2..].parse().ok() } else { None }
}

/// I numeri progressivi inviati che mancano nella sequenza di tutto il log (da 1 al piu' alto), ognuno con il QSO prima del quale
/// cade (quello col primo numero piu' alto): numeri dati in aria a un QSO che poi non e' stato registrato.
pub fn seriali_persi(log: &[Valutato]) -> Vec<(u32, &Valutato)> {
    let mut con: Vec<(u32, &Valutato)> = log.iter().filter_map(|v| seriale_inviato(v).map(|n| (n, v))).collect();
    con.sort_by_key(|(n, v)| (*n, v.qso.time_ms));
    let presenti: std::collections::BTreeSet<u32> = con.iter().map(|(n, _)| *n).collect();
    let mut out = Vec::new();
    let Some(&massimo) = presenti.iter().next_back() else { return out };
    for n in 1..massimo {
        if !presenti.contains(&n)
            && let Some((_, v)) = con.iter().find(|(m, _)| *m > n)
        {
            out.push((n, *v));
        }
    }
    out
}

/// Il file EDI (REG1TEST) di una banda. I QSO segnati come errore restano come righe `ERROR` per tenere la sequenza dei numeri, e cosi'
/// i numeri progressivi persi (dati in aria a un QSO non registrato): una riga `ERROR` col numero, nella banda del QSO che lo segue.
pub fn edi(r: &Regole, h: &Intestazione, log: &[Valutato], banda: &str) -> String {
    let qsos: Vec<&Valutato> = log.iter().filter(|v| v.qso.band.eq_ignore_ascii_case(banda)).collect();
    let validi: Vec<&&Valutato> = qsos.iter().filter(|v| !v.doppio && v.qso.stato != Stato::Errore).collect();
    let punti: u64 = validi.iter().map(|v| u64::from(v.punti)).sum();
    let quadrati: std::collections::BTreeSet<String> = validi.iter().filter_map(|v| geo::quadrato(&crate::punteggio::grid_ricevuto(&v.qso))).collect();
    let migliore = validi.iter().max_by(|a, b| a.km.total_cmp(&b.km));
    let (prima, ultima) = (qsos.iter().map(|v| v.qso.time_ms).min().unwrap_or(0), qsos.iter().map(|v| v.qso.time_ms).max().unwrap_or(0));
    let d8 = |ms: i64| data(ms).replace('-', "");
    let tname = if r.export.edi_tname.is_empty() { r.contest.nome.clone() } else { r.export.edi_tname.clone() };
    let modo = r.export.edi_modo.unwrap_or(7);
    let mut out = String::new();
    out.push_str("[REG1TEST;1]\r\n");
    let campi = [
        ("TName", tname),
        ("TDate", format!("{};{}", d8(prima), d8(ultima))),
        ("PCall", h.call.to_ascii_uppercase()),
        ("PWWLo", h.grid.to_ascii_uppercase()),
        ("PExch", String::new()),
        ("PSect", psect(h)),
        ("PBand", geo::banda_edi(banda)),
        ("PClub", h.club.clone()),
        ("RName", h.nome.clone()),
        ("RCall", h.call.to_ascii_uppercase()),
        ("RAdr1", h.indirizzo.lines().next().unwrap_or_default().to_string()),
        ("RAdr2", h.indirizzo.lines().nth(1).unwrap_or_default().to_string()),
        ("RHBBS", h.email.clone()),
        ("MOpe1", h.operatori.clone()),
        ("SPowe", h.potenza.clone()),
        ("SAnte", h.antenna.trim().to_string()),
        ("CQSOs", format!("{};1", validi.len())),
        ("CQSOP", punti.to_string()),
        ("CWWLs", format!("{};0;1", quadrati.len())),
        ("CToSc", (punti * quadrati.len().max(1) as u64).to_string()),
        ("CODXC", migliore.map(|v| format!("{};{};{}", v.qso.call, crate::punteggio::grid_ricevuto(&v.qso), v.km.round() as i64)).unwrap_or_default()),
    ];
    for (k, v) in campi {
        out.push_str(&format!("{k}={v}\r\n"));
    }
    out.push_str("[Remarks]\r\n");
    for s in h.soapbox.lines() {
        out.push_str(&format!("{s}\r\n"));
    }
    let persi = seriali_persi(log);
    let mut righe: Vec<String> = Vec::with_capacity(qsos.len());
    let mut visti_q: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for v in qsos {
        let x: Vec<&str> = v.qso.exch_sent.split_whitespace().collect();
        let (rs, nr) = match x.first() {
            Some(s) if s.len() == 6 => (s[..2].to_string(), s[2..].to_string()),
            _ => (non_vuoto(&v.qso.rst_sent, ""), String::new()),
        };
        let y: Vec<&str> = v.qso.exch_rcvd.split_whitespace().collect();
        let (rr, nrr) = match y.first() {
            Some(s) if s.len() == 6 => (s[..2].to_string(), s[2..].to_string()),
            _ => (non_vuoto(&v.qso.rst_rcvd, ""), String::new()),
        };
        let grid = crate::punteggio::grid_ricevuto(&v.qso);
        for (n, _) in persi.iter().filter(|(_, dopo)| std::ptr::eq(*dopo, v)) {
            righe.push(format!("{};{};ERROR;{modo};;{n:03};;;;;0;;;;\r\n", &d8(v.qso.time_ms)[2..], hhmm(v.qso.time_ms)));
        }
        if v.qso.stato == Stato::Errore {
            righe.push(format!("{};{};ERROR;{modo};{rs};{nr};;;;;0;;;;\r\n", &d8(v.qso.time_ms)[2..], hhmm(v.qso.time_ms)));
            continue;
        }
        let nuovo_q = if !v.doppio && geo::quadrato(&grid).is_some_and(|q| visti_q.insert(q)) { "N" } else { "" };
        righe.push(format!(
            "{};{};{};{modo};{rs};{nr};{rr};{nrr};;{grid};{};;{nuovo_q};;{}\r\n",
            &d8(v.qso.time_ms)[2..],
            hhmm(v.qso.time_ms),
            v.qso.call.to_ascii_uppercase(),
            v.punti,
            if v.doppio { "D" } else { "" }
        ));
    }
    out.push_str(&format!("[QSORecords;{}]\r\n", righe.len()));
    for riga in righe {
        out.push_str(&riga);
    }
    out
}

/// L'ADIF del contest (ARI FT8 Activity accetta il log intero): i record del log di Aurora con `CONTEST_ID`.
pub fn adif(r: &Regole, h: &Intestazione, log: &[Valutato]) -> String {
    let station = logbook::adif::Station { call: h.call.clone(), grid: h.grid.clone(), tx_power: String::new() };
    let id = if r.export.adif_contest_id.is_empty() { r.contest.id.to_ascii_uppercase() } else { r.export.adif_contest_id.clone() };
    let mut out = format!("Aurora {} - log del contest {}\r\n<adif_ver:5>3.1.4<programid:6>Aurora<eoh>\r\n", h.versione, r.contest.nome);
    for v in log.iter().filter(|v| v.qso.stato != Stato::Errore && !v.doppio) {
        let q = aurora_core::QsoRecord {
            call: v.qso.call.clone(),
            grid: crate::punteggio::grid_ricevuto(&v.qso).chars().take(4).collect(),
            band: v.qso.band.clone(),
            mode: v.qso.mode.clone(),
            freq_hz: v.qso.freq_hz,
            rst_sent: v.qso.rst_sent.clone(),
            rst_rcvd: v.qso.rst_rcvd.clone(),
            time_ms: (v.qso.time_ms.rem_euclid(86_400_000)) as u32,
            time_on_unix_ms: v.qso.time_ms.max(0) as u64,
            time_off_unix_ms: v.qso.time_ms.max(0) as u64,
            exch_sent: v.qso.exch_sent.clone(),
            exch_rcvd: v.qso.exch_rcvd.clone(),
            ..Default::default()
        };
        let rec = logbook::adif::jtdx_file_record_with(&q, &station, &data(v.qso.time_ms).replace('-', ""), false);
        let rec = rec.trim_end().trim_end_matches("<eor>").trim_end_matches("<EOR>").to_string();
        out.push_str(&format!("{rec} <contest_id:{}>{id} <eor>\r\n", id.len()));
    }
    out
}

/// Controllo del log prima dell'invio: l'elenco dei problemi (vuoto = a posto).
pub fn controlla(r: &Regole, h: &Intestazione, log: &[Valutato]) -> Vec<String> {
    let mut e = Vec::new();
    if h.call.trim().is_empty() {
        e.push("manca il nominativo della stazione".into());
    }
    if !geo::grid_valido(h.grid.trim()) {
        e.push("manca il locatore della stazione (GRID-LOCATOR / PWWLo)".into());
    }
    if r.export.formato == "cabrillo" && r.export.cabrillo_contest.is_empty() && h.cabrillo_contest.trim().is_empty() {
        e.push("manca il nome del contest per il Cabrillo (CONTEST)".into());
    }
    if r.export.formato == "edi" && h.grid.trim().len() < 6 {
        e.push("l'EDI vuole il locatore a 6 caratteri (PWWLo)".into());
    }
    if r.export.formato == "edi" {
        // PSect: SINGLE o MULTI (sezioni del REG1TEST); SAnte: l'antenna, chiesta dai regolamenti VHF IARU
        if h.operatore.trim().is_empty() {
            e.push("manca la sezione dell'EDI (PSect): scegli la categoria singolo o multi operatore".into());
        } else if !["SINGLE-OP", "MULTI-OP", "ROVER"].contains(&h.operatore.as_str()) {
            e.push(format!("sezione dell'EDI (PSect) non valida: {} (servono SINGLE o MULTI)", h.operatore));
        }
        let ante = h.antenna.trim();
        if ante.is_empty() {
            e.push("manca l'antenna (SAnte dell'EDI)".into());
        } else if ante.contains([';', '[', ']']) || ante.len() > 80 {
            e.push("antenna (SAnte) non valida: niente punto e virgola o parentesi quadre, al massimo 80 caratteri".into());
        }
        let persi = seriali_persi(log).len();
        if persi > 0 {
            e.push(format!("{persi} numeri progressivi inviati senza QSO nel log: nell'EDI diventano righe ERROR"));
        }
    }
    let fuori_lista: Vec<&Valutato> = log.iter().filter(|v| v.qso.stato != Stato::Errore && !v.scambio_nota.is_empty() && !v.scambio_nota.starts_with("scambio incompleto")).collect();
    if let Some(v) = fuori_lista.first() {
        e.push(format!("{} QSO con lo scambio fuori dalle liste ufficiali (non valgono come moltiplicatore), per esempio {}: {}", fuori_lista.len(), v.qso.call, v.scambio_nota));
    }
    if h.operatore == "MULTI-OP" && h.operatori.trim().is_empty() {
        e.push("categoria multi-operatore: manca l'elenco degli operatori (OPERATORS)".into());
    }
    let senza = log.iter().filter(|v| v.qso.stato != Stato::Errore && v.scambio_nota.starts_with("scambio incompleto")).count();
    if senza > 0 {
        e.push(format!("{senza} QSO senza lo scambio richiesto (rischio di QSO non validi)"));
    }
    let incerti = log.iter().filter(|v| v.qso.stato == Stato::Incerto).count();
    if incerti > 0 {
        e.push(format!("{incerti} QSO incerti (manca la conferma finale): ricontrollali"));
    }
    if log.is_empty() {
        e.push("il log del contest e' vuoto".into());
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::punteggio::{ContestQso, Contesto, valuta_log};
    use crate::regole::carica;

    /// Una riga QSO del WW Digi come nell'esempio delle regole.
    #[test]
    fn cabrillo_ww_digi() {
        let r = carica(None).0.into_iter().find(|r| r.contest.id == "ww-digi").expect("regole");
        let q = ContestQso { n: 1, time_ms: crate::calendario::parse_iso("2026-08-29T12:15Z").expect("ora"), call: "K1ABC".into(), band: "20m".into(), mode: "FT8".into(), freq_hz: 14_074_000, grid: "FN42".into(), exch_rcvd: "FN42".into(), exch_sent: "IO91".into(), rst_rcvd: "-10".into(), ..Default::default() };
        let (v, t) = valuta_log(&r, &Contesto { mio_grid: "IO91".into(), ..Default::default() }, &[q]);
        let h = Intestazione { call: "IU3ABC".into(), grid: "IO91".into(), operatore: "SINGLE-OP".into(), potenza: "LOW".into(), trasmettitori: "ONE".into(), ..Default::default() };
        let c = cabrillo(&r, &h, &v, &t);
        assert!(c.starts_with("START-OF-LOG: 3.0\r\nCONTEST: WW-DIGI\r\n"));
        assert!(c.contains("QSO: 14074 DG 2026-08-29 1215 IU3ABC        IO91 K1ABC         FN42\r\n"), "{c}");
        assert!(c.ends_with("END-OF-LOG:\r\n"));
    }
}
