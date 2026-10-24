//! Messaggi di 77 bit (`packjt77`): nominativi standard, /P, non standard con hash, locatori, rapporti, testo libero.
//!
//! Porting di `lib/ft8v2/packjt77.f90` di JTDX 2.2.159 (GPL 3), con `chkcall.f90` e l'hash dei nominativi (`ihashcall`).
//! Tipi gestiti, nell'ordine in cui li prova `pack`:
//!
//! | i3.n3 | contenuto                                                   |
//! |-------|-------------------------------------------------------------|
//! | 0.1   | DXpedition: `K1ABC RR73; W9XYZ <KH1/KH7Z> -08`               |
//! | 0.3/4 | Field Day: `W9XYZ K1ABC R 17B EMA`                           |
//! | 0.5   | telemetria (fino a 18 cifre esadecimali)                     |
//! | 1, 2  | standard e `/P` (EU VHF): nominativi, CQ, locatore, rapporto |
//! | 3     | RTTY Roundup: `TU; KA0DEF K1ABC R 569 MA`                    |
//! | 4     | un nominativo non standard e uno in hash a 12 bit            |
//! | 5     | EU VHF con due hash: `<G4ABC> <PA9XYZ> R 570007 JO22DB`      |
//! | 0.0   | testo libero di 13 caratteri                                 |
//!
//! Non e' portato il tipo 0.6 (WSPR dentro FT8): `unpack` risponde [`MessageError::Unsupported`].
//!
//! Differenze rispetto al Fortran di JTDX:
//! - le tabelle degli hash sono una sola (JTDX ne tiene una per la ricezione e una per i messaggi in trasmissione, scelte con
//!   `nthr`): `pack` memorizza i nominativi che trasmette, `unpack` quelli che decodifica, cosi' un messaggio con `<K1ABC>` si
//!   rilegge subito. I nominativi decodificati entrano nelle tabelle solo se il messaggio e' valido (JTDX li tiene prima in
//!   `last_calls` e li travasa dopo);
//! - `pack` non tronca in silenzio: dove il Fortran manderebbe un nominativo storpiato (per esempio `K1ABC/QRP` scritto come
//!   `K1ABC`, o `W9XYZ PJ4/K1ABC RR73` senza `<>`) qui il messaggio scende al tipo successivo, in fondo al testo libero, e se
//!   anche quello non va si ottiene un errore. Lo stesso per rapporti fuori da -50..+50, classi Field Day oltre H, rapporti
//!   RTTY non del tipo 5x9, `<...>` scritto a mano e caratteri fuori dall'alfabeto;
//! - il tipo 4 con un nominativo senza barra piu' lungo di 6 caratteri (`<W9XYZ> YW18FIFA`) si impacchetta come fa `ft8code` di
//!   WSJT-X; il `chkcall` di JTDX lo rifiuterebbe;
//! - `unpack` rifiuta in piu' i numeri di 28 bit riservati tra i CQ e i nominativi, i rapporti oltre 105, i testi liberi che
//!   superano 42^13 e la telemetria tutta a zero (JTDX li lascerebbe passare con un testo senza senso);
//! - il tipo 0.6 (WSPR) non e' gestito e quindi `pack` non lo tenta mai: `K1ABC/P 73` va a finire in testo libero.
//!
//! Restano uguali, di proposito, i filtri anti falsi di JTDX in `unpack` (nominativi con due cifre iniziali, spazi interni,
//! `CQ` con rapporto, tipo 4 che finisce con un hash nudo...): sono la ragione per cui JTDX non mostra messaggi spuri.

use std::collections::HashMap;
use std::fmt;

/// 77 bit, uno per elemento (0 o 1).
pub type Bits77 = [u8; 77];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageError {
    /// Tipo di messaggio non gestito.
    Unsupported,
    /// Il testo non e' un messaggio valido.
    Invalid(String),
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::Unsupported => write!(f, "tipo di messaggio non gestito"),
            MessageError::Invalid(motivo) => write!(f, "messaggio non valido: {motivo}"),
        }
    }
}

impl std::error::Error for MessageError {}

fn invalid<T>(motivo: &str) -> Result<T, MessageError> {
    Err(MessageError::Invalid(motivo.to_string()))
}

// ---------------------------------------------------------------------------------------------------------------------
// Costanti e alfabeti
// ---------------------------------------------------------------------------------------------------------------------

/// Numeri di 28 bit riservati a DE, QRZ, CQ, CQ nnn e CQ aaaa.
const NTOKENS: u32 = 2_063_592;
/// Numeri di 28 bit riservati agli hash a 22 bit (subito dopo i token).
const MAX22: u32 = 4_194_304;
/// Primo valore di `igrid4` che non e' un locatore: da qui in poi blank, RRR, RR73, 73 e i rapporti.
const MAXGRID4: u32 = 32_400;
/// Hash a 22 bit tenuti in memoria (i piu' recenti).
const MAXHASH: usize = 1000;

/// Alfabeto dei nominativi per l'hash e per il tipo 4: spazio, cifre, lettere, barra.
const ALFA38: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ/";
const ALFA1: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALFA2: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALFA3: &[u8] = b"0123456789";
const ALFA4: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// Alfabeto del testo libero.
const ALFA_TESTO: &[u8] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ+-./?";

/// Sezioni ARRL del Field Day (85), nell'ordine del protocollo.
const SEZIONI: [&str; 85] = [
    "AB", "AK", "AL", "AR", "AZ", "BC", "CO", "CT", "DE", "EB", "EMA", "ENY", "EPA", "EWA", "GA", "GTA", "IA", "ID", "IL", "IN", "KS", "KY",
    "LA", "LAX", "MAR", "MB", "MDC", "ME", "MI", "MN", "MO", "MS", "MT", "NC", "ND", "NE", "NFL", "NH", "NL", "NLI", "NM", "NNJ", "NNY",
    "NT", "NTX", "NV", "OH", "OK", "ONE", "ONN", "ONS", "OR", "ORG", "PAC", "PR", "QC", "RI", "SB", "SC", "SCV", "SD", "SDG", "SF", "SFL",
    "SJV", "SK", "SNJ", "STX", "SV", "TN", "UT", "VA", "VI", "VT", "WCF", "WI", "WMA", "WNY", "WPA", "WTX", "WV", "WWA", "WY", "DX", "PE",
];

/// Stati e province del RTTY Roundup (65), nell'ordine del protocollo.
const MOLTIPLICATORI: [&str; 65] = [
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN",
    "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA",
    "WA", "WV", "WI", "WY", "NB", "NS", "QC", "ON", "MB", "SK", "AB", "BC", "NWT", "NF", "LB", "NU", "YT", "PEI", "DC",
];

fn indice(tabella: &[u8], c: u8) -> Option<usize> {
    tabella.iter().position(|&t| t == c)
}

fn e_cifra(c: u8) -> bool {
    c.is_ascii_digit()
}

fn e_lettera(c: u8) -> bool {
    c.is_ascii_uppercase()
}

// ---------------------------------------------------------------------------------------------------------------------
// Hash dei nominativi
// ---------------------------------------------------------------------------------------------------------------------

/// Hash di un nominativo a `bits` bit (10, 12 o 22): `ihashcall` di JTDX. Conta solo dei primi 11 caratteri, in base 38
/// (spazio, cifre, lettere, barra); un carattere fuori alfabeto vale -1, come nel Fortran.
pub fn hash_call(call: &str, bits: u32) -> u32 {
    let b = call.as_bytes();
    let mut n: i64 = 0;
    for i in 0..11 {
        let c = b.get(i).copied().unwrap_or(b' ');
        let j = indice(ALFA38, c).map_or(-1, |p| p as i64);
        n = n.wrapping_mul(38).wrapping_add(j);
    }
    let prodotto = 47_055_833_459_i64.wrapping_mul(n) as u64;
    (prodotto >> (64 - bits)) as u32
}

/// I nominativi visti finora, per i messaggi che ne portano solo l'impronta (hash a 10, 12 e 22 bit).
///
/// Un hash non ancora noto si legge come `<...>`, come in JTDX.
#[derive(Debug, Default, Clone)]
pub struct CallHashes {
    calls10: HashMap<u32, String>,
    calls12: HashMap<u32, String>,
    /// I piu' recenti in testa, al massimo [`MAXHASH`].
    calls22: Vec<(u32, String)>,
}

impl CallHashes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Memorizza un nominativo (anche tra `<>`): ignora `<...>`, i nomi sotto i 3 caratteri e quelli che iniziano con uno
    /// spazio (`save_hash_mycall`). Un nominativo nuovo con lo stesso hash prende il posto del vecchio.
    pub fn save_call(&mut self, call: &str) {
        let maiuscolo = call.trim_end().to_ascii_uppercase();
        let mut cw = maiuscolo.as_str();
        if cw.is_empty() || cw.starts_with(' ') || cw.starts_with("<...>") {
            return;
        }
        if let Some(resto) = cw.strip_prefix('<') {
            cw = resto;
        }
        if let Some(i) = cw.find('>') {
            cw = &cw[..i];
        }
        let cw: String = cw.chars().take(13).collect();
        if cw.trim_end().len() < 3 {
            return;
        }
        self.calls12.insert(hash_call(&cw, 12), cw.clone());
        self.calls10.insert(hash_call(&cw, 10), cw.clone());
        let n22 = hash_call(&cw, 22);
        if let Some(voce) = self.calls22.iter_mut().find(|(h, _)| *h == n22) {
            voce.1 = cw;
            return;
        }
        self.calls22.insert(0, (n22, cw));
        self.calls22.truncate(MAXHASH);
    }

    /// Il nominativo che ha questo hash a 10 bit, se lo conosciamo.
    pub fn call10(&self, hash: u32) -> Option<&str> {
        self.calls10.get(&hash).map(String::as_str)
    }

    /// Il nominativo che ha questo hash a 12 bit, se lo conosciamo.
    pub fn call12(&self, hash: u32) -> Option<&str> {
        self.calls12.get(&hash).map(String::as_str)
    }

    /// Il nominativo che ha questo hash a 22 bit, se lo conosciamo.
    pub fn call22(&self, hash: u32) -> Option<&str> {
        self.calls22.iter().find(|(h, _)| *h == hash).map(|(_, c)| c.as_str())
    }

    /// Quanti hash a 22 bit sono in memoria.
    pub fn len(&self) -> usize {
        self.calls22.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls22.is_empty()
    }

    fn mostra(trovato: Option<&str>) -> String {
        match trovato {
            Some(c) => format!("<{c}>"),
            None => "<...>".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Bit
// ---------------------------------------------------------------------------------------------------------------------

struct Scrittore {
    bits: Bits77,
    pos: usize,
}

impl Scrittore {
    fn new() -> Self {
        Scrittore { bits: [0; 77], pos: 0 }
    }

    /// Scrive `valore` su `larghezza` bit, il piu' significativo per primo.
    fn put(&mut self, valore: u128, larghezza: usize) {
        for i in (0..larghezza).rev() {
            if let Some(b) = self.bits.get_mut(self.pos) {
                *b = ((valore >> i) & 1) as u8;
            }
            self.pos += 1;
        }
    }

    fn fine(self) -> Option<Bits77> {
        (self.pos == 77).then_some(self.bits)
    }
}

struct Lettore<'a> {
    bits: &'a Bits77,
    pos: usize,
}

impl<'a> Lettore<'a> {
    fn new(bits: &'a Bits77) -> Self {
        Lettore { bits, pos: 0 }
    }

    fn get(&mut self, larghezza: usize) -> u64 {
        let mut v = 0u64;
        for _ in 0..larghezza {
            v = (v << 1) | u64::from(self.bits.get(self.pos).copied().unwrap_or(0));
            self.pos += 1;
        }
        v
    }
}

/// Tipo del messaggio (i3, n3) letto dagli ultimi 6 bit. Per i3 diverso da 0 `n3` e' parte del contenuto e non ha significato.
pub fn message_type(bits: &Bits77) -> (u8, u8) {
    let leggi = |da: usize| bits[da..da + 3].iter().fold(0u8, |a, &b| (a << 1) | (b & 1));
    let i3 = leggi(74);
    let n3 = if i3 == 0 { leggi(71) } else { 0 };
    (i3, n3)
}

// ---------------------------------------------------------------------------------------------------------------------
// Controlli su nominativi e parole (chkcall, locatori, rapporti)
// ---------------------------------------------------------------------------------------------------------------------

/// `chkcall`: la parola puo' essere un nominativo standard o composto? Restituisce il nominativo di base.
fn chkcall(w: &str) -> Option<String> {
    let b = w.as_bytes();
    let n1 = b.len() as i64;
    if n1 > 11 || b.iter().any(|c| matches!(c, b'.' | b'+' | b'-' | b'?')) {
        return None;
    }
    let barra = b.iter().position(|&c| c == b'/');
    if n1 > 6 && barra.is_none() {
        return None;
    }
    let i0 = barra.map_or(0, |p| p as i64 + 1);
    if (i0 - 1).max(n1 - i0) > 6 {
        return None;
    }
    let mut bc: Vec<u8> = b.iter().take(6).copied().collect();
    if i0 >= 2 && i0 < n1 {
        let i0u = i0 as usize;
        bc = if i0 - 1 <= n1 - i0 { b[i0u..].to_vec() } else { b[..i0u - 1].to_vec() };
    }
    let nbc = bc.len();
    if nbc > 6 {
        return None;
    }
    let at = |i: usize| bc.get(i).copied().unwrap_or(b' ');
    if !e_lettera(at(0)) && !e_lettera(at(1)) {
        return None;
    }
    if at(0) == b'Q' {
        return None;
    }
    let mut i1 = 0;
    if e_cifra(at(1)) {
        i1 = 2;
    }
    if e_cifra(at(2)) {
        i1 = 3;
    }
    if i1 == 0 || i1 == nbc {
        return None;
    }
    let suffisso = &bc[i1..];
    if suffisso.iter().all(|&c| e_lettera(c)) && (1..=3).contains(&suffisso.len()) {
        Some(String::from_utf8_lossy(&bc).into_owned())
    } else {
        None
    }
}

fn e_grid4(s: &[u8]) -> bool {
    s.len() == 4
        && (b'A'..=b'R').contains(&s[0])
        && (b'A'..=b'R').contains(&s[1])
        && e_cifra(s[2])
        && e_cifra(s[3])
}

fn e_grid6(s: &[u8]) -> bool {
    s.len() == 6 && e_grid4(&s[..4]) && (b'A'..=b'X').contains(&s[4]) && (b'A'..=b'X').contains(&s[5])
}

/// Numero con segno opzionale e solo cifre.
fn intero(s: &str) -> Option<i32> {
    let senza = s.strip_prefix(['+', '-']).unwrap_or(s);
    if senza.is_empty() || senza.len() > 9 || !senza.bytes().all(e_cifra) {
        return None;
    }
    s.strip_prefix('+').unwrap_or(s).parse().ok()
}

/// Rapporto in dB tra -50 e +50.
fn rapporto(s: &str) -> Option<i32> {
    intero(s).filter(|r| (-50..=50).contains(r))
}

/// `+05`, `-09`, `+12`: il formato `i3.2` con il segno sempre presente.
fn formato_rapporto(r: i32) -> String {
    if r < 0 { format!("-{:02}", -r) } else { format!("+{r:02}") }
}

// ---------------------------------------------------------------------------------------------------------------------
// pack28 / unpack28
// ---------------------------------------------------------------------------------------------------------------------

/// Numero di 28 bit di un nominativo standard (6 caratteri: area al secondo o terzo posto, suffisso di 1-3 lettere).
fn codifica_standard(c: &str) -> Option<u32> {
    let b = c.as_bytes();
    let n = b.len();
    if !(3..=6).contains(&n) {
        return None;
    }
    let mut iarea = 1;
    for i in (2..=n).rev() {
        if e_cifra(b[i - 1]) {
            iarea = i;
            break;
        }
    }
    if !(2..=3).contains(&iarea) {
        return None;
    }
    let prima = &b[..iarea - 1];
    let dopo = &b[iarea..];
    if !prima.iter().all(u8::is_ascii_alphanumeric) || !dopo.iter().all(|&c| e_lettera(c)) || dopo.len() > 3 {
        return None;
    }
    let npdig = prima.iter().filter(|&&c| e_cifra(c)).count();
    let nplet = prima.iter().filter(|&&c| e_lettera(c)).count();
    if nplet == 0 || npdig >= iarea - 1 {
        return None;
    }
    let mut sei = [b' '; 6];
    if iarea == 2 {
        sei[1..1 + n.min(5)].copy_from_slice(&b[..n.min(5)]);
    } else {
        sei[..n].copy_from_slice(b);
    }
    let i1 = indice(ALFA1, sei[0])? as u32;
    let i2 = indice(ALFA2, sei[1])? as u32;
    let i3 = indice(ALFA3, sei[2])? as u32;
    let i4 = indice(ALFA4, sei[3])? as u32;
    let i5 = indice(ALFA4, sei[4])? as u32;
    let i6 = indice(ALFA4, sei[5])? as u32;
    let n28 = 36 * 10 * 27 * 27 * 27 * i1 + 10 * 27 * 27 * 27 * i2 + 27 * 27 * 27 * i3 + 27 * 27 * i4 + 27 * i5 + i6;
    Some(n28 + NTOKENS + MAX22)
}

/// Nominativo ammesso nel tipo 4 e nell'hash: da 3 a 11 caratteri tra cifre, lettere e barra, senza barra ai bordi.
fn nominativo_valido(c: &str) -> bool {
    let b = c.as_bytes();
    (3..=11).contains(&b.len())
        && b.iter().all(|&x| x.is_ascii_uppercase() || x.is_ascii_digit() || x == b'/')
        && b[0] != b'/'
        && b[b.len() - 1] != b'/'
}

/// Numero di 28 bit di CQ, DE, QRZ, CQ nnn, CQ aaaa. `None` se la parola non e' un token valido.
fn token_n28(t: &str) -> Option<u32> {
    match t {
        "DE" => return Some(0),
        "QRZ" => return Some(1),
        "CQ" => return Some(2),
        _ => {}
    }
    let resto = t.strip_prefix("CQ_")?;
    let b = resto.as_bytes();
    if !(1..=4).contains(&b.len()) {
        return None;
    }
    if b.len() == 3 && b.iter().all(|&c| e_cifra(c)) {
        let nqsy: u32 = resto.parse().ok()?;
        return Some(3 + nqsy);
    }
    if b.iter().all(|&c| e_lettera(c)) {
        // allineato a destra su 4 posti, lo spazio vale 0
        let mut m = 0u32;
        for i in 0..4 {
            let pos = i as isize - (4 - b.len() as isize);
            let j = if pos >= 0 { u32::from(b[pos as usize] - b'A' + 1) } else { 0 };
            m = 27 * m + j;
        }
        return Some(3 + 1000 + m);
    }
    None
}

/// Un nominativo da scrivere: token, hash o standard, con il suffisso che sta a parte.
#[derive(Debug, Clone, PartialEq)]
enum Parte {
    Token(String),
    Hash(String),
    Standard { base: String, suffisso: Suffisso },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Suffisso {
    Nessuno,
    R,
    P,
}

impl Parte {
    /// Numero di 28 bit; i nominativi da ricordare finiscono in `da_salvare`.
    fn n28(&self, da_salvare: &mut Vec<String>) -> Option<u32> {
        match self {
            Parte::Token(t) => token_n28(t),
            Parte::Hash(inner) => {
                da_salvare.push(inner.clone());
                Some(NTOKENS + hash_call(inner, 22))
            }
            Parte::Standard { base, .. } => {
                da_salvare.push(base.clone());
                codifica_standard(base)
            }
        }
    }
}

/// Riconosce la parola come token (solo se `token` e' vero), `<hash>` o nominativo standard con /R o /P.
fn classifica(parola: &str, token: bool) -> Option<Parte> {
    if token && (matches!(parola, "DE" | "CQ" | "QRZ") || parola.starts_with("CQ_")) {
        token_n28(parola)?;
        return Some(Parte::Token(parola.to_string()));
    }
    if parola.starts_with('<') {
        let inner = parola.strip_prefix('<')?.strip_suffix('>')?;
        if inner == "..." || !nominativo_valido(inner) {
            return None;
        }
        return Some(Parte::Hash(inner.to_string()));
    }
    let (base, suffisso) = if parola.len() >= 5 && parola.ends_with("/R") {
        (&parola[..parola.len() - 2], Suffisso::R)
    } else if parola.len() >= 5 && parola.ends_with("/P") {
        (&parola[..parola.len() - 2], Suffisso::P)
    } else {
        (parola, Suffisso::Nessuno)
    };
    if base.contains('/') || chkcall(base).is_none() || codifica_standard(base).is_none() {
        return None;
    }
    Some(Parte::Standard { base: base.to_string(), suffisso })
}

/// 28 bit di un nominativo qualunque, per i tipi che non distinguono (Field Day, RTTY, DXpedition): standard se lo e', altrimenti
/// hash a 22 bit.
fn pack28_libero(c: &str, da_salvare: &mut Vec<String>) -> Option<u32> {
    if let Some(inner) = c.strip_prefix('<') {
        let inner = inner.strip_suffix('>')?;
        if inner == "..." || !nominativo_valido(inner) {
            return None;
        }
        da_salvare.push(inner.to_string());
        return Some(NTOKENS + hash_call(inner, 22));
    }
    if let Some(n) = token_n28(c) {
        return Some(n);
    }
    if !nominativo_valido(c) {
        return None;
    }
    da_salvare.push(c.to_string());
    Some(codifica_standard(c).unwrap_or_else(|| NTOKENS + hash_call(c, 22)))
}

/// Dal numero di 28 bit al nominativo: `None` se non e' un valore valido o il risultato non puo' essere un nominativo.
fn unpack28(n28: u32, hash: &CallHashes) -> Option<String> {
    let testo = if n28 < NTOKENS {
        match n28 {
            0 => "DE".to_string(),
            1 => "QRZ".to_string(),
            2 => "CQ".to_string(),
            3..=1002 => format!("CQ_{:03}", n28 - 3),
            1003..=532_443 => {
                let mut n = n28 - 1003;
                let mut l = [b' '; 4];
                for k in (0..4).rev() {
                    l[k] = ALFA4[(n % 27) as usize];
                    n /= 27;
                }
                let lettere = String::from_utf8_lossy(&l).trim_start().to_string();
                format!("CQ_{lettere}")
            }
            _ => return None,
        }
    } else {
        let n = n28 - NTOKENS;
        if n < MAX22 {
            CallHashes::mostra(hash.call22(n))
        } else {
            let mut n = n - MAX22;
            let i1 = n / (36 * 10 * 27 * 27 * 27);
            n -= 36 * 10 * 27 * 27 * 27 * i1;
            let i2 = n / (10 * 27 * 27 * 27);
            n -= 10 * 27 * 27 * 27 * i2;
            let i3 = n / (27 * 27 * 27);
            n -= 27 * 27 * 27 * i3;
            let i4 = n / (27 * 27);
            n -= 27 * 27 * i4;
            let i5 = n / 27;
            let i6 = n - 27 * i5;
            let sei = [
                *ALFA1.get(i1 as usize)?,
                ALFA2[i2 as usize],
                ALFA3[i3 as usize],
                ALFA4[i4 as usize],
                ALFA4[i5 as usize],
                ALFA4[i6 as usize],
            ];
            String::from_utf8_lossy(&sei).trim().to_string()
        }
    };
    // spazi dentro il nominativo, o due cifre in testa: non e' un nominativo
    let b = testo.as_bytes();
    if testo.contains(' ') || (b.len() >= 2 && e_cifra(b[0]) && e_cifra(b[1])) {
        return None;
    }
    Some(testo)
}

// ---------------------------------------------------------------------------------------------------------------------
// Dal testo ai 77 bit
// ---------------------------------------------------------------------------------------------------------------------

type Impacchettato = (Bits77, Vec<String>);

/// `split77`: maiuscolo, blank ripetuti ridotti a uno, parole.
fn parole(testo: &str) -> Result<Vec<String>, MessageError> {
    if !testo.bytes().all(|c| c == 0 || c == b'\t' || (0x20..0x7f).contains(&c)) {
        return invalid("caratteri non ammessi (solo ASCII stampabile)");
    }
    let w: Vec<String> = testo
        .split([' ', '\t', '\0'])
        .filter(|p| !p.is_empty())
        .map(|p| p.to_ascii_uppercase())
        .collect();
    if w.is_empty() {
        return invalid("messaggio vuoto");
    }
    let lunghezza = w.iter().map(String::len).sum::<usize>() + w.len() - 1;
    if lunghezza > 37 {
        return invalid("messaggio troppo lungo (oltre 37 caratteri)");
    }
    Ok(w)
}

/// "CQ DX IU3ABC IO91" diventa ["CQ_DX", "IU3ABC", "IO91"]: il modificatore del CQ si fonde con la prima parola.
fn fondi_cq(w: &[String]) -> Vec<String> {
    let mut v = w.to_vec();
    if v.len() >= 3 && v[0] == "CQ" && chkcall(&v[2]).is_some() {
        let mod_cq: String = v[1].chars().take(10).collect();
        v[0] = format!("CQ_{mod_cq}");
        v.remove(1);
    }
    v
}

/// Dal testo ai 77 bit.
pub fn pack(text: &str, hashes: &mut CallHashes) -> Result<Bits77, MessageError> {
    let originali = parole(text)?;
    let w = fondi_cq(&originali);
    let tipo_cq = matches!(originali[0].as_str(), "CQ" | "DE" | "QRZ");

    let mut prove: Vec<Option<Impacchettato>> = Vec::new();
    if !tipo_cq {
        prove.push(pack_dxpedition(&w));
        prove.push(pack_field_day(&w));
        if w.len() < 2 {
            prove.push(pack_telemetria(&w[0]));
        }
    }
    prove.push(pack_standard(&w));
    prove.push(pack_rtty(&w));
    prove.push(pack_non_standard(&w));
    prove.push(pack_eu_vhf(&w));

    if let Some((bits, da_salvare)) = prove.into_iter().flatten().next() {
        for c in &da_salvare {
            hashes.save_call(c);
        }
        return Ok(bits);
    }
    pack_testo_libero(&originali.join(" "))
}

/// Tipo 0.1: `K1ABC RR73; W9XYZ <KH1/KH7Z> -08`.
fn pack_dxpedition(w: &[String]) -> Option<Impacchettato> {
    if w.len() != 5 || w[1] != "RR73;" || !w[3].starts_with('<') || !w[3].ends_with('>') {
        return None;
    }
    let n = intero(&w[4])?;
    let n5 = ((n + 30) / 2).clamp(0, 31) as u128;
    chkcall(&w[0])?;
    chkcall(&w[2])?;
    let mut salva = Vec::new();
    let n28a = pack28_libero(&w[0], &mut salva)?;
    let n28b = pack28_libero(&w[2], &mut salva)?;
    let inner = &w[3][1..w[3].len() - 1];
    if inner == "..." || !nominativo_valido(inner) {
        return None;
    }
    salva.push(inner.to_string());
    let mut s = Scrittore::new();
    s.put(n28a.into(), 28);
    s.put(n28b.into(), 28);
    s.put(hash_call(inner, 10).into(), 10);
    s.put(n5, 5);
    s.put(1, 3);
    s.put(0, 3);
    Some((s.fine()?, salva))
}

/// Tipi 0.3 e 0.4, Field Day: `W9XYZ K1ABC R 17B EMA`.
fn pack_field_day(w: &[String]) -> Option<Impacchettato> {
    let n = w.len();
    if !(4..=5).contains(&n) {
        return None;
    }
    chkcall(&w[0])?;
    chkcall(&w[1])?;
    let sezione = &w[n - 1];
    if !(2..=3).contains(&sezione.len()) {
        return None;
    }
    let isec = SEZIONI.iter().position(|s| s == sezione)? + 1;
    if n == 5 && w[2] != "R" {
        return None;
    }
    let esch = w[n - 2].as_bytes();
    if esch.len() < 2 {
        return None;
    }
    let (cifre, classe) = esch.split_at(esch.len() - 1);
    if cifre.len() > 2 || !cifre.iter().all(|&c| e_cifra(c)) {
        return None;
    }
    let ntx: u32 = std::str::from_utf8(cifre).ok()?.parse().ok()?;
    if !(1..=32).contains(&ntx) {
        return None;
    }
    let nclass = u32::from(classe[0]).checked_sub(u32::from(b'A'))?;
    if nclass > 7 {
        return None;
    }
    let (n3, intx) = if ntx > 16 { (4, ntx - 17) } else { (3, ntx - 1) };
    let mut salva = Vec::new();
    let n28a = pack28_libero(&w[0], &mut salva)?;
    let n28b = pack28_libero(&w[1], &mut salva)?;
    let ir = u128::from(n == 5);
    let mut s = Scrittore::new();
    s.put(n28a.into(), 28);
    s.put(n28b.into(), 28);
    s.put(ir, 1);
    s.put(intx.into(), 4);
    s.put(nclass.into(), 3);
    s.put(isec as u128, 7);
    s.put(n3, 3);
    s.put(0, 3);
    Some((s.fine()?, salva))
}

/// Tipo 0.5, telemetria: una sola parola di 1-18 cifre esadecimali (la prima, in 18 cifre, al massimo 7).
fn pack_telemetria(parola: &str) -> Option<Impacchettato> {
    if parola.is_empty() || parola.len() > 18 || !parola.bytes().all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(&c)) {
        return None;
    }
    let v = u128::from_str_radix(parola, 16).ok()?;
    // 18 cifre esadecimali = 72 bit; i primi 23 sono i primi sei esadecimali senza l'ultimo bit alto
    if v >> 71 != 0 {
        return None;
    }
    let mut s = Scrittore::new();
    s.put(v, 71);
    s.put(5, 3);
    s.put(0, 3);
    Some((s.fine()?, Vec::new()))
}

/// Tipi 1 e 2: due nominativi (o CQ/DE/QRZ), con locatore, rapporto, R+rapporto, RRR, RR73, 73 o niente.
fn pack_standard(w: &[String]) -> Option<Impacchettato> {
    let n = w.len();
    if !(2..=4).contains(&n) {
        return None;
    }
    let p1 = classifica(&w[0], true)?;
    let p2 = classifica(&w[1], false)?;
    let hash1 = matches!(p1, Parte::Hash(_));
    let hash2 = matches!(p2, Parte::Hash(_));
    if (hash1 && w[1].contains('/')) || (hash2 && w[0].contains('/')) {
        return None;
    }
    if n == 2 && w[1].contains('/') {
        return None;
    }
    let (ir, igrid4): (u32, u32) = if n == 2 {
        (0, MAXGRID4 + 1)
    } else {
        let ultima = w[n - 1].as_str();
        let primi4 = ultima.as_bytes().get(..4).unwrap_or(&[]);
        if e_grid4(primi4) {
            let ir = u32::from(n == 4 && w[2] == "R");
            if n == 4 && ir == 0 {
                return None;
            }
            let g = |i: usize| u32::from(primi4[i]);
            let valore = (g(0) - 65) * 1800 + (g(1) - 65) * 100 + (g(2) - 48) * 10 + (g(3) - 48);
            (ir, valore)
        } else if n == 4 {
            return None;
        } else if ultima.starts_with(['+', '-']) {
            (0, MAXGRID4 + rapporto_codificato(rapporto(ultima)?))
        } else if let Some(r) = ultima.strip_prefix('R').filter(|r| r.starts_with(['+', '-'])) {
            (1, MAXGRID4 + rapporto_codificato(rapporto(r)?))
        } else if ultima == "RRR" {
            (0, MAXGRID4 + 2)
        } else if ultima == "73" {
            (0, MAXGRID4 + 4)
        } else {
            return None;
        }
    };
    // un CQ non porta rapporto, R, RRR o 73 (i decodificatori lo scarterebbero)
    if matches!(&p1, Parte::Token(t) if t.starts_with("CQ")) && (ir == 1 || (igrid4 > MAXGRID4 && igrid4 != MAXGRID4 + 1)) {
        return None;
    }
    let suffissi = [suffisso_di(&p1), suffisso_di(&p2)];
    let ha_r = suffissi.contains(&Suffisso::R);
    let ha_p = suffissi.contains(&Suffisso::P);
    if ha_r && ha_p {
        return None;
    }
    let i3: u128 = if ha_p { 2 } else { 1 };
    let mut salva = Vec::new();
    let n28a = p1.n28(&mut salva)?;
    let n28b = p2.n28(&mut salva)?;
    let mut s = Scrittore::new();
    s.put(n28a.into(), 28);
    s.put(u128::from(suffissi[0] != Suffisso::Nessuno), 1);
    s.put(n28b.into(), 28);
    s.put(u128::from(suffissi[1] != Suffisso::Nessuno), 1);
    s.put(ir.into(), 1);
    s.put(igrid4.into(), 15);
    s.put(i3, 3);
    Some((s.fine()?, salva))
}

fn suffisso_di(p: &Parte) -> Suffisso {
    match p {
        Parte::Standard { suffisso, .. } => *suffisso,
        _ => Suffisso::Nessuno,
    }
}

/// Rapporto in dB come lo scrive il protocollo: da -30 a +50 diretto (+35), da -50 a -31 sopra il +50 (+101).
fn rapporto_codificato(r: i32) -> u32 {
    let r = if (-50..=-31).contains(&r) { r + 101 } else { r };
    (r + 35) as u32
}

/// Tipo 3, RTTY Roundup: `TU; KA0DEF K1ABC R 569 MA`, `KA1ABC G3AAA 529 0013`.
fn pack_rtty(w: &[String]) -> Option<Impacchettato> {
    let n = w.len();
    if !(4..=6).contains(&n) || (w[0].starts_with('<') && w[1].starts_with('<')) {
        return None;
    }
    let itu = usize::from(w[0] == "TU;");
    chkcall(&w[itu])?;
    chkcall(&w[itu + 1])?;
    let ir = usize::from(w.get(2 + itu).is_some_and(|x| x == "R"));
    if n != 4 + itu + ir {
        return None;
    }
    let crpt = w[n - 2].as_bytes();
    if crpt.len() != 3 || crpt[0] != b'5' || !(b'2'..=b'9').contains(&crpt[1]) || crpt[2] != b'9' {
        return None;
    }
    let ultima = &w[n - 1];
    let nexch: u32 = if let Some(m) = MOLTIPLICATORI.iter().position(|x| x == ultima) {
        8000 + m as u32 + 1
    } else {
        let seriale = intero(ultima).filter(|s| (1..=7999).contains(s))?;
        if ultima.len() > 4 {
            return None;
        }
        seriale as u32
    };
    let irpt = u32::from(crpt[1] - b'2');
    let mut salva = Vec::new();
    let n28a = pack28_libero(&w[itu], &mut salva)?;
    let n28b = pack28_libero(&w[itu + 1], &mut salva)?;
    let mut s = Scrittore::new();
    s.put(itu as u128, 1);
    s.put(n28a.into(), 28);
    s.put(n28b.into(), 28);
    s.put(ir as u128, 1);
    s.put(irpt.into(), 3);
    s.put(nexch.into(), 13);
    s.put(3, 3);
    Some((s.fine()?, salva))
}

/// Tipo 4: un nominativo non standard in chiaro (11 caratteri) e uno in hash a 12 bit, oppure `CQ nominativo`.
fn pack_non_standard(w: &[String]) -> Option<Impacchettato> {
    let n = w.len();
    if !(2..=3).contains(&n) {
        return None;
    }
    let (w1, w2) = (&w[0], &w[1]);
    let mut salva = Vec::new();
    let (chiaro, n12, iflip, icq): (&str, u32, u32, u32) = if w1 == "CQ" {
        // un nominativo standard resta del tipo 1 (con il tipo 4 si perderebbe la coda del messaggio)
        let standard = chkcall(w2).is_some() && !w2.contains('/');
        if w2.len() <= 4 || w2.starts_with('<') || !nominativo_valido(w2) || standard {
            return None;
        }
        salva.push(w2.clone());
        // anche nel CQ il campo a 12 bit porta l'hash del nominativo (save_hash_txcall lo imposta), come in ft8code
        (w2, hash_call(w2, 12), 0, 1)
    } else {
        let (hash, altro, flip) = match (w1.starts_with('<'), w2.starts_with('<')) {
            (true, false) => (w1, w2, 0),
            (false, true) => (w2, w1, 1),
            _ => return None,
        };
        let inner = hash.strip_prefix('<')?.strip_suffix('>')?;
        if inner == "..." || !nominativo_valido(inner) || !nominativo_valido(altro) {
            return None;
        }
        let e_base = |c: &str| chkcall(c).is_some() && !c.contains('/');
        if e_base(inner) && e_base(altro) {
            return None;
        }
        salva.push(inner.to_string());
        (altro, hash_call(inner, 12), flip, 0)
    };
    let mut n58: u128 = 0;
    // allineato a destra su 11 posti
    for i in 0..11 {
        let pos = i as isize - (11 - chiaro.len() as isize);
        let c = if pos >= 0 { chiaro.as_bytes()[pos as usize] } else { b' ' };
        n58 = n58 * 38 + indice(ALFA38, c)? as u128;
    }
    let nrpt = if icq == 1 {
        0
    } else {
        match w.get(2).map(String::as_str) {
            Some("RRR") => 1,
            Some("RR73") => 2,
            Some("73") => 3,
            _ => 0,
        }
    };
    let mut s = Scrittore::new();
    s.put(n12.into(), 12);
    s.put(n58, 58);
    s.put(iflip.into(), 1);
    s.put(nrpt, 2);
    s.put(icq.into(), 1);
    s.put(4, 3);
    Some((s.fine()?, salva))
}

/// Tipo 5, EU VHF con due hash: `<G4ABC> <PA9XYZ> R 570007 JO22DB`.
fn pack_eu_vhf(w: &[String]) -> Option<Impacchettato> {
    let n = w.len();
    if !(4..=5).contains(&n) || !w[0].starts_with('<') || !w[1].starts_with('<') {
        return None;
    }
    let nx = intero(&w[n - 2])?;
    if !(520_001..=594_095).contains(&nx) {
        return None;
    }
    let grid = w[n - 1].as_bytes();
    if !e_grid6(grid) {
        return None;
    }
    let inner1 = w[0].strip_prefix('<')?.strip_suffix('>')?;
    let inner2 = w[1].strip_prefix('<')?.strip_suffix('>')?;
    if inner1 == "..." || inner2 == "..." || !nominativo_valido(inner1) || !nominativo_valido(inner2) {
        return None;
    }
    let ir = u32::from(n == 5 && w[2] == "R");
    if n == 5 && ir == 0 {
        return None;
    }
    let irpt = (nx / 10_000 - 52) as u32;
    let iseriale = ((nx % 10_000) as u32).min(2047);
    let g = |i: usize| u32::from(grid[i]);
    let igrid6 = (g(0) - 65) * 18 * 10 * 10 * 24 * 24
        + (g(1) - 65) * 10 * 10 * 24 * 24
        + (g(2) - 48) * 10 * 24 * 24
        + (g(3) - 48) * 24 * 24
        + (g(4) - 65) * 24
        + (g(5) - 65);
    let mut s = Scrittore::new();
    s.put(hash_call(inner1, 12).into(), 12);
    s.put(hash_call(inner2, 22).into(), 22);
    s.put(ir.into(), 1);
    s.put(irpt.into(), 3);
    s.put(iseriale.into(), 11);
    s.put(igrid6.into(), 25);
    s.put(5, 3);
    Some((s.fine()?, vec![inner1.to_string(), inner2.to_string()]))
}

/// Tipo 0.0, testo libero: fino a 13 caratteri tra spazio, cifre, lettere e `+-./?`.
fn pack_testo_libero(testo: &str) -> Result<Bits77, MessageError> {
    if testo.len() > 13 {
        return invalid("non e' un messaggio standard e come testo libero supera i 13 caratteri");
    }
    let b = testo.as_bytes();
    let mut v: u128 = 0;
    for i in 0..13 {
        let pos = i as isize - (13 - b.len() as isize);
        let c = if pos >= 0 { b[pos as usize] } else { b' ' };
        let Some(j) = indice(ALFA_TESTO, c) else {
            return invalid("carattere non ammesso nel testo libero (solo 0-9 A-Z spazio + - . / ?)");
        };
        v = v * 42 + j as u128;
    }
    let mut s = Scrittore::new();
    s.put(v, 71);
    s.put(0, 3);
    s.put(0, 3);
    s.fine().ok_or_else(|| MessageError::Invalid("errore interno nel testo libero".to_string()))
}

// ---------------------------------------------------------------------------------------------------------------------
// Dai 77 bit al testo
// ---------------------------------------------------------------------------------------------------------------------

/// Dai 77 bit al testo.
pub fn unpack(bits: &Bits77, hashes: &mut CallHashes) -> Result<String, MessageError> {
    if bits.iter().any(|&b| b > 1) {
        return invalid("bit diversi da 0 e 1");
    }
    let (i3, n3) = message_type(bits);
    let mut da_salvare: Vec<String> = Vec::new();
    let h: &CallHashes = hashes;
    let msg = match (i3, n3) {
        (0, 0) => unpack_testo_libero(bits)?,
        (0, 1) => unpack_dxpedition(bits, h)?,
        (0, 3) | (0, 4) => unpack_field_day(bits, n3, h)?,
        (0, 5) => unpack_telemetria(bits)?,
        (0, 6) => return Err(MessageError::Unsupported),
        (0, _) => return invalid("tipo 0 non definito"),
        (1, _) | (2, _) => unpack_standard(bits, i3, h, &mut da_salvare)?,
        (3, _) => unpack_rtty(bits, h)?,
        (4, _) => unpack_non_standard(bits, h, &mut da_salvare)?,
        (5, _) => unpack_eu_vhf(bits, h)?,
        _ => return invalid("tipo 6 e 7 non definiti"),
    };
    if msg.starts_with("CQ <") {
        return invalid("CQ con hash sconosciuto");
    }
    for c in &da_salvare {
        hashes.save_call(c);
    }
    Ok(msg)
}

fn nominativo(n28: u32, h: &CallHashes) -> Result<String, MessageError> {
    unpack28(n28, h).ok_or_else(|| MessageError::Invalid("nominativo non valido".to_string()))
}

fn unpack_testo_libero(bits: &Bits77) -> Result<String, MessageError> {
    let mut v = bits[..71].iter().fold(0u128, |a, &b| (a << 1) | u128::from(b));
    if v >= 42u128.pow(13) {
        return invalid("testo libero fuori scala");
    }
    let mut c = [b' '; 13];
    for k in c.iter_mut().rev() {
        *k = ALFA_TESTO[(v % 42) as usize];
        v /= 42;
    }
    let testo = String::from_utf8_lossy(&c).trim().to_string();
    if testo.is_empty() {
        return invalid("testo libero vuoto");
    }
    Ok(testo)
}

fn unpack_dxpedition(bits: &Bits77, h: &CallHashes) -> Result<String, MessageError> {
    let mut r = Lettore::new(bits);
    let n28a = r.get(28) as u32;
    let n28b = r.get(28) as u32;
    let n10 = r.get(10) as u32;
    let n5 = r.get(5) as i32;
    let c1 = nominativo(n28a, h)?;
    let c2 = nominativo(n28b, h)?;
    if n28a <= 2 || n28b <= 2 {
        return invalid("token al posto di un nominativo");
    }
    let c3 = CallHashes::mostra(h.call10(n10));
    Ok(format!("{c1} RR73; {c2} {c3} {}", formato_rapporto(2 * n5 - 30)))
}

fn unpack_field_day(bits: &Bits77, n3: u8, h: &CallHashes) -> Result<String, MessageError> {
    let mut r = Lettore::new(bits);
    let n28a = r.get(28) as u32;
    let n28b = r.get(28) as u32;
    let ir = r.get(1);
    let intx = r.get(4) as u32;
    let nclass = r.get(3) as u8;
    let isec = r.get(7) as usize;
    if !(1..=SEZIONI.len()).contains(&isec) {
        return invalid("sezione Field Day non valida");
    }
    let c1 = nominativo(n28a, h)?;
    let c2 = nominativo(n28b, h)?;
    if n28a <= 2 || n28b <= 2 {
        return invalid("token al posto di un nominativo");
    }
    let ntx = intx + 1 + if n3 == 4 { 16 } else { 0 };
    let classe = char::from(b'A' + nclass);
    let erre = if ir == 1 { "R " } else { "" };
    Ok(format!("{c1} {c2} {erre}{ntx}{classe} {}", SEZIONI[isec - 1]))
}

fn unpack_telemetria(bits: &Bits77) -> Result<String, MessageError> {
    let v = bits[..71].iter().fold(0u128, |a, &b| (a << 1) | u128::from(b));
    let esadecimale = format!("{v:018X}");
    let senza_zeri = esadecimale.trim_start_matches('0');
    if senza_zeri.is_empty() {
        return invalid("telemetria vuota");
    }
    Ok(senza_zeri.to_string())
}

fn grid4_da(n: u32) -> Option<String> {
    let (j1, j2, j3, j4) = (n / 1800, (n % 1800) / 100, (n % 100) / 10, n % 10);
    if j1 > 17 || j2 > 17 {
        return None;
    }
    Some(format!("{}{}{j3}{j4}", char::from(b'A' + j1 as u8), char::from(b'A' + j2 as u8)))
}

fn grid6_da(n: u32) -> Option<String> {
    let j1 = n / (18 * 10 * 10 * 24 * 24);
    let n = n - j1 * 18 * 10 * 10 * 24 * 24;
    let j2 = n / (10 * 10 * 24 * 24);
    let n = n - j2 * 10 * 10 * 24 * 24;
    let j3 = n / (10 * 24 * 24);
    let n = n - j3 * 10 * 24 * 24;
    let j4 = n / (24 * 24);
    let n = n - j4 * 24 * 24;
    let (j5, j6) = (n / 24, n % 24);
    if j1 > 17 || j2 > 17 {
        return None;
    }
    Some(format!(
        "{}{}{j3}{j4}{}{}",
        char::from(b'A' + j1 as u8),
        char::from(b'A' + j2 as u8),
        char::from(b'A' + j5 as u8),
        char::from(b'A' + j6 as u8)
    ))
}

/// Aggiunge /R o /P e ricorda il nominativo (`add_call_to_recent_calls`): solo se non e' un hash e ha almeno 3 caratteri.
fn con_suffisso(call: &mut String, flag: u64, i3: u8, da_salvare: &mut Vec<String>) {
    if call.contains('<') {
        return;
    }
    let token = matches!(call.as_str(), "DE" | "CQ" | "QRZ") || call.starts_with("CQ ");
    let posizione = call.find(' ').map_or(call.len() + 1, |p| p + 1);
    if posizione >= 4 && flag == 1 {
        call.push_str(if i3 == 1 { "/R" } else { "/P" });
    }
    if posizione >= 4 && !token {
        da_salvare.push(call.clone());
    }
}

fn unpack_standard(bits: &Bits77, i3: u8, h: &CallHashes, da_salvare: &mut Vec<String>) -> Result<String, MessageError> {
    let mut r = Lettore::new(bits);
    let n28a = r.get(28) as u32;
    let ipa = r.get(1);
    let n28b = r.get(28) as u32;
    let ipb = r.get(1);
    let ir = r.get(1);
    let igrid4 = r.get(15) as u32;
    let mut c1 = nominativo(n28a, h)?;
    let mut c2 = nominativo(n28b, h)?;
    if let Some(resto) = c1.strip_prefix("CQ_") {
        c1 = format!("CQ {resto}");
    }
    con_suffisso(&mut c1, ipa, i3, da_salvare);
    con_suffisso(&mut c2, ipb, i3, da_salvare);
    let msg;
    if igrid4 <= MAXGRID4 {
        let grid = grid4_da(igrid4).ok_or_else(|| MessageError::Invalid("locatore non valido".to_string()))?;
        msg = if ir == 0 { format!("{c1} {c2} {grid}") } else { format!("{c1} {c2} R {grid}") };
        if msg.starts_with("CQ ") && ir == 1 {
            return invalid("CQ con R");
        }
        // il locatore non viene mai trasmesso con un primo nominativo in hash e il secondo con /R
        if ir == 0 && c1.starts_with('<') && grid != "RR73" && c2.len() > 1 && c2.ends_with("/R") {
            return invalid("hash con nominativo /R e locatore");
        }
    } else {
        let irpt = igrid4 - MAXGRID4;
        msg = match irpt {
            1 => format!("{c1} {c2}"),
            2 => format!("{c1} {c2} RRR"),
            3 => format!("{c1} {c2} RR73"),
            4 => format!("{c1} {c2} 73"),
            5..=105 => {
                let mut snr = irpt as i32 - 35;
                if snr > 50 {
                    snr -= 101;
                }
                let rpt = formato_rapporto(snr);
                if ir == 0 { format!("{c1} {c2} {rpt}") } else { format!("{c1} {c2} R{rpt}") }
            }
            _ => return invalid("rapporto fuori scala"),
        };
        if msg.starts_with("CQ ") && irpt >= 2 {
            return invalid("CQ con rapporto o RRR/73");
        }
    }
    Ok(msg)
}

fn unpack_rtty(bits: &Bits77, h: &CallHashes) -> Result<String, MessageError> {
    let mut r = Lettore::new(bits);
    let itu = r.get(1);
    let n28a = r.get(28) as u32;
    let n28b = r.get(28) as u32;
    let ir = r.get(1);
    let irpt = r.get(3);
    let nexch = r.get(13) as usize;
    let c1 = nominativo(n28a, h)?;
    let c2 = nominativo(n28b, h)?;
    let crpt = format!("5{}9", irpt + 2);
    let scambio = if nexch > 8000 {
        MOLTIPLICATORI.get(nexch - 8001).map(|m| (*m).to_string())
    } else if (1..=7999).contains(&nexch) {
        Some(format!("{nexch:04}"))
    } else {
        None
    }
    .ok_or_else(|| MessageError::Invalid("scambio RTTY non valido".to_string()))?;
    let tu = if itu == 1 { "TU; " } else { "" };
    let erre = if ir == 1 { "R " } else { "" };
    Ok(format!("{tu}{c1} {c2} {erre}{crpt} {scambio}"))
}

/// Controlli di JTDX su un nominativo del tipo 4: due cifre in testa o forme impossibili a 11 caratteri.
fn nominativo_tipo4_ok(call: &str) -> bool {
    let b = call.as_bytes();
    if b.len() >= 2 && e_cifra(b[0]) && e_cifra(b[1]) {
        return false;
    }
    if b.len() != 11 {
        return true;
    }
    if call.contains(' ') {
        return false;
    }
    let barra = call.find('/').map_or(0, |p| p + 1);
    if matches!(barra, 1 | 2 | 11) {
        return false;
    }
    let ultimo = b[10];
    if barra == 10 && ultimo > b'@' && ultimo < b'[' && ultimo != b'P' {
        return false;
    }
    if barra < 6 && e_cifra(ultimo) {
        return false;
    }
    true
}

fn unpack_non_standard(bits: &Bits77, h: &CallHashes, da_salvare: &mut Vec<String>) -> Result<String, MessageError> {
    let mut r = Lettore::new(bits);
    let n12 = r.get(12) as u32;
    let mut n58 = r.get(58);
    let iflip = r.get(1);
    let nrpt = r.get(2);
    let icq = r.get(1);
    let mut c11 = [b' '; 11];
    for k in (0..11).rev() {
        c11[k] = ALFA38[(n58 % 38) as usize];
        n58 /= 38;
    }
    let c11 = String::from_utf8_lossy(&c11).trim_start().trim_end().to_string();
    let c3 = CallHashes::mostra(h.call12(n12));
    let (call1, call2) = if iflip == 0 { (c3, c11.clone()) } else { (c11.clone(), c3) };
    da_salvare.push(c11);
    let msg = if icq == 0 {
        match nrpt {
            0 => format!("{call1} {call2}"),
            1 => format!("{call1} {call2} RRR"),
            2 => format!("{call1} {call2} RR73"),
            _ => format!("{call1} {call2} 73"),
        }
    } else {
        format!("CQ {call2}")
    };
    let e_cq = msg.starts_with("CQ ");
    if !nominativo_tipo4_ok(&call2) || (!e_cq && !nominativo_tipo4_ok(&call1)) {
        return invalid("nominativo non standard impossibile");
    }
    // messaggi che JTDX non trasmetterebbe mai: finiscono con un hash nudo
    let indice_p = format!("{msg} ").find("/P ").map_or(0, |p| p + 1);
    if ((icq == 0 && nrpt == 0 && !(1..=7).contains(&indice_p)) || icq == 1) && msg.ends_with('>') {
        return invalid("messaggio che finisce con un hash");
    }
    if iflip == 0 && icq == 0 && nrpt == 0 && call2.len() > 9 {
        let b = call2.as_bytes();
        if b[0] == b'/' || b[b.len() - 1] == b'/' || call2.contains(' ') {
            return invalid("nominativo non standard impossibile");
        }
    }
    Ok(msg)
}

fn unpack_eu_vhf(bits: &Bits77, h: &CallHashes) -> Result<String, MessageError> {
    let mut r = Lettore::new(bits);
    let n12 = r.get(12) as u32;
    let n22 = r.get(22) as u32;
    let ir = r.get(1);
    let irpt = r.get(3);
    let iseriale = r.get(11);
    let igrid6 = r.get(25) as u32;
    if igrid6 > 18_662_399 {
        return invalid("locatore a 6 caratteri non valido");
    }
    let c1 = CallHashes::mostra(h.call12(n12));
    let c2 = CallHashes::mostra(h.call22(n22));
    let grid = grid6_da(igrid6).ok_or_else(|| MessageError::Invalid("locatore a 6 caratteri non valido".to_string()))?;
    let scambio = format!("{}{iseriale:04}", 52 + irpt);
    let erre = if ir == 1 { "R " } else { "" };
    Ok(format!("{c1} {c2} {erre}{scambio} {grid}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Da una sequenza di 79 toni di `ft8code` ai 77 bit del messaggio: i toni di dati stanno nel blocco centrale prima del
    /// secondo gruppo di sincronismo, 3 bit per tono (codice Gray `0,1,3,2,5,6,4,7`); il codice e' sistematico, quindi i primi 77
    /// bit del codeword sono il messaggio.
    fn bit_da_toni(simboli: &str) -> Bits77 {
        const INVERSO: [u8; 8] = [0, 1, 3, 2, 6, 4, 5, 7];
        let blocchi: Vec<&str> = simboli.split_whitespace().collect();
        assert_eq!(blocchi.len(), 5, "sequenza di toni malformata");
        assert_eq!(blocchi[1].len(), 29);
        let mut bits = [0u8; 77];
        for (i, t) in blocchi[1].bytes().enumerate() {
            let v = INVERSO[usize::from(t - b'0')];
            for k in 0..3 {
                if let Some(b) = bits.get_mut(i * 3 + k) {
                    *b = (v >> (2 - k)) & 1;
                }
            }
        }
        bits
    }

    fn valore(bits: &[u8]) -> u32 {
        bits.iter().fold(0, |a, &b| (a << 1) | u32::from(b))
    }

    fn giro(testo: &str) -> String {
        let mut h = CallHashes::new();
        let bits = pack(testo, &mut h).unwrap_or_else(|e| panic!("pack({testo}) ha dato {e}"));
        unpack(&bits, &mut h).unwrap_or_else(|e| panic!("unpack({testo}) ha dato {e}"))
    }

    /// I bit di un messaggio di cui `ft8code` da' i toni.
    fn vettore(testo: &str) -> Bits77 {
        let toni = VETTORI.iter().find(|v| v.0 == testo).map(|v| v.1).unwrap_or_else(|| panic!("manca il vettore {testo}"));
        bit_da_toni(toni)
    }

    /// Sostituisce i primi 28 bit (il primo nominativo) con `n28`.
    fn con_n28(mut bits: Bits77, n28: u32) -> Bits77 {
        for (k, b) in bits.iter_mut().take(28).enumerate() {
            *b = ((n28 >> (27 - k)) & 1) as u8;
        }
        bits
    }

    fn tipo(i3: u8, n3: u8) -> Bits77 {
        let mut b = [0u8; 77];
        for k in 0..3 {
            b[71 + k] = (n3 >> (2 - k)) & 1;
            b[74 + k] = (i3 >> (2 - k)) & 1;
        }
        b
    }

    // Vettori: uscite di `ft8code` di WSJT-X, nella forma in cui le riporta il progetto `pengowray/ft8play`
    // (docs/ft8_tests.js, `testInputs_ft8code`; l'ultimo viene dalla documentazione FT4_FT8_QEX). Terza colonna: il tipo
    // (i3, n3) atteso. Il testo e' quello che `ft8code` stampa dopo la decodifica.
    const VETTORI: &[(&str, &str, (u8, u8))] = &[
        ("TNX BOB 73 GL", "3140652 20744714706333640177350001770 3140652 64642730654607244050367013053 3140652", (0, 0)),
        ("K1ABC RR73; W9XYZ <KH1/KH7Z> -08", "3140652 03224752351513326402113431715 3140652 02740707273004136231012725466 3140652", (0, 1)),
        ("K1ABC W9XYZ 6A WI", "3140652 03224752351513326403532040530 3140652 10102016670002655450507772062 3140652", (0, 3)),
        ("W9XYZ K1ABC R 17B EMA", "3140652 02035572501167241620053701303 3140652 33067700140344412531772156322 3140652", (0, 4)),
        ("123456789ABCDEF012", "3140652 11045365753236716724005630431 3140652 62063315364670325657643764734 3140652", (0, 5)),
        ("CQ K1ABC FN42", "3140652 00000000100547670460602153343 3140652 73601104751700733474545513354 3140652", (1, 0)),
        ("K1ABC W9XYZ EN37", "3140652 03224752350406114700513432537 3140652 46455756156477030037617546223 3140652", (1, 0)),
        ("W9XYZ K1ABC -11", "3140652 02035572500547670461746302406 3140652 53631651575170007704437750721 3140652", (1, 0)),
        ("K1ABC W9XYZ R-09", "3140652 03224752350406114702746352703 3140652 32340613021374326763445304061 3140652", (1, 0)),
        ("W9XYZ K1ABC RRR", "3140652 02035572500547670461745553031 3140652 56430553516111752452312775327 3140652", (1, 0)),
        ("K1ABC W9XYZ 73", "3140652 03224752350406114701745602375 3140652 17607411336153312604471562627 3140652", (1, 0)),
        ("K1ABC W9XYZ RR73", "3140652 03224752350406114701742633261 3140652 07130116160034651115122642402 3140652", (1, 0)),
        ("CQ FD K1ABC FN42", "3140652 00000111050547670460602153374 3140652 55174470534654011726436723642 3140652", (1, 0)),
        ("CQ TEST K1ABC/R FN42", "3140652 00040627550547670465602152224 3140652 71213145507156124364617773774 3140652", (1, 0)),
        ("K1ABC/R W9XYZ EN37", "3140652 03224752340406114700513433215 3140652 62370751224150151376024752710 3140652", (1, 0)),
        ("W9XYZ K1ABC/R R FN42", "3140652 02035572500547670464602153406 3140652 44723332345776463750651236762 3140652", (1, 0)),
        ("K1ABC/R W9XYZ RR73", "3140652 03224752340406114701742632543 3140652 21615111232711530254513456331 3140652", (1, 0)),
        ("CQ TEST K1ABC FN42", "3140652 00040627550547670460602152013 3140652 21250156061177140165223103534 3140652", (1, 0)),
        ("W9XYZ <PJ4/K1ABC> -11", "3140652 02035572500163365131746302533 3140652 72170230536772674157704703716 3140652", (1, 0)),
        ("<PJ4/K1ABC> W9XYZ R-09", "3140652 00461340600406114702746352340 3140652 70026642670307536111017334622 3140652", (1, 0)),
        ("CQ W9XYZ EN37", "3140652 00000000100406114700513432702 3140652 52747657056166064010134615661 3140652", (1, 0)),
        ("<YW18FIFA> W9XYZ -11", "3140652 00623063400406114701746302517 3140652 30150124063350453045610770170 3140652", (1, 0)),
        ("W9XYZ <YW18FIFA> R-09", "3140652 02035572500134513652746353567 3140652 66624306126013657212127134512 3140652", (1, 0)),
        ("<YW18FIFA> KA1ABC", "3140652 00623063411370435511745532507 3140652 11234620355321153427122035255 3140652", (1, 0)),
        ("KA1ABC <YW18FIFA> -11", "3140652 56252133050134513651746303556 3140652 74533650035271066027111247354 3140652", (1, 0)),
        ("<YW18FIFA> KA1ABC R-17", "3140652 00623063411370435512746053034 3140652 74604310174542156350005646506 3140652", (1, 0)),
        ("<YW18FIFA> KA1ABC 73", "3140652 00623063411370435511745602026 3140652 67366214515344515710231352751 3140652", (1, 0)),
        ("CQ G4ABC/P IO91", "3140652 00000000100551506545740545627 3140652 31175355577321326610325460211 3140652", (2, 0)),
        ("G4ABC/P PA9XYZ JO22", "3140652 03304034222247341351054655667 3140652 12536520441247353333124433552 3140652", (2, 0)),
        ("PA9XYZ G4ABC/P RR73", "3140652 66726206300551506546742636670 3140652 15517475057750450200643367234 3140652", (2, 0)),
        ("K1ABC W9XYZ 579 WI", "3140652 01167241630406114703772534752 3140652 30651246340340407163645351036 3140652", (3, 0)),
        ("W9XYZ K1ABC R 589 MA", "3140652 01513326400547670467273637070 3140652 55623141267017142221066633172 3140652", (3, 0)),
        ("K1ABC KA0DEF 559 MO", "3140652 01167241621370305261773434421 3140652 53073311535771475412613547162 3140652", (3, 0)),
        ("TU; KA0DEF K1ABC R 569 MA", "3140652 43640510730547670464273634510 3140652 33075230717267321153244675425 3140652", (3, 0)),
        ("KA1ABC G3AAA 529 0013", "3140652 33641561030550731540000234316 3140652 70274723435676575424462342006 3140652", (3, 0)),
        ("TU; G3AAA K1ABC R 559 MA", "3140652 51101452150547670466773637467 3140652 14744315730123530774210116161 3140652", (3, 0)),
        ("CQ KH1/KH7Z", "3140652 15540000031701604265033021440 3140652 24633254146442554247330021155 3140652", (4, 0)),
        ("CQ PJ4/K1ABC", "3140652 36620001607315314363000521041 3140652 66141674641464745632374427542 3140652", (4, 0)),
        ("<W9XYZ> PJ4/K1ABC RRR", "3140652 75410001607315314363000561406 3140652 36120666006707717126111740701 3140652", (4, 0)),
        ("PJ4/K1ABC <W9XYZ> 73", "3140652 75410001607315314363000761140 3140652 31017216621763234100217441572 3140652", (4, 0)),
        ("<W9XYZ> YW18FIFA", "3140652 75410000026470717462032511444 3140652 12630524656764232273327446164 3140652", (4, 0)),
        ("YW18FIFA <W9XYZ> RRR", "3140652 75410000026470717462032460402 3140652 02547175064545617102353316564 3140652", (4, 0)),
        ("<W9XYZ> YW18FIFA 73", "3140652 75410000026470717462032660144 3140652 07650725643521134124055215735 3140652", (4, 0)),
        ("CQ YW18FIFA", "3140652 12410000026470717462032520503 3140652 43235636455104172263345306357 3140652", (4, 0)),
        ("<KA1ABC> YW18FIFA RR73", "3140652 12320000026470717462032610755 3140652 73041016005003413426660204571 3140652", (4, 0)),
        ("<G4ABC> <PA9XYZ> R 570007 JO22DB", "3140652 13665172067730026162514322642 3140652 15503311223566473014467745175 3140652", (5, 0)),
    ];

    #[test]
    fn vettori_ft8code_pack() {
        for (testo, toni, tipo) in VETTORI {
            let atteso = bit_da_toni(toni);
            let mut h = CallHashes::new();
            let bits = pack(testo, &mut h).unwrap_or_else(|e| panic!("pack({testo}) ha dato {e}"));
            assert_eq!(bits, atteso, "bit diversi da ft8code per {testo}");
            assert_eq!(message_type(&bits), *tipo, "tipo sbagliato per {testo}");
        }
    }

    #[test]
    fn vettori_ft8code_unpack() {
        for (testo, toni, _) in VETTORI {
            let bits = bit_da_toni(toni);
            let mut h = CallHashes::new();
            // gli hash si imparano come in ricezione: prima i nominativi, poi il messaggio che li cita
            for c in ["PJ4/K1ABC", "W9XYZ", "YW18FIFA", "KA1ABC", "KH1/KH7Z", "G4ABC", "PA9XYZ"] {
                h.save_call(c);
            }
            assert_eq!(unpack(&bits, &mut h).as_deref(), Ok(*testo), "unpack di {testo}");
        }
    }

    #[test]
    fn hash_sconosciuto_si_legge_come_puntini() {
        // "W9XYZ <PJ4/K1ABC> -11" da ft8code, ma senza aver mai visto PJ4/K1ABC
        let bits = vettore("W9XYZ <PJ4/K1ABC> -11");
        let mut h = CallHashes::new();
        assert_eq!(unpack(&bits, &mut h).as_deref(), Ok("W9XYZ <...> -11"));
        // il tipo 4 con hash sconosciuto (h ha pero' imparato W9XYZ dal messaggio appena letto: ne serve uno nuovo)
        let bits = vettore("<W9XYZ> PJ4/K1ABC RRR");
        let mut nuovo = CallHashes::new();
        assert_eq!(unpack(&bits, &mut nuovo).as_deref(), Ok("<...> PJ4/K1ABC RRR"));
        // chi ha visto W9XYZ in chiaro ora legge il messaggio completo
        nuovo.save_call("W9XYZ");
        assert_eq!(unpack(&bits, &mut nuovo).as_deref(), Ok("<W9XYZ> PJ4/K1ABC RRR"));
        // e lo stesso se W9XYZ lo ha imparato da un messaggio decodificato prima
        assert_eq!(unpack(&bits, &mut h).as_deref(), Ok("<W9XYZ> PJ4/K1ABC RRR"));
    }

    #[test]
    fn tipo4_con_hash_nudo_finale_e_scartato_da_jtdx() {
        // "PJ4/K1ABC <W9XYZ>" (nrpt = 0, hash in coda) esce da ft8code, ma il filtro di unpack77 di JTDX lo rifiuta
        let toni = "3140652 75410001607315314363000410440 3140652 26077017614526132255145210301 3140652";
        let bits = bit_da_toni(toni);
        let mut h = CallHashes::new();
        h.save_call("W9XYZ");
        assert!(matches!(unpack(&bits, &mut h), Err(MessageError::Invalid(_))));
        // pack invece lo produce identico a ft8code
        assert_eq!(pack("PJ4/K1ABC <W9XYZ>", &mut CallHashes::new()), Ok(bits));
    }

    #[test]
    fn hash_di_ft8code() {
        // i primi 12 bit di "<W9XYZ> PJ4/K1ABC RRR" sono l'hash a 12 bit di W9XYZ secondo ft8code
        let bits = vettore("<W9XYZ> PJ4/K1ABC RRR");
        assert_eq!(valore(&bits[..12]), hash_call("W9XYZ", 12));
        // i primi 10 bit di "K1ABC RR73; W9XYZ <KH1/KH7Z> -08" dopo i due nominativi: hash a 10 bit di KH1/KH7Z
        let bits = vettore("K1ABC RR73; W9XYZ <KH1/KH7Z> -08");
        assert_eq!(valore(&bits[56..66]), hash_call("KH1/KH7Z", 10));
        // l'hash a 22 bit di <YW18FIFA> sta nei 28 bit del primo nominativo, dopo NTOKENS
        let bits = vettore("<YW18FIFA> W9XYZ -11");
        assert_eq!(valore(&bits[..28]), NTOKENS + hash_call("YW18FIFA", 22));
        // dieci e dodici bit sono i bit alti dello stesso prodotto a 22
        for c in ["W9XYZ", "PJ4/K1ABC", "YW18FIFA", "KH1/KH7Z", "G4ABC/P"] {
            assert_eq!(hash_call(c, 10), hash_call(c, 22) >> 12);
            assert_eq!(hash_call(c, 12), hash_call(c, 22) >> 10);
            assert!(hash_call(c, 22) < MAX22);
        }
        // la chiave e' sempre di 11 caratteri, gli spazi finali non contano
        assert_eq!(hash_call("W9XYZ", 22), hash_call("W9XYZ      ", 22));
    }

    #[test]
    fn giro_messaggi_reali() {
        let casi = [
            "CQ K1ABC FN42",
            "CQ DX IU3ABC IO91",
            "CQ POTA IU3ABC JN55",
            "CQ SOTA K1ABC/R FN42",
            "CQ 123 K1ABC FN42",
            "CQ IU3ABC",
            "CQ IU3ABC JN55",
            "QRZ K1ABC FN42",
            "DE K1ABC FN42",
            "IU3ABC K1ABC -15",
            "K1ABC IU3ABC R-07",
            "IU3ABC K1ABC RR73",
            "IU3ABC K1ABC RRR",
            "IU3ABC K1ABC 73",
            "K1ABC IU3ABC",
            "K1ABC IU3ABC JN55",
            "K1ABC IU3ABC R JN55",
            "IU3ABC K1ABC +00",
            "IU3ABC K1ABC -30",
            "IU3ABC K1ABC +50",
            "IU3ABC K1ABC -50",
            "IU3ABC K1ABC R-31",
            "IU3ABC K1ABC R+49",
            "G4ABC/P PA9XYZ/P JO22",
            "IU3ABC/P K1ABC R JO22",
            "IU3ABC K1ABC/P 73",
            "CQ EA8/G5LSI",
            "<IU3ABC> EA8/G5LSI RR73",
            "EA8/G5LSI <IU3ABC> 73",
            "<IU3ABC> K1ABC -10",
            "K1ABC <IU3ABC> R-10",
            "IU3ABC/R <K1ABC> 73",
            "TNX BOB 73 GL",
            "HELLO WORLD",
            "IU3ABC K1/QRP",
            "A",
            "CQ",
            "73",
            "5W+0.5/?",
            "K1ABC W9XYZ 6A WI",
            "W9XYZ K1ABC R 32A EMA",
            "K1ABC KA0DEF 559 MO",
            "TU; KA0DEF K1ABC R 569 MA",
            "KA1ABC G3AAA 529 7999",
            "K1ABC RR73; W9XYZ <KH1/KH7Z> -08",
            "1005E1EAFCAFEC0FFE",
            "7FFFFFFFFFFFFFFFFF",
            "<G4ABC> <PA9XYZ> R 570007 JO22DB",
            "<G4ABC> <PA9XYZ> 520001 AA00AA",
        ];
        for c in casi {
            assert_eq!(giro(c), c, "giro di {c}");
        }
    }

    #[test]
    fn minuscole_e_blank_ripetuti() {
        let mut h = CallHashes::new();
        let a = pack("cq   dx  iu3abc   io91", &mut h);
        let b = pack("CQ DX IU3ABC IO91", &mut h);
        assert_eq!(a, b);
        assert!(a.is_ok());
    }

    #[test]
    fn combinazioni_come_la_prova_di_ft8_lib() {
        // Stessa idea di `test/test.c` di ft8_lib (kgoba): ogni coppia di nominativi con ogni coda, ogni token con ogni
        // nominativo. Qui solo con nominativi che JTDX tratta da standard.
        let calls = ["YL3JG", "W1A", "W1A/R", "W5AB", "W8ABC", "DE6ABC", "DE6ABC/R", "DE7AB", "DE9A", "3Y0X", "A12BC"];
        let tokens = ["CQ", "QRZ", "DE", "CQ 123", "CQ 000", "CQ POTA", "CQ SA", "CQ O", "CQ ASD", "CQ ABCD"];
        let locatori = ["KO26", "RR99", "AA00", "RR09", "AA01", "RR73"];
        let code = [
            "KO26", "RR99", "AA00", "RR09", "AA01", "RRR", "RR73", "73", "R+10", "R+05", "R-12", "R-02", "+10", "+05", "-02", "-50", "+50", "",
        ];
        for coda in code {
            for c1 in calls {
                for c2 in calls {
                    // a due parole JTDX non accetta /R sul secondo nominativo: sarebbe testo libero, troppo lungo
                    if coda.is_empty() && c2.contains('/') && c1.len() + c2.len() > 12 {
                        continue;
                    }
                    let t = format!("{c1} {c2} {coda}").trim().to_string();
                    assert_eq!(giro(&t), t);
                }
            }
            for tok in tokens {
                // un CQ porta solo il locatore, o niente
                if tok.starts_with("CQ") && !coda.is_empty() && !locatori.contains(&coda) {
                    continue;
                }
                for c2 in calls {
                    if coda.is_empty() && c2.contains('/') && tok.len() + c2.len() > 12 {
                        continue;
                    }
                    let t = format!("{tok} {c2} {coda}").trim().to_string();
                    assert_eq!(giro(&t), t);
                }
            }
        }
    }

    #[test]
    fn tipo_dei_messaggi_di_ft8_lib() {
        // (messaggio, i3): vengono da test.c di ft8_lib, dove si controlla il tipo scelto dal codificatore
        let casi = [("CQ K7IHZ DM43", 1), ("CQ EA8/G5LSI", 4), ("CQ JA LB2JK JO59", 1), ("CQ 123 LB2JK JO59", 1), ("TNX BOB 73 GL", 0)];
        for (testo, i3) in casi {
            let mut h = CallHashes::new();
            let bits = pack(testo, &mut h).unwrap_or_else(|e| panic!("{testo}: {e}"));
            assert_eq!(message_type(&bits).0, i3, "{testo}");
            assert_eq!(unpack(&bits, &mut h).as_deref(), Ok(testo));
        }
        // CQ YL/LB2JK KO16SW: il locatore non entra nel tipo 4 (anche ft8_lib lo scarta)
        let mut h = CallHashes::new();
        let bits = pack("CQ YL/LB2JK KO16SW", &mut h).unwrap_or([9; 77]);
        assert_eq!(message_type(&bits).0, 4);
        assert_eq!(unpack(&bits, &mut h).as_deref(), Ok("CQ YL/LB2JK"));
    }

    #[test]
    fn nominativi_non_standard_con_hash() {
        let mut h = CallHashes::new();
        let bits = pack("<IU3ABC> PJ4/K1ABC RR73", &mut h).unwrap_or([9; 77]);
        assert_eq!(message_type(&bits), (4, 0));
        assert_eq!(unpack(&bits, &mut h).as_deref(), Ok("<IU3ABC> PJ4/K1ABC RR73"));
        // chi riceve senza aver mai visto IU3ABC legge puntini
        let mut vuoto = CallHashes::new();
        assert_eq!(unpack(&bits, &mut vuoto).as_deref(), Ok("<...> PJ4/K1ABC RR73"));
        // e dopo aver letto un CQ di IU3ABC il messaggio si completa
        let cq = pack("CQ IU3ABC JN55", &mut CallHashes::new()).unwrap_or([9; 77]);
        assert_eq!(unpack(&cq, &mut vuoto).as_deref(), Ok("CQ IU3ABC JN55"));
        assert_eq!(unpack(&bits, &mut vuoto).as_deref(), Ok("<IU3ABC> PJ4/K1ABC RR73"));
        // il tipo 1 con un hash a 22 bit funziona allo stesso modo
        let t1 = pack("<IU3ABC> K1ABC -15", &mut CallHashes::new()).unwrap_or([9; 77]);
        assert_eq!(unpack(&t1, &mut vuoto).as_deref(), Ok("<IU3ABC> K1ABC -15"));
        assert_eq!(unpack(&t1, &mut CallHashes::new()).as_deref(), Ok("<...> K1ABC -15"));
    }

    #[test]
    fn tabella_a_22_bit_tiene_i_piu_recenti() {
        let mut h = CallHashes::new();
        for i in 0..1200 {
            h.save_call(&format!("K{i}ABC"));
        }
        assert_eq!(h.len(), MAXHASH);
        h.save_call("IU3ABC");
        assert_eq!(h.call22(hash_call("IU3ABC", 22)), Some("IU3ABC"));
        assert_eq!(h.call12(hash_call("IU3ABC", 12)), Some("IU3ABC"));
        assert_eq!(h.call10(hash_call("IU3ABC", 10)), Some("IU3ABC"));
        // ignorati: troppo corti, <...>, vuoti
        let prima = h.len();
        h.save_call("AB");
        h.save_call("<...>");
        h.save_call("");
        assert_eq!(h.len(), prima);
        // tra <> si salva lo stesso
        h.save_call("<PJ4/K1ABC>");
        assert_eq!(h.call22(hash_call("PJ4/K1ABC", 22)), Some("PJ4/K1ABC"));
    }

    #[test]
    fn campi_estremi() {
        // locatori ai bordi, serie e classi
        for t in ["K1ABC W9XYZ AA00", "K1ABC W9XYZ RR99", "K1ABC W9XYZ RR73", "CQ AAAA K1ABC AA00", "CQ ZZ K1ABC AA00"] {
            assert_eq!(giro(t), t);
        }
        for t in ["K1ABC W9XYZ 1A DX", "K1ABC W9XYZ R 32F WY", "K1ABC W9XYZ 16A AB", "K1ABC W9XYZ 17A PE"] {
            assert_eq!(giro(t), t);
        }
        for t in ["K1ABC W9XYZ 529 AL", "K1ABC W9XYZ R 599 DC", "K1ABC W9XYZ 599 PEI", "TU; K1ABC W9XYZ R 539 0001"] {
            assert_eq!(giro(t), t);
        }
    }

    #[test]
    fn errori_di_pack() {
        let mut h = CallHashes::new();
        // troppo lungo come testo libero
        assert!(matches!(pack("QUESTO TESTO E TROPPO LUNGO", &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(pack("ABCDEFGHIJKLMN", &mut h), Err(MessageError::Invalid(_))));
        // oltre 37 caratteri in totale
        assert!(matches!(pack("A B C D E F G H I J K L M N O P Q R S T U V", &mut h), Err(MessageError::Invalid(_))));
        // caratteri fuori alfabeto
        assert!(matches!(pack("HELLO #1", &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(pack("CIAO @", &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(pack("PERCHE\u{e9}", &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(pack("CQ <...> JN55", &mut h), Err(MessageError::Invalid(_))));
        // vuoto
        assert!(matches!(pack("", &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(pack("   ", &mut h), Err(MessageError::Invalid(_))));
        // la tabella non cambia se il messaggio non si impacchetta
        assert!(h.is_empty());
    }

    #[test]
    fn niente_nominativi_storpiati() {
        // il Fortran scriverebbe K1ABC senza /QRP e senza PJ4/: qui diventano testo libero (se ci stanno) o errore
        let mut h = CallHashes::new();
        assert!(matches!(pack("IU3ABC K1ABC/QRP RR73", &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(pack("W9XYZ PJ4/K1ABC 73", &mut h), Err(MessageError::Invalid(_))));
        let bits = pack("IU3ABC K1/QRP", &mut h).unwrap_or([9; 77]);
        assert_eq!(message_type(&bits), (0, 0));
        // R e P insieme, rapporto fuori scala, CQ con rapporto
        assert!(pack("K1ABC/R W9XYZ/P 73", &mut h).is_err());
        assert!(pack("K1ABC W9XYZ +99", &mut h).is_err());
        // un CQ con rapporto o RRR non e' un messaggio standard: resta testo libero (12 caratteri)
        assert_eq!(message_type(&pack("CQ K1ABC -10", &mut h).unwrap_or([9; 77])), (0, 0));
        assert_eq!(message_type(&pack("CQ K1ABC RRR", &mut h).unwrap_or([9; 77])), (0, 0));
        assert!(h.is_empty());
    }

    #[test]
    fn errori_di_unpack() {
        let mut h = CallHashes::new();
        // bit non validi
        let mut bits = [0u8; 77];
        bits[10] = 2;
        assert!(matches!(unpack(&bits, &mut h), Err(MessageError::Invalid(_))));
        bits[10] = 255;
        assert!(unpack(&bits, &mut h).is_err());
        // tutti zero: testo libero vuoto
        assert!(matches!(unpack(&[0u8; 77], &mut h), Err(MessageError::Invalid(_))));
        // tutti uno: i3 = 7
        assert!(matches!(unpack(&[1u8; 77], &mut h), Err(MessageError::Invalid(_))));
        // tipi non definiti: i3 = 6, n3 = 2 e 7, e WSPR (0.6) non gestito
        assert!(matches!(unpack(&tipo(6, 0), &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(unpack(&tipo(0, 2), &mut h), Err(MessageError::Invalid(_))));
        assert!(matches!(unpack(&tipo(0, 7), &mut h), Err(MessageError::Invalid(_))));
        assert_eq!(unpack(&tipo(0, 6), &mut h), Err(MessageError::Unsupported));
        // tipo 1 con un numero riservato tra i CQ e i nominativi (n28 = 532444)
        assert!(unpack(&con_n28(vettore("K1ABC W9XYZ EN37"), 532_444), &mut h).is_err());
        // CQ con rapporto: il primo nominativo diventa CQ (n28 = 2) in "W9XYZ K1ABC -11"
        assert!(unpack(&con_n28(vettore("W9XYZ K1ABC -11"), 2), &mut h).is_err());
        // tabella intatta dopo i rifiuti
        assert!(h.is_empty());
    }

    #[test]
    fn display_e_errore() {
        assert_eq!(MessageError::Unsupported.to_string(), "tipo di messaggio non gestito");
        assert!(MessageError::Invalid("x".into()).to_string().contains('x'));
    }
}
