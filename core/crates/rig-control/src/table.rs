//! Frequenze di lavoro per banda e modo, come la scheda Frequenze di JTDX.

use serde::{Deserialize, Serialize};

use crate::bands::canonical_band;

/// Regione IARU scelta nelle impostazioni Generali (`Region` di JTDX): `ALL` mostra tutte le righe, `R1`, `R2` e `R3` solo quelle valide ovunque
/// o nella regione scelta (`FrequencyList_v2::filterAcceptsRow`, `REA 0x654aa0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Region {
    #[default]
    #[serde(rename = "ALL")]
    All,
    #[serde(rename = "R1")]
    R1,
    #[serde(rename = "R2")]
    R2,
    #[serde(rename = "R3")]
    R3,
}

impl Region {
    /// Il nome nel `JTDX.ini` e nel file delle impostazioni (`ALL`, `R1`, `R2`, `R3`).
    pub fn as_str(self) -> &'static str {
        match self {
            Region::All => "ALL",
            Region::R1 => "R1",
            Region::R2 => "R2",
            Region::R3 => "R3",
        }
    }

    /// Da un nome (`ALL`, `R1`...); un nome sconosciuto vale `ALL`, come `IARURegions::value` di JTDX.
    pub fn parse(name: &str) -> Region {
        match name.trim().to_uppercase().as_str() {
            "R1" => Region::R1,
            "R2" => Region::R2,
            "R3" => Region::R3,
            _ => Region::All,
        }
    }

    /// Una riga della tabella con questa regione si vede quando la regione scelta e' `self`: sempre con `ALL`, altrimenti se la riga e' valida
    /// per tutte le regioni o per quella scelta.
    pub fn sees(self, row: Region) -> bool {
        self == Region::All || row == Region::All || row == self
    }
}

fn all_regions() -> Region {
    Region::All
}

fn yes() -> bool {
    true
}

/// Una riga della tabella delle frequenze: banda, modo, frequenza di dial, regione IARU e se e' una frequenza «predefinita» (quella che la banda
/// sceglie da sola; le altre sono alternative, come la colonna di JTDX).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub band: String,
    pub mode: String,
    pub hz: u64,
    #[serde(default = "all_regions")]
    pub region: Region,
    #[serde(default = "yes")]
    pub preferred: bool,
}

impl Entry {
    /// Una riga valida in tutte le regioni e predefinita.
    pub fn new(band: &str, mode: &str, hz: u64) -> Entry {
        Entry { band: band.into(), mode: mode.into(), hz, region: Region::All, preferred: true }
    }
}

/// Limiti (Hz) entro cui deve cadere la frequenza di ogni banda.
fn band_limits(band: &str) -> Option<(u64, u64)> {
    crate::bands::BANDS.iter().find(|(name, _, _)| *name == band).map(|(_, lo, hi)| (*lo, *hi))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyTable(Vec<Entry>);

/// La tabella iniziale di JTDX 2.2.159, solo le righe FT8 e FT4 delle bande che Aurora conosce: letta con REA da `jtdx.exe`
/// (`FrequencyList_v2::reset_to_defaults`, `REA 0x5a48a0`, che copia l'elenco `default_frequency_list` di 115 righe, a `REA 0x13eca00`, 24 byte per riga:
/// frequenza, modo, regione, «predefinita»). Mancano le righe di 8 m (40,680 MHz), 4 m (70 MHz) e 70 cm, bande che Aurora non ha ancora, e quelle di JT65, JT9, T10 e WSPR.
/// Colonne: banda, modo, frequenza (Hz), regione, predefinita.
const JTDX_ROWS: &[(&str, &str, u64, Region, bool)] = &[
    ("160m", "FT8", 1_810_000, Region::R1, true),
    ("160m", "FT8", 1_840_000, Region::All, true),
    ("160m", "FT8", 1_908_000, Region::R1, true),
    ("80m", "FT8", 3_567_000, Region::All, false),
    ("80m", "FT8", 3_573_000, Region::All, true),
    ("80m", "FT4", 3_568_000, Region::R3, true),
    ("80m", "FT4", 3_575_000, Region::All, true),
    ("80m", "FT8", 3_585_000, Region::All, false),
    ("60m", "FT8", 5_357_000, Region::All, true),
    // FT4 a 60 m: non e' nella tabella di JTDX, e' in quella di WSJT-X (FrequencyList.cpp, provvisoria)
    ("60m", "FT4", 5_357_000, Region::All, true),
    ("40m", "FT4", 7_047_500, Region::All, true),
    ("40m", "FT8", 7_056_000, Region::All, false),
    ("40m", "FT8", 7_071_000, Region::All, false),
    ("40m", "FT8", 7_074_000, Region::All, true),
    ("40m", "FT8", 7_080_000, Region::All, false),
    ("30m", "FT8", 10_131_000, Region::All, false),
    ("30m", "FT8", 10_133_000, Region::All, false),
    ("30m", "FT8", 10_136_000, Region::All, true),
    ("30m", "FT4", 10_140_000, Region::All, true),
    ("30m", "FT8", 10_143_000, Region::All, false),
    ("20m", "FT8", 14_071_000, Region::All, false),
    ("20m", "FT8", 14_074_000, Region::All, true),
    ("20m", "FT4", 14_080_000, Region::All, true),
    ("20m", "FT8", 14_090_000, Region::All, false),
    ("17m", "FT8", 18_095_000, Region::All, false),
    ("17m", "FT8", 18_100_000, Region::All, true),
    ("17m", "FT4", 18_104_000, Region::All, true),
    ("15m", "FT8", 21_074_000, Region::All, true),
    ("15m", "FT8", 21_091_000, Region::All, false),
    ("15m", "FT4", 21_140_000, Region::All, true),
    ("12m", "FT8", 24_911_000, Region::All, false),
    ("12m", "FT8", 24_915_000, Region::All, true),
    ("12m", "FT4", 24_919_000, Region::All, true),
    ("10m", "FT8", 28_074_000, Region::All, true),
    // 28,091 MHz: la frequenza DXpedition di WSJT-X (FrequencyList.cpp); JTDX ha 28,095
    ("10m", "FT8", 28_091_000, Region::All, false),
    ("10m", "FT8", 28_095_000, Region::All, false),
    ("10m", "FT4", 28_180_000, Region::All, true),
    ("6m", "FT8", 50_310_000, Region::All, false),
    ("6m", "FT8", 50_313_000, Region::All, true),
    ("6m", "FT4", 50_318_000, Region::All, true),
    ("6m", "FT8", 50_323_000, Region::All, true),
    ("2m", "FT4", 144_170_000, Region::All, true),
    ("2m", "FT8", 144_174_000, Region::All, true),
    // 70 cm (FT-991A, IC-9700 e simili): frequenze FT8/FT4 d'uso (richiesta SP4UBW, 2026-11-08)
    ("70cm", "FT8", 432_174_000, Region::All, true),
    ("70cm", "FT4", 432_170_000, Region::All, true),
];

/// Le righe di JT65 e JT9 della tabella iniziale di JTDX 2.2.159 per le bande che Aurora conosce: lette con REA da `jtdx.exe`
/// (`default_frequency_list`, `REA 0x13eca00`: 115 righe da 24 byte; modo 1 = JT65, 2 = JT9, 3 = T10, 4 = FT8, 5 = FT4, 6 = WSPR; regione 0 = tutte,
/// 1 = R1, 2 = R2, 3 = R3). Sono tutte predefinite. Mancano 4 m (70,102 e 70,104 MHz, R1) e le bande oltre i 2 m.
const JT_ROWS: &[(&str, &str, u64, Region, bool)] = &[
    ("160m", "JT65", 1_838_000, Region::All, true),
    ("80m", "JT65", 3_570_000, Region::All, true),
    ("60m", "JT65", 5_357_000, Region::All, true),
    ("40m", "JT65", 7_076_000, Region::All, true),
    ("30m", "JT65", 10_138_000, Region::All, true),
    ("20m", "JT65", 14_076_000, Region::All, true),
    ("17m", "JT65", 18_102_000, Region::All, true),
    ("15m", "JT65", 21_076_000, Region::All, true),
    ("12m", "JT65", 24_917_000, Region::All, true),
    ("10m", "JT65", 28_076_000, Region::All, true),
    ("6m", "JT65", 50_276_000, Region::R2, true),
    ("6m", "JT65", 50_276_000, Region::R3, true),
    ("6m", "JT65", 50_310_000, Region::All, true),
    ("2m", "JT65", 144_120_000, Region::All, true),
    ("160m", "JT9", 1_839_000, Region::All, true),
    ("80m", "JT9", 3_572_000, Region::All, true),
    ("60m", "JT9", 5_357_000, Region::All, true),
    ("40m", "JT9", 7_078_000, Region::All, true),
    ("30m", "JT9", 10_140_000, Region::All, true),
    ("20m", "JT9", 14_078_000, Region::All, true),
    ("17m", "JT9", 18_104_000, Region::All, true),
    ("15m", "JT9", 21_078_000, Region::All, true),
    ("12m", "JT9", 24_919_000, Region::All, true),
    ("10m", "JT9", 28_078_000, Region::All, true),
    ("6m", "JT9", 50_312_000, Region::All, true),
];

/// Le righe MSK144 dell'elenco di WSJT-X (`FrequencyList.cpp`); la' sono tutte non predefinite, qui sono le principali della loro regione.
const MSK144_ROWS: &[(&str, &str, u64, Region, bool)] = &[
    ("6m", "MSK144", 50_260_000, Region::R2, true),
    ("6m", "MSK144", 50_260_000, Region::R3, true),
    ("6m", "MSK144", 50_380_000, Region::R1, true),
    ("4m", "MSK144", 70_230_000, Region::R1, true),
    ("2m", "MSK144", 144_150_000, Region::R2, true),
    ("2m", "MSK144", 144_360_000, Region::R1, true),
    ("70cm", "MSK144", 432_360_000, Region::All, true),
];

/// Le bande da 2200 m, 630 m, 8 m, 4 m e da 1,25 m in su (UHF, microonde): tutte le righe di questi modi nell'elenco di WSJT-X (`FrequencyList.cpp`).
const EXTRA_BAND_ROWS: &[(&str, &str, u64, Region, bool)] = &[
    ("2200m", "JT9", 136_000, Region::All, false),
    ("630m", "JT9", 474_200, Region::All, false),
    ("4m", "JT65", 70_102_000, Region::R1, false),
    ("4m", "JT9", 70_104_000, Region::R1, false),
    ("4m", "FT8", 70_154_000, Region::R1, true),
    // 70,100 MHz: frequenza FT8 dei 4 m indicata da ft8.it (piano IARU), alternativa a quella di WSJT-X
    ("4m", "FT8", 70_100_000, Region::R1, false),
    ("1.25m", "JT65", 222_065_000, Region::R2, false),
    ("1.25m", "Q65", 222_065_000, Region::R2, false),
    ("1.25m", "FT8", 222_174_000, Region::R2, true),
    ("70cm", "JT65", 432_065_000, Region::All, false),
    ("70cm", "FT8", 432_174_000, Region::All, true),
    ("70cm", "Q65", 432_065_000, Region::All, false),
    ("33cm", "JT65", 902_065_000, Region::R2, false),
    ("33cm", "Q65", 902_065_000, Region::R2, false),
    ("23cm", "JT65", 1_296_065_000, Region::All, false),
    ("23cm", "Q65", 1_296_065_000, Region::All, false),
    ("23cm", "FT8", 1_296_174_000, Region::All, true),
    ("13cm", "JT4", 2_301_065_000, Region::All, false),
    ("13cm", "JT65", 2_301_065_000, Region::All, false),
    ("13cm", "Q65", 2_301_065_000, Region::All, false),
    ("13cm", "JT4", 2_304_065_000, Region::All, false),
    ("13cm", "JT65", 2_304_065_000, Region::All, false),
    ("13cm", "Q65", 2_304_065_000, Region::All, false),
    ("13cm", "JT4", 2_320_065_000, Region::All, false),
    ("13cm", "JT65", 2_320_065_000, Region::All, false),
    ("13cm", "Q65", 2_320_065_000, Region::All, false),
    ("9cm", "JT4", 3_400_065_000, Region::All, false),
    ("9cm", "JT65", 3_400_065_000, Region::All, false),
    ("9cm", "Q65", 3_400_065_000, Region::All, false),
    ("6cm", "JT4", 5_760_065_000, Region::All, false),
    ("6cm", "JT65", 5_760_065_000, Region::All, false),
    ("6cm", "Q65", 5_760_200_000, Region::All, false),
    ("3cm", "JT4", 10_368_200_000, Region::All, false),
    ("3cm", "Q65", 10_368_200_000, Region::All, false),
    ("1.25cm", "JT4", 24_048_200_000, Region::All, false),
    ("1.25cm", "Q65", 24_048_200_000, Region::All, false),
];

/// Le righe Q65 dell'elenco di WSJT-X (`FrequencyList.cpp`) sulle bande che Aurora conosce: 50,275 e 144,180 MHz, con quelle EME come alternative.
const Q65_ROWS: &[(&str, &str, u64, Region, bool)] = &[
    ("6m", "Q65", 50_211_000, Region::All, false),
    ("6m", "Q65", 50_275_000, Region::All, true),
    ("2m", "Q65", 144_116_000, Region::All, false),
    ("2m", "Q65", 144_180_000, Region::All, true),
];

/// Le 24 righe della tabella che Aurora usava prima di avere le regioni (una per banda e modo): chi le ha salvate senza cambiarle passa alla tabella di JTDX.
const LEGACY_ROWS: &[(&str, &str, u64)] = &[
    ("160m", "FT8", 1_840_000),
    ("160m", "FT4", 1_840_000),
    ("80m", "FT8", 3_573_000),
    ("80m", "FT4", 3_575_000),
    ("60m", "FT8", 5_357_000),
    ("40m", "FT8", 7_074_000),
    ("40m", "FT4", 7_047_500),
    ("30m", "FT8", 10_136_000),
    ("30m", "FT4", 10_140_000),
    ("20m", "FT8", 14_074_000),
    ("20m", "FT4", 14_080_000),
    ("17m", "FT8", 18_100_000),
    ("17m", "FT4", 18_104_000),
    ("15m", "FT8", 21_074_000),
    ("15m", "FT4", 21_140_000),
    ("12m", "FT8", 24_915_000),
    ("12m", "FT4", 24_919_000),
    ("10m", "FT8", 28_074_000),
    ("10m", "FT4", 28_180_000),
    ("6m", "FT8", 50_313_000),
    ("6m", "FT4", 50_318_000),
    ("2m", "FT8", 144_174_000),
    ("2m", "FT4", 144_170_000),
    ("70cm", "FT8", 432_174_000),
    ("70cm", "FT4", 432_170_000),
];

impl Default for FrequencyTable {
    fn default() -> Self {
        FrequencyTable(JTDX_ROWS.iter().chain(JT_ROWS.iter()).chain(Q65_ROWS.iter()).chain(EXTRA_BAND_ROWS.iter()).chain(MSK144_ROWS.iter()).map(|(band, mode, hz, region, preferred)| Entry { band: (*band).into(), mode: (*mode).into(), hz: *hz, region: *region, preferred: *preferred }).collect())
    }
}

fn valid_entries(entries: &[Entry]) -> bool {
    if entries.is_empty() {
        return false;
    }
    // la stessa frequenza non puo' stare due volte nella stessa banda, nello stesso modo e nella stessa regione; la stessa banda e lo stesso modo
    // possono avere piu' righe (frequenze alternative e righe di regioni diverse, anche con la stessa frequenza: 50,276 MHz JT65 in R2 e R3)
    let mut seen = std::collections::HashSet::new();
    entries.iter().all(|entry| {
        let Some(band) = canonical_band(&entry.band) else { return false };
        let mode = entry.mode.trim().to_uppercase();
        let in_band = band_limits(&band).is_some_and(|(lo, hi)| (lo..=hi).contains(&entry.hz));
        in_band && matches!(mode.as_str(), "FT8" | "FT4" | "JT65" | "JT9" | "Q65" | "JT4" | "MSK144") && seen.insert((band, mode, entry.hz, entry.region.as_str()))
    })
}

impl FrequencyTable {
    pub fn entries(&self) -> &[Entry] {
        &self.0
    }

    pub fn is_valid(&self) -> bool {
        valid_entries(&self.0)
    }

    /// La tabella salvata da una versione senza regioni (una riga per banda e modo, tutte uguali a quelle di allora) passa alla tabella di JTDX;
    /// qualunque altra tabella resta come l'ha fatta l'utente.
    pub fn upgraded(entries: Vec<Entry>) -> Vec<Entry> {
        let legacy = entries.len() == LEGACY_ROWS.len()
            && LEGACY_ROWS.iter().all(|(band, mode, hz)| entries.iter().any(|e| e.band == *band && e.mode == *mode && e.hz == *hz && e.region == Region::All && e.preferred));
        if legacy {
            return FrequencyTable::default().0;
        }
        // una tabella salvata prima di JT65 e JT9 non ha le loro righe: si aggiungono quelle di JTDX, senza toccare le righe dell'utente
        let mut entries = entries;
        if !entries.iter().any(|e| matches!(e.mode.to_uppercase().as_str(), "JT65" | "JT9")) {
            entries.extend(JT_ROWS.iter().map(|(band, mode, hz, region, preferred)| Entry { band: (*band).into(), mode: (*mode).into(), hz: *hz, region: *region, preferred: *preferred }));
        }
        // e per le bande aggiunte dopo (2200 m, 630 m, 8 m, 4 m, UHF e microonde)
        if !entries.iter().any(|e| EXTRA_BAND_ROWS.iter().any(|r| r.0 == e.band)) {
            entries.extend(EXTRA_BAND_ROWS.iter().map(|(band, mode, hz, region, preferred)| Entry { band: (*band).into(), mode: (*mode).into(), hz: *hz, region: *region, preferred: *preferred }));
        }
        // lo stesso per MSK144
        if !entries.iter().any(|e| e.mode.eq_ignore_ascii_case("MSK144")) {
            entries.extend(MSK144_ROWS.iter().map(|(band, mode, hz, region, preferred)| Entry { band: (*band).into(), mode: (*mode).into(), hz: *hz, region: *region, preferred: *preferred }));
        }
        // lo stesso per Q65
        if !entries.iter().any(|e| e.mode.eq_ignore_ascii_case("Q65")) {
            entries.extend(Q65_ROWS.iter().map(|(band, mode, hz, region, preferred)| Entry { band: (*band).into(), mode: (*mode).into(), hz: *hz, region: *region, preferred: *preferred }));
        }
        entries
    }

    /// La banda in cui cade una frequenza di dial, tra quelle della tabella.
    pub fn band_of(&self, hz: u64) -> Option<String> {
        self.0.iter().find(|x| band_limits(&x.band).is_some_and(|(lo, hi)| (lo..=hi).contains(&hz))).map(|x| x.band.clone())
    }

    /// Le righe che la regione fa vedere (l'elenco delle frequenze di JTDX dopo `FrequencyList_v2::filter`).
    pub fn visible(&self, region: Region) -> impl Iterator<Item = &Entry> {
        self.0.iter().filter(move |x| region.sees(x.region))
    }

    /// Frequenza di dial per banda e modo con tutte le regioni (`ALL`).
    pub fn frequency(&self, band: &str, mode: &str) -> Option<u64> {
        self.frequency_in(Region::All, band, mode)
    }

    /// Frequenza di dial per banda e modo con la regione scelta. Tra le righe visibili di quella banda e di quel modo vince, nell'ordine: una riga
    /// predefinita, una della regione scelta (con `ALL` una valida ovunque), la frequenza piu' bassa. Se nessuna e' predefinita vince la prima per frequenza.
    pub fn frequency_in(&self, region: Region, band: &str, mode: &str) -> Option<u64> {
        let band = canonical_band(band)?;
        let mode = mode.trim().to_uppercase();
        self.visible(region)
            .filter(|x| x.band == band && x.mode.to_uppercase() == mode)
            .min_by_key(|x| (!x.preferred, if region == Region::All { x.region != Region::All } else { x.region != region }, x.hz))
            .map(|x| x.hz)
    }

    /// La frequenza di lavoro da usare in un modo partendo dal dial di adesso: `MainWindow::switch_mode` di JTDX cerca con
    /// `FrequencyList_v2::best_working_frequency(dial)` (`REA 0x654ca0`) tra le righe visibili del modo, nella banda del dial: se il dial e' gia' una riga si
    /// resta li', altrimenti vince la riga predefinita piu' vicina; senza righe predefinite la piu' vicina.
    pub fn best_working(&self, region: Region, mode: &str, dial_hz: u64) -> Option<u64> {
        let band = self.band_of(dial_hz)?;
        let mode = mode.trim().to_uppercase();
        let rows: Vec<&Entry> = self.visible(region).filter(|x| x.band == band && x.mode.to_uppercase() == mode).collect();
        let distance = |x: &&Entry| x.hz.abs_diff(dial_hz);
        let nearest = rows.iter().min_by_key(|x| distance(x))?;
        if nearest.hz == dial_hz {
            return Some(dial_hz);
        }
        let preferred = rows.iter().filter(|x| x.preferred).min_by_key(|x| distance(x));
        Some(preferred.unwrap_or(nearest).hz)
    }

    /// Sostituisce la tabella se e' valida (bande e modi noti, frequenza nella banda, senza doppioni).
    pub fn set(&mut self, entries: Vec<Entry>) -> Result<(), &'static str> {
        if !valid_entries(&entries) {
            return Err("tabella delle frequenze non valida");
        }
        self.0 = entries
            .into_iter()
            .map(|x| Entry { band: canonical_band(&x.band).unwrap_or(x.band), mode: x.mode.trim().to_uppercase(), hz: x.hz, region: x.region, preferred: x.preferred })
            .collect();
        Ok(())
    }
}
