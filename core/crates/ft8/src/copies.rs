//! Ridecodifica di un messaggio gia' sentito nel periodo precedente della stessa parita' (JTDX 2.2.159, sorgenti pubblici letti alla
//! lettera: `lib/ft8b.f90` con `lsd`/`iqso = 4`/`ldeepsync`, `lib/tonesd.f90`, `lib/ft8sd1.f90`, `lib/ft8mf1.f90`, `lib/ft8mfcq.f90`,
//! `lib/sync8d.f90` e la lista `msgcq25` di `lib/cwfilter.f90`; la memoria `evencopy`/`oddcopy` di `lib/decoder.f90` e
//! `lib/ft8_decode.f90`).
//!
//! Se un candidato cade dove 30 s prima c'era un messaggio decodificato (a meno di 3 Hz e 0,19 s), la stazione con ogni probabilita'
//! ripete lo stesso messaggio o manda il seguente dello stesso QSO. Allora, oltre alla decodifica normale:
//! - [`ft8sd1`] confronta i toni piu' forti di ogni simbolo con quelli del messaggio di prima (o, per un rapporto, anche con RRR, RR73
//!   e 73 della stessa coppia di nominativi);
//! - [`ft8mf1`] (non CQ) prova i 76 seguiti possibili della coppia di nominativi (rapporti, R+rapporto, RRR, RR73, 73, locatore) e
//!   [`ft8mfcq`] (CQ) il CQ di prima contro 24 CQ di riferimento: vince solo se spicca netto sugli altri.
//!
//! Con `ldeepsync` (Modo SWL, sottopassate, soglia bassa) prima di queste prove il segnale si risincronizza usando anche i toni
//! noti del messaggio atteso ([`deep_refine`]), come `sync8d` con `iqso = 4`.

use std::sync::OnceLock;

use crate::message::CallHashes;
use crate::sync::{cd_at, freq_tweak, Complex, SyncKind};
use crate::{NN, NSPS};

/// Campioni di un simbolo a 200 Hz.
const SYM200: usize = 32;
/// Entro questa distanza (Hz e s) il candidato e' sul messaggio ricordato (`ft8b`: `abs(...%freq-f1).lt.3.0 .and. abs(...%dt-xdt).lt.0.19`).
pub const MATCH_FREQ_HZ: f32 = 3.0;
pub const MATCH_DT_S: f32 = 0.19;
/// Messaggi ricordati per parita' (`evencopy(130)`).
pub const MAX_COPIES: usize = 130;

/// I rapporti di `tonesd`/`ft8mf1`, nell'ordine di JTDX (1-36 rapporti, 37-72 R+rapporto, 73 RRR, 74 RR73, 75 73).
const RPT: [&str; 75] = [
    "+09", "+08", "+07", "+06", "+05", "+04", "+03", "+02", "+01", "+00", "-01", "-02", "-03", "-04", "-05", "-06", "-07", "-08", "-09", "-10", "-11", "-12", "-13", "-14", "-15", "-16",
    "-17", "-18", "-19", "-20", "-21", "-22", "-23", "-24", "-25", "-26", "R+09", "R+08", "R+07", "R+06", "R+05", "R+04", "R+03", "R+02", "R+01", "R+00", "R-01", "R-02", "R-03", "R-04",
    "R-05", "R-06", "R-07", "R-08", "R-09", "R-10", "R-11", "R-12", "R-13", "R-14", "R-15", "R-16", "R-17", "R-18", "R-19", "R-20", "R-21", "R-22", "R-23", "R-24", "R-25", "R-26", "RRR",
    "RR73", "73",
];

/// I CQ di riferimento di `ft8mfcq` (`msgcq25(2:25)` di `cwfilter`).
const CQ25: [&str; 24] = [
    "CQ 2E0DLA IO92",
    "CQ BH3NEB ON81",
    "CQ CG3CGT FN04",
    "CQ DX CT1JA IM59",
    "CQ CU20E",
    "CQ NA CX1OB GF14",
    "CQ DF2AJ JN49",
    "CQ DG4XPZ JN58",
    "CQ EA8XR IL19",
    "CQ F1YE IN94",
    "CQ DX G1KLN IO82",
    "CQ HL2KVF PM38",
    "CQ IU1ZSV JN45",
    "CQ JG1TWO PM96",
    "CQ K2ST EM96",
    "CQ N9TUX EL98",
    "CQ SA NO2FA FM18",
    "CQ OH6GKE KP13",
    "CQ PD0ORM JO24",
    "CQ DX PT7DS HI06",
    "CQ RA3XEP KO84",
    "CQ SM2GSH KP05",
    "CQ UA9OHX NO15",
    "CQ JA W0YH EN12",
];

/// Un messaggio decodificato tenuto per il prossimo periodo della stessa parita'.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyMessage {
    pub text: String,
    pub freq_hz: f32,
    pub dt_s: f32,
}

/// I toni (79) di un messaggio (`genft8sd`); tutti zero se non si impacchetta, come in JTDX.
pub fn tones_of(text: &str) -> [u8; NN] {
    crate::message::pack(text, &mut CallHashes::default())
        .map(|bits| crate::modem::tones_from_codeword(&crate::ldpc::encode(&crate::crc::append_crc(&bits))))
        .unwrap_or([0u8; NN])
}

/// I 58 toni di dati (`idtone(1:29)=itone(8:36)`, `idtone(30:58)=itone(44:72)`).
fn data_tones(t: &[u8; NN]) -> [u8; 58] {
    let mut d = [0u8; 58];
    d[..29].copy_from_slice(&t[7..36]);
    d[29..].copy_from_slice(&t[43..72]);
    d
}

/// La colonna di `s8` del simbolo di dati `i` (0..58).
fn data_col(i: usize) -> usize {
    if i < 29 { i + 7 } else { i + 14 }
}

fn argmax(v: &[f32; 8]) -> usize {
    // `maxloc`: il primo massimo
    let mut best = 0;
    for (i, &x) in v.iter().enumerate() {
        if x > v[best] {
            best = i;
        }
    }
    best
}

/// Le parti di un messaggio come le legge JTDX: primo e secondo nominativo e terza parola (vuota se non c'e').
struct Parts<'a> {
    c1: &'a str,
    c2: &'a str,
    third: &'a str,
}

fn parts(msgd: &str) -> Option<Parts<'_>> {
    let mut it = msgd.split(' ');
    let c1 = it.next()?;
    let c2 = it.next()?;
    let third = it.next().unwrap_or("");
    (c1.len() <= 12 && c2.len() <= 12).then_some(Parts { c1, c2, third })
}

/// Il messaggio di prima e' un CQ (`msgd(1:3).eq.'CQ '`, `'DE '`, `'QRZ '`).
pub fn is_cq(msgd: &str) -> bool {
    msgd.starts_with("CQ ") || msgd.starts_with("DE ") || msgd.starts_with("QRZ ")
}

/// `ft8sd1`: il messaggio di prima (o il suo seguito RRR/RR73/73) confrontato tono per tono con i simboli ricevuti.
pub fn ft8sd1(s8: &[[f32; 8]; NN], msgd: &str, lcq: bool, mycall: &str) -> Option<(String, [u8; NN])> {
    let mut lr73 = false;
    let mut lgrid = false;
    let mut msg4: Vec<String> = Vec::new();
    if !lcq {
        let p = parts(msgd)?;
        if p.c1.len() < 3 || p.c2.len() < 3 || p.c2 == mycall.trim() {
            return None;
        }
        lr73 = msgd.contains(" RR73") || msgd.contains(" 73");
        lgrid = !lr73 && p.third.len() == 4 && !p.third.starts_with("R+") && !p.third.starts_with("R-");
        if !lgrid && !lr73 {
            msg4 = vec![msgd.to_string(), format!("{} {} RRR", p.c1, p.c2), format!("{} {} RR73", p.c1, p.c2), format!("{} {} 73", p.c1, p.c2)];
        }
    }
    let single = lcq || lgrid || lr73;
    let single_tones = if single { tones_of(msgd) } else { [0u8; NN] };
    let four: Vec<[u8; NN]> = msg4.iter().map(|m| tones_of(m)).collect();

    // i toni piu' forti di ogni simbolo di dati, poi tolti per trovare i secondi
    let mut s8_1 = *s8;
    let mut dem = [0u8; 58];
    for (i, d) in dem.iter_mut().enumerate() {
        let col = data_col(i);
        let ip = argmax(&s8_1[col]);
        *d = ip as u8;
        s8_1[col][ip] = 0.0;
    }
    let count = |idtone: &[u8; 58], dem: &[u8; 58], skip: Option<&[bool; 58]>| -> (usize, usize) {
        let (mut n, mut parity) = (0, 0);
        for k in 0..58 {
            if skip.is_some_and(|s| s[k]) {
                continue;
            }
            if idtone[k] == dem[k] {
                n += 1;
                if k >= 25 {
                    parity += 1;
                }
            }
        }
        (n, parity)
    };

    let (idtone, text, tones, nmatch1, ncrc1) = if single {
        let idt = data_tones(&single_tones);
        let (n, p) = count(&idt, &dem, None);
        (idt, msgd.to_string(), single_tones, n, p)
    } else {
        let mut imax = None;
        let (mut best_n, mut best_p) = (0, 0);
        for (i, t) in four.iter().enumerate() {
            let (n, p) = count(&data_tones(t), &dem, None);
            if n > best_n {
                imax = Some(i);
                best_n = n;
                best_p = p;
            }
        }
        let imax = imax?;
        if lr73 && imax == 1 {
            return None; // RRR is not a valid message
        }
        (data_tones(&four[imax]), msg4[imax].clone(), four[imax], best_n, best_p)
    };
    if nmatch1 > 29 && ncrc1 > 10 {
        return Some((text, tones));
    }
    if nmatch1 >= 22 {
        let mut matched = [false; 58];
        for k in 0..58 {
            matched[k] = idtone[k] == dem[k];
        }
        // dove il tono piu' forte non era quello atteso si guarda il secondo
        let mut dem2 = dem;
        for (i, d) in dem2.iter_mut().enumerate() {
            if !matched[i] {
                *d = argmax(&s8_1[data_col(i)]) as u8;
            }
        }
        let (n, p) = count(&idtone, &dem2, Some(&matched));
        if nmatch1 + n > 41 && ncrc1 + p > 19 {
            return Some((text, tones));
        }
    }
    None
}

/// Le ampiezze dei 58 simboli di dati, il tono migliore e il secondo di ognuno e il riferimento (`ft8mf1`/`ft8mfcq`).
fn match_score(s8: &[[f32; 8]; NN], rows: &[[u8; 58]]) -> Option<(usize, f32, f32)> {
    let mut s8d = [[0.0f32; 8]; 58];
    for (i, row) in s8d.iter_mut().enumerate() {
        *row = s8[data_col(i)];
    }
    let mut mrs = [0usize; 58];
    let mut mrs2 = [0usize; 58];
    for j in 0..58 {
        let (mut s1, mut s2) = (-1.0e6f32, -1.0e6f32);
        let (mut i1, mut i2) = (0, 0);
        for i in 0..8 {
            if s8d[j][i] > s1 {
                s1 = s8d[j][i];
                i1 = i;
            }
        }
        for i in 0..8 {
            if i != i1 && s8d[j][i] > s2 {
                s2 = s8d[j][i];
                i2 = i;
            }
        }
        mrs[j] = i1;
        mrs2[j] = i2;
    }
    let ref0: f32 = (0..58).map(|i| s8d[i][mrs[i]]).sum();
    let (mut ipk, mut u1, mut u2) = (None, 0.0f32, 0.0f32);
    for (k, row) in rows.iter().enumerate() {
        let mut psum = 0.0f32;
        let mut r = ref0;
        for j in 0..58 {
            let t = row[j] as usize;
            psum += s8d[j][t];
            if t == mrs[j] {
                r = r - s8d[j][t] + s8d[j][mrs2[j]];
            }
        }
        if r == 0.0 || !r.is_finite() {
            continue;
        }
        let p = psum / r;
        if p > u1 {
            u2 = u1;
            u1 = p;
            ipk = Some(k);
        } else if p > u2 {
            u2 = p;
        }
    }
    ipk.map(|k| (k, u1, u2))
}

/// La soglia comune di `ft8mf1` e `ft8mfcq`.
fn clear_winner(u1: f32, u2: f32) -> bool {
    let qual = 100.0 * (u1 - u2);
    let thresh = (qual + 10.0) * (u1 - 0.6);
    thresh > 4.0 && qual > 2.6 && u1 > 0.77
}

/// I 76 seguiti di `tonesd` per la coppia di nominativi di `msgd`: testo e toni.
fn variants76(p: &Parts<'_>, msgd: &str) -> Vec<(String, [u8; NN])> {
    let lr73 = msgd.contains(" RR73") || msgd.contains(" 73");
    let lgrid = !lr73 && p.third.len() == 4 && !p.third.starts_with("R+") && !p.third.starts_with("R-");
    let grid = if lgrid { p.third } else { "AA00" };
    let mut v: Vec<(String, [u8; NN])> = RPT
        .iter()
        .map(|r| {
            let m = format!("{} {} {}", p.c1, p.c2, r);
            let t = tones_of(&m);
            (m, t)
        })
        .collect();
    let m = format!("{} {} {}", p.c1, p.c2, grid);
    let t = tones_of(&m);
    v.push((m, t));
    v
}

/// `ft8mf1`: il seguito piu' probabile tra i 76 della stessa coppia di nominativi, coerente con il tipo del messaggio di prima.
pub fn ft8mf1(s8: &[[f32; 8]; NN], msgd: &str) -> Option<(String, [u8; NN])> {
    let p = parts(msgd)?;
    if p.c1.len() < 3 || p.c2.len() < 3 {
        return None;
    }
    let lr73 = p.third == "RR73" || p.third == "73";
    let lgrid = !lr73 && p.third.len() == 4 && !p.third.starts_with("R+") && !p.third.starts_with("R-");
    let lreport = p.third.starts_with('+') || p.third.starts_with('-');
    let lrreport = p.third.starts_with("R+") || p.third.starts_with("R-");
    let lrrr = p.third == "RRR";
    let variants = variants76(&p, msgd);
    let rows: Vec<[u8; 58]> = variants.iter().map(|(_, t)| data_tones(t)).collect();
    let (k, u1, u2) = match_score(s8, &rows)?;
    let ipk = k + 1; // da 1 come in JTDX
    if lgrid {
        if ipk == 76 && variants[75].0.ends_with(" AA00") {
            return None;
        }
        if ipk < 37 || (ipk > 72 && ipk < 76) {
            return None;
        }
    }
    if lreport && ((ipk > 36 && ipk < 73) || ipk == 76) {
        return None;
    }
    if lrreport && (ipk < 37 || ipk == 73 || ipk == 76) {
        return None;
    }
    if lrrr && (ipk < 73 || ipk == 76) {
        return None;
    }
    if lr73 && (ipk < 74 || ipk == 76) {
        return None;
    }
    clear_winner(u1, u2).then(|| variants[k].clone())
}

fn cq_rows() -> &'static [[u8; 58]; 24] {
    static ROWS: OnceLock<[[u8; 58]; 24]> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut rows = [[0u8; 58]; 24];
        for (row, text) in rows.iter_mut().zip(CQ25.iter()) {
            *row = data_tones(&tones_of(text));
        }
        rows
    })
}

/// `ft8mfcq`: il CQ di prima, accettato solo se spicca sui 24 CQ di riferimento.
pub fn ft8mfcq(s8: &[[f32; 8]; NN], msgd: &str) -> Option<(String, [u8; NN])> {
    if msgd.trim_end().len() < 6 {
        return None;
    }
    let tones = tones_of(msgd);
    let mut rows = vec![data_tones(&tones)];
    rows.extend_from_slice(cq_rows());
    let (k, u1, u2) = match_score(s8, &rows)?;
    (k == 0 && clear_winner(u1, u2)).then(|| (msgd.to_string(), tones))
}

/// I simboli di riferimento (32 campioni a 200 Hz, onda GFSK a frequenza 0) del messaggio atteso per la risincronizzazione:
/// per un CQ i 58 simboli di dati del CQ di prima (`csyncsdcq`), altrimenti i simboli 8-26 del primo seguito, «c1 c2 +09» (`csyncsd`).
/// Ogni voce e' (simbolo, riferimento).
pub fn known_rows(msgd: &str, lcq: bool) -> Vec<(usize, [Complex; SYM200])> {
    let tones = if lcq {
        tones_of(msgd)
    } else {
        let Some(p) = parts(msgd) else { return Vec::new() };
        tones_of(&format!("{} {} {}", p.c1, p.c2, RPT[0]))
    };
    let (sin, cos) = crate::modem::gfsk_iq(&tones, 0.0, crate::modem::GFSK_BT);
    let row = |symbol: usize| -> [Complex; SYM200] {
        let mut r = [Complex::new(0.0, 0.0); SYM200];
        for (k, v) in r.iter_mut().enumerate() {
            let at = symbol * NSPS + 60 * k;
            *v = Complex::new(cos.get(at).copied().unwrap_or(0.0), sin.get(at).copied().unwrap_or(0.0));
        }
        r
    };
    if lcq { (0..58).map(|i| (data_col(i), row(data_col(i)))).collect() } else { (0..19).map(|i| (7 + i, row(7 + i))).collect() }
}

/// `sync8d` con `iqso = 4`: il sincronismo di Costas piu' quello dei simboli noti del messaggio atteso.
fn sync_known(cd: &[Complex], i0: i64, tweak: Option<&[Complex; SYM200]>, kind: SyncKind, rows: &[(usize, [Complex; SYM200])], lcq: bool) -> f32 {
    let mut sync = crate::sync::sync8d(cd, i0, tweak, kind, false);
    let z: Vec<Complex> = rows
        .iter()
        .map(|(symbol, r)| {
            let start = i0 + (*symbol * SYM200) as i64;
            let mut z = Complex::new(0.0, 0.0);
            for (n, rv) in r.iter().enumerate() {
                let rv = match tweak {
                    Some(t) => *rv * t[n],
                    None => *rv,
                };
                z += cd_at(cd, start + n as i64) * rv.conj();
            }
            z
        })
        .collect();
    let term = |z: Complex| match kind {
        SyncKind::Magnitude => z.norm(),
        SyncKind::Power => z.norm_sqr(),
        SyncKind::AvgAbs => z.re.abs() + z.im.abs(),
    };
    let mut z4 = Complex::new(0.0, 0.0);
    for (i, zi) in z.iter().enumerate() {
        // per i 19 simboli (non CQ) con la potenza si media ogni simbolo con il seguente; l'ultimo riusa il penultimo
        z4 = if !lcq && kind == SyncKind::Power { if i + 1 < z.len() { (*zi + z[i + 1]) / 2.0 } else { z4 } } else { *zi };
        sync += term(z4);
    }
    sync
}

/// La risincronizzazione di `ldeepsync`: ritardo a passi di 5 ms entro +-40 ms da `dt_s`, poi frequenza entro +-2,5 Hz a passi di
/// 0,5 Hz, sul segnale a 200 Hz `cd` (alla frequenza del candidato). Ritorna (scarto di frequenza in Hz, nuovo ritardo in s).
pub fn deep_refine(cd: &[Complex], dt_s: f32, kind: SyncKind, rows: &[(usize, [Complex; SYM200])], lcq: bool) -> (f32, f32) {
    let i0 = ((dt_s + 0.5) * 200.0).round() as i64;
    let (mut smax, mut ibest) = (0.0f32, i0);
    for idt in i0 - 8..=i0 + 8 {
        let s = sync_known(cd, idt, None, kind, rows, lcq);
        if s > smax {
            smax = s;
            ibest = idt;
        }
    }
    let (mut smax, mut delfbest) = (0.0f32, 0.0f32);
    for ifr in -5..=5 {
        let delf = ifr as f32 * 0.5;
        let s = sync_known(cd, ibest, Some(&freq_tweak(delf)), kind, rows, lcq);
        if s > smax {
            smax = s;
            delfbest = delf;
        }
    }
    (delfbest, ibest as f32 * 0.005 - 0.5)
}

/// Il messaggio decodificato va ricordato per il prossimo periodo della stessa parita'? (`ft8_decode.f90`: messaggi standard,
/// senza hash ne' barre, non nostri; i CQ anche non standard).
pub fn worth_keeping(text: &str, mycall: &str) -> bool {
    if text.contains('<') {
        return false;
    }
    if text.starts_with("CQ ") {
        return true;
    }
    let first = text.split(' ').next().unwrap_or("");
    if !mycall.trim().is_empty() && first == mycall.trim() {
        return false;
    }
    if text.contains('/') {
        return false;
    }
    // messaggio standard (i3 = 1): due nominativi e un rapporto, un locatore o RRR/RR73/73
    crate::message::pack(text, &mut CallHashes::default()).is_ok_and(|bits| crate::message::message_type(&bits).0 == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `s8` ideale di un messaggio: il tono giusto a 1, gli altri a `noise`.
    fn s8_of(text: &str, noise: f32) -> [[f32; 8]; NN] {
        let t = tones_of(text);
        let mut s8 = [[noise; 8]; NN];
        for (k, row) in s8.iter_mut().enumerate() {
            row[t[k] as usize] = 1.0;
        }
        s8
    }

    #[test]
    fn ft8sd1_finds_the_same_message_and_the_next_one_of_the_qso() {
        let s8 = s8_of("K1ABC W9XYZ -10", 0.1);
        assert_eq!(ft8sd1(&s8, "K1ABC W9XYZ -10", false, "IU3ABC").map(|r| r.0).as_deref(), Some("K1ABC W9XYZ -10"));
        let next = s8_of("K1ABC W9XYZ RR73", 0.1);
        assert_eq!(ft8sd1(&next, "K1ABC W9XYZ -10", false, "IU3ABC").map(|r| r.0).as_deref(), Some("K1ABC W9XYZ RR73"));
        assert!(ft8sd1(&s8, "K1ABC IU3ABC -10", false, "IU3ABC").is_none(), "non per i messaggi a noi");
    }

    #[test]
    fn ft8mf1_picks_the_report_and_respects_the_order_of_the_qso() {
        // la stessa stazione, dopo il suo rapporto, manda RR73 (l'R+rapporto e' dell'altra stazione)
        let s8 = s8_of("K1ABC W9XYZ RR73", 0.1);
        assert_eq!(ft8mf1(&s8, "K1ABC W9XYZ -10").map(|r| r.0).as_deref(), Some("K1ABC W9XYZ RR73"));
        let r = s8_of("K1ABC W9XYZ R-07", 0.1);
        assert!(ft8mf1(&r, "K1ABC W9XYZ -10").is_none());
        // dopo un R+rapporto non si torna a un rapporto semplice
        let back = s8_of("K1ABC W9XYZ -07", 0.1);
        assert!(ft8mf1(&back, "K1ABC W9XYZ R-10").is_none());
    }

    #[test]
    fn ft8mfcq_accepts_only_the_same_cq() {
        let s8 = s8_of("CQ K1ABC FN42", 0.1);
        assert!(ft8mfcq(&s8, "CQ K1ABC FN42").is_some());
        let other = s8_of("CQ DL1ABC JO62", 0.1);
        assert!(ft8mfcq(&other, "CQ K1ABC FN42").is_none());
    }

    #[test]
    fn random_symbols_do_not_match_anything() {
        let mut s8 = [[0.0f32; 8]; NN];
        let mut x = 12345u64;
        for row in s8.iter_mut() {
            for v in row.iter_mut() {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                *v = ((x >> 33) as f32) / (u32::MAX as f32);
            }
        }
        assert!(ft8sd1(&s8, "K1ABC W9XYZ -10", false, "IU3ABC").is_none());
        assert!(ft8mf1(&s8, "K1ABC W9XYZ -10").is_none());
        assert!(ft8mfcq(&s8, "CQ K1ABC FN42").is_none());
    }

    #[test]
    fn only_standard_messages_not_ours_are_kept() {
        assert!(worth_keeping("K1ABC W9XYZ -10", "IU3ABC"));
        assert!(worth_keeping("CQ K1ABC FN42", "IU3ABC"));
        assert!(!worth_keeping("IU3ABC W9XYZ -10", "IU3ABC"));
        assert!(!worth_keeping("<K1ABC> W9XYZ -10", "IU3ABC"));
        assert!(!worth_keeping("K1ABC/P W9XYZ -10", "IU3ABC"));
    }
}
