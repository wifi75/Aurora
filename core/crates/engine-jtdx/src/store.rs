use std::collections::VecDeque;

use udp_bridge::Decode as WireDecode;

pub const STORE_CAPACITY: usize = 600;

struct Entry {
    id: u64,
    wire: WireDecode,
    inserted_ms: u64,
}

/// Decode recenti nella forma originale ricevuta da JTDX, necessari per rispondere.
pub struct DecodeStore {
    entries: VecDeque<Entry>,
    next_id: u64,
}

impl DecodeStore {
    pub fn new() -> Self {
        DecodeStore { entries: VecDeque::new(), next_id: 1 }
    }

    /// Salva un decode e restituisce il suo identificativo progressivo (parte da 1).
    pub fn insert(&mut self, wire: WireDecode, now_ms: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push_back(Entry { id, wire, inserted_ms: now_ms });
        while self.entries.len() > STORE_CAPACITY {
            self.entries.pop_front();
        }
        id
    }

    /// Identificativo di un decode identico gia' presente (per ignorare i Replay ripetuti).
    pub fn find_same(&self, wire: &WireDecode) -> Option<u64> {
        self.entries.iter().rev().find(|e| same(&e.wire, wire)).map(|e| e.id)
    }

    pub fn get(&self, id: u64, now_ms: u64, max_age_ms: u64) -> Option<&WireDecode> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .filter(|e| now_ms.saturating_sub(e.inserted_ms) <= max_age_ms)
            .map(|e| &e.wire)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Stesso decode: stessi ora, SNR, DT, DF, modo e testo (il flag "nuovo" non conta).
fn same(a: &WireDecode, b: &WireDecode) -> bool {
    a.time_ms == b.time_ms
        && a.snr == b.snr
        && a.delta_time == b.delta_time
        && a.delta_freq == b.delta_freq
        && a.mode == b.mode
        && a.message == b.message
}

impl Default for DecodeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(text: &str, t: u32) -> WireDecode {
        WireDecode {
            id: Some("JTDX".into()),
            is_new: true,
            time_ms: t,
            snr: -10,
            delta_time: 0.2,
            delta_freq: 1500,
            mode: Some("~".into()),
            message: Some(text.into()),
            low_confidence: false,
            off_air: false,
        }
    }

    #[test]
    fn store_ids_are_progressive_and_capped_at_600() {
        let mut s = DecodeStore::new();
        for i in 0..650u32 {
            assert_eq!(s.insert(wire(&format!("CQ X{i}"), i), 0), u64::from(i) + 1);
        }
        assert_eq!(s.len(), STORE_CAPACITY);
        assert!(s.get(1, 0, 10).is_none(), "i piu vecchi vengono scartati");
        assert!(s.get(650, 0, 10).is_some());
    }

    #[test]
    fn store_get_respects_max_age() {
        let mut s = DecodeStore::new();
        let id = s.insert(wire("CQ AA1AA FN42", 1), 1_000);
        assert!(s.get(id, 1_000 + 45_000, 45_000).is_some());
        assert!(s.get(id, 1_000 + 45_001, 45_000).is_none());
    }

    #[test]
    fn store_clear_empties() {
        let mut s = DecodeStore::new();
        let id = s.insert(wire("CQ AA1AA FN42", 1), 0);
        s.clear();
        assert!(s.is_empty());
        assert!(s.get(id, 0, 1_000).is_none());
    }

    #[test]
    fn store_finds_identical_decode_but_not_a_different_one() {
        let mut s = DecodeStore::new();
        let id = s.insert(wire("CQ AA1AA FN42", 5), 0);
        assert_eq!(s.find_same(&wire("CQ AA1AA FN42", 5)), Some(id));
        assert_eq!(s.find_same(&wire("CQ AA1AA FN42", 6)), None);
        assert_eq!(s.find_same(&wire("CQ BB2BB FN42", 5)), None);
    }
}
