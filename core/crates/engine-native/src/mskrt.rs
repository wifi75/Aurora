//! MSK144 in tempo reale come WSJT-X (`hspec.f90:91-100`, `decode_msk144.f90:46-62`): dall'inizio del periodo, ogni 3584 campioni nuovi
//! (0,3 s) si passa a `mskrtd` il blocco degli ultimi 7168 (0,6 s), finche' il blocco finisce entro il periodo (e comunque entro 30 s).
//! `mskrtd` ([`jt::msk144::Rtd`]) tiene il rumore medio, il filtro dei doppioni del periodo e i nominativi recenti dei messaggi brevi.
//! WSJT-X non decodifica MSK144 a fine periodo (`decodeNow=false`, `mainwindow.cpp:3288`): la decodifica di fine periodo del motore prende
//! i messaggi gia' usciti qui ([`Realtime::period_decodes`]) e li passa alla sequenza dei QSO senza ripubblicarli.

use ft8::message::CallHashes;
use jt::msk144::{BLOCK, LAST_K, Options, Rtd, STEP};

/// Campioni al secondo.
const RATE: u64 = 12;
/// Oltre questo scarto (0,2 s) fra i campioni contati e l'ora del blocco di audio si riparte dal punto giusto del periodo (audio
/// interrotto, per esempio durante la trasmissione).
const JUMP: usize = 2_400;
/// I periodi di cui si ricordano i messaggi.
const KEEP_PERIODS: usize = 2;

#[derive(Default)]
pub struct Realtime {
    rtd: Option<Rtd>,
    /// Il periodo in corso (ms Unix del suo inizio).
    slot: Option<u64>,
    /// Campioni del periodo dalla posizione `base` (gli ultimi, quanti ne servono al blocco seguente).
    buf: Vec<f32>,
    base: usize,
    /// Fine del prossimo blocco da decodificare, in campioni dall'inizio del periodo.
    next_k: usize,
    /// I messaggi usciti negli ultimi periodi.
    found: Vec<(u64, Vec<ft8::decode::Decoded>)>,
}

impl Realtime {
    /// Aggiunge l'audio (12 kHz) che finisce a `now_ms` e decodifica i blocchi completati; ritorna i messaggi nuovi con l'inizio del loro periodo.
    pub fn push(&mut self, samples: &[f32], now_ms: u64, period_ms: u64, options: &Options, hashes: &mut CallHashes) -> Vec<(u64, Vec<ft8::decode::Decoded>)> {
        let mut out = Vec::new();
        if samples.is_empty() || period_ms == 0 {
            return out;
        }
        let slot = now_ms / period_ms * period_ms;
        let pos_end = usize::try_from((now_ms - slot) * RATE).unwrap_or(usize::MAX);
        let n = samples.len();
        let mut rest = samples;
        if self.slot != Some(slot) {
            // i campioni prima del confine chiudono il periodo precedente, se e' quello appena finito
            let new_len = pos_end.min(n);
            if let Some(old) = self.slot
                && old + period_ms == slot
                && new_len < n
            {
                let got = self.feed(&samples[..n - new_len], old, period_ms, options, hashes);
                if !got.is_empty() {
                    out.push((old, got));
                }
            }
            self.start(slot, pos_end - new_len);
            rest = &samples[n - new_len..];
        } else if (self.base + self.buf.len()).abs_diff(pos_end.saturating_sub(n)) > JUMP {
            self.start(slot, pos_end.saturating_sub(n));
        }
        let got = self.feed(rest, slot, period_ms, options, hashes);
        if !got.is_empty() {
            out.push((slot, got));
        }
        out
    }

    /// Comincia il periodo `slot` con il primo campione alla posizione `at`: il primo blocco e' il primo che vi sta tutto dentro.
    fn start(&mut self, slot: u64, at: usize) {
        self.slot = Some(slot);
        self.buf.clear();
        self.base = at;
        self.next_k = (at + BLOCK).div_ceil(STEP) * STEP;
    }

    fn feed(&mut self, samples: &[f32], slot: u64, period_ms: u64, options: &Options, hashes: &mut CallHashes) -> Vec<ft8::decode::Decoded> {
        self.buf.extend_from_slice(samples);
        let last = LAST_K.min(usize::try_from(period_ms * RATE).unwrap_or(usize::MAX));
        let rtd = self.rtd.get_or_insert_with(Rtd::new);
        let mut got = Vec::new();
        while self.next_k <= last && self.base + self.buf.len() >= self.next_k {
            let a = self.next_k - BLOCK - self.base;
            let window = &self.buf[a..a + BLOCK];
            // un blocco con una meta' tutta a zero non si decodifica (`hspec.f90:95-98`)
            let silent = |w: &[f32]| w.iter().all(|&x| x == 0.0);
            if !silent(&window[..STEP]) && !silent(&window[STEP..]) {
                let tsec = (self.next_k - BLOCK) as f32 / 12_000.0;
                if let Some(d) = rtd.block(window, slot, tsec, options, hashes) {
                    got.push(ft8::decode::Decoded { freq_hz: d.freq_hz, dt_s: d.t_s, snr_db: d.snr_db, text: d.text, hint: false });
                }
            }
            self.next_k += STEP;
        }
        // si tengono solo i campioni del prossimo blocco
        let keep_from = (self.next_k - BLOCK).saturating_sub(self.base).min(self.buf.len());
        if keep_from > 0 {
            self.buf.drain(..keep_from);
            self.base += keep_from;
        }
        if !got.is_empty() {
            match self.found.iter_mut().find(|(s, _)| *s == slot) {
                Some((_, list)) => list.extend(got.iter().cloned()),
                None => {
                    self.found.push((slot, got.clone()));
                    if self.found.len() > KEEP_PERIODS {
                        self.found.remove(0);
                    }
                }
            }
        }
        got
    }

    /// I messaggi del periodo che comincia a `slot_ms`, uno per testo (il piu' forte): sono la decodifica di fine periodo di MSK144.
    pub fn period_decodes(&self, slot_ms: u64) -> Vec<ft8::decode::Decoded> {
        let mut out: Vec<ft8::decode::Decoded> = Vec::new();
        for d in self.found.iter().filter(|(s, _)| *s == slot_ms).flat_map(|(_, l)| l) {
            match out.iter_mut().find(|x| x.text == d.text) {
                Some(x) if d.snr_db > x.snr_db => *x = d.clone(),
                Some(_) => {}
                None => out.push(d.clone()),
            }
        }
        out
    }

    /// I testi gia' mostrati per il periodo che comincia a `slot_ms` (la decodifica di fine periodo non li ripubblica).
    pub fn shown(&self, slot_ms: u64) -> Vec<String> {
        self.period_decodes(slot_ms).into_iter().map(|d| d.text).collect()
    }

    /// Cambio di modo o di banda: si riparte da zero.
    pub fn reset(&mut self) {
        if self.rtd.is_some() || self.slot.is_some() {
            *self = Realtime::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ping_audio(text: &str, at: usize, len: usize, freq: f64) -> Vec<f32> {
        let tones = jt::msk144::tones(text, &mut CallHashes::default()).expect("messaggio");
        let ping = jt::msk144::modulate(&tones, freq, 4, 1.0);
        // un filo di rumore: l'audio tutto a zero non si decodifica
        let mut x = 7u64;
        let mut audio: Vec<f32> = (0..len)
            .map(|_| {
                x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                ((x >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.02
            })
            .collect();
        for (k, v) in ping.iter().enumerate() {
            audio[at + k] += v;
        }
        audio
    }

    fn run(rt: &mut Realtime, audio: &[f32], start: u64, options: &Options) -> Vec<(u64, ft8::decode::Decoded)> {
        let mut shown = Vec::new();
        for (i, chunk) in audio.chunks(1_200).enumerate() {
            let now = start + (i as u64 + 1) * 100;
            for (slot, list) in rt.push(chunk, now, 15_000, options, &mut CallHashes::default()) {
                shown.extend(list.into_iter().map(|d| (slot, d)));
            }
        }
        shown
    }

    #[test]
    fn a_ping_shows_up_before_the_end_of_the_period_and_only_once() {
        let mut rt = Realtime::default();
        let start = 1_794_000_000_000u64;
        let audio = ping_audio("CQ IU3ABC IO91", 20_000, 36_000, 1500.0);
        let shown = run(&mut rt, &audio, start, &Options::default());
        let texts: Vec<&str> = shown.iter().map(|(_, d)| d.text.as_str()).collect();
        assert_eq!(texts, vec!["CQ IU3ABC IO91"]);
        let (slot, d) = &shown[0];
        assert_eq!(*slot, start);
        // l'istante e' quello nel periodo (il ping comincia a 1,67 s e dura 0,29 s)
        assert!((1.5..2.1).contains(&d.dt_s), "{d:?}");
        assert_eq!(rt.shown(start), vec!["CQ IU3ABC IO91".to_string()]);
        assert_eq!(rt.period_decodes(start).len(), 1);
    }

    /// La frequenza di ricezione passa al decodificatore: un ping a 1580 Hz con Rx 1600 Hz.
    #[test]
    fn the_rx_frequency_reaches_the_decoder() {
        let audio = ping_audio("K1ABC IU3ABC R-05", 30_000, 48_000, 1580.0);
        let start = 1_794_000_015_000u64;
        let at_1600 = Options { f_center_hz: 1600.0, ..Options::default() };
        let shown = run(&mut Realtime::default(), &audio, start, &at_1600);
        assert!(shown.iter().any(|(_, d)| d.text == "K1ABC IU3ABC R-05" && (d.freq_hz - 1580.0).abs() <= 3.0), "{shown:?}");
    }

    /// I blocchi partono dall'inizio del periodo: con l'audio che comincia a meta' periodo il primo blocco e' il primo che vi sta tutto.
    #[test]
    fn blocks_are_aligned_to_the_period_and_resync_after_a_gap() {
        let mut rt = Realtime::default();
        let start = 1_794_000_030_000u64;
        rt.push(&vec![0.01f32; 1_200], start + 5_000, 15_000, &Options::default(), &mut CallHashes::default());
        // primo campione a 4,9 s (58 800): il primo blocco intero finisce a 19 x 3584 = 68 096
        assert_eq!((rt.base + rt.buf.len(), rt.next_k), (60_000, 19 * STEP));
        // dopo un buco di 3 s si riparte dal punto giusto
        rt.push(&vec![0.01f32; 1_200], start + 8_100, 15_000, &Options::default(), &mut CallHashes::default());
        assert_eq!(rt.base + rt.buf.len(), 97_200);
    }
}
