//! I file delle macro: le macro dei modi testuali nella cartella `macro/` dei dati (un file `.macro` per gruppo: `comuni.macro` e uno per
//! modo), la cartella dei file di testo che la macro `<TEXTFILE:nome>` manda, e il riepilogo che risponde a «Ripristina le macro di Aurora».
//!
//! Il formato e' lo stesso dei file di macro dei contest (`[Macros]`, `NameN`/`LabelN`/`TextN`), con il colore leggibile `ColorN=#RRGGBB`
//! (si legge anche `BkColorN`, il colore di Windows in decimale `0xBBGGRR`). E' descritto in `docs/macro-file.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::service::DigiMacro;

/// La cartella delle macro dentro la cartella dei dati.
pub const MACRO_DIR: &str = "macro";
/// Il file delle macro comuni a tutti i modi.
pub const COMMON_FILE: &str = "comuni.macro";
/// L'estensione dei file di macro.
pub const EXTENSION: &str = "macro";

/// Il riepilogo di un ripristino delle macro (vuoto: le macro di Aurora non vengono da file).
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
pub struct Imported {
    pub folder: String,
    pub default_count: usize,
    pub default_file: Option<String>,
    pub modes: std::collections::BTreeMap<String, usize>,
    pub skipped: usize,
}

/// La cartella dei file di testo delle macro (`<TEXTFILE:nome>`): `%APPDATA%\Aurora\testi`.
pub fn text_folder() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Aurora").join("testi"))
}

/// Un colore di Windows (`0xBBGGRR`) come `0xRRGGBB`.
pub fn from_windows_color(c: u32) -> u32 {
    ((c & 0xff) << 16) | (c & 0xff00) | ((c >> 16) & 0xff)
}

/// Le coppie chiave = valore della sezione `[Macros]` (le righe fuori sezione valgono come dentro, se il file non ha intestazione).
fn pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut inside = true;
    for line in text.lines() {
        let line = line.trim_start_matches('\u{feff}').trim_end_matches('\r');
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            inside = t[1..t.len() - 1].trim().eq_ignore_ascii_case("Macros");
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.push((k.trim().to_string(), v.to_string()));
        }
    }
    out
}

/// Separa `Name12` in (`Name`, 12).
fn key_index(k: &str) -> Option<(String, usize)> {
    let at = k.find(|c: char| c.is_ascii_digit())?;
    let (name, n) = k.split_at(at);
    Some((name.to_ascii_lowercase(), n.parse().ok()?))
}

fn parse_color(v: &str, windows: bool) -> Option<u32> {
    let v = v.trim();
    if let Some(hex) = v.strip_prefix('#') {
        return u32::from_str_radix(hex, 16).ok().filter(|c| *c <= 0xFF_FFFF);
    }
    let n = v.parse::<u32>().ok()?;
    Some(if windows { from_windows_color(n) } else { n }).filter(|c| *c <= 0xFF_FFFF)
}

/// Il testo di una macro senza i comandi `<EXEC:...>`: Aurora non avvia programmi esterni (i vecchi file `.bat` di copia di sicurezza
/// li sostituisce la copia automatica all'avvio), quindi il comando si toglie dal file invece di restare li' ignorato.
pub fn senza_exec(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.to_ascii_uppercase().find("<EXEC:") {
        out.push_str(&rest[..i]);
        match rest[i..].find('>') {
            Some(j) => rest = &rest[i + j + 1..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Toglie `<EXEC:...>` da un elenco di macro; vero se ha cambiato qualcosa.
pub fn togli_exec(list: &mut [DigiMacro]) -> bool {
    let mut changed = false;
    for m in list.iter_mut() {
        let t = senza_exec(&m.text);
        if t != m.text {
            m.text = t;
            changed = true;
        }
    }
    changed
}

/// Legge un file di macro. Contano le voci numerate che hanno un nome, in ordine di numero; `nItems` non limita (file scritti a mano spesso
/// lo sbagliano). Per lo stesso numero vale la prima riga.
pub fn parse(text: &str) -> Vec<DigiMacro> {
    let mut by_index: BTreeMap<usize, DigiMacro> = BTreeMap::new();
    let mut seen: std::collections::HashSet<(String, usize)> = std::collections::HashSet::new();
    for (k, v) in pairs(text) {
        let Some((field, i)) = key_index(&k) else { continue };
        if !seen.insert((field.clone(), i)) {
            continue;
        }
        let m = by_index.entry(i).or_default();
        match field.as_str() {
            "name" => m.name = v.trim().to_string(),
            "label" => m.label = v.trim().to_string(),
            "text" => m.text = v,
            "color" => m.color = parse_color(&v, false).or(m.color),
            "bkcolor" => {
                if m.color.is_none() {
                    m.color = parse_color(&v, true);
                }
            }
            _ => {}
        }
    }
    by_index.into_values().filter(|m| !m.name.is_empty()).collect()
}

/// Un valore su una riga sola: gli a capo scritti nel testo diventano i comandi `<CR>` e `<LF>`.
fn one_line(s: &str) -> String {
    s.replace("\r\n", "<CR><LF>").replace('\r', "<CR>").replace('\n', "<LF>")
}

/// Scrive un file di macro: un'intestazione di commento, `[Macros]`, `nItems` e le voci numerate.
pub fn render(title: &str, macros: &[DigiMacro]) -> String {
    let mut s = String::new();
    s.push_str(&format!("; Aurora - macro: {title}\n"));
    s.push_str("; Una voce per macro: NameN (tasto o evento), LabelN (scritta del pulsante), TextN (testo), ColorN (#RRGGBB, facoltativo).\n");
    s.push_str("; Si puo' modificare a mano con Aurora chiusa. Formato: docs/macro-file.md\n\n");
    s.push_str("[Macros]\n");
    s.push_str(&format!("nItems={}\n", macros.len()));
    for (i, m) in macros.iter().enumerate() {
        s.push('\n');
        s.push_str(&format!("Name{i}={}\n", one_line(m.name.trim())));
        s.push_str(&format!("Label{i}={}\n", one_line(m.label.trim())));
        s.push_str(&format!("Text{i}={}\n", one_line(&m.text)));
        if let Some(c) = m.color {
            s.push_str(&format!("Color{i}=#{:06X}\n", c & 0xFF_FFFF));
        }
    }
    s
}

/// Il file di un modo (`psk31.macro`...).
pub fn mode_file(mode_id: &str) -> String {
    format!("{mode_id}.{EXTENSION}")
}

/// Le macro lette dalla cartella: `None` se manca `comuni.macro` (cartella mai scritta: si migra da `digi.json`). Per i modi contano i file
/// presenti dei modi indicati.
pub fn load_dir(dir: &Path, modes: &[&str]) -> Option<(Vec<DigiMacro>, BTreeMap<String, Vec<DigiMacro>>)> {
    let common = std::fs::read(dir.join(COMMON_FILE)).ok()?;
    let common = parse(&String::from_utf8_lossy(&common));
    let mut per_mode = BTreeMap::new();
    for id in modes {
        if let Ok(b) = std::fs::read(dir.join(mode_file(id))) {
            per_mode.insert((*id).to_string(), parse(&String::from_utf8_lossy(&b)));
        }
    }
    Some((common, per_mode))
}

/// Scrive un file passando da uno temporaneo, cosi' un'interruzione non lascia un file a meta'.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// Scrive le macro nella cartella: `comuni.macro` e il file di ogni modo che ha le sue macro; il file di un modo senza voce si toglie.
pub fn save_dir(dir: &Path, common: &[DigiMacro], per_mode: &BTreeMap<String, Vec<DigiMacro>>, modes: &[&str]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    write_atomic(&dir.join(COMMON_FILE), &render("comuni a tutti i modi", common))?;
    for id in modes {
        let path = dir.join(mode_file(id));
        match per_mode.get(*id) {
            Some(list) => write_atomic(&path, &render(&format!("solo {id}"), list))?,
            None => {
                if path.exists() {
                    std::fs::remove_file(&path)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(name: &str, label: &str, text: &str, color: Option<u32>) -> DigiMacro {
        DigiMacro { name: name.into(), label: label.into(), text: text.into(), color }
    }

    #[test]
    fn exec_tolto_dal_testo() {
        assert_eq!(senza_exec("CQ <exec:Backup.bat>DE IU3ABC<EXEC:x.bat>"), "CQ DE IU3ABC");
        assert_eq!(senza_exec("niente"), "niente");
        assert_eq!(senza_exec("A <EXEC:rotto"), "A ");
        let mut l = vec![m("F1", "CQ", "<EXEC:Backup.bat>", None), m("F2", "x", "TU", None)];
        assert!(togli_exec(&mut l));
        assert_eq!(l[0].text, "");
        assert!(!togli_exec(&mut l));
    }

    #[test]
    fn scrive_e_rilegge_le_stesse_macro() {
        let list = vec![m("F2", "CQ", "<TX>CQ DE <MYCALL> K<RX>", Some(0xFFFF80)), m("Ctrl-F5", "Call", "<CALL>", None), m("OnTX", "*", "riga 1\r\nriga 2", Some(0x102030))];
        let back = parse(&render("prova", &list));
        assert_eq!(back.len(), 3);
        assert_eq!(back[0], list[0]);
        assert_eq!(back[1], list[1]);
        assert_eq!(back[2].text, "riga 1<CR><LF>riga 2", "gli a capo diventano comandi");
        assert_eq!(back[2].color, Some(0x102030));
    }

    #[test]
    fn legge_il_formato_dei_contest_con_il_colore_di_windows_e_nitems_sbagliato() {
        let t = "[Macros]\r\nnItems=1\r\nName0=F1\r\nLabel0=AutoCQ\r\nText0=<AUTOCQ>\r\nBkColor0=255\r\n\r\nName3=F4\r\nText3=x\r\nName=F9\r\n[Altro]\r\nName5=F5\r\n";
        let l = parse(t);
        assert_eq!(l.len(), 2, "{l:?}");
        assert_eq!(l[0].color, Some(0xFF0000), "rosso di Windows (0x0000FF) diventa 0xFF0000");
        assert_eq!(l[1].name, "F4");
    }

    #[test]
    fn la_cartella_si_salva_e_si_rilegge_e_il_modo_tolto_sparisce() {
        let dir = std::env::temp_dir().join(format!("aurora-macro-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let modes = ["psk31", "rtty"];
        assert!(load_dir(&dir, &modes).is_none(), "cartella vuota: niente");
        let mut per = BTreeMap::new();
        per.insert("rtty".to_string(), vec![m("F10", "45", "<BAUDRATE:45.45>", None)]);
        per.insert("psk31".to_string(), vec![]);
        save_dir(&dir, &[m("F1", "A", "<AUTOCQ>", None)], &per, &modes).expect("salva");
        let (c, p) = load_dir(&dir, &modes).expect("rilegge");
        assert_eq!(c.len(), 1);
        assert_eq!(p.get("rtty").map(Vec::len), Some(1));
        assert_eq!(p.get("psk31").map(Vec::len), Some(0));
        per.remove("psk31");
        save_dir(&dir, &c, &per, &modes).expect("salva");
        assert!(!dir.join("psk31.macro").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
