//! CW (alfabeto Morse): trasmissione con un tono audio dai fronti arrotondati e ricezione con un rivelatore a soglia adattiva.
//!
//! Scritto da zero dalle regole del Morse: l'unità è un punto, una linea vale 3 unità, la pausa fra gli elementi 1, fra le lettere 3 e fra
//! le parole 7; la velocità in parole al minuto (WPM) usa la parola PARIS di 50 unità, quindi un'unità dura 1,2 / WPM secondi. La tabella
//! dei caratteri è quella del Morse internazionale (ITU-R M.1677).

use crate::psk::RATE;
use std::f64::consts::PI;

/// Velocità minima e massima accettate (parole al minuto).
pub const WPM_MIN: f32 = 5.0;
pub const WPM_MAX: f32 = 60.0;

/// Il Morse internazionale: lettere, cifre e i segni di punteggiatura più usati.
const MORSE: &[(u8, &str)] = &[
    (b'A', ".-"),
    (b'B', "-..."),
    (b'C', "-.-."),
    (b'D', "-.."),
    (b'E', "."),
    (b'F', "..-."),
    (b'G', "--."),
    (b'H', "...."),
    (b'I', ".."),
    (b'J', ".---"),
    (b'K', "-.-"),
    (b'L', ".-.."),
    (b'M', "--"),
    (b'N', "-."),
    (b'O', "---"),
    (b'P', ".--."),
    (b'Q', "--.-"),
    (b'R', ".-."),
    (b'S', "..."),
    (b'T', "-"),
    (b'U', "..-"),
    (b'V', "...-"),
    (b'W', ".--"),
    (b'X', "-..-"),
    (b'Y', "-.--"),
    (b'Z', "--.."),
    (b'0', "-----"),
    (b'1', ".----"),
    (b'2', "..---"),
    (b'3', "...--"),
    (b'4', "....-"),
    (b'5', "....."),
    (b'6', "-...."),
    (b'7', "--..."),
    (b'8', "---.."),
    (b'9', "----."),
    (b'.', ".-.-.-"),
    (b',', "--..--"),
    (b'?', "..--.."),
    (b'\'', ".----."),
    (b'!', "-.-.--"),
    (b'/', "-..-."),
    (b'(', "-.--."),
    (b')', "-.--.-"),
    (b'&', ".-..."),
    (b':', "---..."),
    (b';', "-.-.-."),
    (b'=', "-...-"),
    (b'+', ".-.-."),
    (b'-', "-....-"),
    (b'_', "..--.-"),
    (b'"', ".-..-."),
    (b'$', "...-..-"),
    (b'@', ".--.-."),
];

fn pattern_of(c: u8) -> Option<&'static str> {
    let up = c.to_ascii_uppercase();
    MORSE.iter().find(|(ch, _)| *ch == up).map(|(_, p)| *p)
}

fn char_of(pattern: &str) -> Option<u8> {
    MORSE.iter().find(|(_, p)| *p == pattern).map(|(c, _)| *c)
}

/// Durata di un'unità (un punto) in secondi alla velocità data.
pub fn unit_seconds(wpm: f32) -> f64 {
    1.2 / f64::from(wpm.clamp(WPM_MIN, WPM_MAX))
}

/// Audio (a 12 kHz) del testo in Morse; `amp` è l'ampiezza di picco (0..1). I caratteri che il Morse non ha si saltano; lo spazio e il
/// ritorno a capo sono la pausa fra le parole. I fronti salgono e scendono in 5 ms per non fare «click».
pub fn modulate(text: &[u8], tone_hz: f32, wpm: f32, amp: f32) -> Vec<f32> {
    let unit = unit_seconds(wpm) * f64::from(RATE);
    // (acceso?, unita')
    let mut seq: Vec<(bool, f64)> = Vec::new();
    for &c in text {
        if c == b' ' || c == b'\n' || c == b'\r' || c == b'\t' {
            seq.push((false, 4.0));
            continue;
        }
        let Some(p) = pattern_of(c) else { continue };
        for e in p.bytes() {
            seq.push((true, if e == b'-' { 3.0 } else { 1.0 }));
            seq.push((false, 1.0));
        }
        seq.push((false, 2.0));
    }
    let lead = (0.05 * f64::from(RATE)) as usize;
    let tail = (0.25 * f64::from(RATE)) as usize;
    let mut out = vec![0.0_f32; lead];
    let step = 2.0 * PI * f64::from(tone_hz) / f64::from(RATE);
    let ramp = (0.005 * f64::from(RATE)) as usize;
    let mut phase = 0.0_f64;
    let mut t_end = lead as f64;
    for (on, units) in seq {
        t_end += units * unit;
        let start = out.len();
        let stop = t_end.round() as usize;
        let len = stop.saturating_sub(start);
        for i in 0..len {
            if !on {
                out.push(0.0);
                continue;
            }
            let edge = i.min(len - 1 - i);
            let w = if edge < ramp { 0.5 * (1.0 - (PI * edge as f64 / ramp as f64).cos()) } else { 1.0 };
            out.push((f64::from(amp) * w * phase.sin()) as f32);
            phase += step;
        }
        phase %= 2.0 * PI;
    }
    out.extend(std::iter::repeat_n(0.0, tail));
    out
}

/// Lunghezza (campioni) della finestra del rivelatore: 6 ms, cioè circa 160 Hz di banda.
const WINDOW: usize = (RATE as usize * 6) / 1000;

/// Il ricevitore: un rivelatore a quadratura sul tono, una soglia che segue il segnale e il rumore, e la misura delle durate che dà punti,
/// linee e pause. La velocità si adatta da sola ai punti e alle linee che sente (partendo da `wpm_hint`).
pub struct CwDemodulator {
    tone_hz: f32,
    squelch_db: Option<f32>,
    step: f64,
    phase: f64,
    ring_i: Vec<f32>,
    ring_q: Vec<f32>,
    sum_i: f32,
    sum_q: f32,
    pos: usize,
    level_db: f32,
    /// Media lenta della potenza di tutto l'audio: il tono deve pesare qualcosa su quella, altrimenti e' solo la coda di un altro segnale.
    wide: f32,
    peak: f32,
    floor: f32,
    key_down: bool,
    /// Campioni passati nello stato attuale (tasto giù o su).
    run: f64,
    /// Durata stimata dell'unità, in campioni.
    unit: f64,
    pattern: String,
    flushed: bool,
    word_sent: bool,
    /// Si e' stampato almeno un carattere dall'ultima pausa fra parole.
    spoke: bool,
    open: bool,
}

impl CwDemodulator {
    pub fn new(tone_hz: f32, wpm_hint: f32) -> Self {
        Self {
            tone_hz,
            squelch_db: None,
            step: 2.0 * PI * f64::from(tone_hz) / f64::from(RATE),
            phase: 0.0,
            ring_i: vec![0.0; WINDOW],
            ring_q: vec![0.0; WINDOW],
            sum_i: 0.0,
            sum_q: 0.0,
            pos: 0,
            level_db: -90.0,
            wide: 0.0,
            peak: 0.0,
            floor: 1e-4,
            key_down: false,
            run: 0.0,
            unit: unit_seconds(wpm_hint) * f64::from(RATE),
            pattern: String::new(),
            flushed: true,
            word_sent: true,
            spoke: false,
            open: true,
        }
    }

    /// Il tono su cui si ascolta.
    pub fn frequency(&self) -> f32 {
        self.tone_hz
    }

    /// Nel CW la frequenza resta quella scelta: l'AFC non cambia niente.
    pub fn set_afc(&mut self, _on: bool) {}

    pub fn set_squelch_db(&mut self, db: Option<f32>) {
        self.squelch_db = db;
    }

    /// Livello (dB) del tono e squelch aperto.
    pub fn signal(&self) -> (f32, bool) {
        (self.level_db, self.open)
    }

    /// La velocità che il ricevitore ha capito, in parole al minuto.
    pub fn wpm(&self) -> f32 {
        (1.2 / (self.unit / f64::from(RATE))) as f32
    }

    fn emit(&mut self, out: &mut Vec<u8>) {
        if self.pattern.is_empty() {
            return;
        }
        out.push(char_of(&self.pattern).unwrap_or(b'*'));
        self.pattern.clear();
        self.spoke = true;
    }

    fn clamp_unit(&mut self) {
        let lo = unit_seconds(WPM_MAX) * f64::from(RATE);
        let hi = unit_seconds(WPM_MIN) * f64::from(RATE);
        self.unit = self.unit.clamp(lo, hi);
    }

    /// Il tasto e' appena stato rilasciato: la durata del tono dice se e' un punto o una linea.
    fn mark_ended(&mut self, len: f64) {
        if len < 0.3 * self.unit {
            return;
        }
        if len < 2.0 * self.unit {
            self.pattern.push('.');
            self.unit = 0.7 * self.unit + 0.3 * len;
        } else {
            self.pattern.push('-');
            self.unit = 0.7 * self.unit + 0.3 * (len / 3.0);
        }
        self.clamp_unit();
        // un carattere piu' lungo di 7 elementi non esiste: era rumore
        if self.pattern.len() > 8 {
            self.pattern.clear();
        }
    }

    /// Passa l'audio (a 12 kHz); restituisce i caratteri decodificati.
    pub fn process(&mut self, audio: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        let n = WINDOW as f32;
        for &s in audio {
            let i = s * self.phase.cos() as f32;
            let q = -s * self.phase.sin() as f32;
            self.phase += self.step;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
            self.sum_i += i - self.ring_i[self.pos];
            self.sum_q += q - self.ring_q[self.pos];
            self.ring_i[self.pos] = i;
            self.ring_q[self.pos] = q;
            self.pos = (self.pos + 1) % WINDOW;
            let env = (self.sum_i * self.sum_i + self.sum_q * self.sum_q).sqrt() / n;

            // la soglia: il picco sale subito e scende piano, il fondo scende subito e sale piano
            self.peak = if env > self.peak { env } else { self.peak - (self.peak - self.floor) / (RATE as f32 * 6.0) };
            self.floor = if env < self.floor { env.max(1e-6) } else { self.floor + (env - self.floor) / (RATE as f32 * 10.0) };
            let span = self.peak - self.floor;
            // sale in fretta (30 ms) e scende piano (1 s): subito dopo l'avvio la media non e' ancora bassa
            let tau = if s * s > self.wide { 0.03 } else { 1.0 };
            self.wide += (s * s - self.wide) / (RATE as f32 * tau);
            let valid = self.peak > self.floor * 3.0 + 1e-4 && self.peak > 0.15 * self.wide.sqrt();
            let on_at = self.floor + 0.55 * span;
            let off_at = self.floor + 0.40 * span;
            self.level_db = 20.0 * env.max(1e-5).log10();
            let loud = self.squelch_db.is_none_or(|th| self.level_db >= th);
            self.open = valid && loud;

            let down = if self.key_down { valid && loud && env > off_at } else { valid && loud && env > on_at };
            self.run += 1.0;
            if down != self.key_down {
                let len = self.run;
                self.key_down = down;
                self.run = 0.0;
                if down {
                    // fine di una pausa: se era piu' lunga di una lettera, quella era gia' uscita
                } else {
                    self.mark_ended(len);
                    self.flushed = false;
                    self.word_sent = false;
                }
            } else if !self.key_down {
                if !self.flushed && self.run > 2.0 * self.unit {
                    self.emit(&mut out);
                    self.flushed = true;
                }
                if !self.word_sent && self.run > 5.0 * self.unit {
                    if self.spoke {
                        out.push(b' ');
                        self.spoke = false;
                    }
                    self.word_sent = true;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(wave: &[f32], tone: f32, hint: f32) -> String {
        let mut d = CwDemodulator::new(tone, hint);
        let mut audio = wave.to_vec();
        // mezzo secondo di silenzio alla fine fa uscire l'ultimo carattere
        audio.extend(std::iter::repeat_n(0.0, RATE as usize));
        let mut text = Vec::new();
        for chunk in audio.chunks(480) {
            text.extend(d.process(chunk));
        }
        String::from_utf8_lossy(&text).trim().to_string()
    }

    #[test]
    fn morse_table_round_trips_and_has_no_duplicates() {
        for (c, p) in MORSE {
            assert_eq!(char_of(p), Some(*c), "{p}");
        }
        assert_eq!(pattern_of(b'k'), Some("-.-"));
        assert_eq!(pattern_of(b'#'), None);
    }

    #[test]
    fn paris_lasts_fifty_units() {
        // «PARIS » a 20 WPM dura 3 secondi (50 unita' da 60 ms), con il silenzio d'avvio e di coda
        let wave = modulate(b"PARIS ", 700.0, 20.0, 0.9);
        let secs = wave.len() as f64 / f64::from(RATE) - 0.05 - 0.25;
        assert!((secs - 3.0).abs() < 0.02, "{secs}");
    }

    #[test]
    fn it_hears_what_it_sends_at_several_speeds() {
        let text = "CQ CQ DE IU3ABC K";
        for wpm in [12.0, 20.0, 30.0] {
            let wave = modulate(text.as_bytes(), 700.0, wpm, 0.8);
            assert_eq!(decode(&wave, 700.0, 20.0), text, "a {wpm} WPM");
        }
    }

    #[test]
    fn it_adapts_to_a_speed_far_from_the_hint() {
        // lontano dal valore di partenza i primi segni si sbagliano: un avvio «VVV» fa capire la velocita'
        let text = "TNX FER QSO 73 DE IU3ABC";
        let wave = modulate(format!("VVV VVV {text}").as_bytes(), 600.0, 30.0, 0.7);
        let heard = decode(&wave, 600.0, 18.0);
        assert!(heard.ends_with(text), "{heard}");
    }

    #[test]
    fn digits_and_punctuation_pass() {
        let text = "RST 599 QTH? 1+2=3";
        let wave = modulate(text.as_bytes(), 800.0, 22.0, 0.9);
        assert_eq!(decode(&wave, 800.0, 22.0), text);
    }

    #[test]
    fn silence_and_noise_print_nothing() {
        let mut d = CwDemodulator::new(700.0, 20.0);
        let mut seed = 12345_u32;
        let noise: Vec<f32> = (0..RATE as usize * 3)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 16) as f32 / 32768.0 - 1.0) * 0.05
            })
            .collect();
        assert!(d.process(&noise).is_empty());
        assert!(d.process(&vec![0.0; RATE as usize]).is_empty());
    }

    #[test]
    fn a_different_tone_is_not_decoded() {
        let wave = modulate(b"CQ CQ DE IU3ABC", 1500.0, 20.0, 0.9);
        assert_eq!(decode(&wave, 700.0, 20.0), "");
    }

    #[test]
    fn the_squelch_hides_weak_signals() {
        let wave = modulate(b"CQ DE IU3ABC", 700.0, 20.0, 0.01);
        let mut d = CwDemodulator::new(700.0, 20.0);
        d.set_squelch_db(Some(-30.0));
        let mut text = Vec::new();
        for chunk in wave.chunks(480) {
            text.extend(d.process(chunk));
        }
        assert!(text.is_empty());
    }
}
