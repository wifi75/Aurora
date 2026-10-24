//! Stazione simulata: spettro, decode e stato deterministici a partire da un seme.

use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aurora_core::{
    Command, DecodeTag, DecodedMessage, Engine, EngineIo, Event, SpectrumRow, StationStatus,
};
use tokio::time::{interval, MissedTickBehavior};

const BINS: usize = 1024;
const F_MAX_HZ: u32 = 3000;
const ROW_PERIOD_MS: u64 = 66;
const MAX_KNOWN_DECODES: usize = 300;

const CALLS: &[&str] = &[
    "JA1XYZ", "DL2ABC", "K1ABC", "PY2XX", "EA3FOO", "VK3ZZ", "OH2BBB", "W9XYZ", "SM5QQ", "ZS6AA",
    "I2ABC", "F5XYZ", "G3AAA", "UA3DJY", "LU1ZZ", "HB9AA", "OK1ZZ", "SP9ZZ", "9A1AA", "VE3XX",
];
const RARE_PREFIXES: &[&str] = &["JA", "VK", "PY", "ZS", "LU"];
const GRIDS: &[&str] = &["PM95", "JO31", "FN42", "GG66", "JN11", "QF22", "KP20", "EN34", "JO89", "KG33"];

#[derive(Debug, Clone, Copy)]
pub struct SimConfig {
    pub seed: u64,
    pub cycle_ms: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig { seed: 1, cycle_ms: 15_000 }
    }
}

pub struct SimEngine {
    cfg: SimConfig,
}

impl SimEngine {
    pub fn new(cfg: SimConfig) -> Self {
        SimEngine { cfg }
    }
}

/// xorshift64*: piccolo, senza dipendenze e riproducibile.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

struct Track {
    bin: usize,
    amp: u8,
}

struct State {
    rng: Rng,
    status: StationStatus,
    tracks: Vec<Track>,
    seq: u64,
    next_decode_id: u64,
    known: BTreeMap<u64, ()>,
}

impl State {
    fn new(seed: u64) -> Self {
        State {
            rng: Rng::new(seed),
            status: StationStatus {
                dial_hz: 14_074_000,
                mode: "FT8".into(),
                tx_enabled: true,
                transmitting: false,
                decoding: true,
                de_call: "AA1AA".into(),
                de_grid: "FN42".into(),
                ..StationStatus::default()
            },
            tracks: Vec::new(),
            seq: 0,
            next_decode_id: 1,
            known: BTreeMap::new(),
        }
    }

    fn emit_status(&self, io: &EngineIo) {
        let _ = io.events.send(Event::Status(self.status.clone()));
    }

    fn begin_cycle(&mut self, io: &EngineIo) {
        let n_tracks = 6 + self.rng.below(5);
        self.tracks = (0..n_tracks)
            .map(|_| Track { bin: 20 + self.rng.below(BINS - 40), amp: 120 + self.rng.below(100) as u8 })
            .collect();
        self.emit_status(io);
        let n_decodes = 5 + self.rng.below(8);
        for i in 0..n_decodes {
            let decode = self.make_decode(i);
            self.known.insert(decode.id, ());
            while self.known.len() > MAX_KNOWN_DECODES {
                self.known.pop_first();
            }
            let _ = io.events.send(Event::Decode(decode));
        }
    }

    fn make_decode(&mut self, index: usize) -> DecodedMessage {
        let call = self.rng.pick(CALLS).to_string();
        let other = self.rng.pick(CALLS).to_string();
        let grid = self.rng.pick(GRIDS);
        let text = match self.rng.below(6) {
            0 => format!("CQ {call} {grid}"),
            1 => format!("CQ DX {call} {grid}"),
            2 => format!("CQ TEST {call} {grid}"),
            3 => format!("{other} {call} {grid}"),
            4 => format!("{other} {call} -{:02}", 1 + self.rng.below(24)),
            _ => format!("{other} {call} RR73"),
        };
        let is_cq = text.starts_with("CQ");
        let rare = RARE_PREFIXES.iter().any(|p| call.starts_with(p));
        let tag = if is_cq && rare {
            Some(DecodeTag::NewDxcc)
        } else if is_cq && self.rng.below(10) == 0 {
            Some(DecodeTag::NewZone)
        } else if is_cq {
            Some(DecodeTag::Cq)
        } else {
            None
        };
        let bin = self.tracks.get(index % self.tracks.len().max(1)).map_or(100, |t| t.bin);
        let id = self.next_decode_id;
        self.next_decode_id += 1;
        DecodedMessage {
            id,
            time_ms: ms_since_midnight_utc(),
            snr: -(self.rng.below(25) as i32) - 1,
            delta_time: (self.rng.below(20) as f32 - 5.0) / 10.0,
            freq_hz: (bin as u32 * F_MAX_HZ) / BINS as u32,
            text,
            tag,
            country: None,
        }
    }

    fn emit_row(&mut self, io: &EngineIo) {
        let mut bins: Vec<u8> = (0..BINS).map(|_| 18 + self.rng.below(30) as u8).collect();
        for t in &self.tracks {
            for (offset, share) in [(0i32, 100u32), (-1, 45), (1, 45), (-2, 15), (2, 15)] {
                let i = t.bin as i32 + offset;
                if (0..BINS as i32).contains(&i) {
                    let add = (t.amp as u32 * share / 100) as u8;
                    bins[i as usize] = bins[i as usize].saturating_add(add);
                }
            }
        }
        self.seq += 1;
        let row = SpectrumRow { seq: self.seq, f_min_hz: 0, f_max_hz: F_MAX_HZ, bins };
        if row.validate().is_ok() {
            let _ = io.spectrum.send(row);
        }
    }

    fn handle(&mut self, command: Command, io: &EngineIo) {
        match command {
            Command::HaltTx => {
                self.status.transmitting = false;
                let _ = io.events.send(Event::TxHalted);
                self.emit_status(io);
            }
            Command::Reply { decode_id } => {
                if self.known.contains_key(&decode_id) {
                    self.status.transmitting = true;
                    self.emit_status(io);
                }
            }
            // il simulatore non ha un motore di messaggi: questi comandi non hanno effetto
            Command::SendFreeText { .. } | Command::CallCq { .. } | Command::SetTxFrequency { .. } => {}
        }
    }
}

fn ms_since_midnight_utc() -> u32 {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis());
    (ms % 86_400_000) as u32
}

impl Engine for SimEngine {
    async fn run(self, mut io: EngineIo) {
        let mut state = State::new(self.cfg.seed);
        let mut rows = interval(Duration::from_millis(ROW_PERIOD_MS));
        let mut cycle = interval(Duration::from_millis(self.cfg.cycle_ms.max(1)));
        rows.set_missed_tick_behavior(MissedTickBehavior::Skip);
        cycle.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = cycle.tick() => state.begin_cycle(&io),
                command = io.commands.recv() => match command {
                    Some(c) => state.handle(c, &io),
                    None => return,
                },
                _ = rows.tick() => state.emit_row(&io),
            }
        }
    }
}
