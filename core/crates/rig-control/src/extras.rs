//! I dati della radio in piu' (preamplificatore, attenuatore, AGC, riduzione del rumore, guadagni, RIT/XIT, sintonizzatore, VFO e
//! secondo VFO), letti con Hamlib solo se «Leggi anche gli altri dati della radio» e' acceso. Si leggono pochi per volta, a turno, solo
//! in ricezione; quelli che la radio non sa dare si ricordano e non si chiedono piu'.

use serde::Serialize;

use crate::hamlib::RigctldClient;
use crate::meters::is_unsupported;
use crate::RigError;

/// Le letture in piu', ognuna presente solo se la radio la sa dare.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RigExtras {
    /// Preamplificatore in dB (0 = spento).
    pub preamp_db: Option<f32>,
    /// Attenuatore in dB (0 = spento).
    pub att_db: Option<f32>,
    /// AGC come lo numera Hamlib: 0 spento, 1 superfast, 2 fast, 3 slow, 5 medium, 6 auto.
    pub agc: Option<u8>,
    pub nb: Option<bool>,
    pub nr: Option<bool>,
    /// Notch automatico.
    pub notch: Option<bool>,
    pub tuner: Option<bool>,
    /// Guadagni e livelli impostati, 0.0..=1.0 della scala.
    pub rf_gain: Option<f32>,
    pub af_gain: Option<f32>,
    pub squelch: Option<f32>,
    pub power_set: Option<f32>,
    pub mic_gain: Option<f32>,
    pub rit_hz: Option<i32>,
    pub xit_hz: Option<i32>,
    /// Il VFO in uso («VFOA», «VFOB», «Main»...).
    pub vfo: Option<String>,
    /// Frequenza e modo dell'altro VFO.
    pub vfo_b_hz: Option<u64>,
    pub vfo_b_mode: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Preamp,
    Att,
    Agc,
    Nb,
    Nr,
    Notch,
    Tuner,
    RfGain,
    AfGain,
    Squelch,
    PowerSet,
    MicGain,
    Rit,
    Xit,
    Vfo,
    VfoB,
}

const ITEMS: [Item; 16] = [
    Item::VfoB,
    Item::Preamp,
    Item::Att,
    Item::Agc,
    Item::Nb,
    Item::Nr,
    Item::Notch,
    Item::Tuner,
    Item::RfGain,
    Item::AfGain,
    Item::Squelch,
    Item::PowerSet,
    Item::MicGain,
    Item::Rit,
    Item::Xit,
    Item::Vfo,
];

/// Dopo tanti guasti consecutivi una lettura si considera non disponibile.
const MAX_FAILS: u8 = 4;

/// Lo stato delle letture a turno: a che punto si e', cosa la radio non da' e l'ultimo quadro letto.
#[derive(Debug, Default)]
pub struct ExtrasPoll {
    turn: usize,
    unsupported: u32,
    fails: [u8; 16],
    pub cache: RigExtras,
}

impl ExtrasPoll {
    fn supported(&self, item: Item) -> bool {
        self.unsupported & (1 << item as u32) == 0
    }

    /// Legge `count` voci a turno (saltando quelle che la radio non da'). Gli errori di una voce non fermano le altre.
    pub fn read(&mut self, client: &RigctldClient, count: usize) {
        let mut done = 0;
        for _ in 0..ITEMS.len() {
            if done >= count {
                break;
            }
            let item = ITEMS[self.turn % ITEMS.len()];
            self.turn = self.turn.wrapping_add(1);
            if !self.supported(item) {
                continue;
            }
            done += 1;
            match read_item(client, item, &mut self.cache) {
                Ok(()) => self.fails[item as usize] = 0,
                Err(e) => {
                    let n = &mut self.fails[item as usize];
                    *n = n.saturating_add(1);
                    if is_unsupported(&e) || *n >= MAX_FAILS {
                        tracing::debug!("dato della radio non disponibile ({item:?}): {e}");
                        self.unsupported |= 1 << item as u32;
                    }
                }
            }
        }
    }
}

fn read_item(client: &RigctldClient, item: Item, out: &mut RigExtras) -> Result<(), RigError> {
    match item {
        Item::Preamp => out.preamp_db = Some(client.level("PREAMP")?),
        Item::Att => out.att_db = Some(client.level("ATT")?),
        Item::Agc => out.agc = Some(client.level("AGC")?.round().clamp(0.0, 255.0) as u8),
        Item::Nb => out.nb = Some(client.func("NB")?),
        Item::Nr => out.nr = Some(client.func("NR")?),
        Item::Notch => out.notch = Some(client.func("ANF")?),
        Item::Tuner => out.tuner = Some(client.func("TUNER")?),
        Item::RfGain => out.rf_gain = Some(client.level("RF")?),
        Item::AfGain => out.af_gain = Some(client.level("AF")?),
        Item::Squelch => out.squelch = Some(client.level("SQL")?),
        Item::PowerSet => out.power_set = Some(client.level("RFPOWER")?),
        Item::MicGain => out.mic_gain = Some(client.level("MICGAIN")?),
        Item::Rit => out.rit_hz = Some(client.offset(false)?),
        Item::Xit => out.xit_hz = Some(client.offset(true)?),
        Item::Vfo => out.vfo = Some(client.vfo()?),
        Item::VfoB => {
            let (hz, mode) = client.vfo_info("VFOB")?;
            out.vfo_b_hz = Some(hz);
            out.vfo_b_mode = Some(mode).filter(|m| !m.is_empty());
        }
    }
    Ok(())
}
