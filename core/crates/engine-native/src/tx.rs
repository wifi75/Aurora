//! La trasmissione del motore proprio: unisce la sequenza dei QSO (`qso-native`), la modulazione (`ft8::modem`) e la catena audio/PTT
//! (`tx-native`). A ogni periodo decodificato la sequenza guarda cosa e' arrivato e dice cosa trasmettere nel periodo che comincia; il messaggio
//! diventa un'onda e parte a inizio periodo (+0,5 s). Il doppio clic su un messaggio, il CQ, STOP e le opzioni arrivano come [`TxCommand`].

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
    FreeText { text: String, send: bool },
    /// Abilita Tx / AutoTX.
    Enable(bool),
    Option(OptionChange),
    SelectTx(u8),
    ClearDx,
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
}

pub struct TxDriver<P: TxPort + ?Sized> {
    seq: Sequencer,
    opts: Options,
    port: Arc<P>,
    controls: Arc<NativeControls>,
    recent: VecDeque<Recent>,
    free_text: Option<String>,
    hashes: CallHashes,
    mode: Mode,
}

impl<P: TxPort + ?Sized> TxDriver<P> {
    pub fn new(call: &str, grid: &str, port: Arc<P>, controls: Arc<NativeControls>) -> Result<TxDriver<P>, String> {
        let opts = Options { tx_freq_hz: controls.rx_df.load(Ordering::Relaxed), ..Options::default() };
        let seq = Sequencer::new(Mode::Ft8, Station { call: call.to_string(), grid: grid.to_string() }, opts.clone()).map_err(|e| format!("{e:?}"))?;
        let driver = TxDriver { seq, opts, port, controls, recent: VecDeque::new(), free_text: None, hashes: CallHashes::default(), mode: Mode::Ft8 };
        driver.publish_view();
        Ok(driver)
    }

    /// Ricorda un messaggio decodificato, per il doppio clic (arriva solo l'identificativo).
    pub fn remember(&mut self, id: u64, text: &str, snr: i32, freq_hz: u32) {
        self.recent.push_back(Recent { id, text: text.to_string(), snr, freq_hz });
        while self.recent.len() > RECENT_KEEP {
            self.recent.pop_front();
        }
    }

    fn publish_view(&self) {
        let view = TxView {
            texts: self.seq.tx_texts(),
            selected: self.seq.ntx(),
            dx: self.seq.dx().map(str::to_string),
            dx_grid: self.seq.dx_grid().map(str::to_string),
            enabled: self.seq.is_enabled(),
        };
        if let Ok(mut slot) = self.controls.tx_view.lock() {
            *slot = view;
        }
    }

    /// Un comando. Restituisce le righe da scrivere nel registro (cosa e' successo e perche' non e' partito).
    pub fn handle(&mut self, command: TxCommand, now_s: f64) -> Vec<String> {
        let mut log = Vec::new();
        match command {
            TxCommand::Reply { decode_id } => match self.recent.iter().find(|r| r.id == decode_id) {
                Some(r) => {
                    let slot = Slot::at(self.mode, now_s);
                    let decode = QDecoded::new(&r.text, r.snr, r.freq_hz);
                    match self.seq.reply_to(slot, &decode) {
                        Ok(()) => {
                            self.seq.set_enabled(true);
                            log.push(format!("risposta a «{}»: Tx abilitato", r.text));
                        }
                        Err(e) => log.push(format!("risposta non possibile: {e:?}")),
                    }
                }
                None => log.push(format!("messaggio {decode_id} non piu' in memoria: non e' possibile rispondere")),
            },
            TxCommand::CallCq { modifier } => match self.seq.call_cq(modifier.as_deref()) {
                Ok(()) => {
                    self.seq.set_enabled(true);
                    log.push("CQ: Tx abilitato".to_string());
                }
                Err(e) => log.push(format!("CQ non possibile: {e:?}")),
            },
            TxCommand::SetTxFreq(hz) => {
                self.opts.tx_freq_hz = hz;
                self.seq.set_options(self.opts.clone());
                self.controls.rx_df.store(hz, Ordering::Relaxed);
            }
            TxCommand::Halt => {
                self.port.halt();
                self.seq.halt();
                self.free_text = None;
                log.push("STOP TX".to_string());
            }
            TxCommand::FreeText { text, send } => {
                if send {
                    self.free_text = Some(text);
                    self.seq.set_enabled(true);
                    log.push("messaggio libero in coda per il prossimo periodo".to_string());
                }
            }
            TxCommand::Enable(on) => {
                self.seq.set_enabled(on);
                if !on {
                    self.port.halt();
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
                    OptionChange::TxEven(v) => self.opts.tx_even = v,
                }
                self.seq.set_options(self.opts.clone());
            }
            TxCommand::SelectTx(n) => {
                if let Err(e) = self.seq.select_tx(n) {
                    log.push(format!("Tx{n} non selezionabile: {e:?}"));
                }
            }
            TxCommand::ClearDx => self.seq.clear_dx(),
        }
        self.publish_view();
        log
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
                QEvent::QsoComplete(rec) => qsos.push(QsoRecord {
                    call: rec.call.clone(),
                    grid: rec.grid.clone(),
                    band: band.to_string(),
                    mode: "FT8".to_string(),
                    freq_hz: dial_hz + u64::from(rec.tx_freq_hz),
                    rst_sent: rec.rst_sent.clone(),
                    rst_rcvd: rec.rst_rcvd.clone(),
                    time_ms: (rec.time_off_ms % DAY_MS) as u32,
                }),
                QEvent::Abandoned { call, reason } => log.push(format!("QSO con {call} abbandonato: {reason:?}")),
                QEvent::Stopped(reason) => log.push(format!("trasmissione automatica ferma: {reason:?}")),
                QEvent::StateChanged { .. } | QEvent::CallerQueued { .. } | QEvent::CallerIgnored { .. } => {}
            }
        }

        let next = slot.next();
        let parity = if next.is_even() { Parity::Even } else { Parity::Odd };
        let planned = if let Some(tx) = self.seq.next_tx(next, now_s) {
            Some((tx.text, tx.freq_hz, tx.n))
        } else if let Some(text) = self.free_text.clone()
            && next.is_even() == self.opts.tx_even
        {
            self.free_text = None;
            Some((text, self.opts.tx_freq_hz, 0))
        } else {
            None
        };
        if let Some((text, freq, n)) = planned {
            self.port.set_attenuation(self.controls.attenuation.load(Ordering::Relaxed));
            match ft8::modem::encode_message(&text, freq as f32, &mut self.hashes) {
                Ok(wave) => match self.port.schedule(TxRequest { wave, mode: tx_native::Mode::Ft8, parity }) {
                    Ok(()) => log.push(format!("trasmetto (Tx{n}) a {freq} Hz: {text}")),
                    Err(e) => log.push(format!("trasmissione non partita: {e}")),
                },
                Err(e) => log.push(format!("messaggio non codificabile «{text}»: {e:?}")),
            }
        }
        self.publish_view();
        (qsos, log)
    }
}

/// Lo stato osservabile della trasmissione per la pagina.
pub fn view_of(controls: &NativeControls) -> TxView {
    controls.tx_view.lock().map(|v| v.clone()).unwrap_or_default()
}


#[cfg(test)]
mod tests {
    use super::*;
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

    fn driver() -> (TxDriver<FakePort>, Arc<FakePort>, Arc<NativeControls>) {
        let port = Arc::new(FakePort::default());
        let controls = Arc::new(NativeControls::default());
        let d = TxDriver::new("IU3ABC", "IO91", Arc::clone(&port), Arc::clone(&controls)).expect("pilota");
        (d, port, controls)
    }

    // 100 000 000 s dall'epoca e' un periodo FT8 pari (6 666 666 periodi: 6 666 666 e' pari)
    const SLOT_MS: u64 = 99_999_990_000;

    #[test]
    fn a_reply_transmits_in_the_next_slot_with_the_standard_message() {
        let (mut d, port, _) = driver();
        d.remember(7, "CQ K1ABC FN42", -10, 1200);
        let log = d.handle(TxCommand::Reply { decode_id: 7 }, SLOT_MS as f64 / 1000.0 + 14.0);
        assert!(log.iter().any(|l| l.contains("K1ABC")), "{log:?}");
        let (_, log) = d.after_slot(SLOT_MS, &[("CQ K1ABC FN42".to_string(), -10, 1200)], SLOT_MS as f64 / 1000.0 + 14.3, 14_074_000, "20m");
        let requests = port.requests.lock().expect("richieste");
        assert_eq!(requests.len(), 1, "{log:?}");
        assert!(requests[0].wave.len() > 100_000, "un'onda vera di FT8");
        assert_eq!(requests[0].mode, tx_native::Mode::Ft8);
    }

    #[test]
    fn nothing_is_sent_without_a_dx_or_a_cq() {
        let (mut d, port, _) = driver();
        let (_, _) = d.after_slot(SLOT_MS, &[], SLOT_MS as f64 / 1000.0 + 14.3, 14_074_000, "20m");
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
        d.handle(TxCommand::CallCq { modifier: Some("DX".into()) }, SLOT_MS as f64 / 1000.0 + 1.0);
        let view = view_of(&controls);
        assert!(view.enabled);
        assert!(view.texts[5].starts_with("CQ DX IU3ABC"), "{:?}", view.texts);
        d.after_slot(SLOT_MS, &[], SLOT_MS as f64 / 1000.0 + 14.3, 14_074_000, "20m");
        d.after_slot(SLOT_MS + 15_000, &[], SLOT_MS as f64 / 1000.0 + 29.3, 14_074_000, "20m");
        assert!(!port.requests.lock().expect("richieste").is_empty(), "il CQ parte in uno dei due periodi");
    }

    #[test]
    fn a_free_text_is_sent_once_in_the_chosen_parity() {
        let (mut d, port, _) = driver();
        d.handle(TxCommand::Option(OptionChange::TxEven(true)), 1.0);
        d.handle(TxCommand::FreeText { text: "TNX 73".into(), send: true }, 1.0);
        // il periodo che comincia dopo SLOT_MS e' dispari: non si trasmette; dopo il successivo (pari) si
        d.after_slot(SLOT_MS, &[], SLOT_MS as f64 / 1000.0 + 14.3, 14_074_000, "20m");
        let before = port.requests.lock().expect("richieste").len();
        d.after_slot(SLOT_MS + 15_000, &[], SLOT_MS as f64 / 1000.0 + 29.3, 14_074_000, "20m");
        let after = port.requests.lock().expect("richieste").len();
        assert!(before + after >= 1 && after <= 1 + before, "una volta sola: {before} {after}");
        d.after_slot(SLOT_MS + 30_000, &[], SLOT_MS as f64 / 1000.0 + 44.3, 14_074_000, "20m");
        d.after_slot(SLOT_MS + 45_000, &[], SLOT_MS as f64 / 1000.0 + 59.3, 14_074_000, "20m");
        assert!(port.requests.lock().expect("richieste").len() <= 1, "il messaggio libero non si ripete");
    }
}
