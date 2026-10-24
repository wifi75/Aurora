//! Modifica delle impostazioni di JTDX nel suo `JTDX.ini`. Si scrive solo a JTDX chiuso: JTDX legge il file all'avvio
//! e lo riscrive uscendo, quindi un cambio fatto mentre e' acceso andrebbe perso. Ogni voce e' controllata contro la
//! tabella `FIELDS` (nome, tipo e valori ammessi): nulla di cio' che non e' elencato si puo' toccare.

use std::{collections::BTreeMap, path::Path};

use crate::ini::FIELDS;

/// Il valore ammesso per una voce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Text,
    Num,
    /// Uno tra questi valori.
    Choice(&'static [&'static str]),
}

pub fn kind_of(key: &str) -> Option<Kind> {
    let code = FIELDS.iter().find(|(k, _)| *k == key).map(|(_, c)| *c)?;
    Some(match code {
        "bool" => Kind::Bool,
        "text" => Kind::Text,
        "num" => Kind::Num,
        "enum:save" | "enum:reply" => Kind::Choice(&["0", "1", "2"]),
        "enum:depth" => Kind::Choice(&["1", "2", "3"]),
        _ => return None,
    })
}

/// Controlla un valore prima di scriverlo.
pub fn check_value(key: &str, value: &str) -> Result<(), String> {
    let kind = kind_of(key).ok_or_else(|| format!("{key}: voce non modificabile"))?;
    if value.len() > 200 || value.chars().any(char::is_control) {
        return Err(format!("{key}: valore non valido"));
    }
    match kind {
        Kind::Bool if matches!(value, "true" | "false") => Ok(()),
        Kind::Bool => Err(format!("{key}: deve essere true o false")),
        Kind::Num if !value.is_empty() && value.len() <= 9 && value.chars().all(|c| c.is_ascii_digit() || c == '-') && value.parse::<i64>().is_ok() => Ok(()),
        Kind::Num => Err(format!("{key}: deve essere un numero intero")),
        Kind::Choice(allowed) if allowed.contains(&value) => Ok(()),
        Kind::Choice(_) => Err(format!("{key}: valore non ammesso")),
        // il testo non puo' contenere virgolette ne' barre rovesciate: QSettings le tratterebbe come escape
        Kind::Text if !value.contains(['"', '\\']) => Ok(()),
        Kind::Text => Err(format!("{key}: caratteri non ammessi")),
    }
}

/// QSettings mette tra virgolette i testi con virgole o punti e virgola.
fn encode(value: &str, was_quoted: bool) -> String {
    if was_quoted || value.contains([',', ';']) || value.starts_with(' ') || value.ends_with(' ') {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

/// Applica le modifiche al testo di un `JTDX.ini` (voci nelle sezioni `Common` e `Configuration`); restituisce il testo nuovo.
/// Le voci assenti si aggiungono in `Configuration`. Righe, fine riga e tutto il resto restano come sono.
pub fn edit_text(text: &str, changes: &BTreeMap<String, String>) -> Result<String, String> {
    for (key, value) in changes {
        check_value(key, value)?;
    }
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l).to_string()).collect();
    let mut done = std::collections::BTreeSet::new();
    let mut section = String::new();
    let mut configuration_at = None;
    for (index, line) in lines.iter_mut().enumerate() {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_string();
            if section == "Configuration" && configuration_at.is_none() {
                configuration_at = Some(index);
            }
            continue;
        }
        if section != "Common" && section != "Configuration" {
            continue;
        }
        let replacement = {
            let Some((key, old)) = line.split_once('=') else { continue };
            let key = key.trim();
            changes.get(key).map(|new| (key.to_string(), format!("{key}={}", encode(new, old.trim().starts_with('"')))))
        };
        if let Some((key, new_line)) = replacement {
            *line = new_line;
            done.insert(key);
        }
    }
    let missing: Vec<&String> = changes.keys().filter(|k| !done.contains(*k)).collect();
    if !missing.is_empty() {
        let at = configuration_at.ok_or_else(|| "sezione Configuration non trovata: il file non sembra di JTDX".to_string())?;
        for (offset, key) in missing.iter().enumerate() {
            lines.insert(at + 1 + offset, format!("{key}={}", encode(&changes[*key], false)));
        }
    }
    Ok(lines.join(newline))
}

/// Salva una copia (`JTDX.ini.bak-aurora`) e riscrive il file con le modifiche. Va chiamata a JTDX chiuso.
pub fn apply_changes(path: &Path, changes: &BTreeMap<String, String>) -> Result<(), String> {
    if changes.is_empty() {
        return Ok(());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let edited = edit_text(&text, changes)?;
    let backup = path.with_file_name("JTDX.ini.bak-aurora");
    std::fs::write(&backup, &text).map_err(|e| format!("copia di sicurezza: {e}"))?;
    let tmp = path.with_extension("ini.tmp");
    std::fs::write(&tmp, edited).and_then(|()| std::fs::rename(&tmp, path)).map_err(|e| format!("scrittura: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI: &str = "[MainWindow]\r\nMyCall=NON-TOCCARE\r\n[Common]\r\nNDepth=3\r\nSWLMode=false\r\n[Configuration]\r\nMyCall=AA1AA\r\nMyGrid=FN42\r\nSoundInName=Microfono (2- USB)\r\nPTTMethod=@Variant(\\0\\0)\r\nUDPServerPort=2334\r\n[LogQSO]\r\nMyCall=FUORI\r\n";

    fn change(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn changes_only_the_asked_values_in_the_right_sections_and_keeps_the_rest() {
        let out = edit_text(INI, &change(&[("MyCall", "IU3ABC"), ("NDepth", "2"), ("SWLMode", "true")])).unwrap();
        assert!(out.contains("[Configuration]\r\nMyCall=IU3ABC\r\n"), "{out}");
        assert!(out.contains("[Common]\r\nNDepth=2\r\nSWLMode=true\r\n"));
        assert!(out.contains("[MainWindow]\r\nMyCall=NON-TOCCARE\r\n"), "altra sezione");
        assert!(out.contains("[LogQSO]\r\nMyCall=FUORI\r\n"), "altra sezione");
        assert!(out.contains("PTTMethod=@Variant(\\0\\0)\r\n"), "i blocchi binari restano");
        assert!(out.ends_with("MyCall=FUORI\r\n"), "stesso fine riga e stessa coda");
    }

    #[test]
    fn adds_a_missing_value_to_the_configuration_section() {
        let out = edit_text(INI, &change(&[("AcceptUDPRequests", "true")])).unwrap();
        assert!(out.contains("[Configuration]\r\nAcceptUDPRequests=true\r\nMyCall=AA1AA"), "{out}");
    }

    #[test]
    fn keeps_unix_line_endings_and_quotes_what_needs_quotes() {
        let out = edit_text("[Configuration]\nSoundInName=Uno\n", &change(&[("SoundInName", "Mic, 2")])).unwrap();
        assert_eq!(out, "[Configuration]\nSoundInName=\"Mic, 2\"\n");
        let quoted = edit_text("[Configuration]\nSoundInName=\"Uno\"\n", &change(&[("SoundInName", "Due")])).unwrap();
        assert_eq!(quoted, "[Configuration]\nSoundInName=\"Due\"\n");
    }

    #[test]
    fn refuses_unknown_keys_and_bad_values() {
        for (k, v) in [
            ("EQSLPasswd", "x"),
            ("PTTMethod", "PTT_method_VOX"),
            ("SWLMode", "si"),
            ("NDepth", "9"),
            ("NFT8Cycles", "uno"),
            ("MyCall", "A\r\nB=C"),
            ("MyCall", "A\"B"),
            ("MyCall", "A\\B"),
            ("MyCall", &"x".repeat(201)),
        ] {
            assert!(edit_text(INI, &change(&[(k, v)])).is_err(), "{k}={v}");
        }
        assert!(edit_text("[Common]\nNDepth=3\n", &change(&[("MyCall", "AA1AA")])).is_err(), "senza sezione Configuration");
    }

    #[test]
    fn every_shown_value_has_a_kind() {
        for key in crate::ini::SHOWN {
            assert!(kind_of(key).is_some(), "{key}");
        }
    }

    #[test]
    fn writes_the_file_and_keeps_a_copy() {
        let dir = std::env::temp_dir().join(format!("aurora-ini-edit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("JTDX.ini");
        std::fs::write(&path, INI).unwrap();
        apply_changes(&path, &change(&[("MyGrid", "IO91wm")])).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("MyGrid=IO91wm"));
        assert_eq!(std::fs::read_to_string(dir.join("JTDX.ini.bak-aurora")).unwrap(), INI, "la copia e' il file di prima");
        assert!(apply_changes(&dir.join("non-esiste.ini"), &change(&[("MyGrid", "X")])).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
