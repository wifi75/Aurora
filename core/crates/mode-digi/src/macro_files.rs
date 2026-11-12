//! Importa le macro da file di testo (`.mc`, `.ini`): un elenco `[Macros]` con `nItems` e, per ogni voce, `Name`, `Label`, `Text` e `BkColor`.
//! Si legge solo dalla cartella indicata dall'utente, al momento. Il colore e' un valore di Windows (blu, verde,
//! rosso dal byte piu' alto): qui diventa 0xRRGGBB.

use std::path::{Path, PathBuf};

use crate::service::{DigiError, DigiMacro};

/// I file di macro per modo: nome del file, modo di Aurora a cui vanno.
pub const MODE_FILES: [(&str, &str); 3] = [("bpsk31.mc", "psk31"), ("bpsk63.mc", "psk63"), ("rtty.mc", "rtty")];

/// I file dell'insieme di base, in ordine di preferenza (`Default.mc` per primo).
pub const DEFAULT_FILES: [&str; 2] = ["Default.mc", "macros.ini"];

/// I byte di un file di macro come testo: sono in Windows-1252, che per le lettere accentate coincide con Latin-1.
fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// Windows (`0x00BBGGRR`) -> `0xRRGGBB`.
fn rgb_of(colorref: u32) -> u32 {
    let r = colorref & 0xff;
    let g = (colorref >> 8) & 0xff;
    let b = (colorref >> 16) & 0xff;
    (r << 16) | (g << 8) | b
}

/// Le macro di un file. Le voci senza nome o senza testo e etichetta si saltano.
pub fn parse_macros(bytes: &[u8]) -> Vec<DigiMacro> {
    let text = text_of(bytes);
    let mut in_macros = false;
    let mut items: std::collections::BTreeMap<usize, DigiMacro> = std::collections::BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_macros = section.eq_ignore_ascii_case("Macros");
            continue;
        }
        if !in_macros {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (field, index) = key.trim().split_at(key.trim().trim_end_matches(|c: char| c.is_ascii_digit()).len());
        let Ok(i) = index.parse::<usize>() else { continue };
        let entry = items.entry(i).or_default();
        match field.to_ascii_lowercase().as_str() {
            "name" => entry.name = value.trim().to_string(),
            "label" => entry.label = value.to_string(),
            "text" => entry.text = value.to_string(),
            "bkcolor" => entry.color = value.trim().parse::<u32>().ok().map(rgb_of),
            _ => {}
        }
    }
    items.into_values().filter(|m| !m.name.is_empty() && !(m.label.trim().is_empty() && m.text.trim().is_empty())).collect()
}

/// Quello che si e' importato da una cartella.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
pub struct Imported {
    pub folder: String,
    /// Macro dell'insieme di base e il file da cui vengono.
    pub default_count: usize,
    pub default_file: Option<String>,
    /// Macro per modo (`psk31`, `psk63`, `rtty`).
    pub modes: std::collections::BTreeMap<String, usize>,
    /// Voci che Aurora non puo' tenere (nome di tasto o evento che non conosce).
    pub skipped: usize,
}

/// Le macro trovate in una cartella: base e per modo. Se la cartella non esiste o non ha nessun file di macro e' un errore.
pub fn read_folder(folder: &Path) -> Result<(Vec<DigiMacro>, std::collections::BTreeMap<String, Vec<DigiMacro>>, Imported), DigiError> {
    if !folder.is_dir() {
        return Err(DigiError::BadValue(format!("cartella non trovata: {}", folder.display())));
    }
    let mut report = Imported { folder: folder.display().to_string(), ..Imported::default() };
    let mut base = Vec::new();
    for file in DEFAULT_FILES {
        if let Ok(bytes) = std::fs::read(folder.join(file)) {
            base = parse_macros(&bytes);
            report.default_file = Some(file.to_string());
            break;
        }
    }
    let mut modes = std::collections::BTreeMap::new();
    for (file, mode) in MODE_FILES {
        if let Ok(bytes) = std::fs::read(folder.join(file)) {
            modes.insert(mode.to_string(), parse_macros(&bytes));
        }
    }
    if report.default_file.is_none() && modes.is_empty() {
        return Err(DigiError::BadValue("in quella cartella non ci sono file di macro".into()));
    }
    // si tengono solo i nomi che Aurora conosce
    let before = base.len() + modes.values().map(Vec::len).sum::<usize>();
    base.retain(|m| crate::service::valid_macro_name(&m.name));
    for set in modes.values_mut() {
        set.retain(|m| crate::service::valid_macro_name(&m.name));
    }
    // un tasto compare una volta sola: se il file lo ripete vale l'ultima voce
    dedupe_by_name(&mut base);
    for set in modes.values_mut() {
        dedupe_by_name(set);
    }
    let after = base.len() + modes.values().map(Vec::len).sum::<usize>();
    report.skipped = before - after;
    report.default_count = base.len();
    report.modes = modes.iter().map(|(k, v)| (k.clone(), v.len())).collect();
    Ok((base, modes, report))
}

fn dedupe_by_name(list: &mut Vec<DigiMacro>) {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<DigiMacro> = Vec::with_capacity(list.len());
    for m in list.drain(..).rev() {
        if seen.insert(m.name.clone()) {
            out.push(m);
        }
    }
    out.reverse();
    *list = out;
}

/// L'opzione «Merge» per modo (`BPSK31-Merge`, `BPSK63-Merge`, `RTTY-Merge`, `CW-Merge` nella sezione `[Macros]` di un file `.ini` della cartella):
/// 1 = le macro del modo si uniscono a quelle di base, 0 = le sostituiscono. Chiave: il modo di Aurora.
pub fn read_merge_flags(folder: &Path) -> std::collections::BTreeMap<String, bool> {
    let mut flags = std::collections::BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(folder) else { return flags };
    let mut inis: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ini"))).collect();
    inis.sort();
    for path in inis {
        if let Ok(bytes) = std::fs::read(&path) {
            merge_flags_of(&bytes, &mut flags);
        }
    }
    flags
}

fn merge_flags_of(bytes: &[u8], flags: &mut std::collections::BTreeMap<String, bool>) {
    let text = text_of(bytes);
    let mut in_macros = false;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r').trim();
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_macros = section.eq_ignore_ascii_case("Macros");
            continue;
        }
        if !in_macros {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let mode = match key.trim().to_ascii_uppercase().as_str() {
            "BPSK31-MERGE" => "psk31",
            "BPSK63-MERGE" => "psk63",
            "RTTY-MERGE" => "rtty",
            "CW-MERGE" => "cw",
            _ => continue,
        };
        flags.insert(mode.to_string(), value.trim() != "0");
    }
}

/// La cartella dei file di testo delle macro (`<TEXTFILE:nome>`): `%APPDATA%\Aurora\testi`.
pub fn text_folder() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Aurora").join("testi"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[OPTIONS]\r\nSearchCall=0\r\n[Macros]\r\nnItems=3\r\n\r\nName0=F1\r\nLabel0=AutoCQ\r\nText0=<AUTOCQ>\r\nBkColor0=255\r\n\r\nName1=Ctrl-F10\r\nLabel1=PSK31\r\nText1=<BAUDRATE:31.25>\r\nBkColor1=16777088\r\nName2=F9\r\nLabel2=\r\nText2=\r\n";

    #[test]
    fn legge_nomi_etichette_testi_e_colori() {
        let m = parse_macros(SAMPLE.as_bytes());
        assert_eq!(m.len(), 2, "la voce vuota si salta");
        assert_eq!(m[0].name, "F1");
        assert_eq!(m[0].label, "AutoCQ");
        assert_eq!(m[0].text, "<AUTOCQ>");
        // il rosso di Windows (255) e' 0xFF0000
        assert_eq!(m[0].color, Some(0xFF0000));
        // 16777088 = 0xFFFF80 in Windows (blu FF, verde FF, rosso 80) = 0x80FFFF
        assert_eq!(m[1].color, Some(0x80FFFF));
    }

    #[test]
    fn l_opzione_merge_si_legge_per_modo() {
        let dir = std::env::temp_dir().join("aurora-prova-macro-merge");
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        std::fs::write(dir.join("Opzioni.ini"), "[Other]\r\nRTTY-Merge=0\r\n[Macros]\r\nRTTY-Merge=1\r\nCW-Merge=0\r\nBPSK31-Merge=1\r\nSSB-Merge=0\r\n").expect("file");
        let flags = read_merge_flags(&dir);
        assert_eq!(flags.get("rtty"), Some(&true));
        assert_eq!(flags.get("cw"), Some(&false));
        assert_eq!(flags.get("psk31"), Some(&true));
        assert_eq!(flags.len(), 3, "gli altri modi non sono di Aurora");
        assert!(read_merge_flags(&dir.join("non-esiste")).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn un_tasto_ripetuto_vale_una_volta_sola() {
        let mut list = vec![
            DigiMacro { name: "F3".into(), label: "73".into(), text: "a".into(), color: None },
            DigiMacro { name: "F4".into(), label: "x".into(), text: "b".into(), color: None },
            DigiMacro { name: "F3".into(), label: "INFO".into(), text: "c".into(), color: None },
        ];
        dedupe_by_name(&mut list);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "F4");
        assert_eq!(list[1].label, "INFO");
    }

    #[test]
    fn le_lettere_accentate_si_conservano() {
        let bytes = b"[Macros]\r\nName0=F2\r\nLabel0=Citt\xe0\r\nText0=<TX>ciao\r\n";
        assert_eq!(parse_macros(bytes)[0].label, "Citt\u{e0}");
    }

    #[test]
    fn una_cartella_senza_file_di_macro_e_un_errore() {
        let dir = std::env::temp_dir().join("aurora-prova-macro-vuota");
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        assert!(read_folder(&dir).is_err());
        assert!(read_folder(&dir.join("non-esiste")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn importa_base_e_modi_da_una_cartella() {
        let dir = std::env::temp_dir().join("aurora-prova-macro");
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        std::fs::write(dir.join("Default.mc"), SAMPLE).expect("file");
        std::fs::write(dir.join("rtty.mc"), "[Macros]\r\nName0=F10\r\nLabel0=BAUD45\r\nText0=<BAUDRATE:45.45>\r\nBkColor0=0\r\nName1=Strano\r\nLabel1=x\r\nText1=y\r\n").expect("file");
        let (base, modes, report) = read_folder(&dir).expect("importazione");
        assert_eq!(base.len(), 2);
        assert_eq!(modes["rtty"].len(), 1);
        assert_eq!(report.default_file.as_deref(), Some("Default.mc"));
        assert_eq!(report.skipped, 1, "il nome «Strano» non e' un tasto");
        std::fs::remove_dir_all(&dir).ok();
    }
}
