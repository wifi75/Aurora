use std::collections::{HashMap, HashSet};

use dxcc::DxccDb;

/// Che cosa c'e' di nuovo in una stazione rispetto ai QSO gia' fatti, in ordine di importanza.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Novelty {
    /// Paese mai collegato, su nessuna banda.
    NewDxcc,
    /// Paese gia' collegato, ma non su questa banda.
    NewDxccBand,
    /// Zona CQ mai collegata.
    NewZone,
    /// Locatore (4 caratteri) mai collegato.
    NewGrid,
    /// Stessa stazione gia' collegata su questa banda.
    Worked,
    Nothing,
}

/// QSO gia' fatti: servono per capire se una stazione e' una novita'.
#[derive(Default)]
pub struct WorkedLog {
    qsos: usize,
    /// (banda, nominativo) gia' collegati.
    calls: HashSet<(String, String)>,
    /// Paesi collegati su almeno una banda.
    entities: HashSet<usize>,
    /// (paese, banda) collegati.
    entity_bands: HashSet<(usize, String)>,
    /// Zone CQ collegate.
    zones: HashSet<u8>,
    /// Locatori di 4 caratteri collegati.
    grids: HashSet<String>,
}

fn grid4(grid: &str) -> Option<String> {
    let g: String = grid.trim().chars().take(4).collect::<String>().to_uppercase();
    (g.chars().count() == 4).then_some(g)
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
        }
        if let Some(g) = grid4(grid) {
            self.grids.insert(g);
        }
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
        }
        if grid.and_then(grid4).is_some_and(|g| !self.grids.contains(&g)) {
            return Novelty::NewGrid;
        }
        if self.calls.contains(&(band, call)) {
            return Novelty::Worked;
        }
        Novelty::Nothing
    }

    /// QSO completi letti o aggiunti.
    pub fn len(&self) -> usize {
        self.qsos
    }

    pub fn is_empty(&self) -> bool {
        self.qsos == 0
    }
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
        // zona CQ 15 (AT5) gia' collegata sul log: AT5EF era un QSO; zona 14 e' di AT3AB
        assert_eq!(log.novelty("AT9ZZ", Some("FN42"), "20m", &db), Novelty::Nothing);
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
        assert_eq!(log.novelty("LM2ZZ", Some("KK00"), "20m", &db), Novelty::Nothing, "il paese non e' piu' nuovo");
    }

    #[test]
    fn unknown_calls_are_never_dxcc_news() {
        let db = db();
        let log = WorkedLog::from_adif(ADIF, &db);
        assert_eq!(log.novelty("QQ9QQ", None, "20m", &db), Novelty::Nothing);
        assert_eq!(log.novelty("QQ9QQ", Some("KK00"), "20m", &db), Novelty::NewGrid);
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
