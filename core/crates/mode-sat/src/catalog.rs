//! I satelliti radioamatoriali con i loro transponder: dove si ascolta e dove si trasmette, e come le due frequenze si legano
//! (transponder lineare invertente come RS-44 o non invertente). L'elenco e' di base: l'utente lo allarga con `satelliti-utente.json`
//! nella cartella dei dati. Le frequenze vanno controllate su AMSAT: i satelliti cambiano.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transponder {
    pub name: String,
    /// Modo da usare (per esempio «USB/CW», «FM»).
    pub mode: String,
    /// La banda di salita (verso il satellite), in Hz. Un transponder FM ha un'unica frequenza: basso e alto coincidono.
    pub up_low_hz: f64,
    pub up_high_hz: f64,
    /// La banda di discesa (dal satellite), in Hz: `down_low_hz` e' sempre il bordo piu' basso. In un invertente il bordo basso della salita
    /// corrisponde al bordo alto della discesa.
    pub down_low_hz: f64,
    pub down_high_hz: f64,
    /// Invertente: salendo con la frequenza di salita, la discesa scende (RS-44, FO-29, AO-7...).
    pub inverting: bool,
    /// Tono CTCSS da mandare (Hz), per i ripetitori FM che lo chiedono.
    #[serde(default)]
    pub tone_hz: Option<f64>,
}

impl Transponder {
    pub fn down_center_hz(&self) -> f64 {
        (self.down_low_hz + self.down_high_hz) / 2.0
    }

    /// La frequenza di salita nominale che corrisponde a una frequenza di discesa nominale. Un transponder invertente specchia la banda:
    /// in salita il bordo basso diventa il bordo alto in discesa.
    pub fn up_for_down(&self, down_hz: f64) -> f64 {
        let down_span = self.down_high_hz - self.down_low_hz;
        let up_span = self.up_high_hz - self.up_low_hz;
        if down_span.abs() < 1.0 || up_span.abs() < 1.0 {
            // FM: una frequenza sola
            return (self.up_low_hz + self.up_high_hz) / 2.0;
        }
        let x = ((down_hz - self.down_low_hz) / down_span).clamp(0.0, 1.0);
        if self.inverting { self.up_high_hz - x * up_span } else { self.up_low_hz + x * up_span }
    }

    /// Il contrario: la discesa che corrisponde a una salita.
    pub fn down_for_up(&self, up_hz: f64) -> f64 {
        let down_span = self.down_high_hz - self.down_low_hz;
        let up_span = self.up_high_hz - self.up_low_hz;
        if down_span.abs() < 1.0 || up_span.abs() < 1.0 {
            return self.down_center_hz();
        }
        let x = ((up_hz - self.up_low_hz) / up_span).clamp(0.0, 1.0);
        if self.inverting { self.down_high_hz - x * down_span } else { self.down_low_hz + x * down_span }
    }

    /// Il transponder e' lineare (si sceglie la frequenza dentro la banda) o a frequenza fissa (FM).
    pub fn is_linear(&self) -> bool {
        (self.down_high_hz - self.down_low_hz).abs() > 1.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SatInfo {
    pub norad: u64,
    pub name: String,
    pub transponders: Vec<Transponder>,
    /// Appunti da mostrare (frequenze dei segnali, stato).
    #[serde(default)]
    pub notes: String,
}

const MHZ: f64 = 1e6;

fn tp(name: &str, mode: &str, up: (f64, f64), down: (f64, f64), inverting: bool, tone: Option<f64>) -> Transponder {
    Transponder { name: name.into(), mode: mode.into(), up_low_hz: up.0 * MHZ, up_high_hz: up.1 * MHZ, down_low_hz: down.0 * MHZ, down_high_hz: down.1 * MHZ, inverting, tone_hz: tone }
}

/// I satelliti di base.
pub fn default_catalog() -> Vec<SatInfo> {
    vec![
        SatInfo {
            norad: 25544,
            name: "ISS".into(),
            transponders: vec![
                tp("Ripetitore FM", "FM", (145.990, 145.990), (437.800, 437.800), false, Some(67.0)),
                tp("APRS / pacchetto", "FM 1k2", (145.825, 145.825), (145.825, 145.825), false, None),
            ],
            notes: "International Space Station. FM voice repeater (check the region), APRS 145.825 MHz, SSTV events on 145.800 MHz.".into(),
        },
        SatInfo {
            norad: 44909,
            name: "RS-44".into(),
            transponders: vec![tp("Lineare invertente", "USB/CW", (145.935, 145.995), (435.610, 435.640), true, None)],
            notes: "DOSAAF-85. Linear transponder inverting, SSB and CW. CW beacon on 435.605 MHz.".into(),
        },
        SatInfo {
            norad: 27607,
            name: "SO-50".into(),
            transponders: vec![tp("Ripetitore FM", "FM", (145.850, 145.850), (436.795, 436.795), false, Some(67.0))],
            notes: "SaudiSat-1C. FM repeater: arm it with a 74.4 Hz tone for a few seconds, then 67 Hz.".into(),
        },
        SatInfo {
            norad: 24278,
            name: "FO-29".into(),
            transponders: vec![tp("Lineare invertente", "USB/CW", (145.900, 146.000), (435.800, 435.900), true, None)],
            notes: "JAS-2 Fuji-OSCAR 29. Linear transponder inverting, SSB and CW; digitalker and 1200 baud BPSK on other days.".into(),
        },
        SatInfo {
            norad: 7530,
            name: "AO-7".into(),
            transponders: vec![tp("Modo B", "USB/CW", (432.125, 432.175), (145.925, 145.975), true, None), tp("Modo A", "USB/CW", (145.850, 145.950), (29.400, 29.500), true, None)],
            notes: "OSCAR 7. Mode B and mode A alternate; beacons 145.975 and 29.502 MHz.".into(),
        },
        SatInfo {
            norad: 42761,
            name: "CAS-4A".into(),
            transponders: vec![tp("Lineare invertente", "USB/CW", (435.210, 435.230), (145.855, 145.875), true, None)],
            notes: "ZHUHAI-1 01. Linear transponder inverting.".into(),
        },
        SatInfo {
            norad: 42759,
            name: "CAS-4B".into(),
            transponders: vec![tp("Lineare invertente", "USB/CW", (435.280, 435.300), (145.925, 145.945), true, None)],
            notes: "ZHUHAI-1 02. Linear transponder inverting.".into(),
        },
    ]
}

/// L'elenco di base piu' quello dell'utente: a parita' di numero NORAD vince l'utente.
pub fn merge(base: Vec<SatInfo>, user: Vec<SatInfo>) -> Vec<SatInfo> {
    let mut out = base;
    for u in user {
        match out.iter_mut().find(|s| s.norad == u.norad) {
            Some(slot) => *slot = u,
            None => out.push(u),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rs44() -> Transponder {
        default_catalog().into_iter().find(|s| s.norad == 44909).expect("RS-44").transponders.remove(0)
    }

    #[test]
    fn l_invertente_specchia_la_banda() {
        let t = rs44();
        // il bordo alto della discesa corrisponde al bordo basso della salita
        assert!((t.up_for_down(435.640e6) - 145.935e6).abs() < 1.0);
        assert!((t.up_for_down(435.610e6) - 145.995e6).abs() < 1.0);
        // il centro resta il centro
        assert!((t.up_for_down(435.625e6) - 145.965e6).abs() < 1.0);
        // la conversione inversa torna indietro
        let down = 435.6321e6;
        assert!((t.down_for_up(t.up_for_down(down)) - down).abs() < 1.0);
    }

    #[test]
    fn il_non_invertente_segue_nello_stesso_verso() {
        let t = Transponder { name: "t".into(), mode: "USB".into(), up_low_hz: 100.0, up_high_hz: 200.0, down_low_hz: 1000.0, down_high_hz: 1100.0, inverting: false, tone_hz: None };
        assert!((t.up_for_down(1000.0) - 100.0).abs() < 1e-9);
        assert!((t.up_for_down(1100.0) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn l_fm_ha_una_frequenza_sola() {
        let iss = default_catalog().into_iter().find(|s| s.norad == 25544).expect("ISS");
        let t = &iss.transponders[0];
        assert!(!t.is_linear());
        assert!((t.up_for_down(437.8e6) - 145.990e6).abs() < 1.0);
        assert_eq!(t.tone_hz, Some(67.0));
    }

    #[test]
    fn l_elenco_dell_utente_vince_a_parita_di_norad() {
        let mut mine = default_catalog().remove(0);
        mine.name = "MIA ISS".into();
        let extra = SatInfo { norad: 99999, name: "NUOVO".into(), transponders: vec![], notes: String::new() };
        let merged = merge(default_catalog(), vec![mine, extra]);
        assert_eq!(merged.iter().find(|s| s.norad == 25544).map(|s| s.name.as_str()), Some("MIA ISS"));
        assert!(merged.iter().any(|s| s.norad == 99999));
        assert_eq!(merged.len(), default_catalog().len() + 1);
    }
}
