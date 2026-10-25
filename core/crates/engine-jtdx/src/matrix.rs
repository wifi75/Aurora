//! Scheda DXCC: matrice paese x banda x modo dei QSO gia' fatti e le sei etichette NEW ONE / NEW BAND / NEW MODE / NEW GRID / GRID BAND /
//! GRID MODE di una stazione. Stessi insiemi di `WorkedLog` che colorano le righe delle decodifiche: la logica e' una sola
//! (vedi `docs/scheda-dxcc.md`).

use std::collections::{BTreeMap, HashMap, HashSet};

use dxcc::DxccDb;
use serde::Serialize;

use crate::worked::WorkedLog;

/// Il modo come lo contano la scheda e le etichette. FT8 e FT4 sono modi distinti (come in JTDX, dove si confronta il testo del modo);
/// `Dig` raccoglie tutti gli altri modi digitali; CW e fonia contano per il paese e la banda ma non hanno righe nella scheda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModeClass {
    Ft8,
    Ft4,
    Dig,
    Cw,
    Ph,
}

impl ModeClass {
    /// La classe di un modo ADIF (`MODE` e, per `MFSK`, il `SUBMODE`); `None` se il modo e' vuoto.
    pub fn from_adif(mode: &str, submode: &str) -> Option<ModeClass> {
        let mut m = mode.trim().to_uppercase();
        if m == "MFSK" && !submode.trim().is_empty() {
            m = submode.trim().to_uppercase();
        }
        match m.as_str() {
            "" => None,
            "FT8" => Some(ModeClass::Ft8),
            "FT4" => Some(ModeClass::Ft4),
            "CW" => Some(ModeClass::Cw),
            "SSB" | "USB" | "LSB" | "AM" | "FM" | "DSB" | "PHONE" | "PH" | "DIGITALVOICE" | "C4FM" | "DSTAR" | "DMR" | "FREEDV" => Some(ModeClass::Ph),
            _ => Some(ModeClass::Dig),
        }
    }

    /// Il nome nelle righe della scheda; `None` per CW e fonia, che non hanno riga.
    pub fn row(self) -> Option<&'static str> {
        match self {
            ModeClass::Ft8 => Some("FT8"),
            ModeClass::Ft4 => Some("FT4"),
            ModeClass::Dig => Some("DIG"),
            ModeClass::Cw | ModeClass::Ph => None,
        }
    }
}

/// Le righe della scheda, in ordine; `ALL` (tutti i modi, anche CW e fonia) serve alla tabella dei paesi.
pub const MATRIX_ROWS: [&str; 3] = ["FT8", "FT4", "DIG"];
/// Le colonne della scheda: HF, 6 e 4 metri; `U` raccoglie tutto cio' che e' piu' corto di 4 metri (2 m e oltre: VHF, UHF, SHF).
pub const MATRIX_BANDS: [&str; 13] = ["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "4m", "U"];

/// La colonna della scheda per una banda ADIF (`20m`, `70cm`...); `None` per le bande sotto i 160 m (2200 m, 630 m) e per quelle sconosciute.
pub fn matrix_column(band: &str) -> Option<&'static str> {
    let band = band.trim().to_lowercase();
    if let Some(col) = MATRIX_BANDS.iter().find(|c| **c == band) {
        return Some(col);
    }
    let (digits, unit) = band.split_at(band.find(|c: char| c.is_ascii_alphabetic())?);
    let value: f64 = digits.parse().ok()?;
    let metres = match unit {
        "m" => value,
        "cm" => value / 100.0,
        "mm" => value / 1000.0,
        _ => return None,
    };
    (metres > 0.0 && metres <= 2.0).then_some("U")
}

/// QSO di una casella (paese, banda, modo) e quanti hanno una conferma nota.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cell {
    pub qsos: u32,
    pub confirmed: u32,
    /// QSO di cui si sa che non sono confermati (il log ha i campi delle conferme e nessuno e' a «Y»).
    pub unconfirmed: u32,
}

/// Come si presenta una casella: verde `worked` o arancio `unconfirmed`; senza QSO la casella non c'e' (grigio).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CellState {
    /// Lavorato (e confermato, o il log non dice nulla sulle conferme).
    Worked,
    /// Lavorato ma tutti i QSO sono segnati come non confermati.
    Unconfirmed,
}

impl Cell {
    pub(crate) fn add(&mut self, confirmed: Option<bool>) {
        self.qsos += 1;
        match confirmed {
            Some(true) => self.confirmed += 1,
            Some(false) => self.unconfirmed += 1,
            None => {}
        }
    }

    fn merge(&mut self, other: &Cell) {
        self.qsos += other.qsos;
        self.confirmed += other.confirmed;
        self.unconfirmed += other.unconfirmed;
    }

    /// Verde se almeno un QSO e' confermato o senza informazione sulle conferme; arancio se tutti sono noti come non confermati.
    pub fn state(&self) -> Option<CellState> {
        if self.qsos == 0 {
            None
        } else if self.confirmed > 0 || self.unconfirmed < self.qsos {
            Some(CellState::Worked)
        } else {
            Some(CellState::Unconfirmed)
        }
    }
}

/// Le sei etichette di una stazione rispetto al log. Ognuna e' un si/no indipendente: un paese nuovo e' nuovo anche su ogni banda e su ogni
/// modo, quindi NEW ONE accende anche NEW BAND e NEW MODE (come JTDX: `WorkedBefore` falso azzera anche `WorkedBeforeBandMode`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Labels {
    /// Paese mai collegato, su nessuna banda e in nessun modo.
    pub new_one: bool,
    /// Paese mai collegato su questa banda (in nessun modo).
    pub new_band: bool,
    /// Paese mai collegato in questo modo (su nessuna banda).
    pub new_mode: bool,
    /// Locatore (4 caratteri) mai collegato.
    pub new_grid: bool,
    /// Locatore mai collegato su questa banda.
    pub grid_band: bool,
    /// Locatore mai collegato in questo modo.
    pub grid_mode: bool,
}

/// Il paese lavorato per banda e modo: `rows` ha FT8, FT4, DIG e `ALL`, ognuna con le colonne `MATRIX_BANDS` in cui c'e' almeno un QSO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixCountry {
    pub id: usize,
    pub name: String,
    pub prefix: String,
    pub continent: String,
    pub cells: BTreeMap<String, BTreeMap<String, CellState>>,
}

/// Quanti paesi sono stati lavorati in una riga e colonna: `state` e' `worked` se tutti sono confermati o senza informazione, `unconfirmed`
/// se almeno uno e' noto come non confermato (confermato solo in parte).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TotalCell {
    pub countries: usize,
    pub unconfirmed: usize,
    pub state: CellState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DxccMatrix {
    pub bands: Vec<String>,
    pub rows: Vec<String>,
    /// Paesi lavorati in totale (qualunque banda e modo).
    pub worked: usize,
    /// Paesi di `cty.dat`.
    pub total: usize,
    /// QSO letti dalle sorgenti, senza doppioni.
    pub qsos: usize,
    /// Per ogni riga (FT8, FT4, DIG, ALL): quanti paesi lavorati su qualunque banda.
    pub row_worked: BTreeMap<String, usize>,
    /// Per ogni riga e colonna: quanti paesi.
    pub totals: BTreeMap<String, BTreeMap<String, TotalCell>>,
    pub countries: Vec<MatrixCountry>,
}

/// Una stazione e quello che il log dice di lei: le etichette e la riga dei suoi paesi per banda e modo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Subject {
    pub call: String,
    pub country: Option<String>,
    pub prefix: Option<String>,
    pub continent: Option<String>,
    pub grid: Option<String>,
    pub band: Option<String>,
    pub mode: Option<String>,
    pub labels: Labels,
    pub cells: BTreeMap<String, BTreeMap<String, CellState>>,
}

fn grid4(grid: &str) -> Option<String> {
    let g: String = grid.trim().chars().take(4).collect::<String>().to_uppercase();
    (g.chars().count() == 4).then_some(g)
}

impl WorkedLog {
    /// Le sei etichette per una stazione. Senza paese noto o senza banda (modo) le etichette di paese (banda, modo) restano spente, come in
    /// JTDX: nessun paese = niente di nuovo; il locatore conta solo se ha almeno 4 caratteri.
    pub fn labels(&self, call: &str, grid: Option<&str>, band: Option<&str>, mode: Option<&str>, db: &DxccDb) -> Labels {
        let band = band.map(|b| b.trim().to_lowercase()).filter(|b| !b.is_empty());
        let class = mode.and_then(|m| ModeClass::from_adif(m, ""));
        let mut out = Labels::default();
        if let Some(found) = db.lookup(&call.trim().to_uppercase()) {
            out.new_one = !self.entities.contains(&found.entity);
            out.new_band = out.new_one || band.as_ref().is_some_and(|b| !self.entity_bands.contains(&(found.entity, b.clone())));
            out.new_mode = out.new_one || class.is_some_and(|c| !self.entity_modes.contains(&(found.entity, c)));
        }
        if let Some(g) = grid.and_then(grid4) {
            out.new_grid = !self.grids.contains(&g);
            out.grid_band = out.new_grid || band.as_ref().is_some_and(|b| !self.grid_bands.contains(&(g.clone(), b.clone())));
            out.grid_mode = out.new_grid || class.is_some_and(|c| !self.grid_modes.contains(&(g.clone(), c)));
        }
        out
    }

    /// Le caselle fuse per riga (FT8, FT4, DIG, ALL) e colonna della scheda, una per paese.
    fn merged_cells(&self) -> HashMap<(usize, &'static str, &'static str), Cell> {
        let mut merged: HashMap<(usize, &'static str, &'static str), Cell> = HashMap::new();
        for ((entity, band, class), cell) in &self.cells {
            if let (Some(col), Some(row)) = (matrix_column(band), class.row()) {
                merged.entry((*entity, row, col)).or_default().merge(cell);
            }
        }
        for ((entity, band), cell) in &self.any_cells {
            if let Some(col) = matrix_column(band) {
                merged.entry((*entity, "ALL", col)).or_default().merge(cell);
            }
        }
        merged
    }

    /// La matrice paese x banda x modo con i contatori.
    pub fn matrix(&self, db: &DxccDb) -> DxccMatrix {
        let merged = self.merged_cells();
        let mut per_country: HashMap<usize, BTreeMap<String, BTreeMap<String, CellState>>> = HashMap::new();
        let mut totals: BTreeMap<String, BTreeMap<String, TotalCell>> = BTreeMap::new();
        let mut row_entities: HashMap<&str, HashSet<usize>> = HashMap::new();
        for ((entity, row, col), cell) in &merged {
            let Some(state) = cell.state() else { continue };
            per_country.entry(*entity).or_default().entry((*row).to_string()).or_default().insert((*col).to_string(), state);
            let total = totals.entry((*row).to_string()).or_default().entry((*col).to_string()).or_insert(TotalCell { countries: 0, unconfirmed: 0, state: CellState::Worked });
            total.countries += 1;
            if state == CellState::Unconfirmed {
                total.unconfirmed += 1;
                total.state = CellState::Unconfirmed;
            }
            row_entities.entry(row).or_default().insert(*entity);
        }
        // anche un paese lavorato solo su una banda fuori scheda (630 m) o con un modo non noto conta nei totali generali
        let mut countries: Vec<MatrixCountry> = self
            .entities
            .iter()
            .map(|id| {
                let e = db.entity(*id);
                MatrixCountry { id: *id, name: e.name.clone(), prefix: e.prefix.clone(), continent: e.continent.clone(), cells: per_country.remove(id).unwrap_or_default() }
            })
            .collect();
        countries.sort_by(|a, b| a.name.cmp(&b.name));
        let mut row_worked: BTreeMap<String, usize> = MATRIX_ROWS.iter().map(|r| ((*r).to_string(), 0)).collect();
        row_worked.insert("ALL".to_string(), self.entities.len());
        for (row, set) in row_entities {
            if row != "ALL" {
                row_worked.insert(row.to_string(), set.len());
            }
        }
        DxccMatrix {
            bands: MATRIX_BANDS.iter().map(|b| (*b).to_string()).collect(),
            rows: MATRIX_ROWS.iter().map(|r| (*r).to_string()).collect(),
            worked: self.entities.len(),
            total: db.len(),
            qsos: self.len(),
            row_worked,
            totals,
            countries,
        }
    }

    /// Cio' che il log dice di una stazione: etichette e caselle del suo paese. `None` se manca il nominativo.
    pub fn subject(&self, call: &str, grid: Option<&str>, band: Option<&str>, mode: Option<&str>, db: &DxccDb) -> Option<Subject> {
        let call = call.trim().to_uppercase();
        if call.is_empty() {
            return None;
        }
        let found = db.lookup(&call);
        let mut cells: BTreeMap<String, BTreeMap<String, CellState>> = BTreeMap::new();
        if let Some(found) = found {
            for ((entity, row, col), cell) in self.merged_cells() {
                if entity == found.entity
                    && let Some(state) = cell.state()
                {
                    cells.entry(row.to_string()).or_default().insert(col.to_string(), state);
                }
            }
        }
        let entity = found.map(|f| db.entity(f.entity));
        Some(Subject {
            labels: self.labels(&call, grid, band, mode, db),
            call,
            country: entity.map(|e| e.name.clone()),
            prefix: entity.map(|e| e.prefix.clone()),
            continent: entity.map(|e| e.continent.clone()),
            grid: grid.and_then(grid4),
            band: band.map(|b| b.trim().to_lowercase()).filter(|b| !b.is_empty()),
            mode: mode.map(|m| m.trim().to_uppercase()).filter(|m| !m.is_empty()),
            cells,
        })
    }
}

#[cfg(test)]
mod tests {
    use aurora_core::adif::AdifQso;

    use super::*;

    const CTY: &str = "\
Atlantis:                 14:  27:  EU:   43.00:    -7.00:    -1.0:  AT:
    AT,AU;
Lemuria:                  05:  08:  NA:   10.00:    60.00:     5.0:  LM:
    LM,LMA,LMB;
Utopia:                   25:  45:  AS:   35.00:   -139.0:    -9.0:  UT:
    UT;
";

    fn db() -> DxccDb {
        DxccDb::parse(CTY).unwrap()
    }

    fn qso(call: &str, grid: &str, band: &str, mode: &str, confirmed: Option<bool>) -> AdifQso {
        AdifQso { call: call.into(), grid: grid.into(), band: band.into(), mode: mode.into(), confirmed, ..AdifQso::default() }
    }

    /// Log di prova: Atlantis in FT8 su 20m e in FT4 su 40m (grid FN42), Lemuria in CW su 20m e in FT8 su 2m, Utopia in RTTY su 17m non confermato.
    fn sample(db: &DxccDb) -> WorkedLog {
        let mut log = WorkedLog::new();
        for q in [
            qso("AT3AB", "FN42", "20m", "FT8", None),
            qso("AT4CD", "FN42", "40m", "FT4", None),
            qso("LM1XY", "KK00", "20m", "CW", None),
            qso("LM2XY", "KK01", "2m", "FT8", None),
            qso("UT1AA", "PM95", "17m", "RTTY", Some(false)),
        ] {
            assert!(log.add_adif_qso(&q, db));
        }
        log
    }

    #[test]
    fn mode_classes_and_matrix_columns() {
        assert_eq!(ModeClass::from_adif("ft8", ""), Some(ModeClass::Ft8));
        assert_eq!(ModeClass::from_adif("MFSK", "FT4"), Some(ModeClass::Ft4));
        assert_eq!(ModeClass::from_adif("MFSK", "MFSK16"), Some(ModeClass::Dig));
        assert_eq!(ModeClass::from_adif("JT65", ""), Some(ModeClass::Dig));
        assert_eq!(ModeClass::from_adif("USB", ""), Some(ModeClass::Ph));
        assert_eq!(ModeClass::from_adif("", ""), None);
        assert_eq!(matrix_column("20m"), Some("20m"));
        assert_eq!(matrix_column("4m"), Some("4m"));
        assert_eq!(matrix_column("2m"), Some("U"));
        assert_eq!(matrix_column("70cm"), Some("U"));
        assert_eq!(matrix_column("1.25m"), Some("U"));
        assert_eq!(matrix_column("6mm"), Some("U"));
        assert_eq!(matrix_column("630m"), None);
        assert_eq!(matrix_column("boh"), None);
    }

    #[test]
    fn a_never_worked_country_lights_one_band_and_mode() {
        let db = db();
        let log = WorkedLog::new();
        let l = log.labels("AT3AB", Some("FN42"), Some("20m"), Some("FT8"), &db);
        assert_eq!(l, Labels { new_one: true, new_band: true, new_mode: true, new_grid: true, grid_band: true, grid_mode: true });
    }

    #[test]
    fn band_and_mode_are_separate_questions() {
        let db = db();
        let log = sample(&db);
        // Atlantis: FT8 su 20m e FT4 su 40m. FT8 su 40m: paese, banda e modo gia' fatti (ma non insieme) -> niente di nuovo
        let l = log.labels("AT9ZZ", None, Some("40m"), Some("FT8"), &db);
        assert_eq!((l.new_one, l.new_band, l.new_mode), (false, false, false));
        // 15m: banda nuova
        let l = log.labels("AT9ZZ", None, Some("15m"), Some("FT8"), &db);
        assert_eq!((l.new_one, l.new_band, l.new_mode), (false, true, false));
        // Lemuria e' stata fatta solo in CW e in FT8 su 2m: FT4 su 20m e' modo nuovo, banda no
        let l = log.labels("LM9ZZ", None, Some("20m"), Some("FT4"), &db);
        assert_eq!((l.new_one, l.new_band, l.new_mode), (false, false, true));
        // il CW su 20m non rende FT8 su 20m un modo gia' fatto per... ma FT8 c'e' su 2m: modo non nuovo
        let l = log.labels("LM9ZZ", None, Some("20m"), Some("FT8"), &db);
        assert_eq!((l.new_band, l.new_mode), (false, false));
        // Utopia: solo RTTY (DIG): FT8 e' un modo nuovo
        let l = log.labels("UT9ZZ", None, Some("17m"), Some("FT8"), &db);
        assert_eq!((l.new_one, l.new_band, l.new_mode), (false, false, true));
        let l = log.labels("UT9ZZ", None, Some("17m"), Some("JT65"), &db);
        assert!(!l.new_mode, "JT65 e RTTY sono entrambi DIG");
    }

    #[test]
    fn grid_labels_follow_the_same_rules_on_four_characters() {
        let db = db();
        let log = sample(&db);
        let l = log.labels("QQ9QQ", Some("FN42ab"), Some("20m"), Some("FT8"), &db);
        assert_eq!((l.new_grid, l.grid_band, l.grid_mode), (false, false, false), "FN42 e' stato fatto su 20m in FT8");
        let l = log.labels("QQ9QQ", Some("fn42"), Some("15m"), Some("FT4"), &db);
        assert_eq!((l.new_grid, l.grid_band, l.grid_mode), (false, true, false), "FN42 c'e' in FT4, ma non su 15m");
        let l = log.labels("QQ9QQ", Some("FN42"), Some("20m"), Some("JT65"), &db);
        assert_eq!((l.new_grid, l.grid_band, l.grid_mode), (false, false, true));
        let l = log.labels("QQ9QQ", Some("JJ00"), Some("20m"), Some("FT8"), &db);
        assert!(l.new_grid && l.grid_band && l.grid_mode);
        let l = log.labels("QQ9QQ", Some("FN"), Some("20m"), Some("FT8"), &db);
        assert_eq!(l, Labels::default(), "senza paese ne' locatore valido non c'e' niente di nuovo");
    }

    #[test]
    fn unknown_band_or_mode_never_lights_the_variants() {
        let db = db();
        let log = sample(&db);
        let l = log.labels("AT9ZZ", Some("FN42"), None, None, &db);
        assert_eq!(l, Labels::default());
        assert_eq!(log.labels("AT9ZZ", None, Some("20m"), Some(""), &db), Labels::default());
    }

    #[test]
    fn the_matrix_counts_countries_per_row_and_column() {
        let db = db();
        let m = sample(&db).matrix(&db);
        assert_eq!((m.worked, m.total, m.qsos), (3, 3, 5));
        assert_eq!(m.totals["FT8"]["20m"].countries, 1);
        assert_eq!(m.totals["FT8"]["U"].countries, 1, "2m finisce nella colonna U");
        assert_eq!(m.totals["FT4"]["40m"].countries, 1);
        assert_eq!(m.totals["DIG"]["17m"].countries, 1);
        assert_eq!(m.totals["DIG"]["17m"].state, CellState::Unconfirmed);
        assert_eq!(m.totals["ALL"]["20m"].countries, 2, "Atlantis in FT8 e Lemuria in CW");
        assert!(!m.totals["FT8"].contains_key("40m"));
        assert_eq!((m.row_worked["FT8"], m.row_worked["FT4"], m.row_worked["DIG"], m.row_worked["ALL"]), (2, 1, 1, 3));
        let names: Vec<&str> = m.countries.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Atlantis", "Lemuria", "Utopia"]);
        assert_eq!(m.countries[0].cells["FT8"]["20m"], CellState::Worked);
        assert!(!m.countries[1].cells.contains_key("FT4"));
        assert_eq!(m.countries[2].cells["DIG"]["17m"], CellState::Unconfirmed);
    }

    #[test]
    fn a_qso_without_mode_or_on_a_band_out_of_the_card_counts_only_for_the_country() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_qso("AT3AB", "", "630m", &db);
        log.add_qso("LM1XY", "", "20m", &db);
        let m = log.matrix(&db);
        assert_eq!(m.worked, 2);
        assert!(m.totals.is_empty() || m.totals.get("ALL").is_some_and(|r| r.len() == 1), "solo il 20m ha una colonna");
        assert!(log.labels("AT9ZZ", None, Some("20m"), Some("FT8"), &db).new_mode, "senza modo noto il paese non ha mai fatto FT8");
        assert!(!log.labels("AT9ZZ", None, Some("630m"), Some("FT8"), &db).new_band);
    }

    #[test]
    fn a_confirmed_qso_turns_the_cell_green_and_partial_totals_stay_orange() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_adif_qso(&qso("AT3AB", "", "20m", "FT8", Some(false)), &db);
        log.add_adif_qso(&qso("LM1XY", "", "20m", "FT8", Some(false)), &db);
        assert_eq!(log.matrix(&db).totals["FT8"]["20m"].state, CellState::Unconfirmed);
        log.add_adif_qso(&qso("AT4CD", "", "20m", "FT8", Some(true)), &db);
        let m = log.matrix(&db);
        assert_eq!(m.countries[0].cells["FT8"]["20m"], CellState::Worked, "basta una conferma per il paese");
        let t = &m.totals["FT8"]["20m"];
        assert_eq!((t.countries, t.unconfirmed, t.state), (2, 1, CellState::Unconfirmed), "Lemuria non e' confermato: solo in parte");
    }

    #[test]
    fn the_same_qso_from_two_logs_counts_once() {
        let db = db();
        let mut log = WorkedLog::new();
        let mut q = qso("AT3AB", "FN42", "20m", "FT8", None);
        q.date = "20261024".into();
        q.time_on = "164645".into();
        assert!(log.add_adif_qso(&q, &db));
        q.time_on = "164610".into();
        assert!(!log.add_adif_qso(&q, &db), "stesso minuto: e' lo stesso QSO");
        q.time_on = "170000".into();
        assert!(log.add_adif_qso(&q, &db));
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn the_subject_carries_labels_and_the_cells_of_its_country() {
        let db = db();
        let log = sample(&db);
        let s = log.subject("lm5zz", Some("KK00"), Some("20m"), Some("ft4"), &db).unwrap();
        assert_eq!((s.call.as_str(), s.country.as_deref(), s.mode.as_deref()), ("LM5ZZ", Some("Lemuria"), Some("FT4")));
        assert!(s.labels.new_mode && !s.labels.new_band && !s.labels.new_one);
        assert_eq!(s.cells["ALL"]["20m"], CellState::Worked);
        assert_eq!(s.cells["FT8"]["U"], CellState::Worked);
        assert!(log.subject("  ", None, None, None, &db).is_none());
        let unknown = log.subject("QQ9QQ", None, None, None, &db).unwrap();
        assert!(unknown.country.is_none() && unknown.cells.is_empty());
    }
}
