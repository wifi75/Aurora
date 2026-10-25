//! I pulsanti della pagina (Monitora, Hound, Modo SWL, Sinc, AGCc, Filtro, Ferma, Decodifica, Cancella...) quando il motore e' quello proprio:
//! non c'e' nessun altro programma da comandare, quindi lo stato sta qui e agisce sul decoder. La frequenza e il cambio banda passano dalla
//! radio (OmniRig o Hamlib) se c'e'. Monitora **parte sempre acceso** all'avvio.

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc, Arc, Mutex,
};

use crate::tx::{OptionChange, TxCommand, TxView};

use rig_control::{
    controls::{self, Kind, Reading},
    Rig, RigError,
};

const NO_TX: &str = "la trasmissione non e' disponibile: manca l'ingresso audio o il motore non e' partito";

/// Lo stato dei pulsanti: il decoder lo legge a ogni periodo.
pub struct NativeControls {
    /// Monitora: con ascolto spento non si decodifica (e lo stato dice che non si sta decodificando). Di base acceso.
    pub monitor: AtomicBool,
    /// Frequenza audio di ricezione, in Hz.
    pub rx_df: AtomicU32,
    /// Attenuazione dell'uscita audio (cursore Pwr), in decimi di dB: la usera' la trasmissione.
    pub attenuation: AtomicU32,
    /// Gli altri interruttori, per nome.
    switches: Mutex<Vec<(String, bool)>>,
    /// Cio' che la pagina mostra della trasmissione (i sei messaggi, il DX, se la sequenza e' accesa): lo scrive il motore.
    pub tx_view: Mutex<TxView>,
    /// Stiamo trasmettendo (lo scrive la catena di trasmissione).
    pub transmitting: AtomicBool,
    sender: Mutex<Option<mpsc::Sender<TxCommand>>>,
}

impl Default for NativeControls {
    fn default() -> Self {
        let switches = controls::CONTROLS
            .iter()
            .filter(|c| c.kind == Kind::Toggle && c.id != "monitor")
            .map(|c| (c.id.to_string(), matches!(c.id, "autoseq" | "agc" | "filter" | "show_wanted")))
            .collect();
        NativeControls { monitor: AtomicBool::new(true), rx_df: AtomicU32::new(1500), attenuation: AtomicU32::new(0), switches: Mutex::new(switches), tx_view: Mutex::new(TxView::default()), transmitting: AtomicBool::new(false), sender: Mutex::new(None) }
    }
}

impl NativeControls {
    pub fn is_monitoring(&self) -> bool {
        self.monitor.load(Ordering::Relaxed)
    }

    pub(crate) fn switch_on(&self, id: &str) -> bool {
        self.switches.lock().map(|all| all.iter().any(|(name, on)| name == id && *on)).unwrap_or(false)
    }

    fn set_switch(&self, id: &str, on: bool) {
        if id == "monitor" {
            self.monitor.store(on, Ordering::Relaxed);
        } else if let Ok(mut all) = self.switches.lock()
            && let Some(entry) = all.iter_mut().find(|(name, _)| name == id)
        {
            entry.1 = on;
        }
    }

    /// Chi riceve i comandi di trasmissione (il motore, quando parte).
    pub fn set_sender(&self, sender: mpsc::Sender<TxCommand>) {
        if let Ok(mut slot) = self.sender.lock() {
            *slot = Some(sender);
        }
    }

    /// Manda un comando di trasmissione al motore.
    pub fn send(&self, command: TxCommand) -> Result<(), RigError> {
        let sender = self.sender.lock().map_err(|_| RigError(NO_TX.into()))?;
        sender.as_ref().ok_or_else(|| RigError(NO_TX.into()))?.send(command).map_err(|_| RigError(NO_TX.into()))
    }

    fn snapshot(&self) -> Vec<(String, bool)> {
        let enabled = self.tx_view.lock().map(|v| v.enabled).unwrap_or(false);
        let mut out = vec![("monitor".to_string(), self.is_monitoring())];
        if let Ok(all) = self.switches.lock() {
            out.extend(all.iter().map(|(id, on)| (id.clone(), if matches!(id.as_str(), "enable_tx" | "auto_tx") { enabled } else { *on })));
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

    fn set_rx_frequency(&self, hz: u32) -> Result<(), RigError> {
        if !(100..=5000).contains(&hz) {
            return Err(RigError(format!("frequenza audio fuori campo: {hz} Hz")));
        }
        self.controls.rx_df.store(hz, Ordering::Relaxed);
        Ok(())
    }

    fn output_attenuation(&self) -> Result<u32, RigError> {
        Ok(self.controls.attenuation.load(Ordering::Relaxed))
    }

    fn set_output_attenuation(&self, tenths: u32) -> Result<(), RigError> {
        self.controls.attenuation.store(tenths.min(450), Ordering::Relaxed);
        Ok(())
    }

    fn set_mode(&self, mode: &str) -> Result<(), RigError> {
        match mode {
            // la radio va sulla frequenza del modo scelto (lo fa chi chiama); FT4 e JT per ora non hanno ancora il decoder collegato
            "FT8" | "FT4" | "JT" | "JT65" | "JT9" => Ok(()),
            other => Err(RigError(format!("modo non gestito: {other}"))),
        }
    }

    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        Ok(self.controls.snapshot())
    }

    fn controls_and_tx(&self) -> Result<Reading, RigError> {
        let view = self.controls.tx_view.lock().map(|v| v.clone()).unwrap_or_default();
        let even = self.controls.switch_on("tx_period");
        let tx = controls::TxState {
            messages: view.texts.to_vec(),
            selected: (view.selected > 0).then_some(view.selected),
            minute_label: if even { "TX 00/30".to_string() } else { "TX 15/45".to_string() },
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
                let command = match id {
                    "enable_tx" | "auto_tx" => Some(TxCommand::Enable(value)),
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
                // «Ferma» ferma l'ascolto; Decodifica, Cancella e gli altri per ora non hanno nulla da fare
                match id {
                    "stop_rx" => self.controls.monitor.store(false, Ordering::Relaxed),
                    "clear_dx" => {
                        let _ = self.controls.send(TxCommand::ClearDx);
                    }
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
}
