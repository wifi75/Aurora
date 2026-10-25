//! I pulsanti della pagina (Monitora, Hound, Modo SWL, Sinc, AGCc, Filtro, Ferma, Decodifica, Cancella...) quando il motore e' quello proprio:
//! non c'e' nessun altro programma da comandare, quindi lo stato sta qui e agisce sul decoder. La frequenza e il cambio banda passano dalla
//! radio (OmniRig o Hamlib) se c'e'. Monitora **parte sempre acceso** all'avvio.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc, Arc, Mutex,
    },
};

use qso_native::Mode;

use crate::tx::{OptionChange, TxCommand, TxView};

use rig_control::{
    controls::{self, Kind, Reading},
    Rig, RigError,
};

/// Livello di uscita di fabbrica (cursore Pwr): `OutAttenuation` di JTDX vale 225 su 0..450 e l'attenuazione e' `(450 - valore) / 10` dB;
/// in decimi di dB i due numeri coincidono (verificato in `jtdx.exe`, `MainWindow::readSettings` e `on_outAttenuation_valueChanged`).
pub const DEFAULT_ATTENUATION_TENTHS: u32 = 225;

/// Il livello di uscita ricordato per banda (`m_pwrBandTxMemory` di JTDX, chiave «banda + modo»).
#[derive(Default)]
struct BandLevels {
    /// L'ultima banda vista.
    band: Option<String>,
    levels: HashMap<String, u32>,
}

const NO_TX: &str = "la trasmissione non e' disponibile: manca l'ingresso audio o il motore non e' partito";

/// Lo stato dei pulsanti: il decoder lo legge a ogni periodo.
pub struct NativeControls {
    /// Monitora: con ascolto spento non si decodifica (e lo stato dice che non si sta decodificando). Di base acceso.
    pub monitor: AtomicBool,
    /// Frequenza audio di ricezione, in Hz.
    pub rx_df: AtomicU32,
    /// Frequenza audio di trasmissione, in Hz. Con «Tx/Rx Split» spento segue la ricezione (e viceversa); con Split acceso sono indipendenti.
    pub tx_df: AtomicU32,
    /// Il modo corrente e' FT4 (altrimenti FT8). Lo legge il motore a ogni blocco di audio e cambia periodi e decoder.
    ft4: AtomicBool,
    /// «Decodifica»: chiesto di rielaborare l'ultimo periodo (lo raccoglie il motore).
    pub redecode: AtomicBool,
    /// Attenuazione dell'uscita audio (cursore Pwr), in decimi di dB: la usera' la trasmissione.
    pub attenuation: AtomicU32,
    /// Il livello di uscita di ogni banda e modo: al cambio di banda si ripristina quello della nuova banda.
    levels: Mutex<BandLevels>,
    /// Gli altri interruttori, per nome.
    switches: Mutex<Vec<(String, bool)>>,
    /// Cio' che la pagina mostra della trasmissione (i sei messaggi, il DX, se la sequenza e' accesa): lo scrive il motore.
    pub tx_view: Mutex<TxView>,
    /// Stiamo trasmettendo (lo scrive la catena di trasmissione).
    pub transmitting: AtomicBool,
    sender: Mutex<Option<mpsc::Sender<TxCommand>>>,
    /// Gli strumenti della radio (S-meter, potenza, ROS...) letti dal CAT; `None` se la radio non li da' (nessuna radio, OmniRig).
    meters: Mutex<Option<aurora_core::RadioMeters>>,
    /// L'S-meter stimato dall'audio in ingresso e quando e' stato calcolato (vedi `smeter`).
    s_estimate: Mutex<Option<(crate::smeter::SEstimate, std::time::Instant)>>,
    /// STOP TX immediato (vedi `set_halt_now`).
    halt_now: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    /// Cosa fare quando il livello di uscita cambia (portarlo subito al dispositivo, anche a trasmissione in corso).
    attenuation_hook: Mutex<Option<Arc<dyn Fn(u32) + Send + Sync>>>,
}

/// Oltre questo tempo senza aggiornamenti la stima dell'S-meter non si pubblica piu'.
const ESTIMATE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(2);

impl Default for NativeControls {
    fn default() -> Self {
        let switches = controls::CONTROLS
            .iter()
            .filter(|c| c.kind == Kind::Toggle && c.id != "monitor")
            .map(|c| (c.id.to_string(), matches!(c.id, "autoseq" | "agc" | "filter" | "show_wanted" | "tx_period" | "auto_tx" | "split")))
            .collect();
        NativeControls { monitor: AtomicBool::new(true), rx_df: AtomicU32::new(1500), tx_df: AtomicU32::new(1500), ft4: AtomicBool::new(false), redecode: AtomicBool::new(false), attenuation: AtomicU32::new(DEFAULT_ATTENUATION_TENTHS), levels: Mutex::new(BandLevels::default()), switches: Mutex::new(switches), tx_view: Mutex::new(TxView::default()), transmitting: AtomicBool::new(false), sender: Mutex::new(None), meters: Mutex::new(None), s_estimate: Mutex::new(None), halt_now: Mutex::new(None), attenuation_hook: Mutex::new(None) }
    }
}

impl NativeControls {
    pub fn is_monitoring(&self) -> bool {
        self.monitor.load(Ordering::Relaxed)
    }

    /// Il modo corrente (FT8 di base).
    pub fn mode(&self) -> Mode {
        if self.ft4.load(Ordering::Relaxed) { Mode::Ft4 } else { Mode::Ft8 }
    }

    pub fn set_mode(&self, mode: Mode) {
        self.ft4.store(mode == Mode::Ft4, Ordering::Relaxed);
    }

    /// Il livello di uscita che si sta usando: si ricorda per la banda e il modo correnti (se la banda e' nota), come `on_outAttenuation_valueChanged`.
    pub fn set_attenuation(&self, tenths: u32) {
        let tenths = tenths.min(450);
        self.attenuation.store(tenths, Ordering::Relaxed);
        if let Ok(mut memory) = self.levels.lock()
            && let Some(band) = memory.band.clone()
        {
            memory.levels.insert(self.level_key(&band), tenths);
        }
        self.notify_attenuation(tenths);
    }

    /// Chi porta il livello di uscita al dispositivo (lo imposta chi ha la catena di trasmissione).
    pub fn set_attenuation_hook(&self, hook: Arc<dyn Fn(u32) + Send + Sync>) {
        if let Ok(mut slot) = self.attenuation_hook.lock() {
            *slot = Some(hook);
        }
    }

    fn notify_attenuation(&self, tenths: u32) {
        let hook = self.attenuation_hook.lock().ok().and_then(|slot| slot.clone());
        if let Some(hook) = hook {
            hook(tenths);
        }
    }

    /// La banda della radio (letta di continuo): al primo ingresso e a ogni cambio, come `band_changed` di JTDX, se per la nuova banda c'e' un livello
    /// ricordato lo si ripristina, altrimenti si ricorda quello attuale. Con la stessa banda non fa niente.
    pub fn track_band(&self, band: &str) {
        if band.is_empty() {
            return;
        }
        let Ok(mut memory) = self.levels.lock() else { return };
        if memory.band.as_deref() == Some(band) {
            return;
        }
        memory.band = Some(band.to_string());
        let key = self.level_key(band);
        match memory.levels.get(&key) {
            Some(level) => {
                self.attenuation.store(*level, Ordering::Relaxed);
                let level = *level;
                drop(memory);
                self.notify_attenuation(level);
                return;
            }
            None => {
                memory.levels.insert(key, self.attenuation.load(Ordering::Relaxed));
            }
        }
    }

    fn level_key(&self, band: &str) -> String {
        format!("{band}{}", self.mode().name())
    }

    /// Tocca a noi il primo periodo (interruttore «Tx 1st/even»): lo Status UDP lo manda come `tx_first`, come JTDX.
    pub fn tx_first(&self) -> bool {
        self.switch_on("tx_period")
    }

    pub(crate) fn switch_on(&self, id: &str) -> bool {
        self.switches.lock().map(|all| all.iter().any(|(name, on)| name == id && *on)).unwrap_or(false)
    }

    pub(crate) fn set_switch(&self, id: &str, on: bool) {
        if id == "monitor" {
            self.monitor.store(on, Ordering::Relaxed);
        } else if let Ok(mut all) = self.switches.lock()
            && let Some(entry) = all.iter_mut().find(|(name, _)| name == id)
        {
            entry.1 = on;
        }
    }

    /// Registra gli strumenti letti dalla radio; `None` (o nessuna lettura) li spegne. Li legge `meters()` per lo stato della stazione.
    pub fn set_meters(&self, meters: Option<rig_control::meters::Meters>) {
        let converted = meters.map(|m| aurora_core::RadioMeters {
            s_points: m.s_points,
            s_over_db: m.s_over_db,
            power_w: m.power_w,
            swr: m.swr,
            alc: m.alc,
            comp_db: m.comp_db,
            id_a: m.id_a,
            vdd_v: m.vdd_v,
            estimated: false,
        });
        // un quadro senza nessuna lettura equivale a nessuno strumento
        let converted = converted.filter(|m| *m != aurora_core::RadioMeters::default());
        if let Ok(mut slot) = self.meters.lock() {
            *slot = converted;
        }
    }

    pub fn meters(&self) -> Option<aurora_core::RadioMeters> {
        self.meters.lock().ok().and_then(|m| m.clone())
    }

    /// Registra (o toglie, con `None`) lo S-meter stimato dall'audio in ingresso.
    pub(crate) fn set_s_estimate(&self, estimate: Option<crate::smeter::SEstimate>) {
        if let Ok(mut slot) = self.s_estimate.lock() {
            *slot = estimate.map(|e| (e, std::time::Instant::now()));
        }
    }

    /// I misuratori da pubblicare nello stato: quelli veri della radio, se danno l'S-meter; altrimenti l'S-meter stimato dall'audio
    /// (aggiunto alle altre letture vere, se ce ne sono, con `estimated: true`). Una stima piu' vecchia di 2 s (audio fermo) non conta.
    pub fn published_meters(&self) -> Option<aurora_core::RadioMeters> {
        let real = self.meters();
        if real.as_ref().is_some_and(|m| m.s_points.is_some()) {
            return real;
        }
        let estimate = self.s_estimate.lock().ok().and_then(|s| *s).filter(|(_, at)| at.elapsed() < ESTIMATE_MAX_AGE).map(|(e, _)| e);
        match (real, estimate) {
            (real, Some(e)) => Some(aurora_core::RadioMeters { s_points: Some(e.s_points), s_over_db: Some(e.s_over_db), estimated: true, ..real.unwrap_or_default() }),
            (real, None) => real,
        }
    }

    /// Chi riceve i comandi di trasmissione (il motore, quando parte).
    pub fn set_sender(&self, sender: mpsc::Sender<TxCommand>) {
        if let Ok(mut slot) = self.sender.lock() {
            *slot = Some(sender);
        }
    }

    /// Cosa fare SUBITO per fermare la trasmissione (audio e PTT), senza aspettare il ciclo del motore: lo imposta chi ha la catena di trasmissione.
    pub fn set_halt_now(&self, halt: Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut slot) = self.halt_now.lock() {
            *slot = Some(halt);
        }
    }

    /// Manda un comando di trasmissione al motore. STOP TX (Halt) ferma prima l'audio e il PTT dal thread di chi chiama: il ciclo del motore puo'
    /// essere occupato a decodificare un periodo per secondi, e il tasto Esc deve annullare la trasmissione all'istante.
    pub fn send(&self, command: TxCommand) -> Result<(), RigError> {
        if matches!(command, TxCommand::Halt) {
            let halt = self.halt_now.lock().ok().and_then(|slot| slot.clone());
            if let Some(halt) = halt {
                halt();
            }
        }
        let sender = self.sender.lock().map_err(|_| RigError(NO_TX.into()))?;
        sender.as_ref().ok_or_else(|| RigError(NO_TX.into()))?.send(command).map_err(|_| RigError(NO_TX.into()))
    }

    fn snapshot(&self) -> Vec<(String, bool)> {
        let enabled = self.tx_view.lock().map(|v| v.enabled).unwrap_or(false);
        let mut out = vec![("monitor".to_string(), self.is_monitoring())];
        if let Ok(all) = self.switches.lock() {
            out.extend(all.iter().map(|(id, on)| (id.clone(), if id == "enable_tx" { enabled } else { *on })));
        }
        out
    }
}

/// La «radio» del motore proprio: i pulsanti agiscono sul decoder, la frequenza va alla radio vera (se c'e').
pub struct NativeRig {
    controls: Arc<NativeControls>,
    radio: Option<Arc<dyn Rig>>,
}

impl NativeRig {
    pub fn new(controls: Arc<NativeControls>, radio: Option<Arc<dyn Rig>>) -> NativeRig {
        NativeRig { controls, radio }
    }

    fn radio(&self) -> Result<&Arc<dyn Rig>, RigError> {
        self.radio.as_ref().ok_or_else(|| RigError("nessuna radio collegata: scegli il CAT in Impostazioni → Radio".into()))
    }
}

impl Rig for NativeRig {
    fn set_frequency(&self, hz: u64) -> Result<(), RigError> {
        self.radio()?.set_frequency(hz)
    }

    fn frequency(&self) -> Result<u64, RigError> {
        self.radio()?.frequency()
    }

    fn meters(&self) -> Result<rig_control::meters::Meters, RigError> {
        self.radio()?.meters()
    }

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        if !(100..=5000).contains(&hz) {
            return Err(RigError(format!("frequenza audio fuori campo: {hz} Hz")));
        }
        self.controls.rx_df.store(hz, Ordering::Relaxed);
        // con Tx = Rx (Split spento) la trasmissione segue la ricezione
        if !self.controls.switch_on("split") {
            self.controls.tx_df.store(hz, Ordering::Relaxed);
        }
        Ok(())
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Ok(self.controls.attenuation.load(Ordering::Relaxed))
    }

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError> {
        self.controls.set_attenuation(tenths);
        Ok(())
    }

    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        match mode {
            // la radio va sulla frequenza del modo scelto (lo fa chi chiama); il motore se ne accorge al blocco di audio dopo e cambia
            // periodi, decodifica e trasmissione. JT65 e JT9 si accettano ma non hanno decoder: il modo resta quello di prima.
            "FT8" => {
                self.controls.set_mode(Mode::Ft8);
                Ok(())
            }
            "FT4" => {
                self.controls.set_mode(Mode::Ft4);
                Ok(())
            }
            "JT" | "JT65" | "JT9" => Ok(()),
            other => Err(RigError(format!("modo non gestito: {other}"))),
        }
    }

    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        Ok(self.controls.snapshot())
    }

    fn controls_and_tx(&self) -> Result<Reading, RigError> {
        let view = self.controls.tx_view.lock().map(|v| v.clone()).unwrap_or_default();
        let even = self.controls.switch_on("tx_period");
        // le etichette sono quelle di JTDX (`setMinButton`): in FT4 «TX 00» e «TX 7.5»
        let minute_label = match (self.controls.mode(), even) {
            (Mode::Ft8, true) => "TX 00/30",
            (Mode::Ft8, false) => "TX 15/45",
            (Mode::Ft4, true) => "TX 00",
            (Mode::Ft4, false) => "TX 7.5",
        };
        let tx = controls::TxState {
            messages: view.texts.to_vec(),
            selected: (view.selected > 0).then_some(view.selected),
            minute_label: minute_label.to_string(),
            autoseq_label: if self.controls.switch_on("autoseq") { "AutoSeq 3".to_string() } else { "AutoSeq".to_string() },
        };
        Ok((self.controls.snapshot(), Some(tx)))
    }

    fn select_tx_message(&self, n: u8) -> Result<(), RigError> {
        self.controls.send(TxCommand::SelectTx(n))
    }

    fn set_control(&self, id: &str, on: Option<bool>) -> Result<(), RigError> {
        let control = controls::find(id).ok_or_else(|| RigError(format!("comando sconosciuto: {id}")))?;
        match (control.kind, on) {
            (Kind::Toggle, Some(value)) => {
                self.controls.set_switch(id, value);
                // Split spento = Tx e Rx insieme: come `on_pbTxLock_clicked` di JTDX, la trasmissione va sulla frequenza di ricezione
                if id == "split" && !value {
                    self.controls.tx_df.store(self.controls.rx_df.load(Ordering::Relaxed), Ordering::Relaxed);
                }
                let command = match id {
                    // «Abilita Tx» e «AutoTX» sono due cose: il primo permette di trasmettere, il secondo fa accendere il primo al doppio clic
                    "enable_tx" => Some(TxCommand::Enable(value)),
                    "auto_tx" => Some(TxCommand::AutoTx(value)),
                    "autoseq" => Some(TxCommand::Option(OptionChange::AutoSeq(value))),
                    "skip_tx1" => Some(TxCommand::Option(OptionChange::SkipTx1(value))),
                    "bypass" => Some(TxCommand::Option(OptionChange::Bypass(value))),
                    "single_qso" => Some(TxCommand::Option(OptionChange::SingleQso(value))),
                    "answer_b4" => Some(TxCommand::Option(OptionChange::AnswerWorkedBefore(value))),
                    "rrr" => Some(TxCommand::Option(OptionChange::UseRrr(value))),
                    "tx_period" => Some(TxCommand::Option(OptionChange::TxEven(value))),
                    _ => None,
                };
                match command {
                    // abilitare la trasmissione senza motore e' un errore chiaro; le altre opzioni si ricordano comunque
                    Some(c @ TxCommand::Enable(_)) => self.controls.send(c),
                    Some(c) => {
                        let _ = self.controls.send(c);
                        Ok(())
                    }
                    None => Ok(()),
                }
            }
            (Kind::Press, None) => {
                match id {
                    // «Ferma» ferma l'ascolto
                    "stop_rx" => self.controls.monitor.store(false, Ordering::Relaxed),
                    "clear_dx" => {
                        let _ = self.controls.send(TxCommand::ClearDx);
                    }
                    // «Gen.Msgs»: rigenera i messaggi per il DX attuale
                    "gen_msgs" => {
                        let _ = self.controls.send(TxCommand::GenMsgs);
                    }
                    // Rx -> Tx e Tx -> Rx copiano le frequenze (con Split spento sono comunque uguali)
                    "rx_to_tx" => self.controls.tx_df.store(self.controls.rx_df.load(Ordering::Relaxed), Ordering::Relaxed),
                    "tx_to_rx" => self.controls.rx_df.store(self.controls.tx_df.load(Ordering::Relaxed), Ordering::Relaxed),
                    // «Cancella»: svuota i messaggi ricordati per il doppio clic
                    "erase" => {
                        let _ = self.controls.send(TxCommand::Erase);
                    }
                    // «Decodifica»: rielabora l'ultimo periodo (se non ce n'e' ancora uno, il motore non fa nulla)
                    "decode" => self.controls.redecode.store(true, Ordering::Relaxed),
                    _ => {}
                }
                Ok(())
            }
            _ => Err(RigError(format!("comando {id}: tipo di richiesta non valido"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig() -> (NativeRig, Arc<NativeControls>) {
        let controls = Arc::new(NativeControls::default());
        (NativeRig::new(Arc::clone(&controls), None), controls)
    }

    #[test]
    fn monitor_starts_on_and_follows_the_button_and_stop() {
        let (rig, controls) = rig();
        assert!(controls.is_monitoring(), "Monitora parte sempre acceso");
        rig.set_control("monitor", Some(false)).expect("spegne");
        assert!(!controls.is_monitoring());
        let reading = rig.controls().expect("lettura");
        assert!(reading.contains(&("monitor".to_string(), false)));
        rig.set_control("monitor", Some(true)).expect("accende");
        rig.set_control("stop_rx", None).expect("ferma");
        assert!(!controls.is_monitoring(), "Ferma ferma l'ascolto");
    }

    /// Valori iniziali di JTDX: `OutAttenuation` 225, `LockTxFreq` falso (Split acceso), `QuickCall` vero (AutoTX acceso), Abilita Tx spento.
    #[test]
    fn the_initial_state_is_the_one_of_jtdx() {
        let (rig, controls) = rig();
        assert_eq!(rig.output_attenuation().expect("pwr"), 225);
        let reading = rig.controls().expect("lettura");
        assert!(reading.contains(&("split".to_string(), true)), "Split e' acceso di fabbrica");
        assert!(reading.contains(&("auto_tx".to_string(), true)), "AutoTX e' acceso di fabbrica");
        assert!(reading.contains(&("enable_tx".to_string(), false)), "Abilita Tx e' spento");
        assert!(controls.switch_on("split") && controls.switch_on("auto_tx"));
    }

    /// «AutoTX» e «Abilita Tx» sono due comandi diversi: il primo non accende il secondo e non si confonde con lui nello stato.
    #[test]
    fn auto_tx_and_enable_tx_are_separate_commands_and_separate_states() {
        let (rig, controls) = rig();
        let (tx, rx) = mpsc::channel();
        controls.set_sender(tx);
        rig.set_control("auto_tx", Some(false)).expect("auto_tx");
        assert_eq!(rx.try_recv().expect("comando"), TxCommand::AutoTx(false));
        rig.set_control("enable_tx", Some(true)).expect("enable_tx");
        assert_eq!(rx.try_recv().expect("comando"), TxCommand::Enable(true));
        // lo stato: auto_tx e' il suo interruttore, enable_tx quello che dice la sequenza
        let reading = rig.controls().expect("lettura");
        assert!(reading.contains(&("auto_tx".to_string(), false)), "{reading:?}");
        controls.tx_view.lock().expect("vista").enabled = true;
        let reading = rig.controls().expect("lettura");
        assert!(reading.contains(&("enable_tx".to_string(), true)) && reading.contains(&("auto_tx".to_string(), false)), "{reading:?}");
    }

    /// `m_pwrBandTxMemory` di JTDX: il livello e' ricordato per banda (e modo); al cambio di banda si ripristina quello della nuova, se mai impostato
    /// si ricorda quello attuale.
    #[test]
    fn the_output_level_is_remembered_per_band_and_restored_on_a_band_change() {
        let (rig, controls) = rig();
        // prima lettura della banda: niente di ricordato, si ricorda il livello attuale (225)
        controls.track_band("20m");
        rig.set_output_attenuation(100).expect("pwr");
        // 40m mai vista: tiene il livello attuale (100) e lo ricorda per se'
        controls.track_band("40m");
        assert_eq!(rig.output_attenuation().expect("pwr"), 100);
        rig.set_output_attenuation(300).expect("pwr");
        // ritorno su 20m: il suo livello e' quello impostato prima
        controls.track_band("20m");
        assert_eq!(rig.output_attenuation().expect("pwr"), 100);
        controls.track_band("40m");
        assert_eq!(rig.output_attenuation().expect("pwr"), 300);
        // la stessa banda riletta non cambia niente, nemmeno un livello cambiato a mano
        rig.set_output_attenuation(250).expect("pwr");
        controls.track_band("40m");
        assert_eq!(rig.output_attenuation().expect("pwr"), 250);
        // una banda senza nome non conta
        controls.track_band("");
        assert_eq!(rig.output_attenuation().expect("pwr"), 250);
        // la chiave comprende il modo: in FT4 la stessa banda ha il suo livello
        controls.set_mode(Mode::Ft4);
        rig.set_output_attenuation(50).expect("pwr");
        controls.track_band("20m");
        assert_eq!(rig.output_attenuation().expect("pwr"), 50, "20m in FT4 non e' mai stata vista: tiene il livello attuale");
        controls.set_mode(Mode::Ft8);
        controls.track_band("40m");
        assert_eq!(rig.output_attenuation().expect("pwr"), 250, "40m in FT8 ha il suo livello");
    }

    #[test]
    fn the_other_switches_are_remembered_and_every_ui_toggle_is_listed() {
        let (rig, _) = rig();
        rig.set_control("hound", Some(true)).expect("hound");
        let reading = rig.controls().expect("lettura");
        assert!(reading.contains(&("hound".to_string(), true)));
        for control in controls::CONTROLS.iter().filter(|c| c.kind == Kind::Toggle) {
            assert!(reading.iter().any(|(id, _)| id == control.id), "manca {}", control.id);
        }
    }

    #[test]
    fn transmission_controls_say_so_when_there_is_no_engine_and_pass_commands_when_there_is() {
        let (rig, controls) = rig();
        let err = rig.set_control("enable_tx", Some(true)).expect_err("senza motore");
        assert!(err.0.contains("trasmissione"), "{err}");
        assert!(rig.select_tx_message(2).is_err());
        let (tx, rx) = mpsc::channel();
        controls.set_sender(tx);
        rig.set_control("enable_tx", Some(true)).expect("con il motore");
        rig.set_control("skip_tx1", Some(true)).expect("opzione");
        rig.select_tx_message(3).expect("Tx3");
        rig.set_control("clear_dx", None).expect("Canc DX");
        let got: Vec<TxCommand> = rx.try_iter().collect();
        assert_eq!(
            got,
            vec![TxCommand::Enable(true), TxCommand::Option(OptionChange::SkipTx1(true)), TxCommand::SelectTx(3), TxCommand::ClearDx]
        );
    }

    #[test]
    fn a_wrong_request_shape_or_an_unknown_control_is_an_error() {
        let (rig, _) = rig();
        assert!(rig.set_control("monitor", None).is_err(), "un interruttore vuole acceso o spento");
        assert!(rig.set_control("erase", Some(true)).is_err(), "un pulsante non ha stato");
        assert!(rig.set_control("nonesiste", Some(true)).is_err());
        rig.set_control("erase", None).expect("Cancella non fa danni");
    }

    #[test]
    fn audio_frequency_attenuation_and_modes() {
        let (rig, controls) = rig();
        rig.set_rx_frequency(1234).expect("rx");
        assert_eq!(controls.rx_df.load(Ordering::Relaxed), 1234);
        assert!(rig.set_rx_frequency(20).is_err());
        rig.set_output_attenuation(120).expect("pwr");
        assert_eq!(rig.output_attenuation().expect("pwr"), 120);
        assert!(rig.set_mode("FT8").is_ok());
        assert!(rig.set_mode("FT4").is_ok());
        assert!(rig.set_mode("SSB").is_err());
    }

    #[test]
    fn without_a_radio_the_frequency_commands_say_so() {
        let (rig, _) = rig();
        assert!(rig.set_frequency(14_074_000).expect_err("nessuna radio").0.contains("nessuna radio"));
        assert!(rig.frequency().is_err());
    }

    fn estimate(s: f32) -> crate::smeter::SEstimate {
        crate::smeter::SEstimate { s_points: s, s_over_db: 0.0 }
    }

    #[test]
    fn without_real_meters_the_estimated_s_meter_is_published_and_marked() {
        let controls = NativeControls::default();
        assert_eq!(controls.published_meters(), None);
        controls.set_s_estimate(Some(estimate(5.0)));
        let m = controls.published_meters().expect("stima");
        assert_eq!((m.s_points, m.s_over_db, m.estimated), (Some(5.0), Some(0.0), true));
        assert_eq!((m.power_w, m.swr, m.alc), (None, None, None));
        // in trasmissione (o ascolto spento) la stima viene tolta
        controls.set_s_estimate(None);
        assert_eq!(controls.published_meters(), None);
    }

    #[test]
    fn real_s_meter_wins_over_the_estimate() {
        let controls = NativeControls::default();
        controls.set_meters(Some(rig_control::meters::Meters { s_points: Some(7.0), s_over_db: Some(0.0), ..Default::default() }));
        controls.set_s_estimate(Some(estimate(3.0)));
        let m = controls.published_meters().expect("misuratori veri");
        assert_eq!((m.s_points, m.estimated), (Some(7.0), false));
    }

    #[test]
    fn real_meters_without_s_meter_get_the_estimate_beside_them() {
        let controls = NativeControls::default();
        controls.set_meters(Some(rig_control::meters::Meters { power_w: Some(10.0), ..Default::default() }));
        controls.set_s_estimate(Some(estimate(4.0)));
        let m = controls.published_meters().expect("misuratori");
        assert_eq!((m.power_w, m.s_points, m.estimated), (Some(10.0), Some(4.0), true));
    }
}
