//! WSPR-2 nel motore proprio: ricezione con `jt::wspr::rx` (wsprd di WSJT-X tradotto) e la tabella degli hash tenuta fra un periodo e l'altro
//! (`hashtable.txt` di wsprd, qui in memoria per la sessione), trasmissione del radiofaro come WSJT-X 3.3.0-beta6:
//!
//! - il messaggio (`MainWindow::WSPR_message`, `widgets/mainwindow.cpp:16591-16613`): `CALL GRID4 dBm`; con un nominativo composto, o un
//!   locatore a 6 caratteri e «Prefer type 1 messages» spento, si alternano il tipo 2 (`CALL dBm` o `CALL GRID4 dBm`) e il tipo 3
//!   (`<CALL> GRID6 dBm`);
//! - la potenza (`m_dBm`, di fabbrica 37 dBm, `mainwindow_settings.cpp:589`) fra i valori della tendina (`mainwindow.cpp:1559-1563`);
//! - la percentuale dei periodi in trasmissione («Tx Pct», di fabbrica 20, `mainwindow_settings.cpp:584`) con la tabella di due ore di
//!   `next_tx_state` (`WSPR/WsprTxScheduler.cpp`), quella che WSJT-X usa in WSPR senza il salto di banda (`mainwindow.cpp:14452`).
//!
//! FST4W usa lo stesso radiofaro (`MainWindow::WSPR_message`, `mainwindow.cpp:8079-8085`: il tipo 3 senza la potenza, riga 16606), la stessa
//! potenza e la stessa frequenza Tx (`WSPRfreqSpinBox`, 1400-1600 Hz, `mainwindow.cpp:8105`), ma i periodi in trasmissione li sceglie lo
//! scheduler semplice di `WSPRBandHopping::next_is_tx(true)` (`WSPR/WSPRBandHopping.cpp:256-272,486-491`, chiamato da
//! `beaconScheduleProposal`, `mainwindow.cpp:14443`): un tiro a caso per periodo con la percentuale e, sotto il 40%, mai due di fila
//! (il periodo saltato si recupera al giro dopo). Il «Round Robin» fisso di WSJT-X (1/2, 2/3...) non c'e': vale «random», quello di fabbrica.
//!
//! L'invio degli spot a WSPRnet (facoltativo, spento di fabbrica come «Upload spots» di WSJT-X, `mainwindow_settings.cpp:594`) lo fa il crate
//! `reporting` dall'evento `aurora_core::WsprCycle`; qui si tiene solo la deriva dei messaggi WSPR dell'ultimo periodo, che l'elenco dei
//! messaggi non porta.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use jt::wspr::rx::{Decoder, Options};

/// Potenza dichiarata nel messaggio, in dBm.
static DBM: AtomicU8 = AtomicU8::new(37);
/// Percentuale dei periodi in trasmissione.
static TX_PCT: AtomicU8 = AtomicU8::new(20);
/// «Prefer type 1 messages» (di fabbrica acceso, `mainwindow_settings.cpp:593`).
static PREFER_TYPE1: AtomicBool = AtomicBool::new(true);
/// `m_tx` di `WSPR_message`: alterna tipo 2 e tipo 3.
static ALTERNATE: AtomicBool = AtomicBool::new(false);
static DECODER: Mutex<Option<Decoder>> = Mutex::new(None);
static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);
/// Lo scheduler semplice di FST4W, con l'ultimo periodo gia' deciso (un periodo si decide una volta sola).
static SIMPLE: Mutex<Option<(SimpleScheduler, Option<(u64, bool)>)>> = Mutex::new(None);
/// La deriva (Hz) dei messaggi WSPR dell'ultimo periodo decodificato, per WSPRnet (la colonna «Drift» di `wspr_spots.txt`).
static LAST_DRIFTS: Mutex<Vec<(String, f32)>> = Mutex::new(Vec::new());

/// I valori della tendina della potenza di WSJT-X: `int(10 * i / 3 + 0,5)` per 1 mW ... 1 kW.
pub const POWERS_DBM: [u8; 19] = [0, 3, 7, 10, 13, 17, 20, 23, 27, 30, 33, 37, 40, 43, 47, 50, 53, 57, 60];

pub fn power_dbm() -> u8 {
    DBM.load(Ordering::Relaxed)
}

/// Imposta la potenza: solo i valori della tendina. `false` se il valore non e' fra quelli.
pub fn set_power_dbm(dbm: u32) -> bool {
    match u8::try_from(dbm).ok().filter(|d| POWERS_DBM.contains(d)) {
        Some(d) => {
            DBM.store(d, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

pub fn tx_pct() -> u8 {
    TX_PCT.load(Ordering::Relaxed)
}

/// Imposta la percentuale dei periodi in trasmissione (0-100, come la casella di WSJT-X).
pub fn set_tx_pct(pct: u32) -> bool {
    match u8::try_from(pct).ok().filter(|p| *p <= 100) {
        Some(p) => {
            TX_PCT.store(p, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

pub fn prefer_type1() -> bool {
    PREFER_TYPE1.load(Ordering::Relaxed)
}

pub fn set_prefer_type1(on: bool) {
    PREFER_TYPE1.store(on, Ordering::Relaxed);
}

/// Il messaggio del radiofaro (`WSPR_message`, `mainwindow.cpp:16591-16613`), alternando tipo 2 e tipo 3 quando serve. In FST4W
/// (`fst4w` vero) il tipo 3 non porta la potenza (`if(m_mode=="WSPR") msg0 += sdBm`, riga 16606).
pub fn beacon_message(call: &str, grid: &str, fst4w: bool) -> String {
    compose(call, grid, power_dbm(), prefer_type1(), fst4w, &ALTERNATE)
}

fn compose(call: &str, grid: &str, dbm: u8, prefer_type1: bool, fst4w: bool, alternate: &AtomicBool) -> String {
    let call = call.trim().to_ascii_uppercase();
    // il locatore a 6 caratteri come lo scrive WSJT-X (`AA00aa`): nel messaggio maiuscolo
    let grid = grid.trim().to_ascii_uppercase();
    let compound = call.find('/').is_some_and(|i| i > 0);
    let grid4: String = grid.chars().take(4).collect();
    if compound || (grid.len() == 6 && !prefer_type1) {
        // `m_tx = 1 - m_tx`: il primo messaggio e' il tipo 2 (m_tx parte da 0)
        let type2 = !alternate.fetch_xor(true, Ordering::Relaxed);
        if type2 {
            if compound { format!("{call} {dbm}") } else { format!("{call} {grid4} {dbm}") }
        } else if fst4w {
            format!("<{call}> {grid}")
        } else {
            format!("<{call}> {grid} {dbm}")
        }
    } else {
        format!("{call} {grid4} {dbm}")
    }
}

/// Decodifica un periodo di due minuti alla profondita' di WSJT-X (1 Fast, 2 Normal, 3 Deep), con la tabella degli hash dei periodi prima.
/// Il limite di tempo tiene la decodifica dentro il periodo anche su un PC lento.
pub fn decode(samples: &[f32], depth: u8) -> Vec<jt::wspr::rx::Decode> {
    let mut opts = Options::depth(depth);
    opts.deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
    let mut guard = DECODER.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let found = guard.get_or_insert_with(Decoder::new).decode(samples, &opts);
    *LAST_DRIFTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = found.iter().map(|d| (d.message.clone(), d.drift_hz)).collect();
    found
}

/// La deriva di un messaggio WSPR dell'ultimo periodo decodificato (0 se non c'e').
pub fn last_drift(message: &str) -> f32 {
    LAST_DRIFTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().find(|(m, _)| m == message).map_or(0.0, |(_, d)| *d)
}

/// Il periodo che comincia a `slot_start_unix_s` va trasmesso? `next_tx_state` di `WsprTxScheduler.cpp` con la percentuale scelta.
pub fn next_is_tx(slot_start_unix_s: u64) -> bool {
    let pct = tx_pct();
    if pct >= 100 {
        return true;
    }
    let mut guard = SCHEDULER.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.get_or_insert_with(Scheduler::from_clock).next_tx_state(pct, slot_start_unix_s)
}

/// FST4W: il periodo che comincia a `slot_start_ms` va trasmesso? Lo scheduler semplice di WSJT-X con la percentuale scelta, una volta per
/// periodo (la stessa domanda sullo stesso periodo ha la stessa risposta).
pub fn next_is_tx_fst4w(slot_start_ms: u64) -> bool {
    let pct = tx_pct();
    let mut guard = SIMPLE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (scheduler, last) = guard.get_or_insert_with(|| (SimpleScheduler::new(clock_seed()), None));
    if let Some((slot, tx)) = *last
        && slot == slot_start_ms
    {
        return tx;
    }
    let tx = scheduler.next(pct);
    *last = Some((slot_start_ms, tx));
    tx
}

fn clock_seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x9E37_79B9)
}

/// `WSPRBandHopping::impl::simple_scheduler` (`WSPR/WSPRBandHopping.cpp:256-272`): trasmette se un numero a caso fra 0 e 99 e' sotto la
/// percentuale; sotto il 40% una trasmissione subito dopo un'altra si rimanda al periodo seguente (`carry_`), che allora trasmette di sicuro.
pub struct SimpleScheduler {
    last_was_tx: bool,
    carry: bool,
    rng: Rng,
}

impl SimpleScheduler {
    pub fn new(seed: u64) -> Self {
        SimpleScheduler { last_was_tx: false, carry: false, rng: Rng(seed | 1) }
    }

    pub fn next(&mut self, pct: u8) -> bool {
        let mut tx = self.carry || u32::from(pct) > self.rng.next() % 100;
        if self.carry {
            self.carry = false;
        } else if pct < 40 && self.last_was_tx && tx {
            tx = false;
            self.carry = true;
        }
        self.last_was_tx = tx;
        tx
    }
}

/// Generatore pseudo-casuale (xorshift64*) al posto di `rand()` del C.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as u32
    }
}

/// `WsprTxScheduler.cpp`: una tabella di 60 periodi (due ore: sei righe da 20 minuti, dieci periodi da 2 minuti) rifatta a ogni blocco di due
/// ore o cambio di percentuale.
pub struct Scheduler {
    tx: [u8; 60],
    table_2hr_slot: i64,
    table_pctx: u8,
    rng: Rng,
}

impl Scheduler {
    pub fn new(seed: u64) -> Self {
        Scheduler { tx: [0; 60], table_2hr_slot: -1, table_pctx: 0, rng: Rng(seed | 1) }
    }

    fn from_clock() -> Self {
        Scheduler::new(clock_seed())
    }

    fn rand(&mut self) -> usize {
        self.rng.next() as usize
    }

    /// `tx_band_sum`: le trasmissioni di ogni colonna (periodo da 2 minuti nei 20 minuti).
    fn band_sum(&self) -> [u8; 10] {
        std::array::from_fn(|j| (0..6).map(|i| self.tx[i * 10 + j]).sum())
    }

    /// `tx_add_to_band`: una trasmissione in una riga a caso della colonna `band`.
    fn add_to_band(&mut self, band: usize) {
        for _ in 0..10 {
            let islot = self.rand() % 6;
            if self.tx[islot * 10 + band] != 1 {
                self.tx[islot * 10 + band] = 1;
                return;
            }
        }
    }

    /// `tx_add_one`: aggiunge una trasmissione nel buco piu' grande, senza due trasmissioni di fila.
    fn add_one(&mut self) {
        let mut ngap = 13;
        while ngap >= 3 {
            for i in 0..(60 - ngap) {
                if (0..ngap).all(|j| self.tx[i + j] == 0) {
                    self.tx[i + ngap / 2] = 1;
                    return;
                }
            }
            ngap -= 2;
        }
        if self.tx[58] == 0 && self.tx[59] == 0 {
            self.tx[59] = 1;
        }
    }

    /// `tx_trim`: al piu' `ntxlim` trasmissioni di fila e mai il primo periodo della tabella.
    fn trim(&mut self, ntxlim: u32) -> u32 {
        if self.tx[0] == 1 {
            self.tx[0] = 0;
        }
        let mut nrun = 0;
        for i in 0..60 {
            if self.tx[i] == 1 {
                nrun += 1;
                if nrun > ntxlim {
                    self.tx[i] = 0;
                    nrun = 0;
                }
            } else {
                nrun = 0;
            }
        }
        self.tx.iter().map(|&v| u32::from(v)).sum()
    }

    /// `create_tx_schedule`.
    fn create(&mut self, pctx: u8) {
        let needed = (60.0 * (f64::from(pctx) / 100.0) + 0.5) as u32;
        self.tx = [0; 60];
        if pctx == 0 {
            return;
        }
        if pctx <= 25 {
            // l'algoritmo di K1JT: fra due trasmissioni da 2 a 5 periodi di ricezione con il 25%
            let rxavg = 100.0 / f32::from(pctx) - 1.0;
            let mut i = 0usize;
            loop {
                let x = (self.rand() % 100) as f32 / 100.0;
                let nrx = (rxavg + 3.0 * x - 1.0) as usize;
                i += nrx + 1;
                if i < 60 {
                    self.tx[i] = 1;
                } else {
                    return;
                }
            }
        }
        let (ntxlim, ntxbandmin) = match pctx {
            26..=32 => (1, 1),
            33..=49 => (1, 2),
            50..=59 => (2, 3),
            _ => (3, 4),
        };
        for _ in 0..ntxbandmin {
            for j in 0..10 {
                self.add_to_band(j);
            }
        }
        let mut sum = self.trim(ntxlim);
        let mut j = 0;
        let mut done = false;
        while j < 100 && !done {
            let bsum = self.band_sum();
            done = true;
            for (i, &b) in bsum.iter().enumerate() {
                if b < ntxbandmin {
                    done = false;
                    for _ in 0..(ntxbandmin - b) {
                        self.add_to_band(i);
                    }
                }
            }
            sum = self.trim(ntxlim);
            j += 1;
        }
        for _ in 0..needed.saturating_sub(sum) {
            self.add_one();
        }
    }

    /// `next_tx_state`: l'ora e' quella del periodo da decidere (WSJT-X usa l'ora attuale piu' 30 s, a fine del periodo prima).
    pub fn next_tx_state(&mut self, pctx: u8, slot_start_unix_s: u64) -> bool {
        let t = slot_start_unix_s + 30;
        let hour = (t / 3600) % 24;
        let minute = (t / 60) % 60;
        let tx_2hr_slot = (hour / 2) as i64;
        let tx_20min_slot = ((hour - (hour / 2) * 2) * 3 + minute / 20) as usize;
        let tx_2min_slot = ((minute % 20) / 2) as usize;
        if tx_2hr_slot != self.table_2hr_slot || pctx != self.table_pctx {
            self.create(pctx);
            self.table_2hr_slot = tx_2hr_slot;
            self.table_pctx = pctx;
        }
        self.tx[tx_20min_slot * 10 + tx_2min_slot] == 1
    }
}

/// Le prove che cambiano i valori globali del radiofaro (Tx Pct, potenza, periodi) girano una alla volta: in parallelo una rimetteva il Tx
/// Pct dell'altra e il radiofaro poteva non partire (prova che falliva ogni tanto).
#[cfg(test)]
pub(crate) fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(pct: u8, seed: u64) -> (usize, bool) {
        let mut s = Scheduler::new(seed);
        // due ore intere: 60 periodi da due minuti a partire da un'ora pari
        let base = 1_700_000_000u64 / 7200 * 7200;
        let flags: Vec<bool> = (0..60).map(|k| s.next_tx_state(pct, base + 120 * k)).collect();
        let consecutive = flags.windows(2).any(|w| w[0] && w[1]);
        (flags.iter().filter(|&&f| f).count(), consecutive)
    }

    #[test]
    fn the_schedule_follows_the_percentage_without_consecutive_transmissions() {
        for seed in 1..20 {
            let (n, consecutive) = count(20, seed);
            // 20%: in media 12 su 60 (fra due trasmissioni da 3 a 5 periodi)
            assert!((9..=16).contains(&n), "{n}");
            assert!(!consecutive);
            let (n, consecutive) = count(33, seed);
            assert!((18..=22).contains(&n), "{n}");
            assert!(!consecutive);
        }
        assert_eq!(count(0, 3).0, 0);
    }

    #[test]
    fn the_message_follows_wsjtx() {
        let alt = AtomicBool::new(false);
        assert_eq!(compose("iu3abc", "IO91wm", 37, true, false, &alt), "IU3ABC IO91 37");
        // nominativo composto: prima il tipo 2, poi il tipo 3
        assert_eq!(compose("PJ4/K1ABC", "FK52ud", 37, true, false, &alt), "PJ4/K1ABC 37");
        assert_eq!(compose("PJ4/K1ABC", "FK52ud", 37, true, false, &alt), "<PJ4/K1ABC> FK52UD 37");
        // locatore a 6 caratteri senza «Prefer type 1»
        assert_eq!(compose("IU3ABC", "IO91wm", 23, false, false, &alt), "IU3ABC IO91 23");
        assert_eq!(compose("IU3ABC", "IO91wm", 23, false, false, &alt), "<IU3ABC> IO91WM 23");
        for m in ["IU3ABC IO91 37", "PJ4/K1ABC 37", "<PJ4/K1ABC> FK52UD 37", "<IU3ABC> IO91WM 23"] {
            assert!(jt::wspr::tx::tones(m).is_some(), "{m}");
        }
        assert!(!set_power_dbm(36));
        assert!(!set_tx_pct(101));
    }

    /// FST4W: lo stesso radiofaro, ma il tipo 3 senza la potenza (`WSPR_message`, riga 16606); ogni forma si codifica in FST4W.
    #[test]
    fn the_fst4w_beacon_drops_the_power_from_type_3() {
        let alt = AtomicBool::new(false);
        assert_eq!(compose("IU3ABC", "IO91wm", 37, true, true, &alt), "IU3ABC IO91 37");
        assert_eq!(compose("IU3ABC", "IO91wm", 30, false, true, &alt), "IU3ABC IO91 30");
        assert_eq!(compose("IU3ABC", "IO91wm", 30, false, true, &alt), "<IU3ABC> IO91WM");
        assert_eq!(compose("PJ4/K1ABC", "FK52ud", 30, true, true, &alt), "PJ4/K1ABC 30");
        assert_eq!(compose("PJ4/K1ABC", "FK52ud", 30, true, true, &alt), "<PJ4/K1ABC> FK52UD");
        let mut hashes = ft8::message::CallHashes::default();
        for m in ["IU3ABC IO91 37", "<IU3ABC> IO91WM", "PJ4/K1ABC 30", "<PJ4/K1ABC> FK52UD"] {
            assert!(jt::fst4::encode(m, true, &mut hashes).is_ok(), "{m}");
        }
    }

    /// FST4W: lo scheduler semplice tiene la percentuale e sotto il 40% non trasmette mai due periodi di fila; al 100% sempre, a 0 mai.
    #[test]
    fn the_fst4w_scheduler_follows_the_percentage() {
        for seed in 1..20u64 {
            let mut s = SimpleScheduler::new(seed);
            let flags: Vec<bool> = (0..1000).map(|_| s.next(20)).collect();
            let n = flags.iter().filter(|&&f| f).count();
            assert!((150..=250).contains(&n), "{n}");
            assert!(!flags.windows(2).any(|w| w[0] && w[1]));
            let mut s = SimpleScheduler::new(seed);
            let n = (0..1000).filter(|_| s.next(60)).count();
            assert!((540..=660).contains(&n), "{n}");
        }
        let mut s = SimpleScheduler::new(5);
        assert!((0..100).all(|_| s.next(100)));
        let mut s = SimpleScheduler::new(5);
        assert!((0..100).all(|_| !s.next(0)));
    }
}
