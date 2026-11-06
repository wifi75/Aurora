use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    pub name: String,
    pub prefix: String,
    pub cq_zone: u8,
    pub itu_zone: u8,
    pub continent: String,
    /// Latitudine e longitudine del paese in centesimi di grado (nord ed est positivi; cty.dat scrive la longitudine ovest positiva).
    pub lat_e2: i32,
    pub lon_e2: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lookup {
    pub entity: usize,
    pub cq_zone: u8,
    pub itu_zone: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DxccError {
    Io(String),
    NoEntities,
}

impl std::fmt::Display for DxccError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxccError::Io(e) => write!(f, "cty.dat non leggibile: {e}"),
            DxccError::NoEntities => write!(f, "il file non contiene nessun paese nel formato di cty.dat"),
        }
    }
}

impl std::error::Error for DxccError {}

pub struct DxccDb {
    entities: Vec<Entity>,
    exact: std::collections::HashMap<String, Lookup>,
    prefixes: std::collections::HashMap<String, Lookup>,
    longest_prefix: usize,
}

impl DxccDb {
    pub fn from_file(path: &Path) -> Result<DxccDb, DxccError> {
        let bytes = std::fs::read(path).map_err(|e| DxccError::Io(format!("{}: {e}", path.display())))?;
        Self::parse(&String::from_utf8_lossy(&bytes))
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn entity(&self, index: usize) -> &Entity {
        &self.entities[index]
    }

    pub fn parse(text: &str) -> Result<DxccDb, DxccError> {
        let mut db = DxccDb {
            entities: Vec::new(),
            exact: Default::default(),
            prefixes: Default::default(),
            longest_prefix: 0,
        };
        let mut aliases = String::new();
        let mut current: Option<usize> = None;
        for line in text.lines() {
            let indented = line.starts_with(' ') || line.starts_with('\t');
            if !indented && line.contains(':') {
                db.flush(current, &aliases);
                aliases.clear();
                current = db.read_header(line);
            } else if current.is_some() {
                aliases.push_str(line.trim());
            }
        }
        db.flush(current, &aliases);
        if db.entities.is_empty() {
            return Err(DxccError::NoEntities);
        }
        Ok(db)
    }

    fn read_header(&mut self, line: &str) -> Option<usize> {
        let fields: Vec<&str> = line.split(':').map(str::trim).collect();
        if fields.len() < 8 {
            return None;
        }
        let entity = Entity {
            name: fields[0].to_string(),
            cq_zone: fields[1].parse().ok()?,
            itu_zone: fields[2].parse().ok()?,
            continent: fields[3].to_string(),
            prefix: fields[7].trim_start_matches('*').to_string(),
            lat_e2: fields[4].parse::<f64>().map_or(0, |v| (v * 100.0).round() as i32),
            lon_e2: fields[5].parse::<f64>().map_or(0, |v| (-v * 100.0).round() as i32),
        };
        self.entities.push(entity);
        Some(self.entities.len() - 1)
    }

    fn flush(&mut self, entity: Option<usize>, aliases: &str) {
        let Some(index) = entity else { return };
        let (cq, itu) = (self.entities[index].cq_zone, self.entities[index].itu_zone);
        for token in aliases.trim_end_matches(';').split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let (exact, token) = match token.strip_prefix('=') {
                Some(rest) => (true, rest),
                None => (false, token),
            };
            let (key, cq_zone, itu_zone) = split_overrides(token, cq, itu);
            if key.is_empty() {
                continue;
            }
            let lookup = Lookup { entity: index, cq_zone, itu_zone };
            if exact {
                self.exact.insert(key, lookup);
            } else {
                self.longest_prefix = self.longest_prefix.max(key.len());
                self.prefixes.insert(key, lookup);
            }
        }
        // il prefisso principale e' sempre valido
        let main = self.entities[index].prefix.to_uppercase();
        if !main.is_empty() {
            self.longest_prefix = self.longest_prefix.max(main.len());
            self.prefixes.entry(main).or_insert(Lookup { entity: index, cq_zone: cq, itu_zone: itu });
        }
    }

    /// Paese di un nominativo; `None` se non e' riconoscibile.
    pub fn lookup(&self, call: &str) -> Option<Lookup> {
        let call = call.trim().to_uppercase();
        if call.is_empty() || call.starts_with('<') || !call.chars().any(|c| c.is_ascii_digit()) {
            return None;
        }
        if let Some(hit) = self.exact.get(&call) {
            return Some(*hit);
        }
        let parts: Vec<&str> = call.split('/').filter(|p| !p.is_empty()).collect();
        let mut kept = Vec::new();
        for part in &parts {
            match *part {
                "MM" | "AM" => return None,
                "P" | "M" | "QRP" | "A" | "B" | "LH" => {}
                p if p.len() == 1 && p.chars().all(|c| c.is_ascii_digit()) => {}
                p => kept.push(p),
            }
        }
        match kept.as_slice() {
            [] => None,
            [only] => self.by_prefix(only),
            [a, b] => {
                // il pezzo piu' corto e' il prefisso; se non da' un paese si prova l'altro
                let (short, long) = if a.len() <= b.len() { (*a, *b) } else { (*b, *a) };
                self.by_prefix(short).or_else(|| self.by_prefix(long))
            }
            [first, ..] => self.by_prefix(first),
        }
    }

    /// Il locatore e' credibile per il paese del nominativo? Falso solo se il centro del quadrato dista dal centro del paese (come lo scrive
    /// `cty.dat`) piu' di [`GRID_MAX_KM`]: e' un controllo largo, che prende i locatori dall'altra parte del mondo ma non i paesi grandi. Vero se il
    /// nominativo ha un `/` (portatili, maritimi), non si riconosce o il paese non ha una posizione.
    pub fn grid_plausible(&self, call: &str, grid: &str) -> bool {
        // «RR73» ha la forma di un locatore (nel Pacifico) ma in FT8 e' un saluto: non si giudica
        if call.contains('/') || grid.trim().eq_ignore_ascii_case("RR73") {
            return true;
        }
        let Some(hit) = self.lookup(call) else { return true };
        let entity = &self.entities[hit.entity];
        if entity.lat_e2 == 0 && entity.lon_e2 == 0 {
            return true;
        }
        let Some((lat, lon)) = grid_center(grid) else { return true };
        distance_km(lat, lon, f64::from(entity.lat_e2) / 100.0, f64::from(entity.lon_e2) / 100.0) <= GRID_MAX_KM
    }

    fn by_prefix(&self, call: &str) -> Option<Lookup> {
        if let Some(hit) = self.exact.get(call) {
            return Some(*hit);
        }
        let max = call.len().min(self.longest_prefix);
        (1..=max).rev().find_map(|n| call.get(..n).and_then(|p| self.prefixes.get(p)).copied())
    }
}

/// Distanza massima, in km, tra il centro di un paese e un locatore creduto suo. Il paese piu' esteso (Canada, Russia asiatica, Australia) arriva
/// a 4500 km dal suo centro; oltre 5500 non e' piu' lo stesso paese.
pub const GRID_MAX_KM: f64 = 5500.0;

/// Latitudine e longitudine (gradi) del centro di un locatore a 4 o 6 caratteri; `None` se non e' un locatore.
pub fn grid_center(grid: &str) -> Option<(f64, f64)> {
    let b = grid.trim().as_bytes();
    if b.len() != 4 && b.len() != 6 {
        return None;
    }
    let field = |c: u8| (b'A'..=b'R').contains(&c.to_ascii_uppercase()).then(|| f64::from(c.to_ascii_uppercase() - b'A'));
    let digit = |c: u8| c.is_ascii_digit().then(|| f64::from(c - b'0'));
    let (mut lon, mut lat) = (field(b[0])? * 20.0 - 180.0 + digit(b[2])? * 2.0, field(b[1])? * 10.0 - 90.0 + digit(b[3])?);
    if b.len() == 6 {
        let sub = |c: u8| (b'A'..=b'X').contains(&c.to_ascii_uppercase()).then(|| f64::from(c.to_ascii_uppercase() - b'A'));
        lon += sub(b[4])? * (2.0 / 24.0) + 1.0 / 24.0;
        lat += sub(b[5])? * (1.0 / 24.0) + 0.5 / 24.0;
    } else {
        lon += 1.0;
        lat += 0.5;
    }
    Some((lat, lon))
}

/// Distanza sul cerchio massimo tra due punti, in km.
pub fn distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let a = ((p2 - p1) / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    6371.0 * 2.0 * a.sqrt().min(1.0).asin()
}

/// `AT5(15)[30]` -> ("AT5", Some zona CQ 15, zona ITU 30); gli altri modificatori si ignorano.
fn split_overrides(token: &str, cq: u8, itu: u8) -> (String, u8, u8) {
    let (mut cq_zone, mut itu_zone) = (cq, itu);
    let key_end = token.find(['(', '[', '<', '{', '~']).unwrap_or(token.len());
    let mut rest = &token[key_end..];
    while let Some(open) = rest.chars().next() {
        let close = match open {
            '(' => ')',
            '[' => ']',
            '<' => '>',
            '{' => '}',
            '~' => '~',
            _ => break,
        };
        let Some(end) = rest[1..].find(close) else { break };
        let value = &rest[1..1 + end];
        match open {
            '(' => cq_zone = value.parse().unwrap_or(cq_zone),
            '[' => itu_zone = value.parse().unwrap_or(itu_zone),
            _ => {}
        }
        rest = &rest[end + 2..];
    }
    (token[..key_end].to_uppercase(), cq_zone, itu_zone)
}

#[cfg(test)]
mod grid_tests {
    use super::*;

    const CTY: &str = "Italy:                    15:  28:  EU:   43.00:   -12.00:    -1.0:  I:
    I,IK,IU;
United States:            05:  08:  NA:   37.53:    91.67:     5.0:  K:
    K,W,N,AA;
Japan:                    25:  45:  AS:   36.00:  -138.00:    -9.0:  JA:
    JA,JE;
";

    #[test]
    fn il_centro_di_un_locatore() {
        let (lat, lon) = grid_center("JN45").expect("locatore");
        // JN45: campo J (0 gradi E) + quadrato 4 (8 gradi) + mezzo quadrato = 9 gradi E, 45,5 N (zona di Milano)
        assert!((lat - 45.5).abs() < 0.01 && (lon - 9.0).abs() < 0.01, "{lat} {lon}");
        let (lat, lon) = grid_center("FN42ab").expect("locatore");
        assert!((lat - 42.02).abs() < 0.05 && (lon + 71.96).abs() < 0.05, "{lat} {lon}");
        assert!(grid_center("RR73x").is_none() && grid_center("ZZ99").is_none() && grid_center("-15").is_none());
    }

    #[test]
    fn la_distanza_tra_citta_note() {
        // Roma-New York: circa 6900 km
        let d = distance_km(41.9, 12.5, 40.7, -74.0);
        assert!((6800.0..7000.0).contains(&d), "{d}");
    }

    #[test]
    fn un_locatore_dall_altra_parte_del_mondo_non_e_credibile() {
        let db = DxccDb::parse(CTY).expect("cty");
        assert!(db.grid_plausible("IU3ABC", "JN45"));
        assert!(db.grid_plausible("IU3ABC", "JO62"), "Berlino e' vicina");
        assert!(db.grid_plausible("IU3ABC", "KN32"), "Mar Nero, 1800 km");
        assert!(!db.grid_plausible("IU3ABC", "FN42"), "New York");
        assert!(!db.grid_plausible("IU3ABC", "PM95"), "Tokyo");
        assert!(db.grid_plausible("W1AW", "FN31") && db.grid_plausible("W7XYZ", "CN87"), "tutto il paese e' dentro il limite");
        assert!(!db.grid_plausible("W1AW", "JN45"));
    }

    #[test]
    fn portatili_sconosciuti_e_locatori_non_validi_non_si_giudicano() {
        let db = DxccDb::parse(CTY).expect("cty");
        assert!(db.grid_plausible("IU3ABC/P", "FN42"));
        assert!(db.grid_plausible("ZZ9ZZZ", "FN42"));
        assert!(db.grid_plausible("IU3ABC", "RR73"));
    }
}
