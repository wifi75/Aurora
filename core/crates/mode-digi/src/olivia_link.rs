//! Olivia e Contestia nel servizio dei modi testuali: il ricevitore a 12 kHz (decimatore 3:2 davanti al [`Receiver`] a 8 kHz) con lo
//! squelch sul rapporto segnale/rumore del sincronismo.
//!
//! Lo squelch lavora dentro il ricevitore, blocco per blocco: un blocco esce solo se il rapporto del sincronismo in quel momento supera la
//! soglia (`sync_threshold`). Nel solo rumore il sincronismo trova ogni tanto un rapporto sopra 3 (la soglia minima della libreria) e il
//! ricevitore scrive caratteri senza senso: la soglia di base [`OliviaSettings::SQUELCH_DEFAULT_DB`] sta sopra quei valori. Il sincronismo
//! pero' integra piu' blocchi e resta alto per qualche blocco prima e dopo un segnale, quando i blocchi in uscita sono ancora rumore: per
//! quelli c'e' in piu' lo squelch sul singolo blocco ([`BLOCK_QUALITY_MIN`]).

use crate::olivia::{Params, Variant};
use crate::olivia_rx::Receiver;
use crate::resample_8k::Decimator;
use crate::service::OliviaSettings;

/// La qualita' minima di un blocco per uscire (squelch sul singolo blocco, sempre acceso): nel solo rumore i blocchi stanno sotto 3,7, un
/// blocco ricevuto bene (anche con S/N negativo in 3 kHz) sopra 15 (prove qui sotto).
pub const BLOCK_QUALITY_MIN: f64 = 4.5;

/// Il rapporto del sincronismo (ampiezza contro rumore) in dB.
pub fn ratio_db(ratio: f64) -> f32 {
    if ratio > 0.0 { (20.0 * ratio.log10()) as f32 } else { 0.0 }
}

/// Il contrario di [`ratio_db`].
pub fn db_ratio(db: f32) -> f64 {
    10f64.powf(f64::from(db) / 20.0)
}

/// Il ricevitore di Olivia o Contestia ad audio a 12 kHz.
pub struct OliviaRx {
    rx: Receiver,
    decimator: Decimator,
    buf: Vec<f32>,
    centre_hz: f32,
    threshold_db: f32,
    level_db: f32,
}

impl OliviaRx {
    /// `squelch_db`: la soglia dello squelch (dB). `None` se toni, banda e centro non stanno nella banda audio.
    pub fn new(variant: Variant, s: &OliviaSettings, centre_hz: f32, squelch_db: f32) -> Option<OliviaRx> {
        let params: Params = s.params(variant, centre_hz);
        let mut rx = Receiver::new(params, s.sync_margin as usize, s.sync_integration as usize, db_ratio(squelch_db))?;
        rx.set_block_squelch(Some(BLOCK_QUALITY_MIN));
        Some(OliviaRx { rx, decimator: Decimator::new(), buf: Vec::new(), centre_hz, threshold_db: squelch_db, level_db: -90.0 })
    }

    /// Audio a 12 kHz in, i caratteri nuovi in uscita (senza gli zeri; CR, LF e backspace restano com'erano).
    pub fn process(&mut self, audio: &[f32]) -> String {
        self.buf.clear();
        self.decimator.process(audio, &mut self.buf);
        if !audio.is_empty() {
            let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
            self.level_db = (20.0 * rms.max(1e-5).log10()).max(-90.0);
        }
        self.rx.process(&self.buf);
        self.rx.take().into_iter().map(char::from).collect()
    }

    /// Il rapporto segnale/rumore del sincronismo (dB).
    pub fn snr_db(&self) -> f32 {
        ratio_db(self.rx.snr())
    }

    /// Il centro del segnale trovato dal sincronismo (Hz).
    pub fn frequency(&self) -> f32 {
        self.centre_hz + self.rx.frequency_offset() as f32
    }

    /// Livello (dB) e squelch aperto (il rapporto del sincronismo sopra la soglia).
    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.snr_db() >= self.threshold_db)
    }

    /// La qualita' dell'ultimo blocco arrivato all'uscita (vedi [`Receiver::last_block_quality`]).
    pub fn block_quality(&self) -> f32 {
        self.rx.last_block_quality() as f32
    }

    /// La soglia dello squelch (dB).
    pub fn threshold_db(&self) -> f32 {
        self.threshold_db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::olivia::Transmitter;
    use crate::resample_8k::Interpolator;

    /// Rumore gaussiano ripetibile.
    pub(crate) struct Noise(u64);

    impl Noise {
        pub(crate) fn new(seed: u64) -> Noise {
            Noise(seed | 1)
        }
        fn uniform(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        pub(crate) fn gauss(&mut self) -> f32 {
            ((-2.0 * self.uniform().ln()).sqrt() * (2.0 * std::f64::consts::PI * self.uniform()).cos()) as f32
        }
    }

    fn cfg(tones: u32, bw: u32) -> OliviaSettings {
        OliviaSettings { tones, bandwidth_hz: bw, ..OliviaSettings::olivia() }
    }

    /// Olivia 8/250 e 32/1000, Contestia 16/500.
    const CASI: [(Variant, u32, u32); 3] = [(Variant::Olivia, 8, 250), (Variant::Olivia, 32, 1000), (Variant::Contestia, 16, 500)];

    /// Nel solo rumore (debole, medio, forte: lo squelch non dipende dal volume) per un minuto non esce nessun carattere, con la soglia di
    /// base; con la soglia minima della libreria e senza lo squelch sui blocchi ne uscivano decine.
    #[test]
    fn lo_squelch_di_olivia_tace_nel_rumore() {
        for (variant, tones, bw) in CASI {
            for sigma in [0.02f32, 0.3, 1.0] {
                let mut rx = OliviaRx::new(variant, &cfg(tones, bw), 1500.0, OliviaSettings::SQUELCH_DEFAULT_DB).expect("rx");
                let mut n = Noise::new(7);
                let mut out = String::new();
                for _ in 0..600 {
                    let b: Vec<f32> = (0..1200).map(|_| sigma * n.gauss()).collect();
                    out.push_str(&rx.process(&b));
                }
                assert!(out.is_empty(), "{variant:?} {tones}/{bw} rumore {sigma}: {out:?}");
                assert!(!rx.signal().1, "squelch chiuso");
            }
            // la prova che serve: senza squelch lo stesso rumore scrive caratteri
            let mut rx = OliviaRx::new(variant, &cfg(tones, bw), 1500.0, 0.0).expect("rx");
            rx.rx.set_block_squelch(None);
            let mut n = Noise::new(7);
            let mut spurious = 0;
            for _ in 0..600 {
                let b: Vec<f32> = (0..1200).map(|_| 0.3 * n.gauss()).collect();
                spurious += rx.process(&b).len();
            }
            assert!(spurious > 0, "{variant:?} {tones}/{bw}: il rumore senza squelch non da' caratteri, la prova non prova niente");
        }
    }

    /// Lo squelch lascia passare un segnale nel rumore (anche forte, S/N negativo in 3 kHz) e toglie i caratteri spuri prima e dopo.
    #[test]
    fn lo_squelch_di_olivia_lascia_passare_il_segnale() {
        let text = "CQ CQ DE IU3ABC IU3ABC IO91WM K";
        for (variant, tones, bw) in CASI {
            for sigma in [0.3f32, 2.0] {
                let p = cfg(tones, bw).params(variant, 1500.0);
                let w8 = Transmitter::new(p).expect("tx").transmit(text.as_bytes());
                let mut w = vec![0.0; 12_000 * 3];
                let mut it = Interpolator::new();
                it.process(&w8, &mut w);
                w.extend(vec![0.0; 12_000 * 25]);
                let mut rx = OliviaRx::new(variant, &cfg(tones, bw), 1500.0, OliviaSettings::SQUELCH_DEFAULT_DB).expect("rx");
                let mut n = Noise::new(9);
                let mut out = String::new();
                for c in w.chunks(1200) {
                    let b: Vec<f32> = c.iter().map(|v| 0.9 * v + sigma * n.gauss()).collect();
                    out.push_str(&rx.process(&b));
                }
                assert!(out.contains(text), "{variant:?} {tones}/{bw} rumore {sigma}: {out:?}");
                assert!(out.len() <= text.len() + 4, "{variant:?} {tones}/{bw} rumore {sigma}: caratteri spuri in {out:?}");
            }
        }
    }

    #[test]
    fn i_db_del_sincronismo() {
        assert!((ratio_db(3.0) - 9.54).abs() < 0.01);
        assert!((db_ratio(ratio_db(7.0)) - 7.0).abs() < 1e-6);
        assert_eq!(ratio_db(0.0), 0.0);
    }
}
