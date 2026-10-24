//! Il confronto con JTDX: per ogni periodo, quanti messaggi ha decodificato JTDX, quanti il motore proprio e quanti li hanno in comune.
//! E' il numero che dice quanto manca prima di poter fare a meno di JTDX.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
};

use serde::Serialize;

use crate::slots::SLOT_MS;

/// Dopo questo tempo dalla fine del periodo si chiude il confronto (JTDX decodifica fino a circa 14 s dopo l'inizio, i ritardi di rete si perdonano).
const SETTLE_MS: u64 = 25_000;
/// Un periodo per cui il motore proprio non ha mai risposto si scarta dopo questo tempo.
const GIVE_UP_MS: u64 = 75_000;
/// Stessa frequenza se non differiscono di piu' di questo (JTDX scrive Hz interi, il motore proprio decimali).
const FREQ_TOLERANCE_HZ: i64 = 6;
const KEEP_RECENT: usize = 20;
const KEEP_LISTS: usize = 40;

type Item = (String, i64);

#[derive(Default)]
struct SlotRecord {
    jtdx: Vec<Item>,
    native: Option<Vec<Item>>,
}

/// Il risultato del confronto di un periodo.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotSummary {
    pub slot_unix_ms: u64,
    pub jtdx: usize,
    pub native: usize,
    pub both: usize,
    pub only_jtdx: Vec<String>,
    pub only_native: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Totals {
    pub slots: u64,
    pub jtdx: u64,
    pub native: u64,
    pub both: u64,
}

/// Cio' che si mostra all'esterno.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParitySummary {
    pub totals: Totals,
    /// Quota dei messaggi di JTDX che il motore proprio ha ritrovato (0-1); `null` finche' JTDX non ne ha prodotti.
    pub parity: Option<f64>,
    pub recent: Vec<SlotSummary>,
    pub only_jtdx_recent: Vec<String>,
    pub only_native_recent: Vec<String>,
}

#[derive(Default)]
struct Inner {
    pending: BTreeMap<u64, SlotRecord>,
    totals: Totals,
    recent: VecDeque<SlotSummary>,
    only_jtdx: VecDeque<String>,
    only_native: VecDeque<String>,
}

#[derive(Default)]
pub struct Parity {
    inner: Mutex<Inner>,
}

fn push_capped(list: &mut VecDeque<String>, items: &[String]) {
    for item in items {
        list.push_back(item.clone());
        while list.len() > KEEP_LISTS {
            list.pop_front();
        }
    }
}

impl Parity {
    pub fn add_jtdx(&self, slot_unix_ms: u64, text: &str, freq_hz: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.pending.entry(slot_unix_ms).or_default().jtdx.push((text.trim().to_string(), freq_hz));
        }
    }

    pub fn add_native(&self, slot_unix_ms: u64, items: Vec<(String, i64)>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.pending.entry(slot_unix_ms).or_default().native = Some(items);
        }
    }

    /// Chiude i periodi per cui il tempo e' passato; restituisce i loro riepiloghi (da registrare).
    pub fn finalize(&self, now_unix_ms: u64) -> Vec<SlotSummary> {
        let mut done = Vec::new();
        let Ok(mut inner) = self.inner.lock() else { return done };
        let due: Vec<u64> = inner
            .pending
            .iter()
            .filter(|&(&slot, rec)| {
                let end = slot + SLOT_MS;
                if rec.native.is_some() { now_unix_ms >= end + SETTLE_MS } else { now_unix_ms >= end + GIVE_UP_MS }
            })
            .map(|(&slot, _)| slot)
            .collect();
        for slot in due {
            let Some(rec) = inner.pending.remove(&slot) else { continue };
            let Some(native) = rec.native else { continue };
            if rec.jtdx.is_empty() && native.is_empty() {
                continue;
            }
            let summary = compare(slot, &rec.jtdx, &native);
            inner.totals.slots += 1;
            inner.totals.jtdx += summary.jtdx as u64;
            inner.totals.native += summary.native as u64;
            inner.totals.both += summary.both as u64;
            push_capped(&mut inner.only_jtdx, &summary.only_jtdx);
            push_capped(&mut inner.only_native, &summary.only_native);
            inner.recent.push_back(summary.clone());
            while inner.recent.len() > KEEP_RECENT {
                inner.recent.pop_front();
            }
            done.push(summary);
        }
        done
    }

    pub fn summary(&self) -> ParitySummary {
        let Ok(inner) = self.inner.lock() else {
            return ParitySummary { totals: Totals::default(), parity: None, recent: Vec::new(), only_jtdx_recent: Vec::new(), only_native_recent: Vec::new() };
        };
        let parity = (inner.totals.jtdx > 0).then(|| inner.totals.both as f64 / inner.totals.jtdx as f64);
        ParitySummary {
            totals: inner.totals.clone(),
            parity,
            recent: inner.recent.iter().cloned().collect(),
            only_jtdx_recent: inner.only_jtdx.iter().cloned().collect(),
            only_native_recent: inner.only_native.iter().cloned().collect(),
        }
    }
}

/// Abbina i messaggi con lo stesso testo e frequenza vicina; ognuno si usa una volta sola.
pub fn compare(slot_unix_ms: u64, jtdx: &[Item], native: &[Item]) -> SlotSummary {
    let mut used = vec![false; jtdx.len()];
    let mut both = 0;
    let mut only_native = Vec::new();
    for (text, freq) in native {
        let hit = jtdx.iter().enumerate().position(|(i, (t, f))| !used[i] && t == text && (f - freq).abs() <= FREQ_TOLERANCE_HZ);
        match hit {
            Some(i) => {
                used[i] = true;
                both += 1;
            }
            None => only_native.push(format!("{freq} {text}")),
        }
    }
    let only_jtdx = jtdx.iter().zip(&used).filter(|&(_, &u)| !u).map(|((t, f), _)| format!("{f} {t}")).collect();
    SlotSummary { slot_unix_ms, jtdx: jtdx.len(), native: native.len(), both, only_jtdx, only_native }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(text: &str, freq: i64) -> Item {
        (text.to_string(), freq)
    }

    #[test]
    fn matching_uses_text_and_nearby_frequency_once() {
        let jtdx = vec![item("CQ K1ABC FN42", 1500), item("CQ K1ABC FN42", 2100), item("IU3ABC K1ABC -15", 800)];
        let native = vec![item("CQ K1ABC FN42", 1502), item("IU3ABC K1ABC -15", 900), item("73", 400)];
        let s = compare(0, &jtdx, &native);
        assert_eq!((s.jtdx, s.native, s.both), (3, 3, 1));
        assert_eq!(s.only_jtdx, vec!["2100 CQ K1ABC FN42", "800 IU3ABC K1ABC -15"]);
        assert_eq!(s.only_native, vec!["900 IU3ABC K1ABC -15", "400 73"]);
    }

    #[test]
    fn a_slot_closes_after_the_settling_time_and_feeds_the_totals() {
        let p = Parity::default();
        p.add_jtdx(15_000, "CQ K1ABC FN42", 1500);
        p.add_jtdx(15_000, "CQ W9XYZ EN37", 900);
        p.add_native(15_000, vec![item("CQ K1ABC FN42", 1500)]);
        assert!(p.finalize(15_000 + SLOT_MS + SETTLE_MS - 1).is_empty(), "troppo presto");
        let done = p.finalize(15_000 + SLOT_MS + SETTLE_MS);
        assert_eq!(done.len(), 1);
        let sum = p.summary();
        assert_eq!(sum.totals, Totals { slots: 1, jtdx: 2, native: 1, both: 1 });
        assert_eq!(sum.parity, Some(0.5));
        assert_eq!(sum.only_jtdx_recent, vec!["900 CQ W9XYZ EN37"]);
    }

    #[test]
    fn a_slot_the_native_engine_never_answered_is_dropped_quietly() {
        let p = Parity::default();
        p.add_jtdx(0, "CQ K1ABC FN42", 1500);
        assert!(p.finalize(SLOT_MS + GIVE_UP_MS).is_empty());
        assert_eq!(p.summary().totals, Totals::default());
        assert_eq!(p.summary().parity, None);
    }

    #[test]
    fn an_empty_slot_on_both_sides_is_not_counted() {
        let p = Parity::default();
        p.add_native(0, Vec::new());
        p.finalize(SLOT_MS + SETTLE_MS);
        assert_eq!(p.summary().totals.slots, 0);
    }
}
