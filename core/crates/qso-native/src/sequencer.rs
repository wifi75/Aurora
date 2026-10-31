//! La macchina a stati della sequenza dei QSO.
//!
//! E' il porting della logica di QSO di JTDX 2.2.159, letta nei sorgenti pubblici e verificata nell'eseguibile (`docs/analisi-jtdx-reverse-rea.md`):
//!
//! | JTDX | Qui |
//! |---|---|
//! | `QsoHistory` (stato per stazione) | [`crate::history::QsoHistory`] |
//! | `DisplayText::displayDecodedText` / `displayTransmittedText` | [`crate::classify`] |
//! | `MainWindow::readFromStdout` (riga per riga, guardia di frequenza, `reply_other`) | [`Sequencer::on_decodes`] |
//! | `MainWindow::process_Auto` (contatori, scelta di chi chiamare, messaggio dallo stato) | `process_auto` |
//! | `MainWindow::processMessage` (doppio clic) | [`Sequencer::reply_to`] |
//! | `MainWindow::guiUpdate` (avvio, ritrasmissione, registrazione al 73, watchdog) | [`Sequencer::next_tx`] |
//!
//! Il principio e' quello di JTDX: **lo stato del QSO e' dato da cio' che si e' ricevuto e trasmesso con quella stazione** (la tabella
//! `QsoHistory`) e a ogni stato corrisponde il messaggio Tx da cui parte la trasmissione successiva (`process_Auto`):
//!
//! | stato letto | prossimo Tx |
//! |---|---|
//! | RFIN, RCQ | Tx1 (Tx2 con «Salta Tx1») |
//! | RCALL | Tx2 `DX MIA -07` |
//! | RREPORT | Tx3 `DX MIA R-07` |
//! | RRREPORT | Tx4 `DX MIA RR73` |
//! | RRR, RRR73, R73 | Tx5 `DX MIA 73` |
//! | SRR73 / S73 / FIN | niente: si ferma (con la registrazione automatica) |
//!
//! Nessun orologio e nessun I/O: il chiamante passa i periodi (`Slot`) e i secondi UTC.

use std::collections::{HashMap, HashSet};

use crate::classify::{DisplayState, RxContext, classify_rx, classify_tx, de_call_and_grid, report as decoded_report, words};
use crate::history::{Autoseq, Qso, QsoHistory, Sec, Status, algo};
use crate::message::{Body, Message, base_call, is_modifier, is_std_call, is_valid_call, is_valid_locator, parse, same_station};
use crate::score::{CallerFacts, caller_score};
use crate::slot::{Mode, Slot};
use crate::stdmsg::{StdMessages, gen_cq, gen_std_msgs};
use crate::types::{AbandonReason, CallerInfo, CallerSelection, Decoded, Event, Options, Phase, QsoRecord, ReportPolicy, SeqError, Station, StopReason, Tx};

/// Chi sa dire cosa ricorda il registro di una stazione (continente, priorita'): le evidenziazioni di JTDX.
pub type CallerInfoFn = Box<dyn Fn(&str) -> CallerInfo + Send>;

/// Un rapporto come lo scrive JTDX (`%+2.2d`): `-07`, `+05`.
fn rpt_text(n: i32) -> String {
    format!("{n:+03}")
}

/// I secondi del giorno di un istante UTC.
fn day_secs(utc_s: f64) -> Sec {
    (utc_s.max(0.0).floor() as u64 % 86_400) as Sec
}

/// Quel che serve a scrivere il QSO nel registro.
#[derive(Clone, Debug, Default)]
struct QsoLogInfo {
    /// Secondi del giorno in cui e' cominciato il QSO (`m_dateTimeQSOOn`).
    on_sec: Sec,
}

/// La sequenza automatica dei QSO.
pub struct Sequencer {
    mode: Mode,
    my: Station,
    my_base: String,
    opts: Options,
    hist: QsoHistory,
    ds: DisplayState,
    ds_max_r_time: Sec,
    caller_info: Option<CallerInfoFn>,

    // la finestra principale di JTDX
    /// «Abilita Tx» (`m_enableTx`).
    enabled: bool,
    /// Il DX in collegamento (`m_hisCall`, vuoto = nessuno) e il suo locatore.
    his_call: String,
    his_grid: String,
    /// Il rapporto che si manda (`rptSpinBox`) come numero e come testo (`m_rpt`).
    rpt: i32,
    rpt_text: String,
    /// Il messaggio da cui parte la prossima trasmissione (`m_ntx`, 1..=6).
    ntx: u8,
    tx_even: bool,
    /// Il periodo per cui sono stati generati gli ultimi messaggi (`m_txGenerated`).
    tx_generated: bool,
    /// Lo stato letto da `process_Auto` (`m_status`).
    status: Status,
    /// Frequenza di ricezione (`RxFreqSpinBox`).
    rx_hz: i32,
    last_calling_freq: i32,

    // letture della decodifica in corso
    reply_me: bool,
    reply_other: bool,
    reply_cq73: bool,
    process_auto_done: bool,
    call_first_73: bool,
    decode_started_for: Option<Slot>,
    /// Cooldown dopo una rinuncia: per tanti periodi non si cercano altri CQ (`m_counter`).
    counter: u32,
    used_freq: i32,

    // trasmissione
    last_tx: Option<(Slot, Tx)>,
    /// `m_restart`: il messaggio e' cambiato mentre si trasmetteva.
    restart: bool,
    /// Si sta trasmettendo il periodo che segue quello che si sta decodificando.
    rx_transmitting: bool,
    /// Il messaggio con cui e' cominciata l'ultima trasmissione (`m_transmittedQSOProgress`).
    transmitted_ntx: u8,
    cur_slot: Slot,
    current_message: String,
    last_message_sent: String,
    msg_sent0: String,
    rpt_sent: String,
    rpt_rcvd: String,
    last_logged: Option<(String, f64)>,
    qso_log: QsoLogInfo,

    // watchdog
    idle_minutes: u32,
    watchdog_expired: bool,
    last_minute: Option<i64>,
    sec_tx_stopped: f64,
    pending_tx_end: Option<f64>,

    worked: HashSet<String>,
    announced: HashSet<String>,
    /// Cosa il registro dice delle stazioni sentite (per il punteggio di «Best caller first»).
    info_cache: HashMap<String, CallerInfo>,
    /// AutoSeq spento da un messaggio libero che non e' un 73 (`m_wasAutoSeq`): si riaccende alla generazione dei messaggi successiva.
    auto_seq_suspended: bool,
    events: Vec<Event>,
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
        let mut hist = QsoHistory::new();
        hist.set_own(&opts.my_continent, &opts.my_prefix, &my.grid, opts.strict_directional_cq);
        let tx_hz = opts.tx_freq_hz as i32;
        Ok(Sequencer {
            mode,
            my_base: base_call(&my.call),
            my,
            opts,
            hist,
            ds: DisplayState::default(),
            ds_max_r_time: 0,
            caller_info: None,
            enabled: false,
            his_call: String::new(),
            his_grid: String::new(),
            rpt: 0,
            rpt_text: rpt_text(0),
            ntx: 6,
            tx_even,
            tx_generated: tx_even,
            status: Status::None,
            rx_hz: tx_hz,
            last_calling_freq: tx_hz,
            reply_me: false,
            reply_other: false,
            reply_cq73: false,
            process_auto_done: false,
            call_first_73: false,
            decode_started_for: None,
            counter: 0,
            used_freq: 0,
            last_tx: None,
            restart: false,
            rx_transmitting: false,
            transmitted_ntx: 6,
            cur_slot: Slot(0),
            current_message: String::new(),
            last_message_sent: String::new(),
            msg_sent0: String::new(),
            rpt_sent: String::new(),
            rpt_rcvd: String::new(),
            last_logged: None,
            qso_log: QsoLogInfo::default(),
            idle_minutes: 0,
            watchdog_expired: false,
            last_minute: None,
            sec_tx_stopped: 0.0,
            pending_tx_end: None,
            worked: HashSet::new(),
            announced: HashSet::new(),
            info_cache: HashMap::new(),
            auto_seq_suspended: false,
            events: Vec::new(),
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

    /// Chi sa dire continente e priorita' di una stazione (registro, `cty.dat`, desiderati). Senza, ogni stazione ha continente ignoto e priorita' 0.
    pub fn set_caller_info(&mut self, info: Option<CallerInfoFn>) {
        self.caller_info = info;
    }

    pub fn set_options(&mut self, opts: Options) {
        let skip_changed = opts.skip_tx1 != self.opts.skip_tx1;
        let skip = opts.skip_tx1;
        let auto_seq_changed = opts.auto_seq != self.opts.auto_seq;
        let strict_changed = opts.strict_directional_cq != self.opts.strict_directional_cq || opts.my_continent != self.opts.my_continent || opts.my_prefix != self.opts.my_prefix;
        self.opts = opts;
        if strict_changed {
            self.hist.set_own(&self.opts.my_continent, &self.opts.my_prefix, &self.my.grid, self.opts.strict_directional_cq);
        }
        if auto_seq_changed {
            // l'utente che preme AutoSeq annulla l'eventuale spegnimento da messaggio libero (`on_AutoSeqButton_clicked`)
            self.auto_seq_suspended = false;
        }
        // come `on_skipTx1_clicked` di JTDX: se il messaggio scelto e' ancora quello di partenza, accendere «Salta Tx1» porta da Tx1 a Tx2
        // e spegnerlo da Tx2 a Tx1. Se il DX ci ha gia' scritto, Tx2 e' un avanzamento vero del QSO e non si tocca.
        if skip_changed && !self.his_call.is_empty() && !self.dx_has_written() {
            let to = match (skip, self.ntx) {
                (true, 1) => Some(2),
                (false, 2) => Some(1),
                _ => None,
            };
            if let Some(n) = to {
                self.ntx = n;
                if self.transmitting_now() {
                    self.restart = true;
                }
            }
        }
        self.note_state();
    }

    /// Quante volte di fila e' stato trasmesso il messaggio corrente al DX (il `count` di `QsoHistory`).
    pub fn dx_tx_count(&self) -> u32 {
        self.hist.entry(&self.his_call).map_or(0, |t| t.count.max(0) as u32)
    }

    /// Il DX ha mandato almeno un messaggio che avanza il QSO (rapporto, locatore diretto a noi...).
    pub fn dx_has_written(&self) -> bool {
        self.hist
            .entry(&self.his_call)
            .is_some_and(|t| matches!(t.srx_c, Status::Rcall | Status::Rreport | Status::Rrreport | Status::Rrr | Status::Rrr73 | Status::R73))
    }

    /// Cambio di banda: ferma la trasmissione (Abilita Tx spento), libera la stazione DX e svuota la memoria della sequenza
    /// (`clearDX` e `QsoHistory::init` di JTDX). Le impostazioni e la lista dei lavorati restano.
    pub fn band_changed(&mut self) {
        self.reset_all(true);
    }

    /// Cambia il modo (FT8 <-> FT4): i periodi hanno un'altra durata, quindi il QSO in corso e la trasmissione si fermano e si riparte da capo
    /// (`switch_mode`: `QsoHistory::init`). Le impostazioni e la lista dei lavorati restano.
    pub fn set_mode(&mut self, mode: Mode) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        self.cur_slot = Slot(0);
        self.reset_all(true);
    }

    fn reset_all(&mut self, stop: bool) {
        if !self.his_call.is_empty() {
            let old = std::mem::take(&mut self.his_call);
            self.events.push(Event::Abandoned { call: old, reason: AbandonReason::Cleared });
        }
        self.his_grid.clear();
        if stop && self.enabled {
            self.stop(StopReason::Halt);
        }
        self.hist.init();
        self.ds = DisplayState::default();
        self.announced.clear();
        self.last_tx = None;
        self.restart = false;
        self.current_message.clear();
        self.last_message_sent.clear();
        self.status = Status::None;
        self.counter = 0;
        self.process_auto_done = false;
        self.decode_started_for = None;
        self.ntx = 6;
        self.tx_generated = self.tx_even;
        self.rx_hz = self.tx_hz();
        self.note_state();
    }

    /// «Gen.Msgs»: rigenera i messaggi Tx1-Tx5 per il DX in collegamento e rimette il messaggio di partenza (Tx1, o Tx2 con «Salta Tx1»),
    /// come `genStdMsgs` di JTDX. Senza DX non c'e' nulla da rigenerare. I testi si compongono sempre dal DX e dal rapporto correnti.
    pub fn regen_messages(&mut self) {
        // `m_txGenerated = m_txFirst` sta in cima a `genStdMsgs`, prima del controllo sul DX (verificato in `jtdx.exe` a 0x474800)
        self.tx_generated = self.tx_even;
        self.restore_auto_seq();
        if !self.his_call.is_empty() {
            self.ntx = self.start_ntx();
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
        if self.tx_generated != even && self.enabled && self.auto_seq_effective() {
            self.clear_dx();
        }
        // il periodo e' cambiato: l'eventuale trasmissione gia' pianificata in questo periodo non vale piu'
        self.last_tx = None;
        self.restart = false;
    }

    /// AutoSeq e' acceso e non e' stato spento da un messaggio libero.
    pub fn auto_seq_effective(&self) -> bool {
        self.opts.auto_seq && !self.auto_seq_suspended
    }

    /// Il DX in collegamento.
    pub fn dx(&self) -> Option<&str> {
        (!self.his_call.is_empty()).then_some(self.his_call.as_str())
    }

    pub fn dx_grid(&self) -> Option<&str> {
        (!self.his_grid.is_empty()).then_some(self.his_grid.as_str())
    }

    /// La frequenza di ricezione che la sequenza ha scelto (segue il DX, come `process_Auto`).
    pub fn rx_freq(&self) -> u32 {
        self.rx_hz.max(0) as u32
    }

    /// Un cambio di frequenza di ricezione fatto dall'utente (cursore Rx).
    pub fn set_rx_freq(&mut self, hz: u32) {
        self.rx_hz = hz as i32;
    }

    /// Il numero del messaggio da cui parte la prossima trasmissione (6 = CQ, 0 = nessuno).
    pub fn ntx(&self) -> u8 {
        self.shown_ntx()
    }

    pub fn phase(&self) -> Phase {
        if !self.his_call.is_empty() {
            Phase::from_ntx(self.ntx)
        } else if self.enabled && self.ntx == 6 {
            Phase::Cq
        } else {
            Phase::Idle
        }
    }

    /// I chiamanti che aspettano: stazioni che ci hanno scritto nei 5 minuti dopo il nostro ultimo CQ (o nell'ultimo periodo) e che non sono il DX.
    pub fn queued(&self) -> Vec<String> {
        self.hist_callers().into_iter().map(|(call, _)| call).collect()
    }

    fn hist_callers(&self) -> Vec<(String, Sec)> {
        let mut out: Vec<(String, Sec)> = Vec::new();
        for call in &self.announced {
            if same_station(call, &self.his_call) {
                continue;
            }
            if let Some(t) = self.hist.entry(call)
                && matches!(t.status, Status::Rcall | Status::Rreport | Status::Rrreport)
            {
                out.push((t.call.clone(), t.b_time));
            }
        }
        out.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// I sei testi Tx1..Tx6 correnti (vuoti quelli che non si possono comporre senza DX).
    pub fn tx_texts(&self) -> [String; 6] {
        let mut out: [String; 6] = Default::default();
        if !self.his_call.is_empty() {
            let m = self.std_msgs();
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

    /// Il registro delle stazioni (per le prove e per chi vuole mostrare lo stato).
    pub fn history(&self) -> &QsoHistory {
        &self.hist
    }

    // ---- comandi -------------------------------------------------------------------------

    /// Un intervento dell'operatore (clic, tasto, comando): azzera il watchdog come `eventFilter` e `txwatchdog(false)` di JTDX.
    pub fn user_activity(&mut self) {
        self.txwatchdog(false);
    }

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
        let (call, grid, directed) = match parse(&decode.text) {
            Message::Cq { call, grid, .. } => (call, grid, None),
            Message::Directed { to, from, body } => {
                let to_me = same_station(&to, &self.my.call);
                let grid = if let Body::Grid(g) = &body { Some(g.clone()) } else { None };
                (from, grid, to_me.then_some(body))
            }
            Message::Other => return Err(SeqError::NotReplyable(decode.text.clone())),
        };
        if same_station(&call, &self.my.call) {
            return Err(SeqError::OwnCall);
        }
        self.txwatchdog(false);
        let ntx = match &directed {
            Some(Body::Grid(_)) | Some(Body::Other) => 2,
            Some(Body::Report(_)) => 3,
            Some(Body::RReport(_)) => 4,
            Some(Body::Rrr) | Some(Body::Rr73) | Some(Body::Bye) => 5,
            None => self.start_ntx(),
        };
        // la riga cliccata entra nella memoria se non c'era ancora (di norma e' gia' stata letta a fine decodifica)
        let slot_sec = day_secs(slot.start_s(self.mode));
        let known = self.hist.entry(&call).is_some_and(|t| t.time >= slot_sec);
        if !known {
            self.record_rx(slot, decode, true);
        }
        // un altro DX sostituisce il precedente
        let call_changed = base_call(&call) != base_call(&self.his_call) || call != self.his_call;
        if call_changed && !self.his_call.is_empty() {
            let old = std::mem::take(&mut self.his_call);
            self.events.push(Event::Abandoned { call: old, reason: AbandonReason::Replaced });
        }
        self.tx_even = !slot.is_even();
        self.rx_hz = decode.freq_hz as i32;
        if call_changed {
            self.his_grid.clear();
            // un messaggio diretto a me dice gia' a che punto e' il QSO (R-07: ha il mio rapporto): lo stato riparte da li', non da zero,
            // altrimenti al periodo dopo la sequenza tornava al rapporto invece di mandare RR73
            let from_click = match &directed {
                Some(Body::Grid(_)) => Status::Rcall,
                Some(Body::Report(_)) => Status::Rreport,
                Some(Body::RReport(_)) => Status::Rrreport,
                Some(Body::Rrr) => Status::Rrr,
                Some(Body::Rr73) => Status::Rrr73,
                Some(Body::Bye) => Status::R73,
                Some(Body::Other) | None => Status::None,
            };
            self.hist.reset_count(&call, from_click);
            self.his_call = call.clone();
            self.rpt_rcvd.clear();
        }
        if let Some(g) = grid {
            self.his_grid = g;
        }
        self.gen_std_msgs(decode.snr);
        self.ntx = ntx;
        if directed.is_none() && self.opts.skip_tx1 && !self.opts.hound && call_changed {
            // evita che i valori vecchi della memoria rompano la sequenza (`m_qsoHistory.remove(hiscall)`)
            self.hist.remove(&call);
        }
        let b_time = self.hist.entry(&call).map_or(slot_sec, |t| t.b_time);
        self.qso_log.on_sec = b_time;
        self.announced.remove(&call);
        self.last_tx = None;
        self.restart = false;
        if enable {
            self.enabled = true;
        }
        self.note_state();
        Ok(())
    }

    /// Comincia a chiamare CQ, con un modificatore se si vuole («DX», «NA», «POTA», «000»).
    ///
    /// E' il comando `TriggerCQ` con l'invio (`MainWindow`, connessione `trigger_CQ`): preme il Tx6 e poi, se non e' gia' acceso, «Abilita Tx».
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
        self.txwatchdog(false);
        self.opts.cq_modifier = modifier;
        self.txb(6);
        self.enabled = true;
        self.note_state();
        Ok(())
    }

    /// «Ferma Tx»: ferma la trasmissione e le serie di CQ. Il DX resta impostato, come in JTDX:
    /// `set_enabled(true)` riprende da dove si era.
    pub fn halt(&mut self) {
        if self.enabled {
            self.stop(StopReason::Halt);
        } else {
            self.halt_housekeeping();
        }
        self.note_state();
    }

    /// «Abilita Tx». Con `false` equivale a `halt`.
    pub fn set_enabled(&mut self, on: bool) {
        if on {
            self.enabled = true;
            self.note_state();
        } else {
            self.halt();
        }
    }

    /// «Canc DX»: libera la stazione; si torna al messaggio CQ.
    pub fn clear_dx(&mut self) {
        let old = self.his_call.clone();
        self.clear_dx_inner();
        if !old.is_empty() {
            self.events.push(Event::Abandoned { call: old, reason: AbandonReason::Cleared });
        }
        self.note_state();
    }

    /// Scelta a mano del messaggio da cui parte la prossima trasmissione (il pallino «Next»).
    pub fn select_tx(&mut self, n: u8) -> Result<(), SeqError> {
        if !(1..=6).contains(&n) {
            return Err(SeqError::InvalidTxNumber(n));
        }
        if n != 6 && self.his_call.is_empty() {
            return Err(SeqError::NoDx);
        }
        if n == 6 {
            self.restore_auto_seq();
            self.ntx = 6;
        } else {
            self.ntx = n;
        }
        self.note_state();
        Ok(())
    }

    /// Un messaggio libero e' partito (o sta per partire): con AutoSeq acceso e un testo che non e' un 73 (nessuna delle prime quattro parole
    /// e' `73`, `TNX`, `TKS` o `TU`) JTDX spegne AutoSeq e lo riaccende alla generazione dei messaggi successiva (`m_FTsetAutoSeqOff`,
    /// `m_wasAutoSeq`, `isAutoSeq73`).
    pub fn note_free_text(&mut self, text: &str) {
        if self.opts.auto_seq && !self.auto_seq_suspended && !is_autoseq_73(text) {
            self.auto_seq_suspended = true;
        }
        self.note_state();
    }

    /// Dopo la generazione dei messaggi (`genStdMsgs`, `on_txb6_clicked`) AutoSeq torna com'era.
    fn restore_auto_seq(&mut self) {
        self.auto_seq_suspended = false;
    }

    // ---- ricezione -----------------------------------------------------------------------

    /// I messaggi decodificati al termine del periodo `slot` (senza l'orologio: vale il periodo successivo come periodo in trasmissione).
    pub fn on_decodes(&mut self, slot: Slot, decodes: &[Decoded]) {
        self.on_decodes_at(slot, decodes, None);
    }

    /// Come `on_decodes`, con l'ora attuale (secondi UTC), che serve al watchdog.
    ///
    /// Equivale a `readFromStdout` di JTDX: ogni riga aggiorna la memoria della sequenza (anche con AutoSeq spento), il DX che risponde a un altro
    /// ferma la trasmissione, e a fine decodifica (o subito, a seconda del livello di AutoSeq) gira `process_Auto`. Se si sta gia' trasmettendo il
    /// periodo successivo (avvio anticipato) e il messaggio cambia, [`Sequencer::next_tx`] restituisce la ritrasmissione.
    pub fn on_decodes_at(&mut self, slot: Slot, decodes: &[Decoded], now: Option<f64>) {
        self.cur_slot = self.cur_slot.max(slot);
        if let Some(now) = now {
            self.advance_clock(now);
        }
        self.start_decode_if_needed(slot);
        self.rx_transmitting = self.last_tx.as_ref().is_some_and(|(s, _)| s.0 == slot.0 + 1);
        for d in decodes {
            self.read_line(slot, d);
        }
        // `<DecodeFinished>`
        if self.auto_seq_effective() && !self.process_auto_done {
            self.process_auto_done = true;
            self.process_auto();
        }
        self.rx_transmitting = false;
        self.note_state();
    }

    /// Il periodo `slot` comincia a essere decodificato: si azzerano le letture (`decode()`: `m_reply_me`, `m_processAuto_done`...).
    fn start_decode_if_needed(&mut self, slot: Slot) {
        if self.decode_started_for == Some(slot) {
            return;
        }
        self.decode_started_for = Some(slot);
        self.reply_me = false;
        self.reply_other = false;
        self.reply_cq73 = false;
        self.process_auto_done = false;
        self.call_first_73 = false;
        self.used_freq = 0;
    }

    /// Una riga decodificata (`readFromStdout`, ramo della riga).
    fn read_line(&mut self, slot: Slot, d: &Decoded) {
        let msg = words(&d.text).join(" ");
        let freq = d.freq_hz as i32;
        let my_base = self.my_base.clone();
        let his_base = base_call(&self.his_call);
        let (to_call, _) = first_word_call(&d.text);
        let mycallinmsg = !my_base.is_empty() && base_call(&to_call) == my_base;
        let (de_call, _grid) = de_call_and_grid(&d.text);
        let de_is_his = !de_call.is_empty() && !self.his_call.is_empty() && base_call(&de_call) == his_base;
        if de_is_his {
            self.rx_hz = freq;
        }
        if !de_call.is_empty() && !self.reply_me && base_call(&de_call) == his_base {
            if mycallinmsg {
                self.reply_me = true;
                self.reply_other = false;
            } else if msg.contains("73") || msg.starts_with("CQ") {
                self.reply_other = false;
                self.reply_cq73 = true;
            } else if !self.reply_cq73 {
                self.reply_other = true;
            }
        }
        // il rapporto che il DX ci manda (`m_rptRcvd`)
        let (_, rep) = decoded_report(&d.text, &my_base, &his_base);
        if let Some((r, _)) = rep.filter(|(r, _)| !r.is_empty()) {
            self.rpt_rcvd = r;
        }
        // la memoria della sequenza (`displayDecodedText`)
        self.record_rx(slot, d, false);

        if mycallinmsg {
            if de_is_his {
                let has73 = msg.contains(" 73");
                let has_rr73 = msg.contains("RR73");
                if self.auto_seq_effective()
                    && !self.process_auto_done
                    && ((!has73 && !has_rr73)
                        || (has73 && self.status == Status::Rrreport && self.opts.use_rrr)
                        || (has_rr73 && self.status == Status::Rreport)
                        || self.opts.call_mode == 0
                        || self.opts.single_qso
                        || self.opts.hound)
                {
                    self.process_auto_done = true;
                    self.process_auto();
                } else if (has73 || has_rr73) && self.opts.call_mode <= 1 {
                    self.call_first_73 = true;
                }
            } else if self.opts.call_mode == 1 && !self.process_auto_done && self.auto_seq_effective() && self.his_call.is_empty() {
                self.process_auto_done = true;
                self.process_auto();
            }
        } else if de_is_his && !msg.starts_with("CQ ") && !msg.starts_with("DE ") && !msg.starts_with("QRZ ") && !msg.contains(" 73") && !msg.contains(" RR73") && !msg.contains(" RRR") {
            // il DX parla con un altro: se e' vicino alla nostra frequenza (o sempre, con `SeqHaltTxReplyOther`) si ferma la trasmissione
            self.used_freq = freq;
            if self.enabled && !self.reply_me && !self.opts.hound && ((self.used_freq - self.tx_hz()).abs() < self.opts.guard_hz(self.mode) || self.opts.abandon_if_dx_answers_other) {
                self.halt_tx_reason(StopReason::ReplyOther);
            }
        }
    }

    /// `displayDecodedText`: lo stato della riga e, se c'e', la registrazione in `QsoHistory`. `forced` vale per il doppio clic.
    fn record_rx(&mut self, slot: Slot, d: &Decoded, forced: bool) {
        let c_time = day_secs(slot.start_s(self.mode));
        let his_base = base_call(&self.his_call);
        if c_time != 0 && c_time != self.ds_max_r_time {
            self.ds_max_r_time = c_time;
            self.hist.set_time(c_time);
            if !his_base.is_empty() {
                let (st, g) = self.hist.status(&his_base);
                self.ds.mystatus = Some(st);
                self.ds.mygrid = if g.is_empty() { self.his_grid.chars().take(4).collect() } else { g };
                self.ds.myhiscall = his_base.clone();
            }
        }
        let his_grid4: String = self.his_grid.chars().take(4).collect();
        let ctx = RxContext { my_base: &self.my_base, his_base: &his_base, his_grid4: &his_grid4, rx_freq: self.rx_hz, rr73_marker: self.opts.rr73_marker };
        let class = classify_rx(&d.text, &ctx, &mut self.ds, &self.hist, d.freq_hz as i32);
        if class.check_call.is_empty() {
            return;
        }
        let relevant = matches!(class.std_type, 1 | 2 | 4) || (class.std_type == 3 && !class.param.is_empty());
        if !relevant {
            return;
        }
        // un chiamante gia' lavorato non entra tra i candidati se «Risp B4» e' spento (non il DX attuale)
        if class.status == Some(Status::Rcall)
            && !forced
            && !self.opts.answer_worked_before
            && self.is_worked(&class.check_call)
            && !same_station(&class.check_call, &self.his_call)
        {
            self.events.push(Event::CallerIgnored { call: class.check_call.clone() });
            return;
        }
        let info = self.info_of(&class.check_call);
        self.info_cache.insert(base_call(&class.check_call), info.clone());
        let status = class.status.unwrap_or(Status::None);
        let continent: String = info.continent.chars().take(2).collect();
        self.hist.message(&class.check_call, status, i32::from(info.priority), &class.param, &class.tyyp, &continent, &info.prefix, c_time, &rpt_text(d.snr.clamp(-50, 49)), d.freq_hz as i32);
        if status == Status::Rcall && !same_station(&class.check_call, &self.his_call) && self.announced.insert(class.check_call.clone()) {
            self.events.push(Event::CallerQueued { call: class.check_call.clone() });
        }
    }

    fn info_of(&self, call: &str) -> CallerInfo {
        let mut info = self.caller_info.as_ref().map(|f| f(call)).unwrap_or_default();
        if info.continent.trim().is_empty() {
            // JTDX non sceglie mai una stazione senza continente: senza registro nessuna lo ha, quindi vale «??»
            info.continent = crate::history::UNKNOWN_CONTINENT.to_string();
        }
        info
    }

    // ---- process_Auto --------------------------------------------------------------------

    /// `MainWindow::process_Auto` (`REA 0x47d3e0`): legge lo stato della stazione in collegamento, applica i contatori di rinuncia, sceglie il
    /// prossimo chiamante o il CQ e il messaggio da trasmettere.
    fn process_auto(&mut self) {
        let hound = self.opts.hound;
        let mut counters = true;
        let mut counters2 = true;
        self.status = Status::None;
        let mut his_call = self.his_call.clone();
        let mut io = Autoseq { grid: self.his_grid.clone(), rep: self.rpt_text.clone(), rx: self.rx_hz, tx: self.tx_hz(), time: 0, count: 0, prio: 0 };
        if !his_call.is_empty() {
            io.count = if hound { -1 } else { 0 };
            self.status = self.hist.autoseq_call(&his_call, &mut io, hound);
            if self.rx_transmitting {
                io.count -= 1;
            }
            let count = io.count;
            let prio = io.prio;
            let limits = self.opts.limits;
            let guard = self.opts.guard_hz(self.mode);
            let st = self.status;
            if hound {
                // WSJT-X Fox lascia cadere il QSO se l'R+rapporto del Hound non e' decodificato dopo tre trasmissioni
                if matches!(st, Status::Srreport | Status::Rreport) && count > 3 {
                    self.halt_tx_reason(StopReason::Halt);
                    self.hist.reset_count(&his_call, Status::Rcq);
                    self.opts.tx_freq_hz = self.last_calling_freq.max(0) as u32;
                    self.status = Status::Rcq;
                }
            } else if (st == Status::Srr73 || st >= Status::S73)
                && !self.opts.single_qso
                && !self.opts.autolog
                && self.last_logged.as_ref().is_some_and(|(c, _)| *c == self.his_call)
                && !self.opts.follow_dx_freq
                && (io.tx == 1 || (io.rx - self.tx_hz()).abs() > guard)
            {
                // fine QSO senza registrazione automatica: si passa al prossimo senza fermarsi
                self.clear_dx_inner();
                his_call.clear();
                io.grid.clear();
                self.status = Status::None;
            } else if (st == Status::Rcq || st == Status::Scall || (st == Status::Sreport && self.opts.skip_tx1 && !hound))
                && let Some(limit) = limits.answer_cq
                && ((prio > 4 && prio < 17) || prio < 2 || self.opts.strict_directional_cq)
                && (limit as i32 <= count || self.reply_other)
            {
                let old = his_call.clone();
                self.clear_dx_inner();
                self.events.push(Event::Abandoned { call: old.clone(), reason: if self.reply_other { AbandonReason::DxAnsweredOther } else { AbandonReason::NoReply } });
                if self.reply_other {
                    counters2 = false;
                } else {
                    self.counter = limit;
                    self.hist.calllist(&old, io.rep.trim().parse().unwrap_or(0), io.time);
                }
                self.hist.reset_count(&old, Status::None);
                his_call.clear();
                io.grid.clear();
                self.status = Status::None;
                if self.opts.single_qso {
                    counters = false;
                }
            } else if (st == Status::Rcall || (st == Status::Sreport && !self.opts.skip_tx1))
                && let Some(limit) = limits.answer_in_call
                && (limit as i32 <= count || self.reply_other)
            {
                let old = his_call.clone();
                self.clear_dx_inner();
                self.events.push(Event::Abandoned { call: old.clone(), reason: if self.reply_other { AbandonReason::DxAnsweredOther } else { AbandonReason::NoReply } });
                self.hist.calllist(&old, io.rep.trim().parse().unwrap_or(0), io.time);
                self.hist.reset_count(&old, Status::None);
                his_call.clear();
                io.grid.clear();
                self.status = Status::None;
                counters2 = false;
                if self.opts.single_qso {
                    counters = false;
                }
            } else if matches!(st, Status::Rreport | Status::Srreport) && let Some(limit) = limits.sent_r_report && limit as i32 <= count {
                let old = his_call.clone();
                self.clear_dx_inner();
                self.events.push(Event::Abandoned { call: old.clone(), reason: AbandonReason::NoReply });
                self.hist.reset_count(&old, Status::None);
                his_call.clear();
                io.grid.clear();
                self.status = Status::None;
                counters2 = false;
                if self.opts.single_qso {
                    counters = false;
                }
            } else if matches!(st, Status::Rrr | Status::Rrr73 | Status::R73 | Status::Srr73 | Status::S73) && let Some(limit) = limits.closing && limit as i32 <= count {
                let old = his_call.clone();
                // un QSO chiuso da noi (RR73/73 inviato) che non riceve il 73 e' comunque concluso: non e' un tentativo fallito
                let logged = self.last_logged.as_ref().is_some_and(|(c, _)| *c == self.his_call);
                self.clear_dx_inner();
                if !logged {
                    self.events.push(Event::Abandoned { call: old.clone(), reason: AbandonReason::NoReply });
                }
                self.hist.reset_count(&old, Status::None);
                his_call.clear();
                io.grid.clear();
                self.status = Status::None;
                counters2 = false;
                if self.opts.single_qso {
                    counters = false;
                }
            }
        }

        if his_call.is_empty() && counters && !hound && self.opts.call_mode != 0 {
            // la ricerca tra i CQ degli altri vale solo se il prossimo periodo di trasmissione e' quello che segue il periodo decodificato
            let decoded_even = self.decode_started_for.is_some_and(|s| s.is_even());
            let next_is_ours = self.tx_even != decoded_even;
            let mut flags: u32 = 0;
            if self.opts.call_prio_cq && !self.opts.follow_dx_freq && counters2 && self.counter == 0 && next_is_ours {
                flags = algo::CQ_SEARCH;
            } else if self.counter > 0 {
                self.counter -= 1;
            }
            if !self.opts.highlights || self.opts.answer_worked_before {
                flags |= algo::ANSWER_WORKED;
            }
            if !self.opts.highlights || self.opts.call_worked_before {
                flags |= algo::CALL_WORKED;
            } else if self.opts.call_higher_new_call {
                flags |= algo::HIGHER_NEW_CALL;
            }
            if self.opts.report_priority {
                flags |= algo::REPORT_PRIORITY;
            }
            if self.opts.max_distance {
                flags |= algo::MAX_DISTANCE;
            }
            let mut pick_io = Autoseq { rx: self.rx_hz, tx: self.tx_hz(), ..Autoseq::default() };
            let picked = if self.opts.caller_selection == CallerSelection::BestCallerFirst {
                // «Best caller first» di Decodium: punteggio al posto delle priorita' di JTDX
                let cache = &self.info_cache;
                let now = self.ds_max_r_time;
                let scorer = |t: &Qso| {
                    let info = cache.get(&base_call(&t.call));
                    caller_score(&CallerFacts {
                        snr: t.s_rep.trim().parse().ok(),
                        waited_s: f64::from(now.saturating_sub(t.b_time)),
                        dxcc_new: info.is_some_and(|i| i.dxcc_new),
                        dxcc_new_band: info.is_some_and(|i| i.dxcc_new_band),
                        worked_band: info.is_some_and(|i| i.worked_band),
                        worked_elsewhere: info.is_some_and(|i| i.worked_elsewhere),
                        pinned: false,
                    })
                };
                self.hist.autoseq_pick_with(flags, &mut pick_io, Some(&scorer))
            } else {
                self.hist.autoseq_pick(flags, &mut pick_io)
            };
            if let Some(pick) = picked {
                self.status = pick.status;
                self.his_call = pick.call.clone();
                his_call = pick.call.clone();
                io = pick.out;
                if !io.rep.is_empty() && io.rep == self.rpt_text {
                    // forza la rigenerazione dei messaggi (`m_rpt = "-60"`)
                    self.rpt_text = "-60".to_string();
                }
                self.announced.remove(&his_call);
            } else if self.transmitted_ntx != 6 {
                self.txb(6);
            }
        }

        if io.rx > 0 && io.rx != self.rx_hz {
            self.rx_hz = io.rx;
        }
        if self.status > Status::None {
            if !io.grid.is_empty() && self.his_grid.chars().take(4).collect::<String>() != io.grid {
                self.his_grid = io.grid.clone();
            }
            if io.time < 86_400 && self.status < Status::R73 {
                self.qso_log.on_sec = io.time;
            }
            if !io.rep.is_empty() && io.rep != self.rpt_text {
                let n: i32 = io.rep.trim().parse().unwrap_or(0);
                if self.status == Status::Sreport {
                    self.status = if self.opts.skip_tx1 && !hound { Status::Rcq } else { Status::Rreport };
                }
                self.gen_std_msgs(n);
            }
            let skip = self.opts.skip_tx1 && !hound;
            match self.status {
                Status::Rfin | Status::Rcq => self.txb(if skip { 2 } else { 1 }),
                Status::Rcall => {
                    if self.enabled && self.auto_seq_effective() && self.opts.call_mode > 0 {
                        self.txwatchdog(false);
                    }
                    self.txb(2);
                }
                Status::Rreport => {
                    if self.enabled && self.auto_seq_effective() && self.opts.call_mode > 0 {
                        self.txwatchdog(false);
                    }
                    self.txb(3);
                }
                Status::Rrreport => self.txb(4),
                Status::Rrr => self.txb(5),
                Status::Rrr73 => {
                    if !hound {
                        self.txb(5);
                    } else {
                        // fine di un QSO con una spedizione: si registra e si ferma
                        self.log_qso(self.cur_slot, self.cur_slot.start_s(self.mode));
                        if self.enabled || self.rx_transmitting {
                            self.halt_tx_reason(StopReason::QsoFinished);
                        }
                    }
                }
                Status::Srr73 => {
                    if !self.opts.single_qso && !self.opts.autolog && self.last_logged.as_ref().is_some_and(|(c, _)| *c == self.his_call) {
                        self.auto_stop_tx(StopReason::QsoFinished, false);
                    }
                }
                Status::R73 => self.txb(5),
                Status::S73 => self.auto_stop_tx(StopReason::QsoFinished, false),
                Status::Fin => {
                    // tutti i rami di JTDX (1 QSO, registrazione automatica, non registrato, non titolare della frequenza) fermano la trasmissione
                    let reason = if self.opts.single_qso { StopReason::SingleQso } else { StopReason::QsoFinished };
                    self.auto_stop_tx(reason, false);
                }
                _ => {}
            }
        } else {
            if self.enabled && self.his_call.is_empty() {
                self.rx_hz = self.tx_hz();
            }
            if !counters && self.opts.single_qso {
                self.auto_stop_tx(StopReason::SingleQso, true);
            }
        }
    }

    /// `autoStopTx`: ferma la trasmissione e, con la registrazione automatica, libera il DX.
    fn auto_stop_tx(&mut self, reason: StopReason, counter_triggered: bool) {
        if self.enabled || self.rx_transmitting {
            self.halt_tx_reason(reason);
        }
        if self.opts.skip_tx1 && counter_triggered && self.ntx == 2 && !self.his_call.is_empty() {
            // evita che la memoria rompa il prossimo tentativo con questa stazione
            let call = self.his_call.clone();
            self.hist.remove(&call);
        }
        // `autoStopTx`: `(clear_DX || autolog) && !m_hisCall.isEmpty() && !m_houndMode`
        if (self.opts.autolog || self.opts.clear_dx) && !self.his_call.is_empty() && !self.opts.hound {
            self.clear_dx_inner();
        }
    }

    /// `on_txbN_clicked`: sceglie il messaggio; se si sta trasmettendo il messaggio riparte (`m_restart`).
    fn txb(&mut self, n: u8) {
        if n == 6 {
            // `on_txb6_clicked`: svuota il DX, riaccende AutoSeq
            self.restore_auto_seq();
            if !self.his_call.is_empty() {
                self.his_call.clear();
                self.his_grid.clear();
            }
        }
        self.ntx = n;
        if self.rx_transmitting {
            self.restart = true;
        }
    }

    // ---- trasmissione --------------------------------------------------------------------

    /// Cosa trasmettere nel periodo `slot`, se ne tocca a noi. `now` sono i secondi UTC attuali: servono alla finestra di avvio e al watchdog.
    ///
    /// Si puo' partire in qualunque istante dei primi 13,64 s (FT8) o 6,04 s (FT4) del periodo: chi trasmette emette la **coda** del
    /// messaggio dal punto giusto (avvio tardivo). La stessa chiamata ripetuta nello stesso periodo da' lo stesso messaggio senza contarlo due
    /// volte; se nel frattempo la decodifica ha cambiato il messaggio (`m_restart`) da' il nuovo con `retransmit` vero.
    /// Ogni trasmissione ripetuta conta verso i contatori di rinuncia.
    pub fn next_tx(&mut self, slot: Slot, now: f64) -> Option<Tx> {
        self.cur_slot = self.cur_slot.max(slot);
        self.advance_clock(now);
        self.start_decode_if_needed(Slot(slot.0.saturating_sub(1)));
        if !self.enabled {
            return None;
        }
        let in_progress = self.last_tx.as_ref().is_some_and(|(s, _)| *s == slot);
        if self.opts.watchdog_min > 0 && self.idle_minutes >= self.opts.watchdog_min && !in_progress {
            self.txwatchdog(true);
            self.stop(StopReason::Watchdog);
            self.note_state();
            return None;
        }
        if slot.is_even() != self.tx_even {
            return None;
        }
        if let Some((s, old)) = self.last_tx.clone()
            && s == slot
        {
            if !self.restart {
                return Some(old);
            }
            self.restart = false;
            let tx = self.emit_tx(slot, now, true)?;
            self.note_state();
            if tx.text == old.text && tx.freq_hz == old.freq_hz {
                // stesso messaggio: JTDX lo riconta ma l'audio e' lo stesso
                return Some(old);
            }
            return Some(tx);
        }
        if now - slot.start_s(self.mode) > self.opts.late_limit_s(self.mode) {
            self.note_state();
            return None;
        }
        // livello 2 (e 0/1 con un 73): `process_Auto` all'inizio della trasmissione, con quanto e' stato decodificato fin qui
        if self.auto_seq_effective()
            && !self.process_auto_done
            && (self.opts.call_mode == 2 || (self.opts.call_mode <= 1 && (self.call_first_73 || self.status >= Status::Rrr73)))
        {
            self.process_auto_done = true;
            self.process_auto();
            if !self.enabled {
                self.note_state();
                return None;
            }
        }
        let tx = self.emit_tx(slot, now, false);
        self.note_state();
        tx
    }

    /// L'inizio (o la ripartenza) di una trasmissione nel periodo `slot`: genera il messaggio, lo registra nella memoria (`displayTransmittedText`),
    /// registra il QSO se e' il primo 73 e aggiorna i contatori. `None` se non c'e' niente da trasmettere.
    fn emit_tx(&mut self, slot: Slot, now: f64, restart: bool) -> Option<Tx> {
        let n = if self.ntx == 0 { 6 } else { self.ntx };
        let text = self.text_for(n);
        if text.trim().is_empty() {
            // «Transmission of the empty message is not allowed»
            self.halt_tx_reason(StopReason::Halt);
            return None;
        }
        if self.opts.hound && n == 6 {
            // «Transmission of CQ message is not allowed in the Hound mode»
            self.halt_tx_reason(StopReason::Halt);
            return None;
        }
        // in FT8/FT4 JTDX toglie le parentesi `<>` dal testo trasmesso (`m_curMsgTx`) prima di registrarlo
        let plain = text.replace(['<', '>'], "");
        let parts: Vec<&str> = plain.split_whitespace().collect();
        let progress_roger = (3..=5).contains(&n);
        let is_73 = progress_roger && (parts.contains(&"73") || parts.contains(&"RR73"));
        let last_parts: Vec<&str> = self.current_message.split_whitespace().collect();
        let last_is_73 = last_parts.contains(&"73") || last_parts.contains(&"RR73");
        self.last_message_sent = std::mem::take(&mut self.current_message);
        self.current_message = plain.clone();
        let sent_first_73 = is_73
            && self.last_logged.as_ref().is_none_or(|(c, _)| *c != self.his_call)
            && (!last_is_73 || !self.last_message_sent.contains(self.his_call.as_str()));
        if sent_first_73 && !self.his_call.is_empty() {
            let period = self.mode.period_s();
            if self.last_logged.as_ref().is_none_or(|(c, t)| *c != self.his_call || (now - *t).abs() > period.floor() * 2.0) {
                self.log_qso(slot, now);
            }
        }
        // il rapporto che abbiamo mandato (`m_rptSent`)
        if parts.len() >= 3 && (parts[1] == self.my.call || parts[1] == self.my_base) {
            let r = parts[2];
            if let Ok(v) = r.parse::<i32>()
                && (-50..50).contains(&v)
            {
                self.rpt_sent = r.to_string();
            } else if let Some(rest) = r.strip_prefix('R')
                && let Ok(v) = rest.parse::<i32>()
                && (-50..50).contains(&v)
            {
                self.rpt_sent = rest.to_string();
            }
        }
        // la memoria della sequenza (`displayTransmittedText`)
        let ttime = day_secs(now);
        let tx_hz = self.tx_hz();
        let skip_mark = self.opts.skip_tx1 && !self.opts.hound;
        if let Some((call, status, param, tyyp)) = classify_tx(&plain, &self.my_base, &self.his_call, skip_mark) {
            self.hist.message(&call, status, 0, &param, &tyyp, "", "", ttime, "", tx_hz);
        }
        if !restart {
            if self.opts.watchdog_min > 0 && plain != self.msg_sent0 {
                // un messaggio nuovo azzera il watchdog; ripetere lo stesso no
                self.txwatchdog(false);
                self.msg_sent0 = plain.clone();
            }
            if self.auto_seq_effective() && n == 6 && !self.his_call.is_empty() {
                // il campo del DX deve restare vuoto mentre si trasmette il CQ sotto AutoSeq
                self.his_call.clear();
                self.his_grid.clear();
            }
            if self.opts.hound && n == 1 {
                self.last_calling_freq = tx_hz;
            }
            self.transmitted_ntx = n;
            self.pending_tx_end = Some(slot.start_s(self.mode) + self.opts.late_limit_s(self.mode) - 0.5);
        }
        let tx = Tx { n, text, freq_hz: tx_hz.max(0) as u32, even_slot: self.tx_even, retransmit: restart };
        self.last_tx = Some((slot, tx.clone()));
        Some(tx)
    }

    /// Il QSO si registra (`logQSOTimer`): rapporto inviato e ricevuto dalla memoria della sequenza (`log_data`), come in `on_logQSOButton_clicked`.
    fn log_qso(&mut self, slot: Slot, now: f64) {
        if self.his_call.is_empty() {
            return;
        }
        let (status, b_time, r_rep, s_rep) = self.hist.log_data(&self.his_call);
        let (mut rst_sent, mut rst_rcvd, on_sec) = if status > Status::Sreport {
            (s_rep, r_rep, b_time)
        } else {
            (String::new(), String::new(), self.qso_log.on_sec)
        };
        if rst_rcvd.is_empty() {
            rst_rcvd = self.rpt_rcvd.clone();
        }
        if rst_sent.is_empty() {
            rst_sent = self.rpt_sent.clone();
        }
        if rst_sent.is_empty() {
            rst_sent = rpt_text(self.rpt_value());
        }
        let off_ms = slot.start_ms(self.mode);
        let day_ms = (off_ms / 86_400_000) * 86_400_000;
        let mut on_ms = day_ms + u64::from(on_sec) * 1000;
        if on_ms > off_ms {
            on_ms = on_ms.saturating_sub(86_400_000);
        }
        // un QSO lungo piu' di 10 periodi non e' credibile: si prende l'ora di fine (`10 * periodo`)
        if off_ms.abs_diff(on_ms) > (self.mode.period_s() as u64) * 10_000 {
            on_ms = off_ms;
        }
        let record = QsoRecord {
            call: self.his_call.clone(),
            grid: self.his_grid.chars().take(4).collect(),
            rst_sent,
            rst_rcvd,
            mode: self.mode,
            time_on_ms: on_ms,
            time_off_ms: off_ms,
            tx_freq_hz: self.tx_hz().max(0) as u32,
            rx_freq_hz: self.hist.entry(&self.his_call).map_or(self.rx_hz, |t| t.rx).max(0) as u32,
        };
        self.worked.insert(base_call(&self.his_call));
        self.last_logged = Some((self.his_call.clone(), now));
        self.events.push(Event::QsoComplete(record));
    }

    // ---- stato e messaggi ------------------------------------------------------------------

    /// La frequenza di trasmissione (con Tx = Rx e' quella di ricezione).
    fn tx_hz(&self) -> i32 {
        if self.opts.follow_dx_freq && !self.his_call.is_empty() {
            self.rx_hz
        } else {
            self.opts.tx_freq_hz as i32
        }
    }

    fn start_ntx(&self) -> u8 {
        if self.opts.skip_tx1 && !self.opts.hound { 2 } else { 1 }
    }

    fn rpt_value(&self) -> i32 {
        match self.opts.report {
            ReportPolicy::Measured => self.rpt,
            ReportPolicy::Fixed(n) => n,
        }
    }

    /// `genStdMsgs(rpt)`: aggiorna il rapporto e rimette il messaggio di partenza (i testi si compongono a richiesta).
    fn gen_std_msgs(&mut self, rpt: i32) {
        self.restore_auto_seq();
        self.tx_generated = self.tx_even;
        self.rpt = rpt.clamp(-50, 49);
        self.rpt_text = rpt_text(self.rpt_value());
        if !self.his_call.is_empty() {
            self.ntx = self.start_ntx();
        }
    }

    fn std_msgs(&self) -> StdMessages {
        gen_std_msgs(&self.my.call, &self.my.grid, &self.his_call, self.rpt_value(), self.opts.use_rrr)
    }

    fn cq_text(&self) -> String {
        gen_cq(&self.my.call, &self.my.grid, self.opts.cq_modifier.as_deref())
    }

    fn text_for(&self, n: u8) -> String {
        if n == 6 {
            return self.cq_text();
        }
        if self.his_call.is_empty() {
            return String::new();
        }
        self.std_msgs().get(n).map(str::to_string).unwrap_or_default()
    }

    /// `clearDX` senza evento: libera la stazione e rimette il messaggio CQ.
    fn clear_dx_inner(&mut self) {
        self.his_call.clear();
        self.his_grid.clear();
        self.tx_generated = self.tx_even;
        self.rx_hz = self.tx_hz();
        self.ntx = 6;
        self.restore_auto_seq();
    }

    fn transmitting_now(&self) -> bool {
        self.rx_transmitting
    }

    /// `haltTx` per una causa interna: Abilita Tx si spegne e la trasmissione in corso si ferma.
    fn halt_tx_reason(&mut self, reason: StopReason) {
        if self.enabled {
            self.stop(reason);
        } else {
            self.halt_housekeeping();
        }
    }

    fn stop(&mut self, reason: StopReason) {
        self.enabled = false;
        self.halt_housekeeping();
        self.events.push(Event::Stopped(reason));
    }

    /// Quanto fa `on_stopTxButton_clicked` oltre a spegnere Abilita Tx.
    fn halt_housekeeping(&mut self) {
        if self.last_tx.is_some() || self.rx_transmitting {
            if self.opts.skip_tx1 && !self.his_call.is_empty() && self.ntx == 2 {
                let call = self.his_call.clone();
                self.hist.remove(&call);
            }
            self.sec_tx_stopped = self.sec_tx_stopped.max(0.0);
        }
        self.last_tx = None;
        self.restart = false;
        self.pending_tx_end = None;
    }

    // ---- watchdog ---------------------------------------------------------------------------

    /// `txwatchdog`: con `false` azzera i minuti di inattivita'; con `true` il watchdog e' scaduto.
    fn txwatchdog(&mut self, triggered: bool) {
        self.watchdog_expired = triggered;
        if !triggered {
            self.idle_minutes = 0;
        }
    }

    /// `on_the_minute`: a ogni minuto UTC si somma un minuto di inattivita' solo se si e' trasmesso negli ultimi 32 s (FT8) o 16 s (FT4).
    fn advance_clock(&mut self, now: f64) {
        let cur_min = (now / 60.0).floor() as i64;
        let last = *self.last_minute.get_or_insert(cur_min);
        let limit = match self.mode {
            Mode::Ft8 => 32.0,
            Mode::Ft4 => 16.0,
            Mode::Jt65 | Mode::Jt9 => 32.0,
        };
        for m in (last + 1)..=cur_min {
            let t = m as f64 * 60.0;
            if let Some(end) = self.pending_tx_end
                && end <= t
            {
                self.sec_tx_stopped = end;
                self.pending_tx_end = None;
            }
            if self.opts.watchdog_min == 0 {
                self.idle_minutes = 0;
                continue;
            }
            let update = self.watchdog_expired || t - self.sec_tx_stopped <= limit;
            if update && self.idle_minutes < self.opts.watchdog_min {
                self.idle_minutes += 1;
            }
        }
        self.last_minute = Some(cur_min);
        if let Some(end) = self.pending_tx_end
            && end <= now
        {
            self.sec_tx_stopped = end;
            self.pending_tx_end = None;
        }
    }

    fn shown_ntx(&self) -> u8 {
        if !self.his_call.is_empty() {
            self.ntx
        } else if self.enabled && self.ntx == 6 {
            6
        } else {
            0
        }
    }

    fn note_state(&mut self) {
        let key = (self.phase(), (!self.his_call.is_empty()).then(|| self.his_call.clone()), self.shown_ntx());
        if self.reported.as_ref() != Some(&key) {
            self.events.push(Event::StateChanged { phase: key.0, dx: key.1.clone(), ntx: key.2 });
            self.reported = Some(key);
        }
    }

    /// Il nominativo e' standard per l'FT8 (utile a chi compone i messaggi a mano).
    pub fn my_call_is_standard(&self) -> bool {
        is_std_call(&self.my.call)
    }
}

/// `isAutoSeq73`: una delle prime quattro parole e' `73`, `TNX`, `TKS` o `TU`.
pub fn is_autoseq_73(text: &str) -> bool {
    text.split_whitespace().take(4).any(|w| matches!(w, "73" | "TNX" | "TKS" | "TU"))
}

/// La prima parola di un messaggio se e' un nominativo (`DecodedText::call`), con il testo normalizzato.
fn first_word_call(text: &str) -> (String, String) {
    let w = words(text);
    let first = w.first().cloned().unwrap_or_default();
    let is_call = matches!(parse(&format!("{first} X1ABC")), Message::Directed { .. });
    (if is_call && !matches!(first.as_str(), "CQ" | "DE" | "QRZ") { first } else { String::new() }, w.join(" "))
}
