//! Come JTDX legge un messaggio ricevuto e un messaggio trasmesso per aggiornare la memoria della sequenza.
//!
//! - [`classify_rx`] e' la parte di `DisplayText::displayDecodedText` (`displaytext.cpp:222-432`, `REA 0x4eb170`) che decide lo stato da
//!   registrare in `QsoHistory` (RCQ, RCALL, RREPORT, RRREPORT, RRR, RRR73, R73, RFIN) e il «tipo» della riga (`std_type`);
//! - [`classify_tx`] e' `DisplayText::displayTransmittedText` (`displaytext.cpp:946-1063`, `REA 0x4f2810`): dal testo trasmesso lo stato inviato
//!   (SCQ, SCALL, SREPORT, SRREPORT, SRR, SRR73, S73).
//!
//! Non ci sono i marcatori «nominativo non standard» del decoder di JTDX (`,` e `.` in colonna 47): le righe sono sempre trattate come standard.

use crate::history::{QsoHistory, Status};
use crate::message::{Message, base_call, is_grid, parse};

/// Cio' che la finestra di JTDX ricorda tra una riga e l'altra (`mystatus_`, `mygrid_`, `myhisCall_` di `DisplayText`).
#[derive(Clone, Debug, Default)]
pub struct DisplayState {
    pub mystatus: Option<Status>,
    pub mygrid: String,
    pub myhiscall: String,
}

/// Il contesto della finestra principale: il DX attuale (nominativo di base e locatore a 4 caratteri) e la frequenza di ricezione.
pub struct RxContext<'a> {
    pub my_base: &'a str,
    pub his_base: &'a str,
    pub his_grid4: &'a str,
    pub rx_freq: i32,
    /// «Marker RR73»: i messaggi di chiusura fra altri stazioni diventano RFIN (di fabbrica acceso).
    pub rr73_marker: bool,
}

/// Esito per una riga ricevuta.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RxClass {
    /// 0 niente, 1 CQ, 2 diretto a noi o al DX, 3 fra altri, 4 chiusura fra altri.
    pub std_type: u8,
    pub status: Option<Status>,
    pub param: String,
    pub check_call: String,
    pub grid: String,
    /// Direzione del CQ («DX», «EU», ...).
    pub tyyp: String,
}

/// Le parole di un messaggio, senza `<>`, in maiuscolo.
pub fn words(text: &str) -> Vec<String> {
    text.replace(['<', '>'], "").split_whitespace().map(str::to_ascii_uppercase).collect()
}

fn message_of(text: &str) -> String {
    words(text).join(" ")
}

fn is_call_word(w: &str) -> bool {
    matches!(parse(&format!("CQ {w}")), Message::Cq { .. })
}

/// `CQersCall`: il nominativo di chi chiama (`CQ`, `DE`, `QRZ`) con griglia e direzione; vuoto se il modificatore e' piu' lungo di due caratteri.
fn cq_call(w: &[String]) -> Option<(String, String, String)> {
    let first = w.first()?.as_str();
    if !matches!(first, "CQ" | "DE" | "QRZ") {
        return None;
    }
    let mut idx = 1;
    let mut tyyp = String::new();
    if let Some(t) = w.get(1)
        && w.get(2).is_some_and(|c| is_call_word(c))
        && ((1..=4).contains(&t.len()) && t.chars().all(|c| c.is_ascii_uppercase()) || (t.len() == 3 && t.chars().all(|c| c.is_ascii_digit())))
        && !is_call_word(t)
    {
        tyyp = t.clone();
        idx = 2;
    }
    match tyyp.as_str() {
        "CQ" | "LP" | "POTA" | "SOTA" | "IOTA" => tyyp.clear(),
        "908" => tyyp = "JA".to_string(),
        "ASIA" => tyyp = "AS".to_string(),
        "USA" => tyyp = "K".to_string(),
        _ => {}
    }
    if tyyp.chars().count() > 2 {
        return None;
    }
    let call = w.get(idx).filter(|c| is_call_word(c))?.clone();
    let grid = w.get(idx + 1).filter(|g| is_grid(g)).cloned().unwrap_or_default();
    Some((call, grid, tyyp))
}

/// `DecodedText::call`: la prima parola se e' un nominativo (vuoto per `CQ`, `DE`, `QRZ`).
fn first_call(w: &[String]) -> String {
    match w.first() {
        Some(f) if is_call_word(f) && !matches!(f.as_str(), "CQ" | "DE" | "QRZ") && !f.contains("TU73") && !f.contains("73GL") => f.clone(),
        _ => String::new(),
    }
}

/// `DecodedText::deCallAndGrid`: la seconda parola (il mittente) e il locatore.
pub fn de_call_and_grid(text: &str) -> (String, String) {
    let w = words(text);
    let word1 = w.first().cloned().unwrap_or_default();
    let mut call = w.get(1).cloned().unwrap_or_default();
    let mut grid = w.get(2).cloned().unwrap_or_default();
    if grid == "R" {
        grid = w.get(3).cloned().unwrap_or_default();
    }
    if word1 == "CQ" && (3..=4).contains(&call.len()) && !call.chars().any(|c| c.is_ascii_digit()) {
        call = w.get(2).cloned().unwrap_or_default();
        grid = w.get(3).cloned().unwrap_or_default();
    }
    if !is_call_word(&call) || is_grid(&call) || call.contains("TU73") || call.contains("73GL") {
        call.clear();
    }
    if !is_grid(&grid) {
        grid.clear();
    }
    (call, grid)
}

/// `DecodedText::report(myBase, dxBase)`: `(standard, rapporto, tipo)`. Il tipo e' `R` per `R-07`, vuoto per `-07`.
pub fn report(text: &str, my_base: &str, dx_base: &str) -> (bool, Option<(String, String)>) {
    let w = words(text);
    let standard = matches!(parse(text), Message::Cq { .. } | Message::Directed { .. });
    let mut ret = standard;
    let mut out = None;
    let w0_ok = w.first().is_some_and(|w0| w0 == my_base || w0 == "DE" || w0.ends_with(&format!("/{my_base}")) || w0.starts_with(&format!("{my_base}/")));
    let w1_ok = dx_base.is_empty()
        || w.get(1).is_some_and(|w1| w1 == dx_base || w1.ends_with(&format!("/{dx_base}")) || w1.starts_with(&format!("{dx_base}/")));
    if w0_ok && w1_ok {
        let tt = w.get(2).cloned().unwrap_or_default();
        if w.len() > 2 {
            ret = true;
        }
        let num = |s: &str| s.parse::<i32>().ok().filter(|n| (-50..50).contains(n));
        if num(&tt).is_some() {
            out = Some((tt, String::new()));
        } else if let Some(rest) = tt.strip_prefix('R')
            && num(rest).is_some()
        {
            out = Some((rest.to_string(), "R".to_string()));
        }
    }
    (ret, out)
}

/// Lo stato di una riga ricevuta, come lo decide `displayDecodedText` per la finestra «Band Activity».
pub fn classify_rx(text: &str, ctx: &RxContext<'_>, ds: &mut DisplayState, hist: &QsoHistory, freq: i32) -> RxClass {
    let msg = message_of(text);
    let w = words(text);
    let mut out = RxClass::default();
    let his_base = ctx.his_base;
    let contains_his = |c: &str| !his_base.is_empty() && c.contains(his_base);
    let cq = cq_call(&w);
    let check_call2 = first_call(&w);
    let mystatus = ds.mystatus.unwrap_or(Status::None);

    if let Some((call, mut grid, tyyp)) = cq {
        out.check_call = call.clone();
        out.tyyp = tyyp;
        if grid.is_empty() {
            grid = hist.status(&call).1;
        }
        if grid.is_empty() && base_call(&call) == his_base {
            grid = ctx.his_grid4.to_string();
        }
        if msg.starts_with("DE ") {
            out.tyyp.clear();
            if (ctx.rx_freq - freq).abs() < 10 {
                out.std_type = 2;
                if !grid.is_empty() && grid != "RR73" && his_base.is_empty() {
                    out.status = Some(Status::Rcall);
                    out.param = grid.clone();
                } else if his_base.is_empty() && msg.ends_with(&call) {
                    out.status = Some(Status::Rcall);
                } else if contains_his(&call) && mystatus > Status::Scq && mystatus != Status::Fin {
                    let (_, rep) = report(text, ctx.my_base, &base_call(&call));
                    if let Some((r, ty)) = rep.filter(|(r, _)| !r.is_empty()) {
                        out.status = Some(if ty == "R" { Status::Rrreport } else { Status::Rreport });
                        out.param = r;
                    } else if msg.contains(" RRR") && mystatus > Status::Sreport {
                        out.status = Some(Status::Rrr);
                    } else if msg.contains("RR73") && mystatus > Status::Sreport {
                        out.status = Some(Status::Rrr73);
                    } else if msg.contains(" 73") && mystatus >= Status::Rrr && mystatus != Status::Fin {
                        out.status = Some(Status::R73);
                    } else {
                        out.std_type = 3;
                    }
                } else {
                    out.std_type = 3;
                }
            } else {
                out.std_type = 3;
            }
        } else {
            out.std_type = 1;
            out.status = Some(Status::Rcq);
            out.param = grid.clone();
        }
        out.grid = grid;
        return out;
    }

    if !ctx.my_base.is_empty() && base_call(&check_call2) == ctx.my_base {
        // un messaggio indirizzato a noi
        out.std_type = 2;
        let (mut check_call, mut grid) = de_call_and_grid(text);
        if !grid.is_empty() || (!check_call.is_empty() && w.len() == 2) {
            out.status = Some(Status::Rcall);
            if grid.is_empty() {
                grid = hist.status(&check_call).1;
            }
            if grid.is_empty() && base_call(&check_call) == his_base {
                grid = ctx.his_grid4.to_string();
            }
            out.param = grid.clone();
        } else {
            if grid.is_empty() {
                grid = hist.status(&check_call).1;
            }
            if grid.is_empty() && base_call(&check_call) == his_base {
                grid = ctx.his_grid4.to_string();
            }
            let (std, rep) = report(text, ctx.my_base, &base_call(&check_call));
            if std {
                if !check_call.is_empty() {
                    if let Some((r, ty)) = rep.filter(|(r, _)| !r.is_empty()) {
                        out.status = Some(if ty == "R" { Status::Rrreport } else { Status::Rreport });
                        out.param = r;
                    } else if msg.contains(" RRR") {
                        out.status = Some(Status::Rrr);
                    } else if msg.contains("RR73") {
                        out.status = Some(Status::Rrr73);
                    } else if msg.contains(" 73") {
                        out.status = Some(Status::R73);
                    } else {
                        out.status = Some(Status::Rcall);
                    }
                }
            } else if msg.contains("73") && !his_base.is_empty() && mystatus >= Status::Rrr && mystatus != Status::Fin {
                out.status = Some(Status::R73);
                check_call = his_base.to_string();
            }
            if !check_call.is_empty() {
                if his_base.is_empty() && (ds.myhiscall.is_empty() || !check_call.contains(ds.myhiscall.as_str())) {
                    let (st, g) = hist.status(&base_call(&check_call));
                    ds.mystatus = Some(st);
                    ds.mygrid = g;
                    ds.myhiscall = base_call(&check_call);
                }
                if contains_his(&check_call) || (!ds.myhiscall.is_empty() && check_call.contains(ds.myhiscall.as_str())) {
                    ds.mystatus = out.status;
                    if grid.is_empty() && !ds.mygrid.is_empty() {
                        grid = ds.mygrid.clone();
                    }
                }
            }
        }
        out.check_call = check_call;
        out.grid = grid;
        return out;
    }

    // un messaggio fra altre stazioni (o testo libero)
    let (check_call, mut grid) = de_call_and_grid(text);
    if !check_call.is_empty() {
        if grid.is_empty() {
            grid = hist.status(&check_call).1;
        }
        if grid.is_empty() && base_call(&check_call) == his_base {
            grid = ctx.his_grid4.to_string();
        }
        out.std_type = 3;
        if !grid.is_empty() {
            out.param = grid.clone();
        }
        if ctx.rr73_marker && (msg.contains("RR73") || msg.contains(" 73")) {
            out.std_type = 4;
            out.status = Some(Status::Rfin);
        }
        out.check_call = check_call;
        out.grid = grid;
    }
    out
}

/// Il testo di una trasmissione, come lo legge `displayTransmittedText`: `(nominativo, stato, parametro, direzione)`. `his_call` e' il DX
/// attuale (per i messaggi `73` senza nominativo) e `skip_tx1` la marcatura «S» dei rapporti con «Salta Tx1».
pub fn classify_tx(text: &str, my_base: &str, his_call: &str, skip_tx1: bool) -> Option<(String, Status, String, String)> {
    let parts: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    if parts.len() < 2 {
        return None;
    }
    let mut status = Status::S73;
    let mut param = String::new();
    let mut tyyp = String::new();
    let mut call = parts[0].clone();
    let skip = if skip_tx1 { "S" } else { "" };
    let is73 = |s: &str| matches!(s, "73" | "TNX" | "TKS" | "TU");
    if parts.len() > 2 {
        if parts[0] == "CQ" {
            status = Status::Scq;
            if parts.len() > 3 {
                tyyp = if parts[1] == "908" { "JA".to_string() } else { parts[1].clone() };
                call = parts[2].clone();
                param = parts[3].clone();
            } else {
                call = parts[1].clone();
                param = parts[2].clone();
            }
        } else if parts[2].len() == 4 {
            if parts[2] == "RR73" {
                status = Status::Srr73;
            } else if parts[2].starts_with("R+") || parts[2].starts_with("R-") {
                param = parts[2].chars().skip(1).take(3).collect();
                status = Status::Srreport;
                tyyp = skip.to_string();
            } else {
                status = Status::Scall;
            }
        } else if parts[2].len() == 3 {
            if parts[2] == "RRR" {
                status = Status::Srr;
            } else if parts[2].starts_with('+') || parts[2].starts_with('-') {
                param = parts[2].chars().take(3).collect();
                status = Status::Sreport;
                tyyp = skip.to_string();
            }
        } else if is73(&parts[2]) || is73(&parts[1]) || is73(&parts[0]) || (parts.len() == 4 && is73(&parts[3])) {
            call = his_call.to_string();
            status = Status::S73;
        }
    }
    if parts.len() == 2 {
        if parts[0] == "CQ" {
            status = Status::Scq;
            call = parts[1].clone();
        } else if !my_base.is_empty() && base_call(&parts[1]) == my_base && !is73(&call) {
            status = Status::Scall;
        } else if is73(&call) || is73(&parts[1]) {
            call = his_call.to_string();
            status = Status::S73;
        }
    }
    Some((call, status, param, tyyp))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(his: &'a str, rx: i32) -> RxContext<'a> {
        RxContext { my_base: "IU3ABC", his_base: his, his_grid4: "", rx_freq: rx, rr73_marker: true }
    }

    fn rx(text: &str, his: &str, mystatus: Status) -> RxClass {
        let mut ds = DisplayState { mystatus: Some(mystatus), ..DisplayState::default() };
        classify_rx(text, &ctx(his, 1200), &mut ds, &QsoHistory::new(), 1200)
    }

    #[test]
    fn a_cq_is_rcq_with_grid_and_direction() {
        let c = rx("CQ DX K1ABC FN42", "", Status::None);
        assert_eq!((c.std_type, c.status, c.check_call.as_str(), c.grid.as_str(), c.tyyp.as_str()), (1, Some(Status::Rcq), "K1ABC", "FN42", "DX"));
        let c = rx("CQ POTA K1ABC FN42", "", Status::None);
        assert_eq!(c.tyyp, "", "POTA vale CQ semplice");
        let c = rx("CQ TEST K1ABC FN42", "", Status::None);
        assert_eq!((c.std_type, c.status), (3, None), "un modificatore piu' lungo di due caratteri annulla il CQ: resta un messaggio fra altri");
        let c = rx("CQ 908 JA1AA PM95", "", Status::None);
        assert_eq!(c.tyyp, "JA");
    }

    #[test]
    fn messages_to_us_map_to_the_states_of_qsohistory() {
        let cases = [
            ("IU3ABC K1ABC FN42", Status::Rcall, "FN42"),
            ("IU3ABC K1ABC", Status::Rcall, ""),
            ("IU3ABC K1ABC -07", Status::Rreport, "-07"),
            ("IU3ABC K1ABC R-07", Status::Rrreport, "-07"),
            ("IU3ABC K1ABC RRR", Status::Rrr, ""),
            ("IU3ABC K1ABC RR73", Status::Rrr73, ""),
            ("IU3ABC K1ABC 73", Status::R73, ""),
        ];
        for (text, status, param) in cases {
            let c = rx(text, "K1ABC", Status::Sreport);
            assert_eq!((c.std_type, c.status, c.param.as_str(), c.check_call.as_str()), (2, Some(status), param, "K1ABC"), "{text}");
        }
    }

    #[test]
    fn a_closing_between_others_is_rfin_with_the_marker_and_plain_otherwise() {
        let c = rx("W1AW K1ABC RR73", "", Status::None);
        assert_eq!((c.std_type, c.status, c.check_call.as_str()), (4, Some(Status::Rfin), "K1ABC"));
        let c = rx("W1AW K1ABC FN42", "", Status::None);
        assert_eq!((c.std_type, c.status, c.param.as_str()), (3, None, "FN42"));
        let mut ds = DisplayState::default();
        let mut cx = ctx("", 1200);
        cx.rr73_marker = false;
        let c = classify_rx("W1AW K1ABC RR73", &cx, &mut ds, &QsoHistory::new(), 1200);
        assert_eq!((c.std_type, c.status), (3, None));
    }

    #[test]
    fn de_messages_count_only_near_the_rx_frequency() {
        let mut ds = DisplayState::default();
        let c = classify_rx("DE K1ABC FN42", &ctx("", 1200), &mut ds, &QsoHistory::new(), 1200);
        assert_eq!((c.std_type, c.status), (2, Some(Status::Rcall)));
        let c = classify_rx("DE K1ABC FN42", &ctx("", 1200), &mut ds, &QsoHistory::new(), 1500);
        assert_eq!((c.std_type, c.status), (3, None), "fuori da 10 Hz dalla frequenza di ricezione");
    }

    #[test]
    fn transmitted_texts_map_to_the_sent_states() {
        let cases = [
            ("CQ IU3ABC JN55", Status::Scq),
            ("CQ DX IU3ABC JN55", Status::Scq),
            ("K1ABC IU3ABC JN55", Status::Scall),
            ("K1ABC IU3ABC -07", Status::Sreport),
            ("K1ABC IU3ABC R-07", Status::Srreport),
            ("K1ABC IU3ABC RRR", Status::Srr),
            ("K1ABC IU3ABC RR73", Status::Srr73),
            ("K1ABC IU3ABC 73", Status::S73),
            ("K1ABC IU3ABC", Status::Scall),
            ("TU 73", Status::S73),
        ];
        for (text, want) in cases {
            let (call, status, ..) = classify_tx(text, "IU3ABC", "K1ABC", false).expect("classificato");
            assert_eq!(status, want, "{text}");
            if want == Status::S73 && text == "TU 73" {
                assert_eq!(call, "K1ABC", "un 73 senza nominativo va al DX attuale");
            }
        }
        let (call, status, param, tyyp) = classify_tx("CQ DX IU3ABC JN55", "IU3ABC", "", false).expect("cq");
        assert_eq!((call.as_str(), status, param.as_str(), tyyp.as_str()), ("IU3ABC", Status::Scq, "JN55", "DX"));
        let (_, _, param, tyyp) = classify_tx("K1ABC IU3ABC -07", "IU3ABC", "K1ABC", true).expect("rapporto");
        assert_eq!((param.as_str(), tyyp.as_str()), ("-07", "S"), "con Salta Tx1 il rapporto e' marcato");
    }

    #[test]
    fn report_extraction_follows_decodedtext() {
        assert_eq!(report("IU3ABC K1ABC -07", "IU3ABC", "K1ABC").1, Some(("-07".to_string(), String::new())));
        assert_eq!(report("IU3ABC K1ABC R-07", "IU3ABC", "K1ABC").1, Some(("-07".to_string(), "R".to_string())));
        assert_eq!(report("IU3ABC K1ABC FN42", "IU3ABC", "K1ABC").1, None);
        assert_eq!(report("IU3ABC W1AW -07", "IU3ABC", "K1ABC").1, None, "il mittente non e' il DX");
    }
}
