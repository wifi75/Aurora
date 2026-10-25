//! La memoria della sequenza: una tabella per stazione con lo stato del QSO, i contatori, la priorita' e l'ultima frequenza.
//!
//! E' la classe `QsoHistory` di JTDX 2.2.159 (`qsohistory.cpp`, di Arvo ES1JA), riscritta riga per riga: `message()` aggiorna lo stato con
//! ogni messaggio ricevuto (`R...`) o trasmesso (`S...`), `autoseq_call()` dice a che punto siamo con una stazione e `autoseq_pick()`
//! sceglie chi chiamare quando non c'e' un DX (tre blocchi: risposte al proprio CQ nell'ultimo periodo, risposte nei 5 minuti, CQ degli altri).
//!
//! Verificato anche nell'eseguibile (`QsoHistory::autoseq` 0x4d8260, `::message` 0x4dbbf0): stesse soglie, stesso `switch`.
//!
//! Gli orari sono i **secondi del giorno** come in JTDX (`unsigned`, con l'aritmetica senza segno e i ripieghi `300` e `86100` per il
//! passaggio di mezzanotte). La chiave della tabella e' il nominativo di base; l'ordine di visita e' quello alfabetico (JTDX usa un `QHash`
//! dall'ordine non definito: a parita' completa di punteggio qui vince il nominativo che viene prima).

use std::collections::BTreeMap;

use crate::message::base_call;

/// Secondi del giorno (UTC).
pub type Sec = u32;

/// Il continente di una stazione di cui il registro non sa niente. JTDX (che ha sempre `cty.dat`) non sceglie mai una stazione senza continente;
/// qui, senza dati sui paesi, la direzione del CQ non esclude nessuno.
pub const UNKNOWN_CONTINENT: &str = "??";

/// I 17 stati di `QsoHistory::Status`. Il prefisso `R` e' «ricevuto dal DX», `S` «inviato da noi».
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    None = 0,
    Rfin,
    Rcq,
    Scq,
    Rcall,
    Scall,
    Rreport,
    Sreport,
    Rrreport,
    Srreport,
    Rrr,
    Srr,
    Rrr73,
    Srr73,
    R73,
    S73,
    Fin,
}

/// Cio' che la tabella ricorda di una stazione (`QsoHistory::QSO`).
#[derive(Clone, Debug)]
pub struct Qso {
    pub call: String,
    pub status: Status,
    pub srx_c: Status,
    pub srx_p: Status,
    pub stx_c: Status,
    pub stx_p: Status,
    /// Quante volte di fila e' stato trasmesso lo stesso messaggio (con la stessa frequenza).
    pub count: i32,
    pub priority: i32,
    /// Frequenza audio di trasmissione e di ricezione.
    pub tx: i32,
    pub rx: i32,
    /// Inizio dell'attivita' con questa stazione e ultimo messaggio.
    pub b_time: Sec,
    pub time: Sec,
    /// Direzione del CQ («DX», «EU», «NA»...).
    pub tyyp: String,
    pub continent: String,
    pub mpx: String,
    pub grid: String,
    /// Il rapporto che il DX ci ha mandato e quello che gli mandiamo (o il suo SNR).
    pub r_rep: String,
    pub s_rep: String,
    pub distance: i32,
}

impl Qso {
    fn new(call: &str) -> Qso {
        Qso {
            call: call.to_string(),
            status: Status::None,
            srx_c: Status::None,
            srx_p: Status::None,
            stx_c: Status::None,
            stx_p: Status::None,
            count: 0,
            priority: 0,
            tx: 0,
            rx: 0,
            b_time: 0,
            time: 0,
            tyyp: String::new(),
            continent: String::new(),
            mpx: String::new(),
            grid: String::new(),
            r_rep: String::new(),
            s_rep: String::new(),
            distance: 0,
        }
    }
}

/// Il nostro CQ (`_CQ`).
#[derive(Clone, Debug, Default)]
pub struct CqState {
    pub call: String,
    pub tyyp: String,
    pub time: Sec,
    pub b_time: Sec,
    pub tx: i32,
    pub count: i32,
}

/// Cio' che `autoseq` restituisce oltre allo stato.
#[derive(Clone, Debug, Default)]
pub struct Autoseq {
    pub grid: String,
    pub rep: String,
    pub rx: i32,
    pub tx: i32,
    pub time: Sec,
    pub count: i32,
    pub prio: i32,
}

/// La scelta di un chiamante (blocco A, B o C).
#[derive(Clone, Debug)]
pub struct Pick {
    pub status: Status,
    pub call: String,
    pub out: Autoseq,
}

/// Una stazione gia' chiamata senza esito (`QsoHistory::calllist`): rapporto e orario della rinuncia.
#[derive(Clone, Copy, Debug)]
struct Called {
    rep: i32,
    time: Sec,
}

/// I flag di `process_Auto` per la scelta (il campo `time` di `autoseq`).
pub mod algo {
    /// Cerca anche tra i CQ degli altri (blocco C): AutoSeq 6 e 7.
    pub const CQ_SEARCH: u32 = 1;
    /// A parita' preferisce chi ha mandato il rapporto.
    pub const REPORT_PRIORITY: u32 = 16;
    /// A parita' preferisce il piu' lontano.
    pub const MAX_DISTANCE: u32 = 32;
    /// Blocco C: risponde a chiunque (nessuna evidenziazione di «nuovo» o «Chiama gia' lavorati»).
    pub const CALL_WORKED: u32 = 64;
    /// Blocchi A e B: risponde anche ai gia' lavorati (nessuna evidenziazione o «Risp B4»).
    pub const ANSWER_WORKED: u32 = 128;
    /// Blocco C: prima i nuovi nominativi con priorita' alta.
    pub const HIGHER_NEW_CALL: u32 = 256;
}

/// Stringa di rapporto in intero come `QString::toInt()` (0 se non e' un numero).
fn to_int(s: &str) -> i32 {
    s.trim().parse().unwrap_or(0)
}

/// Una coordinata in radianti (latitudine, longitudine), `(0, 0)` se il locatore non e' leggibile.
fn from_qth(qth: &str) -> (f64, f64) {
    let up = qth.to_ascii_uppercase();
    let padded = match up.len() {
        0..=3 => format!("{}55LL55LL", up.chars().take(2).collect::<String>()),
        4..=5 => format!("{}LL55LL", up.chars().take(4).collect::<String>()),
        6..=7 => format!("{}55LL", up.chars().take(6).collect::<String>()),
        8..=9 => format!("{}LL", up.chars().take(8).collect::<String>()),
        _ => up,
    };
    let b = padded.as_bytes();
    if b.len() != 10 {
        return (0.0, 0.0);
    }
    let in_range = |i: usize, lo: u8, hi: u8| (lo..=hi).contains(&b[i]);
    let ok = in_range(0, b'A', b'R')
        && in_range(1, b'A', b'R')
        && in_range(2, b'0', b'9')
        && in_range(3, b'0', b'9')
        && in_range(4, b'A', b'X')
        && in_range(5, b'A', b'X')
        && in_range(6, b'0', b'9')
        && in_range(7, b'0', b'9')
        && in_range(8, b'A', b'X')
        && in_range(9, b'A', b'X');
    if !ok {
        return (0.0, 0.0);
    }
    let mut l = [0f64; 10];
    for (i, c) in b.iter().enumerate() {
        l[i] = f64::from(*c) - 65.0;
    }
    // le cifre partono da '0' = 48: -65 + 17 le riporta a 0..9 come fa JTDX
    l[2] += 17.0;
    l[3] += 17.0;
    l[6] += 17.0;
    l[7] += 17.0;
    let pi = std::f64::consts::PI;
    let rad_0 = 20.0 * pi / 180.0;
    let rad_1 = 10.0 * pi / 180.0;
    let rad_2 = 2.0 * pi / 180.0;
    let rad_3 = 1.0 * pi / 180.0;
    let rad_4 = rad_2 / 24.0;
    let rad_5 = rad_3 / 24.0;
    let rad_6 = rad_4 / 10.0;
    let rad_7 = rad_5 / 10.0;
    let rad_8 = rad_6 / 24.0;
    let rad_9 = rad_7 / 24.0;
    let lng = l[0] * rad_0 + l[2] * rad_2 + l[4] * rad_4 + l[6] * rad_6 + (l[8] + 0.5) * rad_8 - pi;
    let lat = l[1] * rad_1 + l[3] * rad_3 + l[5] * rad_5 + l[7] * rad_7 + (l[9] + 0.5) * rad_9 - pi / 2.0;
    (lat, lng)
}

/// Distanza in chilometri sul cerchio massimo tra due coordinate (0 se una non e' nota), come `QsoHistory::Distance`.
fn distance(a: (f64, f64), b: (f64, f64)) -> i32 {
    if (a.0 == 0.0 && a.1 == 0.0) || (b.0 == 0.0 && b.1 == 0.0) {
        return 0;
    }
    let dlon = b.1 - a.1;
    let dlat = b.0 - a.0;
    let h = (dlat / 2.0).sin().powi(2) + a.0.cos() * b.0.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * h.sqrt().atan2((1.0 - h).sqrt());
    (6378.137 * c) as i32
}

/// Distanza in km tra due locatori (4 o 6 caratteri), 0 se uno dei due non e' leggibile.
pub fn grid_distance_km(a: &str, b: &str) -> i32 {
    distance(from_qth(a), from_qth(b))
}

/// La tabella per stazione e le sue regole.
#[derive(Default)]
pub struct QsoHistory {
    data: BTreeMap<String, Qso>,
    black: BTreeMap<String, i32>,
    called: BTreeMap<String, Called>,
    cq: CqState,
    /// Orario dell'ultimo periodo decodificato (`max_r_time`).
    max_r_time: Sec,
    /// E' arrivato qualcosa di utile dall'ultima scelta (`as_active`).
    as_active: bool,
    my_continent: String,
    my_prefix: String,
    my_pos: (f64, f64),
    /// Il contrario dell'opzione «direzione CQ rigorosa» (`_strictdirCQ = !strictdirCQ`): vero = i desiderati passano oltre la direzione.
    wanted_pass_direction: bool,
}

impl QsoHistory {
    pub fn new() -> QsoHistory {
        QsoHistory::default()
    }

    /// `QsoHistory::init`: scarta tutto (cambio di banda di piu' di 1 MHz, cambio di modo).
    pub fn init(&mut self) {
        self.data.clear();
        self.black.clear();
        self.called.clear();
        self.cq = CqState::default();
        self.as_active = false;
    }

    /// `owndata`: continente, prefisso e locatore propri; `strict` e' l'opzione «direzione CQ rigorosa».
    pub fn set_own(&mut self, continent: &str, prefix: &str, grid: &str, strict: bool) {
        self.wanted_pass_direction = !strict;
        self.my_continent = continent.trim().to_string();
        self.my_prefix = prefix.to_string();
        self.my_pos = from_qth(grid);
    }

    /// `time()`: l'orario dell'ultimo periodo decodificato.
    pub fn set_time(&mut self, time: Sec) {
        self.max_r_time = time;
    }

    pub fn cq(&self) -> &CqState {
        &self.cq
    }

    /// Una stazione ha scritto qualcosa di utile (`as_active`).
    pub fn is_active(&self) -> bool {
        self.as_active
    }

    pub fn entry(&self, call: &str) -> Option<&Qso> {
        self.data.get(&base_call(call))
    }

    /// `remove`: toglie la stazione da tutte e tre le tabelle.
    pub fn remove(&mut self, call: &str) {
        let key = base_call(call);
        self.black.remove(&key);
        self.called.remove(&key);
        self.data.remove(&key);
    }

    /// `blacklist`: la stazione non verra' piu' scelta.
    pub fn blacklist(&mut self, call: &str) {
        *self.black.entry(base_call(call)).or_insert(0) += 1;
    }

    /// `calllist`: ricorda una rinuncia con il rapporto e l'orario (si tiene la piu' recente o la col rapporto migliore).
    pub fn calllist(&mut self, call: &str, level: i32, time: Sec) {
        let key = base_call(call);
        let mut ret = self.called.get(&key).copied().unwrap_or(Called { rep: -35, time: 0 });
        if ret.rep < level || ret.time < time || ret.time.wrapping_sub(time) > 43_200 {
            ret.rep = level;
            ret.time = time;
            self.called.insert(key, ret);
        }
    }

    /// `reset_count`: azzera il conteggio e, se richiesto, cambia lo stato. 1 se la stazione c'era, altrimenti 0.
    pub fn reset_count(&mut self, call: &str, status: Status) -> i32 {
        let key = base_call(call);
        if let Some(t) = self.data.get_mut(&key)
            && t.call == call
        {
            t.count = 0;
            if status != Status::None {
                t.status = status;
            }
            return 1;
        }
        0
    }

    /// `rx`: aggiorna la frequenza di ricezione di una stazione nota (se l'orario non e' quello dell'ultimo periodo).
    pub fn rx(&mut self, call: &str, freq: i32) {
        let key = base_call(call);
        let max_r_time = self.max_r_time;
        if let Some(t) = self.data.get_mut(&key)
            && t.time != max_r_time
            && t.rx != freq
            && freq > 0
        {
            t.rx = freq;
        }
    }

    /// `status`: lo stato e il locatore di una stazione.
    pub fn status(&self, call: &str) -> (Status, String) {
        match self.data.get(&base_call(call)) {
            Some(t) => (t.status, t.grid.clone()),
            None => (Status::None, String::new()),
        }
    }

    /// `log_data`: orario di inizio, rapporto ricevuto e rapporto inviato di una stazione.
    pub fn log_data(&self, call: &str) -> (Status, Sec, String, String) {
        match self.data.get(&base_call(call)) {
            Some(t) => (t.status, t.b_time, t.r_rep.clone(), t.s_rep.clone()),
            None => (Status::None, 0, String::new(), String::new()),
        }
    }

    /// `autoseq` con un nominativo: lo stato della stazione e i dati da usare. `io` porta i valori attuali (griglia, rapporto, frequenze,
    /// `count`) che vengono sostituiti; con `hound` (`count == -1` in JTDX) vale la variante del modo Hound.
    pub fn autoseq_call(&mut self, call: &str, io: &mut Autoseq, hound: bool) -> Status {
        let key = base_call(call);
        let default = Qso::new("");
        let t = self.data.get(&key).cloned().unwrap_or(default);
        if t.status > Status::None || t.rx > 0 {
            let ret = t.status;
            io.prio = t.priority;
            if hound {
                io.count = if t.stx_c == Status::Scall && ret == Status::Rreport { 1 } else { t.count + 1 };
            } else if self.direction_not_for_me(&t) && (!self.wanted_pass_direction || (t.priority < 20 && t.status != Status::Rcq)) {
                io.count = 1;
            } else {
                io.count = t.count;
            }
            if t.grid.len() > 3 {
                io.grid = t.grid.clone();
            }
            if !t.s_rep.is_empty() {
                io.rep = t.s_rep.clone();
            }
            if t.rx > 0 {
                io.rx = t.rx;
            }
            if (ret == Status::Srr73 || ret >= Status::S73) && t.tx == self.cq.tx {
                io.tx = 1;
            } else if t.tx > 0 {
                io.tx = t.tx;
            }
            io.time = t.b_time;
            if hound && ret == Status::Rrr73 {
                let mut t2 = t;
                t2.status = Status::Fin;
                self.data.insert(key, t2);
            }
            ret
        } else {
            io.rep = String::new();
            io.rx = 0;
            io.tx = 0;
            io.prio = 0;
            Status::None
        }
    }

    /// Il CQ che la stazione ha chiamato era diretto a un'altra zona (condizione dell'`else if` che mette `count = 1` in `autoseq`).
    fn direction_not_for_me(&self, t: &Qso) -> bool {
        let ty = t.tyyp.as_str();
        let cq_call = self.cq.call.as_str();
        let left = |s: &str, n: usize| s.chars().take(n).collect::<String>();
        (ty.chars().count() == 2
            && ty != self.my_continent
            && ty != left(cq_call, 2)
            && ty != self.my_prefix
            && (ty != "DX" || t.continent == self.my_continent))
            || (ty.chars().count() == 1 && ty != left(cq_call, 1) && ty != self.my_prefix)
    }

    /// `autoseq` senza nominativo: sceglie chi chiamare con i flag `algo`. Restituisce la stazione scelta (stato `None` e nessuna
    /// stazione se non c'e' nessuno). Consuma `as_active`: la scelta si rifa' solo se e' arrivato qualcosa di nuovo.
    pub fn autoseq_pick(&mut self, algo_flags: u32, io: &mut Autoseq) -> Option<Pick> {
        self.autoseq_pick_with(algo_flags, io, None)
    }

    /// Come `autoseq_pick`; con `scorer` i blocchi A e B (le risposte al proprio CQ) scelgono il punteggio piu' alto, a parita' chi e' arrivato prima
    /// (come «Best caller first» di Decodium), al posto delle regole di priorita' di JTDX. I filtri (periodo, stato, direzione del CQ, lista nera)
    /// restano; non c'e' soglia di priorita'. Il blocco C (i CQ degli altri) resta quello di JTDX.
    pub fn autoseq_pick_with(&mut self, algo_flags: u32, io: &mut Autoseq, scorer: Option<&dyn Fn(&Qso) -> f64>) -> Option<Pick> {
        if !self.as_active || self.data.is_empty() {
            self.as_active = false;
            return None;
        }
        let a_init: i32 = if algo_flags & algo::ANSWER_WORKED != 0 { -1 } else { 4 };
        let b_init: i32 = if algo_flags & algo::CALL_WORKED != 0 {
            -1
        } else if algo_flags & algo::HIGHER_NEW_CALL != 0 {
            8
        } else {
            4
        };
        let keys: Vec<String> = self.data.keys().cloned().collect();

        // blocco A: risposte al mio CQ nell'ultimo periodo; blocco B: nei 5 minuti dal mio ultimo CQ
        for block in 0..2 {
            if let Some(scorer) = scorer {
                if let Some(pick) = self.pick_scored(block, &keys, scorer, io) {
                    self.as_active = false;
                    return Some(pick);
                }
                continue;
            }
            let mut priority = a_init;
            let mut rep = "-60".to_string();
            let mut r_rep = "-60".to_string();
            let mut dist = 0;
            let mut mycall = false;
            let mut lastcalled = false;
            let mut chosen: Option<Pick> = None;
            for key in &keys {
                let on_black = self.black.get(key).copied().unwrap_or(0);
                let Some(tt) = self.data.get(key) else { continue };
                let in_window = if block == 0 {
                    tt.time == self.max_r_time
                } else {
                    // `(tt.time - _CQ.time < 300 && tt.time >= 300) || (tt.time < 300 && tt.time - (_CQ.time - 86100) < 300)`, senza segno
                    (tt.time.wrapping_sub(self.cq.time) < 300 && tt.time >= 300)
                        || (tt.time < 300 && tt.time.wrapping_sub(self.cq.time.wrapping_sub(86_100)) < 300)
                };
                let answering = matches!(tt.status, Status::Rcall | Status::Rreport | Status::Rrreport | Status::Rrr | Status::Rrr73);
                let wanted_cq = matches!(tt.status, Status::Rcq | Status::Rfin) && !mycall && tt.priority > 16 && tt.priority < 20;
                if !(on_black == 0 && in_window && !tt.continent.is_empty() && (!lastcalled || tt.time == tt.b_time) && (answering || wanted_cq)) {
                    continue;
                }
                if !lastcalled && tt.time == tt.b_time {
                    priority = a_init;
                }
                let direct = matches!(tt.status, Status::Rcall | Status::Rreport | Status::Rrreport);
                let tt_rep = to_int(&tt.s_rep);
                let better = tt.priority > priority
                    || (priority > a_init
                        && ((direct && !mycall)
                            || (tt.priority == priority
                                && ((algo_flags & algo::MAX_DISTANCE == 0
                                    && ((algo_flags & algo::REPORT_PRIORITY == 0 && !tt.s_rep.is_empty() && tt_rep > to_int(&rep))
                                        || (algo_flags & algo::REPORT_PRIORITY != 0
                                            && ((tt.status == Status::Rcall && !tt.s_rep.is_empty() && tt_rep > to_int(&rep) && r_rep == "-60")
                                                || (tt.status == Status::Rreport && !tt.s_rep.is_empty() && tt_rep > to_int(&r_rep))))))
                                    || (algo_flags & algo::MAX_DISTANCE != 0 && tt.distance > dist)))));
                if !better {
                    continue;
                }
                let wanted_dir = self.wanted_pass_direction && (tt.priority > 16 || (tt.priority > 1 && tt.priority < 5));
                let cq_dir = self.cq.tyyp.as_str();
                let direction_ok = cq_dir.is_empty()
                    || tt.continent == UNKNOWN_CONTINENT
                    || wanted_dir
                    || cq_dir == tt.continent
                    || cq_dir == tt.mpx
                    || tt.call.starts_with(cq_dir)
                    || (cq_dir == "DX" && tt.continent != self.my_continent);
                if !direction_ok {
                    continue;
                }
                if tt.time == tt.b_time {
                    lastcalled = true;
                }
                if matches!(tt.status, Status::Rcall | Status::Rreport) {
                    mycall = true;
                }
                priority = tt.priority;
                dist = tt.distance;
                if !tt.s_rep.is_empty() {
                    if tt.status == Status::Rreport && algo_flags & algo::REPORT_PRIORITY != 0 {
                        r_rep = tt.s_rep.clone();
                    }
                    rep = tt.s_rep.clone();
                }
                let mut out = Autoseq { grid: tt.grid.clone(), rep: rep.clone(), count: tt.count, prio: tt.priority, time: tt.b_time, ..Autoseq::default() };
                out.rx = if tt.rx > 0 { tt.rx } else { io.rx };
                out.tx = if tt.tx > 0 { tt.tx } else { io.tx };
                chosen = Some(Pick { status: tt.status, call: tt.call.clone(), out });
            }
            if let Some(p) = chosen {
                self.as_active = false;
                return Some(p);
            }
        }

        // blocco C: i CQ degli altri (AutoSeq 6 e 7)
        if algo_flags & algo::CQ_SEARCH != 0 {
            let mut priority = b_init;
            let mut rep = "-60".to_string();
            let mut dist = 0;
            let mut chosen: Option<Pick> = None;
            for key in &keys {
                let on_black = self.black.get(key).copied().unwrap_or(0);
                let called = self.called.get(key).copied().unwrap_or(Called { rep: -35, time: 0 });
                let Some(tt) = self.data.get(key) else { continue };
                let tt_rep = to_int(&tt.s_rep);
                let not_recent = called.rep == -35
                    || called.rep < tt_rep
                    || (tt.b_time > 300 && tt.b_time.wrapping_sub(called.time) > 300)
                    || (tt.b_time <= 300 && called.time.wrapping_sub(tt.b_time) < 86_100);
                if !(not_recent
                    && on_black == 0
                    && tt.time == self.max_r_time
                    && (tt.status == Status::Rcq || (tt.status == Status::Rfin && tt.priority > 0))
                    && !tt.continent.is_empty())
                {
                    continue;
                }
                let better = tt.priority > priority
                    || (priority > b_init
                        && tt.priority == priority
                        && ((algo_flags & algo::MAX_DISTANCE == 0 && !tt.s_rep.is_empty() && tt_rep > to_int(&rep))
                            || (algo_flags & algo::MAX_DISTANCE != 0 && tt.distance > dist)));
                if !better {
                    continue;
                }
                let ty = tt.tyyp.as_str();
                let cq_call = self.cq.call.as_str();
                // la direzione del suo CQ deve includermi
                let theirs_ok = ty.is_empty()
                    || ty.chars().count() > 2
                    || (self.wanted_pass_direction && tt.priority > 19)
                    || ty == self.my_continent
                    || cq_call.starts_with(ty)
                    || ty == self.my_prefix
                    || (ty == "DX" && tt.continent != self.my_continent);
                // e la direzione del mio CQ deve includere lui
                let cq_dir = self.cq.tyyp.as_str();
                let mine_ok = cq_dir.is_empty()
                    || tt.continent == UNKNOWN_CONTINENT
                    || cq_dir == tt.continent
                    || (self.wanted_pass_direction && (tt.priority > 16 || (tt.priority > 1 && tt.priority < 5)))
                    || tt.call.starts_with(cq_dir)
                    || (cq_dir == "DX" && tt.continent != self.my_continent);
                if !(theirs_ok && mine_ok) {
                    continue;
                }
                priority = tt.priority;
                dist = tt.distance;
                if !tt.s_rep.is_empty() {
                    rep = tt.s_rep.clone();
                }
                let mut out = Autoseq { grid: tt.grid.clone(), rep: rep.clone(), count: tt.count, prio: tt.priority, time: tt.b_time, ..Autoseq::default() };
                out.rx = if tt.rx > 0 { tt.rx } else { io.rx };
                out.tx = if tt.tx > 0 { tt.tx } else { io.tx };
                chosen = Some(Pick { status: tt.status, call: tt.call.clone(), out });
            }
            self.as_active = false;
            return chosen;
        }
        self.as_active = false;
        None
    }

    /// La scelta per punteggio di un blocco (A o B): tra i chiamanti validi vince il punteggio piu' alto, a parita' il primo arrivato (`b_time` piu' basso).
    fn pick_scored(&self, block: u8, keys: &[String], scorer: &dyn Fn(&Qso) -> f64, io: &Autoseq) -> Option<Pick> {
        let mut eligible: Vec<&Qso> = Vec::new();
        for key in keys {
            let on_black = self.black.get(key).copied().unwrap_or(0);
            let Some(tt) = self.data.get(key) else { continue };
            let in_window = if block == 0 {
                tt.time == self.max_r_time
            } else {
                (tt.time.wrapping_sub(self.cq.time) < 300 && tt.time >= 300) || (tt.time < 300 && tt.time.wrapping_sub(self.cq.time.wrapping_sub(86_100)) < 300)
            };
            let answering = matches!(tt.status, Status::Rcall | Status::Rreport | Status::Rrreport | Status::Rrr | Status::Rrr73);
            if on_black != 0 || !in_window || tt.continent.is_empty() || !answering {
                continue;
            }
            let wanted_dir = self.wanted_pass_direction && (tt.priority > 16 || (tt.priority > 1 && tt.priority < 5));
            let cq_dir = self.cq.tyyp.as_str();
            let direction_ok = cq_dir.is_empty()
                || tt.continent == UNKNOWN_CONTINENT
                || wanted_dir
                || cq_dir == tt.continent
                || cq_dir == tt.mpx
                || tt.call.starts_with(cq_dir)
                || (cq_dir == "DX" && tt.continent != self.my_continent);
            if direction_ok {
                eligible.push(tt);
            }
        }
        eligible.sort_by_key(|t| t.b_time);
        let scores: Vec<f64> = eligible.iter().map(|t| scorer(t)).collect();
        let best = crate::score::best_index(&scores)?;
        let tt = eligible.get(best)?;
        let mut out = Autoseq { grid: tt.grid.clone(), rep: if tt.s_rep.is_empty() { "-60".to_string() } else { tt.s_rep.clone() }, count: tt.count, prio: tt.priority, time: tt.b_time, ..Autoseq::default() };
        out.rx = if tt.rx > 0 { tt.rx } else { io.rx };
        out.tx = if tt.tx > 0 { tt.tx } else { io.tx };
        Some(Pick { status: tt.status, call: tt.call.clone(), out })
    }

    /// `message`: registra un messaggio ricevuto (`R...`, `Rcq`, `Rfin`) o trasmesso (`S...`). `param` e' il locatore o il rapporto del messaggio,
    /// `rep` l'SNR della riga, `freq` la frequenza audio.
    #[allow(clippy::too_many_arguments)]
    pub fn message(&mut self, callsign: &str, status: Status, priority: i32, param: &str, tyyp: &str, continent: &str, mpx: &str, time: Sec, rep: &str, freq: i32) {
        if status == Status::Scq {
            self.cq.call = callsign.to_string();
            self.cq.tyyp = tyyp.to_string();
            self.cq.time = time;
            self.cq.b_time = time;
            self.cq.tx = freq;
            self.cq.count += 1;
            return;
        }
        let key = base_call(callsign);
        let mut t = self.data.get(&key).cloned().unwrap_or_else(|| Qso::new(callsign));
        if !(time >= t.time || time == 0 || status >= t.status || status == Status::Rreport) {
            return;
        }
        if status > Status::None {
            t.time = time;
            if self.cq.time == 0 {
                self.cq.time = time;
            }
            if matches!(status, Status::Scall | Status::Sreport | Status::Srreport | Status::Srr | Status::Srr73 | Status::S73) {
                if t.status == Status::None && (status == Status::Scall || (status == Status::Sreport && tyyp == "S")) {
                    t.status = Status::Rcq;
                    t.srx_c = Status::Rcq;
                }
                if status == t.stx_c {
                    if t.tx == freq {
                        t.count += 1;
                    }
                } else {
                    t.count = 1;
                }
                self.cq.count = 0;
            } else if status > Status::Rcq && status != t.srx_c {
                t.count = 0;
            }
        }
        if status == Status::Rcq || status == Status::Rcall || (t.call.len() < callsign.len() && callsign.contains(t.call.as_str())) {
            t.call = callsign.to_string();
        }
        let set_grid = |t: &mut Qso, my_pos: (f64, f64)| {
            if !param.is_empty() {
                t.grid = param.to_string();
                t.distance = distance(my_pos, from_qth(param));
            }
        };
        let my_pos = self.my_pos;
        let mut active = false;
        match status {
            Status::None => set_grid(&mut t, my_pos),
            Status::Rfin => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                if t.status <= Status::Scq
                    || t.status == Status::Scall
                    || (t.status == Status::Sreport && t.srx_p < Status::Rreport)
                    || (t.status == Status::Rcall && t.time != time)
                    || (t.status > Status::Scq && t.status < Status::Srr73 && ((t.time > 300 && t.time.wrapping_sub(t.b_time) > 300) || (t.time <= 300 && t.b_time.wrapping_sub(t.time) < 86_100)))
                {
                    t.status = status;
                    if priority > t.priority {
                        t.priority = priority;
                    }
                    if t.continent.is_empty() {
                        t.tyyp = tyyp.to_string();
                        t.continent = continent.trim().to_string();
                    }
                    if t.mpx.is_empty() {
                        t.mpx = mpx.to_string();
                    }
                    t.b_time = time;
                    t.s_rep = rep.to_string();
                    t.rx = freq;
                }
                active = true;
            }
            Status::Rcq => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                if t.status <= Status::Scq
                    || t.status == Status::Scall
                    || (t.status == Status::Sreport && t.srx_p < Status::Rreport)
                    || (t.status == Status::Rcall && t.time != time)
                    || (t.status > Status::Scq && t.status < Status::Srr73 && ((t.time > 300 && t.time.wrapping_sub(t.b_time) > 300) || (t.time <= 300 && t.b_time.wrapping_sub(t.time) < 86_100)))
                {
                    t.status = status;
                    if priority > t.priority {
                        t.priority = priority;
                    }
                    set_grid(&mut t, my_pos);
                    t.tyyp = tyyp.to_string();
                    t.continent = continent.trim().to_string();
                    t.mpx = mpx.to_string();
                    t.b_time = time;
                    t.s_rep = rep.to_string();
                    t.rx = freq;
                }
                active = true;
            }
            Status::Rcall => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                t.status = status;
                t.priority = priority;
                set_grid(&mut t, my_pos);
                t.continent = continent.trim().to_string();
                t.mpx = mpx.to_string();
                t.b_time = time;
                t.s_rep = rep.to_string();
                t.rx = freq;
                active = true;
            }
            Status::Scall => {
                t.stx_p = t.stx_c;
                t.stx_c = status;
                t.status = status;
                t.b_time = time;
                t.tx = freq;
            }
            Status::Rreport => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                if t.status < Status::Fin {
                    t.s_rep = rep.to_string();
                    let old = t.status;
                    t.status = if t.status == Status::Sreport { Status::Rrreport } else { status };
                    if priority > t.priority {
                        t.priority = priority;
                    }
                    if t.continent.is_empty() {
                        t.continent = continent.trim().to_string();
                    }
                    if t.mpx.is_empty() {
                        t.mpx = mpx.to_string();
                    }
                    t.r_rep = param.to_string();
                    if old < Status::Scq || t.stx_c == Status::None {
                        t.b_time = time;
                    }
                    t.rx = freq;
                    active = true;
                }
            }
            Status::Sreport => {
                t.stx_p = t.stx_c;
                t.stx_c = status;
                if t.status <= Status::Sreport {
                    t.s_rep = param.to_string();
                    let old = t.status;
                    t.status = status;
                    t.tx = freq;
                    if old < Status::Scq || tyyp == "S" {
                        t.b_time = time;
                    }
                }
            }
            Status::Rrreport => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                if t.status < Status::Fin && t.stx_c >= Status::Sreport {
                    let old = t.status;
                    t.status = status;
                    if priority > t.priority {
                        t.priority = priority;
                    }
                    if t.continent.is_empty() {
                        t.continent = continent.trim().to_string();
                    }
                    if t.mpx.is_empty() {
                        t.mpx = mpx.to_string();
                    }
                    t.r_rep = param.to_string();
                    if old < Status::Scq || t.stx_c == Status::None {
                        t.b_time = time;
                    }
                    t.rx = freq;
                    active = true;
                }
            }
            Status::Srreport => {
                t.stx_p = t.stx_c;
                t.stx_c = status;
                if t.status <= Status::Srreport {
                    t.s_rep = param.to_string();
                    let old = t.status;
                    t.status = status;
                    t.tx = freq;
                    if old < Status::Scq {
                        t.b_time = time;
                    }
                }
            }
            Status::Rrr => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                if !t.r_rep.is_empty() && !t.s_rep.is_empty() && t.status < Status::Fin {
                    t.status = status;
                    t.rx = freq;
                }
            }
            Status::Srr => {
                t.stx_p = t.stx_c;
                t.stx_c = status;
                if t.status <= Status::Srr {
                    t.status = status;
                    t.tx = freq;
                }
            }
            Status::Rrr73 => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                if !t.r_rep.is_empty() && !t.s_rep.is_empty() && t.status >= Status::Sreport {
                    if t.status == Status::Srr73 || (t.srx_p != Status::Rrr73 && t.status == Status::Fin) {
                        t.status = Status::Fin;
                        t.priority = 0;
                    } else {
                        t.status = status;
                    }
                    t.rx = freq;
                }
            }
            Status::Srr73 => {
                t.stx_p = t.stx_c;
                t.stx_c = status;
                if matches!(t.status, Status::Rrr73 | Status::R73 | Status::Fin) {
                    t.status = Status::Fin;
                    t.priority = 0;
                } else {
                    t.status = status;
                }
                t.tx = freq;
            }
            Status::R73 => {
                t.srx_p = t.srx_c;
                t.srx_c = status;
                if !t.r_rep.is_empty() && !t.s_rep.is_empty() && t.status >= Status::Sreport {
                    if matches!(t.status, Status::Srr73 | Status::S73 | Status::Fin) {
                        t.status = Status::Fin;
                        t.priority = 0;
                    } else {
                        t.status = status;
                    }
                    t.rx = freq;
                }
            }
            Status::S73 => {
                t.stx_p = t.stx_c;
                t.stx_c = status;
                if matches!(t.status, Status::Rrr73 | Status::R73 | Status::Fin) {
                    t.status = Status::Fin;
                    t.priority = 0;
                } else {
                    t.status = status;
                }
                t.tx = freq;
            }
            Status::Scq | Status::Fin => {
                t.status = status;
            }
        }
        if active {
            self.as_active = true;
        }
        self.data.insert(key, t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "IU3ABC";

    fn hist() -> QsoHistory {
        let mut h = QsoHistory::new();
        h.set_own("EU", "IU", "JN55", false);
        h
    }

    fn rx(h: &mut QsoHistory, call: &str, st: Status, param: &str, time: Sec, snr: &str, prio: i32) {
        h.message(call, st, prio, param, "", "EU", "", time, snr, 1200);
    }

    #[test]
    fn locators_and_distance_match_the_great_circle() {
        // JN55 (Slovenia e Croazia) e IO91 (sud dell'Inghilterra): circa 1100 km
        let d = grid_distance_km("JN55", "IO91");
        assert!((1000..1250).contains(&d), "{d}");
        assert_eq!(grid_distance_km("", "IO91"), 0, "locatore assente: distanza 0, come JTDX");
        assert_eq!(grid_distance_km("JN55", "JN55"), 0, "stesso punto");
    }

    /// `message`: RFIN/RCQ ricevuti da una stazione nuova; una risposta nostra la porta a SCALL e il conteggio parte da 1.
    #[test]
    fn a_cq_then_our_calls_count_up_and_a_different_message_resets() {
        let mut h = hist();
        rx(&mut h, "LZ2KSQ", Status::Rcq, "KN23", 100, "-12", 0);
        assert_eq!(h.status("LZ2KSQ").0, Status::Rcq);
        h.message("LZ2KSQ", Status::Scall, 0, "", "", "", "", 110, "", 1500);
        assert_eq!(h.entry("LZ2KSQ").map(|t| (t.status, t.count)), Some((Status::Scall, 1)));
        h.message("LZ2KSQ", Status::Scall, 0, "", "", "", "", 125, "", 1500);
        assert_eq!(h.entry("LZ2KSQ").map(|t| t.count), Some(2), "stesso messaggio e stessa frequenza: si somma");
        h.message("LZ2KSQ", Status::Scall, 0, "", "", "", "", 140, "", 1600);
        assert_eq!(h.entry("LZ2KSQ").map(|t| t.count), Some(2), "frequenza diversa: non si conta");
        h.message("LZ2KSQ", Status::Sreport, 0, "-07", "", "", "", 155, "", 1500);
        assert_eq!(h.entry("LZ2KSQ").map(|t| (t.status, t.count)), Some((Status::Sreport, 1)), "messaggio diverso: da capo");
    }

    /// Salta Tx1 (`tyyp == "S"`): un Tx2 verso una stazione sconosciuta finge un CQ ricevuto.
    #[test]
    fn a_report_with_skip_tx1_fakes_the_cq_and_a_new_reply_resets_the_count() {
        let mut h = hist();
        h.message("LZ2KSQ", Status::Sreport, 0, "-12", "S", "", "", 100, "", 1500);
        let t = h.entry("LZ2KSQ").expect("stazione");
        assert_eq!((t.status, t.srx_c, t.count, t.b_time), (Status::Sreport, Status::Rcq, 1, 100));
        // una risposta nuova del DX azzera il conteggio (`status > RCQ && status != srx_c`)
        rx(&mut h, "LZ2KSQ", Status::Rrreport, "R-10", 115, "-10", 0);
        assert_eq!(h.entry("LZ2KSQ").map(|t| (t.status, t.count)), Some((Status::Rrreport, 0)));
    }

    /// `RREPORT` mentre siamo in SREPORT vale R+rapporto; `RRREPORT` senza un nostro rapporto non avanza.
    #[test]
    fn a_report_after_our_report_counts_as_r_report_and_r_report_alone_does_not_advance() {
        let mut h = hist();
        rx(&mut h, "AA1AA", Status::Rcall, "FN42", 100, "-05", 0);
        h.message("AA1AA", Status::Sreport, 0, "-05", "", "", "", 110, "", 1500);
        rx(&mut h, "AA1AA", Status::Rreport, "-09", 115, "-09", 0);
        assert_eq!(h.status("AA1AA").0, Status::Rrreport, "un rapporto in risposta al nostro vale R+rapporto");
        let mut h = hist();
        rx(&mut h, "AA1AA", Status::Rcq, "FN42", 100, "-05", 0);
        rx(&mut h, "AA1AA", Status::Rrreport, "R-09", 115, "-09", 0);
        assert_eq!(h.status("AA1AA").0, Status::Rcq, "R+rapporto senza un nostro rapporto non avanza");
    }

    /// RR73 e 73 senza il rapporto ricevuto non fanno niente; FIN solo quando le due chiusure si sono incrociate.
    #[test]
    fn closings_without_reports_do_nothing_and_fin_needs_both_closings() {
        let mut h = hist();
        rx(&mut h, "AA1AA", Status::Rcall, "FN42", 100, "-05", 0);
        h.message("AA1AA", Status::Sreport, 0, "-05", "", "", "", 110, "", 1500);
        rx(&mut h, "AA1AA", Status::Rrr73, "", 115, "-05", 0);
        assert_eq!(h.status("AA1AA").0, Status::Sreport, "nessun rapporto ricevuto: RR73 ignorato");
        rx(&mut h, "AA1AA", Status::Rrreport, "R-09", 125, "-09", 0);
        h.message("AA1AA", Status::Srr73, 0, "", "", "", "", 140, "", 1500);
        assert_eq!(h.status("AA1AA").0, Status::Srr73);
        rx(&mut h, "AA1AA", Status::R73, "", 145, "-09", 0);
        assert_eq!(h.status("AA1AA").0, Status::Fin, "RR73 inviato e 73 ricevuto: FIN");
        // l'altro ordine: loro RR73, poi il nostro 73
        let mut h = hist();
        rx(&mut h, "BB2BB", Status::Rcq, "JN45", 100, "-05", 0);
        h.message("BB2BB", Status::Scall, 0, "", "", "", "", 110, "", 1500);
        rx(&mut h, "BB2BB", Status::Rreport, "-07", 125, "-07", 0);
        h.message("BB2BB", Status::Srreport, 0, "-09", "", "", "", 140, "", 1500);
        rx(&mut h, "BB2BB", Status::Rrr73, "", 155, "-07", 0);
        assert_eq!(h.status("BB2BB").0, Status::Rrr73);
        h.message("BB2BB", Status::S73, 0, "", "", "", "", 170, "", 1500);
        assert_eq!(h.status("BB2BB").0, Status::Fin, "RR73 ricevuto e 73 inviato: FIN");
        assert_eq!(h.entry("BB2BB").map(|t| t.priority), Some(0));
    }

    /// `autoseq_pick`, blocco A: chi mi ha chiamato nell'ultimo periodo; chi mi chiama direttamente batte chi manda solo un CQ voluto.
    #[test]
    fn block_a_picks_the_direct_caller_of_the_last_period_with_the_best_signal() {
        let mut h = hist();
        h.message(ME, Status::Scq, 0, "JN55", "", "", "", 90, "", 1500);
        h.set_time(100);
        rx(&mut h, "AA1AA", Status::Rcall, "FN42", 100, "-15", 0);
        rx(&mut h, "BB2BB", Status::Rcall, "JN45", 100, "-05", 0);
        rx(&mut h, "CC3CC", Status::Rcall, "KN23", 85, "+10", 0);
        let mut io = Autoseq::default();
        let pick = h.autoseq_pick(algo::ANSWER_WORKED | algo::CALL_WORKED, &mut io).expect("scelto");
        assert_eq!((pick.call.as_str(), pick.status), ("BB2BB", Status::Rcall), "il segnale piu' forte del periodo (CC3CC e' di un periodo prima)");
        assert_eq!(h.autoseq_pick(algo::ANSWER_WORKED, &mut io).map(|p| p.call), None, "senza niente di nuovo non si sceglie di nuovo (as_active)");
    }

    /// Con le evidenziazioni accese e «Risp B4» spenta chi non e' nuovo o voluto (priorita' 0) viene ignorato; con priorita' alta no.
    #[test]
    fn without_answer_b4_only_stations_above_priority_four_are_picked() {
        let mut h = hist();
        h.message(ME, Status::Scq, 0, "JN55", "", "", "", 90, "", 1500);
        h.set_time(100);
        rx(&mut h, "AA1AA", Status::Rcall, "FN42", 100, "-05", 0);
        assert!(h.autoseq_pick(0, &mut Autoseq::default()).is_none(), "priorita' 0, soglia 4: ignorato");
        rx(&mut h, "BB2BB", Status::Rcall, "JN45", 100, "-20", 22);
        let pick = h.autoseq_pick(0, &mut Autoseq::default()).expect("nuovo DXCC");
        assert_eq!((pick.call.as_str(), pick.out.prio), ("BB2BB", 22));
    }

    /// Blocco B: chi ha risposto al CQ nei 5 minuti ma non nell'ultimo periodo; fuori dai 5 minuti no.
    #[test]
    fn block_b_covers_five_minutes_after_our_cq() {
        let mut h = hist();
        h.message(ME, Status::Scq, 0, "JN55", "", "", "", 1000, "", 1500);
        rx(&mut h, "AA1AA", Status::Rcall, "FN42", 1030, "-05", 0);
        rx(&mut h, "BB2BB", Status::Rcall, "JN45", 1290, "-05", 0);
        h.set_time(1500);
        let first = h.autoseq_pick(algo::ANSWER_WORKED, &mut Autoseq::default());
        assert_eq!(first.map(|p| p.call), Some("AA1AA".to_string()), "BB2BB e' a 290 s dal CQ, AA1AA a 30 s: vince il segnale pari, poi l'ordine");
    }

    /// Blocco C: i CQ degli altri solo con AutoSeq 6/7; `calllist` impedisce di richiamare una stazione per 5 minuti salvo un segnale migliore.
    #[test]
    fn block_c_needs_the_cq_search_flag_and_respects_the_call_list() {
        let mut h = hist();
        h.message(ME, Status::Scq, 0, "JN55", "", "", "", 1, "", 1500);
        h.set_time(1000);
        rx(&mut h, "AA1AA", Status::Rcq, "FN42", 1000, "-15", 0);
        assert!(h.autoseq_pick(algo::ANSWER_WORKED, &mut Autoseq::default()).is_none(), "senza il flag 1 non si cerca tra i CQ");
        rx(&mut h, "AA1AA", Status::Rcq, "FN42", 1000, "-15", 0);
        let pick = h.autoseq_pick(algo::CQ_SEARCH | algo::ANSWER_WORKED | algo::CALL_WORKED, &mut Autoseq::default()).expect("CQ di AA1AA");
        assert_eq!((pick.call.as_str(), pick.status), ("AA1AA", Status::Rcq));
        // rinuncia annotata a t = 1000 con rapporto -15: lo stesso rapporto, entro 5 minuti, non lo richiama
        h.calllist("AA1AA", -15, 1000);
        h.set_time(1030);
        rx(&mut h, "AA1AA", Status::Rcq, "FN42", 1030, "-15", 0);
        assert!(h.autoseq_pick(algo::CQ_SEARCH | algo::ANSWER_WORKED | algo::CALL_WORKED, &mut Autoseq::default()).is_none(), "gia' chiamato da meno di 5 minuti");
        h.set_time(1060);
        rx(&mut h, "AA1AA", Status::Rcq, "FN42", 1060, "-08", 0);
        assert!(h.autoseq_pick(algo::CQ_SEARCH | algo::ANSWER_WORKED | algo::CALL_WORKED, &mut Autoseq::default()).is_some(), "con un segnale migliore si richiama");
    }

    /// Direzione del proprio CQ: un chiamante di un altro continente e' escluso da «CQ EU»; i desiderati passano se la direzione non e' rigorosa.
    #[test]
    fn the_direction_of_our_cq_limits_the_callers() {
        let mut h = hist();
        h.message(ME, Status::Scq, 0, "JN55", "EU", "", "", 90, "", 1500);
        h.set_time(100);
        h.message("W1AW", Status::Rcall, 0, "FN31", "", "NA", "", 100, "-05", 1200);
        assert!(h.autoseq_pick(algo::ANSWER_WORKED, &mut Autoseq::default()).is_none(), "un americano non risponde a «CQ EU»");
        h.message("K1ABC", Status::Rcall, 18, "FN42", "", "NA", "", 100, "-05", 1200);
        // desiderato (priorita' 17-19): passa se la direzione non e' rigorosa
        let pick = h.autoseq_pick(algo::ANSWER_WORKED, &mut Autoseq::default());
        assert_eq!(pick.map(|p| p.call), Some("K1ABC".to_string()));
        h.set_own("EU", "IU", "JN55", true);
        h.message("N1XYZ", Status::Rcall, 18, "FN42", "", "NA", "", 100, "-05", 1200);
        assert!(h.autoseq_pick(algo::ANSWER_WORKED, &mut Autoseq::default()).is_none(), "con la direzione rigorosa passano solo i continenti giusti");
    }

    /// Con il punteggio di Decodium (`scorer`) tra chi mi ha chiamato nell'ultimo periodo vince il punteggio migliore, non la priorita' di JTDX; a parita' chi e' arrivato prima.
    #[test]
    fn a_scorer_replaces_the_priority_rules_in_blocks_a_and_b() {
        let mut h = hist();
        h.message(ME, Status::Scq, 0, "JN55", "", "", "", 90, "", 1500);
        h.set_time(100);
        // JTDX sceglierebbe BB2BB (segnale piu' forte); il punteggio premia il paese nuovo (CC3CC, segnato dal solo nominativo per la prova)
        rx(&mut h, "AA1AA", Status::Rcall, "FN42", 100, "-15", 0);
        rx(&mut h, "BB2BB", Status::Rcall, "JN45", 100, "+05", 0);
        rx(&mut h, "CC3CC", Status::Rcall, "KN23", 100, "-20", 0);
        let by_jtdx = h.autoseq_pick(algo::ANSWER_WORKED, &mut Autoseq::default()).map(|p| p.call);
        assert_eq!(by_jtdx.as_deref(), Some("BB2BB"));
        let mut h2 = hist();
        h2.message(ME, Status::Scq, 0, "JN55", "", "", "", 90, "", 1500);
        h2.set_time(100);
        rx(&mut h2, "AA1AA", Status::Rcall, "FN42", 100, "-15", 0);
        rx(&mut h2, "BB2BB", Status::Rcall, "JN45", 100, "+05", 0);
        rx(&mut h2, "CC3CC", Status::Rcall, "KN23", 100, "-20", 0);
        let scorer = |t: &Qso| if t.call == "CC3CC" { 54.6 } else { 20.0 };
        let pick = h2.autoseq_pick_with(algo::ANSWER_WORKED, &mut Autoseq::default(), Some(&scorer)).expect("scelto");
        assert_eq!(pick.call, "CC3CC");
        // a parita' di punteggio vince chi e' arrivato prima (b_time piu' basso)
        let mut h3 = hist();
        h3.message(ME, Status::Scq, 0, "JN55", "", "", "", 90, "", 1500);
        h3.set_time(100);
        rx(&mut h3, "ZZ9ZZ", Status::Rcall, "FN42", 100, "-15", 0);
        h3.message("AA1AA", Status::Rcall, 0, "JN45", "", "EU", "", 100, "-15", 1200);
        let same = |_: &Qso| 10.0;
        // lo stesso b_time: vince il primo nell'ordine delle chiavi, ma la stabilita' non cambia niente tra i pari
        let pick = h3.autoseq_pick_with(algo::ANSWER_WORKED, &mut Autoseq::default(), Some(&same)).expect("scelto");
        assert_eq!(pick.call, "AA1AA");
    }

    /// Lista nera e `remove`.
    #[test]
    fn blacklisted_stations_are_never_picked_and_remove_forgets_them() {
        let mut h = hist();
        h.message(ME, Status::Scq, 0, "JN55", "", "", "", 90, "", 1500);
        h.set_time(100);
        rx(&mut h, "AA1AA", Status::Rcall, "FN42", 100, "-05", 0);
        h.blacklist("AA1AA");
        assert!(h.autoseq_pick(algo::ANSWER_WORKED, &mut Autoseq::default()).is_none());
        h.remove("AA1AA");
        assert_eq!(h.status("AA1AA").0, Status::None);
    }

    /// `autoseq_call`: con una stazione nota restituisce lo stato e i dati. Un CQ diretto a un'altra zona mette `count = 1` (il contatore scatta
    /// subito) solo con la direzione rigorosa o quando lo stato non e' piu' RCQ e la priorita' e' sotto 20.
    #[test]
    fn autoseq_call_reports_state_and_directional_cq_forces_count_one() {
        let mut h = hist();
        h.message("W1AW", Status::Rcq, 0, "FN31", "NA", "NA", "", 100, "-05", 1200);
        let mut io = Autoseq::default();
        let st = h.autoseq_call("W1AW", &mut io, false);
        assert_eq!((st, io.rx, io.grid.as_str(), io.rep.as_str()), (Status::Rcq, 1200, "FN31", "-05"));
        assert_eq!(io.count, 0, "direzione non rigorosa e stato RCQ: nessun conteggio forzato");
        // dopo la nostra chiamata (SCALL) la stessa stazione, con priorita' bassa, fa scattare subito il contatore
        h.message("W1AW", Status::Scall, 0, "", "", "", "", 110, "", 1500);
        h.autoseq_call("W1AW", &mut io, false);
        assert_eq!(io.count, 1);
        // direzione rigorosa: scatta anche in RCQ
        h.set_own("EU", "IU", "JN55", true);
        h.message("K1ABC", Status::Rcq, 0, "FN42", "NA", "NA", "", 100, "-05", 1300);
        h.autoseq_call("K1ABC", &mut io, false);
        assert_eq!(io.count, 1, "«CQ NA» da fuori continente con la direzione rigorosa");
        h.message("DL1AA", Status::Rcq, 0, "JO62", "", "EU", "", 100, "-05", 1300);
        let st = h.autoseq_call("DL1AA", &mut io, false);
        assert_eq!((st, io.count), (Status::Rcq, 0));
        let st = h.autoseq_call("ZZ9ZZ", &mut io, false);
        assert_eq!((st, io.rx, io.prio), (Status::None, 0, 0));
    }
}
