use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    pub name: String,
    pub prefix: String,
    pub cq_zone: u8,
    pub itu_zone: u8,
    pub continent: String,
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

    fn by_prefix(&self, call: &str) -> Option<Lookup> {
        if let Some(hit) = self.exact.get(call) {
            return Some(*hit);
        }
        let max = call.len().min(self.longest_prefix);
        (1..=max).rev().find_map(|n| call.get(..n).and_then(|p| self.prefixes.get(p)).copied())
    }
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
