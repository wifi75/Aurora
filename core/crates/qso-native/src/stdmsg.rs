//! I messaggi standard: Tx1-Tx5 verso il DX e il CQ (Tx6).
//!
//! Riscrittura di `MainWindow::genStdMsgs` e `genCQMsg` di JTDX per i modi FT8/FT4, comprese le
//! forme con nominativo composto o non standard (`<DX> MIA`, `DX <MIA>`), dove JTDX usa il
//! nominativo nascosto tra parentesi angolari.

use crate::message::{base_call, is_std_call};
use crate::report::{format_r_report, format_report};

/// Tx1..Tx5 per un DX e un rapporto dati.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdMessages {
    /// `DX MIA GRIGLIA`
    pub tx1: String,
    /// `DX MIA -07`
    pub tx2: String,
    /// `DX MIA R-07`
    pub tx3: String,
    /// `DX MIA RR73` (o `RRR`)
    pub tx4: String,
    /// `DX MIA 73`
    pub tx5: String,
}

impl StdMessages {
    /// Il testo del messaggio `n` (1..=5).
    pub fn get(&self, n: u8) -> Option<&str> {
        match n {
            1 => Some(&self.tx1),
            2 => Some(&self.tx2),
            3 => Some(&self.tx3),
            4 => Some(&self.tx4),
            5 => Some(&self.tx5),
            _ => None,
        }
    }
}

fn grid4(grid: &str) -> String {
    grid.chars()
        .take(4)
        .collect::<String>()
        .to_ascii_uppercase()
}

fn tidy(s: String) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `genStdMsgs`: `my` e `his` sono i nominativi interi, `report` e' l'SNR da mandare al DX.
pub fn gen_std_msgs(my: &str, my_grid: &str, his: &str, report: i32, use_rrr: bool) -> StdMessages {
    let my = my.to_ascii_uppercase();
    let his = his.to_ascii_uppercase();
    let grid = grid4(my_grid);
    let my_std = is_std_call(&my);
    let his_std = is_std_call(&his);
    let my_base = base_call(&my);
    let his_base = base_call(&his);
    let both_p = my.ends_with("/P") && his.ends_with("/P");
    let both_r = my.ends_with("/R") && his.ends_with("/R");
    let mixed_pr =
        (my.ends_with("/R") && his.ends_with("/P")) || (my.ends_with("/P") && his.ends_with("/R"));
    let both_compound = my.contains('/') && his.contains('/');
    let hashed_pair = both_compound && !both_p && !both_r;

    // t0: prefisso dei messaggi; con nominativi non standard si usano quelli di base
    let t0 = if his_std && my_std {
        format!("{his} {my} ")
    } else {
        format!("{his_base} {my_base} ")
    };
    let t0a = format!("<{his}> {my} ");
    let t0b = format!("{his} <{my}> ");

    // Tx1
    let mut tx1 = if my_std {
        format!("{t0}{grid}")
    } else {
        t0a.clone()
    };
    let mut t = t0.clone();
    if mixed_pr {
        tx1 = format!("<{his}> {my}");
    } else {
        if !my_std {
            t = t0b.clone();
            tx1 = t0a.clone();
        }
        if !his_std {
            t = t0a.clone();
            tx1 = if my_std && !my.ends_with("/P") {
                format!("{t0a}{grid}")
            } else {
                t0a.clone()
            };
        }
    }
    if hashed_pair {
        t = format!("<{his}> {my_base} ");
    }

    let sent = format_report(report);
    let tx2 = format!("{t}{sent}");
    let tx3 = format!("{t}{}", format_r_report(report));

    let closing = |word: &str| -> String {
        if (!his_std && my_std) || hashed_pair {
            format!("{his} <{my}> {word}")
        } else if his_std && !my_std {
            format!("<{his}> {my} {word}")
        } else {
            format!("{t0}{word}")
        }
    };
    let tx4 = closing(if use_rrr { "RRR" } else { "RR73" });
    let tx5 = closing("73");

    StdMessages {
        tx1: tidy(tx1),
        tx2: tidy(tx2),
        tx3: tidy(tx3),
        tx4: tidy(tx4),
        tx5: tidy(tx5),
    }
}

/// `genCQMsg` (Tx6): `CQ [mod] MIA GRIGLIA`; con un nominativo non standard resta `CQ MIA`.
pub fn gen_cq(my: &str, my_grid: &str, modifier: Option<&str>) -> String {
    let my = my.to_ascii_uppercase();
    if !is_std_call(&my) {
        return format!("CQ {my}");
    }
    let grid = grid4(my_grid);
    match modifier {
        Some(m) if !m.is_empty() => tidy(format!("CQ {} {my} {grid}", m.to_ascii_uppercase())),
        _ => tidy(format!("CQ {my} {grid}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `genStdMsgs`, ramo FT con due nominativi standard.
    #[test]
    fn standard_pair() {
        let m = gen_std_msgs("IU3ABC", "JN55rk", "LZ2KSQ", -7, false);
        assert_eq!(m.tx1, "LZ2KSQ IU3ABC JN55");
        assert_eq!(m.tx2, "LZ2KSQ IU3ABC -07");
        assert_eq!(m.tx3, "LZ2KSQ IU3ABC R-07");
        assert_eq!(m.tx4, "LZ2KSQ IU3ABC RR73");
        assert_eq!(m.tx5, "LZ2KSQ IU3ABC 73");
    }

    /// `rrr1CheckBox`: `t0 + (m_rrr ? "RRR" : "RR73")`.
    #[test]
    fn rrr_option_changes_tx4_only() {
        let m = gen_std_msgs("IU3ABC", "JN55", "LZ2KSQ", 3, true);
        assert_eq!(m.tx4, "LZ2KSQ IU3ABC RRR");
        assert_eq!(m.tx5, "LZ2KSQ IU3ABC 73");
        assert_eq!(m.tx2, "LZ2KSQ IU3ABC +03");
    }

    /// `/P` e `/R` sono nominativi standard: i messaggi li portano interi.
    #[test]
    fn portable_suffix_is_kept_whole() {
        let m = gen_std_msgs("IU3ABC", "JN55", "AA1AA/P", -12, false);
        assert_eq!(m.tx1, "AA1AA/P IU3ABC JN55");
        assert_eq!(m.tx3, "AA1AA/P IU3ABC R-12");
        assert_eq!(m.tx4, "AA1AA/P IU3ABC RR73");
    }

    /// `bothCallsP`: due /P non attivano la forma con nominativo nascosto.
    #[test]
    fn both_portable_stays_plain() {
        let m = gen_std_msgs("IU3ABC/P", "JN55", "AA1AA/P", -1, false);
        assert_eq!(m.tx2, "AA1AA/P IU3ABC/P -01");
        assert_eq!(m.tx5, "AA1AA/P IU3ABC/P 73");
    }

    /// `mixedPR` e `bothCallsCompound`: /R con /P, Tx1 senza griglia e con il DX tra parentesi;
    /// poiche' entrambi hanno una barra, rapporti e chiusure usano il nominativo di base di chi scrive.
    #[test]
    fn mixed_r_and_p() {
        let m = gen_std_msgs("IU3ABC/R", "JN55", "AA1AA/P", -1, false);
        assert_eq!(m.tx1, "<AA1AA/P> IU3ABC/R");
        assert_eq!(m.tx2, "<AA1AA/P> IU3ABC -01");
        assert_eq!(m.tx4, "AA1AA/P <IU3ABC/R> RR73");
    }

    /// `!m_bHisCallStd`: il DX con prefisso (non standard) va tra `<>`; i rapporti usano il suo nominativo di base.
    #[test]
    fn nonstandard_dx() {
        let m = gen_std_msgs("IU3ABC", "JN55", "DL/AA1AA", -7, false);
        assert_eq!(m.tx1, "<DL/AA1AA> IU3ABC JN55");
        assert_eq!(m.tx2, "<DL/AA1AA> IU3ABC -07");
        assert_eq!(m.tx4, "DL/AA1AA <IU3ABC> RR73");
        assert_eq!(m.tx5, "DL/AA1AA <IU3ABC> 73");
    }

    /// `!m_bMyCallStd`: il proprio nominativo non standard: Tx1 senza griglia.
    #[test]
    fn nonstandard_me() {
        let m = gen_std_msgs("IU3ABC/QRP", "JN55", "AA1AA", -7, false);
        assert_eq!(m.tx1, "<AA1AA> IU3ABC/QRP");
        assert_eq!(m.tx2, "AA1AA <IU3ABC/QRP> -07");
        assert_eq!(m.tx4, "<AA1AA> IU3ABC/QRP RR73");
    }

    /// `bothCallsCompound`: due nominativi composti diversi da /P-/P e /R-/R.
    #[test]
    fn both_compound() {
        let m = gen_std_msgs("DL/IU3ABC", "JN55", "OH0/AA1AA", -7, false);
        assert_eq!(m.tx2, "<OH0/AA1AA> IU3ABC -07");
        assert_eq!(m.tx4, "OH0/AA1AA <DL/IU3ABC> RR73");
    }

    /// `genCQMsg`.
    #[test]
    fn cq_messages() {
        assert_eq!(gen_cq("IU3ABC", "JN55rk", None), "CQ IU3ABC JN55");
        assert_eq!(gen_cq("IU3ABC", "JN55", Some("dx")), "CQ DX IU3ABC JN55");
        assert_eq!(gen_cq("IU3ABC/P", "JN55", None), "CQ IU3ABC/P JN55");
        assert_eq!(gen_cq("DL/IU3ABC", "JN55", Some("DX")), "CQ DL/IU3ABC");
    }

    #[test]
    fn get_by_number() {
        let m = gen_std_msgs("IU3ABC", "JN55", "LZ2KSQ", -7, false);
        assert_eq!(m.get(2), Some("LZ2KSQ IU3ABC -07"));
        assert_eq!(m.get(6), None);
        assert_eq!(m.get(0), None);
    }
}
