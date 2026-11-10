//! Tabella dei modi SSTV e disposizione dei segmenti di ogni riga.
//!
//! Codici VIS, risoluzioni, ordine dei colori e famiglie dei modi come in `sstvparam.cpp` e `modes/*.cpp` di QSSTV
//! (Johan Maes ON4QZ, GPL); i tempi dei segmenti sono quelli pubblicati delle specifiche SSTV (la durata delle righe coincide con
//! `imageTime / numberOfDataLines` della tabella di QSSTV).

use serde::{Deserialize, Serialize};

/// Frequenze dei toni (Hz).
pub const SYNC_HZ: f32 = 1200.0;
pub const BLACK_HZ: f32 = 1500.0;
pub const WHITE_HZ: f32 = 2300.0;
pub const LEADER_HZ: f32 = 1900.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SstvMode {
    Martin1,
    Martin2,
    Scottie1,
    Scottie2,
    ScottieDx,
    Robot36,
    Robot72,
    Pd50,
    Pd90,
    Pd120,
    Pd160,
    Pd180,
    Pd240,
    Pd290,
    Sc2_180,
}

/// Famiglia: dice come sono disposti i canali nella riga e come si combinano i colori.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Sincronismo in testa, poi verde, blu, rosso (modeGBR).
    Martin,
    /// Verde, blu, sincronismo a meta', rosso (modeGBR2); un sincronismo in piu' prima della prima riga.
    Scottie,
    /// Rosso, verde, blu dopo il sincronismo (modeRGB).
    Wraase,
    /// Y e una crominanza per riga, alternate (modeRobot1).
    Robot36,
    /// Y, R-Y, B-Y in ogni riga (modeRobot2).
    Robot72,
    /// Due righe per gruppo: Y pari, R-Y, B-Y, Y dispari (modePD).
    Pd,
}

/// Canale trasmesso in un tratto di scansione.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chan {
    R,
    G,
    B,
    /// Luminanza della riga (per PD: la prima del gruppo).
    Y0,
    /// Luminanza della seconda riga del gruppo (PD).
    Y1,
    /// R-Y.
    V,
    /// B-Y.
    U,
    /// Crominanza alternata del Robot 36: R-Y nelle righe pari, B-Y nelle dispari.
    Alt,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SegKind {
    Tone(f32),
    /// Il separatore del Robot 36: 1500 Hz nelle righe pari (R-Y), 2300 Hz nelle dispari (B-Y).
    RobotSep,
    Scan(Chan),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub kind: SegKind,
    /// Durata in secondi.
    pub dur: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModeSpec {
    pub mode: SstvMode,
    pub name: &'static str,
    pub id: &'static str,
    /// Codice VIS a 7 bit (senza parita').
    pub vis: u8,
    pub width: u32,
    pub height: u32,
    pub family: Family,
    pub sync: f64,
    pub porch: f64,
    pub sep: f64,
    /// Durata di un canale (per i Robot: quella della Y).
    pub scan: f64,
}

const fn spec(mode: SstvMode, name: &'static str, id: &'static str, vis: u8, width: u32, height: u32, family: Family, sync_ms: f64, porch_ms: f64, sep_ms: f64, scan_ms: f64) -> ModeSpec {
    ModeSpec { mode, name, id, vis, width, height, family, sync: sync_ms / 1000.0, porch: porch_ms / 1000.0, sep: sep_ms / 1000.0, scan: scan_ms / 1000.0 }
}

pub const MODES: [ModeSpec; 15] = [
    spec(SstvMode::Martin1, "Martin 1", "M1", 44, 320, 256, Family::Martin, 4.862, 0.572, 0.572, 146.432),
    spec(SstvMode::Martin2, "Martin 2", "M2", 40, 320, 256, Family::Martin, 4.862, 0.572, 0.572, 73.216),
    spec(SstvMode::Scottie1, "Scottie 1", "S1", 60, 320, 256, Family::Scottie, 9.0, 1.5, 1.5, 138.24),
    spec(SstvMode::Scottie2, "Scottie 2", "S2", 56, 320, 256, Family::Scottie, 9.0, 1.5, 1.5, 88.064),
    spec(SstvMode::ScottieDx, "Scottie DX", "SDX", 76, 320, 256, Family::Scottie, 9.0, 1.5, 1.5, 345.6),
    spec(SstvMode::Robot36, "Robot 36", "R36", 8, 320, 240, Family::Robot36, 9.0, 3.0, 4.5, 88.0),
    spec(SstvMode::Robot72, "Robot 72", "R72", 12, 320, 240, Family::Robot72, 9.0, 3.0, 4.5, 138.0),
    spec(SstvMode::Pd50, "PD 50", "PD50", 93, 320, 256, Family::Pd, 20.0, 2.08, 0.0, 91.52),
    spec(SstvMode::Pd90, "PD 90", "PD90", 99, 320, 256, Family::Pd, 20.0, 2.08, 0.0, 170.24),
    spec(SstvMode::Pd120, "PD 120", "PD120", 95, 640, 496, Family::Pd, 20.0, 2.08, 0.0, 121.6),
    spec(SstvMode::Pd160, "PD 160", "PD160", 98, 512, 400, Family::Pd, 20.0, 2.08, 0.0, 195.584),
    spec(SstvMode::Pd180, "PD 180", "PD180", 96, 640, 496, Family::Pd, 20.0, 2.08, 0.0, 183.04),
    spec(SstvMode::Pd240, "PD 240", "PD240", 97, 640, 496, Family::Pd, 20.0, 2.08, 0.0, 244.48),
    spec(SstvMode::Pd290, "PD 290", "PD290", 94, 800, 616, Family::Pd, 20.0, 2.08, 0.0, 228.8),
    spec(SstvMode::Sc2_180, "Wraase SC2-180", "SC2-180", 55, 320, 256, Family::Wraase, 5.5225, 0.5, 0.0, 235.0),
];

/// Durata di intestazione: toni di calibrazione (0,8 s), leader (0,61 s), VIS (0,3 s).
pub const HEADER_SECONDS: f64 = 0.8 + 0.61 + 0.3;

impl SstvMode {
    pub fn spec(self) -> &'static ModeSpec {
        // la tabella copre ogni variante
        MODES.iter().find(|m| m.mode == self).unwrap_or(&MODES[0])
    }

    pub fn id(self) -> &'static str {
        self.spec().id
    }

    pub fn from_id(id: &str) -> Option<SstvMode> {
        MODES.iter().find(|m| m.id.eq_ignore_ascii_case(id.trim())).map(|m| m.mode)
    }

    pub fn from_vis(code: u8) -> Option<SstvMode> {
        MODES.iter().find(|m| m.vis == code).map(|m| m.mode)
    }

    pub fn all() -> impl Iterator<Item = SstvMode> {
        MODES.iter().map(|m| m.mode)
    }
}

impl Serialize for SstvMode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for SstvMode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<SstvMode, D::Error> {
        let text = String::deserialize(d)?;
        SstvMode::from_id(&text).ok_or_else(|| serde::de::Error::custom(format!("modo SSTV sconosciuto: {text}")))
    }
}

impl ModeSpec {
    /// Righe dell'immagine trasmesse in un gruppo (2 per i PD).
    pub fn rows_per_group(&self) -> u32 {
        if self.family == Family::Pd { 2 } else { 1 }
    }

    pub fn groups(&self) -> u32 {
        self.height / self.rows_per_group()
    }

    /// Durata della scansione di un canale di crominanza (Robot) o di un canale qualunque.
    fn chroma(&self) -> f64 {
        match self.family {
            Family::Robot36 | Family::Robot72 => self.scan / 2.0,
            _ => self.scan,
        }
    }

    /// I segmenti di un gruppo, nell'ordine in cui si trasmettono.
    pub fn segments(&self) -> Vec<Segment> {
        let t = |f: f32, dur: f64| Segment { kind: SegKind::Tone(f), dur };
        let s = |c: Chan, dur: f64| Segment { kind: SegKind::Scan(c), dur };
        let (sync, porch, sep, scan, chroma) = (self.sync, self.porch, self.sep, self.scan, self.chroma());
        match self.family {
            Family::Martin => vec![t(SYNC_HZ, sync), t(BLACK_HZ, porch), s(Chan::G, scan), t(BLACK_HZ, sep), s(Chan::B, scan), t(BLACK_HZ, sep), s(Chan::R, scan), t(BLACK_HZ, sep)],
            Family::Scottie => vec![t(BLACK_HZ, sep), s(Chan::G, scan), t(BLACK_HZ, sep), s(Chan::B, scan), t(SYNC_HZ, sync), t(BLACK_HZ, porch), s(Chan::R, scan)],
            Family::Wraase => vec![t(SYNC_HZ, sync), t(BLACK_HZ, porch), s(Chan::R, scan), s(Chan::G, scan), s(Chan::B, scan)],
            Family::Robot36 => vec![t(SYNC_HZ, sync), t(BLACK_HZ, porch), s(Chan::Y0, scan), Segment { kind: SegKind::RobotSep, dur: sep }, t(LEADER_HZ, 0.0015), s(Chan::Alt, chroma)],
            Family::Robot72 => vec![t(SYNC_HZ, sync), t(BLACK_HZ, porch), s(Chan::Y0, scan), t(BLACK_HZ, sep), t(LEADER_HZ, 0.0015), s(Chan::V, chroma), t(WHITE_HZ, sep), t(LEADER_HZ, 0.0015), s(Chan::U, chroma)],
            Family::Pd => vec![t(SYNC_HZ, sync), t(BLACK_HZ, porch), s(Chan::Y0, scan), s(Chan::V, scan), s(Chan::U, scan), s(Chan::Y1, scan)],
        }
    }

    /// Durata di un gruppo in secondi.
    pub fn group_seconds(&self) -> f64 {
        self.segments().iter().map(|s| s.dur).sum()
    }

    /// Dove comincia il sincronismo dentro il gruppo (secondi dall'inizio del gruppo).
    pub fn sync_offset(&self) -> f64 {
        let mut at = 0.0;
        for seg in self.segments() {
            if seg.kind == SegKind::Tone(SYNC_HZ) {
                return at;
            }
            at += seg.dur;
        }
        0.0
    }

    /// Il sincronismo in piu' prima della prima riga (Scottie).
    pub fn lead_sync(&self) -> f64 {
        if self.family == Family::Scottie { self.sync } else { 0.0 }
    }

    /// Durata dell'immagine (senza intestazione).
    pub fn image_seconds(&self) -> f64 {
        self.lead_sync() + self.group_seconds() * f64::from(self.groups())
    }

    /// Durata totale di una trasmissione, intestazione compresa.
    pub fn total_seconds(&self) -> f64 {
        HEADER_SECONDS + self.image_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_times_match_the_qsstv_table() {
        // imageTime di sstvparam.cpp di QSSTV
        let qsstv = [
            (SstvMode::Martin1, 114.297),
            (SstvMode::Martin2, 58.064),
            (SstvMode::Scottie1, 109.6325),
            (SstvMode::Scottie2, 71.0945),
            (SstvMode::ScottieDx, 268.8938),
            (SstvMode::Robot36, 36.002),
            (SstvMode::Robot72, 72.005),
            (SstvMode::Pd50, 49.6877),
            (SstvMode::Pd90, 89.995),
            (SstvMode::Pd120, 126.1115),
            (SstvMode::Pd160, 160.8942),
            (SstvMode::Pd180, 187.0645),
            (SstvMode::Pd240, 248.017),
            (SstvMode::Pd290, 288.702),
            (SstvMode::Sc2_180, 182.0385),
        ];
        for (mode, seconds) in qsstv {
            let spec = mode.spec();
            let ours = spec.group_seconds() * f64::from(spec.groups());
            assert!((ours - seconds).abs() / seconds < 0.002, "{}: {ours} contro {seconds}", spec.name);
        }
    }

    #[test]
    fn ids_and_vis_codes_are_unique() {
        for (i, a) in MODES.iter().enumerate() {
            for b in &MODES[i + 1..] {
                assert_ne!(a.vis, b.vis);
                assert_ne!(a.id, b.id);
            }
            assert_eq!(SstvMode::from_id(a.id), Some(a.mode));
            assert_eq!(SstvMode::from_vis(a.vis), Some(a.mode));
        }
    }
}
