use std::collections::{HashMap, HashSet};

use dxcc::DxccDb;
use serde::Serialize;

/// Che cosa c'e' di nuovo in una stazione rispetto ai QSO gia' fatti, dal piu' raro al meno raro
/// (come l'ordine dei colori di JTDX).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Novelty {
    /// Paese mai collegato, su nessuna banda.
    NewDxcc,
    /// Paese gia' collegato, ma non su questa banda.
    NewDxccBand,
    /// Zona CQ mai collegata.
    NewZone,
    NewZoneBand,
    /// Zona ITU mai collegata.
    NewItu,
    NewItuBand,
    /// Locatore (4 caratteri) mai collegato.
    NewGrid,
    NewGridBand,
    /// Prefisso (WPX) mai collegato.
    NewPx,
    NewPxBand,
    /// Stazione mai collegata.
    NewCall,
    NewCallBand,
    /// Stessa stazione gia' collegata su questa banda.
    Worked,
}

/// QSO gia' fatti: servono per capire se una stazione e' una novita'.
#[derive(Default)]
pub struct WorkedLog {
    qsos: usize,
    /// (banda, nominativo) gia' collegati.
    calls: HashSet<(String, String)>,
    /// Nominativi collegati su almeno una banda.
    any_call: HashSet<String>,
    /// Paesi collegati su almeno una banda.
    entities: HashSet<usize>,
    /// (paese, banda) collegati.
    entity_bands: HashSet<(usize, String)>,
    zones: HashSet<u8>,
    zone_bands: HashSet<(u8, String)>,
    itu: HashSet<u8>,
    itu_bands: HashSet<(u8, String)>,
    /// Locatori di 4 caratteri collegati.
    grids: HashSet<String>,
    grid_bands: HashSet<(String, String)>,
    prefixes: HashSet<String>,
    prefix_bands: HashSet<(String, String)>,
    band_qsos: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BandStats {
    pub band: String,
    pub dxcc: usize,
    pub qsos: usize,
}

/// Avanzamento nei premi, calcolato dai QSO gia' fatti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stats {
    pub qsos: usize,
    pub dxcc_worked: usize,
    pub dxcc_total: usize,
    pub cq_zones_worked: usize,
    pub cq_zones_total: usize,
    pub itu_zones_worked: usize,
    pub itu_zones_total: usize,
    pub grids: usize,
    pub prefixes: usize,
    pub calls: usize,
    pub bands: Vec<BandStats>,
}

fn grid4(grid: &str) -> Option<String> {
    let g: String = grid.trim().chars().take(4).collect::<String>().to_uppercase();
    (g.chars().count() == 4).then_some(g)
}

/// Prefisso WPX: tutto fino all'ultima cifra del nominativo (senza suffissi come /P).
pub fn wpx_prefix(call: &str) -> Option<String> {
    let call = call.trim().to_uppercase();
    if call.starts_with('<') {
        return None;
    }
    let main = call
        .split('/')
        .filter(|p| !matches!(*p, "P" | "M" | "QRP" | "MM" | "AM" | "A" | "B" | "LH") && !(p.len() == 1 && p.chars().all(|c| c.is_ascii_digit())))
        .max_by_key(|p| p.len())?;
    let last_digit = main.rfind(|c: char| c.is_ascii_digit())?;
    Some(main[..=last_digit].to_string())
}

impl WorkedLog {
    pub fn new() -> Self {
        WorkedLog::default()
    }

    /// Legge un file ADIF (formato `<CAMPO:lunghezza>valore`, lunghezze in byte). I record senza
    /// nominativo o senza banda, o senza `<EOR>` finale, vengono ignorati.
    pub fn from_adif(text: &str, db: &DxccDb) -> Self {
        let mut log = WorkedLog::new();
        for record in parse_adif(text) {
            let (Some(call), Some(band)) = (record.get("CALL"), record.get("BAND")) else {
                continue;
            };
            log.add_qso(call, record.get("GRIDSQUARE").map_or("", String::as_str), band, db);
        }
        log
    }

    pub fn add_qso(&mut self, call: &str, grid: &str, band: &str, db: &DxccDb) {
        let call = call.trim().to_uppercase();
        let band = band.trim().to_lowercase();
        if call.is_empty() || band.is_empty() {
            return;
        }
        if let Some(found) = db.lookup(&call) {
            self.entities.insert(found.entity);
            self.entity_bands.insert((found.entity, band.clone()));
            self.zones.insert(found.cq_zone);
            self.zone_bands.insert((found.cq_zone, band.clone()));
            self.itu.insert(found.itu_zone);
            self.itu_bands.insert((found.itu_zone, band.clone()));
        }
        if let Some(g) = grid4(grid) {
            self.grid_bands.insert((g.clone(), band.clone()));
            self.grids.insert(g);
        }
        if let Some(px) = wpx_prefix(&call) {
            self.prefix_bands.insert((px.clone(), band.clone()));
            self.prefixes.insert(px);
        }
        self.any_call.insert(call.clone());
        *self.band_qsos.entry(band.clone()).or_insert(0) += 1;
        self.calls.insert((band, call));
        self.qsos += 1;
    }

    pub fn novelty(&self, call: &str, grid: Option<&str>, band: &str, db: &DxccDb) -> Novelty {
        let call = call.trim().to_uppercase();
        let band = band.trim().to_lowercase();
        if let Some(found) = db.lookup(&call) {
            if !self.entities.contains(&found.entity) {
                return Novelty::NewDxcc;
            }
            if !self.entity_bands.contains(&(found.entity, band.clone())) {
                return Novelty::NewDxccBand;
            }
            if !self.zones.contains(&found.cq_zone) {
                return Novelty::NewZone;
            }
            if !self.zone_bands.contains(&(found.cq_zone, band.clone())) {
                return Novelty::NewZoneBand;
            }
            if !self.itu.contains(&found.itu_zone) {
                return Novelty::NewItu;
            }
            if !self.itu_bands.contains(&(found.itu_zone, band.clone())) {
                return Novelty::NewItuBand;
            }
        }
        if let Some(g) = grid.and_then(grid4) {
            if !self.grids.contains(&g) {
                return Novelty::NewGrid;
            }
            if !self.grid_bands.contains(&(g, band.clone())) {
                return Novelty::NewGridBand;
            }
        }
        if let Some(px) = wpx_prefix(&call) {
            if !self.prefixes.contains(&px) {
                return Novelty::NewPx;
            }
            if !self.prefix_bands.contains(&(px, band.clone())) {
                return Novelty::NewPxBand;
            }
        }
        if !self.any_call.contains(&call) {
            return Novelty::NewCall;
        }
        if !self.calls.contains(&(band, call)) {
            return Novelty::NewCallBand;
        }
        Novelty::Worked
    }

    /// Avanzamento nei premi DXCC, WAZ, ITU, griglie e prefissi, con il dettaglio per banda.
    pub fn stats(&self, db: &DxccDb) -> Stats {
        let mut bands: Vec<BandStats> = self
            .band_qsos
            .iter()
            .map(|(band, qsos)| BandStats {
                band: band.clone(),
                dxcc: self.entity_bands.iter().filter(|(_, b)| b == band).count(),
                qsos: *qsos,
            })
            .collect();
        bands.sort_by_key(|b| band_order(&b.band));
        Stats {
            qsos: self.qsos,
            dxcc_worked: self.entities.len(),
            dxcc_total: db.len(),
            cq_zones_worked: self.zones.len(),
            cq_zones_total: 40,
            itu_zones_worked: self.itu.len(),
            itu_zones_total: 90,
            grids: self.grids.len(),
            prefixes: self.prefixes.len(),
            calls: self.any_call.len(),
            bands,
        }
    }

    /// Paesi di `cty.dat` mai collegati, in ordine alfabetico.
    pub fn missing_dxcc(&self, db: &DxccDb) -> Vec<String> {
        let mut names: Vec<String> = (0..db.len()).filter(|i| !self.entities.contains(i)).map(|i| db.entity(i).name.clone()).collect();
        names.sort();
        names
    }

    /// QSO completi letti o aggiunti.
    pub fn len(&self) -> usize {
        self.qsos
    }

    pub fn is_empty(&self) -> bool {
        self.qsos == 0
    }
}

/// Ordine delle bande dalla piu' bassa (altre bande in fondo, per nome).
fn band_order(band: &str) -> (usize, String) {
    const ORDER: [&str; 12] = ["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m"];
    (ORDER.iter().position(|b| *b == band).unwrap_or(ORDER.len()), band.to_string())
}

/// Record ADIF completi (terminati da `<EOR>`), con i nomi dei campi in maiuscolo.
/// Le lunghezze sono in byte, quindi si lavora sui byte e non sui caratteri.
fn parse_adif(text: &str) -> Vec<HashMap<String, String>> {
    let bytes = text.as_bytes();
    let mut records = Vec::new();
    let mut current: HashMap<String, String> = HashMap::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let Some(end) = bytes[i..].iter().position(|b| *b == b'>').map(|p| i + p) else {
            break;
        };
        let tag = String::from_utf8_lossy(&bytes[i + 1..end]).to_uppercase();
        i = end + 1;
        match tag.as_str() {
            "EOH" => current.clear(),
            "EOR" => records.push(std::mem::take(&mut current)),
            _ => {
                let mut parts = tag.split(':');
                let name = parts.next().unwrap_or_default().trim().to_string();
                let Some(len) = parts.next().and_then(|l| l.trim().parse::<usize>().ok()) else {
                    continue;
                };
                let Some(value) = bytes.get(i..i + len) else {
                    break;
                };
                current.insert(name, String::from_utf8_lossy(value).trim().to_string());
                i += len;
            }
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTY: &str = "\
Atlantis:                 14:  27:  EU:   43.00:    -7.00:    -1.0:  AT:
    AT,AU,AT5(15)[30];
Lemuria:                  05:  08:  NA:   10.00:    60.00:     5.0:  LM:
    LM,LMA,LMB;
";

    fn db() -> DxccDb {
        DxccDb::parse(CTY).unwrap()
    }

    const ADIF: &str = "Esportazione di prova\n<ADIF_VER:5>3.1.4 <EOH>\n\
<CALL:5>AT3AB<BAND:3>20m<GRIDSQUARE:6>FN42AB<MODE:3>FT8<RST_SENT:3:S>-10<COMMENT:6>caffè<EOR>\n\
<call:5>AT4CD<band:3>40m<gridsquare:4>GG55<EOR>\n\
<CALL:5>AT5EF<BAND:3>20M<EOR>\n";

    #[test]
    fn reads_adif_records_with_types_lowercase_tags_and_utf8_lengths() {
        let log = WorkedLog::from_adif(ADIF, &db());
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn novelty_follows_the_precedence() {
        let db = db();
        let log = WorkedLog::from_adif(ADIF, &db);
        // paese mai collegato
        assert_eq!(log.novelty("LM1XY", Some("KK00"), "20m", &db), Novelty::NewDxcc);
        // paese collegato su 20m e 40m, su 15m no
        assert_eq!(log.novelty("AT9ZZ", Some("FN42"), "15m", &db), Novelty::NewDxccBand);
        // zona CQ 14, ITU 27 e griglia FN42 gia' collegate su 20m; il prefisso AT9 no
        assert_eq!(log.novelty("AT9ZZ", Some("FN42"), "20m", &db), Novelty::NewPx);
        // griglia mai collegata
        assert_eq!(log.novelty("AT9ZZ", Some("KK00"), "20m", &db), Novelty::NewGrid);
        // stessa stazione, stessa banda
        assert_eq!(log.novelty("AT3AB", Some("FN42"), "20m", &db), Novelty::Worked);
        // nominativo scritto in minuscolo
        assert_eq!(log.novelty("at3ab", Some("fn42"), "20m", &db), Novelty::Worked);
    }

    #[test]
    fn a_new_cq_zone_is_reported_before_a_new_grid() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_qso("AT3AB", "FN42", "20m", &db); // zona 14
        // AT5 ha la zona CQ 15 (sovrascritta), mai collegata
        assert_eq!(log.novelty("AT5XX", Some("KK00"), "20m", &db), Novelty::NewZone);
    }

    #[test]
    fn grid_is_compared_on_four_characters() {
        let db = db();
        let log = WorkedLog::from_adif(ADIF, &db);
        assert_ne!(log.novelty("AT9ZZ", Some("FN42"), "20m", &db), Novelty::NewGrid);
        assert_ne!(log.novelty("AT9ZZ", Some("fn42xy"), "20m", &db), Novelty::NewGrid);
    }

    #[test]
    fn new_qso_updates_immediately() {
        let db = db();
        let mut log = WorkedLog::new();
        assert_eq!(log.novelty("LM1XY", Some("KK00"), "20m", &db), Novelty::NewDxcc);
        log.add_qso("LM1XY", "KK00", "20m", &db);
        assert_eq!(log.len(), 1);
        assert_eq!(log.novelty("LM1XY", Some("KK00"), "20m", &db), Novelty::Worked);
        assert_eq!(log.novelty("LM2ZZ", Some("KK00"), "20m", &db), Novelty::NewPx, "il paese non e' piu' nuovo, il prefisso LM2 si");
    }

    #[test]
    fn unknown_calls_are_never_dxcc_news() {
        let db = db();
        let log = WorkedLog::from_adif(ADIF, &db);
        assert_eq!(log.novelty("QQ9QQ", None, "20m", &db), Novelty::NewPx, "senza paese restano prefisso e stazione");
        assert_eq!(log.novelty("QQ9QQ", Some("KK00"), "20m", &db), Novelty::NewGrid);
    }

    #[test]
    fn the_wpx_prefix_is_everything_up_to_the_last_digit() {
        assert_eq!(wpx_prefix("IU3WJQ").as_deref(), Some("IU3"));
        assert_eq!(wpx_prefix("3DA0XX").as_deref(), Some("3DA0"));
        assert_eq!(wpx_prefix("kh6abc").as_deref(), Some("KH6"));
        assert_eq!(wpx_prefix("AT3AB/P").as_deref(), Some("AT3"));
        assert_eq!(wpx_prefix("<...>"), None);
        assert_eq!(wpx_prefix("ABC"), None);
    }

    #[test]
    fn every_level_of_novelty_follows_the_jtdx_order() {
        let db = db();
        // AT3AB: zona CQ 14 ITU 27, FN42, prefisso AT3, solo su 20m
        let log = WorkedLog::from_adif("<CALL:5>AT3AB<BAND:3>20m<GRIDSQUARE:4>FN42<EOR>
", &db);
        assert_eq!(log.novelty("AT3AB", Some("FN42"), "20m", &db), Novelty::Worked);
        assert_eq!(log.novelty("AT3AB", Some("FN42"), "15m", &db), Novelty::NewDxccBand, "il paese e' nuovo sulla banda: vince");
        // stesso paese, banda gia' fatta: dal piu' raro al meno raro
        assert_eq!(log.novelty("AT3CD", Some("FN42"), "20m", &db), Novelty::NewCall, "stessi zona, griglia e prefisso, stazione nuova");
        assert_eq!(log.novelty("AT9ZZ", Some("FN42"), "20m", &db), Novelty::NewPx);
        assert_eq!(log.novelty("AT3CD", Some("GG55"), "20m", &db), Novelty::NewGrid);
        assert_eq!(log.novelty("AT5XX", Some("FN42"), "20m", &db), Novelty::NewZone, "AT5 ha zona CQ 15");
    }

    #[test]
    fn band_variants_appear_when_the_item_was_worked_only_elsewhere() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_qso("AT3AB", "FN42", "20m", &db);
        log.add_qso("AT3CD", "FN43", "15m", &db); // il paese ha entrambe le bande
        // griglia FN42 e prefisso AT3 sono stati fatti solo su 20m
        assert_eq!(log.novelty("AT3EF", Some("FN42"), "15m", &db), Novelty::NewGridBand);
        assert_eq!(log.novelty("AT3EF", Some("FN43"), "20m", &db), Novelty::NewGridBand);
        // stazione fatta solo su 20m, vista ora su 15m, con griglia gia' fatta su 15m
        assert_eq!(log.novelty("AT3AB", Some("FN43"), "15m", &db), Novelty::NewCallBand);
    }

    #[test]
    fn stats_count_what_was_worked_per_award() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_qso("AT3AB", "FN42", "20m", &db);
        log.add_qso("AT5EF", "FN43", "15m", &db);
        log.add_qso("LM1XY", "KK00", "20m", &db);
        log.add_qso("LM1XY", "KK00", "20m", &db);
        let s = log.stats(&db);
        assert_eq!(s.qsos, 4);
        assert_eq!((s.dxcc_worked, s.dxcc_total), (2, 2));
        assert_eq!(s.cq_zones_worked, 3, "zone 14, 15 e 5");
        assert_eq!(s.cq_zones_total, 40);
        assert_eq!(s.itu_zones_total, 90);
        assert_eq!(s.grids, 3);
        assert_eq!(s.prefixes, 3, "AT3, AT5, LM1");
        assert_eq!(s.calls, 3, "LM1XY conta una volta sola");
        let b20 = s.bands.iter().find(|b| b.band == "20m").unwrap();
        assert_eq!((b20.dxcc, b20.qsos), (2, 3));
        let b15 = s.bands.iter().find(|b| b.band == "15m").unwrap();
        assert_eq!((b15.dxcc, b15.qsos), (1, 1));
    }

    #[test]
    fn missing_dxcc_lists_the_countries_never_worked() {
        let db = db();
        let mut log = WorkedLog::new();
        assert_eq!(log.missing_dxcc(&db), vec!["Atlantis".to_string(), "Lemuria".to_string()]);
        log.add_qso("LM1XY", "KK00", "20m", &db);
        assert_eq!(log.missing_dxcc(&db), vec!["Atlantis".to_string()]);
    }

    #[test]
    fn incomplete_records_are_ignored() {
        let text = "<CALL:5>AT3AB<EOR>\n<BAND:3>20m<EOR>\n<CALL:5>AT4CD<BAND:3>40m<EOR>\n";
        assert_eq!(WorkedLog::from_adif(text, &db()).len(), 1);
    }

    #[test]
    fn file_without_final_eor_or_with_garbage_is_ok() {
        let db = db();
        assert_eq!(WorkedLog::from_adif("<CALL:5>AT3AB<BAND:3>20m", &db).len(), 0);
        assert_eq!(WorkedLog::from_adif("", &db).len(), 0);
        assert_eq!(WorkedLog::from_adif("<<<:::>>> <CALL:99>x<EOR><BAND:abc>", &db).len(), 0);
        assert_eq!(WorkedLog::from_adif("<CALL:5>AT3", &db).len(), 0);
    }
}
