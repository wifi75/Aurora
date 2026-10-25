//! La trasmissione del motore proprio: unisce la sequenza dei QSO (`qso-native`), la modulazione (`ft8::modem` o `ft4::modem`, secondo il modo)
//! e la catena audio/PTT (`tx-native`). A ogni periodo decodificato la sequenza guarda cosa e' arrivato e dice cosa trasmettere nel periodo che
//! comincia; il messaggio diventa un'onda e parte a inizio periodo (+0,5 s). Il doppio clic su un messaggio, il CQ, STOP e le opzioni arrivano
//! come [`TxCommand`]. Il doppio clic, il CQ e il messaggio libero, se il periodo in corso e' quello giusto e non e' troppo tardi, partono subito
//! senza aspettare la fine del periodo (come JTDX, che ammette gli avvii tardivi).

use std::{
    collections::VecDeque,
    sync::{Arc, atomic::Ordering},
};

use aurora_core::QsoRecord;
use ft8::message::CallHashes;
use qso_native::{Decoded as QDecoded, Event as QEvent, Mode, Options, Sequencer, Slot, Station};
use tx_native::{Parity, Transmitter, TxRequest};

use crate::rig::NativeControls;

const DAY_MS: u64 = 86_400_000;
const RECENT_KEEP: usize = 300;
/// Un messaggio di JTDX e' lungo al massimo 37 caratteri (`character*37 msg` di `pack77`); se non e' standard si trasmettono i primi 13.
pub const MESSAGE_MAX: usize = 37;
/// Caratteri del testo libero.
pub const FREE_TEXT_MAX: usize = 13;

/// Chi sa far partire e fermare una trasmissione (la catena vera, o un finto nelle prove).
pub trait TxPort: Send + Sync {
    fn schedule(&self, request: TxRequest) -> Result<(), String>;
    fn halt(&self);
    /// Attenuazione dell'uscita audio (cursore Pwr), in decimi di dB, da usare per le prossime trasmissioni.
    fn set_attenuation(&self, _tenths: u32) {}
}

impl TxPort for Transmitter {
    fn schedule(&self, request: TxRequest) -> Result<(), String> {
        Transmitter::schedule(self, request).map(|_| ()).map_err(|e| e.to_string())
    }

    fn halt(&self) {
        Transmitter::halt(self);
    }

    fn set_attenuation(&self, tenths: u32) {
        let mut config = self.config();
        config.attenuation_tenths = Some(tenths);
        let _ = self.set_config(config);
    }
}

/// Cambi alle opzioni della sequenza (i pulsanti AutoSeq, Salta Tx1, 1 QSO...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionChange {
    AutoSeq(bool),
    SkipTx1(bool),
    Bypass(bool),
    SingleQso(bool),
    AnswerWorkedBefore(bool),
    UseRrr(bool),
    TxEven(bool),
}

/// Cio' che arriva dalla pagina, dall'automazione e dai pulsanti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxCommand {
    /// Doppio clic su un messaggio decodificato (o risposta automatica): imposta il DX e comincia.
    Reply { decode_id: u64 },
    CallCq { modifier: Option<String> },
    SetTxFreq(u32),
    /// STOP TX: ferma subito audio e PTT e spegne la sequenza.
    Halt,
    /// Messaggio libero: con testo lo prepara (come scriverlo nel campo Tx5), con `send` lo trasmette una volta; `send` senza testo
    /// trasmette quello preparato; senza testo e senza `send` lo svuota.
    FreeText { text: String, send: bool },
    /// «Abilita Tx» (`m_enableTx`): permette di trasmettere. Spento, non parte niente.
    Enable(bool),
    /// «AutoTX» (`m_autoTx`, impostazione `QuickCall`, di fabbrica acceso): serve solo al doppio clic (e alla risposta via comando), che accende
    /// «Abilita Tx». Da solo non fa trasmettere e non lo spegne nessun altro comando (verificato in `jtdx.exe`: `on_AutoTxButton_clicked` 0x4aa4d0).
    AutoTx(bool),
    Option(OptionChange),
    SelectTx(u8),
    ClearDx,
    /// «Gen.Msgs»: rigenera i messaggi per il DX attuale.
    GenMsgs,
    /// «Cancella»: svuota i messaggi ricordati per il doppio clic.
    Erase,
}

/// Cio' che la pagina mostra della trasmissione.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TxView {
    pub texts: [String; 6],
    pub selected: u8,
    pub dx: Option<String>,
    pub dx_grid: Option<String>,
    pub enabled: bool,
}

struct Recent {
    id: u64,
    text: String,
    snr: i32,
    freq_hz: u32,
    /// Inizio del periodo in cui e' stato ricevuto (ms dal 1970): la parita' del periodo di risposta si ricava da qui, non dall'ora del clic.
    slot_start_ms: u64,
}

pub struct TxDriver<P: TxPort + ?Sized> {
    seq: Sequencer,
    opts: Options,
    port: Arc<P>,
    controls: Arc<NativeControls>,
    recent: VecDeque<Recent>,
    /// Messaggio libero in coda per la prossima trasmissione.
    free_text: Option<String>,
    /// «AutoTX» acceso: il doppio clic accende anche «Abilita Tx».
    auto_tx: bool,
    /// Messaggio libero preparato (campo Tx5): lo mostra la pagina, «Invia» senza testo lo trasmette.
    prepared: Option<String>,
    hashes: CallHashes,
    mode: Mode,
    /// L'ultimo periodo per cui si e' chiesta una trasmissione (non si chiede due volte).
    last_planned: Option<Slot>,
    /// Le trasmissioni consegnate alla catena e non ancora cominciate: servono a scrivere la riga di `ALL.TXT` quando partono davvero.
    notes: Vec<TxNote>,
}

/// Una trasmissione consegnata alla catena audio/PTT: il periodo in cui parte e che cosa e' stato modulato (per `ALL.TXT`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxNote {
    /// Inizio del periodo di trasmissione (ms dal 1970).
    pub start_ms: u64,
    pub mode: Mode,
    /// Frequenza audio (Hz) della trasmissione.
    pub tx_df: u32,
    /// Il messaggio davvero inviato (dopo l'eventuale taglio del testo libero).
    pub text: String,
}

/// Testo di un messaggio come lo accetta JTDX: maiuscolo, spazi singoli, solo l'alfabeto dei messaggi, al massimo 37 caratteri.
pub fn clean_message(text: &str) -> String {
    let up: String = text.to_uppercase().chars().filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || " +-./?@#<>".contains(*c)).collect();
    up.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(MESSAGE_MAX).collect::<String>().trim().to_string()
}

impl<P: TxPort + ?Sized> TxDriver<P> {
    pub fn new(call: &str, grid: &str, port: Arc<P>, controls: Arc<NativeControls>) -> Result<TxDriver<P>, String> {
        let mode = controls.mode();
        let opts = Options { tx_freq_hz: controls.tx_df.load(Ordering::Relaxed), ..Options::default() };
        let seq = Sequencer::new(mode, Station { call: call.to_string(), grid: grid.to_string() }, opts.clone()).map_err(|e| format!("{e:?}"))?;
        let auto_tx = controls.switch_on("auto_tx");
        let driver = TxDriver { seq, opts, port, controls, recent: VecDeque::new(), free_text: None, prepared: None, hashes: CallHashes::default(), mode, last_planned: None, auto_tx, notes: Vec::new() };
        driver.publish_view();
        Ok(driver)
    }

    /// Ferma la catena e dimentica le trasmissioni annunciate e non ancora partite.
    fn halt_tx(&mut self) {
        self.port.halt();
        self.notes.clear();
    }

    /// Le trasmissioni il cui periodo e' cominciato entro `now_ms`: si tolgono dall'elenco e si restituiscono (per scriverle in `ALL.TXT`).
    pub fn due_notes(&mut self, now_ms: u64) -> Vec<TxNote> {
        let (due, later): (Vec<TxNote>, Vec<TxNote>) = std::mem::take(&mut self.notes).into_iter().partition(|n| n.start_ms <= now_ms);
        self.notes = later;
        due
    }

    /// Il modo in cui lavora la trasmissione.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Cambio di modo (FT8 <-> FT4): ferma la trasmissione in corso, ripulisce i messaggi ricordati e riparte la sequenza da capo.
    pub fn set_mode(&mut self, mode: Mode) -> Vec<String> {
        if mode == self.mode {
            return Vec::new();
        }
        self.halt_tx();
        self.seq.set_mode(mode);
        self.mode = mode;
        self.recent.clear();
        self.free_text = None;
        self.prepared = None;
        self.last_planned = None;
        self.publish_view();
        vec![format!("modo {}: trasmissione ferma e messaggi ricordati svuotati", mode.name())]
    }

    /// Cambio di banda (da qualunque causa: pagina, radio, spot, modo che sposta la frequenza): ferma sempre la trasmissione (PTT spento, audio
    /// fermo), spegne Abilita Tx/AutoTx, azzera la stazione DX con i messaggi di sequenza e svuota i messaggi ricordati per il doppio clic.
    /// JTDX (`band_changed`, `clearDX`, `QsoHistory::init`) azzera il DX e la memoria della sequenza e ferma solo nel «QSY cieco»; qui si ferma sempre.
    pub fn band_changed(&mut self) -> Vec<String> {
        self.halt_tx();
        self.seq.band_changed();
        self.recent.clear();
        self.free_text = None;
        self.prepared = None;
        self.last_planned = None;
        self.publish_view();
        vec!["cambio di banda: trasmissione ferma, Tx spento, DX e messaggi ricordati azzerati".to_string()]
    }

    /// Ricorda un messaggio decodificato, per il doppio clic (arriva solo l'identificativo). `slot_start_ms` e' l'inizio del periodo in cui e' arrivato.
    pub fn remember(&mut self, id: u64, text: &str, snr: i32, freq_hz: u32, slot_start_ms: u64) {
        self.recent.push_back(Recent { id, text: text.to_string(), snr, freq_hz, slot_start_ms });
        while self.recent.len() > RECENT_KEEP {
            self.recent.pop_front();
        }
    }

    fn publish_view(&self) {
        let mut texts = self.seq.tx_texts();
        if let Some(t) = &self.prepared {
            texts[4] = t.clone();
        }
        let view = TxView {
            texts,
            selected: self.seq.ntx(),
            dx: self.seq.dx().map(str::to_string),
            dx_grid: self.seq.dx_grid().map(str::to_string),
            enabled: self.seq.is_enabled(),
        };
        if let Ok(mut slot) = self.controls.tx_view.lock() {
            *slot = view;
        }
        // il periodo lo cambia anche il doppio clic (opposto a quello della riga): il pulsante deve mostrarlo
        self.controls.set_switch("tx_period", self.seq.tx_even());
    }

    /// Un comando. Restituisce le righe da scrivere nel registro (cosa e' successo e perche' non e' partito). `now_s` sono i secondi UTC attuali.
    pub fn handle(&mut self, command: TxCommand, now_s: f64) -> Vec<String> {
        let mut log = Vec::new();
        match command {
            TxCommand::Reply { decode_id } => match self.recent.iter().find(|r| r.id == decode_id) {
                Some(r) => {
                    // il periodo e' quello in cui il messaggio e' stato ricevuto: si risponde nel periodo opposto, anche se il clic
                    // arriva quando e' gia' cominciato il successivo
                    let slot = Slot(r.slot_start_ms / self.mode.period_ms());
                    let (text, freq) = (r.text.clone(), r.freq_hz);
                    let decode = QDecoded::new(&r.text, r.snr, r.freq_hz);
                    match self.seq.reply_to_with(slot, &decode, self.auto_tx) {
                        Ok(()) => {
                            self.prepared = None;
                            self.free_text = None;
                            self.follow_decode_frequency(freq);
                            log.push(if self.auto_tx { format!("risposta a «{text}»: Tx abilitato") } else { format!("risposta a «{text}»: AutoTX spento, Abilita Tx resta com'e'") });
                            // un messaggio gia' pianificato per il periodo in corso o per il prossimo e' superato: si riparte col nuovo (come `m_restart` di JTDX)
                            if self.last_planned.is_some_and(|planned| planned >= Slot::at(self.mode, now_s)) {
                                self.halt_tx();
                                self.last_planned = None;
                            }
                            log.extend(self.start_now(now_s));
                        }
                        Err(e) => log.push(format!("risposta non possibile: {e:?}")),
                    }
                }
                None => log.push(format!("messaggio {decode_id} non piu' in memoria: non e' possibile rispondere")),
            },
            TxCommand::CallCq { modifier } => match self.seq.call_cq(modifier.as_deref()) {
                Ok(()) => {
                    self.free_text = None;
                    self.seq.set_enabled(true);
                    log.push("CQ: Tx abilitato".to_string());
                    log.extend(self.start_now(now_s));
                }
                Err(e) => log.push(format!("CQ non possibile: {e:?}")),
            },
            TxCommand::SetTxFreq(hz) => {
                if (100..=5000).contains(&hz) {
                    self.controls.tx_df.store(hz, Ordering::Relaxed);
                    // con Tx = Rx (Split spento) la ricezione segue la trasmissione, come con il blocco di JTDX
                    if !self.controls.switch_on("split") {
                        self.controls.rx_df.store(hz, Ordering::Relaxed);
                    }
                } else {
                    log.push(format!("frequenza Tx fuori campo: {hz} Hz"));
                }
            }
            TxCommand::Halt => {
                self.halt_tx();
                self.seq.halt();
                self.free_text = None;
                log.push("STOP TX".to_string());
            }
            TxCommand::FreeText { text, send } => {
                let clean = clean_message(&text);
                if !clean.is_empty() {
                    self.prepared = Some(clean);
                } else if !send {
                    self.prepared = None;
                }
                if send {
                    match self.prepared.clone() {
                        Some(t) => {
                            log.push(format!("messaggio libero «{t}» in coda per il prossimo periodo"));
                            self.free_text = Some(t);
                            self.seq.set_enabled(true);
                            log.extend(self.start_now(now_s));
                        }
                        None => log.push("messaggio libero: nessun testo da inviare".to_string()),
                    }
                }
            }
            TxCommand::AutoTx(on) => self.auto_tx = on,
            TxCommand::Enable(on) => {
                self.seq.set_enabled(on);
                if on {
                    log.extend(self.start_now(now_s));
                } else {
                    self.halt_tx();
                    self.free_text = None;
                }
            }
            TxCommand::Option(change) => {
                match change {
                    OptionChange::AutoSeq(v) => self.opts.auto_seq = v,
                    OptionChange::SkipTx1(v) => self.opts.skip_tx1 = v,
                    OptionChange::Bypass(v) => self.opts.bypass_tx1 = v,
                    OptionChange::SingleQso(v) => self.opts.single_qso = v,
                    OptionChange::AnswerWorkedBefore(v) => self.opts.answer_worked_before = v,
                    OptionChange::UseRrr(v) => self.opts.use_rrr = v,
                    OptionChange::TxEven(v) => {
                        let changed = self.seq.tx_even() != v;
                        self.opts.tx_even = v;
                        self.seq.set_tx_even(v);
                        // cambiare periodo con una trasmissione in corso la ferma (JTDX: «Tx halted: period changed»)
                        if changed && self.controls.transmitting.load(Ordering::Relaxed) {
                            self.halt_tx();
                            log.push("periodo di trasmissione cambiato: trasmissione ferma".to_string());
                        }
                        self.last_planned = None;
                    }
                }
                self.seq.set_options(self.opts.clone());
            }
            TxCommand::SelectTx(n) => {
                self.free_text = None;
                if let Err(e) = self.seq.select_tx(n) {
                    log.push(format!("Tx{n} non selezionabile: {e:?}"));
                }
            }
            TxCommand::ClearDx => {
                self.seq.clear_dx();
                self.prepared = None;
            }
            TxCommand::GenMsgs => {
                self.seq.regen_messages();
                self.free_text = None;
                self.prepared = None;
                log.push("messaggi rigenerati".to_string());
            }
            TxCommand::Erase => self.recent.clear(),
        }
        self.publish_view();
        log
    }

    /// Risposta a un messaggio: la ricezione va sulla sua frequenza e, con Tx = Rx (Split spento), anche la trasmissione (`processMessage` di JTDX).
    fn follow_decode_frequency(&self, freq_hz: u32) {
        if (100..=5000).contains(&freq_hz) {
            self.controls.rx_df.store(freq_hz, Ordering::Relaxed);
            if !self.controls.switch_on("split") {
                self.controls.tx_df.store(freq_hz, Ordering::Relaxed);
            }
        }
    }

    /// Chiede subito la trasmissione nel periodo in corso, se tocca a questo periodo e non e' troppo tardi.
    fn start_now(&mut self, now_s: f64) -> Vec<String> {
        let slot = Slot::at(self.mode, now_s);
        self.plan(slot, now_s)
    }

    /// Un periodo e' stato decodificato (o saltato: elenco vuoto). `slot_start_ms` e' l'inizio di quel periodo. Fa avanzare la sequenza e,
    /// se serve, prepara la trasmissione del periodo che comincia ora. Restituisce i QSO conclusi e le righe di registro.
    pub fn after_slot(&mut self, slot_start_ms: u64, decoded: &[(String, i32, u32)], now_s: f64, dial_hz: u64, band: &str) -> (Vec<QsoRecord>, Vec<String>) {
        let mut log = Vec::new();
        let slot = Slot(slot_start_ms / self.mode.period_ms());
        let list: Vec<QDecoded> = decoded.iter().map(|(text, snr, freq)| QDecoded::new(text, *snr, *freq)).collect();
        self.seq.on_decodes(slot, &list);

        let mut qsos = Vec::new();
        for event in self.seq.drain_events() {
            match event {
                QEvent::QsoComplete(rec) => {
                    // come JTDX (`on_logQSOButton_clicked`): l'inizio e' il primo periodo del QSO, la fine e' l'istante in cui comincia a trasmettere
                    // RR73 o 73 (il periodo dopo l'ultima ricezione, se il QSO si chiude in ricezione) piu' 14 s in FT8 e 7 s in FT4
                    let tx_slot_ms = rec.time_off_ms.max(slot_start_ms + self.mode.period_ms());
                    let tail_ms = if self.mode.name() == "FT4" { 7_000 } else { 14_000 };
                    qsos.push(QsoRecord {
                        call: rec.call.clone(),
                        grid: rec.grid.clone(),
                        band: band.to_string(),
                        mode: self.mode.name().to_string(),
                        freq_hz: dial_hz + u64::from(rec.tx_freq_hz),
                        rst_sent: rec.rst_sent.clone(),
                        rst_rcvd: rec.rst_rcvd.clone(),
                        time_ms: (rec.time_on_ms % DAY_MS) as u32,
                        time_on_unix_ms: rec.time_on_ms,
                        time_off_unix_ms: tx_slot_ms + tail_ms,
                    });
                }
                QEvent::Abandoned { call, reason } => log.push(format!("QSO con {call} abbandonato: {reason:?}")),
                QEvent::Stopped(reason) => log.push(format!("trasmissione automatica ferma: {reason:?}")),
                QEvent::StateChanged { .. } | QEvent::CallerQueued { .. } | QEvent::CallerIgnored { .. } => {}
            }
        }

        log.extend(self.plan(slot.next(), now_s));
        self.publish_view();
        (qsos, log)
    }

    /// Prepara la trasmissione del periodo `slot` (se tocca a lui): sceglie il messaggio, lo modula nel modo corrente e lo consegna alla catena.
    fn plan(&mut self, slot: Slot, now_s: f64) -> Vec<String> {
        let mut log = Vec::new();
        if self.last_planned == Some(slot) {
            return log;
        }
        // la frequenza Tx e' quella dei comandi (Tx -> Rx, Rx -> Tx, barra della cascata, Split): la sequenza la legge qui
        let tx_df = self.controls.tx_df.load(Ordering::Relaxed);
        if self.opts.tx_freq_hz != tx_df {
            self.opts.tx_freq_hz = tx_df;
            self.seq.set_options(self.opts.clone());
        }
        let parity = if slot.is_even() { Parity::Even } else { Parity::Odd };
        // il messaggio libero prende il posto della sequenza nel suo periodo (JTDX: il campo Tx5 diventa il messaggio scelto)
        // JTDX non lo toglie dopo l'invio (`m_ntx` resta 8, `guiUpdate` non lo cambia): si ripete a ogni periodo di trasmissione finche' Abilita Tx
        // e' acceso e non si sceglie un altro messaggio
        let planned = if self.free_text.is_some() && self.seq.is_enabled() && slot.is_even() == self.seq.tx_even() {
            self.free_text.clone().map(|text| (text, self.opts.tx_freq_hz, 0u8))
        } else {
            self.seq.next_tx(slot, now_s).map(|tx| (tx.text, tx.freq_hz, tx.n))
        };
        if let Some((text, freq, n)) = planned {
            self.last_planned = Some(slot);
            self.port.set_attenuation(self.controls.attenuation.load(Ordering::Relaxed));
            match self.encode(&text, freq, n == 0) {
                Ok((wave, sent)) => {
                    let mode = match self.mode {
                        Mode::Ft8 => tx_native::Mode::Ft8,
                        Mode::Ft4 => tx_native::Mode::Ft4,
                    };
                    match self.port.schedule(TxRequest { wave, mode, parity }) {
                        Ok(()) => {
                            self.notes.push(TxNote { start_ms: slot.start_ms(self.mode), mode: self.mode, tx_df: freq, text: sent.clone() });
                            log.push(format!("trasmetto (Tx{n}) {} a {freq} Hz: {sent}", self.mode.name()))
                        }
                        Err(e) => log.push(format!("trasmissione non partita: {e}")),
                    }
                }
                Err(e) => log.push(format!("messaggio non codificabile «{text}»: {e}")),
            }
        }
        log
    }

    /// Il testo diventa un'onda a 12 kHz nel modo corrente. Un testo libero oltre i 13 caratteri non e' un messaggio standard: come JTDX
    /// (`pack77`) se ne trasmettono i primi 13. Restituisce l'onda e il testo davvero inviato.
    fn encode(&mut self, text: &str, freq: u32, free: bool) -> Result<(Vec<f32>, String), String> {
        let mode = self.mode;
        let mut run = |t: &str| -> Result<Vec<f32>, String> {
            match mode {
                Mode::Ft8 => ft8::modem::encode_message(t, freq as f32, &mut self.hashes),
                Mode::Ft4 => ft4::modem::encode_message(t, freq as f32, &mut self.hashes),
            }
            .map_err(|e| format!("{e:?}"))
        };
        match run(text) {
            Ok(wave) => Ok((wave, text.to_string())),
            Err(e) if free && text.chars().count() > FREE_TEXT_MAX => {
                let cut: String = text.chars().take(FREE_TEXT_MAX).collect::<String>().trim().to_string();
                run(&cut).map(|wave| (wave, cut)).map_err(|_| e)
            }
            Err(e) => Err(e),
        }
    }
}

/// Lo stato osservabile della trasmissione per la pagina.
pub fn view_of(controls: &NativeControls) -> TxView {
    controls.tx_view.lock().map(|v| v.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::NativeRig;
    use rig_control::Rig;
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct FakePort {
        requests: StdMutex<Vec<TxRequest>>,
        halts: StdMutex<u32>,
    }

    impl TxPort for FakePort {
        fn schedule(&self, request: TxRequest) -> Result<(), String> {
            self.requests.lock().map_err(|_| "avvelenato".to_string())?.push(request);
            Ok(())
        }

        fn halt(&self) {
            if let Ok(mut h) = self.halts.lock() {
                *h += 1;
            }
        }
    }

    /// Il pilota con la «radio» della pagina collegata: i pulsanti passano dal canale come nel motore vero.
    struct Bench {
        d: TxDriver<FakePort>,
        port: Arc<FakePort>,
        controls: Arc<NativeControls>,
        rig: NativeRig,
        rx: std::sync::mpsc::Receiver<TxCommand>,
    }

    impl Bench {
        fn new() -> Bench {
            let port = Arc::new(FakePort::default());
            let controls = Arc::new(NativeControls::default());
            let (tx, rx) = std::sync::mpsc::channel();
            controls.set_sender(tx);
            let d = TxDriver::new("IU3ABC", "IO91", Arc::clone(&port), Arc::clone(&controls)).expect("pilota");
            let rig = NativeRig::new(Arc::clone(&controls), None);
            Bench { d, port, controls, rig, rx }
        }

        /// Serve i comandi arrivati dalla pagina (come fa il ciclo del motore) e restituisce il registro.
        fn pump(&mut self, now_s: f64) -> Vec<String> {
            let mut log = Vec::new();
            while let Ok(c) = self.rx.try_recv() {
                log.extend(self.d.handle(c, now_s));
            }
            log
        }

        fn press(&mut self, id: &str, on: Option<bool>, now_s: f64) -> Vec<String> {
            self.rig.set_control(id, on).expect("comando");
            self.pump(now_s)
        }

        fn requests(&self) -> Vec<(tx_native::Mode, Parity, usize)> {
            self.port.requests.lock().expect("richieste").iter().map(|r| (r.mode, r.parity, r.wave.len())).collect()
        }

        fn texts(&self) -> [String; 6] {
            view_of(&self.controls).texts
        }
    }

    fn driver() -> (TxDriver<FakePort>, Arc<FakePort>, Arc<NativeControls>) {
        let port = Arc::new(FakePort::default());
        let controls = Arc::new(NativeControls::default());
        let d = TxDriver::new("IU3ABC", "IO91", Arc::clone(&port), Arc::clone(&controls)).expect("pilota");
        (d, port, controls)
    }

    // 100 000 000 s dall'epoca e' un periodo FT8 pari (6 666 666 periodi: 6 666 666 e' pari)
    const SLOT_MS: u64 = 99_999_990_000;
    const BASE: f64 = SLOT_MS as f64 / 1000.0;
    // un periodo FT4 pari (13 333 334 periodi da 7,5 s)
    const FT4_SLOT_MS: u64 = 100_000_005_000;
    const FT4_BASE: f64 = FT4_SLOT_MS as f64 / 1000.0;

    /// Rumore deterministico.
    fn noise(len: usize, seed: u64) -> Vec<f32> {
        let mut s = seed;
        (0..len)
            .map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((s >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1
            })
            .collect()
    }

    /// I messaggi che un ricevitore leggerebbe dall'onda consegnata al trasmettitore: la si mette in un periodo con rumore e la si decodifica.
    fn decode_request(req: &TxRequest) -> Vec<String> {
        match req.mode {
            tx_native::Mode::Ft8 => {
                let mut audio = noise(180_000, 11);
                for (i, w) in req.wave.iter().enumerate() {
                    audio[6_000 + i] += 0.05 * w;
                }
                ft8::decode::decode_slot(&audio, &ft8::decode::Options::default(), &mut CallHashes::default()).into_iter().map(|d| d.text).collect()
            }
            tx_native::Mode::Ft4 => {
                let mut audio = noise(90_000, 12);
                // l'onda parte un simbolo di rampa prima del primo simbolo di sincronismo, che comincia a 0,5 s
                for (i, w) in req.wave.iter().enumerate() {
                    audio[6_000 - ft4::NSPS + i] += 0.05 * w;
                }
                ft4::decode::decode_slot(&audio, &ft4::decode::Options::default(), &mut CallHashes::default()).into_iter().map(|d| d.text).collect()
            }
        }
    }

    fn last_text(port: &FakePort) -> Vec<String> {
        let requests = port.requests.lock().expect("richieste");
        requests.last().map(decode_request).unwrap_or_default()
    }

    #[test]
    fn a_reply_transmits_in_the_next_slot_with_the_standard_message() {
        let (mut d, port, _) = driver();
        d.remember(7, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        let log = d.handle(TxCommand::Reply { decode_id: 7 }, BASE + 14.0);
        assert!(log.iter().any(|l| l.contains("K1ABC")), "{log:?}");
        let (_, log) = d.after_slot(SLOT_MS, &[("CQ K1ABC FN42".to_string(), -10, 1200)], BASE + 14.3, 14_074_000, "20m");
        let requests = port.requests.lock().expect("richieste");
        assert_eq!(requests.len(), 1, "{log:?}");
        assert!(requests[0].wave.len() > 100_000, "un'onda vera di FT8");
        assert_eq!(requests[0].mode, tx_native::Mode::Ft8);
        // il CQ e' arrivato in un periodo pari: si risponde nei dispari
        assert_eq!(requests[0].parity, Parity::Odd);
    }

    /// Per `ALL.TXT`: una trasmissione consegnata alla catena si annuncia con il suo periodo e il testo davvero modulato, esce una volta sola quando
    /// il periodo comincia, e un STOP la cancella prima che parta.
    #[test]
    fn a_planned_transmission_is_announced_for_the_log_once_when_its_slot_starts_and_stop_cancels_it() {
        let (mut d, _port, _) = driver();
        d.remember(7, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        d.handle(TxCommand::Reply { decode_id: 7 }, BASE + 14.0);
        d.after_slot(SLOT_MS, &[("CQ K1ABC FN42".to_string(), -10, 1200)], BASE + 14.3, 14_074_000, "20m");
        // il periodo di trasmissione e' il successivo: prima che cominci non c'e' niente da scrivere
        assert!(d.due_notes(SLOT_MS + 14_999).is_empty());
        let notes = d.due_notes(SLOT_MS + 15_100);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!((notes[0].start_ms, notes[0].mode, notes[0].text.as_str()), (SLOT_MS + 15_000, Mode::Ft8, "K1ABC IU3ABC IO91"));
        assert!(d.due_notes(SLOT_MS + 16_000).is_empty(), "una volta sola");
        // una trasmissione annunciata e fermata prima di partire non si scrive
        d.remember(8, "CQ K1ABC FN42", -10, 1200, SLOT_MS + 30_000);
        d.handle(TxCommand::Reply { decode_id: 8 }, BASE + 44.0);
        d.after_slot(SLOT_MS + 30_000, &[("CQ K1ABC FN42".to_string(), -10, 1200)], BASE + 44.3, 14_074_000, "20m");
        d.handle(TxCommand::Halt, BASE + 44.5);
        assert!(d.due_notes(SLOT_MS + 90_000).is_empty());
    }

    /// Il doppio clic arriva quasi sempre quando e' gia' cominciato il periodo dopo quello del CQ: la parita' va presa dal periodo del messaggio
    /// e non da quello del clic (altrimenti si trasmette sopra al DX). Qui il CQ e' del periodo pari N, il clic a N+1 + 0,5 s.
    #[test]
    fn a_click_in_the_following_slot_answers_in_the_opposite_parity_of_the_cq_and_starts_at_once() {
        let mut b = Bench::new();
        b.d.remember(7, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        b.d.after_slot(SLOT_MS, &[("CQ K1ABC FN42".to_string(), -10, 1200)], BASE + 14.3, 14_074_000, "20m");
        let log = b.d.handle(TxCommand::Reply { decode_id: 7 }, BASE + 15.5);
        // parte subito, nel periodo dispari in corso
        assert_eq!(b.requests(), vec![(tx_native::Mode::Ft8, Parity::Odd, b.requests()[0].2)], "{log:?}");
        // a fine periodo non si chiede una seconda volta
        b.d.after_slot(SLOT_MS + 15_000, &[], BASE + 29.3, 14_074_000, "20m");
        assert_eq!(b.requests().len(), 1, "il periodo dispari e' gia' coperto; il pari non e' il suo");
        // e il periodo dopo (dispari: 3) il messaggio si ripete
        b.d.after_slot(SLOT_MS + 30_000, &[], BASE + 44.3, 14_074_000, "20m");
        assert_eq!(b.requests().len(), 2);
    }

    /// La causa del difetto: Salta Tx1 arriva dalla pagina come comando separato; il primo messaggio trasmesso deve essere Tx2 (nominativo + rapporto).
    #[test]
    fn skip_tx1_set_from_the_page_makes_the_double_click_start_from_tx2() {
        for skip in [true, false] {
            let mut b = Bench::new();
            b.press("skip_tx1", Some(skip), BASE + 14.0);
            b.d.remember(7, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
            b.d.after_slot(SLOT_MS, &[("CQ K1ABC FN42".to_string(), -10, 1200)], BASE + 14.3, 14_074_000, "20m");
            let log = b.d.handle(TxCommand::Reply { decode_id: 7 }, BASE + 15.5);
            let sent = last_text(&b.port);
            let line = log.iter().find(|l| l.starts_with("trasmetto")).cloned().unwrap_or_default();
            if skip {
                assert!(line.contains("(Tx2)"), "{log:?}");
                assert_eq!(sent, vec!["K1ABC IU3ABC -10".to_string()], "con Salta Tx1 il primo messaggio e' Tx2");
                assert_eq!(view_of(&b.controls).selected, 2);
            } else {
                assert!(line.contains("(Tx1)"), "{log:?}");
                assert_eq!(sent, vec!["K1ABC IU3ABC IO91".to_string()], "senza Salta Tx1 il primo messaggio e' Tx1");
                assert_eq!(view_of(&b.controls).selected, 1);
            }
        }
    }

    #[test]
    fn skip_tx1_toggled_after_the_click_moves_the_pending_message() {
        let mut b = Bench::new();
        b.d.remember(7, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        b.d.after_slot(SLOT_MS, &[], BASE + 14.3, 14_074_000, "20m");
        // il clic arriva troppo tardi per partire subito (oltre 2 s): resta in attesa del periodo giusto con Tx1
        b.d.handle(TxCommand::Reply { decode_id: 7 }, BASE + 20.0);
        assert_eq!(view_of(&b.controls).selected, 1);
        b.press("skip_tx1", Some(true), BASE + 21.0);
        assert_eq!(view_of(&b.controls).selected, 2);
        b.d.after_slot(SLOT_MS + 15_000, &[], BASE + 29.3, 14_074_000, "20m");
        b.d.after_slot(SLOT_MS + 30_000, &[], BASE + 44.3, 14_074_000, "20m");
        assert_eq!(last_text(&b.port), vec!["K1ABC IU3ABC -10".to_string()]);
    }

    /// Cambio di banda con una risposta in corso: Halt, DX e messaggi azzerati, Abilita Tx spento, messaggi ricordati svuotati.
    #[test]
    fn a_band_change_halts_clears_the_dx_and_switches_tx_off() {
        let (mut d, port, controls) = driver();
        d.remember(7, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        d.after_slot(SLOT_MS, &[("CQ K1ABC FN42".to_string(), -10, 1200)], BASE + 14.3, 14_074_000, "20m");
        d.handle(TxCommand::Reply { decode_id: 7 }, BASE + 15.5);
        let before = view_of(&controls);
        assert!(before.enabled, "{before:?}");
        assert_eq!(before.dx.as_deref(), Some("K1ABC"));
        let halts_before = *port.halts.lock().expect("halt");
        let log = d.band_changed();
        assert!(log[0].contains("cambio di banda"), "{log:?}");
        assert_eq!(*port.halts.lock().expect("halt"), halts_before + 1, "il trasmettitore va fermato subito");
        let after = view_of(&controls);
        assert!(!after.enabled, "Abilita Tx deve restare spento");
        assert_eq!((after.dx, after.dx_grid), (None, None));
        assert_eq!(after.selected, 0, "nessun messaggio selezionato senza DX");
        // i messaggi ricordati non sono piu' rispondibili
        let log = d.handle(TxCommand::Reply { decode_id: 7 }, BASE + 16.0);
        assert!(log[0].contains("non piu' in memoria"), "{log:?}");
        // e dopo il cambio non parte niente nel periodo successivo
        let sent = port.requests.lock().expect("richieste").len();
        d.after_slot(SLOT_MS + 15_000, &[], BASE + 29.3, 14_074_000, "20m");
        assert_eq!(port.requests.lock().expect("richieste").len(), sent);
    }

    #[test]
    fn nothing_is_sent_without_a_dx_or_a_cq() {
        let (mut d, port, _) = driver();
        let (_, _) = d.after_slot(SLOT_MS, &[], BASE + 14.3, 14_074_000, "20m");
        assert!(port.requests.lock().expect("richieste").is_empty());
    }

    #[test]
    fn an_unknown_decode_id_says_so_and_stop_halts_everything() {
        let (mut d, port, _) = driver();
        let log = d.handle(TxCommand::Reply { decode_id: 99 }, 1.0);
        assert!(log[0].contains("non piu' in memoria"));
        d.handle(TxCommand::Halt, 1.0);
        assert_eq!(*port.halts.lock().expect("halt"), 1);
    }

    #[test]
    fn calling_cq_enables_transmission_and_publishes_the_six_messages() {
        let (mut d, port, controls) = driver();
        d.handle(TxCommand::CallCq { modifier: Some("DX".into()) }, BASE + 1.0);
        let view = view_of(&controls);
        assert!(view.enabled);
        assert!(view.texts[5].starts_with("CQ DX IU3ABC"), "{:?}", view.texts);
        d.after_slot(SLOT_MS, &[], BASE + 14.3, 14_074_000, "20m");
        d.after_slot(SLOT_MS + 15_000, &[], BASE + 29.3, 14_074_000, "20m");
        assert!(!port.requests.lock().expect("richieste").is_empty(), "il CQ parte in uno dei due periodi");
    }

    /// Pulsante CQ con la casella DX: il testo che arriva al trasmettitore e' «CQ DX nominativo locatore».
    #[test]
    fn the_cq_button_with_the_dx_modifier_reaches_the_transmitter_with_the_right_text() {
        for (modifier, expected) in [(Some("DX"), "CQ DX IU3ABC IO91"), (None, "CQ IU3ABC IO91"), (Some("NA"), "CQ NA IU3ABC IO91")] {
            let (mut d, port, _) = driver();
            // il periodo SLOT_MS e' pari e la sequenza parte dai pari: il CQ parte subito
            let log = d.handle(TxCommand::CallCq { modifier: modifier.map(str::to_string) }, BASE + 1.0);
            assert!(log.iter().any(|l| l.contains(expected)), "{log:?}");
            assert_eq!(last_text(&port), vec![expected.to_string()]);
        }
        // un modificatore non ammesso non fa trasmettere
        let (mut d, port, _) = driver();
        let log = d.handle(TxCommand::CallCq { modifier: Some("QUESTO E TROPPO".into()) }, BASE + 1.0);
        assert!(log.iter().any(|l| l.contains("CQ non possibile")), "{log:?}");
        assert!(port.requests.lock().expect("richieste").is_empty());
    }

    /// JTDX non toglie il messaggio libero dopo l'invio (`m_ntx` resta 8 in `guiUpdate`): si ripete a ogni periodo di trasmissione, nella parita'
    /// scelta, finche' Abilita Tx e' acceso e non si sceglie un altro messaggio.
    #[test]
    fn a_free_text_repeats_every_period_of_the_chosen_parity_until_halted() {
        let (mut d, port, _) = driver();
        d.handle(TxCommand::Option(OptionChange::TxEven(true)), 1.0);
        d.handle(TxCommand::FreeText { text: "TNX 73".into(), send: true }, 1.0);
        let count = |p: &Arc<FakePort>| p.requests.lock().expect("richieste").len();
        let first = count(&port);
        // dopo un periodo pari si pianifica il dispari (niente); dopo il dispari, il pari (si trasmette); e cosi' via
        d.after_slot(SLOT_MS, &[], BASE + 14.3, 14_074_000, "20m");
        assert_eq!(count(&port), first, "il periodo che comincia e' dispari");
        d.after_slot(SLOT_MS + 15_000, &[], BASE + 29.3, 14_074_000, "20m");
        let second = count(&port);
        assert_eq!(second, first + 1, "il pari successivo ripete il messaggio libero");
        d.after_slot(SLOT_MS + 30_000, &[], BASE + 44.3, 14_074_000, "20m");
        d.after_slot(SLOT_MS + 45_000, &[], BASE + 59.3, 14_074_000, "20m");
        assert_eq!(count(&port), second + 1, "e ancora al pari dopo");
        assert_eq!(last_text(&port), vec!["TNX 73".to_string()]);
        // «Ferma Tx» lo toglie
        d.handle(TxCommand::Halt, BASE + 60.0);
        let halted = count(&port);
        d.after_slot(SLOT_MS + 60_000, &[], BASE + 74.3, 14_074_000, "20m");
        d.after_slot(SLOT_MS + 75_000, &[], BASE + 89.3, 14_074_000, "20m");
        assert_eq!(count(&port), halted, "dopo Ferma Tx non si ripete");
    }

    /// `doubleClickOnCall`: il doppio clic accende «Abilita Tx» solo se «AutoTX» e' acceso; con AutoTX spento sceglie il DX e il messaggio ma non
    /// fa trasmettere. Abilita Tx e AutoTX non si influenzano (ne' lo spegne «Ferma Tx»).
    #[test]
    fn the_double_click_enables_tx_only_with_auto_tx_on() {
        // AutoTX acceso (di fabbrica): il doppio clic accende Abilita Tx e si trasmette
        let mut b = Bench::new();
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 16.0);
        assert!(view_of(&b.controls).enabled);
        assert!(!b.requests().is_empty());
        // AutoTX spento: scelto il DX, Abilita Tx resta spento e non parte niente
        let mut b = Bench::new();
        b.press("auto_tx", Some(false), 1.0);
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        let log = b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 16.0);
        let view = view_of(&b.controls);
        assert!(!view.enabled, "{log:?}");
        assert_eq!(view.dx.as_deref(), Some("K1ABC"), "il DX e' scelto lo stesso");
        assert!(b.requests().is_empty(), "senza Abilita Tx non si trasmette");
        // Abilita Tx a mano: parte, e AutoTX resta com'era
        b.press("enable_tx", Some(true), BASE + 17.0);
        assert!(view_of(&b.controls).enabled);
        assert!(!b.controls.switch_on("auto_tx"));
        assert!(!b.requests().is_empty(), "con Abilita Tx acceso la risposta parte");
        // Ferma Tx spegne Abilita Tx e lascia AutoTX
        b.press("auto_tx", Some(true), BASE + 18.0);
        b.d.handle(TxCommand::Halt, BASE + 18.0);
        assert!(!view_of(&b.controls).enabled);
        assert!(b.controls.switch_on("auto_tx"), "Ferma Tx non tocca AutoTX");
    }

    #[test]
    fn the_free_text_reaches_the_transmitter_with_the_right_text() {
        let mut b = Bench::new();
        // «Prepara»: solo il campo Tx5 della pagina, niente trasmissione
        b.d.handle(TxCommand::FreeText { text: "tnx   73  gl".into(), send: false }, BASE + 1.0);
        assert_eq!(b.texts()[4], "TNX 73 GL");
        assert!(b.requests().is_empty(), "preparare non trasmette");
        // «Invia» senza testo manda quello preparato (periodo pari, in corso: parte subito)
        let log = b.d.handle(TxCommand::FreeText { text: String::new(), send: true }, BASE + 1.0);
        assert!(log.iter().any(|l| l.contains("TNX 73 GL")), "{log:?}");
        assert_eq!(last_text(&b.port), vec!["TNX 73 GL".to_string()]);
        // «Invia» con testo nuovo
        let mut b = Bench::new();
        b.d.handle(TxCommand::FreeText { text: "CQ TEST".into(), send: true }, BASE + 1.0);
        assert_eq!(last_text(&b.port), vec!["CQ TEST".to_string()]);
        // senza niente da inviare lo dice
        let mut b = Bench::new();
        let log = b.d.handle(TxCommand::FreeText { text: String::new(), send: true }, BASE + 1.0);
        assert!(log.iter().any(|l| l.contains("nessun testo")), "{log:?}");
        // testo vuoto senza «invia» svuota il preparato
        b.d.handle(TxCommand::FreeText { text: "73".into(), send: false }, BASE + 1.0);
        b.d.handle(TxCommand::FreeText { text: String::new(), send: false }, BASE + 1.0);
        assert_eq!(b.texts()[4], "");
    }

    /// Come JTDX (`pack77`: `character*37 msg`, testo libero = primi 13 caratteri): oltre i 13 caratteri si trasmettono i primi 13,
    /// oltre i 37 il testo e' tagliato prima.
    #[test]
    fn a_free_text_longer_than_13_characters_goes_out_truncated_like_in_jtdx() {
        let mut b = Bench::new();
        let log = b.d.handle(TxCommand::FreeText { text: "GRAZIE PER IL QSO E BUONA SERATA".into(), send: true }, BASE + 1.0);
        assert!(log.iter().any(|l| l.contains("trasmetto")), "{log:?}");
        assert_eq!(last_text(&b.port), vec!["GRAZIE PER IL".to_string()]);
        // il campo non supera i 37 caratteri
        let long = "A".repeat(60);
        assert_eq!(clean_message(&long).chars().count(), MESSAGE_MAX);
        assert_eq!(clean_message("  ciao,   mondo! 73 "), "CIAO MONDO 73", "solo l'alfabeto dei messaggi, spazi singoli");
    }

    /// I macro della pagina («@ TNX 73», «TU ^ 73», «@ &», «@ #», «@ R#») arrivano espansi; ognuno si codifica e si rilegge uguale.
    #[test]
    fn the_page_macros_reach_the_transmitter_and_read_back_the_same() {
        let cases = [("K1ABC TNX 73", "K1ABC TNX 73"), ("TU MARIO 73", "TU MARIO 73"), ("K1ABC IU3ABC", "K1ABC IU3ABC"), ("K1ABC -10", "K1ABC -10"), ("K1ABC R-10", "K1ABC R-10")];
        for (text, expected) in cases {
            let mut b = Bench::new();
            b.d.handle(TxCommand::FreeText { text: text.into(), send: true }, BASE + 1.0);
            assert_eq!(last_text(&b.port), vec![expected.to_string()], "macro {text}");
        }
        // la coda che l'interfaccia taglia a 13 caratteri passa anche cosi'
        let mut b = Bench::new();
        b.d.handle(TxCommand::FreeText { text: "TU DAVIDE 73".into(), send: true }, BASE + 1.0);
        assert_eq!(last_text(&b.port), vec!["TU DAVIDE 73".to_string()]);
    }

    #[test]
    fn set_tx_freq_changes_the_transmitted_audio_frequency_and_follows_the_split_switch() {
        let mut b = Bench::new();
        // di fabbrica Split e' acceso (`LockTxFreq` falso); spento, Tx e Rx vanno insieme
        b.controls.set_switch("split", false);
        b.d.handle(TxCommand::SetTxFreq(1800), BASE + 1.0);
        assert_eq!((b.controls.tx_df.load(Ordering::Relaxed), b.controls.rx_df.load(Ordering::Relaxed)), (1800, 1800));
        b.d.handle(TxCommand::FreeText { text: "73".into(), send: true }, BASE + 1.0);
        let freq = {
            let requests = b.port.requests.lock().expect("richieste");
            let req = requests.last().expect("richiesta");
            // la frequenza dell'onda si legge decodificando: il messaggio esce a 1800 Hz
            let mut audio = noise(180_000, 5);
            for (i, w) in req.wave.iter().enumerate() {
                audio[6_000 + i] += 0.05 * w;
            }
            ft8::decode::decode_slot(&audio, &ft8::decode::Options::default(), &mut CallHashes::default()).into_iter().next().map(|d| d.freq_hz)
        };
        assert!(freq.is_some_and(|f| (f - 1800.0).abs() < 2.0), "{freq:?}");
        // Split acceso: Tx e Rx indipendenti
        b.press("split", Some(true), BASE + 2.0);
        b.d.handle(TxCommand::SetTxFreq(2200), BASE + 2.0);
        assert_eq!((b.controls.tx_df.load(Ordering::Relaxed), b.controls.rx_df.load(Ordering::Relaxed)), (2200, 1800));
        b.rig.set_rx_frequency(900).expect("rx");
        assert_eq!((b.controls.tx_df.load(Ordering::Relaxed), b.controls.rx_df.load(Ordering::Relaxed)), (2200, 900));
        // Rx -> Tx, Tx -> Rx
        b.press("rx_to_tx", None, BASE + 3.0);
        assert_eq!(b.controls.tx_df.load(Ordering::Relaxed), 900);
        b.d.handle(TxCommand::SetTxFreq(2500), BASE + 3.0);
        b.press("tx_to_rx", None, BASE + 3.0);
        assert_eq!(b.controls.rx_df.load(Ordering::Relaxed), 2500);
        // fuori campo: rifiutata
        let log = b.d.handle(TxCommand::SetTxFreq(20), BASE + 3.0);
        assert!(log.iter().any(|l| l.contains("fuori campo")), "{log:?}");
        assert_eq!(b.controls.tx_df.load(Ordering::Relaxed), 2500);
    }

    #[test]
    fn turning_split_off_puts_tx_on_the_rx_frequency_and_rx_changes_drag_tx() {
        let mut b = Bench::new();
        b.press("split", Some(true), 1.0);
        b.rig.set_rx_frequency(1000).expect("rx");
        b.d.handle(TxCommand::SetTxFreq(2000), 1.0);
        assert_eq!((b.controls.tx_df.load(Ordering::Relaxed), b.controls.rx_df.load(Ordering::Relaxed)), (2000, 1000));
        b.press("split", Some(false), 1.0);
        assert_eq!(b.controls.tx_df.load(Ordering::Relaxed), 1000, "come `on_pbTxLock_clicked`: Tx = Rx");
        b.rig.set_rx_frequency(1300).expect("rx");
        assert_eq!(b.controls.tx_df.load(Ordering::Relaxed), 1300);
    }

    #[test]
    fn a_double_click_moves_rx_and_with_split_off_tx_to_the_frequency_of_the_message() {
        let mut b = Bench::new();
        b.controls.set_switch("split", false);
        b.d.remember(1, "CQ K1ABC FN42", -10, 1234, SLOT_MS);
        b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 16.0);
        assert_eq!((b.controls.rx_df.load(Ordering::Relaxed), b.controls.tx_df.load(Ordering::Relaxed)), (1234, 1234));
        // con Split acceso la trasmissione resta dov'era
        let mut b = Bench::new();
        b.press("split", Some(true), 1.0);
        b.d.remember(1, "CQ K1ABC FN42", -10, 1234, SLOT_MS);
        b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 16.0);
        assert_eq!((b.controls.rx_df.load(Ordering::Relaxed), b.controls.tx_df.load(Ordering::Relaxed)), (1234, 1500));
    }

    #[test]
    fn gen_msgs_regenerates_the_messages_for_the_current_dx() {
        let mut b = Bench::new();
        // senza DX non c'e' niente da rigenerare e non da' errori
        b.press("gen_msgs", None, BASE);
        assert_eq!(view_of(&b.controls).selected, 0);
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 20.0);
        b.d.handle(TxCommand::SelectTx(4), BASE + 20.0);
        assert_eq!(view_of(&b.controls).selected, 4);
        let log = b.press("gen_msgs", None, BASE + 21.0);
        assert!(log.iter().any(|l| l.contains("rigenerati")), "{log:?}");
        assert_eq!(view_of(&b.controls).selected, 1, "riparte da Tx1");
        assert_eq!(b.texts()[0], "K1ABC IU3ABC IO91");
        b.press("skip_tx1", Some(true), BASE + 22.0);
        b.press("gen_msgs", None, BASE + 22.0);
        assert_eq!(view_of(&b.controls).selected, 2, "con Salta Tx1 riparte da Tx2");
        // un messaggio libero preparato lascia il posto a quello standard
        b.d.handle(TxCommand::FreeText { text: "PIPPO".into(), send: false }, BASE + 23.0);
        assert_eq!(b.texts()[4], "PIPPO");
        b.press("gen_msgs", None, BASE + 23.0);
        assert_eq!(b.texts()[4], "K1ABC IU3ABC 73");
    }

    #[test]
    fn erase_forgets_the_remembered_messages_and_decode_without_a_period_is_accepted() {
        let mut b = Bench::new();
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        b.press("erase", None, BASE);
        let log = b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 16.0);
        assert!(log[0].contains("non piu' in memoria"), "{log:?}");
        b.press("decode", None, BASE);
        assert!(b.controls.redecode.load(Ordering::Relaxed), "la richiesta aspetta il motore");
    }

    #[test]
    fn tx_period_button_changes_the_slots_in_which_we_transmit_and_shows_the_double_click_choice() {
        for even in [true, false] {
            let mut b = Bench::new();
            b.press("tx_period", Some(even), BASE);
            b.press("autoseq", Some(true), BASE);
            b.d.handle(TxCommand::CallCq { modifier: None }, BASE + 14.9);
            // il CQ non e' partito subito (periodo SLOT_MS pari, ma troppo tardi o dispari)
            b.port.requests.lock().expect("richieste").clear();
            b.d.after_slot(SLOT_MS, &[], BASE + 14.3, 14_074_000, "20m");
            let first = b.requests();
            b.d.after_slot(SLOT_MS + 15_000, &[], BASE + 29.3, 14_074_000, "20m");
            let second = b.requests();
            // dopo il periodo pari (SLOT_MS) comincia un dispari: si trasmette solo lì se il pulsante e' sui dispari
            if even {
                assert!(first.is_empty() && second.len() == 1 && second[0].1 == Parity::Even, "{first:?} {second:?}");
            } else {
                assert!(first.len() == 1 && first[0].1 == Parity::Odd && second.len() == 1, "{first:?} {second:?}");
            }
        }
        // il doppio clic sceglie il periodo opposto a quello del messaggio e il pulsante lo mostra
        let mut b = Bench::new();
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS + 15_000);
        b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 40.0);
        assert!(b.controls.switch_on("tx_period"), "CQ in un periodo dispari: si trasmette nei pari");
        let label = b.rig.controls_and_tx().expect("lettura").1.expect("tx").minute_label;
        assert_eq!(label, "TX 00/30");
    }

    /// Cambiare periodo con Abilita Tx e AutoSeq accesi libera il DX (`on_TxMinuteButton_clicked`).
    #[test]
    fn changing_the_tx_period_with_autoseq_on_clears_the_dx() {
        let mut b = Bench::new();
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS + 15_000);
        b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 40.0);
        assert!(view_of(&b.controls).dx.is_some());
        b.press("tx_period", Some(false), BASE + 41.0);
        assert!(view_of(&b.controls).dx.is_none());
        assert!(!b.controls.switch_on("tx_period"));
    }

    #[test]
    fn autoseq_toggle_reaches_the_sequencer() {
        for auto in [true, false] {
            let mut b = Bench::new();
            b.press("autoseq", Some(auto), BASE);
            b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
            b.d.after_slot(SLOT_MS, &[], BASE + 14.3, 14_074_000, "20m");
            b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 15.5);
            // il DX risponde col suo rapporto: con AutoSeq si passa a Tx3, senza il messaggio resta
            b.d.after_slot(SLOT_MS + 15_000, &[("IU3ABC K1ABC -07".to_string(), -9, 1200)], BASE + 29.3, 14_074_000, "20m");
            let selected = view_of(&b.controls).selected;
            assert_eq!(selected, if auto { 3 } else { 1 }, "autoseq {auto}");
        }
    }

    #[test]
    fn rrr_toggle_changes_tx4() {
        let mut b = Bench::new();
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
        b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 20.0);
        assert_eq!(b.texts()[3], "K1ABC IU3ABC RR73");
        b.press("rrr", Some(true), BASE + 21.0);
        assert_eq!(b.texts()[3], "K1ABC IU3ABC RRR");
        b.press("rrr", Some(false), BASE + 22.0);
        assert_eq!(b.texts()[3], "K1ABC IU3ABC RR73");
    }

    #[test]
    fn bypass_toggle_lets_the_first_transmission_start_late() {
        for bypass in [true, false] {
            let mut b = Bench::new();
            b.press("bypass", Some(bypass), BASE);
            b.d.remember(1, "CQ K1ABC FN42", -10, 1200, SLOT_MS);
            b.d.after_slot(SLOT_MS, &[], BASE + 14.3, 14_074_000, "20m");
            // clic 8 s dopo l'inizio del periodo dispari: oltre i 2 s normali, dentro i 3/4 del periodo con «Bypassa»
            b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 15.0 + 8.0);
            assert_eq!(b.requests().len(), usize::from(bypass), "bypass {bypass}");
        }
    }

    /// Un QSO intero in modalita' CQ: «1 QSO» ferma la trasmissione a QSO fatto, senza torna al CQ; «Risp B4» decide se si risponde a chi e' gia' stato lavorato.
    fn run_cq_qso(b: &mut Bench) -> Vec<aurora_core::QsoRecord> {
        let mut all = Vec::new();
        b.d.handle(TxCommand::CallCq { modifier: None }, BASE + 14.5);
        // periodo 0 (pari): decodifica vuota; K1ABC ci chiama nel periodo 1 (dispari)
        let steps: [(u64, &[(&str, i32)]); 4] = [
            (SLOT_MS + 15_000, &[("IU3ABC K1ABC FN42", -10)]),
            (SLOT_MS + 45_000, &[("IU3ABC K1ABC -07", -10)]),
            (SLOT_MS + 75_000, &[("IU3ABC K1ABC RR73", -10)]),
            (SLOT_MS + 105_000, &[]),
        ];
        for (slot, msgs) in steps {
            let list: Vec<(String, i32, u32)> = msgs.iter().map(|(t, s)| (t.to_string(), *s, 1200)).collect();
            let (q, _) = b.d.after_slot(slot, &list, slot as f64 / 1000.0 + 14.3, 14_074_000, "20m");
            all.extend(q);
        }
        all
    }

    #[test]
    fn single_qso_toggle_stops_after_the_qso_and_without_it_the_cq_goes_on() {
        for single in [true, false] {
            let mut b = Bench::new();
            b.press("single_qso", Some(single), BASE);
            let qsos = run_cq_qso(&mut b);
            assert_eq!(qsos.len(), 1, "single {single}");
            assert_eq!(qsos[0].call, "K1ABC");
            assert_eq!(qsos[0].mode, "FT8");
            assert_eq!(view_of(&b.controls).enabled, !single, "single {single}");
        }
    }

    #[test]
    fn answer_b4_toggle_decides_whether_a_worked_station_is_answered() {
        for answer in [true, false] {
            let mut b = Bench::new();
            b.press("answer_b4", Some(answer), BASE);
            let qsos = run_cq_qso(&mut b);
            assert_eq!(qsos.len(), 1);
            // K1ABC richiama a QSO fatto
            b.d.after_slot(SLOT_MS + 135_000, &[("IU3ABC K1ABC FN42".to_string(), -10, 1200)], BASE + 149.3, 14_074_000, "20m");
            let dx = view_of(&b.controls).dx;
            assert_eq!(dx.is_some(), answer, "risp B4 {answer}: {dx:?}");
        }
    }

    /// FT4: il motore trasmette con il modulatore FT4 nel periodo giusto, e l'onda si rilegge con il decoder FT4.
    #[test]
    fn ft4_reply_is_modulated_with_ft4_and_planned_in_the_right_7_5_second_slot() {
        let mut b = Bench::new();
        b.rig.set_mode("FT4").expect("modo");
        // il motore (ciclo di lettura) se ne accorge e porta il pilota in FT4
        assert_eq!(b.controls.mode(), Mode::Ft4);
        b.d.set_mode(Mode::Ft4);
        assert_eq!(b.d.mode(), Mode::Ft4);
        // CQ ricevuto nel periodo FT4 pari (indice 13 333 334): risposta nei dispari, a 7,5 s
        b.d.remember(1, "CQ K1ABC FN42", -10, 1200, FT4_SLOT_MS);
        b.d.after_slot(FT4_SLOT_MS, &[("CQ K1ABC FN42".to_string(), -10, 1200)], FT4_BASE + 6.9, 14_080_000, "20m");
        b.d.handle(TxCommand::Reply { decode_id: 1 }, FT4_BASE + 7.5 + 0.4);
        let requests = b.requests();
        assert_eq!(requests.len(), 1, "{requests:?}");
        assert_eq!((requests[0].0, requests[0].1), (tx_native::Mode::Ft4, Parity::Odd));
        assert_eq!(requests[0].2, ft4::modem::WAVE_SAMPLES);
        assert_eq!(last_text(&b.port), vec!["K1ABC IU3ABC IO91".to_string()], "il testo si rilegge con il decoder FT4");
        // a fine del periodo dispari (7,5 s dopo) si pianifica il successivo dispari? no: il pari non e' suo, poi il dispari si'
        b.d.after_slot(FT4_SLOT_MS + 7_500, &[], FT4_BASE + 7.5 + 6.9, 14_080_000, "20m");
        assert_eq!(b.requests().len(), 1);
        b.d.after_slot(FT4_SLOT_MS + 15_000, &[], FT4_BASE + 15.0 + 6.9, 14_080_000, "20m");
        let requests = b.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].1, Parity::Odd);
        // tornando a FT8 il pilota riparte pulito
        b.rig.set_mode("FT8").expect("modo");
        let log = b.d.set_mode(Mode::Ft8);
        assert!(log.iter().any(|l| l.contains("FT8")), "{log:?}");
        assert_eq!(*b.port.halts.lock().expect("halt"), 2, "ogni cambio di modo ferma la trasmissione");
        let log = b.d.handle(TxCommand::Reply { decode_id: 1 }, BASE + 16.0);
        assert!(log[0].contains("non piu' in memoria"), "i messaggi ricordati si svuotano col modo: {log:?}");
        assert!(view_of(&b.controls).dx.is_none());
    }

    #[test]
    fn mode_change_stops_a_running_transmission_and_clears_the_qso() {
        let mut b = Bench::new();
        b.d.handle(TxCommand::CallCq { modifier: None }, BASE + 1.0);
        assert!(view_of(&b.controls).enabled);
        b.d.set_mode(Mode::Ft4);
        assert!(!view_of(&b.controls).enabled);
        assert_eq!(*b.port.halts.lock().expect("halt"), 1);
    }

    /// Con il trasmettitore vero (e un orologio manuale) una risposta FT4 parte all'inizio del periodo da 7,5 s e non a quello da 15 s.
    #[test]
    fn ft4_transmission_starts_on_the_seven_and_a_half_second_boundary_with_the_real_transmitter() {
        use std::time::Duration;
        use tx_native::{
            ManualClock, NoPtt, Transmitter, TxConfig,
            testing::{Recorder, RecordingSink},
        };
        struct Real(Transmitter, StdMutex<Vec<Duration>>);
        impl TxPort for Real {
            fn schedule(&self, request: TxRequest) -> Result<(), String> {
                let plan = self.0.schedule(request).map_err(|e| e.to_string())?;
                self.1.lock().map_err(|_| "avvelenato".to_string())?.push(plan.slot_start);
                Ok(())
            }
            fn halt(&self) {
                self.0.halt();
            }
        }
        let start = Duration::from_millis(FT4_SLOT_MS) + Duration::from_millis(6_900);
        let clock = Arc::new(ManualClock::new(start));
        let recorder = Recorder::new(clock.clone());
        let (sink, _state) = RecordingSink::new(Arc::clone(&recorder), clock.clone(), 12_000);
        let config = TxConfig { allow_tx: true, attenuation_tenths: Some(0), ..TxConfig::default() };
        let transmitter = Transmitter::new(clock.clone(), Arc::new(NoPtt::new()), Box::new(sink), config).expect("trasmettitore");
        let port = Arc::new(Real(transmitter, StdMutex::new(Vec::new())));
        let controls = Arc::new(NativeControls::default());
        controls.set_mode(Mode::Ft4);
        let mut d = TxDriver::new("IU3ABC", "IO91", Arc::clone(&port), Arc::clone(&controls)).expect("pilota");
        d.remember(1, "CQ K1ABC FN42", -10, 1200, FT4_SLOT_MS);
        d.handle(TxCommand::Reply { decode_id: 1 }, FT4_BASE + 6.9);
        let (_, log) = d.after_slot(FT4_SLOT_MS, &[], FT4_BASE + 6.9, 14_080_000, "20m");
        let starts = port.1.lock().expect("inizi").clone();
        assert_eq!(starts, vec![Duration::from_millis(FT4_SLOT_MS + 7_500)], "{log:?}");
        port.halt();
    }

    #[test]
    fn a_qso_in_ft4_is_logged_as_ft4() {
        let b = Bench::new();
        b.controls.set_mode(Mode::Ft4);
        let port = Arc::new(FakePort::default());
        let mut d = TxDriver::new("IU3ABC", "IO91", Arc::clone(&port), Arc::clone(&b.controls)).expect("pilota");
        d.handle(TxCommand::CallCq { modifier: None }, FT4_BASE + 1.0);
        let mut qsos = Vec::new();
        for (i, msg) in [(1u64, "IU3ABC K1ABC FN42"), (3, "IU3ABC K1ABC -07"), (5, "IU3ABC K1ABC RR73")] {
            let slot = FT4_SLOT_MS + i * 7_500;
            let (q, _) = d.after_slot(slot, &[(msg.to_string(), -10, 1200)], slot as f64 / 1000.0 + 6.9, 14_080_000, "20m");
            qsos.extend(q);
        }
        assert_eq!(qsos.len(), 1);
        assert_eq!(qsos[0].mode, "FT4");
    }

    #[test]
    fn the_status_modes_and_labels_follow_the_mode_button() {
        let b = Bench::new();
        assert_eq!(b.rig.controls_and_tx().expect("lettura").1.expect("tx").minute_label, "TX 00/30");
        b.rig.set_mode("FT4").expect("modo");
        assert_eq!(b.controls.mode(), Mode::Ft4);
        assert_eq!(b.rig.controls_and_tx().expect("lettura").1.expect("tx").minute_label, "TX 00");
        b.controls.set_switch("tx_period", false);
        assert_eq!(b.rig.controls_and_tx().expect("lettura").1.expect("tx").minute_label, "TX 7.5");
        // JT65 e JT9 si accettano e non cambiano il modo
        b.rig.set_mode("JT65").expect("accettato");
        b.rig.set_mode("JT9").expect("accettato");
        assert_eq!(b.controls.mode(), Mode::Ft4);
        b.rig.set_mode("FT8").expect("modo");
        assert_eq!(b.controls.mode(), Mode::Ft8);
        assert!(b.rig.set_mode("SSB").is_err());
    }
}
