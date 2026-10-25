//! La frequenza di trasmissione libera (regola A di `docs/analisi-decodium-banda-satura.md`).
//!
//! **Ne' JTDX ne' Decodium la fanno**: nell'eseguibile di JTDX non c'e' nessuna stringa o funzione che cerchi uno spazio libero per la trasmissione, e in
//! Decodium la frequenza Tx la sceglie sempre l'operatore (1500 Hz di base, «Hold Tx Freq»); le sole eccezioni sono la spaziatura di 60 Hz dei flussi paralleli
//! del Multi Answer Mode e un salto casuale di +-25 Hz del solo FT2. E' quindi una funzione **nuova**, che Aurora puo' fare meglio degli altri perche' ha
//! la lista dei segnali decodificati con la frequenza di ognuno.
//!
//! Logica pura: dati i segnali recenti (frequenza, parita' del periodo in cui sono stati sentiti, eta') e la frequenza attuale dice se spostarsi e dove.
//! Un segnale FT8 occupa da `freq` a `freq + 50 Hz` (6,25 Hz per 8 toni), FT4 da `freq` a `freq + 83 Hz`: la distanza minima tra due toni di partenza
//! e' la larghezza piu' 10 Hz (`guard_hz`: 60 in FT8, 90 in FT4).
//!
//! Limite: mentre si trasmette non si riceve, quindi i segnali del **nostro** periodo di trasmissione (gli altri chiamanti, quelli che disturbano davvero)
//! non si sentono; si ricostruiscono dai cicli in cui eravamo in ascolto in entrambi i periodi e decadono. Per questo i segnali del periodo in cui
//! trasmettiamo pesano il doppio di quelli dell'altro periodo (che non si sovrappongono a noi, ma segnalano dove e' occupata la banda).

use crate::slot::Mode;

/// Un segnale sentito di recente.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heard {
    pub freq_hz: f32,
    /// E' nel periodo in cui trasmettiamo noi (peso pieno) o nell'altro (mezzo peso).
    pub same_parity: bool,
}

/// Parametri di partenza (valori proposti in `docs/analisi-decodium-banda-satura.md`, da tarare con la simulazione e con la radio).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreeTxParams {
    /// Limiti della frequenza Tx.
    pub min_hz: u32,
    pub max_hz: u32,
    /// Distanza minima tra toni: larghezza del segnale piu' 10 Hz.
    pub guard_hz: f32,
    /// Fascia preferita (audio piu' piatto della radio).
    pub sweet_lo: u32,
    pub sweet_hi: u32,
    /// Non si sposta per meno di tanto.
    pub min_move_hz: u32,
    /// Passo di ricerca.
    pub step_hz: u32,
}

impl FreeTxParams {
    pub fn for_mode(mode: Mode) -> FreeTxParams {
        FreeTxParams {
            min_hz: 300,
            max_hz: 2700,
            guard_hz: match mode {
                Mode::Ft8 => 60.0,
                Mode::Ft4 => 90.0,
            },
            sweet_lo: 800,
            sweet_hi: 2200,
            min_move_hz: 40,
            step_hz: 5,
        }
    }
}

/// La distanza dal segnale piu' vicino, in «distanza equivalente»: un segnale dell'altro periodo pesa la meta', quindi la sua distanza vale il doppio.
pub fn clearance(freq_hz: f32, heard: &[Heard]) -> f32 {
    heard
        .iter()
        .map(|h| {
            let d = (freq_hz - h.freq_hz).abs();
            if h.same_parity { d } else { d * 2.0 }
        })
        .fold(f32::MAX, f32::min)
}

/// Dove trasmettere, se serve spostarsi. `None` = restare dov'e': la frequenza attuale e' gia' libera (distanza almeno `guard_hz` da ogni segnale), oppure
/// non c'e' spazio libero da nessuna parte, oppure lo spostamento sarebbe sotto `min_move_hz`.
///
/// Sceglie la posizione con piu' spazio libero attorno (distanza dal segnale piu' vicino, contata fino a 400 Hz), con un piccolo premio dentro la fascia
/// preferita e una piccola penalita' per ogni Hz di salto (a parita' di spazio, meno salti).
pub fn choose_free_tx(heard: &[Heard], current_hz: u32, p: &FreeTxParams) -> Option<u32> {
    let current_clear = clearance(current_hz as f32, heard);
    if current_clear >= p.guard_hz {
        return None;
    }
    let step = p.step_hz.max(1);
    let mut best: Option<(f32, u32)> = None;
    let mut f = p.min_hz;
    while f <= p.max_hz {
        let clear = clearance(f as f32, heard);
        if clear >= p.guard_hz {
            let sweet = if (p.sweet_lo..=p.sweet_hi).contains(&f) { 50.0 } else { 0.0 };
            let jump = (f as f32 - current_hz as f32).abs();
            let score = clear.min(400.0) + sweet - 0.1 * jump;
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, f));
            }
        }
        f += step;
    }
    let (_, target) = best?;
    (target.abs_diff(current_hz) >= p.min_move_hz).then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heard(freqs: &[f32]) -> Vec<Heard> {
        freqs.iter().map(|f| Heard { freq_hz: *f, same_parity: true }).collect()
    }

    #[test]
    fn a_free_frequency_is_kept() {
        let p = FreeTxParams::for_mode(Mode::Ft8);
        assert_eq!(choose_free_tx(&heard(&[500.0, 900.0, 1800.0]), 1500, &p), None, "1500 e' a 300 Hz dal piu' vicino");
        assert_eq!(choose_free_tx(&[], 1500, &p), None, "banda vuota");
    }

    /// Segnali ogni 100 Hz da 300 a 2700 tranne un varco da 1400 a 1800, con un segnale a 1499 proprio dove siamo: la Tx finisce al centro di quel che resta
    /// del varco (1499..1800, cioe' circa 1650).
    #[test]
    fn a_crowded_band_sends_the_tx_into_the_widest_gap() {
        let p = FreeTxParams::for_mode(Mode::Ft8);
        let mut v: Vec<f32> = (3..=27).map(|i| i as f32 * 100.0).filter(|f| *f <= 1400.0 || *f >= 1800.0).collect();
        v.push(1499.0);
        let all = heard(&v);
        let target = choose_free_tx(&all, 1500, &p).expect("si sposta");
        assert!((1640..=1660).contains(&target), "{target}");
        assert!(clearance(target as f32, &all) >= p.guard_hz);
        // la scelta e' deterministica
        assert_eq!(Some(target), choose_free_tx(&all, 1500, &p));
    }

    #[test]
    fn without_any_gap_nothing_moves_and_the_limits_are_respected() {
        let p = FreeTxParams::for_mode(Mode::Ft8);
        // un segnale ogni 50 Hz in tutta la banda: nessuna posizione a 60 Hz da tutti
        let wall: Vec<f32> = (0..=60).map(|i| 300.0 + i as f32 * 40.0).collect();
        assert_eq!(choose_free_tx(&heard(&wall), 1500, &p), None);
        // con spazio solo fuori dai limiti non si esce da min_hz/max_hz
        let crowd: Vec<f32> = (6..=26).map(|i| i as f32 * 100.0).collect();
        let t = choose_free_tx(&heard(&crowd), 1500, &p);
        assert!(t.is_none_or(|f| (p.min_hz..=p.max_hz).contains(&f)), "{t:?}");
    }

    #[test]
    fn ft4_needs_a_wider_guard() {
        let ft8 = FreeTxParams::for_mode(Mode::Ft8);
        let ft4 = FreeTxParams::for_mode(Mode::Ft4);
        let v = heard(&[1500.0 - 70.0]);
        assert_eq!(choose_free_tx(&v, 1500, &ft8), None, "a 70 Hz e' libero in FT8");
        assert!(choose_free_tx(&v, 1500, &ft4).is_some(), "a 70 Hz non basta in FT4 (90 Hz)");
    }

    /// Un segnale dell'altro periodo pesa la meta': a 35 Hz non da' fastidio... a 25 si' (distanza equivalente 50 < 60).
    #[test]
    fn a_signal_of_the_other_period_counts_half() {
        let p = FreeTxParams::for_mode(Mode::Ft8);
        let other = |d: f32| vec![Heard { freq_hz: 1500.0 + d, same_parity: false }];
        assert_eq!(choose_free_tx(&other(35.0), 1500, &p), None);
        assert!(choose_free_tx(&other(25.0), 1500, &p).is_some());
        // lo stesso a 35 Hz nel nostro periodo disturba
        assert!(choose_free_tx(&[Heard { freq_hz: 1535.0, same_parity: true }], 1500, &p).is_some());
    }

    #[test]
    fn small_moves_are_not_worth_it() {
        let p = FreeTxParams { min_move_hz: 600, ..FreeTxParams::for_mode(Mode::Ft8) };
        let v = heard(&[1500.0]);
        // il posto con piu' spazio (400 Hz di distanza, il massimo contato) e' a 400 Hz dalla Tx: meno dei 600 richiesti
        assert_eq!(choose_free_tx(&v, 1500, &p), None);
        let p = FreeTxParams { min_move_hz: 300, ..p };
        assert_eq!(choose_free_tx(&v, 1500, &p), Some(1100));
    }

    /// Dentro la fascia piatta, a parita' di spazio si preferisce la piu' vicina alla frequenza attuale.
    #[test]
    fn it_prefers_the_sweet_band_and_fewer_jumps() {
        let p = FreeTxParams::for_mode(Mode::Ft8);
        // due varchi uguali: uno a 400-500 (fuori dalla fascia 800-2200), uno a 1700-1800; la Tx e' sul segnale a 1450
        let v = heard(&[300.0, 380.0, 520.0, 600.0, 1450.0, 1650.0, 1850.0, 2000.0]);
        let t = choose_free_tx(&v, 1450, &p).expect("si sposta");
        assert!((800..=2200).contains(&t), "{t}");
    }
}
