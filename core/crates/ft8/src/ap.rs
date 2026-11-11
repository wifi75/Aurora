//! Analisi a priori (AP) come `ft8apset.f90`, `ft8_mod1.f90` (tabella `naptypes`) e la parte AP di `ft8b.f90` di JTDX 2.2.159.
//!
//! Se del messaggio si conoscono alcuni bit (CQ, il proprio nominativo, il nominativo del DX in QSO, il suo locatore, RRR/RR73/73) li si
//! mettono nel decoder con un LLR enorme e li si dichiarano fissi (`apmask`): la propagazione di credenze e OSD cercano allora solo
//! tra le parole che li rispettano, e la soglia scende di qualche dB. Il prezzo e' il rischio di "trovare" il messaggio atteso nel
//! rumore, per questo ogni tipo ha le sue condizioni (toni attesi nei simboli noti, rapporto segnale/rumore, stato del QSO) e il
//! messaggio trovato passa poi per i filtri di `filters`.
//!
//! # Tipi (iaptype) portati, per nominativi standard
//!
//! | tipo | messaggio                         | bit noti |
//! |------|-----------------------------------|----------|
//! | 1    | `CQ ??? ???`                      | 29 + 3   |
//! | 2    | `MYCALL ??? ???`                  | 29 + 3   |
//! | 3    | `MYCALL DXCALL ???`               | 58 + 3   |
//! | 4    | `MYCALL DXCALL RRR`               | 77       |
//! | 5    | `MYCALL DXCALL 73`                | 77       |
//! | 6    | `MYCALL DXCALL RR73`              | 77       |
//! | 31   | `CQ DXCALL GRID` (ricerca del DX) | 77 (o 58 + 3 senza locatore) |
//! | 35   | `??? DXCALL 73`                   | 29 + 19  |
//! | 36   | `??? DXCALL RR73`                 | 29 + 19  |
//!
//! L'ordine con cui i tipi si provano dipende dall'avanzamento del QSO (`nQSOProgress`, tabella [`NAPTYPES`]).
//!
//! # Tipi per nominativi non standard (WSJT-X 3.3.0-beta6, `ft8var/ft8bvar.f90:1606-1980`, `ft8var/ft8apsetvar.f90`)
//!
//! Un nominativo e' standard come per `Radio::is_standard_callsign` (`Radio.cpp:144-155`, [`is_standard_callsign`]); se uno dei due
//! non lo e', i tipi vengono da un'altra tabella ([`ApTable`]) e i messaggi attesi portano l'altro nominativo in hash (`<...>`):
//!
//! | tipo | messaggio                         | bit noti | tabella |
//! |------|-----------------------------------|----------|---------|
//! | 11   | `MYCALL <DXCALL> ???` (i3 = 1)    | 58 + 3   | [`NDXNSAPTYPES`] (DX non standard) |
//! | 12   | `<MYCALL> DXCALL RRR` (i3 = 4)    | 77       | [`NDXNSAPTYPES`] |
//! | 13   | `<MYCALL> DXCALL 73` (i3 = 4)     | 77       | [`NDXNSAPTYPES`] |
//! | 14   | `<MYCALL> DXCALL RR73` (i3 = 4)   | 77       | [`NDXNSAPTYPES`] |
//! | 31   | `CQ DXCALL` (i3 = 4)              | 77       | [`NDXNSAPTYPES`] |
//! | 35   | `<???> DXCALL 73` (i3 = 4)        | 64 (bit 14-77) | [`NDXNSAPTYPES`] |
//! | 36   | `<???> DXCALL RR73` (i3 = 4)      | 64 (bit 14-77) | [`NDXNSAPTYPES`] |
//! | 40   | `<MYCALL> ??? ???` (i3 = 1)       | 29 + 3   | [`NMYCNSAPTYPES`] (proprio non standard) |
//! | 41   | `<MYCALL> DXCALL ???` (i3 = 1)    | 58 + 3   | [`NMYCNSAPTYPES`] |
//! | 42   | `MYCALL <DXCALL> RRR` (i3 = 4)    | 77       | [`NMYCNSAPTYPES`] |
//! | 43   | `MYCALL <DXCALL> 73` (i3 = 4)     | 77       | [`NMYCNSAPTYPES`] |
//! | 44   | `MYCALL <DXCALL> RR73` (i3 = 4)   | 77       | [`NMYCNSAPTYPES`] |
//!
//! Gli hash (12 bit nel tipo 4, 22 bit nel tipo 1) si calcolano sul nominativo completo, con la barra e il prefisso o il suffisso
//! (`IU3ABC/P`, `PJ4/K1ABC`), come fa `pack77` in WSJT-X.
//!
//! # Differenze dal Fortran
//!
//! - Non e' portata la modalita' Hound/Fox (tipi 21-24 e 111, tabella `nhaptypes` di `ft8_mod1.f90:83-95`): il motore in Hound non
//!   passa l'analisi a priori e quei tipi chiedono lo stato della Fox (`apsymsp`, `idtonefox73`, `idtonespec`, frequenze a passi di 60 Hz).
//! - Quando un messaggio atteso non si impacchetta come dovrebbe, `ft8apsetvar` riempie la maschera di zeri con due 99 (bit 1 e 30) e la prova
//!   comunque; qui quel tipo si salta.
//! - Senza il proprio nominativo si resta sulla tabella standard (in WSJT-X un nominativo vuoto non e' standard e si passerebbe a
//!   `nmycnsaptypes`): e' il caso dell'analisi senza dati (`Options::ap = None`), che deve restare il solo tipo CQ. Con il proprio vuoto e un
//!   DX non standard si usa invece la tabella dei due non standard, come in WSJT-X.
//! - Le condizioni che in JTDX vengono da `tone8.f90` (`idtone56`, `idtonemyc`) sono ricavate qui dagli stessi messaggi
//!   (`MYCALL DXCALL -01`, `... RRR`, `... RR73`, `... 73`, con l'hash dove `tone8.f90:81-129` lo mette) con `pack` e la codifica.

use crate::message::{self, Bits77, CallHashes};
use crate::{COSTAS, crc, ldpc, modem};

/// Opzioni dell'analisi a priori. Con `Options::ap = None` resta il solo tipo 1 (CQ), che non richiede di conoscere niente.
#[derive(Debug, Clone, PartialEq)]
pub struct ApOptions {
    /// Il proprio nominativo.
    pub mycall: String,
    /// Il nominativo del DX in QSO o che si sta chiamando (vuoto: nessuno).
    pub hiscall: String,
    /// Il suo locatore a 4 caratteri (vuoto: ignoto).
    pub hisgrid: String,
    /// Avanzamento del QSO (`nQSOProgress`): 0 CQ, 1 prima chiamata (locatore), 2 rapporto, 3 rapporto con R, 4 RRR/RR73, 5 73.
    pub qso_progress: u8,
    /// Frequenza di ricezione scelta (`nfqso`, Hz), se c'e'.
    pub rx_freq_hz: Option<f32>,
    /// Frequenza di trasmissione (`nftx`, Hz), se c'e'.
    pub tx_freq_hz: Option<f32>,
    /// Larghezza attorno alle due frequenze entro cui valgono i tipi di QSO (`napwid`, Hz).
    pub width_hz: f32,
    /// `lapmyc`: i tipi che usano il proprio nominativo sono attivi (JTDX li spegne da 2-3 minuti dopo l'ultima trasmissione, perche'
    /// in ricezione a vuoto un messaggio "per me" trovato nel rumore e' quasi sempre un falso).
    pub my_call_active: bool,
    /// `stophint`: non si sta trasmettendo (ricezione libera). Allora si cercano CQ, 73 e RR73 del DX (tipi 31, 35, 36) e non i tipi 3-6.
    pub idle: bool,
    /// `lenabledxcsearch`: ricerca del DX (tipi 31, 35, 36) permessa.
    pub dx_search: bool,
    /// `lwidedxcsearch`: la ricerca del DX vale su tutta la banda e non solo attorno alla frequenza di ricezione.
    pub wide_dx_search: bool,
}

impl ApOptions {
    /// Opzioni per ricezione libera con il proprio nominativo (`my_call_active` acceso, nessun DX, ricerca del DX spenta).
    pub fn new(mycall: &str) -> ApOptions {
        ApOptions {
            mycall: mycall.trim().to_uppercase(),
            hiscall: String::new(),
            hisgrid: String::new(),
            qso_progress: 0,
            rx_freq_hz: None,
            tx_freq_hz: None,
            width_hz: 50.0,
            my_call_active: true,
            idle: true,
            dx_search: false,
            wide_dx_search: false,
        }
    }
}

/// `naptypes(nQSOProgress, 1:27)` di `ft8_mod1.f90`: per ognuno dei 27 posti dopo le quattro letture normali (`isubp2` da 5 a 31) il
/// tipo da provare, 0 = saltare.
pub const NAPTYPES: [[u8; 27]; 6] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 6, 6, 6, 5, 5, 5, 4, 4, 4, 0, 0, 0, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 6, 6, 6, 5, 5, 5, 4, 4, 4, 2, 2, 2, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
];

/// `ndxnsaptypes(nQSOProgress, 1:27)` di `ft8_mod1.f90:97-109`: nominativo del DX non standard (il proprio standard o vuoto).
pub const NDXNSAPTYPES: [[u8; 27]; 6] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [11, 11, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [11, 11, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [11, 11, 11, 14, 14, 14, 13, 13, 13, 12, 12, 12, 0, 0, 0, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [11, 11, 11, 14, 14, 14, 13, 13, 13, 12, 12, 12, 2, 2, 2, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
];

/// `nmycnsaptypes(nQSOProgress, 1:27)` di `ft8_mod1.f90:69-81`: il proprio nominativo non standard, quello del DX standard o vuoto.
pub const NMYCNSAPTYPES: [[u8; 27]; 6] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 40, 40, 40, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [41, 41, 41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [41, 41, 41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [41, 41, 41, 44, 44, 44, 43, 43, 43, 42, 42, 42, 0, 0, 0, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [41, 41, 41, 44, 44, 44, 43, 43, 43, 42, 42, 42, 40, 40, 40, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 40, 40, 40, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
];

/// Quale ramo AP di `ft8bvar.f90` vale per questi due nominativi (fuori da Hound).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApTable {
    /// Proprio standard, DX standard o vuoto: [`NAPTYPES`] (`ft8bvar.f90:1424`).
    Standard,
    /// Proprio standard, DX non standard: [`NDXNSAPTYPES`] (`ft8bvar.f90:1606-1764`).
    DxNonStandard,
    /// Entrambi non standard (o il proprio vuoto) con il DX non standard: [`NDXNSAPTYPES`], ma solo CQ e la ricerca del DX
    /// (`ft8bvar.f90:1766-1847`).
    BothNonStandard,
    /// Proprio non standard, DX standard o vuoto: [`NMYCNSAPTYPES`] (`ft8bvar.f90:1849-1980`).
    MyNonStandard,
}

/// `Radio::is_standard_callsign` di WSJT-X (`Radio.cpp:144-155`), che decide `lmycallstd` e `lhiscallstd`
/// (`mainwindow.cpp:5546-5549`): `( [A-Z]{0,2} | [A-Z][0-9] | [0-9][A-Z] ) ( [0-9][A-Z]{0,3} ) (/R | /P)?`, maiuscole o minuscole.
pub fn is_standard_callsign(call: &str) -> bool {
    let up = call.trim().to_ascii_uppercase();
    let body = up.strip_suffix("/R").or_else(|| up.strip_suffix("/P")).unwrap_or(&up);
    let b = body.as_bytes();
    let part2 = |p: &[u8]| !p.is_empty() && p.len() <= 4 && p[0].is_ascii_digit() && p[1..].iter().all(u8::is_ascii_uppercase);
    let letters = (0..=2usize.min(b.len())).any(|n| b[..n].iter().all(u8::is_ascii_uppercase) && part2(&b[n..]));
    let mixed = b.len() >= 2
        && ((b[0].is_ascii_uppercase() && b[1].is_ascii_digit()) || (b[0].is_ascii_digit() && b[1].is_ascii_uppercase()))
        && part2(&b[2..]);
    letters || mixed
}

/// Gli ultimi 19 bit di RRR, 73 e RR73 (`mrrr`, `m73`, `mrr73` di `ft8_mod1.f90`): bit 59 (R), locatore/rapporto a 15 bit, `i3`.
const MRRR: [i8; 19] = [-1, 1, 1, 1, 1, 1, 1, -1, 1, -1, -1, 1, -1, -1, 1, -1, -1, -1, 1];
const M73: [i8; 19] = [-1, 1, 1, 1, 1, 1, 1, -1, 1, -1, -1, 1, -1, 1, -1, -1, -1, -1, 1];
const MRR73: [i8; 19] = [-1, 1, 1, 1, 1, 1, 1, -1, -1, 1, 1, 1, -1, 1, -1, 1, -1, -1, 1];

/// Dall'indice della lettura (`isubp2`, 5..=31) e dal tipo, quale serie di LLR usare: 0 = `llra`, 1 = `llrb`, 2 = `llrc`.
/// Ogni tipo prova prima `llrc`, poi `llrb`, poi `llra` (i tipi 4-6 usano `llrb` due volte, come nel Fortran).
pub fn llr_index(isubp2: usize, aptype: u8) -> usize {
    match (isubp2 - 5) % 3 {
        0 => 2,
        1 => 1,
        _ if (4..=6).contains(&aptype) => 1,
        _ => 0,
    }
}

/// I 58 toni di dati (`idtone`) di un messaggio: i simboli 8-36 e 44-72 dei 79.
pub fn data_tones(tones: &[u8; 79]) -> [u8; 58] {
    let mut out = [0u8; 58];
    out[..29].copy_from_slice(&tones[7..36]);
    out[29..].copy_from_slice(&tones[43..72]);
    out
}

/// Dai bit del messaggio ai 79 toni (CRC, codice LDPC, mappa Gray e Costas).
fn tones_of_bits(bits: &Bits77) -> [u8; 79] {
    modem::tones_from_codeword(&ldpc::encode(&crc::append_crc(bits)))
}

/// Impacchetta `text` e verifica che si rilegga uguale (come `ft8apset`), senza toccare le tabelle degli hash di chi decodifica.
fn bits_of(text: &str) -> Option<Bits77> {
    let mut scratch = CallHashes::default();
    let bits = message::pack(text, &mut scratch).ok()?;
    let back = message::unpack(&bits, &mut scratch).ok()?;
    (back == text).then_some(bits)
}

/// Impacchetta `text` senza rileggerlo (come `genft8var` in `tone8.f90`): servono solo i toni.
fn packed(text: &str) -> Option<Bits77> {
    message::pack(text, &mut CallHashes::default()).ok()
}

/// Impacchetta `text` e chiede solo che si rilegga (`unpk77_successvar`), senza confrontare il testo: `ft8apsetvar.f90:89-103`.
fn bits_readable(text: &str) -> Option<Bits77> {
    let mut scratch = CallHashes::default();
    let bits = message::pack(text, &mut scratch).ok()?;
    message::unpack(&bits, &mut scratch).ok().map(|_| bits)
}

/// I bit di `text` come segni (+1/-1) se il messaggio si impacchetta nel tipo `i3` e si rilegge uguale (o solo si rilegge, con
/// `exact` spento); i primi `n` bit.
fn ap_signs(text: &str, i3: u8, n: usize, exact: bool) -> Option<Vec<i8>> {
    let bits = if exact { bits_of(text)? } else { bits_readable(text)? };
    (message::message_type(&bits).0 == i3).then(|| signs(&bits[..n]))
}

/// I 58 toni di dati di `text` (`idtone*` di `tone8.f90`), se si impacchetta.
fn data_tones_of(text: &str) -> Option<[u8; 58]> {
    packed(text).map(|bits| data_tones(&tones_of_bits(&bits)))
}

/// Un nominativo vale come standard se `CQ nominativo` si impacchetta nel tipo 1 (o 2, con /P).
fn is_standard(call: &str) -> bool {
    bits_of(&format!("CQ {call}")).is_some_and(|b| matches!(message::message_type(&b).0, 1 | 2))
}

fn signs(bits: &[u8]) -> Vec<i8> {
    bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).collect()
}

/// Tutto cio' che serve a provare i tipi AP, calcolato una volta per periodo dalle opzioni.
#[derive(Debug, Clone)]
pub struct ApContext {
    pub opts: ApOptions,
    /// Il proprio nominativo e' standard e c'e' (tipi 2-6 possibili).
    pub my_ok: bool,
    /// Il nominativo del DX e' standard e c'e' (tipi 3-6, 31, 35, 36 possibili).
    pub his_ok: bool,
    /// Il locatore del DX e' noto (quattro caratteri validi).
    pub his_grid_known: bool,
    /// `CQ` + 3 bit di tipo: i bit 1-29 e 75-77 del messaggio `CQ ...`.
    mcq: [i8; 29],
    /// I primi 58 bit di `MYCALL DXCALL RRR` (con il proprio nominativo al posto del DX se manca).
    apsym: [i8; 58],
    /// I 77 bit di `CQ DXCALL GRID` (se il locatore e' ignoto contano solo i primi 58 e i tre di tipo).
    apcq: [i8; 77],
    /// Toni di dati attesi per `CQ ...` nei primi nove simboli (`idtone25`): otto volte il tono 0 e poi il tono 1.
    pub cq_tones: [u8; 9],
    /// Toni di dati attesi per i primi nove simboli del proprio nominativo (`idtonemyc`).
    pub my_tones: [u8; 9],
    /// `idtone56(1,:)`: `MYCALL DXCALL -01`, i primi 25 simboli di dati (i 58 bit di nominativi e l'inizio del rapporto).
    pub qso_tones: [u8; 25],
    /// Toni di dati completi di `MYCALL DXCALL 73`, `RR73` e `RRR` (`idtone56(56|55|54,:)`).
    pub tones_73: [u8; 58],
    pub tones_rr73: [u8; 58],
    pub tones_rrr: [u8; 58],
    /// Il ramo di `ft8bvar.f90` (e la tabella dei tipi) per questi due nominativi.
    pub table: ApTable,
    /// `lmycallstd`, `lhiscallstd`: [`is_standard_callsign`] sui due nominativi.
    pub my_std: bool,
    pub his_std: bool,
    /// `idtone56_valid(1)`: [`Self::qso_tones`] e i toni di 73, RR73 e RRR ci sono (entrambi standard, o uno solo non standard con il
    /// messaggio in hash di `tone8.f90:81-129`).
    pub qso_tones_ok: bool,
    /// `idtonecqdxcns` e `idtonedxcns73` di `tone8.f90:51-62` (DX non standard): toni di `CQ DXCALL` e di `<AA1AAA> DXCALL 73`.
    pub dxns_cq_tones: Option<[u8; 58]>,
    pub dxns_73_tones: Option<[u8; 58]>,
    /// Le maschere dei tipi per nominativi non standard (`apsym*` di `ft8apsetvar.f90`).
    ns: NsSymbols,
}

/// I bit noti (come segni) dei messaggi con un nominativo non standard, da `ft8apsetvar.f90`. `None` = non si impacchetta come deve.
#[derive(Debug, Clone, Default)]
struct NsSymbols {
    /// `apcqsym` con il DX non standard: `CQ DXCALL` (i3 = 4), 77 bit (`ft8apsetvar.f90:63-74`).
    cq_dx: Option<Vec<i8>>,
    /// `apsymdxnsrr73`, `apsymdxns73`: `<W9XYZ> DXCALL RR73|73` (i3 = 4), 77 bit (`ft8apsetvar.f90:87-104`).
    dx_rr73: Option<Vec<i8>>,
    dx_73: Option<Vec<i8>>,
    /// `apsymdxns1`: `MYCALL <DXCALL> -16` (i3 = 1), 58 bit (`ft8apsetvar.f90:186-193`).
    dx1: Option<Vec<i8>>,
    /// `apsymdxnsrrr`, `apsymdxnsr73`, `apsymdxns732`: `<MYCALL> DXCALL RRR|RR73|73` (i3 = 4), 77 bit (`ft8apsetvar.f90:195-217`).
    dx_rrr_mine: Option<Vec<i8>>,
    dx_rr73_mine: Option<Vec<i8>>,
    dx_73_mine: Option<Vec<i8>>,
    /// `apsymmyns1`: `<MYCALL> ZZ1ZZZ -15` (i3 = 1), 29 bit, solo senza DX (`ft8apsetvar.f90:136-144`).
    my1: Option<Vec<i8>>,
    /// `apsymmyns2`: `<MYCALL> DXCALL -15` (i3 = 1), 58 bit (`ft8apsetvar.f90:108-114`).
    my2: Option<Vec<i8>>,
    /// `apsymmynsrrr`, `apsymmynsrr73`, `apsymmyns73`: `MYCALL <DXCALL> RRR|RR73|73` (i3 = 4), 77 bit (`ft8apsetvar.f90:116-134`).
    my_rrr: Option<Vec<i8>>,
    my_rr73: Option<Vec<i8>>,
    my_73: Option<Vec<i8>>,
    /// `apsymdxstd`: `DXCALL DXCALL RRR` (i3 = 1), 58 bit, con il proprio non standard (`ft8apsetvar.f90:76-85`).
    dx_std: Option<Vec<i8>>,
}

impl ApContext {
    pub fn new(opts: &ApOptions) -> ApContext {
        let mut ctx = ApContext {
            opts: opts.clone(),
            my_ok: false,
            his_ok: false,
            his_grid_known: false,
            mcq: [-1; 29],
            apsym: [0; 58],
            apcq: [0; 77],
            cq_tones: [0, 0, 0, 0, 0, 0, 0, 0, 1],
            my_tones: [0; 9],
            qso_tones: [0; 25],
            tones_73: [0; 58],
            tones_rr73: [0; 58],
            tones_rrr: [0; 58],
            table: ApTable::Standard,
            my_std: false,
            his_std: false,
            qso_tones_ok: false,
            dxns_cq_tones: None,
            dxns_73_tones: None,
            ns: NsSymbols::default(),
        };
        // CQ: n28 = 2, quindi tutti zero tranne il bit 27 (da 1)
        ctx.mcq[26] = 1;
        let my_up = opts.mycall.trim().to_uppercase();
        let his_up = opts.hiscall.trim().to_uppercase();
        let (my, his) = (my_up.as_str(), his_up.as_str());
        ctx.my_ok = my.len() >= 3 && is_standard(my);
        ctx.his_ok = his.len() >= 3 && is_standard(his);
        // i 58 bit "primo secondo": servono anche senza il proprio nominativo (per i tipi 35 e 36 contano solo quelli del DX)
        let first = if ctx.my_ok { my } else { "K1ABC" };
        let second = if ctx.his_ok { his } else { first };
        match bits_of(&format!("{first} {second} RRR")) {
            Some(bits) if message::message_type(&bits).0 == 1 => {
                ctx.apsym.copy_from_slice(&signs(&bits[..58]));
                let tones = data_tones(&tones_of_bits(&bits));
                ctx.my_tones.copy_from_slice(&tones[..9]);
            }
            _ => {
                ctx.my_ok = false;
                ctx.his_ok = false;
            }
        }
        if ctx.my_ok && ctx.his_ok {
            if let Some(bits) = bits_of(&format!("{my} {his} -01")) {
                ctx.qso_tones.copy_from_slice(&data_tones(&tones_of_bits(&bits))[..25]);
            }
            for (text, dst) in [("73", &mut ctx.tones_73), ("RR73", &mut ctx.tones_rr73), ("RRR", &mut ctx.tones_rrr)] {
                if let Some(bits) = bits_of(&format!("{my} {his} {text}")) {
                    *dst = data_tones(&tones_of_bits(&bits));
                }
            }
        }
        if ctx.his_ok {
            let grid = opts.hisgrid.trim();
            let with_grid = (grid.len() == 4).then(|| bits_of(&format!("CQ {his} {grid}"))).flatten();
            if let Some(bits) = with_grid {
                ctx.apcq.copy_from_slice(&signs(&bits));
                ctx.his_grid_known = true;
            } else if let Some(bits) = bits_of(&format!("CQ {his}")) {
                // senza locatore: i bit del nominativo e il tipo (CQ + nominativo + 'nessun locatore')
                ctx.apcq.copy_from_slice(&signs(&bits));
            } else {
                ctx.his_ok = false;
            }
        }
        if ctx.my_ok && ctx.his_ok {
            ctx.qso_tones_ok = true;
        }
        ctx.set_non_standard(my, his);
        ctx
    }

    /// Ramo, maschere e toni per i nominativi non standard (`ft8apsetvar.f90`, `tone8.f90`), sui nominativi completi.
    fn set_non_standard(&mut self, my: &str, his: &str) {
        let my_given = my.len() >= 3;
        let his_given = his.len() >= 3;
        self.my_std = is_standard_callsign(my);
        self.his_std = is_standard_callsign(his);
        // `ft8bvar.f90:1424`, `:1606`, `:1766`, `:1849` (senza il proprio nominativo vedi le differenze in testa al modulo)
        self.table = if !my_given {
            if his_given && !self.his_std { ApTable::BothNonStandard } else { ApTable::Standard }
        } else if self.my_std {
            if his_given && !self.his_std { ApTable::DxNonStandard } else { ApTable::Standard }
        } else if his_given && !self.his_std {
            ApTable::BothNonStandard
        } else {
            ApTable::MyNonStandard
        };
        let ns = &mut self.ns;
        if his_given && !self.his_std {
            // `ft8apsetvar.f90:63-74`: `CQ DXCALL` deve essere di tipo 4 e rileggersi uguale
            ns.cq_dx = ap_signs(&format!("CQ {his}"), 4, 77, true);
            // `ft8apsetvar.f90:87-104`: ricerca del DX non standard, basta che si rilegga
            ns.dx_rr73 = ap_signs(&format!("<W9XYZ> {his} RR73"), 4, 77, false);
            ns.dx_73 = ap_signs(&format!("<W9XYZ> {his} 73"), 4, 77, false);
            // `tone8.f90:51-62`
            self.dxns_cq_tones = data_tones_of(&format!("CQ {his}"));
            self.dxns_73_tones = data_tones_of(&format!("<AA1AAA> {his} 73"));
        }
        if self.his_std && his_given && !self.my_std {
            // `ft8apsetvar.f90:76-85`
            ns.dx_std = ap_signs(&format!("{his} {his} RRR"), 1, 58, true);
        }
        if my_given && !self.my_std {
            if self.his_std && his_given {
                // `ft8apsetvar.f90:108-134`
                ns.my2 = ap_signs(&format!("<{my}> {his} -15"), 1, 58, true);
                ns.my_rr73 = ap_signs(&format!("{my} <{his}> RR73"), 4, 77, true);
                ns.my_73 = ap_signs(&format!("{my} <{his}> 73"), 4, 77, true);
                ns.my_rrr = ap_signs(&format!("{my} <{his}> RRR"), 4, 77, true);
            } else if !his_given {
                // `ft8apsetvar.f90:136-144`
                ns.my1 = ap_signs(&format!("<{my}> ZZ1ZZZ -15"), 1, 29, true);
            }
        }
        if my_given && self.my_std && his_given && !self.his_std {
            // `ft8apsetvar.f90:186-217`
            ns.dx1 = ap_signs(&format!("{my} <{his}> -16"), 1, 58, true);
            ns.dx_rrr_mine = ap_signs(&format!("<{my}> {his} RRR"), 4, 77, true);
            ns.dx_rr73_mine = ap_signs(&format!("<{my}> {his} RR73"), 4, 77, true);
            ns.dx_73_mine = ap_signs(&format!("<{my}> {his} 73"), 4, 77, true);
        }
        // `tone8.f90:81-129`: con uno solo dei due non standard i messaggi del QSO portano l'hash dell'uno o dell'altro
        if my_given && his_given && self.my_std != self.his_std {
            let (report, ending) = if self.my_std {
                (format!("{my} <{his}> -01"), format!("<{my}> {his}"))
            } else {
                (format!("<{my}> {his} -01"), format!("{my} <{his}>"))
            };
            if let Some(t) = data_tones_of(&report) {
                self.qso_tones.copy_from_slice(&t[..25]);
                self.qso_tones_ok = true;
                for (text, dst) in [("73", &mut self.tones_73), ("RR73", &mut self.tones_rr73), ("RRR", &mut self.tones_rrr)] {
                    // un messaggio che non si impacchetta lascia la casella non valida (`idtone56_valid`): il tono 8 non torna mai
                    *dst = data_tones_of(&format!("{ending} {text}")).unwrap_or([8; 58]);
                }
            }
        }
    }

    /// La tabella dei tipi da provare (`naptypes`, `ndxnsaptypes` o `nmycnsaptypes`) per il ramo di questi nominativi.
    pub fn aptypes(&self) -> &'static [[u8; 27]; 6] {
        match self.table {
            ApTable::Standard => &NAPTYPES,
            ApTable::DxNonStandard | ApTable::BothNonStandard => &NDXNSAPTYPES,
            ApTable::MyNonStandard => &NMYCNSAPTYPES,
        }
    }

    /// Maschera e LLR a priori del tipo `aptype` sopra `llr` (che e' la serie scelta), con `apmag` come ampiezza dei bit noti.
    /// `None` se il tipo non si puo' costruire con questi dati.
    pub fn apply(&self, aptype: u8, llr: &[f32; 174], apmag: f32) -> Option<([f32; 174], [bool; 174])> {
        let mut z = *llr;
        let mut mask = [false; 174];
        let mut set = |from: usize, signs: &[i8], z: &mut [f32; 174]| {
            for (i, &s) in signs.iter().enumerate() {
                z[from + i] = apmag * f32::from(s);
                mask[from + i] = true;
            }
        };
        // i tre bit di tipo di un messaggio standard (i3 = 1: 001)
        let type_bits = [-1i8, -1, 1];
        let ns = &self.ns;
        let dx_ns = matches!(self.table, ApTable::DxNonStandard | ApTable::BothNonStandard);
        match (aptype, self.table) {
            // `ft8bvar.f90:1691-1733`: proprio standard, DX non standard
            (11, ApTable::DxNonStandard) => {
                set(0, ns.dx1.as_deref()?, &mut z);
                set(74, &type_bits, &mut z);
                return Some((z, mask));
            }
            (12..=14, ApTable::DxNonStandard) => {
                let s = match aptype {
                    12 => &ns.dx_rrr_mine,
                    13 => &ns.dx_73_mine,
                    _ => &ns.dx_rr73_mine,
                };
                set(0, s.as_deref()?, &mut z);
                return Some((z, mask));
            }
            // `ft8bvar.f90:1734-1763`, `:1817-1846`: `CQ DXCALL` intero, `<???> DXCALL 73|RR73` dal bit 14
            (31, _) if dx_ns => {
                set(0, ns.cq_dx.as_deref()?, &mut z);
                return Some((z, mask));
            }
            (35 | 36, _) if dx_ns => {
                let s = if aptype == 35 { &ns.dx_73 } else { &ns.dx_rr73 };
                set(13, &s.as_deref()?[13..], &mut z);
                return Some((z, mask));
            }
            // `ft8bvar.f90:1900-1977`: proprio non standard, DX standard
            (35 | 36, ApTable::MyNonStandard) => {
                set(29, &ns.dx_std.as_deref()?[29..], &mut z);
                set(58, if aptype == 35 { &M73 } else { &MRR73 }, &mut z);
                return Some((z, mask));
            }
            (40, ApTable::MyNonStandard) => {
                set(0, ns.my1.as_deref()?, &mut z);
                set(74, &type_bits, &mut z);
                return Some((z, mask));
            }
            (41, ApTable::MyNonStandard) => {
                set(0, ns.my2.as_deref()?, &mut z);
                set(74, &type_bits, &mut z);
                return Some((z, mask));
            }
            (42..=44, ApTable::MyNonStandard) => {
                let s = match aptype {
                    42 => &ns.my_rrr,
                    43 => &ns.my_73,
                    _ => &ns.my_rr73,
                };
                set(0, s.as_deref()?, &mut z);
                return Some((z, mask));
            }
            (11..=14 | 40..=44, _) => return None,
            _ => {}
        }
        match aptype {
            1 => {
                set(0, &self.mcq, &mut z);
                set(74, &type_bits, &mut z);
            }
            2 if self.my_ok => {
                set(0, &self.apsym[..29], &mut z);
                set(74, &type_bits, &mut z);
            }
            3 if self.my_ok && self.his_ok => {
                set(0, &self.apsym, &mut z);
                set(74, &type_bits, &mut z);
            }
            4..=6 if self.my_ok && self.his_ok => {
                set(0, &self.apsym, &mut z);
                let tail: &[i8; 19] = match aptype {
                    4 => &MRRR,
                    5 => &M73,
                    _ => &MRR73,
                };
                set(58, tail, &mut z);
            }
            31 if self.his_ok => {
                if self.his_grid_known {
                    set(0, &self.apcq, &mut z);
                } else {
                    set(0, &self.apcq[..58], &mut z);
                    set(74, &type_bits, &mut z);
                }
            }
            35 | 36 if self.his_ok => {
                set(29, &self.apsym[29..], &mut z);
                set(58, if aptype == 35 { &M73 } else { &MRR73 }, &mut z);
            }
            _ => return None,
        }
        Some((z, mask))
    }
}

/// Numero di simboli (su `expected`, a partire dal primo di dati) il cui tono piu' forte in `s8` e' quello atteso.
/// Il simbolo di dati `i` (da 0) sta nella colonna `i + 7` per i primi 29 e `i + 14` per gli altri.
pub fn tone_matches(s8: &[[f32; 8]; 79], expected: &[u8], from_data_symbol: usize) -> usize {
    let mut n = 0;
    for (k, &tone) in expected.iter().enumerate() {
        let i = from_data_symbol + k;
        let col = if i < 29 { i + 7 } else { i + 14 };
        if col < 79 && argmax8(&s8[col]) == usize::from(tone) {
            n += 1;
        }
    }
    n
}

/// Il tono piu' forte di un simbolo (il primo in caso di parita', come `maxloc`).
pub fn argmax8(v: &[f32; 8]) -> usize {
    let mut best = 0;
    for (i, &x) in v.iter().enumerate() {
        if x > v[best] {
            best = i;
        }
    }
    best
}

/// Rapporto tra l'energia dei toni attesi nei primi nove simboli di dati e la media degli altri sette (`scqnr`, `smycnr`).
pub fn expected_ratio(s8: &[[f32; 8]; 79], expected: &[u8; 9]) -> f32 {
    let mut level = 0.0f32;
    for (i, &t) in expected.iter().enumerate() {
        level += s8[i + 7][usize::from(t)];
    }
    let total: f32 = (7..16).map(|k| s8[k].iter().sum::<f32>()).sum();
    let mut noise = (total - level) / 7.0;
    if noise == 0.0 {
        noise = f32::MIN_POSITIVE;
    }
    level / noise
}

/// Punteggio "CQ" di `ft8b` (`rscq`): nei simboli 8-16 un CQ ha il tono 0 (e il tono 1 all'ultimo), e qualche conferma sugli altri.
pub fn cq_score(s8: &[[f32; 8]; 79]) -> f32 {
    let mut rscq = 0.0f32;
    for (k, row) in s8.iter().enumerate().take(16).skip(7) {
        let ip = argmax8(row);
        if (k < 15 && ip == 0) || (k == 15 && ip == 1) {
            rscq += 1.0;
        }
    }
    let ip = argmax8(&s8[16]);
    if ip == 0 || ip == 1 {
        rscq += 0.5;
    }
    let ip = argmax8(&s8[26]);
    if ip == 0 || ip == 1 {
        rscq += 0.5;
    }
    let ip = argmax8(&s8[32]);
    if ip == 2 || ip == 3 {
        rscq += 0.5;
    }
    rscq
}

/// Il tono atteso nel simbolo di Costas `k` di un blocco (serve ai controlli di sincronismo).
pub fn costas_tone(k: usize) -> usize {
    usize::from(COSTAS[k])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tones_for(text: &str) -> [u8; 79] {
        tones_of_bits(&bits_of(text).expect("messaggio valido"))
    }

    #[test]
    fn il_contesto_riconosce_nominativi_standard_e_costruisce_i_bit() {
        let mut o = ApOptions::new("iu3abc");
        o.hiscall = "K1ABC".into();
        o.hisgrid = "FN42".into();
        let ctx = ApContext::new(&o);
        assert!(ctx.my_ok && ctx.his_ok && ctx.his_grid_known);
        // i 58 bit di nominativi di "IU3ABC K1ABC RRR" sono quelli del messaggio vero
        let bits = bits_of("IU3ABC K1ABC RRR").expect("messaggio");
        let (z, mask) = ctx.apply(3, &[0.0; 174], 10.0).expect("tipo 3");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 58 + 3);
        for i in 0..58 {
            assert_eq!(z[i] > 0.0, bits[i] == 1, "bit {i}");
        }
        let (_, mask) = ctx.apply(6, &[0.0; 174], 10.0).expect("tipo 6");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 77);
        let (_, mask) = ctx.apply(31, &[0.0; 174], 10.0).expect("tipo 31");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 77);
        let (_, mask) = ctx.apply(35, &[0.0; 174], 10.0).expect("tipo 35");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 29 + 19);
        let (_, mask) = ctx.apply(1, &[0.0; 174], 10.0).expect("tipo 1");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 29 + 3);
    }

    #[test]
    fn senza_dati_restano_solo_i_tipi_che_non_ne_chiedono() {
        let ctx = ApContext::new(&ApOptions::new(""));
        assert!(ctx.apply(1, &[0.0; 174], 10.0).is_some());
        for t in [2u8, 3, 4, 5, 6, 31, 35, 36] {
            assert!(ctx.apply(t, &[0.0; 174], 10.0).is_none(), "tipo {t}");
        }
        let mut dx = ApOptions::new("");
        dx.hiscall = "K1ABC".into();
        let ctx = ApContext::new(&dx);
        assert!(ctx.apply(31, &[0.0; 174], 10.0).is_some());
        assert!(ctx.apply(35, &[0.0; 174], 10.0).is_some());
        assert!(ctx.apply(3, &[0.0; 174], 10.0).is_none());
    }

    #[test]
    fn un_nominativo_non_standard_non_abilita_i_tipi_che_lo_richiedono() {
        let mut o = ApOptions::new("IU3ABC");
        o.hiscall = "PJ4/K1ABC".into();
        let ctx = ApContext::new(&o);
        assert!(ctx.my_ok && !ctx.his_ok);
        assert!(ctx.apply(3, &[0.0; 174], 10.0).is_none());
        assert!(ctx.apply(2, &[0.0; 174], 10.0).is_some());
    }

    /// Le posizioni fissate da `apply` e i loro segni rispetto ai bit di `text` (impacchettato con il suo hash).
    fn assert_mask_matches(ctx: &ApContext, aptype: u8, text: &str, positions: std::ops::Range<usize>, extra: usize) {
        let bits = packed(text).expect("messaggio valido");
        let (z, mask) = ctx.apply(aptype, &[0.0; 174], 10.0).unwrap_or_else(|| panic!("tipo {aptype}"));
        assert_eq!(mask.iter().filter(|&&m| m).count(), positions.len() + extra, "tipo {aptype}: bit noti");
        for i in positions {
            assert!(mask[i], "tipo {aptype}: bit {i} non fissato");
            assert_eq!(z[i] > 0.0, bits[i] == 1, "tipo {aptype}, {text}: bit {i}");
        }
        if extra == 3 {
            // i tre bit di tipo di un messaggio di tipo 1 (001)
            assert!(mask[74] && mask[75] && mask[76] && z[74] < 0.0 && z[75] < 0.0 && z[76] > 0.0, "tipo {aptype}: i3");
        }
    }

    #[test]
    fn i_nominativi_standard_sono_quelli_di_wsjtx() {
        for call in ["IU3ABC", "K1ABC", "IU3ABC/P", "K1ABC/R", "iu3abc", "W1AW", "2E0ABC", "E71A"] {
            assert!(is_standard_callsign(call), "{call}");
        }
        for call in ["PJ4/K1ABC", "PJ4/IU3ABC", "IU3ABC/QRP", "IU3ABC/7", "", "AB", "ABC1DEFG"] {
            assert!(!is_standard_callsign(call), "{call}");
        }
    }

    #[test]
    fn con_il_dx_non_standard_i_tipi_11_14_31_35_36_hanno_i_bit_dei_messaggi_veri() {
        let mut o = ApOptions::new("IU3ABC");
        o.hiscall = "PJ4/K1ABC".into();
        let ctx = ApContext::new(&o);
        assert_eq!(ctx.table, ApTable::DxNonStandard);
        assert_eq!(ctx.aptypes(), &NDXNSAPTYPES);
        assert!(ctx.my_ok && !ctx.his_ok && ctx.qso_tones_ok);
        // 11: `MYCALL <DXCALL> ???`, tipo 1 con l'hash a 22 bit del nominativo completo
        assert_mask_matches(&ctx, 11, "IU3ABC <PJ4/K1ABC> -16", 0..58, 3);
        assert_mask_matches(&ctx, 11, "IU3ABC <PJ4/K1ABC> R-09", 0..58, 3);
        // 12-14: `<MYCALL> DXCALL RRR|73|RR73`, tipo 4 intero
        assert_mask_matches(&ctx, 12, "<IU3ABC> PJ4/K1ABC RRR", 0..77, 0);
        assert_mask_matches(&ctx, 13, "<IU3ABC> PJ4/K1ABC 73", 0..77, 0);
        assert_mask_matches(&ctx, 14, "<IU3ABC> PJ4/K1ABC RR73", 0..77, 0);
        // 31: `CQ DXCALL` di tipo 4
        assert_mask_matches(&ctx, 31, "CQ PJ4/K1ABC", 0..77, 0);
        // 35, 36: dal bit 14 in poi valgono per qualunque stazione in hash
        assert_mask_matches(&ctx, 35, "<W9XYZ> PJ4/K1ABC 73", 13..77, 0);
        assert_mask_matches(&ctx, 36, "<IU3ABC> PJ4/K1ABC RR73", 13..77, 0);
        // i tipi del proprio nominativo non standard non valgono qui
        for t in [3u8, 4, 5, 6, 40, 41, 42, 43, 44] {
            assert!(ctx.apply(t, &[0.0; 174], 10.0).is_none(), "tipo {t}");
        }
        // l'hash a 12 bit del tipo 12 e' quello del proprio nominativo
        let (z, _) = ctx.apply(12, &[0.0; 174], 10.0).expect("tipo 12");
        let n12 = z[..12].iter().fold(0u32, |n, &v| (n << 1) | u32::from(v > 0.0));
        assert_eq!(n12, message::hash_call("IU3ABC", 12));
    }

    #[test]
    fn con_il_proprio_non_standard_i_tipi_40_44_usano_il_nominativo_completo() {
        let mut o = ApOptions::new("PJ4/IU3ABC");
        o.hiscall = "K1ABC".into();
        let ctx = ApContext::new(&o);
        assert_eq!(ctx.table, ApTable::MyNonStandard);
        assert_eq!(ctx.aptypes(), &NMYCNSAPTYPES);
        assert!(!ctx.my_ok && ctx.his_ok && ctx.qso_tones_ok);
        assert_mask_matches(&ctx, 41, "<PJ4/IU3ABC> K1ABC -15", 0..58, 3);
        assert_mask_matches(&ctx, 42, "PJ4/IU3ABC <K1ABC> RRR", 0..77, 0);
        assert_mask_matches(&ctx, 43, "PJ4/IU3ABC <K1ABC> 73", 0..77, 0);
        assert_mask_matches(&ctx, 44, "PJ4/IU3ABC <K1ABC> RR73", 0..77, 0);
        // 31, 35, 36 con il DX standard: come nella tabella standard
        assert_mask_matches(&ctx, 35, "W9XYZ K1ABC 73", 29..77, 0);
        assert_mask_matches(&ctx, 36, "W9XYZ K1ABC RR73", 29..77, 0);
        assert!(ctx.apply(31, &[0.0; 174], 10.0).is_some());
        // l'hash a 22 bit e' quello del nominativo completo, non della sola base
        let base = packed("<IU3ABC> K1ABC -15").expect("messaggio");
        let (z, _) = ctx.apply(41, &[0.0; 174], 10.0).expect("tipo 41");
        assert!((0..28).any(|i| (z[i] > 0.0) != (base[i] == 1)));
        // senza DX: solo `<MYCALL> ??? ???` (40)
        let solo = ApContext::new(&ApOptions::new("PJ4/IU3ABC"));
        assert_eq!(solo.table, ApTable::MyNonStandard);
        assert_mask_matches(&solo, 40, "<PJ4/IU3ABC> W9XYZ -10", 0..29, 3);
        assert!(solo.apply(41, &[0.0; 174], 10.0).is_none());
    }

    #[test]
    fn due_non_standard_lasciano_solo_cq_e_la_ricerca_del_dx() {
        let mut o = ApOptions::new("PJ4/IU3ABC");
        o.hiscall = "IU3ABC/QRP".into();
        let ctx = ApContext::new(&o);
        assert_eq!(ctx.table, ApTable::BothNonStandard);
        assert!(!ctx.qso_tones_ok);
        assert_mask_matches(&ctx, 31, "CQ IU3ABC/QRP", 0..77, 0);
        assert_mask_matches(&ctx, 36, "<K1ABC> IU3ABC/QRP RR73", 13..77, 0);
        for t in [11u8, 12, 13, 14, 41, 42] {
            assert!(ctx.apply(t, &[0.0; 174], 10.0).is_none(), "tipo {t}");
        }
        // senza il proprio nominativo e con il DX non standard: lo stesso ramo, come in WSJT-X
        let mut vuoto = ApOptions::new("");
        vuoto.hiscall = "PJ4/K1ABC".into();
        assert_eq!(ApContext::new(&vuoto).table, ApTable::BothNonStandard);
        // senza dati la tabella resta quella standard (solo CQ)
        assert_eq!(ApContext::new(&ApOptions::new("")).table, ApTable::Standard);
    }

    #[test]
    fn i_toni_attesi_con_un_nominativo_in_hash_tornano_sul_segnale_ideale() {
        let ideal = |text: &str| {
            let mut s8 = [[1.0f32; 8]; 79];
            for (row, &t) in s8.iter_mut().zip(tones_for_any(text).iter()) {
                row[usize::from(t)] = 10.0;
            }
            s8
        };
        let mut o = ApOptions::new("IU3ABC");
        o.hiscall = "PJ4/K1ABC".into();
        let ctx = ApContext::new(&o);
        // `idtone56(1,:)` = `MYCALL <DXCALL> -01`, `idtone56(55,:)` = `<MYCALL> DXCALL RR73` (`tone8.f90:81-97`)
        assert_eq!(tone_matches(&ideal("IU3ABC <PJ4/K1ABC> -01"), &ctx.qso_tones[..19], 0), 19);
        assert_eq!(tone_matches(&ideal("<IU3ABC> PJ4/K1ABC RR73"), &ctx.tones_rr73[23..], 23), 35);
        // la ricerca del DX non standard: i simboli 5-23 di `<...> DXCALL 73` non dipendono da chi c'e' in hash
        let dx73 = ctx.dxns_73_tones.expect("toni");
        assert_eq!(tone_matches(&ideal("<W9XYZ> PJ4/K1ABC 73"), &dx73[4..23], 4), 19);
        let cq = ctx.dxns_cq_tones.expect("toni");
        assert_eq!(tone_matches(&ideal("CQ PJ4/K1ABC"), &cq[..23], 0), 23);
    }

    fn tones_for_any(text: &str) -> [u8; 79] {
        tones_of_bits(&packed(text).expect("messaggio valido"))
    }

    #[test]
    fn i_toni_attesi_tornano_sul_segnale_ideale() {
        let mut o = ApOptions::new("IU3ABC");
        o.hiscall = "K1ABC".into();
        let ctx = ApContext::new(&o);
        // un periodo ideale per "IU3ABC K1ABC -01": il tono giusto a 10, gli altri a 1
        let tones = tones_for("IU3ABC K1ABC -01");
        let mut s8 = [[1.0f32; 8]; 79];
        for (row, &t) in s8.iter_mut().zip(tones.iter()) {
            row[usize::from(t)] = 10.0;
        }
        assert_eq!(tone_matches(&s8, &ctx.qso_tones[..19], 0), 19);
        assert_eq!(tone_matches(&s8, &ctx.my_tones, 0), 9);
        assert!(expected_ratio(&s8, &ctx.my_tones) > 5.0);
        // e un CQ ideale ha il punteggio CQ pieno
        let cq = tones_for("CQ K1ABC FN42");
        let mut s8cq = [[1.0f32; 8]; 79];
        for (row, &t) in s8cq.iter_mut().zip(cq.iter()) {
            row[usize::from(t)] = 10.0;
        }
        assert!(cq_score(&s8cq) >= 9.0, "{}", cq_score(&s8cq));
        assert_eq!(&data_tones(&cq)[..9], &ctx.cq_tones);
    }

    #[test]
    fn la_serie_di_llr_di_ogni_tipo_segue_l_ordine_di_jtdx() {
        // tipo 1: isubp2 20, 21, 22 -> llrc, llrb, llra
        assert_eq!([20, 21, 22].map(|i| llr_index(i, 1)), [2, 1, 0]);
        // i tipi 4-6 usano llrb due volte
        assert_eq!([8, 9, 10].map(|i| llr_index(i, 4)), [2, 1, 1]);
        assert_eq!(NAPTYPES[3][3], 6);
        assert_eq!(NAPTYPES[0][12], 2);
    }
}
