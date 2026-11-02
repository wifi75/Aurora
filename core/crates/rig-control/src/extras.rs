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

/// Le impostazioni della radio che il display esteso lascia cambiare con un clic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtraItem {
    Nb,
    Nr,
    Notch,
    Atu,
    /// Avvia l'accordatura automatica (`G TUNE`): la radio emette da sola una portante bassa per qualche secondo.
    Tune,
    /// Potenza RF impostata piu' alta / piu' bassa del 5% (`L RFPOWER`, da 0 a 1).
    PowerUp,
    PowerDown,
    Preamp,
    Att,
    Agc,
    Vfo,
    Rit,
    Xit,
}

impl ExtraItem {
    /// Il nome usato dall'API («nb», «notch», «atu»...).
    pub fn parse(name: &str) -> Option<ExtraItem> {
        Some(match name {
            "nb" => ExtraItem::Nb,
            "nr" => ExtraItem::Nr,
            "notch" => ExtraItem::Notch,
            "atu" => ExtraItem::Atu,
            "tune" => ExtraItem::Tune,
            "pwr_up" => ExtraItem::PowerUp,
            "pwr_down" => ExtraItem::PowerDown,
            "preamp" => ExtraItem::Preamp,
            "att" => ExtraItem::Att,
            "agc" => ExtraItem::Agc,
            "vfo" => ExtraItem::Vfo,
            "rit" => ExtraItem::Rit,
            "xit" => ExtraItem::Xit,
            _ => return None,
        })
    }

    fn reading(self) -> &'static [Item] {
        match self {
            ExtraItem::Nb => &[Item::Nb],
            ExtraItem::Nr => &[Item::Nr],
            ExtraItem::Notch => &[Item::Notch],
            ExtraItem::Atu | ExtraItem::Tune => &[Item::Tuner],
            ExtraItem::PowerUp | ExtraItem::PowerDown => &[Item::PowerSet],
            ExtraItem::Preamp => &[Item::Preamp],
            ExtraItem::Att => &[Item::Att],
            ExtraItem::Agc => &[Item::Agc],
            ExtraItem::Vfo => &[Item::Vfo],
            ExtraItem::Rit => &[Item::Rit],
            ExtraItem::Xit => &[Item::Xit],
        }
    }
}

/// I valori che il clic prova in ordine: lo spento (0) e poi quelli che le radio accettano (10/20 dB; 6/12/18 dB).
/// Il passo della potenza impostata (frazione di quella massima).
const POWER_STEP: f32 = 0.05;

const PREAMP_CYCLE: [i32; 3] = [0, 10, 20];
const ATT_CYCLE: [i32; 6] = [0, 10, 20, 6, 12, 18];
/// AGC come lo numera Hamlib: auto, fast, medium, slow, spento.
const AGC_CYCLE: [i32; 5] = [6, 2, 5, 3, 0];

/// Il VFO «dall'altra parte» di quello dato (A<->B, Main<->Sub), con i nomi di Hamlib.
pub fn other_vfo(current: &str) -> Option<&'static str> {
    Some(match current.trim().to_ascii_lowercase().as_str() {
        "vfoa" => "VFOB",
        "vfob" => "VFOA",
        "maina" => "MainB",
        "mainb" => "MainA",
        "main" => "Sub",
        "sub" => "Main",
        "suba" => "SubB",
        "subb" => "SubA",
        _ => return None,
    })
}

/// Il prossimo valore del ciclo dopo `current` (da `cycle[0]` se non e' nell'elenco), nell'ordine di `cycle`, e poi gli altri a seguire.
fn cycle_from(cycle: &[i32], current: i32) -> Vec<i32> {
    let start = cycle.iter().position(|&v| v == current).map_or(0, |p| p + 1);
    (0..cycle.len()).map(|i| cycle[(start + i) % cycle.len()]).filter(|&v| v != current).collect()
}

/// Il valore e' stato rifiutato dalla radio (si prova il prossimo) e non e' un guasto di comunicazione.
fn is_refusal(e: &RigError) -> bool {
    matches!(crate::hamlib::error_code(e), Some(-1 | -4 | -9 | -11 | -13))
}

impl ExtrasPoll {
    /// Il clic su un indicatore: legge il valore vero dalla radio, scrive il successivo (inversione, ciclo o azzeramento), rilegge e mette in cache.
    pub fn apply(&mut self, client: &RigctldClient, item: ExtraItem) -> Result<(), RigError> {
        let cannot_read = |e: RigError, what: &str| {
            if is_unsupported(&e) { RigError(format!("la radio non sa dare questo dato ({what}): non si puo' cambiare da qui")) } else { e }
        };
        match item {
            ExtraItem::Nb | ExtraItem::Nr | ExtraItem::Notch | ExtraItem::Atu => {
                let (name, label) = match item {
                    ExtraItem::Nb => ("NB", "NB"),
                    ExtraItem::Nr => ("NR", "NR"),
                    ExtraItem::Notch => ("ANF", "notch"),
                    _ => ("TUNER", "ATU"),
                };
                let on = client.func(name).map_err(|e| cannot_read(e, label))?;
                client.set_func(name, !on)?;
            }
            ExtraItem::Preamp | ExtraItem::Att | ExtraItem::Agc => {
                let (name, cycle, label): (&str, &[i32], &str) = match item {
                    ExtraItem::Preamp => ("PREAMP", &PREAMP_CYCLE, "preamplificatore"),
                    ExtraItem::Att => ("ATT", &ATT_CYCLE, "attenuatore"),
                    _ => ("AGC", &AGC_CYCLE, "AGC"),
                };
                let current = client.level(name).map_err(|e| cannot_read(e, label))?.round() as i32;
                let mut last_refusal = None;
                let mut done = false;
                for value in cycle_from(cycle, current) {
                    match client.set_level(name, value as f32) {
                        Ok(()) => {
                            done = true;
                            break;
                        }
                        Err(e) if is_refusal(&e) => last_refusal = Some(e),
                        Err(e) => return Err(e),
                    }
                }
                if !done {
                    return Err(last_refusal.unwrap_or_else(|| RigError(format!("la radio non accetta nessun altro valore per {label}"))));
                }
            }
            ExtraItem::Tune => {
                client.vfo_op("TUNE").map_err(|e| if is_unsupported(&e) || is_refusal(&e) { RigError("la radio non accetta l'avvio dell'accordatore da CAT".into()) } else { e })?;
            }
            ExtraItem::PowerUp | ExtraItem::PowerDown => {
                let now = client.level("RFPOWER").map_err(|e| cannot_read(e, "potenza"))?;
                let step = if item == ExtraItem::PowerUp { POWER_STEP } else { -POWER_STEP };
                // a passi interi del 5%: 0,37 sale a 0,40 e scende a 0,35
                let next = (((now / POWER_STEP).round() * POWER_STEP) + step).clamp(POWER_STEP, 1.0);
                client.set_level("RFPOWER", (next * 100.0).round() / 100.0)?;
            }
            ExtraItem::Vfo => {
                let current = client.vfo().map_err(|e| cannot_read(e, "VFO"))?;
                let other = other_vfo(&current).ok_or_else(|| RigError(format!("non so passare all'altro VFO partendo da «{current}»")))?;
                client.set_vfo(other)?;
            }
            ExtraItem::Rit | ExtraItem::Xit => {
                let xit = item == ExtraItem::Xit;
                let offset = client.offset(xit).map_err(|e| cannot_read(e, if xit { "XIT" } else { "RIT" }))?;
                if offset != 0 {
                    client.set_offset(xit, 0)?;
                }
            }
        }
        // rilettura subito: il display si aggiorna e la voce, se prima era data per non disponibile, torna in lettura
        for &read in item.reading() {
            read_item(client, read, &mut self.cache)?;
            self.unsupported &= !(1 << read as u32);
            self.fails[read as usize] = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::fake_rigctld::FakeRigctld;

    fn setup() -> (FakeRigctld, RigctldClient, ExtrasPoll) {
        let fake = FakeRigctld::start(14_074_000);
        let client = fake.client(Duration::from_millis(500)).expect("client");
        (fake, client, ExtrasPoll::default())
    }

    #[test]
    fn the_cycle_goes_on_from_the_current_value_and_wraps() {
        assert_eq!(cycle_from(&AGC_CYCLE, 6), vec![2, 5, 3, 0]);
        assert_eq!(cycle_from(&AGC_CYCLE, 0), vec![6, 2, 5, 3]);
        assert_eq!(cycle_from(&AGC_CYCLE, 1), vec![6, 2, 5, 3, 0], "un valore fuori elenco riparte dall'inizio");
        assert_eq!(cycle_from(&PREAMP_CYCLE, 20), vec![0, 10]);
        assert_eq!(cycle_from(&ATT_CYCLE, 0)[0], 10);
    }

    #[test]
    fn the_other_vfo_is_the_opposite_one() {
        assert_eq!(other_vfo("VFOA"), Some("VFOB"));
        assert_eq!(other_vfo("VFOB"), Some("VFOA"));
        assert_eq!(other_vfo("Main"), Some("Sub"));
        assert_eq!(other_vfo("Mem"), None);
    }

    #[test]
    fn names_from_the_api_are_known() {
        assert_eq!(ExtraItem::parse("notch"), Some(ExtraItem::Notch));
        assert_eq!(ExtraItem::parse("atu"), Some(ExtraItem::Atu));
        assert_eq!(ExtraItem::parse("tuner"), None);
    }

    #[test]
    fn a_function_is_inverted_and_read_back() {
        let (fake, client, mut poll) = setup();
        fake.set_func("NB", false);
        poll.apply(&client, ExtraItem::Nb).expect("nb");
        assert_eq!(fake.func("NB"), Some(true));
        assert_eq!(poll.cache.nb, Some(true));
        poll.apply(&client, ExtraItem::Nb).expect("nb");
        assert_eq!(fake.func("NB"), Some(false));
        assert_eq!(poll.cache.nb, Some(false));
        assert!(fake.commands().contains(&"U NB 1".to_string()));
    }

    #[test]
    fn tune_starts_the_radio_tuner_only_when_the_radio_has_one() {
        let fake = FakeRigctld::start(14_074_000);
        let client = RigctldClient::connect(fake.addr, Duration::from_secs(2)).expect("collegato");
        let mut poll = ExtrasPoll::default();
        assert!(poll.apply(&client, ExtraItem::Tune).is_err(), "senza accordatore il comando e' rifiutato");
        fake.set_func("TUNER", true);
        poll.apply(&client, ExtraItem::Tune).expect("accordatura avviata");
        assert_eq!(fake.ops(), vec!["TUNE".to_string()]);
        assert_eq!(ExtraItem::parse("tune"), Some(ExtraItem::Tune));
    }

    #[test]
    fn power_goes_up_and_down_in_five_percent_steps() {
        let fake = FakeRigctld::start(14_074_000);
        fake.set_level("RFPOWER", "0.37");
        let client = RigctldClient::connect(fake.addr, Duration::from_secs(2)).expect("collegato");
        let mut poll = ExtrasPoll::default();
        poll.apply(&client, ExtraItem::PowerUp).expect("su");
        assert!((client.level("RFPOWER").expect("letta") - 0.40).abs() < 0.001);
        poll.apply(&client, ExtraItem::PowerDown).expect("giu");
        poll.apply(&client, ExtraItem::PowerDown).expect("giu");
        assert!((client.level("RFPOWER").expect("letta") - 0.30).abs() < 0.001);
    }

    #[test]
    fn notch_and_atu_use_the_hamlib_names() {
        let (fake, client, mut poll) = setup();
        fake.set_func("ANF", false);
        fake.set_func("TUNER", true);
        poll.apply(&client, ExtraItem::Notch).expect("notch");
        poll.apply(&client, ExtraItem::Atu).expect("atu");
        assert_eq!(fake.func("ANF"), Some(true));
        assert_eq!(fake.func("TUNER"), Some(false));
    }

    #[test]
    fn a_function_the_radio_lacks_gives_a_clear_error() {
        let (_fake, client, mut poll) = setup();
        let e = poll.apply(&client, ExtraItem::Nr).expect_err("la radio non ha NR");
        assert!(e.0.contains("non sa dare"), "{e}");
    }

    #[test]
    fn the_attenuator_cycles_skipping_refused_values() {
        let (fake, client, mut poll) = setup();
        fake.set_level("ATT", "0");
        for v in ["10", "20", "6"] {
            fake.refuse_level_value("ATT", v);
        }
        let mut seen = Vec::new();
        for _ in 0..4 {
            poll.apply(&client, ExtraItem::Att).expect("att");
            seen.push(fake.level("ATT").unwrap_or_default());
        }
        assert_eq!(seen, ["12", "18", "0", "12"]);
        assert_eq!(poll.cache.att_db, Some(12.0));
    }

    #[test]
    fn the_preamp_goes_through_10_20_and_off() {
        let (fake, client, mut poll) = setup();
        fake.set_level("PREAMP", "0");
        let mut seen = Vec::new();
        for _ in 0..4 {
            poll.apply(&client, ExtraItem::Preamp).expect("pre");
            seen.push(fake.level("PREAMP").unwrap_or_default());
        }
        assert_eq!(seen, ["10", "20", "0", "10"]);
    }

    #[test]
    fn the_agc_cycles_auto_fast_medium_slow_off() {
        let (fake, client, mut poll) = setup();
        fake.set_level("AGC", "6");
        let mut seen = Vec::new();
        for _ in 0..5 {
            poll.apply(&client, ExtraItem::Agc).expect("agc");
            seen.push(fake.level("AGC").unwrap_or_default());
        }
        assert_eq!(seen, ["2", "5", "3", "0", "6"]);
    }

    #[test]
    fn a_radio_that_refuses_everything_reports_the_refusal() {
        let (fake, client, mut poll) = setup();
        fake.set_level("AGC", "6");
        for v in ["2", "5", "3", "0"] {
            fake.refuse_level_value("AGC", v);
        }
        let e = poll.apply(&client, ExtraItem::Agc).expect_err("tutti rifiutati");
        assert!(e.0.contains("rifiutato"), "{e}");
    }

    #[test]
    fn the_vfo_switches_to_the_other_one() {
        let (fake, client, mut poll) = setup();
        poll.apply(&client, ExtraItem::Vfo).expect("vfo");
        assert_eq!(fake.vfo(), "VFOB");
        assert_eq!(poll.cache.vfo.as_deref(), Some("VFOB"));
        poll.apply(&client, ExtraItem::Vfo).expect("vfo");
        assert_eq!(fake.vfo(), "VFOA");
    }

    #[test]
    fn rit_and_xit_are_cleared_only_when_not_zero() {
        let (fake, client, mut poll) = setup();
        fake.set_offset(false, 150);
        poll.apply(&client, ExtraItem::Rit).expect("rit");
        assert_eq!(fake.offset(false), 0);
        assert_eq!(poll.cache.rit_hz, Some(0));
        fake.clear_commands();
        poll.apply(&client, ExtraItem::Xit).expect("xit");
        assert!(!fake.commands().iter().any(|c| c.starts_with('Z')), "con XIT gia' a zero non si scrive nulla");
    }
}
