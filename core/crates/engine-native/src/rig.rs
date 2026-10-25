//! I pulsanti della pagina (Monitora, Hound, Modo SWL, Sinc, AGCc, Filtro, Ferma, Decodifica, Cancella...) quando il motore e' quello proprio:
//! non c'e' nessun altro programma da comandare, quindi lo stato sta qui e agisce sul decoder. La frequenza e il cambio banda passano dalla
//! radio (OmniRig o Hamlib) se c'e'. Monitora **parte sempre acceso** all'avvio.

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};

use rig_control::{
    controls::{self, Kind, Reading},
    Rig, RigError,
};

const NOT_YET_TX: &str = "la trasmissione del motore proprio non e' ancora pronta";

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
}

impl Default for NativeControls {
    fn default() -> Self {
        let switches = controls::CONTROLS
            .iter()
            .filter(|c| c.kind == Kind::Toggle && c.id != "monitor")
            .map(|c| (c.id.to_string(), matches!(c.id, "autoseq" | "agc" | "filter" | "show_wanted")))
            .collect();
        NativeControls { monitor: AtomicBool::new(true), rx_df: AtomicU32::new(1500), attenuation: AtomicU32::new(0), switches: Mutex::new(switches) }
    }
}

impl NativeControls {
    pub fn is_monitoring(&self) -> bool {
        self.monitor.load(Ordering::Relaxed)
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

    fn snapshot(&self) -> Vec<(String, bool)> {
        let mut out = vec![("monitor".to_string(), self.is_monitoring())];
        if let Ok(all) = self.switches.lock() {
            out.extend(all.iter().cloned());
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
            "FT8" => Ok(()),
            "FT4" => Err(RigError("FT4 non e' ancora collegato all'ingresso audio del motore proprio".into())),
            other => Err(RigError(format!("modo non gestito: {other}"))),
        }
    }

    fn controls(&self) -> Result<Vec<(String, bool)>, RigError> {
        Ok(self.controls.snapshot())
    }

    fn controls_and_tx(&self) -> Result<Reading, RigError> {
        Ok((self.controls.snapshot(), None))
    }

    fn select_tx_message(&self, _n: u8) -> Result<(), RigError> {
        Err(RigError(NOT_YET_TX.into()))
    }

    fn set_control(&self, id: &str, on: Option<bool>) -> Result<(), RigError> {
        let control = controls::find(id).ok_or_else(|| RigError(format!("comando sconosciuto: {id}")))?;
        match (control.kind, on) {
            (Kind::Toggle, Some(value)) => {
                if control.tx && value {
                    return Err(RigError(NOT_YET_TX.into()));
                }
                self.controls.set_switch(id, value);
                Ok(())
            }
            (Kind::Press, None) => {
                // «Ferma» ferma l'ascolto; Decodifica, Cancella e gli altri per ora non hanno nulla da fare
                if id == "stop_rx" {
                    self.controls.monitor.store(false, Ordering::Relaxed);
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
    fn transmission_controls_are_refused_for_now_with_a_clear_reason() {
        let (rig, _) = rig();
        for id in ["enable_tx", "auto_tx"] {
            let err = rig.set_control(id, Some(true)).expect_err("non ancora");
            assert!(err.0.contains("trasmissione"), "{err}");
            rig.set_control(id, Some(false)).expect("spegnere si puo'");
        }
        assert!(rig.select_tx_message(2).is_err());
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
        assert!(rig.set_mode("FT4").is_err());
    }

    #[test]
    fn without_a_radio_the_frequency_commands_say_so() {
        let (rig, _) = rig();
        assert!(rig.set_frequency(14_074_000).expect_err("nessuna radio").0.contains("nessuna radio"));
        assert!(rig.frequency().is_err());
    }
}
