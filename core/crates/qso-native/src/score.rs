//! Il punteggio dei chiamanti di «Best caller first» di Decodium 1.0.711 (`scoreAndSort`, `REA 0x140e1eb40`; ripetuto uguale in `0x140645440` e
//! `0x140700e50`). I pesi non sono nel sorgente pubblico: sono stati letti nell'eseguibile (`docs/analisi-decodium-reverse-rea.md`, sezione 2).
//!
//! ```text
//! punteggio = 1000                                    se la voce e' bloccata (messa a mano)
//! altrimenti
//!   snr      = (SNR noto ? SNR : -10) limitato a [-24, +10]
//!   punteggio  = (snr + 24) * 30 / 34                 # da 0 a 30 punti; SNR ignoto = 12,35
//!              + min(max(attesa_ms, 0), 90000) * 20 / 90000   # da 0 a 20 punti, 90 s o piu' = 20
//!              + (DXCC mai lavorato ? +40 : DXCC nuovo su questa banda ? +25 : 0)
//!              + (gia' lavorato su questa banda ? -35 : lavorato altrove ? -12 : 0)
//! ```
//!
//! L'ordinamento di Decodium e' per inserimento, **stabile**: un elemento sorpassa il precedente solo se lo supera di piu' di `1e-9`, quindi a parita'
//! resta chi e' arrivato prima. Zone, griglie e continenti non pesano.

/// Cio' che conta per il punteggio di un chiamante.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CallerFacts {
    /// SNR dell'ultimo messaggio; `None` = ignoto (vale -10 dB).
    pub snr: Option<i32>,
    /// Da quanto aspetta (secondi): conta fino a 90 s.
    pub waited_s: f64,
    /// Il paese non e' mai stato lavorato.
    pub dxcc_new: bool,
    /// Il paese e' stato lavorato, ma non su questa banda (conta solo se `dxcc_new` e' falso).
    pub dxcc_new_band: bool,
    /// Lavorato su questa banda (oggi o sempre).
    pub worked_band: bool,
    /// Lavorato su un'altra banda o in altri giorni (conta solo se `worked_band` e' falso).
    pub worked_elsewhere: bool,
    /// Scelto a mano dall'operatore: va sempre in testa.
    pub pinned: bool,
}

/// Il punteggio di un chiamante.
pub fn caller_score(f: &CallerFacts) -> f64 {
    if f.pinned {
        return 1000.0;
    }
    let snr = f64::from(f.snr.unwrap_or(-10).clamp(-24, 10));
    let mut score = (snr + 24.0) * 30.0 / 34.0;
    score += (f.waited_s * 1000.0).clamp(0.0, 90_000.0) * 20.0 / 90_000.0;
    if f.dxcc_new {
        score += 40.0;
    } else if f.dxcc_new_band {
        score += 25.0;
    }
    if f.worked_band {
        score -= 35.0;
    } else if f.worked_elsewhere {
        score -= 12.0;
    }
    score
}

/// L'indice del punteggio migliore con l'ordinamento stabile di Decodium: a parita' (entro `1e-9`) vince il primo.
pub fn best_index(scores: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, s) in scores.iter().enumerate() {
        match best {
            None => best = Some(i),
            Some(b) if *s > scores[b] + 1e-9 => best = Some(i),
            _ => {}
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.06
    }

    /// I quattro esempi calcolati nell'analisi (sezione 2.2).
    #[test]
    fn the_four_examples_of_the_analysis() {
        // DXCC mai lavorato a -15 dB, in coda da 30 s: 7,9 + 6,7 + 40 = 54,6
        let a = CallerFacts { snr: Some(-15), waited_s: 30.0, dxcc_new: true, ..CallerFacts::default() };
        assert!(close(caller_score(&a), 54.6), "{}", caller_score(&a));
        // gia' lavorato su questa banda a +5 dB, appena arrivata: 25,6 - 35 = -9,4
        let b = CallerFacts { snr: Some(5), worked_band: true, ..CallerFacts::default() };
        assert!(close(caller_score(&b), -9.4), "{}", caller_score(&b));
        // SNR ignoto, nessuna novita', appena arrivata: 12,4
        assert!(close(caller_score(&CallerFacts::default()), 12.4), "{}", caller_score(&CallerFacts::default()));
        // voce messa a mano: 1000
        assert_eq!(caller_score(&CallerFacts { pinned: true, ..CallerFacts::default() }), 1000.0);
    }

    #[test]
    fn the_signal_is_clamped_and_the_wait_counts_up_to_ninety_seconds() {
        let weak = CallerFacts { snr: Some(-40), ..CallerFacts::default() };
        let floor = CallerFacts { snr: Some(-24), ..CallerFacts::default() };
        assert_eq!(caller_score(&weak), caller_score(&floor));
        assert_eq!(caller_score(&floor), 0.0);
        let strong = CallerFacts { snr: Some(30), ..CallerFacts::default() };
        assert!(close(caller_score(&strong), 30.0));
        let long = CallerFacts { snr: Some(-24), waited_s: 600.0, ..CallerFacts::default() };
        let ninety = CallerFacts { snr: Some(-24), waited_s: 90.0, ..CallerFacts::default() };
        assert_eq!(caller_score(&long), caller_score(&ninety));
        assert!(close(caller_score(&ninety), 20.0));
        assert_eq!(caller_score(&CallerFacts { snr: Some(-24), waited_s: -5.0, ..CallerFacts::default() }), 0.0, "un'attesa negativa non conta");
    }

    #[test]
    fn a_new_country_beats_a_new_band_and_worked_before_beats_worked_elsewhere() {
        let base = CallerFacts { snr: Some(0), ..CallerFacts::default() };
        let both = CallerFacts { dxcc_new: true, dxcc_new_band: true, ..base };
        let band = CallerFacts { dxcc_new_band: true, ..base };
        assert!(close(caller_score(&both) - caller_score(&base), 40.0));
        assert!(close(caller_score(&band) - caller_score(&base), 25.0));
        let here_and_elsewhere = CallerFacts { worked_band: true, worked_elsewhere: true, ..base };
        let elsewhere = CallerFacts { worked_elsewhere: true, ..base };
        assert!(close(caller_score(&base) - caller_score(&here_and_elsewhere), 35.0));
        assert!(close(caller_score(&base) - caller_score(&elsewhere), 12.0));
    }

    /// Ordinamento stabile: a parita' vince chi e' arrivato prima; un punteggio appena superiore sorpassa.
    #[test]
    fn ties_keep_the_arrival_order() {
        assert_eq!(best_index(&[]), None);
        assert_eq!(best_index(&[10.0, 10.0, 10.0]), Some(0));
        assert_eq!(best_index(&[10.0, 10.0 + 1e-10, 9.0]), Some(0), "sotto la tolleranza di 1e-9 e' pari");
        assert_eq!(best_index(&[10.0, 10.1, 10.1]), Some(1));
        assert_eq!(best_index(&[-5.0, -9.4, 1000.0]), Some(2));
    }
}
