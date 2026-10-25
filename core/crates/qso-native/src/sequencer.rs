//! La macchina a stati della sequenza dei QSO.
//!
//! Riscrive, in forma pura, quello che JTDX fa con `processMessage` (doppio clic su una riga),
//! `process_Auto` (avanzamento con AutoSeq a fine decodifica) e `QsoHistory` (stato per stazione).
//! Il principio e' lo stesso: **lo stato del QSO e' dato dall'ultimo messaggio ricevuto dal DX**,
//! e a ogni messaggio ricevuto corrisponde il messaggio Tx da cui parte la trasmissione dopo:
//!
//! | ricevuto dal DX | prossimo Tx |
//! |---|---|
//! | locatore (risposta al nostro CQ) | Tx2 `DX MIA -07` |
//! | rapporto `-07` | Tx3 `DX MIA R-07` |
//! | rapporto con conferma `R-07` | Tx4 `DX MIA RR73` |
//! | `RRR`, `RR73`, `73` (con i due rapporti scambiati) | Tx5 `DX MIA 73` |
//!
//! Nessun orologio e nessun I/O: il chiamante passa i periodi (`Slot`) e i secondi UTC.

use std::collections::HashSet;

use crate::message::{
    Body, Message, base_call, is_modifier, is_std_call, is_valid_call, is_valid_locator, parse,
    same_station,
};
use crate::report::format_report;
use crate::slot::{Mode, Slot};
use crate::stdmsg::{StdMessages, gen_cq, gen_std_msgs};
use crate::types::{
    AbandonReason, Decoded, Event, Options, Phase, QsoRecord, ReportPolicy, SeqError, Station,
    StopReason, Tx,
};

/// Chi ha cominciato il QSO: cambia il contatore che conta le ripetizioni del Tx2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Abbiamo risposto a un CQ o a un messaggio altrui.
    Answering,
    /// Ci ha chiamati lui (risposta al nostro CQ, o messaggio diretto a noi).
    Called,
}

/// La stazione in collegamento (il «DX» di JTDX).
#[derive(Clone, Debug)]
struct Dx {
    call: String,
    grid: Option<String>,
    freq_hz: u32,
    /// SNR dell'ultimo messaggio ricevuto da lui: e' il rapporto che gli mandiamo.
    snr: i32,
    role: Role,
    /// Ci ha mandato almeno un messaggio diretto a noi in questo QSO.
    contacted: bool,
    /// L'ultimo rapporto che abbiamo mandato (Tx2 o Tx3), come scritto («-07»).
    sent_report: Option<String>,
    /// Il rapporto che ci ha mandato («-10»).
    rcvd_report: Option<String>,
    /// Il Tx4 mandato era `RR73` (non `RRR`).
    sent_rr73: bool,
    logged: bool,
    first_slot: Slot,
    last_heard: Slot,
    /// Quante volte di fila e' stato mandato il messaggio corrente.
    repeats: u32,
}

/// Un chiamante che aspetta il suo turno.
#[derive(Clone, Debug)]
struct Caller {
    call: String,
    grid: Option<String>,
    rcvd_report: Option<String>,
    snr: i32,
    freq_hz: u32,
    start_ntx: u8,
    first_heard: Slot,
    last_heard: Slot,
    rx_even: bool,
    announced: bool,
}

/// Cosa fare dopo aver letto un messaggio del DX.
enum Step {
    Nothing,
    Goto(u8),
    CompleteThenGoto(u8),
    CompleteThenFinish,
    Abandon(AbandonReason),
}

/// La sequenza automatica dei QSO.
pub struct Sequencer {
    mode: Mode,
    my: Station,
    opts: Options,
    /// «Abilita Tx»: senza, `next_tx` non da' mai nulla.
    enabled: bool,
    /// Si sta facendo una serie di CQ (con il modificatore, se c'e').
    cq_run: Option<Option<String>>,
    /// Il messaggio da cui parte la prossima trasmissione (il pallino «Next» di JTDX).
    ntx: u8,
    tx_even: bool,
    dx: Option<Dx>,
    queue: Vec<Caller>,
    worked: HashSet<String>,
    events: Vec<Event>,
    last_tx: Option<(Slot, Tx)>,
    watchdog_ref: Option<f64>,
    last_text: Option<String>,
    first_tx_pending: bool,
    /// Il periodo (pari o dispari) per cui sono stati generati gli ultimi messaggi (`m_txGenerated` di JTDX).
    tx_generated: bool,
    cur_slot: Slot,
    reported: Option<(Phase, Option<String>, u8)>,
}

impl Sequencer {
    pub fn new(mode: Mode, my: Station, opts: Options) -> Result<Self, SeqError> {
        if !is_valid_call(&my.call) {
            return Err(SeqError::InvalidCall(my.call));
        }
        if !is_valid_locator(&my.grid) {
            return Err(SeqError::InvalidGrid(my.grid));
        }
        let tx_even = opts.tx_even;
        Ok(Sequencer {
            mode,
            my,
            opts,
            enabled: false,
            cq_run: None,
            ntx: 6,
            tx_even,
            dx: None,
            queue: Vec::new(),
            worked: HashSet::new(),
            events: Vec::new(),
            last_tx: None,
            watchdog_ref: None,
            last_text: None,
            first_tx_pending: false,
            tx_generated: tx_even,
            cur_slot: Slot(0),
            reported: None,
        })
    }

    // ---- lettura dello stato ------------------------------------------------------------

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn station(&self) -> &Station {
        &self.my
    }

    pub fn options(&self) -> &Options {
        &self.opts
    }

    pub fn set_options(&mut self, opts: Options) {
        let skip_changed = opts.skip_tx1 != self.opts.skip_tx1;
        let skip = opts.skip_tx1;
        self.opts = opts;
        // come `on_skipTx1_clicked` di JTDX: se il messaggio scelto e' ancora quello di partenza, accendere «Salta Tx1» porta da Tx1 a Tx2
        // e spegnerlo da Tx2 a Tx1. Se il DX ci ha gia' scritto, Tx2 e' un avanzamento vero del QSO e non si tocca.
        if skip_changed
            && let Some(dx) = self.dx.as_mut()
            && !dx.contacted
        {
            let to = match (skip, self.ntx) {
                (true, 1) => Some(2),
                (false, 2) => Some(1),
                _ => None,
            };
            if let Some(n) = to {
                self.ntx = n;
                dx.repeats = 0;
            }
        }
        self.note_state();
    }

    /// Cambio di banda: ferma la trasmissione e le serie di CQ (Abilita Tx spento), libera la stazione DX e svuota i chiamanti in coda
    /// e i messaggi ricordati della sequenza (come `clearDX` e `QsoHistory::init` di JTDX). Le impostazioni e la lista dei lavorati restano.
    pub fn band_changed(&mut self) {
        if let Some(old) = self.dx.take() {
            self.events.push(Event::Abandoned {
                call: old.call,
                reason: AbandonReason::Cleared,
            });
        }
        if self.enabled || self.cq_run.is_some() {
            self.stop(StopReason::Halt);
        }
        self.queue.clear();
        self.last_tx = None;
        self.last_text = None;
        self.watchdog_ref = None;
        self.first_tx_pending = false;
        self.ntx = 6;
        self.note_state();
    }

    /// Cambia il modo (FT8 <-> FT4): i periodi hanno un'altra durata, quindi il QSO in corso, i chiamanti in coda e la trasmissione si
    /// fermano e si riparte da capo. Le impostazioni e la lista dei lavorati restano.
    pub fn set_mode(&mut self, mode: Mode) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        if let Some(old) = self.dx.take() {
            self.events.push(Event::Abandoned {
                call: old.call,
                reason: AbandonReason::Cleared,
            });
        }
        if self.enabled || self.cq_run.is_some() {
            self.stop(StopReason::Halt);
        }
        self.queue.clear();
        self.last_tx = None;
        self.last_text = None;
        self.watchdog_ref = None;
        self.first_tx_pending = false;
        self.cur_slot = Slot(0);
        self.ntx = 6;
        self.note_state();
    }

    /// «Gen.Msgs»: rigenera i messaggi Tx1-Tx5 per il DX in collegamento e rimette il messaggio di partenza (Tx1, o Tx2 con «Salta Tx1»),
    /// come `genStdMsgs` di JTDX. Senza DX non c'e' nulla da rigenerare. I testi si compongono sempre dal DX e dal rapporto correnti.
    pub fn regen_messages(&mut self) {
        let skip = self.opts.skip_tx1;
        // `m_txGenerated = m_txFirst` sta in cima a `genStdMsgs`, prima del controllo sul DX (verificato in `jtdx.exe` a 0x474800)
        self.tx_generated = self.tx_even;
        if let Some(dx) = self.dx.as_mut() {
            dx.repeats = 0;
            self.ntx = if skip { 2 } else { 1 };
        }
        self.note_state();
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Il periodo di trasmissione e' pari.
    pub fn tx_even(&self) -> bool {
        self.tx_even
    }

    /// Cambio a mano del periodo (`TxMinuteButton`). Come in JTDX (`on_TxMinuteButton_clicked`): con Abilita Tx e AutoSeq accesi, se il
    /// periodo e' diverso da quello per cui sono stati generati i messaggi il DX viene liberato.
    pub fn set_tx_even(&mut self, even: bool) {
        self.tx_even = even;
        if self.tx_generated != even && self.enabled && self.opts.auto_seq {
            self.clear_dx();
        }
        // il periodo e' cambiato: l'eventuale trasmissione gia' pianificata in questo periodo non vale piu'
        self.last_tx = None;
    }

    /// Il DX in collegamento.
    pub fn dx(&self) -> Option<&str> {
        self.dx.as_ref().map(|d| d.call.as_str())
    }

    pub fn dx_grid(&self) -> Option<&str> {
        self.dx.as_ref().and_then(|d| d.grid.as_deref())
    }

    /// Il numero del messaggio da cui parte la prossima trasmissione (6 = CQ, 0 = nessuno).
    pub fn ntx(&self) -> u8 {
        self.shown_ntx()
    }

    pub fn phase(&self) -> Phase {
        if self.dx.is_some() {
            Phase::from_ntx(self.ntx)
        } else if self.enabled && self.cq_run.is_some() {
            Phase::Cq
        } else {
            Phase::Idle
        }
    }

    /// I chiamanti in coda, nell'ordine di arrivo.
    pub fn queued(&self) -> Vec<String> {
        self.queue.iter().map(|c| c.call.clone()).collect()
    }

    /// I sei testi Tx1..Tx6 correnti (vuoti quelli che non si possono comporre senza DX).
    pub fn tx_texts(&self) -> [String; 6] {
        let mut out: [String; 6] = Default::default();
        if let Some(dx) = &self.dx {
            let m = self.std_msgs(dx);
            for (i, slot) in out.iter_mut().take(5).enumerate() {
                if let Some(t) = m.get(i as u8 + 1) {
                    *slot = t.to_string();
                }
            }
        }
        out[5] = self.cq_text();
        out
    }

    pub fn is_worked(&self, call: &str) -> bool {
        self.worked.contains(&base_call(call))
    }

    /// Segna come gia' lavorata una stazione (per esempio dal log all'avvio).
    pub fn mark_worked(&mut self, call: &str) {
        self.worked.insert(base_call(call));
    }

    /// Gli eventi accumulati dall'ultima chiamata.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    // ---- comandi -------------------------------------------------------------------------

    /// Doppio clic su una riga decodificata, o risposta automatica a un CQ: imposta il DX, compone
    /// i sei messaggi, sceglie il periodo opposto al suo e abilita la trasmissione.
    ///
    /// Il messaggio da cui si parte e' quello di `processMessage` di JTDX: da un CQ (o da un
    /// messaggio altrui) Tx1, o Tx2 con «Salta Tx1»; da un messaggio diretto a noi, in base a cio' che
    /// contiene (locatore Tx2, rapporto Tx3, R+rapporto Tx4, RRR/RR73/73 Tx5).
    pub fn reply_to(&mut self, slot: Slot, decode: &Decoded) -> Result<(), SeqError> {
        self.reply_to_with(slot, decode, true)
    }

    /// Come `reply_to`, ma con `enable` falso non accende «Abilita Tx»: e' il doppio clic con «AutoTX» spento (`doubleClickOnCall` di JTDX accende
    /// Enable Tx solo se `m_autoTx`). Il DX e il messaggio di partenza si scelgono comunque; la trasmissione parte quando Enable Tx si accende.
    pub fn reply_to_with(&mut self, slot: Slot, decode: &Decoded, enable: bool) -> Result<(), SeqError> {
        self.cur_slot = self.cur_slot.max(slot);
        let (call, grid, rcvd, directed, role) = match parse(&decode.text) {
            Message::Cq { call, grid, .. } => (call, grid, None, None, Role::Answering),
            Message::Directed { to, from, body } => {
                let to_me = same_station(&to, &self.my.call);
                let grid = if let Body::Grid(g) = &body {
                    Some(g.clone())
                } else {
                    None
                };
                let rcvd = match &body {
                    Body::Report(n) | Body::RReport(n) => Some(format_report(*n)),
                    _ => None,
                };
                if to_me {
                    (from, grid, rcvd, Some(body), Role::Called)
                } else {
                    (from, grid, rcvd, None, Role::Answering)
                }
            }
            Message::Other => return Err(SeqError::NotReplyable(decode.text.clone())),
        };
        if same_station(&call, &self.my.call) {
            return Err(SeqError::OwnCall);
        }
        let ntx = match &directed {
            Some(Body::Grid(_)) | Some(Body::Other) => 2,
            Some(Body::Report(_)) => 3,
            Some(Body::RReport(_)) => 4,
            Some(Body::Rrr) | Some(Body::Rr73) | Some(Body::Bye) => 5,
            None => {
                if self.opts.skip_tx1 {
                    2
                } else {
                    1
                }
            }
        };
        if let Some(old) = self.dx.take() {
            self.events.push(Event::Abandoned {
                call: old.call,
                reason: AbandonReason::Replaced,
            });
        }
        self.queue.retain(|c| !same_station(&c.call, &call));
        self.dx = Some(Dx {
            call,
            grid,
            freq_hz: decode.freq_hz,
            snr: decode.snr,
            role,
            contacted: directed.is_some(),
            sent_report: None,
            rcvd_report: rcvd,
            sent_rr73: false,
            logged: false,
            first_slot: slot,
            last_heard: slot,
            repeats: 0,
        });
        self.ntx = ntx;
        self.tx_even = !slot.is_even();
        self.tx_generated = self.tx_even;
        if enable {
            self.enabled = true;
        }
        self.first_tx_pending = true;
        self.watchdog_ref = None;
        self.note_state();
        Ok(())
    }

    /// Comincia a chiamare CQ, con un modificatore se si vuole («DX», «NA», «POTA», «000»).
    pub fn call_cq(&mut self, modifier: Option<&str>) -> Result<(), SeqError> {
        let modifier = match modifier.map(str::trim).filter(|m| !m.is_empty()) {
            Some(m) => {
                let up = m.to_ascii_uppercase();
                if !is_modifier(&up) {
                    return Err(SeqError::InvalidModifier(m.to_string()));
                }
                Some(up)
            }
            None => None,
        };
        if let Some(old) = self.dx.take() {
            self.events.push(Event::Abandoned {
                call: old.call,
                reason: AbandonReason::Cleared,
            });
        }
        self.cq_run = Some(modifier);
        self.ntx = 6;
        self.enabled = true;
        self.first_tx_pending = false;
        self.watchdog_ref = None;
        self.start_next_caller();
        self.announce_queue();
        self.note_state();
        Ok(())
    }

    /// «Ferma Tx»: ferma la trasmissione e le serie di CQ. Il DX resta impostato, come in JTDX:
    /// `set_enabled(true)` riprende da dove si era.
    pub fn halt(&mut self) {
        if self.enabled || self.cq_run.is_some() {
            self.stop(StopReason::Halt);
        }
        self.note_state();
    }

    /// «Abilita Tx». Con `false` equivale a `halt`.
    pub fn set_enabled(&mut self, on: bool) {
        if on {
            self.enabled = true;
            self.watchdog_ref = None;
            self.note_state();
        } else {
            self.halt();
        }
    }

    /// «Canc DX»: libera la stazione; se si chiamava CQ si torna al CQ.
    pub fn clear_dx(&mut self) {
        if let Some(old) = self.dx.take() {
            self.events.push(Event::Abandoned {
                call: old.call,
                reason: AbandonReason::Cleared,
            });
            self.first_tx_pending = false;
            self.ntx = 6;
        }
        self.note_state();
    }

    /// Scelta a mano del messaggio da cui parte la prossima trasmissione (il pallino «Next»).
    pub fn select_tx(&mut self, n: u8) -> Result<(), SeqError> {
        if !(1..=6).contains(&n) {
            return Err(SeqError::InvalidTxNumber(n));
        }
        if n != 6 && self.dx.is_none() {
            return Err(SeqError::NoDx);
        }
        if n != self.ntx {
            self.ntx = n;
            if let Some(dx) = self.dx.as_mut() {
                dx.repeats = 0;
            }
        }
        self.note_state();
        Ok(())
    }

    // ---- ricezione -----------------------------------------------------------------------

    /// I messaggi decodificati al termine del periodo `slot`.
    ///
    /// Con AutoSeq acceso fa avanzare il QSO in corso, poi sceglie fra i chiamanti nuovi (uno solo
    /// in collegamento, gli altri in coda). Con AutoSeq spento non fa nulla.
    pub fn on_decodes(&mut self, slot: Slot, decodes: &[Decoded]) {
        self.cur_slot = self.cur_slot.max(slot);
        if !self.opts.auto_seq {
            return;
        }
        let parsed: Vec<(&Decoded, Message)> =
            decodes.iter().map(|d| (d, parse(&d.text))).collect();
        for (d, m) in &parsed {
            if self.dx.is_none() {
                break;
            }
            self.handle_dx_message(slot, d, m);
        }
        if self.enabled {
            self.collect_callers(slot, &parsed);
        }
        self.prune_queue();
        if self.dx.is_none() && self.enabled {
            self.start_next_caller();
        }
        // il DX appena preso dalla coda puo' aver parlato anche in questo periodo: si leggono
        // ora i suoi messaggi (rapporto aggiornato, locatore)
        if self.dx.as_ref().is_some_and(|dx| dx.last_heard < slot) {
            for (d, m) in &parsed {
                self.handle_dx_message(slot, d, m);
            }
        }
        self.announce_queue();
        self.note_state();
    }

    fn handle_dx_message(&mut self, slot: Slot, d: &Decoded, m: &Message) {
        let Message::Directed { to, from, body } = m else {
            return;
        };
        let my_call = self.my.call.clone();
        let abandon_other = self.opts.abandon_if_dx_answers_other;
        let step = {
            let Some(dx) = self.dx.as_mut() else {
                return;
            };
            if !same_station(from, &dx.call) {
                return;
            }
            if same_station(to, &my_call) {
                dx.contacted = true;
                dx.snr = d.snr;
                dx.freq_hz = d.freq_hz;
                dx.last_heard = slot;
                if from != &dx.call && from.contains('/') {
                    // lo stesso operatore con un nominativo piu' qualificato (/P, prefisso): come
                    // JTDX (`base_call != hiscall`) lo si adotta; il nominativo di base non sostituisce
                    // uno qualificato
                    dx.call = from.clone();
                }
                match body {
                    Body::Grid(g) => {
                        dx.grid = Some(g.clone());
                        Step::Goto(2)
                    }
                    Body::Report(n) => {
                        dx.rcvd_report = Some(format_report(*n));
                        Step::Goto(3)
                    }
                    Body::RReport(n) => {
                        dx.rcvd_report = Some(format_report(*n));
                        // senza un nostro rapporto gia' mandato lo diamo ora (Tx3), non si chiude
                        Step::Goto(if dx.sent_report.is_some() { 4 } else { 3 })
                    }
                    Body::Rrr | Body::Rr73 | Body::Bye => {
                        if dx.rcvd_report.is_none() || dx.sent_report.is_none() {
                            Step::Nothing
                        } else if matches!(body, Body::Rr73 | Body::Bye) && dx.sent_rr73 {
                            Step::CompleteThenFinish
                        } else {
                            Step::CompleteThenGoto(5)
                        }
                    }
                    Body::Other => Step::Nothing,
                }
            } else {
                let closing = matches!(body, Body::Rrr | Body::Rr73 | Body::Bye);
                if abandon_other && !dx.contacted && !closing {
                    Step::Abandon(AbandonReason::DxAnsweredOther)
                } else {
                    Step::Nothing
                }
            }
        };
        match step {
            Step::Nothing => {}
            Step::Goto(n) => {
                self.progress(slot);
                self.goto(n);
            }
            Step::CompleteThenGoto(n) => {
                self.progress(slot);
                self.complete(true);
                self.goto(n);
            }
            Step::CompleteThenFinish => {
                self.complete(true);
                self.end_qso();
            }
            Step::Abandon(reason) => {
                if let Some(dx) = &self.dx {
                    self.events.push(Event::Abandoned {
                        call: dx.call.clone(),
                        reason,
                    });
                }
                self.end_qso();
            }
        }
    }

    /// Un messaggio del DX che fa progredire il QSO azzera il watchdog.
    fn progress(&mut self, slot: Slot) {
        self.watchdog_ref = Some(slot.next().start_s(self.mode));
    }

    fn goto(&mut self, n: u8) {
        if n != self.ntx {
            self.ntx = n;
            if let Some(dx) = self.dx.as_mut() {
                dx.repeats = 0;
            }
        }
    }

    /// I messaggi diretti a noi da stazioni che non sono il DX in collegamento.
    fn collect_callers(&mut self, slot: Slot, parsed: &[(&Decoded, Message)]) {
        for (d, m) in parsed {
            let Message::Directed { to, from, body } = m else {
                continue;
            };
            if !same_station(to, &self.my.call) || same_station(from, &self.my.call) {
                continue;
            }
            if self
                .dx
                .as_ref()
                .is_some_and(|dx| same_station(from, &dx.call))
            {
                continue;
            }
            let (start_ntx, grid, rcvd) = match body {
                Body::Grid(g) => (2, Some(g.clone()), None),
                Body::Report(n) | Body::RReport(n) => (3, None, Some(format_report(*n))),
                // la chiusura di un QSO che non e' in corso non apre un QSO
                _ => continue,
            };
            if matches!(body, Body::Grid(_))
                && !self.opts.answer_worked_before
                && self.is_worked(from)
            {
                self.events
                    .push(Event::CallerIgnored { call: from.clone() });
                continue;
            }
            let rx_even = slot.is_even();
            if let Some(c) = self.queue.iter_mut().find(|c| same_station(&c.call, from)) {
                c.snr = d.snr;
                c.freq_hz = d.freq_hz;
                c.last_heard = slot;
                c.rx_even = rx_even;
                c.start_ntx = start_ntx;
                if grid.is_some() {
                    c.grid = grid;
                }
                if rcvd.is_some() {
                    c.rcvd_report = rcvd;
                }
            } else if self.queue.len() < self.opts.queue_max {
                self.queue.push(Caller {
                    call: from.clone(),
                    grid,
                    rcvd_report: rcvd,
                    snr: d.snr,
                    freq_hz: d.freq_hz,
                    start_ntx,
                    first_heard: slot,
                    last_heard: slot,
                    rx_even,
                    announced: false,
                });
            }
        }
    }

    fn ttl_slots(&self) -> u64 {
        let ms = (self.opts.queue_ttl_s.max(0.0) * 1000.0).ceil() as u64;
        ms.div_ceil(self.mode.period_ms())
    }

    fn prune_queue(&mut self) {
        let ttl = self.ttl_slots();
        let now = self.cur_slot;
        self.queue.retain(|c| now.since(c.last_heard) <= ttl);
    }

    fn announce_queue(&mut self) {
        for c in self.queue.iter_mut().filter(|c| !c.announced) {
            c.announced = true;
            self.events.push(Event::CallerQueued {
                call: c.call.clone(),
            });
        }
    }

    /// Prende dalla coda il chiamante da servire: il piu' vecchio, a parita' il piu' forte.
    fn start_next_caller(&mut self) -> bool {
        self.prune_queue();
        let mut best: Option<usize> = None;
        for (i, c) in self.queue.iter().enumerate() {
            let better = match best.and_then(|b| self.queue.get(b)) {
                None => true,
                Some(b) => {
                    (c.first_heard, std::cmp::Reverse(c.snr))
                        < (b.first_heard, std::cmp::Reverse(b.snr))
                }
            };
            if better {
                best = Some(i);
            }
        }
        let Some(idx) = best else {
            return false;
        };
        let c = self.queue.remove(idx);
        self.dx = Some(Dx {
            call: c.call,
            grid: c.grid,
            freq_hz: c.freq_hz,
            snr: c.snr,
            role: Role::Called,
            contacted: true,
            sent_report: None,
            rcvd_report: c.rcvd_report,
            sent_rr73: false,
            logged: false,
            first_slot: c.first_heard,
            last_heard: c.last_heard,
            repeats: 0,
        });
        self.ntx = c.start_ntx;
        self.tx_even = !c.rx_even;
        self.tx_generated = self.tx_even;
        self.first_tx_pending = true;
        self.watchdog_ref = None;
        true
    }

    // ---- trasmissione --------------------------------------------------------------------

    /// Cosa trasmettere nel periodo `slot`, se ne tocca a noi. `now` sono i secondi UTC attuali:
    /// servono a non partire troppo tardi nel periodo e al watchdog.
    ///
    /// Una chiamata per periodo: la stessa chiamata ripetuta nello stesso periodo da' lo stesso
    /// messaggio senza contarlo due volte. Ogni trasmissione ripetuta conta verso i tetti di
    /// ripetizione; raggiunto il tetto il tentativo si chiude e si passa al chiamante in coda, al
    /// CQ o all'arresto.
    pub fn next_tx(&mut self, slot: Slot, now: f64) -> Option<Tx> {
        self.cur_slot = self.cur_slot.max(slot);
        if !self.enabled {
            return None;
        }
        if let Some((s, tx)) = &self.last_tx
            && *s == slot
        {
            return Some(tx.clone());
        }
        if self.opts.watchdog_min > 0 {
            let reference = *self.watchdog_ref.get_or_insert(now);
            if now - reference >= f64::from(self.opts.watchdog_min) * 60.0 {
                self.stop(StopReason::Watchdog);
                self.note_state();
                return None;
            }
        }
        if slot.is_even() != self.tx_even {
            return None;
        }
        // i tetti di ripetizione: puo' chiudere il tentativo e far partire il prossimo
        let mut guard = 0;
        let n = loop {
            guard += 1;
            if guard > 4 {
                return None;
            }
            let n = match &self.dx {
                Some(_) => self.ntx,
                None if self.cq_run.is_some() => 6,
                None => return None,
            };
            let give_up = self
                .dx
                .as_ref()
                .is_some_and(|dx| self.limit_for(n, dx.role).is_some_and(|l| dx.repeats >= l));
            if !give_up {
                break n;
            }
            let (call, logged) = self
                .dx
                .as_ref()
                .map(|d| (d.call.clone(), d.logged))
                .unwrap_or_default();
            if !logged {
                self.events.push(Event::Abandoned {
                    call,
                    reason: AbandonReason::NoReply,
                });
            }
            self.end_qso();
            if !self.enabled {
                self.note_state();
                return None;
            }
        };
        if slot.is_even() != self.tx_even {
            self.note_state();
            return None;
        }
        let late = if self.first_tx_pending && self.opts.bypass_tx1 {
            self.mode.period_s() * 0.75
        } else {
            self.opts.late_limit_s(self.mode)
        };
        if now - slot.start_s(self.mode) > late {
            self.note_state();
            return None;
        }

        let text = self.text_for(n);
        let freq_hz = match (&self.dx, self.opts.follow_dx_freq, n) {
            (Some(dx), true, 1..=5) => dx.freq_hz,
            _ => self.opts.tx_freq_hz,
        };
        let report = self
            .dx
            .as_ref()
            .map(|d| format_report(self.report_value(d)));
        let use_rrr = self.opts.use_rrr;
        if n != 6
            && let Some(dx) = self.dx.as_mut()
        {
            dx.repeats += 1;
            match n {
                2 | 3 => dx.sent_report = report,
                4 => dx.sent_rr73 = !use_rrr,
                _ => {}
            }
        }
        if self.last_text.as_deref() != Some(text.as_str()) {
            self.watchdog_ref = Some(now);
            self.last_text = Some(text.clone());
        }
        self.first_tx_pending = false;
        let tx = Tx {
            n,
            text,
            freq_hz,
            even_slot: self.tx_even,
        };
        self.last_tx = Some((slot, tx.clone()));
        // come JTDX: il QSO si registra quando si manda RR73 o 73 avendo il rapporto del DX
        // (anche senza il rapporto del DX: JTDX registra quando `m_QSOProgress >= ROGER_REPORT` o si manda il Tx4/Tx5, e scrive `RST_RCVD` vuoto)
        if (n == 4 && !use_rrr) || n == 5 {
            self.complete(false);
        }
        if n == 5 {
            self.end_qso();
        }
        self.note_state();
        Some(tx)
    }

    fn limit_for(&self, n: u8, role: Role) -> Option<u32> {
        let l = &self.opts.limits;
        match n {
            1 => l.answer_cq,
            2 => match role {
                Role::Answering => l.answer_cq,
                Role::Called => l.answer_in_call,
            },
            3 => l.sent_r_report,
            4 | 5 => l.closing,
            _ => None,
        }
    }

    fn report_value(&self, dx: &Dx) -> i32 {
        match self.opts.report {
            ReportPolicy::Measured => dx.snr,
            ReportPolicy::Fixed(n) => n,
        }
    }

    fn std_msgs(&self, dx: &Dx) -> StdMessages {
        gen_std_msgs(
            &self.my.call,
            &self.my.grid,
            &dx.call,
            self.report_value(dx),
            self.opts.use_rrr,
        )
    }

    fn cq_text(&self) -> String {
        let modifier = self.cq_run.as_ref().and_then(|m| m.as_deref());
        gen_cq(&self.my.call, &self.my.grid, modifier)
    }

    fn text_for(&self, n: u8) -> String {
        if n == 6 {
            return self.cq_text();
        }
        self.dx
            .as_ref()
            .and_then(|dx| self.std_msgs(dx).get(n).map(str::to_string))
            .unwrap_or_default()
    }

    // ---- chiusura ------------------------------------------------------------------------

    /// Registra il QSO (una sola volta). In ricezione serve il rapporto del DX (`need_report`); quando si comincia a trasmettere RR73 o 73 si registra
    /// comunque, come JTDX, con il rapporto ricevuto vuoto se non c'e'.
    fn complete(&mut self, need_report: bool) {
        let fallback = self
            .dx
            .as_ref()
            .map(|d| format_report(self.report_value(d)));
        let Some(dx) = self.dx.as_mut() else {
            return;
        };
        if dx.logged {
            return;
        }
        let rst_rcvd = match dx.rcvd_report.clone() {
            Some(r) => r,
            None if !need_report => String::new(),
            None => return,
        };
        dx.logged = true;
        let rst_sent = dx.sent_report.clone().or(fallback).unwrap_or_default();
        let record = QsoRecord {
            call: dx.call.clone(),
            grid: dx.grid.clone().unwrap_or_default(),
            rst_sent,
            rst_rcvd,
            mode: self.mode,
            time_on_ms: dx.first_slot.start_ms(self.mode),
            time_off_ms: self.cur_slot.start_ms(self.mode),
            tx_freq_hz: self.opts.tx_freq_hz,
            rx_freq_hz: dx.freq_hz,
        };
        self.worked.insert(base_call(&dx.call));
        self.events.push(Event::QsoComplete(record));
    }

    /// Il QSO (riuscito o no) e' finito: si libera il DX e si decide cosa fare dopo.
    ///
    /// «1 QSO» ferma la trasmissione; altrimenti si serve il prossimo chiamante in coda, poi si
    /// torna al CQ se si stava chiamando CQ, altrimenti ci si ferma.
    fn end_qso(&mut self) {
        self.dx = None;
        self.first_tx_pending = false;
        if self.opts.single_qso {
            self.stop(StopReason::SingleQso);
            return;
        }
        if !self.enabled {
            self.ntx = 6;
            return;
        }
        if self.start_next_caller() {
            return;
        }
        if self.cq_run.is_some() {
            self.ntx = 6;
        } else {
            self.stop(StopReason::NothingToDo);
        }
    }

    fn stop(&mut self, reason: StopReason) {
        self.enabled = false;
        self.cq_run = None;
        self.queue.clear();
        self.events.push(Event::Stopped(reason));
    }

    fn shown_ntx(&self) -> u8 {
        if self.dx.is_some() {
            self.ntx
        } else if self.cq_run.is_some() {
            6
        } else {
            0
        }
    }

    fn note_state(&mut self) {
        let key = (
            self.phase(),
            self.dx.as_ref().map(|d| d.call.clone()),
            self.shown_ntx(),
        );
        if self.reported.as_ref() != Some(&key) {
            self.events.push(Event::StateChanged {
                phase: key.0,
                dx: key.1.clone(),
                ntx: key.2,
            });
            self.reported = Some(key);
        }
    }

    /// Il nominativo e' standard per l'FT8 (utile a chi compone i messaggi a mano).
    pub fn my_call_is_standard(&self) -> bool {
        is_std_call(&self.my.call)
    }
}
