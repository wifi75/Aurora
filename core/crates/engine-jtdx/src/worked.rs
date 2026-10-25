use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use aurora_core::adif::{self, AdifQso};
use dxcc::DxccDb;
use serde::Serialize;

use crate::matrix::{Cell, ModeClass};

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
    pub(crate) qsos: usize,
    /// (banda, nominativo) gia' collegati.
    calls: HashSet<(String, String)>,
    /// Nominativi collegati su almeno una banda.
    any_call: HashSet<String>,
    /// Paesi collegati su almeno una banda.
    pub(crate) entities: HashSet<usize>,
    /// (paese, banda) collegati.
    pub(crate) entity_bands: HashSet<(usize, String)>,
    zones: HashSet<u8>,
    zone_bands: HashSet<(u8, String)>,
    itu: HashSet<u8>,
    itu_bands: HashSet<(u8, String)>,
    /// Locatori di 4 caratteri collegati.
    pub(crate) grids: HashSet<String>,
    pub(crate) grid_bands: HashSet<(String, String)>,
    prefixes: HashSet<String>,
    prefix_bands: HashSet<(String, String)>,
    band_qsos: HashMap<String, usize>,
    /// (paese, modo) collegati, su qualunque banda: per «nuovo modo».
    pub(crate) entity_modes: HashSet<(usize, ModeClass)>,
    /// (locatore a 4 caratteri, modo) collegati.
    pub(crate) grid_modes: HashSet<(String, ModeClass)>,
    /// QSO per (paese, banda, modo): la matrice DXCC.
    pub(crate) cells: HashMap<(usize, String, ModeClass), Cell>,
    /// QSO per (paese, banda) con qualunque modo, anche non noto.
    pub(crate) any_cells: HashMap<(usize, String), Cell>,
    /// Chiavi dei QSO con data gia' contati (stesso QSO letto da due log: conta una volta sola).
    seen: HashSet<String>,
    /// Indice dei criteri di «nuovo» di JTDX (zona CQ, ITU, paese, locatore, prefisso, nominativo), ognuno a quattro livelli: in qualunque banda e
    /// modo, solo banda, solo modo, banda e modo (vedi `Criterion` e `novelty_flags`). Si tengono gli hash a 64 bit delle chiavi.
    criteria: HashSet<u64>,
}

/// I sei criteri con cui JTDX decide che cosa e' «nuovo» (`LogBook::matchCQZ`, `matchITUZ`, `matchDXCC`, `matchGrid`, `matchPX`, `matchCall`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Crit {
    Cqz,
    Ituz,
    Dxcc,
    Grid,
    Px,
    Call,
}

/// Il filtro del log della scheda Generali (`ADIF::load` di JTDX): «usa questo nominativo», «usa questa griglia» e «filtra il LOG a partire dalla
/// data/ora». Un record entra se (nominativo vuoto o `STATION_CALLSIGN` uguale) e (griglia vuota o `MY_GRIDSQUARE` uguale nei primi quattro
/// caratteri) e (data vuota o `QSO_DATE` + `TIME_ON` >= data). I log che non dicono con quale nominativo o locatore si e' operato (il log interno,
/// Log4OM2) non si possono escludere per quei due motivi: restano; la data vale per tutti.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogFilter {
    pub call: Option<String>,
    pub grid: Option<String>,
    /// `AAAAMMGGHHMMSS`: i QSO precedenti non contano.
    pub from: Option<u64>,
}

impl LogFilter {
    /// La data/ora del filtro: solo cifre, al piu' 14, completata con zeri a destra (come fa JTDX con l'ora del QSO); vuota = nessun filtro.
    pub fn parse_from(text: &str) -> Option<u64> {
        let digits: String = text.trim().to_string();
        if digits.is_empty() || digits.len() > 14 || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        format!("{digits:0<14}").parse().ok()
    }

    /// Nessun filtro acceso.
    pub fn is_off(&self) -> bool {
        self.call.is_none() && self.grid.is_none() && self.from.is_none()
    }

    /// Il QSO passa il filtro?
    pub fn accepts(&self, q: &AdifQso) -> bool {
        if let Some(call) = &self.call
            && !q.station_call.is_empty()
            && !q.station_call.eq_ignore_ascii_case(call)
        {
            return false;
        }
        if let Some(grid) = &self.grid
            && !q.my_grid.is_empty()
            && !q.my_grid.chars().take(4).collect::<String>().eq_ignore_ascii_case(&grid.chars().take(4).collect::<String>())
        {
            return false;
        }
        if let Some(from) = self.from {
            // senza data il QSO non si puo' datare: JTDX lo confronta come numero (data vuota + ora = piccolo) e lo scarta
            let when = format!("{}{}", q.date.trim(), q.time_on.trim());
            let when: u64 = format!("{when:0<14}").parse().unwrap_or(0);
            if when < from {
                return false;
            }
        }
        true
    }
}

/// Il livello «modo» di una chiave: qualunque, non noto (QSO senza modo: conta per ogni modo, non si puo' dire il contrario) o uno preciso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ModeKey {
    Any,
    Unknown,
    Class(ModeClass),
}

/// La chiave di un criterio a un livello: banda e modo, come i due argomenti di `ADIF::match*` (vuoto = qualunque).
fn crit_key(crit: Crit, key: &str, band: &str, mode: ModeKey) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    crit.hash(&mut h);
    key.hash(&mut h);
    band.hash(&mut h);
    mode.hash(&mut h);
    h.finish()
}

/// Una voce delle evidenziazioni di JTDX (scheda Notifiche): «evidenzia nuovo X», «per banda», «per modo» (`newX`, `newXBand`, `newXBandMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Criterion {
    pub on: bool,
    /// «per banda»: nuovo anche se e' stato collegato solo su un'altra banda. Vale solo con `on`.
    pub band: bool,
    /// «per modo»: nuovo anche se e' stato collegato solo in un altro modo. Con `band` conta la coppia banda e modo.
    pub band_mode: bool,
}

impl Criterion {
    pub const fn new(on: bool, band: bool, band_mode: bool) -> Criterion {
        Criterion { on, band: on && band, band_mode: on && band_mode }
    }
}

/// Le evidenziazioni di «nuovo» di JTDX. Se nessuna e' accesa JTDX non calcola niente: nemmeno «gia' lavorato».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightFlags {
    pub cqz: Criterion,
    pub itu: Criterion,
    pub dxcc: Criterion,
    pub grid: Criterion,
    pub px: Criterion,
    pub call: Criterion,
    /// «Mostra anche i messaggi tra altre stazioni» (`newPotential`): le novita' valgono anche per i messaggi che non sono CQ.
    pub potential: bool,
}

impl HighlightFlags {
    /// Tutto acceso, per banda e per modo.
    pub const fn all_on() -> HighlightFlags {
        let c = Criterion::new(true, true, true);
        HighlightFlags { cqz: c, itu: c, dxcc: c, grid: c, px: c, call: c, potential: false }
    }

    /// Nessuna evidenziazione di «nuovo».
    pub const fn none() -> HighlightFlags {
        let c = Criterion::new(false, false, false);
        HighlightFlags { cqz: c, itu: c, dxcc: c, grid: c, px: c, call: c, potential: false }
    }

    /// Almeno una evidenziazione accesa (`newCQZ_ || newITUZ_ || newDXCC_ || newGrid_ || newPx_ || newCall_`).
    pub fn any(&self) -> bool {
        self.cqz.on || self.itu.on || self.dxcc.on || self.grid.on || self.px.on || self.call.on
    }

    /// I valori di fabbrica di JTDX (`Configuration::read_settings`): paese e nominativo accesi, ognuno anche per banda e per modo; zone CQ e ITU,
    /// locatore e prefisso spenti.
    pub const fn jtdx_defaults() -> HighlightFlags {
        let on = Criterion::new(true, true, true);
        let off = Criterion::new(false, false, false);
        HighlightFlags { cqz: off, itu: off, dxcc: on, grid: off, px: off, call: on, potential: false }
    }

    /// Dai valori delle impostazioni (nomi di `JTDX.ini`: `newCQZ`, `newCQZBand`, `newCQZBandMode`...): `get` risponde `None` se la voce manca
    /// (vale il valore di fabbrica di JTDX).
    pub fn from_lookup(get: impl Fn(&str) -> Option<bool>) -> HighlightFlags {
        let d = HighlightFlags::jtdx_defaults();
        let one = |name: &str, d: Criterion| Criterion::new(get(name).unwrap_or(d.on), get(&format!("{name}Band")).unwrap_or(d.band), get(&format!("{name}BandMode")).unwrap_or(d.band_mode));
        HighlightFlags { cqz: one("newCQZ", d.cqz), itu: one("newITUZ", d.itu), dxcc: one("newDXCC", d.dxcc), grid: one("newGrid", d.grid), px: one("newPx", d.px), call: one("newCall", d.call), potential: get("newPotential").unwrap_or(false) }
    }
}

impl Default for HighlightFlags {
    fn default() -> Self {
        HighlightFlags::jtdx_defaults()
    }
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
        for q in adif::parse_qsos(text) {
            log.add_adif_qso(&q, db);
        }
        log
    }

    /// Aggiunge un QSO senza modo noto: conta per paese, zone, locatore, prefisso e stazione, non per i modi.
    pub fn add_qso(&mut self, call: &str, grid: &str, band: &str, db: &DxccDb) {
        self.add_qso_mode(call, grid, band, "", db);
    }

    /// Aggiunge un QSO con il suo modo (`FT8`, `FT4`...; vuoto = non noto).
    pub fn add_qso_mode(&mut self, call: &str, grid: &str, band: &str, mode: &str, db: &DxccDb) {
        let q = AdifQso { call: call.to_string(), band: band.to_string(), mode: mode.to_string(), grid: grid.to_string(), ..AdifQso::default() };
        self.add_adif_qso(&q, db);
    }

    /// Aggiunge un QSO letto da un log. Se ha data e ora e lo stesso QSO (stessa stazione, banda, modo, giorno e minuto) e' gia' stato aggiunto
    /// da un'altra sorgente, non lo conta di nuovo. Ritorna `false` per un doppione o un QSO senza nominativo o banda.
    pub fn add_adif_qso(&mut self, q: &AdifQso, db: &DxccDb) -> bool {
        let call = q.call.trim().to_uppercase();
        let band = q.band.trim().to_lowercase();
        if call.is_empty() || band.is_empty() {
            return false;
        }
        let class = ModeClass::from_adif(&q.mode, "");
        if !q.date.is_empty() {
            let minute: String = q.time_on.chars().take(4).collect();
            let key = format!("{call}|{band}|{}|{}|{minute}", q.mode.trim().to_uppercase(), q.date);
            if !self.seen.insert(key) {
                return false;
            }
        }
        let found = db.lookup(&call);
        if let Some(found) = found {
            self.index(Crit::Cqz, &found.cq_zone.to_string(), &band, class);
            self.index(Crit::Ituz, &found.itu_zone.to_string(), &band, class);
            self.index(Crit::Dxcc, &found.entity.to_string(), &band, class);
            self.entities.insert(found.entity);
            self.entity_bands.insert((found.entity, band.clone()));
            self.zones.insert(found.cq_zone);
            self.zone_bands.insert((found.cq_zone, band.clone()));
            self.itu.insert(found.itu_zone);
            self.itu_bands.insert((found.itu_zone, band.clone()));
            self.any_cells.entry((found.entity, band.clone())).or_default().add(q.confirmed);
            if let Some(class) = class {
                self.entity_modes.insert((found.entity, class));
                self.cells.entry((found.entity, band.clone(), class)).or_default().add(q.confirmed);
            }
        }
        if let Some(g) = grid4(&q.grid) {
            self.index(Crit::Grid, &g, &band, class);
            self.grid_bands.insert((g.clone(), band.clone()));
            if let Some(class) = class {
                self.grid_modes.insert((g.clone(), class));
            }
            self.grids.insert(g);
        }
        if let Some(px) = wpx_prefix(&call) {
            self.index(Crit::Px, &px, &band, class);
            self.prefix_bands.insert((px.clone(), band.clone()));
            self.prefixes.insert(px);
        }
        self.index(Crit::Call, &call, &band, class);
        self.any_call.insert(call.clone());
        *self.band_qsos.entry(band.clone()).or_insert(0) += 1;
        self.calls.insert((band, call));
        self.qsos += 1;
        true
    }

    /// Registra un QSO nei livelli di un criterio: qualunque, solo banda, solo modo, banda e modo (come `ADIF::add`). Un QSO senza modo si
    /// ricorda a parte: vale per ogni modo (non si puo' dire che sia un altro).
    fn index(&mut self, crit: Crit, key: &str, band: &str, class: Option<ModeClass>) {
        self.criteria.insert(crit_key(crit, key, "", ModeKey::Any));
        self.criteria.insert(crit_key(crit, key, band, ModeKey::Any));
        match class {
            Some(c) => {
                self.criteria.insert(crit_key(crit, key, "", ModeKey::Class(c)));
                self.criteria.insert(crit_key(crit, key, band, ModeKey::Class(c)));
            }
            None => {
                self.criteria.insert(crit_key(crit, key, "", ModeKey::Unknown));
                self.criteria.insert(crit_key(crit, key, band, ModeKey::Unknown));
            }
        }
    }

    /// `ADIF::match*` di JTDX: l'elemento e' nel log con questa banda e questo modo (vuoti = qualunque)?
    fn seen_at(&self, crit: Crit, key: &str, band: &str, class: Option<ModeClass>) -> bool {
        match class {
            None => self.criteria.contains(&crit_key(crit, key, band, ModeKey::Any)),
            Some(c) => self.criteria.contains(&crit_key(crit, key, band, ModeKey::Class(c))) || self.criteria.contains(&crit_key(crit, key, band, ModeKey::Unknown)),
        }
    }

    /// Il criterio e' «nuovo» secondo le sue tre caselle (`DisplayText::displayDecodedText`): `Some(false)` nuovo in assoluto, `Some(true)` nuovo
    /// solo per banda o modo, `None` niente di nuovo. Senza banda e senza modo noti il livello banda/modo non si controlla (come `LogBook::match*`).
    fn criterion_news(&self, crit: Crit, key: &str, c: Criterion, band: &str, class: Option<ModeClass>) -> Option<bool> {
        if c.on && !self.seen_at(crit, key, "", None) {
            return Some(false);
        }
        if !(c.band || c.band_mode) || (band.is_empty() && class.is_none()) {
            return None;
        }
        // banda e modo insieme, solo banda, solo modo: gli argomenti con cui JTDX chiama `match*`
        // Per paese e zone (non per locatore, prefisso e nominativo) «banda» vince su «banda+modo»: se il paese e' gia' stato collegato su questa
        // banda, anche in RTTY o SSB, non e' «nuovo su banda» (scelta del proprietario; JTDX con tutte e due le caselle guarda banda e modo insieme).
        let by_area = matches!(crit, Crit::Dxcc | Crit::Cqz | Crit::Ituz);
        let (b, m) = match (c.band, c.band_mode) {
            (true, true) if by_area => (band, None),
            (true, true) => (band, class),
            (true, false) => (band, None),
            _ => ("", class),
        };
        (!self.seen_at(crit, key, b, m)).then_some(true)
    }

    /// La novita' di una stazione come la calcola JTDX: stesso ordine (zona CQ, zona ITU, paese, locatore, prefisso, nominativo, ognuno prima in
    /// assoluto e poi per banda/modo) e stesse caselle (`flags`). Se la stazione non e' nuova per nessun criterio acceso e' `Worked`: **e' il
    /// «gia' lavorato» di JTDX** (non c'e' un controllo a parte: e' cio' che resta quando niente e' nuovo). Senza nessun criterio acceso JTDX
    /// non calcola niente e qui il risultato e' `None`. `mode` e' il modo dei decode (FT8, FT4...); `None` se non si sa.
    ///
    /// Il paese non riconosciuto da `cty.dat` non e' mai nuovo (`matchDXCC`: `WorkedBefore = true`); il locatore conta solo se c'e'.
    pub fn novelty_flags(&self, call: &str, grid: Option<&str>, band: &str, mode: Option<&str>, flags: &HighlightFlags, db: &DxccDb) -> Option<Novelty> {
        if !flags.any() {
            return None;
        }
        let call = call.trim().to_uppercase();
        let band = band.trim().to_lowercase();
        let class = mode.and_then(|m| ModeClass::from_adif(m, ""));
        let found = db.lookup(&call);
        let news = |crit: Crit, key: &str, c: Criterion| self.criterion_news(crit, key, c, &band, class);
        let pick = |n: Option<bool>, broad: Novelty, narrow: Novelty| n.map(|only_band| if only_band { narrow } else { broad });
        if let Some(f) = &found {
            if let Some(n) = pick(news(Crit::Cqz, &f.cq_zone.to_string(), flags.cqz), Novelty::NewZone, Novelty::NewZoneBand) {
                return Some(n);
            }
            if let Some(n) = pick(news(Crit::Ituz, &f.itu_zone.to_string(), flags.itu), Novelty::NewItu, Novelty::NewItuBand) {
                return Some(n);
            }
            if let Some(n) = pick(news(Crit::Dxcc, &f.entity.to_string(), flags.dxcc), Novelty::NewDxcc, Novelty::NewDxccBand) {
                return Some(n);
            }
        }
        if let Some(g) = grid.and_then(grid4)
            && let Some(n) = pick(news(Crit::Grid, &g, flags.grid), Novelty::NewGrid, Novelty::NewGridBand)
        {
            return Some(n);
        }
        if let Some(px) = wpx_prefix(&call)
            && let Some(n) = pick(news(Crit::Px, &px, flags.px), Novelty::NewPx, Novelty::NewPxBand)
        {
            return Some(n);
        }
        if let Some(n) = pick(news(Crit::Call, &call, flags.call), Novelty::NewCall, Novelty::NewCallBand) {
            return Some(n);
        }
        Some(Novelty::Worked)
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
        assert_eq!(wpx_prefix("IK1ABC").as_deref(), Some("IK1"));
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

    /// Solo nominativo (per banda e per modo), come la schermata di JTDX con «nuovo nominativo» e le sue due caselle.
    fn call_only(band: bool, band_mode: bool) -> HighlightFlags {
        HighlightFlags { call: Criterion::new(true, band, band_mode), ..HighlightFlags::none() }
    }

    fn qso(call: &str, band: &str, mode: &str) -> AdifQso {
        AdifQso { call: call.into(), band: band.into(), mode: mode.into(), ..AdifQso::default() }
    }

    #[test]
    fn worked_before_needs_the_same_band_and_the_same_mode() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_adif_qso(&qso("AT3AB", "20m", "FT8"), &db);
        let flags = call_only(true, true);
        let at = |band: &str, mode: &str| log.novelty_flags("AT3AB", None, band, Some(mode), &flags, &db);
        assert_eq!(at("20m", "FT8"), Some(Novelty::Worked), "stessa banda e stesso modo: gia' lavorato");
        assert_eq!(at("20m", "FT4"), Some(Novelty::NewCallBand), "stessa banda, altro modo: nuovo per modo");
        assert_eq!(at("40m", "FT8"), Some(Novelty::NewCallBand), "altra banda, stesso modo: nuovo per banda");
        assert_eq!(at("40m", "FT4"), Some(Novelty::NewCallBand));
        assert_eq!(log.novelty_flags("AT9ZZ", None, "20m", Some("FT8"), &flags, &db), Some(Novelty::NewCall), "mai collegato");
    }

    #[test]
    fn a_country_worked_on_the_band_in_any_mode_is_not_new_on_the_band() {
        let db = db();
        let mut log = WorkedLog::new();
        // un altro nominativo dello stesso paese, sulla stessa banda ma in RTTY
        log.add_adif_qso(&qso("AT3AB", "20m", "RTTY"), &db);
        let flags = HighlightFlags { dxcc: Criterion::new(true, true, true), ..HighlightFlags::none() };
        let n = log.novelty_flags("AT7XY", None, "20m", Some("FT8"), &flags, &db);
        assert_ne!(n, Some(Novelty::NewDxccBand), "stessa banda, altro modo e altro nominativo: il paese non e' nuovo su banda");
        assert_eq!(log.novelty_flags("AT7XY", None, "40m", Some("FT8"), &flags, &db), Some(Novelty::NewDxccBand), "altra banda: nuovo su banda");
    }

    #[test]
    fn without_the_band_and_mode_boxes_worked_means_in_the_log_at_all() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_adif_qso(&qso("AT3AB", "20m", "FT8"), &db);
        let flags = call_only(false, false);
        assert_eq!(log.novelty_flags("AT3AB", None, "40m", Some("FT4"), &flags, &db), Some(Novelty::Worked), "su un'altra banda e in un altro modo e' comunque lavorato");
    }

    #[test]
    fn only_the_band_box_ignores_the_mode_and_only_the_mode_box_ignores_the_band() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_adif_qso(&qso("AT3AB", "20m", "FT8"), &db);
        let band = call_only(true, false);
        assert_eq!(log.novelty_flags("AT3AB", None, "20m", Some("FT4"), &band, &db), Some(Novelty::Worked), "solo banda: il modo non conta");
        assert_eq!(log.novelty_flags("AT3AB", None, "40m", Some("FT8"), &band, &db), Some(Novelty::NewCallBand));
        // JTDX: casella «per modo» senza «per banda» = solo modo (la banda non conta)
        let mode = call_only(false, true);
        assert_eq!(log.novelty_flags("AT3AB", None, "40m", Some("FT8"), &mode, &db), Some(Novelty::Worked), "solo modo: la banda non conta");
        assert_eq!(log.novelty_flags("AT3AB", None, "20m", Some("FT4"), &mode, &db), Some(Novelty::NewCallBand));
    }

    #[test]
    fn without_any_highlight_nothing_is_computed_not_even_worked() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_adif_qso(&qso("AT3AB", "20m", "FT8"), &db);
        assert_eq!(log.novelty_flags("AT3AB", None, "20m", Some("FT8"), &HighlightFlags::none(), &db), None);
    }

    #[test]
    fn a_new_thing_cancels_worked_and_the_jtdx_order_starts_from_the_cq_zone() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_adif_qso(&AdifQso { grid: "FN42".into(), ..qso("AT3AB", "20m", "FT8") }, &db);
        let all = HighlightFlags::all_on();
        // stessa stazione, stessa banda e modo, ma un locatore mai collegato: nuovo locatore, non piu' «gia' lavorato»
        assert_eq!(log.novelty_flags("AT3AB", Some("GG55"), "20m", Some("FT8"), &all, &db), Some(Novelty::NewGrid));
        assert_eq!(log.novelty_flags("AT3AB", Some("FN42"), "20m", Some("FT8"), &all, &db), Some(Novelty::Worked));
        // zona CQ nuova (AT5 e' la 15) vince su tutto, prima del paese: l'ordine di JTDX parte dalla zona CQ
        assert_eq!(log.novelty_flags("AT5XX", Some("GG55"), "20m", Some("FT8"), &all, &db), Some(Novelty::NewZone));
        // un paese mai collegato: con zona CQ gia' fatta e' il paese; Lemuria ha zona 05
        assert_eq!(log.novelty_flags("LM1XY", Some("KK00"), "20m", Some("FT8"), &all, &db), Some(Novelty::NewZone));
    }

    #[test]
    fn a_qso_without_a_mode_counts_for_every_mode() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_qso("AT3AB", "", "20m", &db);
        let flags = call_only(true, true);
        assert_eq!(log.novelty_flags("AT3AB", None, "20m", Some("FT4"), &flags, &db), Some(Novelty::Worked));
        assert_eq!(log.novelty_flags("AT3AB", None, "40m", Some("FT4"), &flags, &db), Some(Novelty::NewCallBand));
    }

    #[test]
    fn unknown_mode_checks_only_the_band_level() {
        let db = db();
        let mut log = WorkedLog::new();
        log.add_adif_qso(&qso("AT3AB", "20m", "FT8"), &db);
        let flags = call_only(true, true);
        assert_eq!(log.novelty_flags("AT3AB", None, "20m", None, &flags, &db), Some(Novelty::Worked));
        assert_eq!(log.novelty_flags("AT3AB", None, "40m", None, &flags, &db), Some(Novelty::NewCallBand));
    }

    #[test]
    fn flags_come_from_the_settings_names_and_default_to_on() {
        let get = |k: &str| match k {
            "newDXCC" => Some(false),
            "newCallBandMode" => Some(false),
            _ => None,
        };
        let f = HighlightFlags::from_lookup(get);
        assert!(!f.dxcc.on && !f.dxcc.band && !f.dxcc.band_mode, "senza la casella principale le altre due non contano");
        assert!(f.call.on && f.call.band && !f.call.band_mode);
        assert!(!f.cqz.on, "voce mancante: il valore di fabbrica di JTDX (zona CQ spenta)");
    }

    #[test]
    fn the_log_filter_works_like_adif_load() {
        let mut q = qso("AT3AB", "20m", "FT8");
        q.date = "20240315".into();
        q.time_on = "101500".into();
        q.station_call = "IU3ABC".into();
        q.my_grid = "IO91wm".into();
        let off = LogFilter::default();
        assert!(off.is_off() && off.accepts(&q));
        assert!(LogFilter { call: Some("IU3ABC".into()), ..off.clone() }.accepts(&q));
        assert!(!LogFilter { call: Some("IU3XYZ".into()), ..off.clone() }.accepts(&q), "altro nominativo di stazione");
        assert!(LogFilter { grid: Some("IO91".into()), ..off.clone() }.accepts(&q), "contano i primi quattro caratteri");
        assert!(!LogFilter { grid: Some("JN45".into()), ..off.clone() }.accepts(&q));
        let from = |t: &str| LogFilter { from: LogFilter::parse_from(t), ..off.clone() };
        assert!(from("20240315101500").accepts(&q), "uguale: entra");
        assert!(!from("20240315101501").accepts(&q));
        assert!(from("2024").accepts(&q) && !from("2025").accepts(&q), "data incompleta: completata con zeri");
        assert_eq!(LogFilter::parse_from(""), None);
        assert_eq!(LogFilter::parse_from("12ab"), None);
        assert_eq!(LogFilter::parse_from("202403151015001"), None, "piu' di 14 cifre");
        // un log che non dice con quale nominativo ha operato non si esclude per quel motivo; per la data si
        let bare = qso("AT3AB", "20m", "FT8");
        assert!(LogFilter { call: Some("IU3ABC".into()), grid: Some("IO91".into()), ..off.clone() }.accepts(&bare));
        assert!(!from("2024").accepts(&bare), "senza data non si puo' dire che sia recente");
    }
}
