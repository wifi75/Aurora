//! L'elenco dei contest dei modi testuali (PSK, RTTY, CW): un file `.contest` per contest nella cartella `<dati>/contest`, con le macro
//! del contest in `contest/macros` e gli elenchi dei domini (province, stati, zone) in `contest/dom`. Il formato e' descritto in
//! `docs/contest-file.md`. Un solo contest e' attivo alla volta («Nessun contest» = nessuno).
//!
//! Qui stanno anche la lettura dei file di macro (`[Macros]`, `NameN`/`LabelN`/`TextN`/`BkColorN`), dei domini e il convertitore dei file
//! di contest di un altro programma (`converti_cst`), che toglie i dati personali e le parti non supportate.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Estensione dei file di contest.
pub const ESTENSIONE: &str = "contest";
/// Il file che ricorda il contest attivo (solo l'identificativo, vuoto = nessuno).
const FILE_ATTIVO: &str = "attivo.txt";
/// Il segno che i contest predefiniti sono gia' stati copiati: quelli che l'utente cancella non tornano.
const FILE_PREDEFINITI: &str = ".predefiniti";
/// Le bande con un moltiplicatore proprio, nell'ordine dei file.
pub const BANDE: [&str; 6] = ["160", "80", "40", "20", "15", "10"];

/// Il moltiplicatore di una banda: acceso e con quale peso.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MoltBanda {
    pub band: String,
    pub on: bool,
    pub weight: i32,
}

/// Un contest dei modi testuali, come sta nel file `.contest`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContestTesto {
    /// Il nome del file senza estensione (non sta nel file).
    pub id: String,
    pub name: String,
    pub notes: String,
    /// Inizio e fine in UTC, `aaaammgghhmmss` (vuoto: nessun limite).
    pub start: String,
    pub end: String,
    /// Ore di lavoro ammesse (0 = nessun limite).
    pub work_time: i32,
    /// Il numero da mandare: il prossimo progressivo, oppure uno scambio fisso (zona, provincia) se l'incremento e' spento.
    pub nrs: String,
    pub auto_inc: bool,
    pub valid_contacts: i32,
    pub valid_cw: bool,
    pub valid_phone: bool,
    pub valid_digital: bool,
    pub points_my_dxcc: i32,
    pub points_my_cont: i32,
    pub points_dx: i32,
    pub multipliers: i32,
    pub multipliers_option: i32,
    pub band_mult: bool,
    pub bands: Vec<MoltBanda>,
    /// Il file di macro del contest, relativo alla cartella dei contest (`macros/<nome>.mc`).
    pub macros: String,
    pub merge_macros: bool,
    pub grab_number: bool,
    pub grab_number_type: i32,
    pub grab_time: bool,
    pub grab_state: bool,
    /// L'elenco dei domini, relativo alla cartella dei contest (`dom/<nome>.dom`).
    pub domain_file: String,
    pub grab_domain_to: i32,
    pub suggest_cq_zone: bool,
    pub suggest_itu_zone: bool,
    pub suggest_domain: bool,
    pub suggest_dxcc_pfx: bool,
    pub dupe_no_band: bool,
    pub dupe_no_mode: bool,
    pub log_bar_mode: i32,
    /// I QTC del WAE (`m_iQtc`, `docs/contest-qtc.md`): 0 nessuno, 1 dalle stazioni non europee alle europee (CW, SSB), 2 in entrambe
    /// le direzioni fra continenti diversi (RTTY).
    pub qtc: i32,
    /// Le regole del punteggio, righe `chiave=valore` della sezione `[Score]` (`regole_punti`, `docs/contest-file.md`); vuoto = le
    /// vecchie chiavi dei punti e dei moltiplicatori.
    pub score: String,
    /// Lo script dei casi speciali, testo Rhai della sezione `[Script]` (l'ultima del file, `crate::script`); vuoto = nessuno.
    pub script: String,
}

impl Default for ContestTesto {
    fn default() -> Self {
        ContestTesto {
            id: String::new(),
            name: String::new(),
            notes: String::new(),
            start: String::new(),
            end: String::new(),
            work_time: 0,
            nrs: String::new(),
            auto_inc: true,
            valid_contacts: 0,
            valid_cw: false,
            valid_phone: false,
            valid_digital: true,
            points_my_dxcc: 0,
            points_my_cont: 0,
            points_dx: 0,
            multipliers: 0,
            multipliers_option: 0,
            band_mult: false,
            bands: BANDE.iter().map(|b| MoltBanda { band: (*b).to_string(), on: false, weight: 0 }).collect(),
            macros: String::new(),
            merge_macros: true,
            grab_number: true,
            grab_number_type: -1,
            grab_time: false,
            grab_state: false,
            domain_file: String::new(),
            grab_domain_to: 0,
            suggest_cq_zone: false,
            suggest_itu_zone: false,
            suggest_domain: false,
            suggest_dxcc_pfx: false,
            dupe_no_band: false,
            dupe_no_mode: false,
            log_bar_mode: 0,
            qtc: 0,
            score: String::new(),
            script: String::new(),
        }
    }
}

/// Le coppie chiave = valore di una sezione INI (le righe senza `=` e i commenti `;` si saltano; chiavi senza maiuscole/minuscole).
fn coppie(testo: &str, sezione: &str) -> Vec<(String, String)> {
    let mut dentro = false;
    let mut out = Vec::new();
    for riga in testo.lines() {
        let r = riga.trim();
        if r.starts_with('[') && r.ends_with(']') {
            dentro = r[1..r.len() - 1].trim().eq_ignore_ascii_case(sezione);
            continue;
        }
        if !dentro || r.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = r.split_once('=') {
            out.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}

/// Il testo della sezione `[Script]`: tutto quello che la segue (e' l'ultima sezione del file, il codice puo' avere righe fra `[ ]`).
fn sezione_script(testo: &str) -> String {
    let mut righe = testo.lines();
    if !righe.by_ref().any(|r| r.trim().eq_ignore_ascii_case("[script]")) {
        return String::new();
    }
    let v: Vec<&str> = righe.map(str::trim_end).collect();
    let t = v.join("\n");
    let t = t.trim_matches('\n');
    if t.trim().is_empty() { String::new() } else { format!("{t}\n") }
}

fn bool_di(v: &str) -> bool {
    matches!(v.trim(), "1" | "true" | "TRUE" | "True" | "yes")
}

fn int_di(v: &str) -> i32 {
    v.trim().parse().unwrap_or(0)
}

/// Una data `aaaammgghhmmss` valida (o vuota).
pub fn data_valida(d: &str) -> bool {
    let d = d.trim();
    if d.is_empty() {
        return true;
    }
    if d.len() != 14 || !d.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let n = |a: usize, b: usize| d[a..b].parse::<u32>().unwrap_or(99);
    (1..=12).contains(&n(4, 6)) && (1..=31).contains(&n(6, 8)) && n(8, 10) < 24 && n(10, 12) < 60 && n(12, 14) < 60
}

/// L'istante (ms UTC) di una data `aaaammgghhmmss`, se valida e non vuota.
pub fn data_ms(d: &str) -> Option<i64> {
    let d = d.trim();
    if d.is_empty() || !data_valida(d) {
        return None;
    }
    let n = |a: usize, b: usize| d[a..b].parse::<i64>().ok();
    let giorni = crate::calendario::days_from_civil(n(0, 4)?, n(4, 6)? as u32, n(6, 8)? as u32);
    Some(((giorni * 24 + n(8, 10)?) * 60 + n(10, 12)?) * 60_000 + n(12, 14)? * 1000)
}

/// I QTC di un file senza `m_iQtc`: un contest WAE ha i QTC, in entrambe le direzioni se e' RTTY.
fn qtc_dal_nome(nome: &str, note: &str) -> i32 {
    let n = format!("{nome} {note}").to_ascii_uppercase();
    if !n.contains("WAE") {
        0
    } else if n.contains("RTTY") {
        2
    } else {
        1
    }
}

impl ContestTesto {
    /// Legge un file `.contest` (sezione `[Contest]`). Le chiavi sconosciute si ignorano.
    pub fn da_testo(id: &str, testo: &str) -> ContestTesto {
        let mut c = ContestTesto { id: id.to_string(), ..ContestTesto::default() };
        let mut qtc_letto = false;
        for (k, v) in coppie(testo, "Contest") {
            let b = || bool_di(&v);
            let i = || int_di(&v);
            match k.as_str() {
                "m_sName" => c.name = v.clone(),
                "m_sNotes" => c.notes = v.clone(),
                "m_dtStart" => c.start = v.clone(),
                "m_dtEnd" => c.end = v.clone(),
                "m_iWorkTime" => c.work_time = i(),
                "m_sNrs" => c.nrs = v.clone(),
                "n_bAutoIncNrs" => c.auto_inc = b(),
                "m_iValidContacts" => c.valid_contacts = i(),
                "m_bValidCw" => c.valid_cw = b(),
                "m_bValidPhone" => c.valid_phone = b(),
                "m_bValidDigital" => c.valid_digital = b(),
                "m_iPointsMyDxcc" => c.points_my_dxcc = i(),
                "m_iPointsMyCont" => c.points_my_cont = i(),
                "m_iPointsDx" => c.points_dx = i(),
                "m_iMultipliers" => c.multipliers = i(),
                "m_iMultipliersOption" => c.multipliers_option = i(),
                "m_bBandMult" => c.band_mult = b(),
                "m_sMacrosFilename" => c.macros = v.clone(),
                "m_bMergeMacros" => c.merge_macros = b(),
                "m_bAutoGrabNumber" => c.grab_number = b(),
                "m_iGrabNumberType" => c.grab_number_type = i(),
                "m_bAutoGrabTime" => c.grab_time = b(),
                "m_bAutoGrabState" => c.grab_state = b(),
                "m_sStateListFile" => c.domain_file = v.clone(),
                "m_iGrabDomainTo" => c.grab_domain_to = i(),
                "m_bSuggestCqZone" => c.suggest_cq_zone = b(),
                "m_bSuggestItuZone" => c.suggest_itu_zone = b(),
                "m_bSuggestDomain" => c.suggest_domain = b(),
                "m_bSuggestDxccPfx" => c.suggest_dxcc_pfx = b(),
                "m_bDupeNoBand" => c.dupe_no_band = b(),
                "m_bDupeNoMode" => c.dupe_no_mode = b(),
                "m_iLogBarDispMode" => c.log_bar_mode = i(),
                "m_iQtc" => {
                    c.qtc = i().clamp(0, 2);
                    qtc_letto = true;
                }
                _ => {
                    // m_bBand160Mult / m_iBand160Mult ...
                    for (pos, banda) in BANDE.iter().enumerate() {
                        if k == format!("m_bBand{banda}Mult") {
                            c.bands[pos].on = b();
                        } else if k == format!("m_iBand{banda}Mult") {
                            c.bands[pos].weight = i();
                        }
                    }
                }
            }
        }
        if !qtc_letto {
            c.qtc = qtc_dal_nome(&c.name, &c.notes);
        }
        c.score = coppie(testo, "Score").into_iter().map(|(k, v)| format!("{k}={v}
")).collect();
        c.script = sezione_script(testo);
        c
    }

    /// Il testo del file `.contest` (righe CRLF come i file INI di Windows; nessun dato personale).
    pub fn in_testo(&self) -> String {
        let pulito = |s: &str| s.replace(['\r', '\n'], " ");
        let b = |v: bool| if v { "1" } else { "0" };
        let mut r: Vec<String> = vec!["[Contest]".into()];
        let mut s = |k: &str, v: String| r.push(format!("{k}={v}"));
        s("m_sName", pulito(&self.name));
        s("m_sNotes", pulito(&self.notes));
        s("m_dtStart", self.start.trim().to_string());
        s("m_dtEnd", self.end.trim().to_string());
        s("m_iWorkTime", self.work_time.to_string());
        s("m_sNrs", pulito(&self.nrs));
        s("n_bAutoIncNrs", b(self.auto_inc).into());
        s("m_iValidContacts", self.valid_contacts.to_string());
        s("m_bValidCw", b(self.valid_cw).into());
        s("m_bValidPhone", b(self.valid_phone).into());
        s("m_bValidDigital", b(self.valid_digital).into());
        s("m_iPointsMyDxcc", self.points_my_dxcc.to_string());
        s("m_iPointsMyCont", self.points_my_cont.to_string());
        s("m_iPointsDx", self.points_dx.to_string());
        s("m_iMultipliers", self.multipliers.to_string());
        s("m_iMultipliersOption", self.multipliers_option.to_string());
        s("m_bBandMult", b(self.band_mult).into());
        for banda in BANDE {
            let m = self.bands.iter().find(|m| m.band == banda).cloned().unwrap_or_default();
            s(&format!("m_bBand{banda}Mult"), b(m.on).into());
            s(&format!("m_iBand{banda}Mult"), m.weight.to_string());
        }
        s("m_sMacrosFilename", pulito(&self.macros));
        s("m_bMergeMacros", b(self.merge_macros).into());
        s("m_bAutoGrabNumber", b(self.grab_number).into());
        s("m_iGrabNumberType", self.grab_number_type.to_string());
        s("m_bAutoGrabTime", b(self.grab_time).into());
        s("m_bAutoGrabState", b(self.grab_state).into());
        s("m_sStateListFile", pulito(&self.domain_file));
        s("m_iGrabDomainTo", self.grab_domain_to.to_string());
        s("m_bSuggestCqZone", b(self.suggest_cq_zone).into());
        s("m_bSuggestItuZone", b(self.suggest_itu_zone).into());
        s("m_bSuggestDomain", b(self.suggest_domain).into());
        s("m_bSuggestDxccPfx", b(self.suggest_dxcc_pfx).into());
        s("m_bDupeNoBand", b(self.dupe_no_band).into());
        s("m_bDupeNoMode", b(self.dupe_no_mode).into());
        s("m_iLogBarDispMode", self.log_bar_mode.to_string());
        s("m_iQtc", self.qtc.clamp(0, 2).to_string());
        let regole: Vec<String> = self.score.lines().map(str::trim).filter(|x| x.contains('=') && !x.starts_with('[')).map(String::from).collect();
        if !regole.is_empty() {
            r.push(String::new());
            r.push("[Score]".into());
            r.extend(regole);
        }
        if !self.script.trim().is_empty() {
            r.push(String::new());
            r.push("[Script]".into());
            r.extend(self.script.trim_end().lines().map(|l| l.trim_end().to_string()));
        }
        let mut t = r.join("\r\n");
        t.push_str("\r\n");
        t
    }

    /// Controlla i valori prima di salvare.
    pub fn controlla(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("nome mancante".into());
        }
        if !data_valida(&self.start) || !data_valida(&self.end) {
            return Err("data non valida (aaaammgghhmmss)".into());
        }
        if let (Some(a), Some(b)) = (data_ms(&self.start), data_ms(&self.end))
            && b < a
        {
            return Err("la fine viene prima dell'inizio".into());
        }
        if !self.score.trim().is_empty() {
            crate::regole_punti::RegoleScore::da_testo(&self.score).map_err(|e| format!("regole del punteggio: {e}"))?;
        }
        if !self.script.trim().is_empty() {
            crate::script::ScriptContest::compila(&self.script)?;
        }
        for f in [&self.macros, &self.domain_file] {
            if !percorso_relativo_ok(f) {
                return Err(format!("percorso non ammesso: {f}"));
            }
        }
        Ok(())
    }
}

/// Un percorso dentro la cartella dei contest: vuoto o relativo, con `/`, senza `..` ne' unita'.
pub fn percorso_relativo_ok(p: &str) -> bool {
    let p = p.trim();
    p.is_empty() || (!p.starts_with('/') && !p.starts_with('\\') && !p.contains(':') && !p.split(['/', '\\']).any(|x| x == ".."))
}

/// Un nome di file sicuro dal nome del contest (lettere, cifre, spazi, `-`, `_`, `.`; il resto diventa `_`).
pub fn id_da_nome(nome: &str) -> String {
    let s: String = nome.trim().chars().map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') { c } else { '_' }).collect();
    let s = s.trim_matches(['.', ' ']).to_string();
    if s.is_empty() { "contest".into() } else { s }
}

/// Una macro di un file `.mc`. Il colore e' `0xRRGGBB` (nel file e' un colore di Windows `0xBBGGRR`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MacroContest {
    pub name: String,
    pub label: String,
    pub text: String,
    pub color: Option<u32>,
}

/// Il testo senza `<EXEC:...>`: Aurora non avvia programmi esterni, il comando si toglie dalle macro dei contest importate.
fn senza_exec(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.to_ascii_uppercase().find("<EXEC:") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find('>').map_or("", |j| &rest[i + j + 1..]);
    }
    out.push_str(rest);
    out
}

/// Legge un file di macro (sezione `[Macros]`: `nItems`, `NameN`, `LabelN`, `TextN`, `BkColorN`).
pub fn leggi_macro(testo: &str) -> Vec<MacroContest> {
    let kv = coppie(testo, "Macros");
    let get = |k: &str| kv.iter().find(|(a, _)| a.eq_ignore_ascii_case(k)).map(|(_, v)| v.clone());
    let n = get("nItems").map(|v| int_di(&v)).unwrap_or(0).clamp(0, 500);
    let mut out = Vec::new();
    for i in 0..n {
        let Some(name) = get(&format!("Name{i}")).filter(|s| !s.is_empty()) else { continue };
        let color = get(&format!("BkColor{i}")).and_then(|v| v.trim().parse::<u32>().ok()).map(|c| ((c & 0xff) << 16) | (c & 0xff00) | ((c >> 16) & 0xff));
        out.push(MacroContest { name, label: get(&format!("Label{i}")).unwrap_or_default(), text: senza_exec(&get(&format!("Text{i}")).unwrap_or_default()), color });
    }
    out
}

/// Una voce di un elenco di domini: cio' che si riceve (`key`) e il dominio che conta (`value`), con il prefisso della sezione se c'e'.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VoceDominio {
    pub key: String,
    pub value: String,
    pub section: String,
}

/// Un elenco di domini (`.dom`): titolo facoltativo, `COUNTRY = X` per il paese a cui vale, sezioni `PREFISSO:` e righe `ricevuto = dominio`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Dominio {
    pub title: String,
    pub country: String,
    pub entries: Vec<VoceDominio>,
}

pub fn leggi_dominio(testo: &str) -> Dominio {
    let mut d = Dominio::default();
    let mut sezione = String::new();
    for riga in testo.lines() {
        let r = riga.trim();
        if r.is_empty() || r.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = r.split_once('=') {
            let (k, v) = (k.trim(), v.trim());
            if k.eq_ignore_ascii_case("COUNTRY") {
                d.country = v.to_string();
            } else if !k.is_empty() {
                d.entries.push(VoceDominio { key: k.to_uppercase(), value: if v.is_empty() { k.to_uppercase() } else { v.to_uppercase() }, section: sezione.clone() });
            }
        } else if let Some(s) = r.strip_suffix(':') {
            sezione = s.trim().to_uppercase();
        } else if d.title.is_empty() && d.entries.is_empty() {
            d.title = r.to_string();
        }
    }
    d
}

impl Dominio {
    /// Il dominio di una parola ricevuta: prima nelle sezioni il cui prefisso inizia il nominativo, poi in tutto l'elenco.
    pub fn cerca(&self, parola: &str, call: &str) -> Option<String> {
        let p = parola.trim().to_uppercase();
        let call = call.trim().to_uppercase();
        if p.is_empty() {
            return None;
        }
        let nella_sezione = self.entries.iter().find(|e| e.key == p && !e.section.is_empty() && call.starts_with(&e.section));
        nella_sezione.or_else(|| self.entries.iter().find(|e| e.key == p)).map(|e| e.value.clone())
    }
}

// ---------------------------------------------------------------- conversione

/// Il risultato della conversione di un file di contest di un altro programma.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversione {
    pub contest: ContestTesto,
    /// Il file di macro di origine da copiare (solo il nome) e il nominativo personale da togliere dalle macro.
    pub macro_origine: Option<String>,
    pub dom_origine: Option<String>,
    pub nominativo: String,
}

/// L'ultimo pezzo di un percorso Windows o Unix.
fn nome_file(p: &str) -> &str {
    p.trim().rsplit(['\\', '/']).next().unwrap_or("").trim()
}

/// Converte un file di contest (`[Contest]` con le stesse chiavi): toglie il nominativo personale e il modulo di statistiche esterno,
/// rende relativi i percorsi delle macro e dei domini. `esiste_macro` dice se un file di macro c'e' fra quelli da copiare: se quello
/// indicato manca si prova con il nome del contest e con il nome del file.
pub fn converti_cst(testo: &str, stem: &str, esiste_macro: &dyn Fn(&str) -> bool) -> Conversione {
    let mut c = ContestTesto::da_testo(&id_da_nome(stem), testo);
    let kv = coppie(testo, "Contest");
    let nominativo = kv.iter().find(|(k, _)| k == "m_sMyCall").map(|(_, v)| v.trim().to_uppercase()).unwrap_or_default();
    if c.name.trim().is_empty() {
        c.name = stem.to_string();
    }
    ripara_date(&mut c);
    let indicata = nome_file(&c.macros).to_string();
    let candidati = [indicata.clone(), format!("{}.mc", c.name.trim()), format!("{stem}.mc")];
    let macro_origine = candidati.into_iter().find(|n| !n.is_empty() && n.to_lowercase().ends_with(".mc") && esiste_macro(n));
    c.macros = macro_origine.as_ref().map(|n| format!("macros/{n}")).unwrap_or_default();
    let dom = nome_file(&c.domain_file).to_string();
    let dom_origine = if dom.is_empty() { None } else { Some(dom) };
    c.domain_file = dom_origine.as_ref().map(|n| format!("dom/{n}")).unwrap_or_default();
    Conversione { contest: c, macro_origine, dom_origine, nominativo }
}

/// Date non valide: si tolgono; una fine prima dell'inizio (anno vecchio rimasto) prende l'anno dell'inizio, o quello dopo.
fn ripara_date(c: &mut ContestTesto) {
    for d in [&mut c.start, &mut c.end] {
        if !data_valida(d) {
            d.clear();
        }
    }
    if let (Some(a), Some(b)) = (data_ms(&c.start), data_ms(&c.end))
        && b < a
    {
        let anno: i64 = c.start[..4].parse().unwrap_or(0);
        for y in [anno, anno + 1] {
            let e = format!("{y:04}{}", &c.end[4..]);
            if data_ms(&e).is_some_and(|b| b >= a) {
                c.end = e;
                break;
            }
        }
    }
}

/// Toglie da un file di macro cio' che e' personale o non funziona altrove: il nominativo (diventa `<MYCALL>`), i programmi esterni
/// (`<EXEC:...>`), i percorsi dei file audio (resta solo il nome del file). Il resto del testo delle macro non cambia.
pub fn sanifica_macro(testo: &str, nominativi: &[&str]) -> String {
    let mut out = String::with_capacity(testo.len());
    let mut resto = testo;
    while let Some(a) = resto.find('<') {
        out.push_str(&resto[..a]);
        let dopo = &resto[a..];
        let Some(b) = dopo.find('>') else {
            out.push_str(dopo);
            resto = "";
            break;
        };
        let tag = &dopo[1..b];
        let su = tag.to_uppercase();
        if su.starts_with("EXEC:") {
            // programma esterno: si toglie
        } else if let Some(arg) = su.starts_with("WAVE:").then(|| &tag[5..]) {
            out.push_str(&format!("<WAVE:{}>", nome_file(arg)));
        } else {
            out.push('<');
            out.push_str(tag);
            out.push('>');
        }
        resto = &dopo[b + 1..];
    }
    out.push_str(resto);
    let mut t = out;
    for n in nominativi.iter().map(|n| n.trim()).filter(|n| n.len() >= 3) {
        t = sostituisci_parola(&t, n, "<MYCALL>");
    }
    t
}

/// Sostituisce una parola intera (senza maiuscole/minuscole).
fn sostituisci_parola(t: &str, parola: &str, con: &str) -> String {
    let su = t.to_uppercase();
    let p = parola.to_uppercase();
    if su.len() != t.len() {
        return t.replace(parola, con);
    }
    let mut out = String::new();
    let mut i = 0;
    let bytes = su.as_bytes();
    while let Some(pos) = su[i..].find(&p) {
        let a = i + pos;
        let b = a + p.len();
        let bordo_a = a == 0 || !bytes[a - 1].is_ascii_alphanumeric();
        let bordo_b = b >= bytes.len() || !bytes[b].is_ascii_alphanumeric();
        out.push_str(&t[i..a]);
        out.push_str(if bordo_a && bordo_b { con } else { &t[a..b] });
        i = b;
    }
    out.push_str(&t[i..]);
    out
}

/// Il testo di un file che puo' essere UTF-8 o Latin-1 (i vecchi file di Windows).
pub fn testo_da_byte(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.trim_start_matches('\u{feff}').to_string(),
        Err(_) => b.iter().map(|&c| c as char).collect(),
    }
}

// ---------------------------------------------------------------- l'elenco su disco

/// L'elenco dei contest dei modi testuali nella cartella `<dati>/contest`.
#[derive(Debug, Clone)]
pub struct ElencoContest {
    dir: PathBuf,
}

/// Il contest attivo con le sue macro e il suo elenco dei domini, per la pagina dei modi testuali.
#[derive(Debug, Clone, Serialize)]
pub struct Attivo {
    pub contest: ContestTesto,
    pub macros: Vec<MacroContest>,
    pub domain: Option<Dominio>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

impl ElencoContest {
    pub fn new(dir: &Path) -> ElencoContest {
        let _ = std::fs::create_dir_all(dir.join("macros"));
        let _ = std::fs::create_dir_all(dir.join("dom"));
        ElencoContest { dir: dir.to_path_buf() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Copia i contest predefiniti (cartella delle risorse di Aurora: `*.contest`, `macros/`, `dom/`) la prima volta: nessun file
    /// dell'utente si sovrascrive e, una volta fatto, quelli cancellati non tornano. Restituisce quanti file sono stati copiati.
    pub fn copia_predefiniti(&self, da: &Path) -> usize {
        let segno = self.dir.join(FILE_PREDEFINITI);
        if segno.exists() || !da.is_dir() {
            return 0;
        }
        // una cartella delle risorse senza contest (installatore che non li aveva, percorso sbagliato): niente segno, cosi' la prima versione
        // che li porta davvero li copia ancora
        let ha_contest = std::fs::read_dir(da)
            .map(|rd| rd.flatten().any(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case(ESTENSIONE))))
            .unwrap_or(false);
        if !ha_contest {
            tracing::warn!("contest: nessun file .{ESTENSIONE} in {}: predefiniti non copiati", da.display());
            return 0;
        }
        let mut n = 0;
        for sotto in ["", "macros", "dom"] {
            let (src, dst) = (da.join(sotto), self.dir.join(sotto));
            let _ = std::fs::create_dir_all(&dst);
            let Ok(entries) = std::fs::read_dir(&src) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if !p.is_file() {
                    continue;
                }
                let Some(nome) = p.file_name() else { continue };
                let to = dst.join(nome);
                if !to.exists() && std::fs::copy(&p, &to).is_ok() {
                    n += 1;
                }
            }
        }
        if let Err(e) = std::fs::write(&segno, "1") {
            tracing::warn!("contest: segno dei predefiniti non scritto: {e}");
        }
        n
    }

    fn file(&self, id: &str) -> Result<PathBuf, String> {
        if id.trim().is_empty() || id.contains(['/', '\\', ':']) || id.contains("..") {
            return Err("identificativo non valido".into());
        }
        Ok(self.dir.join(format!("{id}.{ESTENSIONE}")))
    }

    /// Tutti i contest, in ordine di nome.
    pub fn elenco(&self) -> Vec<ContestTesto> {
        let mut out: Vec<ContestTesto> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ESTENSIONE)))
                    .filter_map(|p| {
                        let id = p.file_stem()?.to_string_lossy().to_string();
                        let b = std::fs::read(&p).ok()?;
                        Some(ContestTesto::da_testo(&id, &testo_da_byte(&b)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by_key(|c| c.name.to_lowercase());
        out
    }

    pub fn leggi(&self, id: &str) -> Option<ContestTesto> {
        let b = std::fs::read(self.file(id).ok()?).ok()?;
        Some(ContestTesto::da_testo(id, &testo_da_byte(&b)))
    }

    /// Salva un contest: con `id` vuoto e' nuovo (il nome del file viene dal nome, senza sovrascriverne un altro). Restituisce l'id.
    pub fn salva(&self, mut c: ContestTesto) -> Result<String, String> {
        c.controlla()?;
        if c.id.trim().is_empty() {
            let base = id_da_nome(&c.name);
            let mut id = base.clone();
            let mut k = 2;
            while self.file(&id)?.exists() {
                id = format!("{base} ({k})");
                k += 1;
            }
            c.id = id;
        }
        std::fs::write(self.file(&c.id)?, c.in_testo()).map_err(|e| e.to_string())?;
        Ok(c.id)
    }

    /// Cancella un contest (le sue macro restano: possono servire ad altri). Se era attivo, nessun contest resta attivo.
    pub fn elimina(&self, id: &str) -> Result<(), String> {
        std::fs::remove_file(self.file(id)?).map_err(|e| e.to_string())?;
        if self.attivo_id().as_deref() == Some(id) {
            self.seleziona(None)?;
        }
        Ok(())
    }

    pub fn attivo_id(&self) -> Option<String> {
        let id = std::fs::read_to_string(self.dir.join(FILE_ATTIVO)).ok()?.trim().to_string();
        (!id.is_empty() && self.file(&id).is_ok_and(|p| p.exists())).then_some(id)
    }

    /// Sceglie il contest attivo (`None` = «Nessun contest»).
    pub fn seleziona(&self, id: Option<&str>) -> Result<(), String> {
        let id = id.map(str::trim).filter(|s| !s.is_empty());
        if let Some(id) = id
            && !self.file(id)?.exists()
        {
            return Err("contest sconosciuto".into());
        }
        std::fs::write(self.dir.join(FILE_ATTIVO), id.unwrap_or("")).map_err(|e| e.to_string())
    }

    fn leggi_relativo(&self, rel: &str) -> Option<String> {
        if rel.trim().is_empty() || !percorso_relativo_ok(rel) {
            return None;
        }
        std::fs::read(self.dir.join(rel.trim().replace('\\', "/"))).ok().map(|b| testo_da_byte(&b))
    }

    /// Il contest attivo con le sue macro e i domini.
    pub fn attivo(&self) -> Option<Attivo> {
        let c = self.leggi(&self.attivo_id()?)?;
        let macros = self.leggi_relativo(&c.macros).map(|t| leggi_macro(&t)).unwrap_or_default();
        let domain = self.leggi_relativo(&c.domain_file).map(|t| leggi_dominio(&t));
        Some(Attivo { start_ms: data_ms(&c.start), end_ms: data_ms(&c.end), contest: c, macros, domain })
    }

    /// I file di macro e di domini presenti (per scegliere nella finestra di modifica).
    pub fn file_disponibili(&self, sotto: &str, ext: &str) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.dir.join(sotto))
            .map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).filter(|n| n.to_lowercase().ends_with(ext)).map(|n| format!("{sotto}/{n}")).collect())
            .unwrap_or_default();
        v.sort_by_key(|s| s.to_lowercase());
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CST: &str = "[Contest]\r\nm_sNrs=14\r\nm_sStateListFile=\\Contest-DOM\\ita103.dom\r\nm_bDupeNoBand=1\r\nm_sName=Prova DX Contest\r\nm_dtStart=20240607000001\r\nm_dtEnd=20240608235959\r\nm_sMacrosFilename=C:\\Users\\Qualcuno\\Dati\\Contest-Macros\\Prova.mc\r\nm_sMyCall=IU3ABC\r\nm_sStatsDllName=Statistiche.dll\r\nm_sStatsSetName=Prova\r\nm_iStatsSetNumber=3\r\nm_bBand40Mult=1\r\nm_iBand40Mult=2\r\nn_bAutoIncNrs=0\r\n";

    #[test]
    fn converte_e_sanifica() {
        let conv = converti_cst(CST, "Prova DX Contest", &|n| n == "Prova.mc");
        let t = conv.contest.in_testo();
        assert_eq!(conv.nominativo, "IU3ABC");
        assert!(!t.contains("IU3ABC") && !t.contains("MyCall") && !t.contains(".dll") && !t.contains("Stats") && !t.contains(":\\"), "{t}");
        assert!(t.contains("m_sMacrosFilename=macros/Prova.mc"));
        assert!(t.contains("m_sStateListFile=dom/ita103.dom"));
        assert_eq!(conv.macro_origine.as_deref(), Some("Prova.mc"));
        assert_eq!(conv.dom_origine.as_deref(), Some("ita103.dom"));
        // andata e ritorno
        let back = ContestTesto::da_testo(&conv.contest.id, &t);
        assert_eq!(back, conv.contest);
        assert!(back.dupe_no_band && !back.auto_inc && back.nrs == "14");
        assert_eq!(back.bands[2], MoltBanda { band: "40".into(), on: true, weight: 2 });
    }

    #[test]
    fn qtc_dal_file_o_dal_nome() {
        assert_eq!(ContestTesto::da_testo("x", "[Contest]\r\nm_sName=WAEDC\r\nm_sNotes=RTTY\r\n").qtc, 2);
        assert_eq!(ContestTesto::da_testo("x", "[Contest]\r\nm_sName=WAEDC\r\nm_sNotes=CW\r\n").qtc, 1);
        assert_eq!(ContestTesto::da_testo("x", "[Contest]\r\nm_sName=WAEDC RTTY\r\nm_iQtc=0\r\n").qtc, 0);
        assert_eq!(ContestTesto::da_testo("x", "[Contest]\r\nm_sName=Prova\r\n").qtc, 0);
        let c = ContestTesto { name: "Prova".into(), qtc: 2, ..ContestTesto::default() };
        assert_eq!(ContestTesto::da_testo("x", &c.in_testo()).qtc, 2);
    }

    #[test]
    fn macro_mancante_cercata_per_nome() {
        let conv = converti_cst(CST, "Prova DX Contest", &|n| n == "Prova DX Contest.mc");
        assert_eq!(conv.contest.macros, "macros/Prova DX Contest.mc");
        let nessuna = converti_cst(CST, "Prova DX Contest", &|_| false);
        assert_eq!(nessuna.contest.macros, "");
    }

    #[test]
    fn date_riparate() {
        let c = converti_cst("[Contest]
m_sName=X
m_dtStart=20161203200001
m_dtEnd=20131204200000
", "X", &|_| false).contest;
        assert_eq!(c.end, "20161204200000");
        let c = converti_cst("[Contest]
m_sName=X
m_dtStart=Invalid DateTime
m_dtEnd=20150519161836
", "X", &|_| false).contest;
        assert_eq!((c.start.as_str(), c.end.as_str()), ("", "20150519161836"));
        assert!(c.controlla().is_ok());
    }

    #[test]
    fn macro_ripulite() {
        let t = "Text0=<TX>CQ IU3ABC iu3abc IU3ABCD<EXEC:Plugins\\Esterno.exe><WAVE:E:\\Radio\\Onde\\cq.wav><RXANDCLEAR>";
        let s = sanifica_macro(t, &["IU3ABC"]);
        assert_eq!(s, "Text0=<TX>CQ <MYCALL> <MYCALL> IU3ABCD<WAVE:cq.wav><RXANDCLEAR>");
    }

    #[test]
    fn legge_macro_e_colori() {
        let m = leggi_macro("[Macros]\r\nnItems=2\r\nName0=F1\r\nLabel0=CQ\r\nText0=<TX>CQ<RX>\r\nName1=F2\r\nText1=x\r\nBkColor0=255\r\n");
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].color, Some(0xff0000));
        assert_eq!(m[1].label, "");
        assert_eq!(m[1].color, None);
    }

    #[test]
    fn domini_con_sezioni() {
        let d = leggi_dominio("103 province\r\n\r\nI1:\r\nTO = TO\r\nI2:\r\nMI = MI\r\nMilano = MI\r\nCOUNTRY = I\r\n");
        assert_eq!(d.title, "103 province");
        assert_eq!(d.country, "I");
        assert_eq!(d.cerca("milano", "IU2ABC"), Some("MI".into()));
        assert_eq!(d.cerca("TO", "IU3ABC"), Some("TO".into()));
        assert_eq!(d.cerca("XX", "IU3ABC"), None);
    }

    #[test]
    fn date_e_percorsi() {
        assert!(data_valida("20240607000001") && data_valida("") && !data_valida("20241307000000") && !data_valida("2024"));
        assert_eq!(data_ms("19700101000001"), Some(1000));
        assert!(percorso_relativo_ok("macros/x.mc") && !percorso_relativo_ok("C:\\x.mc") && !percorso_relativo_ok("../x") && !percorso_relativo_ok("/etc/x"));
        assert_eq!(id_da_nome("A/B: C?"), "A_B_ C_");
    }

    fn cartella(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aurora-elenco-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn elenco_salva_seleziona_elimina() {
        let d = cartella("ciclo");
        let e = ElencoContest::new(&d);
        assert!(e.elenco().is_empty());
        let id = e.salva(ContestTesto { name: "Prova RTTY".into(), macros: "macros/p.mc".into(), domain_file: "dom/p.dom".into(), start: "20240101000000".into(), end: "20240102000000".into(), ..ContestTesto::default() }).unwrap();
        assert_eq!(id, "Prova RTTY");
        let id2 = e.salva(ContestTesto { name: "Prova RTTY".into(), ..ContestTesto::default() }).unwrap();
        assert_eq!(id2, "Prova RTTY (2)");
        std::fs::write(d.join("macros/p.mc"), "[Macros]\r\nnItems=1\r\nName0=F3\r\nText0=<NRS>\r\n").unwrap();
        std::fs::write(d.join("dom/p.dom"), "TO = TO\r\n").unwrap();
        assert!(e.attivo().is_none());
        e.seleziona(Some(&id)).unwrap();
        let a = e.attivo().unwrap();
        assert_eq!(a.macros[0].name, "F3");
        assert_eq!(a.domain.unwrap().entries.len(), 1);
        assert!(a.start_ms.unwrap() < a.end_ms.unwrap());
        assert!(e.seleziona(Some("nessuno")).is_err());
        assert!(e.salva(ContestTesto { name: "x".into(), macros: "C:\\x.mc".into(), ..ContestTesto::default() }).is_err());
        assert!(e.salva(ContestTesto { name: "x".into(), start: "20240102000000".into(), end: "20240101000000".into(), ..ContestTesto::default() }).is_err());
        e.elimina(&id).unwrap();
        assert!(e.attivo_id().is_none());
        assert_eq!(e.elenco().len(), 1);
        assert!(e.leggi("../fuori").is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn predefiniti_copiati_una_volta() {
        let src = cartella("src");
        std::fs::create_dir_all(src.join("macros")).unwrap();
        std::fs::write(src.join("A.contest"), "[Contest]\r\nm_sName=A\r\n").unwrap();
        std::fs::write(src.join("macros/a.mc"), "[Macros]\r\n").unwrap();
        let d = cartella("dst");
        let e = ElencoContest::new(&d);
        std::fs::write(d.join("A.contest"), "[Contest]\r\nm_sName=Mio\r\n").unwrap();
        assert_eq!(e.copia_predefiniti(&src), 1);
        assert_eq!(e.leggi("A").unwrap().name, "Mio", "il file dell'utente resta");
        std::fs::remove_file(d.join("A.contest")).unwrap();
        assert_eq!(e.copia_predefiniti(&src), 0, "una volta sola");
        assert!(e.leggi("A").is_none());
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&src);
    }

    /// Una cartella delle risorse vuota (o senza `.contest`) non mette il segno: quando arriva quella giusta i contest si copiano.
    #[test]
    fn risorse_senza_contest_non_bloccano_la_copia_successiva() {
        let vuota = cartella("src-vuota");
        std::fs::create_dir_all(vuota.join("macros")).unwrap();
        let d = cartella("dst-vuota");
        let e = ElencoContest::new(&d);
        assert_eq!(e.copia_predefiniti(&vuota), 0);
        assert!(!d.join(FILE_PREDEFINITI).exists(), "nessun segno");
        let src = cartella("src-piena");
        std::fs::create_dir_all(src.join("dom")).unwrap();
        std::fs::write(src.join("B.contest"), "[Contest]
m_sName=B
").unwrap();
        std::fs::write(src.join("dom/b.dom"), "X = X
").unwrap();
        assert_eq!(e.copia_predefiniti(&src), 2);
        assert!(e.leggi("B").is_some());
        assert!(d.join("dom/b.dom").is_file());
        for c in [&d, &vuota, &src] {
            let _ = std::fs::remove_dir_all(c);
        }
    }

    /// I contest distribuiti con Aurora: tutti si leggono, nessun dato personale, nessun percorso assoluto, macro e domini presenti.
    #[test]
    fn predefiniti_del_repository_puliti() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/src-tauri/contest");
        let Ok(rd) = std::fs::read_dir(&dir) else { return };
        let mut n = 0;
        for p in rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == ESTENSIONE)) {
            let t = std::fs::read_to_string(&p).unwrap();
            let c = ContestTesto::da_testo("x", &t);
            assert!(c.controlla().is_ok(), "{}: {:?}", p.display(), c.controlla());
            for vietato in ["MyCall", "Stats", ".dll", ":\\", "Contest-Macros", "Contest-DOM"] {
                assert!(!t.contains(vietato), "{}: {vietato}", p.display());
            }
            for rel in [&c.macros, &c.domain_file] {
                assert!(rel.is_empty() || dir.join(rel).is_file(), "{}: {rel} manca", p.display());
            }
            n += 1;
        }
        for p in std::fs::read_dir(dir.join("macros")).unwrap().flatten().map(|e| e.path()) {
            let t = std::fs::read_to_string(&p).unwrap();
            let su = t.to_uppercase();
            assert!(!su.contains("<EXEC:") && !t.contains(":\\") && !t.contains(":/"), "{}", p.display());
            assert!(!leggi_macro(&t).is_empty(), "{}: nessuna macro", p.display());
        }
        assert!(n >= 80, "contest predefiniti: {n}");
    }

    /// Conversione di tutti i file di origine (cartella in `AURORA_CST_DIR`, con `Contest-Settings`, `Contest-Macros`, `Contest-DOM`):
    /// nessun nominativo personale, nessun percorso assoluto, andata e ritorno identica.
    #[test]
    fn converte_tutti_i_file_di_origine() {
        let Some(base) = std::env::var_os("AURORA_CST_DIR").map(PathBuf::from) else { return };
        let macro_dir = base.join("Contest-Macros");
        let mut n = 0;
        for p in std::fs::read_dir(base.join("Contest-Settings")).unwrap().flatten().map(|e| e.path()) {
            let t = testo_da_byte(&std::fs::read(&p).unwrap());
            let stem = p.file_stem().unwrap().to_string_lossy().to_string();
            let conv = converti_cst(&t, &stem, &|m| macro_dir.join(m).is_file());
            let out = conv.contest.in_testo();
            assert!(!conv.nominativo.is_empty() && !out.to_uppercase().contains(&conv.nominativo), "{stem}");
            assert!(!out.contains(":\\") && !out.contains(".dll") && !out.contains("Stats"), "{stem}");
            assert_eq!(ContestTesto::da_testo(&conv.contest.id, &out), conv.contest, "{stem}");
            if let Some(m) = &conv.macro_origine {
                let mt = sanifica_macro(&testo_da_byte(&std::fs::read(macro_dir.join(m)).unwrap()), &[&conv.nominativo]);
                assert!(!mt.to_uppercase().contains(&conv.nominativo) && !mt.to_uppercase().contains("<EXEC:"), "{m}");
            }
            n += 1;
        }
        assert_eq!(n, 135);
    }
}
