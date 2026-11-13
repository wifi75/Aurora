//! Misure per la «spia di salute»: ultimo tempo di decodifica (scritto dal motore a fine periodo) e storia degli ultimi 10 minuti
//! (un campione ogni [`SAMPLE_EVERY_SECS`] secondi, al massimo [`HISTORY_LEN`]), tenuta in memoria. Le letture del sistema (CPU,
//! memoria) le fa chi campiona (`remote-api`): qui solo i dati, senza chiamate al sistema operativo.

use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
};

use serde::Serialize;

/// Ogni quanti secondi si prende un campione.
pub const SAMPLE_EVERY_SECS: u64 = 5;
/// Campioni tenuti: 10 minuti a uno ogni 5 s.
pub const HISTORY_LEN: usize = 120;

/// L'ultimo periodo decodificato.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LastDecode {
    /// Quanto ha preso la decodifica.
    pub decode_ms: u64,
    /// Il tempo disponibile: la durata del periodo del modo.
    pub period_ms: u64,
    /// Thread di decodifica usati.
    pub threads: usize,
    /// Quando e' finita (ms Unix).
    pub at_unix_ms: u64,
}

/// Un campione della storia.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Sample {
    pub t_unix_ms: u64,
    /// CPU di Aurora in percento di tutti i processori.
    pub cpu_pct: f64,
    /// Ultima decodifica in ms (0 se non c'e' ancora).
    pub decode_ms: u64,
    /// Tempo disponibile di quella decodifica (0 se non c'e').
    pub period_ms: u64,
}

#[derive(Default)]
struct State {
    last: Option<LastDecode>,
    history: VecDeque<Sample>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}

fn lock() -> std::sync::MutexGuard<'static, State> {
    state().lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Il motore ha finito di decodificare un periodo.
pub fn note_decode(decode_ms: u64, period_ms: u64, threads: usize, at_unix_ms: u64) {
    lock().last = Some(LastDecode { decode_ms, period_ms, threads, at_unix_ms });
}

/// L'ultimo periodo decodificato, se c'e'.
pub fn last_decode() -> Option<LastDecode> {
    lock().last
}

/// Aggiunge un campione con la CPU misurata e l'ultima decodifica; oltre [`HISTORY_LEN`] il piu' vecchio esce.
pub fn push_sample(t_unix_ms: u64, cpu_pct: f64) {
    let mut s = lock();
    let (decode_ms, period_ms) = s.last.map(|l| (l.decode_ms, l.period_ms)).unwrap_or((0, 0));
    push_into(&mut s.history, Sample { t_unix_ms, cpu_pct, decode_ms, period_ms });
}

fn push_into(history: &mut VecDeque<Sample>, sample: Sample) {
    history.push_back(sample);
    while history.len() > HISTORY_LEN {
        history.pop_front();
    }
}

/// La storia, dal piu' vecchio al piu' recente.
pub fn history() -> Vec<Sample> {
    lock().history.iter().copied().collect()
}

/// L'ultimo campione (la CPU piu' recente).
pub fn latest() -> Option<Sample> {
    lock().history.back().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_keeps_only_the_last_ten_minutes() {
        let mut h = VecDeque::new();
        for i in 0..(HISTORY_LEN as u64 + 30) {
            push_into(&mut h, Sample { t_unix_ms: i, cpu_pct: 1.0, decode_ms: 0, period_ms: 0 });
        }
        assert_eq!(h.len(), HISTORY_LEN);
        assert_eq!(h.front().map(|s| s.t_unix_ms), Some(30));
    }

    #[test]
    fn a_sample_carries_the_last_decode() {
        note_decode(900, 15_000, 3, 1);
        push_sample(2, 12.5);
        let s = latest().expect("campione");
        assert_eq!((s.decode_ms, s.period_ms), (900, 15_000));
        assert_eq!(last_decode().map(|l| l.threads), Some(3));
    }
}
