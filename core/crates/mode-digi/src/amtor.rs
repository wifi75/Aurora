//! Amtor FEC (SITOR-B, modo collettivo B): FSK a 100 baud con spostamento di 170 Hz, codice CCIR 476 (ITU-R M.476/M.625) a 7 bit con
//! quattro «uni» e tre «zeri» (rapporto 4:3), ogni carattere mandato due volte a cinque caratteri di distanza.
//!
//! Le convenzioni sono quelle di fldigi 4.2.13 (`src/navtex/navtex.cxx`, modo `SITORB`, Dave Freese W1HKJ e contributori; GPL 3, copia in
//! `terze-parti/fldigi`), studiate e riscritte qui; da fldigi vengono le tabelle dei codici:
//! - il bit 1 e' il mark, il tono alto (centro + 85 Hz) in banda laterale superiore; i 7 bit partono dal meno significativo;
//! - i caratteri vanno a coppie: la prima posizione (DX) porta il carattere nuovo, la seconda (RX) quello di due coppie prima, cosi' la
//!   ripetizione arriva 35 bit (280 ms) dopo la prima copia;
//! - all'inizio la fasatura: coppie «ripetizione» (0x66) in DX e «alfa» (0x0F) in RX; le posizioni RX ancora vuote portano alfa; quando
//!   non c'e' niente da mandare si rimanda la fasatura;
//! - lettere e cifre con LTRS (0x5A) e FIGS (0x36); le cifre sono quelle americane (come l'ITA2 dell'RTTY di Aurora e come fldigi di base).
//!
//! Solo il modo FEC: il modo ARQ (A) e' un collegamento a due con risposte entro pochi millisecondi, non realizzabile in sicurezza qui.
//!
//! Il ricevitore e' nostro: due rivelatori (mark e space) integrati su un bit, sincronismo di bit sul punto con la differenza piu' netta,
//! sincronismo dei caratteri sulla posizione dove le due copie coincidono, scelta fra le due copie con il controllo dei quattro uni e la
//! somma «soft» delle due, uscita solo agganciati (nel rumore niente).

use std::collections::VecDeque;
use std::f64::consts::PI;

/// Velocita' (baud) e meta' spostamento (Hz).
pub const BAUD: f64 = 100.0;
pub const DEVIATION_HZ: f64 = 85.0;

const CODE_LTRS: u8 = 0x5a;
const CODE_FIGS: u8 = 0x36;
const CODE_ALPHA: u8 = 0x0f;
const CODE_BETA: u8 = 0x33;
const CODE_CHAR32: u8 = 0x6a;
const CODE_REP: u8 = 0x66;

/// Le lettere di ogni codice valido (0 = nessuna).
const LTRS: [(u8, u8); 32] = [
    (0x17, b'J'), (0x1b, b'F'), (0x1d, b'C'), (0x1e, b'K'), (0x27, b'W'), (0x2b, b'Y'), (0x2d, b'P'), (0x2e, b'Q'),
    (0x35, b'G'), (0x39, b'M'), (0x3a, b'X'), (0x3c, b'V'), (0x47, b'A'), (0x4b, b'S'), (0x4d, b'I'), (0x4e, b'U'),
    (0x53, b'D'), (0x55, b'R'), (0x56, b'E'), (0x59, b'N'), (0x5c, b' '), (0x63, b'Z'), (0x65, b'L'), (0x69, b'H'),
    (0x6c, b'\n'), (0x71, b'O'), (0x72, b'B'), (0x74, b'T'), (0x78, b'\r'), (0, 0), (0, 0), (0, 0),
];
/// Le cifre (americane) di ogni codice valido; 0x4B e' il campanello.
const FIGS: [(u8, u8); 29] = [
    (0x17, b'\''), (0x1b, b'!'), (0x1d, b':'), (0x1e, b'('), (0x27, b'2'), (0x2b, b'6'), (0x2d, b'0'), (0x2e, b'1'),
    (0x35, b'&'), (0x39, b'.'), (0x3a, b'/'), (0x3c, b';'), (0x47, b'-'), (0x4b, 7), (0x4d, b'8'), (0x4e, b'7'),
    (0x53, b'$'), (0x55, b'4'), (0x56, b'3'), (0x59, b','), (0x5c, b' '), (0x63, b'"'), (0x65, b')'), (0x69, b'#'),
    (0x6c, b'\n'), (0x71, b'9'), (0x72, b'?'), (0x74, b'5'), (0x78, b'\r'),
];

fn valid(code: u8) -> bool {
    code < 128 && code.count_ones() == 4
}

fn letter(code: u8) -> Option<u8> {
    LTRS.iter().find(|(c, _)| *c == code && *c != 0).map(|(_, ch)| *ch)
}

fn figure(code: u8) -> Option<u8> {
    FIGS.iter().find(|(c, _)| *c == code).map(|(_, ch)| *ch)
}

/// I codici di un carattere con il registro `figs` (lettere o cifre) che cambia se serve; un carattere che il codice non ha non si manda.
fn encode(c: u8, figs: &mut bool) -> Vec<u8> {
    let c = c.to_ascii_uppercase();
    let in_ltrs = LTRS.iter().find(|(code, ch)| *code != 0 && *ch == c).map(|(code, _)| *code);
    let in_figs = FIGS.iter().find(|(code, ch)| *ch == c && *code != 0x4b).map(|(code, _)| *code);
    match (*figs, in_ltrs, in_figs) {
        (true, _, Some(code)) | (false, Some(code), _) => vec![code],
        (false, None, Some(code)) => {
            *figs = true;
            vec![CODE_FIGS, code]
        }
        (true, Some(code), None) => {
            *figs = false;
            vec![CODE_LTRS, code]
        }
        _ => Vec::new(),
    }
}

/// Il trasmettitore a flusso: coppie DX/RX, fasatura in attesa, FSK a fase continua.
pub struct AmtorTx {
    rate: f64,
    centre_hz: f64,
    reverse: bool,
    amp: f64,
    phase: f64,
    /// Campioni per bit (frazionari: si accumula l'errore).
    spb: f64,
    acc: f64,
    figs: bool,
    /// I codici gia' mandati in DX che aspettano la posizione RX (due coppie dopo).
    rx_queue: VecDeque<u8>,
}

impl AmtorTx {
    /// `centre_hz`: il centro fra i due toni; `reverse`: mark e space scambiati (radio in LSB).
    pub fn new(rate: f64, centre_hz: f64, amp: f64, reverse: bool) -> Self {
        AmtorTx {
            rate,
            centre_hz,
            reverse,
            amp,
            phase: 0.0,
            spb: rate / BAUD,
            acc: 0.0,
            figs: false,
            rx_queue: VecDeque::from([CODE_ALPHA, CODE_ALPHA]),
        }
    }

    fn bits(&mut self, code: u8, out: &mut Vec<f32>) {
        for i in 0..7 {
            let mark = (code >> i) & 1 == 1;
            let f = self.centre_hz + if mark != self.reverse { DEVIATION_HZ } else { -DEVIATION_HZ };
            let step = 2.0 * PI * f / self.rate;
            self.acc += self.spb;
            let n = self.acc.floor();
            self.acc -= n;
            for _ in 0..n as usize {
                self.phase = (self.phase + step) % (2.0 * PI);
                out.push((self.amp * self.phase.sin()) as f32);
            }
        }
    }

    /// Una coppia: `dx` nuovo in DX (e in coda per RX), in RX il codice di due coppie prima. `next_rx`: cosa mettere in coda per RX.
    fn pair(&mut self, dx: u8, next_rx: u8, out: &mut Vec<f32>) {
        let rx = self.rx_queue.pop_front().unwrap_or(CODE_ALPHA);
        self.rx_queue.push_back(next_rx);
        self.bits(dx, out);
        self.bits(rx, out);
    }

    /// La fasatura d'inizio: 16 coppie (2,24 s), poi LTRS perche' chi riceve parta dalle lettere.
    pub fn preamble(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..16 {
            self.pair(CODE_REP, CODE_ALPHA, &mut out);
        }
        self.figs = false;
        self.pair(CODE_LTRS, CODE_LTRS, &mut out);
        out
    }

    pub fn char(&mut self, c: u8) -> Vec<f32> {
        let mut out = Vec::new();
        for code in encode(c, &mut self.figs) {
            self.pair(code, code, &mut out);
        }
        out
    }

    /// Niente da mandare: una coppia di fasatura (le ripetizioni in coda continuano a uscire).
    pub fn idle(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        self.pair(CODE_REP, CODE_ALPHA, &mut out);
        out
    }

    /// La fine: le ultime ripetizioni (con 0x6A in DX, come fldigi) e tre coppie di alfa.
    pub fn postamble(&mut self) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..2 {
            self.pair(CODE_CHAR32, CODE_ALPHA, &mut out);
        }
        for _ in 0..3 {
            self.pair(CODE_ALPHA, CODE_ALPHA, &mut out);
        }
        out
    }

    /// Tutto il testo in un colpo solo.
    pub fn transmit(&mut self, text: &[u8]) -> Vec<f32> {
        let mut out = self.preamble();
        for &c in text {
            out.extend(self.char(c));
        }
        out.extend(self.postamble());
        out
    }
}

/// Le coppie buone (le due copie uguali e valide, o la fasatura) che servono per agganciarsi, sulle ultime sette.
const SYNC_GOOD: usize = 5;
/// Bit tenuti per il sincronismo dei caratteri.
const HISTORY: usize = 140;

/// Un rivelatore: miscela a una frequenza e somma su un bit.
struct Tone {
    phase: f64,
    ring: Vec<(f64, f64)>,
    pos: usize,
    sum: (f64, f64),
    prev: (f64, f64),
}

impl Tone {
    fn new(len: usize) -> Self {
        Tone { phase: 0.0, ring: vec![(0.0, 0.0); len], pos: 0, sum: (0.0, 0.0), prev: (0.0, 0.0) }
    }

    fn push(&mut self, x: f64, step: f64) -> f64 {
        let (s, c) = self.phase.sin_cos();
        self.phase = (self.phase + step) % (2.0 * PI);
        let z = (x * c, -x * s);
        let old = self.ring[self.pos];
        self.ring[self.pos] = z;
        self.pos = (self.pos + 1) % self.ring.len();
        self.prev = self.sum;
        self.sum.0 += z.0 - old.0;
        self.sum.1 += z.1 - old.1;
        self.sum.0.hypot(self.sum.1)
    }

    /// La rotazione di fase fra l'ultimo campione e quello prima (Hz).
    fn drift_hz(&self, rate: f64) -> f64 {
        let (p, q) = (self.prev, self.sum);
        (p.0 * q.1 - p.1 * q.0).atan2(p.0 * q.0 + p.1 * q.1) * rate / (2.0 * PI)
    }
}

/// Il ricevitore.
pub struct AmtorRx {
    rate: f64,
    reverse: bool,
    base_hz: f64,
    centre_hz: f64,
    afc: bool,
    afc_limit_hz: f64,
    mark: Tone,
    space: Tone,
    spb: usize,
    timing: Vec<f64>,
    count: usize,
    countdown: usize,
    /// I bit «soft» (positivo = mark) e quanti ne sono arrivati in tutto.
    bits: VecDeque<f32>,
    total: usize,
    /// Agganciato: il numero assoluto del bit dove finisce la prossima copia RX da leggere.
    next_rx_end: Option<usize>,
    errors: i32,
    figs: bool,
    pending: Vec<u8>,
    out: Vec<u8>,
    level_db: f32,
    quality: f64,
}

impl AmtorRx {
    pub fn new(rate: f64, centre_hz: f64, reverse: bool, afc: bool) -> Self {
        let spb = (rate / BAUD).round() as usize;
        AmtorRx {
            rate,
            reverse,
            base_hz: centre_hz,
            centre_hz,
            afc,
            afc_limit_hz: AFC_LIMIT_HZ,
            mark: Tone::new(spb),
            space: Tone::new(spb),
            spb,
            timing: vec![0.0; spb],
            count: 0,
            countdown: spb,
            bits: VecDeque::new(),
            total: 0,
            next_rx_end: None,
            errors: 0,
            figs: false,
            pending: Vec::new(),
            out: Vec::new(),
            level_db: -90.0,
            quality: 0.0,
        }
    }

    pub fn process(&mut self, audio: &[f32]) {
        if !audio.is_empty() {
            let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
            self.level_db = (20.0 * rms.max(1e-5).log10()).max(-90.0);
        }
        for &x in audio {
            let x = f64::from(x);
            let hi = 2.0 * PI * (self.centre_hz + DEVIATION_HZ) / self.rate;
            let lo = 2.0 * PI * (self.centre_hz - DEVIATION_HZ) / self.rate;
            let m = self.mark.push(x, hi);
            let s = self.space.push(x, lo);
            let (m, s) = if self.reverse { (s, m) } else { (m, s) };
            let d = (m - s) / (m + s + 1e-12);
            let slot = self.count % self.spb;
            self.timing[slot] = 0.9 * self.timing[slot] + 0.1 * d.abs();
            self.count += 1;
            self.countdown = self.countdown.saturating_sub(1);
            if self.countdown == 0 {
                let best = self.timing.iter().enumerate().fold((0, f64::MIN), |b, (i, &v)| if v > b.1 { (i, v) } else { b }).0;
                let len = self.spb as isize;
                let dist = ((best + self.spb + self.spb / 2 - slot) % self.spb) as isize - len / 2;
                self.countdown = (len + (dist / 2).clamp(-len / 8, len / 8)) as usize;
                self.bit(d as f32);
            }
        }
    }

    fn bit(&mut self, soft: f32) {
        if self.afc && self.next_rx_end.is_some() && soft.abs() > 0.5 {
            // la rotazione di fase del tono che ha vinto il bit
            let tone_is_hi = (soft > 0.0) != self.reverse;
            let df = if tone_is_hi { self.mark.drift_hz(self.rate) } else { self.space.drift_hz(self.rate) };
            let next = self.centre_hz + 0.02 * df;
            self.centre_hz = next.clamp(self.base_hz - self.afc_limit_hz, self.base_hz + self.afc_limit_hz);
        }
        self.bits.push_back(soft);
        self.total += 1;
        if self.bits.len() > HISTORY {
            self.bits.pop_front();
        }
        match self.next_rx_end {
            None => self.find_sync(),
            Some(end) if self.total >= end => {
                self.read_pair(end);
                self.next_rx_end = Some(end + 14);
            }
            Some(_) => {}
        }
    }

    /// Il codice di 7 bit che finisce al bit assoluto `end` (e i suoi valori soft), se e' ancora nella memoria.
    fn soft_at(&self, end: usize) -> Option<[f32; 7]> {
        let first = self.total - self.bits.len();
        if end < first + 7 || end > self.total {
            return None;
        }
        let start = end - 7 - first;
        let mut v = [0.0; 7];
        for (i, x) in v.iter_mut().enumerate() {
            *x = self.bits[start + i];
        }
        Some(v)
    }

    fn code(v: &[f32; 7]) -> u8 {
        v.iter().enumerate().fold(0, |c, (i, &x)| if x > 0.0 { c | (1 << i) } else { c })
    }

    /// La coppia (copia RX che finisce a `end`, copia DX 35 bit prima) e' buona: uguali e valide, o fasatura.
    fn good_pair(&self, end: usize) -> bool {
        let (Some(rx), Some(dx)) = (self.soft_at(end), self.soft_at(end - 35)) else { return false };
        let (r, d) = (Self::code(&rx), Self::code(&dx));
        (valid(r) && r == d) || (r == CODE_ALPHA && d == CODE_REP)
    }

    fn find_sync(&mut self) {
        if self.bits.len() < HISTORY {
            return;
        }
        // le copie RX finiscono ogni 14 bit: si prova ogni posizione e si tiene quella con piu' coppie buone
        let mut best = (0, 0usize);
        for off in 0..14 {
            let last = self.total - off;
            let good = (0..7).filter(|k| last >= 35 + 7 + 14 * k && self.good_pair(last - 14 * k)).count();
            if good > best.1 {
                best = (off, good);
            }
        }
        if best.1 >= SYNC_GOOD {
            let last = self.total - best.0;
            self.errors = 0;
            self.figs = false;
            self.pending.clear();
            self.next_rx_end = Some(last + 14);
        }
    }

    fn read_pair(&mut self, end: usize) {
        let (Some(rx), Some(dx)) = (self.soft_at(end), self.soft_at(end - 35)) else {
            self.lose_sync();
            return;
        };
        let (r, d) = (Self::code(&rx), Self::code(&dx));
        let both = valid(r) && r == d;
        let code = if both || valid(r) {
            Some(r)
        } else if valid(d) {
            Some(d)
        } else {
            // somma soft delle due copie; con tre o cinque uni si gira il bit meno sicuro
            let mut sum = [0.0_f32; 7];
            for i in 0..7 {
                sum[i] = rx[i] + dx[i];
            }
            let mut c = Self::code(&sum);
            let ones = c.count_ones();
            if ones == 3 || ones == 5 {
                let want_flip_one = ones == 5;
                let pick = (0..7).filter(|&i| ((c >> i) & 1 == 1) == want_flip_one).min_by(|&a, &b| sum[a].abs().total_cmp(&sum[b].abs()));
                if let Some(i) = pick {
                    c ^= 1 << i;
                }
            }
            valid(c).then_some(c)
        };
        let phasing = r == CODE_ALPHA && d == CODE_REP;
        let good = both || phasing;
        self.errors += match (good, code.is_some()) {
            (true, _) => -2,
            (false, true) => 1,
            (false, false) => 3,
        };
        self.errors = self.errors.max(0);
        self.quality = 0.8 * self.quality + 0.2 * if good { 1.0 } else { 0.0 };
        if self.errors >= 8 {
            self.lose_sync();
            return;
        }
        if let Some(c) = code {
            self.symbol(c);
        }
        if good {
            self.out.append(&mut self.pending);
        }
    }

    fn lose_sync(&mut self) {
        self.next_rx_end = None;
        self.pending.clear();
        self.errors = 0;
    }

    fn symbol(&mut self, code: u8) {
        match code {
            CODE_REP | CODE_ALPHA | CODE_BETA | CODE_CHAR32 => {}
            CODE_LTRS => self.figs = false,
            CODE_FIGS => self.figs = true,
            _ => {
                let ch = if self.figs { figure(code) } else { letter(code) };
                // CR e campanello non si stampano
                if let Some(ch) = ch.filter(|&c| c != b'\r' && c != 7) {
                    self.pending.push(ch);
                }
            }
        }
    }

    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    pub fn frequency(&self) -> f64 {
        self.centre_hz
    }

    /// Agganciato ai caratteri.
    pub fn locked(&self) -> bool {
        self.next_rx_end.is_some()
    }

    /// Quota di coppie buone recenti in dB (0 dB = tutte buone).
    pub fn quality_db(&self) -> f32 {
        (20.0 * self.quality.max(1e-3).log10()) as f32
    }

    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.locked())
    }

    pub fn steer(&mut self, hz: f64) {
        self.base_hz = hz;
        self.centre_hz = hz;
    }
}

/// Di quanto l'AFC puo' allontanarsi dal centro scelto (Hz).
pub const AFC_LIMIT_HZ: f64 = 40.0;

/// Il ricevitore nel servizio (audio a 12 kHz, nessun ricampionamento: 120 campioni per bit).
pub struct AmtorLink {
    rx: AmtorRx,
}

impl AmtorLink {
    pub fn new(centre_hz: f32, reverse: bool, afc: bool) -> Self {
        AmtorLink { rx: AmtorRx::new(12_000.0, f64::from(centre_hz), reverse, afc) }
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        self.rx.process(audio);
        self.rx.take().into_iter().map(char::from).collect()
    }

    pub fn frequency(&self) -> f32 {
        self.rx.frequency() as f32
    }

    pub fn snr_db(&self) -> f32 {
        self.rx.quality_db()
    }

    pub fn signal(&self) -> (f32, bool) {
        self.rx.signal()
    }

    pub fn steer(&mut self, hz: f32) {
        self.rx.steer(f64::from(hz));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn uniform(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f32 {
            ((-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()) as f32
        }
    }

    fn noisy(x: &mut [f32], sigma: f32, seed: u64) {
        let mut r = Rng(seed | 1);
        for v in x {
            *v += sigma * r.gauss();
        }
    }

    const RATE: f64 = 12_000.0;

    fn signal(text: &str, freq: f64, reverse: bool) -> Vec<f32> {
        let mut a = vec![0.0_f32; 3000];
        a.extend(AmtorTx::new(RATE, freq, 0.5, reverse).transmit(text.as_bytes()));
        a.extend(vec![0.0_f32; 6000]);
        a
    }

    fn rx(audio: &[f32], centre: f64, reverse: bool, afc: bool) -> String {
        let mut r = AmtorRx::new(RATE, centre, reverse, afc);
        for b in audio.chunks(500) {
            r.process(b);
        }
        String::from_utf8_lossy(&r.take()).into_owned()
    }

    const TESTO: &str = "CQ CQ DE IU3ABC IU3ABC IO91WM 599 73 (K)? 1234567890 -/.,:";

    #[test]
    fn tabelle_e_codici() {
        // 35 codici validi (sette bit, quattro uni): 32 nelle tabelle (con LTRS, FIGS e le lettere) piu' alfa, beta, ripetizione...
        assert_eq!((0u8..128).filter(|&c| valid(c)).count(), 35);
        for (c, _) in LTRS.iter().filter(|(c, _)| *c != 0) {
            assert!(valid(*c), "{c:#x}");
        }
        for c in [CODE_LTRS, CODE_FIGS, CODE_ALPHA, CODE_BETA, CODE_CHAR32, CODE_REP] {
            assert!(valid(c));
        }
        let mut figs = false;
        assert_eq!(encode(b'a', &mut figs), vec![0x47]);
        assert_eq!(encode(b'1', &mut figs), vec![CODE_FIGS, 0x2e]);
        assert_eq!(encode(b'2', &mut figs), vec![0x27]);
        assert_eq!(encode(b' ', &mut figs), vec![0x5c]);
        assert_eq!(encode(b'B', &mut figs), vec![CODE_LTRS, 0x72]);
        assert_eq!(encode(b'%', &mut figs), Vec::<u8>::new());
    }

    #[test]
    fn ad_anello_chiuso_dritto_e_invertito() {
        for rev in [false, true] {
            let mut a = signal(TESTO, 1500.0, rev);
            noisy(&mut a, 0.02, 1);
            assert_eq!(rx(&a, 1500.0, rev, false), TESTO, "rev {rev}");
            let sbagliato = rx(&a, 1500.0, !rev, false);
            assert!(!sbagliato.contains("IU3ABC"), "rev {rev}: senza inversione {sbagliato:?}");
        }
    }

    /// Rumore forte (circa 0 dB in 3 kHz): la ripetizione ripara gli errori.
    #[test]
    fn nel_rumore() {
        let mut a = signal(TESTO, 1500.0, false);
        // tono di ampiezza 0,5 (potenza 0,125); rumore sigma 0,35 (0,12 su 6 kHz, circa 0,06 in 3 kHz)
        noisy(&mut a, 0.35, 2);
        let r = rx(&a, 1500.0, false, false);
        let ok = r.chars().zip(TESTO.chars()).filter(|(x, y)| x == y).count();
        assert!(ok + 3 >= TESTO.len(), "ricevuto {r:?}");
    }

    #[test]
    fn nel_solo_rumore_tace() {
        for sigma in [0.01_f32, 0.3] {
            let mut a = vec![0.0_f32; 12_000 * 120];
            noisy(&mut a, sigma, 3);
            let r = rx(&a, 1500.0, false, true);
            assert!(r.is_empty(), "sigma {sigma}: {r:?}");
        }
    }

    /// 20 Hz fuori senza AFC, 35 Hz con l'AFC; orologio sbagliato di 300 ppm.
    #[test]
    fn scostamento_di_frequenza_e_di_orologio() {
        let a = signal(TESTO, 1520.0, false);
        assert_eq!(rx(&a, 1500.0, false, false), TESTO);
        let a = signal(TESTO, 1535.0, false);
        assert_eq!(rx(&a, 1500.0, false, true), TESTO);
        let mut r = AmtorRx::new(RATE, 1500.0, false, true);
        r.process(&a[..a.len() - 9000]);
        assert!((r.frequency() - 1535.0).abs() < 5.0, "AFC a {:.1}", r.frequency());
        let a = signal(TESTO, 1500.0, false);
        let k = 1.0003;
        let b: Vec<f32> = (0..((a.len() as f64 / k) as usize - 1))
            .map(|i| {
                let x = i as f64 * k;
                let j = x as usize;
                let f = (x - j as f64) as f32;
                a[j] * (1.0 - f) + a[j + 1] * f
            })
            .collect();
        assert_eq!(rx(&b, 1500.0 / k, false, false), TESTO);
    }

    /// L'attesa in mezzo (fasatura) non perde le ripetizioni in coda e non stampa niente.
    #[test]
    fn attesa_in_mezzo() {
        let mut t = AmtorTx::new(RATE, 1500.0, 0.5, false);
        let mut a = t.preamble();
        a.extend(t.char(b'A'));
        a.extend(t.char(b'B'));
        for _ in 0..5 {
            a.extend(t.idle());
        }
        a.extend(t.char(b'1'));
        a.extend(t.postamble());
        a.extend(vec![0.0; 6000]);
        assert_eq!(rx(&a, 1500.0, false, false), "AB1");
    }
}
