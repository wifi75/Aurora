//! Filtri sui falsi: un messaggio che passa il CRC (14 bit: uno ogni 16384 candidati sbagliati passa) non e' ancora un messaggio vero.
//! Tutto viene da `ft8b.f90` di JTDX 2.2.159 e dai suoi `chkfalse8`, `chklong8`, `filtersfree`, `callsign_q`/`call_q`:
//!
//! 1. [`sync_screen`]: controlli sul sincronismo **prima** di LDPC (`nsync`, `nsyncscore`, `scoreratio`, `nsyncscorew`): scartano i
//!    candidati dove i 21 simboli di Costas non spiccano abbastanza. Le soglie sono quelle calibrate da JTDX su dati veri (nei
//!    commenti del Fortran: "377 out of 20709"). Fanno anche risparmiare tempo, perche' LDPC e OSD non partono.
//! 2. [`accept_decode`]: soglie su `nharderrors + dmin` (meno di 60) e, con i tipi a priori diversi da CQ e nominativo, al massimo 39 bit
//!    ribaltati.
//! 3. [`reject_message`]: controlli sul testo (forma dei nominativi, tipi di messaggio impossibili, testo libero, 73/RR73 su
//!    nominativi nascosti), scattano solo sui messaggi "dubbi" (qualita' bassa, SNR basso, ritardo fuori scala, tipi a priori 1-3) come
//!    in `ft8b`.
//!
//! # Differenze dal Fortran
//!
//! - Mancano i controlli che consultano le tabelle di JTDX: `ALLCALL7.TXT` (`searchcalls`, `chkflscall`: il nominativo e' fra quelli
//!   conosciuti?) e la geografia (`chkgrid`: il locatore e' compatibile con il prefisso?). Quelli di `chkflscall` si ottengono
//!   passando un [`CallBook`] a [`reject_message`]: va riempito con un elenco **completo** di nominativi, altrimenti scarta stazioni
//!   vere mai sentite. `chkgrid` non e' portato (e' una tabella di migliaia di condizioni).
//! - `filtersfree` non usa `datacor` (la correlazione dei dati): scarta di meno i testi liberi falsi.
//! - Non c'e' il ramo Hound/Fox ne' i controlli sui nominativi non standard del proprio QSO (`iaptype` 11, 21, 40, 41).

use std::collections::HashSet;

use crate::ap::argmax8;
use crate::{COSTAS, NN};

/// Elenco di nominativi conosciuti (l'equivalente di `ALLCALL7.TXT` di JTDX).
#[derive(Debug, Clone, Default)]
pub struct CallBook {
    calls: HashSet<String>,
}

impl CallBook {
    pub fn new<I: IntoIterator<Item = String>>(calls: I) -> CallBook {
        CallBook { calls: calls.into_iter().map(|c| c.trim().to_uppercase()).filter(|c| !c.is_empty()).collect() }
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn contains(&self, call: &str) -> bool {
        self.calls.contains(call)
    }

    /// `searchcalls`: con un solo nominativo di piu' di 7 caratteri risponde sempre "trovato" (non si possono tenere tutti gli speciali).
    fn search(&self, a: &str, b: &str) -> bool {
        if self.is_empty() {
            return true;
        }
        if a.len() > 7 && b.is_empty() {
            return true;
        }
        let b = if b.len() > 7 { "" } else { b };
        [a, b].iter().any(|c| !c.is_empty() && *c != "TU;" && self.contains(c))
    }

    /// `chkflscall`: `true` se il messaggio con questi due nominativi e' un falso (nessuno dei due e' conosciuto).
    /// `first` e' `MYCALL` o `CQ` quando il primo campo non e' un nominativo.
    fn is_false(&self, first: &str, second: &str) -> bool {
        if first.starts_with("<.") {
            return false;
        }
        if first == "MYCALL" || first == "CQ" {
            !self.search(second, "")
        } else if second.starts_with('<') {
            !self.search(first, "")
        } else {
            !self.search(first, second)
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Sincronismo

/// Esito di [`sync_screen`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncScreen {
    /// Il candidato supera i controlli.
    pub pass: bool,
    /// Candidato CQ con sincronismo debole (`lapcqonly`): si prova solo il tipo a priori CQ.
    pub cq_only: bool,
    /// Le distanze dei toni di Costas dicono che senza a priori non vale la pena (`lskipnotap`).
    pub skip_plain: bool,
}

const REJECT: SyncScreen = SyncScreen { pass: false, cq_only: false, skip_plain: false };

/// I controlli di sincronismo di `ft8b` (da `nsyncscorew` a `go to 32`), sulle ampiezze `s8` dei toni nei 79 simboli.
///
/// `rrxdt` e' il ritardo nella convenzione di JTDX (lo 0,5 s tolto); `cq_candidate` l'indicatore CQ di `sync8`; con `score_filter`
/// falso si applicano solo i controlli di base (`nsync`), come JTDX per il segnale sulla frequenza di QSO.
pub fn sync_screen(s8: &[[f32; 8]; NN], rrxdt: f32, cq_candidate: bool, score_filter: bool) -> SyncScreen {
    let tone = |k: usize| usize::from(COSTAS[k]);
    let sum = |col: usize| -> f32 { s8[col].iter().sum() };

    // nsyncscorew: sui simboli di Costas "validi" secondo il ritardo
    let mut syncw = [0.0f32; 7];
    let mut sumkw = [0.0f32; 7];
    if (-0.5..=2.13).contains(&rrxdt) {
        for k in 0..7 {
            syncw[tone(k)] = s8[k][tone(k)] + s8[k + 36][tone(k)] + s8[k + 72][tone(k)];
        }
        for t in 0..7 {
            let all: f32 = s8.iter().map(|r| r[t]).sum();
            sumkw[t] = (all - syncw[t]) / 25.333;
        }
    } else if rrxdt < -0.5 {
        for k in 0..7 {
            syncw[tone(k)] = s8[k + 36][tone(k)] + s8[k + 72][tone(k)];
        }
        for t in 0..7 {
            let part: f32 = s8[25..79].iter().map(|r| r[t]).sum();
            sumkw[t] = (part - syncw[t]) / 26.0;
        }
    } else {
        for k in 0..7 {
            syncw[tone(k)] = s8[k][tone(k)] + s8[k + 36][tone(k)];
        }
        for t in 0..7 {
            let part: f32 = s8[..54].iter().map(|r| r[t]).sum();
            sumkw[t] = (part - syncw[t]) / 26.0;
        }
    }
    let nsyncscorew = (0..7).filter(|&k| syncw[k] > sumkw[k]).count();

    // simboli di Costas col tono giusto piu' forte (is1..is3) o secondo (nsync2)
    let mut is = [0usize; 3];
    let mut nsync2 = 0usize;
    for k in 0..7 {
        for (b, col) in [k, k + 36, k + 72].into_iter().enumerate() {
            let ip = argmax8(&s8[col]);
            if ip == tone(k) {
                is[b] += 1;
            } else {
                let mut row = s8[col];
                row[ip] = 0.0;
                if argmax8(&row) == tone(k) {
                    nsync2 += 1;
                }
            }
        }
    }
    // nsyncscore e scoreratio per blocco
    let (mut n1, mut n2, mut n3) = (0usize, 0usize, 0usize);
    let (mut r1, mut r2, mut r3) = (0.0f32, 0.0f32, 0.0f32);
    for k in 0..7 {
        let block = |col: usize, n: &mut usize, r: &mut f32| {
            let synck = s8[col][tone(k)];
            let sumk = (sum(col) - synck) / 7.0;
            if synck > sumk {
                *n += 1;
                *r += synck / sumk;
            }
        };
        if rrxdt >= -0.5 {
            block(k, &mut n1, &mut r1);
        }
        block(k + 36, &mut n2, &mut r2);
        if rrxdt <= 2.13 {
            block(k + 72, &mut n3, &mut r3);
        }
    }
    let nsyncscore = n1 + n2 + n3;
    let mut scoreratio = r1 + r2 + r3;
    let nsync = is[0] + is[1] + is[2];

    // candidato CQ: il sincronismo debole si compensa con i simboli di dati del CQ
    let mut cq_only = false;
    if cq_candidate {
        let rscq = crate::ap::cq_score(s8);
        match nsync {
            4 => {
                if nsync + nsync2 < 12 && rscq < 6.6 {
                    return REJECT;
                }
                cq_only = true;
            }
            5 => {
                if nsync + nsync2 < 12 && rscq < 6.1 {
                    return REJECT;
                }
                cq_only = true;
            }
            6 => {
                if nsync + nsync2 < 11 && rscq < 5.6 {
                    return REJECT;
                }
                cq_only = true;
            }
            _ => {
                if nsync < 7 {
                    return REJECT;
                }
            }
        }
    } else if nsync < 7 {
        return REJECT;
    }

    // lskipnotap: come sono messi i toni giusti nella classifica di ogni simbolo di Costas (`syncdist.f90`)
    let mut skip_plain = false;
    if !cq_only && nsync < 11 {
        let mut nsmax = [0usize; 8];
        for k in 0..7 {
            for col in [k, k + 36, k + 72] {
                let mut row = s8[col];
                for slot in nsmax.iter_mut() {
                    let ip = argmax8(&row);
                    if ip == tone(k) {
                        *slot += 1;
                        break;
                    }
                    row[ip] = 0.0;
                }
            }
        }
        let low = nsmax[1] + nsmax[2];
        if nsmax[6] + nsmax[7] > low || nsmax[4] + nsmax[5] > low {
            skip_plain = true;
        }
    }

    // `scoreratio2` non si media (come nel Fortran: le soglie sono state calibrate cosi')
    let mean = |sum: f32, n: usize| if n > 0 { sum / n as f32 } else { 0.0 };
    scoreratio = mean(scoreratio, nsyncscore);
    let sr1 = mean(r1, n1);
    let sr2 = r2;
    let sr3 = mean(r3, n3);

    let ok = SyncScreen { pass: true, cq_only, skip_plain };
    if !score_filter {
        return ok;
    }
    let verdict = if (-0.5..=2.13).contains(&rrxdt) {
        score_in_range(nsyncscore, scoreratio, nsyncscorew, [n1, n2, n3], [sr1, sr2, sr3])
    } else if rrxdt < -0.5 {
        score_before(nsyncscore, scoreratio, nsyncscorew, [n2, n3], [sr2, sr3])
    } else {
        score_after(nsyncscore, scoreratio, nsyncscorew, [n1, n2], [sr1, sr2])
    };
    if verdict { ok } else { REJECT }
}

/// Ramo `rrxdt` tra -0,5 e 2,13: `true` = il candidato passa.
fn score_in_range(score: usize, ratio: f32, w: usize, n: [usize; 3], sr: [f32; 3]) -> bool {
    let [n1, n2, n3] = n;
    let [sr1, sr2, sr3] = sr;
    if score < 8 || (score < 10 && ratio < 5.5) || (score < 11 && ratio < 3.63) {
        false
    } else if score == 11 && ratio < 5.37 {
        !(n1 < 5 && n3 < 5 && sr1 < 4.2 && sr3 < 4.2)
    } else if score == 12 && ratio < 4.6 {
        !(n1 < 5 && n3 < 5 && sr1 < 4.0 && sr3 < 4.0)
    } else if score == 13 && ratio < 4.4 {
        !(n1 < 5 && n2 < 6 && n3 < 5 && sr1 < 4.4 && sr3 < 4.4)
    } else if w < 3 {
        (n1 > 5 && sr1 > 13.8) || (n2 > 5 && sr2 > 13.8) || (n3 > 5 && sr3 > 13.8)
    } else if w == 3 {
        sr1 > 15.0 || sr2 > 15.0 || sr3 > 15.0
    } else if w == 4 {
        n1 == 7 || n2 == 7 || n3 == 7 || sr1 > 10.0 || sr2 > 10.0 || sr3 > 10.0
    } else if w == 5 {
        score > 17 || n1 == 7 || n2 == 7 || n3 == 7 || sr1 > 10.0 || sr2 > 10.0 || sr3 > 10.0
    } else {
        true
    }
}

/// Ramo `rrxdt < -0,5` (manca il primo blocco di Costas): `n`/`sr` sono secondo e terzo blocco.
fn score_before(score: usize, ratio: f32, w: usize, n: [usize; 2], sr: [f32; 2]) -> bool {
    let [n2, n3] = n;
    let [sr2, sr3] = sr;
    if score < 6 || (score > 5 && score < 8 && w < 6 && sr2 < 5.5 && sr3 < 5.5) {
        false
    } else if score == 8 {
        !(n2 < 6 && n3 < 6 && sr2 < 6.6 && sr3 < 6.6)
    } else if score == 9 && ratio < 6.0 {
        !(n2 < 6 && n3 < 6 && sr2 < 6.6 && sr3 < 6.5)
    } else if w < 3 {
        (n2 > 5 && sr2 > 13.8) || (n3 > 5 && sr3 > 13.8)
    } else if w == 3 {
        // `nsyncscore3.gt.15` nel Fortran: un conteggio che non supera mai 7, quindi conta solo scoreratio2
        sr2 > 15.0 || n3 > 15
    } else if w == 4 {
        n2 == 7 || n3 == 7 || sr2 > 10.0 || n3 > 10
    } else if w == 5 {
        score > 11 || n2 == 7 || n3 == 7 || sr2 > 10.0 || sr3 > 10.0
    } else {
        true
    }
}

/// Ramo `rrxdt > 2,13` (manca il terzo blocco): `n`/`sr` sono primo e secondo blocco.
fn score_after(score: usize, ratio: f32, w: usize, n: [usize; 2], sr: [f32; 2]) -> bool {
    let [n1, n2] = n;
    let [sr1, sr2] = sr;
    if score < 6 || (score > 5 && score < 8 && w < 6 && sr1 < 5.5 && sr2 < 5.5) {
        false
    } else if score == 8 {
        !(n1 < 6 && n2 < 6 && sr1 < 6.6 && sr2 < 6.6)
    } else if score == 9 && ratio < 6.0 {
        !(n1 < 6 && n2 < 6 && sr2 < 6.6 && sr1 < 6.5)
    } else if w < 3 {
        (n1 > 5 && sr1 > 13.8) || (n2 > 5 && sr2 > 13.8)
    } else if w == 3 {
        sr1 > 15.0 || sr2 > 15.0
    } else if w == 4 {
        n1 == 7 || n2 == 7 || sr1 > 10.0 || n2 > 10
    } else if w == 5 {
        score > 11 || n1 == 7 || n2 == 7 || sr1 > 10.0 || sr2 > 10.0
    } else {
        true
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Accettazione della parola trovata

/// SNR "stile JTDX" dai toni decodificati (`xsnr` di `ft8b`): potenza del tono giusto contro la media degli altri sette, simbolo per
/// simbolo, con i ritocchi per i segnali forti e per quelli sotto -17 dB. `ap` e' vero se il messaggio e' venuto da un tipo a priori
/// (il pavimento scende da -23 a -24 dB).
pub fn jtdx_snr(s8: &[[f32; 8]; NN], tones: &[u8; NN], ap: bool) -> f32 {
    let mut total = 0.001f32;
    for (row, &t) in s8.iter().zip(tones.iter()) {
        let xsig = row[usize::from(t)] * row[usize::from(t)];
        let power: f32 = row.iter().map(|v| v * v).sum();
        let mut xnoi = (power - xsig) / 7.0;
        if xnoi < 0.01 {
            xnoi = 0.01;
        }
        total += if xnoi < xsig { xsig / xnoi } else { 1.01 };
    }
    let mut xsnr = total / 79.0 - 1.0;
    xsnr = 10.0 * xsnr.max(1e-6).log10() - 26.5;
    if xsnr > 7.0 {
        xsnr += (xsnr - 7.0) / 2.0;
    }
    if xsnr > 30.0 {
        xsnr -= 1.0;
        if xsnr > 40.0 {
            xsnr -= 1.0;
        }
        if xsnr > 49.0 {
            xsnr = 49.0;
        }
    }
    if xsnr < -17.0 {
        if xsnr < -22.5 && xsnr > -23.5 {
            xsnr = -22.5;
        }
        xsnr = xsnr - (1.0 + 1.4 / (23.0 + xsnr)).powi(2) + 1.2;
    }
    let floor = if ap { -24.0 } else { -23.0 };
    xsnr.max(floor)
}

/// Soglie di `ft8b` sulla parola trovata: `nharderrors + dmin < 60` e, per i tipi a priori oltre il secondo (indice di lettura > 2),
/// al massimo 39 bit ribaltati.
pub fn accept_decode(hard_errors: u32, dmin: f32, extra_read: bool) -> bool {
    if f64::from(hard_errors) + f64::from(dmin) >= 60.0 {
        return false;
    }
    !(extra_read && hard_errors > 39)
}

// ---------------------------------------------------------------------------------------------------------------------
// Testo del messaggio

/// Il messaggio che il filtro deve valutare.
#[derive(Debug, Clone, Copy)]
pub struct MessageCheck<'a> {
    /// Il testo come esce da `unpack` (con `<...>` per gli hash sconosciuti).
    pub text: &'a str,
    pub i3: u8,
    pub n3: u8,
    /// Tipo a priori con cui e' stato decodificato (0 = nessuno).
    pub ap_type: u8,
    /// `1 - (nharderrors + dmin) / 60`.
    pub qual: f32,
    /// SNR in dB come lo scrive JTDX.
    pub snr: f32,
    /// Ritardo (s, convenzione di JTDX: 0 e' 0,5 s dall'inizio del periodo).
    pub dt_s: f32,


}

/// Toglie le parentesi angolari ai nominativi sciolti (`delbraces`), lasciando `<...>`.
fn without_braces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if let Some(after) = tail.strip_prefix("<...>") {
            out.push_str("<...>");
            rest = after;
        } else if let Some(j) = tail.find('>') {
            out.push_str(&tail[1..j]);
            rest = &tail[j + 1..];
        } else {
            out.push_str(tail);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_letter(c: u8) -> bool {
    c.is_ascii_uppercase()
}

/// `callsign_q.f90` / `call_q.f90`: un nominativo non puo' cominciare con Q, con 0, con due cifre, con tre lettere, con una cifra
/// seguita da due lettere (salvo 3DA e 3XY). `true` = impossibile.
fn call_impossible(call: &str) -> bool {
    let b = call.as_bytes();
    let at = |i: usize| b.get(i).copied().unwrap_or(b' ');
    if at(0) == b'Q' || at(0) == b'0' || at(0) == b'/' {
        return true;
    }
    if is_digit(at(0)) {
        if is_digit(at(1)) {
            return true;
        }
        if is_letter(at(1)) && is_letter(at(2)) && is_letter(at(3)) {
            return true;
        }
        if is_letter(at(1)) && is_letter(at(2)) && !call.starts_with("3DA") && !call.starts_with("3XY") {
            return true;
        }
    }
    if is_letter(at(0)) && is_letter(at(1)) && is_letter(at(2)) {
        return true;
    }
    false
}

/// `chklong8`: i nominativi di 10 e 11 caratteri senza barra sono ammessi solo con certe forme (`DL2019ABCDE`, `ZW5STAYHOME`...).
fn long_call_is_false(call: &str) -> bool {
    if call.contains('/') || call.starts_with("8J") || call.starts_with("8N") {
        return false;
    }
    let mask: String = call.bytes().map(|c| if is_digit(c) { '1' } else { '0' }).collect();
    match call.len() {
        10 => ![
            "0011000000", "0011100000", "0011110000", "1011000000", "1011100000", "1011110000", "0010000000", "1010000000", "0110000000",
        ]
        .contains(&mask.as_str()),
        11 => !["00111000000", "00111100000", "10111000000", "10111100000", "00100000000", "01100000000", "10100000000"].contains(&mask.as_str()),
        _ => false,
    }
}

/// Un nominativo come lo scrive il messaggio, senza il suffisso /R o /P.
fn bare(call: &str) -> &str {
    call.strip_suffix("/R").or_else(|| call.strip_suffix("/P")).unwrap_or(call)
}

fn looks_like_grid(w: &str) -> bool {
    let b = w.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && is_digit(b[2]) && is_digit(b[3])
}

/// `filtersfree` (senza `datacor`): `true` se il testo libero di 13 caratteri ha l'aria di un falso. I testi che contengono parole e
/// formule da radioamatori (73, TU, GL, QSL, TNX, rapporti...) passano subito.
pub fn free_text_is_false(text: &str) -> bool {
    let mut d: Vec<u8> = text.bytes().take(13).collect();
    d.resize(13, b' ');
    let s = |from: usize, to: usize| -> &[u8] { &d[from..to.min(13)] };
    let has = |i: usize, pat: &str| -> bool { d.get(i..i + pat.len()).is_some_and(|w| w == pat.as_bytes()) };
    let ch = |i: usize| -> u8 { d.get(i).copied().unwrap_or(b' ') };
    if s(10, 12) == b"73" || s(11, 13) == b"73" {
        return false;
    }
    if ch(0) != b'/' {
        let (mut ndot, mut nsign, mut nother) = (0, 0, 0);
        for i in 0..13 {
            let c = ch(i);
            if has(i, "73") || has(i, "TU") || has(i, "GL") {
                return false;
            }
            if i > 3 {
                if (has(i, " -") || has(i, " +")) && (b'0'..b'3').contains(&ch(i + 2)) {
                    return false;
                }
                if (has(i, " R-") || has(i, " R+")) && (b'0'..b'3').contains(&ch(i + 3)) {
                    return false;
                }
            }
            if ["QSL", "TNX", "RR", "QSO", "CFM", "LOTW", "BND", "QSY", "BAND", "MAS", "HNY", "HPY", " DT", "XMS", "EASTE", "PIRAT", "TEST"]
                .iter()
                .any(|p| has(i, p))
                || ["/HYBR", "/HTML", "/PHOT", "/IMAG", "/JOIN", "QRZ.C", "/GIF", "/JPG", "/AVI", "/MP4", "/CMD"].iter().any(|p| has(i, p))
            {
                return false;
            }
            if c == b'.' {
                ndot += 1;
            }
            if c == b'-' || c == b'+' || c == b'?' {
                nsign += 1;
            }
            if c == b'/' || c == b'?' {
                nother += 1;
            }
        }
        if ndot >= 2 || nsign >= 2 || nother >= 2 {
            return true;
        }
        if has(0, "-0") {
            return true;
        }
        if ch(0) != b' ' && ch(1) == b' ' {
            return true;
        }
        if (is_letter(ch(12)) || is_digit(ch(12))) && ch(11) == b' ' {
            return true;
        }
        if matches!(ch(0), b'.' | b'+' | b'?' | b'/') || matches!(ch(12), b'.' | b'+' | b'-' | b'/') {
            return true;
        }
        if matches!(ch(1), b'-' | b'+') || matches!(ch(11), b'.' | b'+' | b'-') {
            return true;
        }
        if is_digit(ch(11)) && is_digit(ch(12)) && s(11, 13) != b"55" && s(11, 13) != b"73" && s(11, 13) != b"88" {
            return true;
        }
        if ch(0) == b'0' && ch(1) != b'.' {
            return true;
        }
        for i in 0..12 {
            if i < 11 && ch(i) == b'?' && ch(i + 1) != b' ' {
                return true;
            }
            if i < 11 && ch(i) != b' ' && ch(i + 1) == b'+' && (ch(i + 2) <= b'0' || ch(i + 2) >= b'9') {
                return true;
            }
            if is_letter(ch(i)) && ch(i + 1) == b'.' && is_letter(ch(i + 2)) {
                return true;
            }
            if (ch(i + 1) == b'/' && ch(i + 2) == b' ') || (ch(i + 1) == b' ' && ch(i + 2) == b'/') {
                return true;
            }
            if is_digit(ch(i)) && ch(i + 1) == b'/' && is_digit(ch(i + 2)) {
                return true;
            }
            if ch(i) == b' ' && is_letter(ch(i + 1)) && !matches!(ch(i + 1), b'F' | b'G' | b'I') && ch(i + 2) == b'/' {
                return true;
            }
            if ch(i) == b' ' && is_letter(ch(i + 1)) && matches!(ch(i + 2), b'.' | b'+' | b'-') {
                return true;
            }
        }
        if (b'1'..=b'9').contains(&ch(0)) && is_digit(ch(1)) && is_digit(ch(2)) && ch(3) != b'W' {
            return true;
        }
        if (b'1'..=b'9').contains(&ch(0)) && is_digit(ch(1)) && ch(2) != b'W' {
            return true;
        }
        if (b'1'..=b'9').contains(&ch(0)) && s(1, 3) != b"EL" {
            return true;
        }
        if ch(9) != b'/' && ch(10) != b'/' && is_letter(ch(11)) && is_digit(ch(12)) {
            return true;
        }
        // tredici caratteri senza uno spazio: una parola sola di 13 lettere non e' un testo libero
        if d.iter().all(|&c| c != b' ') {
            return true;
        }
        false
    } else {
        true
    }
}

/// I controlli di `chkfalse8` che non richiedono le tabelle. `book` serve a `chkflscall` (se c'e').
fn chkfalse8(c: &MessageCheck, text: &str, my_call: &str, book: Option<&CallBook>) -> bool {
    let words: Vec<&str> = text.split(' ').collect();
    let flscall = |first: &str, second: &str| -> bool { book.is_some_and(|b| b.is_false(first, second)) };

    // --- messaggi che cominciano per CQ
    if words[0] == "CQ" {
        if c.i3 == 4 {
            let rest = text.get(3..).unwrap_or("").trim_end();
            if !rest.contains('/') {
                let last = rest.bytes().last().unwrap_or(b'A');
                // spazio dentro il nominativo, oppure ultimo carattere cifra
                if rest.contains(' ') || last < b':' {
                    return true;
                }
            }
            let call = if rest.contains('/') {
                let slash = rest.find('/').unwrap_or(0);
                // il pezzo piu' lungo tra i due lati della barra
                let (left, right) = (&rest[..slash], &rest[slash + 1..]);
                if slash <= right.len() { right } else { left }
            } else {
                rest
            };
            if call_impossible(call) {
                return true;
            }
            if rest.contains('/') {
                if flscall("CQ", call) {
                    return true;
                }
            } else if call.len() >= 10 {
                if long_call_is_false(call) {
                    return true;
                }
            } else if call.len() == 6 {
                let mask: String = call.bytes().map(|ch| if is_digit(ch) { '1' } else { '0' }).collect();
                if ["001000", "101000", "011000"].contains(&mask.as_str()) {
                    return true;
                }
            }
        } else if c.ap_type == 1 && words.len() > 1 {
            let call = bare(words[1]);
            if call_impossible(call) {
                return true;
            }
        }
        if c.i3 == 4 {
            // CQ Y09Q7/SS60M: un nominativo di 11 caratteri con una barra
            let call = text.get(3..).unwrap_or("").trim_end();
            if let (Some(slash), 11) = (call.find('/'), call.len()) {
                let b = call.as_bytes();
                if call.matches('/').count() == 1 {
                    if slash < 6
                        && ((b[slash + 1] < b':' && b[slash + 2] < b':') || b[slash + 1] == b'Q' || b[call.len() - 1] < b':')
                    {
                        return true;
                    }
                    if slash > 4 && (b[slash - 1] < b':' || (b[0] < b':' && b[1] < b':') || b[0] == b'Q') {
                        return true;
                    }
                    if slash > 3 && slash < 7 && flscall(&call[..slash], &call[slash + 1..]) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // --- primo nominativo mio e secondo a caso (tipo a priori 2)
    if c.ap_type == 2 && words.len() >= 3 {
        let call = words[1];
        let call = match call.find("/R") {
            Some(i) if i > 2 => &call[..i],
            _ => call,
        };
        if call_impossible(call) {
            return true;
        }
        // il quarto campo di un messaggio con ' R ' (dopo la settima colonna) o il terzo senza: se ha quattro caratteri deve essere
        // un locatore (RR73 lo e': R e' tra A e R); un "R-12" no
        let indxr = text.find(" R ").map_or(0, |p| p + 1);
        let tail = if indxr > 7 {
            words.get(3)
        } else if indxr < 1 {
            words.get(2)
        } else {
            None
        };
        return tail.is_some_and(|w| w.len() == 4 && !looks_like_grid(w));
    }
    if matches!(c.ap_type, 3 | 11 | 21 | 40 | 41) {
        return false;
    }

    // --- messaggi con ' R ', '/R ' o '/P ' (i3 da 1 a 3): i due nominativi devono essere plausibili
    if (1..=3).contains(&c.i3) && (text.contains(" R ") || text.contains("/R ") || text.contains("/P ")) && words.len() >= 3 {
        let (a, b) = (bare(words[0]), bare(words[1]));
        if a != my_call && !words[0].starts_with("<.") && !words[1].starts_with("<.") && (call_impossible(a) || call_impossible(b)) {
            return true;
        }
        if !text.contains('/') && flscall(words[0], words[1]) && words[1] != my_call {
            return true;
        }
    }

    // --- forma standard senza R e senza barra (i3 1 o 3), o campo del Field Day: i due nominativi
    if c.ap_type != 2
        && (((c.i3 == 1 || c.i3 == 3) && !text.contains(" R ") && !text.contains('/')) || (c.i3 == 0 && c.n3 == 3))
        && !words[0].starts_with("<.")
        && words.len() >= 3
        && words[0] != my_call
    {
        let (a, b) = (words[0], words[1]);
        if call_impossible(a) || call_impossible(b) || flscall(a, b) {
            return true;
        }
    }

    // --- i3 = 4: messaggi con un hash e un nominativo lungo
    if c.i3 == 4 {
        if let Some(pos) = text.find("<...>").filter(|&p| p > 4) {
            let call = text[..pos].trim_end();
            if !call.is_empty()
                && (call.contains(' ') || (!call.contains('/') && call.bytes().last().is_some_and(|ch| ch < b':')) || call_impossible(call) || long_call_is_false(call))
            {
                return true;
            }
        }
        if let Some(rest) = text.strip_prefix("<...>") {
            let rest = rest.trim();
            let call = ["RR73", "RRR", "73"].iter().find_map(|t| rest.strip_suffix(t)).unwrap_or(rest).trim_end();
            if !call.is_empty()
                && (call.contains(' ')
                    || (!call.contains('/') && call.bytes().last().is_some_and(|ch| ch < b':'))
                    || call.ends_with('/')
                    || call_impossible(call)
                    || long_call_is_false(call))
            {
                return true;
            }
        }
    }

    // --- RTTY Roundup: TU; call call ...
    if c.i3 == 3 && words[0] == "TU;" && words.len() >= 3 && flscall(words[1], words[2]) {
        return true;
    }

    // --- testo libero
    if c.i3 == 0 && c.n3 == 0 {
        if text.contains("/.") {
            return true;
        }
        let mut t: String = text.chars().take(13).collect();
        while t.len() < 13 {
            t.push(' ');
        }
        if free_text_is_false(&t) {
            return true;
        }
    }
    // CQ_RLXA 551626 ^J65XI
    c.i3 == 0 && (text.starts_with("CQ_") || text.contains('^'))
}

/// I controlli sul testo del messaggio decodificato (il blocco `2 if(nbadcrc.eq.0)` di `ft8b`). `true` = e' un falso, da scartare.
///
/// `my_call` e' il proprio nominativo (vuoto se ignoto); `book` l'elenco completo di nominativi, se se ne ha uno.
pub fn reject_message(c: &MessageCheck, my_call: &str, book: Option<&CallBook>) -> bool {
    let text = without_braces(c.text);
    let words: Vec<&str> = text.split(' ').collect();
    let dubious = c.qual < 0.39 || c.snr < -20.5 || c.dt_s < -0.5 || c.dt_s > 1.9 || (1..=3).contains(&c.ap_type);
    let flscall = |first: &str, second: &str| -> bool { book.is_some_and(|b| b.is_false(first, second)) };

    // `go to 4` di ft8b salta i controlli intermedi (il blocco 256)
    let mut middle = true;
    if (35..40).contains(&c.ap_type) {
        // `CALL/R DX 73`: il primo nominativo con /R deve essere conosciuto
        if words[0].strip_suffix("/R").is_some_and(|call| flscall("CQ", call)) {
            return true;
        }
    } else if dubious {
        if (4..=6).contains(&c.ap_type) {
            middle = false;
        } else if chkfalse8(c, &text, my_call, book) {
            return true;
        }
    }
    if matches!(c.ap_type, 2 | 3) {
        middle = false;
    }

    if middle {
        // Field Day (i3 = 0, n3 = 3 o 4): fuori da USA e Canada si trasmette ' DX '
        if c.i3 == 0 && (3..5).contains(&c.n3) && words.len() >= 3 && words[0].len() > 2 {
            let call_b = bare(words[1]);
            let b = call_b.as_bytes();
            if b.len() >= 2 {
                let (c0, c1) = (b[0], b[1]);
                let dx = text.contains(" DX ");
                let outside = !matches!(c0, b'A' | b'K' | b'N' | b'W' | b'V' | b'C' | b'X')
                    || (c0 == b'A' && !(b'A'..=b'L').contains(&c1))
                    || (c0 == b'C' && !(b'F'..=b'K').contains(&c1) && c1 != b'Y' && c1 != b'Z')
                    || (c0 == b'V' && !(b'A'..=b'G').contains(&c1) && !matches!(c1, b'O' | b'X' | b'Y'))
                    || (c0 == b'X' && !(b'J'..=b'O').contains(&c1));
                if outside && !dx {
                    return true;
                }
                if (c.snr < -19.0 || c.dt_s < -0.5 || c.dt_s > 1.0) && flscall(bare(words[0]), call_b) {
                    return true;
                }
            }
        }
        // `<...> CALL RRR|RR73|73`: non e' una forma trasmettibile
        if text.starts_with("<...>") {
            let last = words.last().copied().unwrap_or("");
            if words.len() == 3 && matches!(last, "RRR" | "RR73" | "73") {
                return true;
            }
        }
    }

    // un messaggio i3 = 2 e' un /P: senza /P e' un falso
    if c.i3 == 2 && !text.contains("/P ") {
        return true;
    }
    // AP "MYCALL ??? ???" con un nominativo nascosto e un locatore: falso
    if let (2, Some(p)) = (c.ap_type, text.find("<...>")) {
        let tail = text.get(p + 6..).unwrap_or("").as_bytes();
        if p > 4 && p < 13 && tail.len() >= 2 && tail[0] > b'@' && tail[1] > b'@' {
            return true;
        }
    }
    c.ap_type == 1 && c.text.starts_with("CQ DE AA00")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check<'a>(text: &'a str, i3: u8, n3: u8, ap: u8) -> MessageCheck<'a> {
        MessageCheck { text, i3, n3, ap_type: ap, qual: 0.2, snr: -22.0, dt_s: 0.2 }
    }

    #[test]
    fn i_nominativi_impossibili() {
        for c in ["Q1ABC", "0AB", "12ABC", "ABC1D", "9AB1C"] {
            assert!(call_impossible(c), "{c}");
        }
        for c in ["K1ABC", "IU3ABC", "3DA0XY", "9A1ABC", "JA1ABC", "4X1AB", "A61AB"] {
            assert!(!call_impossible(c), "{c}");
        }
    }

    #[test]
    fn i_testi_liberi_di_rumore_si_scartano_e_quelli_veri_passano() {
        for t in ["TNX 73", "CIAO 73 GL", "TU 73", "QSL TNX", "73 DE IU3ABC"] {
            assert!(!free_text_is_false(&format!("{t:<13}")), "{t}");
        }
        for t in ["XQJ.4Z/PW.K-", "ZZZZZZZZZZZZZ", "J.ZM1R-D1Q89", "?AB?CD EFGH+"] {
            assert!(free_text_is_false(&format!("{t:<13}")), "{t}");
        }
    }

    #[test]
    fn le_forme_lunghe_dei_nominativi() {
        assert!(!long_call_is_false("DL2019ABCDE"));
        assert!(long_call_is_false("A1BCDEFGHIJ"));
        assert!(!long_call_is_false("K1ABC"));
        assert!(!long_call_is_false("PJ4/K1ABC1"));
    }

    #[test]
    fn un_cq_con_nominativo_di_sei_caratteri_standard_nel_tipo_4_e_un_falso() {
        let c = check("CQ VK6ZRW", 4, 1, 0);
        assert!(reject_message(&c, "", None));
        let ok = check("CQ K1ABC FN42", 1, 0, 1);
        assert!(!reject_message(&ok, "", None));
    }

    #[test]
    fn hash_nascosto_con_73_o_rr73_e_un_falso_e_i3_2_senza_p_anche() {
        assert!(reject_message(&check("<...> K1ABC RR73", 4, 0, 0), "", None));
        assert!(reject_message(&check("K1ABC W9XYZ FN42", 2, 0, 0), "", None));
        assert!(!reject_message(&check("K1ABC/P W9XYZ/P FN42", 2, 0, 0), "", None));
    }

    #[test]
    fn con_un_elenco_di_nominativi_un_messaggio_dubbio_senza_stazioni_note_si_scarta() {
        let book = CallBook::new(["K1ABC".to_string(), "W9XYZ".to_string()]);
        let c = MessageCheck { text: "XE1ZZZ YB2YYY -12", ..check("", 1, 0, 0) };
        assert!(reject_message(&c, "", Some(&book)));
        let known = MessageCheck { text: "K1ABC YB2YYY -12", ..check("", 1, 0, 0) };
        assert!(!reject_message(&known, "", Some(&book)));
        assert!(!reject_message(&c, "", None));
    }

    #[test]
    fn la_soglia_sulla_distanza_e_sui_bit_ribaltati() {
        assert!(accept_decode(10, 20.0, false));
        assert!(!accept_decode(30, 31.0, false));
        assert!(!accept_decode(40, 5.0, true));
        assert!(accept_decode(40, 5.0, false));
    }

    #[test]
    fn lo_snr_ha_i_pavimenti_di_jtdx() {
        // tutti i toni uguali: nessun segnale -> pavimento
        let s8 = [[1.0f32; 8]; NN];
        let tones = [0u8; NN];
        assert!((jtdx_snr(&s8, &tones, false) + 23.0).abs() < 1e-3);
        assert!((jtdx_snr(&s8, &tones, true) + 24.0).abs() < 1e-3);
    }

    /// Ampiezze dei toni di un segnale ideale: il tono trasmesso a `level`, gli altri a 1.
    fn ideal_s8(level: f32) -> [[f32; 8]; NN] {
        let mut s8 = [[1.0f32; 8]; NN];
        for start in [0usize, 36, 72] {
            for (k, &t) in COSTAS.iter().enumerate() {
                s8[start + k][usize::from(t)] = level;
            }
        }
        s8
    }

    /// Ampiezze di puro rumore (Rayleigh): la radice della somma di due gaussiane al quadrato.
    fn noise_s8(seed: u64) -> [[f32; 8]; NN] {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut next = || {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut s8 = [[0.0f32; 8]; NN];
        for row in s8.iter_mut() {
            for v in row.iter_mut() {
                *v = (-2.0 * (1.0 - next()).max(1e-12).ln()).sqrt() as f32;
            }
        }
        s8
    }

    #[test]
    fn un_segnale_chiaro_supera_i_controlli_di_sincronismo_a_ogni_ritardo() {
        let s8 = ideal_s8(8.0);
        for dt in [0.3f32, -0.8, 2.5] {
            let s = sync_screen(&s8, dt, false, true);
            assert!(s.pass && !s.cq_only, "dt {dt}: {s:?}");
        }
    }

    #[test]
    fn il_rumore_quasi_mai_supera_i_controlli_di_sincronismo() {
        let passed = (0..400u64).filter(|&seed| sync_screen(&noise_s8(seed), 0.2, false, true).pass).count();
        let passed_basic = (0..400u64).filter(|&seed| sync_screen(&noise_s8(seed), 0.2, false, false).pass).count();
        eprintln!("rumore: passa i controlli completi {passed}/400, solo nsync >= 7 {passed_basic}/400");
        assert!(passed <= 1, "{passed}");
        assert!(passed <= passed_basic);
    }
}
