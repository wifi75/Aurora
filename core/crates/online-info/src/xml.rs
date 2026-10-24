//! Lettura minima dell'XML dei due servizi: attributi dei tag e testo degli elementi. Non e' un parser generale: basta ai formati
//! di PSK Reporter e hamqsl.com, che sono piatti.

use std::collections::HashMap;

fn decode_entities(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    raw.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Gli attributi `nome="valore"` (o con apici singoli) di un tag; i valori hanno le entita' sciolte.
pub fn attributes(tag_body: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let bytes = tag_body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // nome
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let name = &tag_body[start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let Some(&quote) = bytes.get(i) else { break };
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        i += 1;
        let value_start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        if !name.is_empty() {
            out.insert(name.to_string(), decode_entities(&tag_body[value_start..i.min(bytes.len())]));
        }
        i += 1;
    }
    out
}

/// Il corpo (dopo il nome) di ogni tag `<name ...>` o `<name .../>` nel testo.
pub fn tags<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    let open = format!("<{name}");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find(&open) {
        let after = &rest[at + open.len()..];
        // dev'essere proprio quel nome, non uno che comincia allo stesso modo
        if !after.starts_with(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/') {
            rest = after;
            continue;
        }
        let mut in_quote: Option<char> = None;
        let mut end = None;
        for (idx, c) in after.char_indices() {
            match (in_quote, c) {
                (Some(q), c) if c == q => in_quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => in_quote = Some(c),
                (None, '>') => {
                    end = Some(idx);
                    break;
                }
                _ => {}
            }
        }
        let Some(end) = end else { break };
        out.push(&after[..end]);
        rest = &after[end..];
    }
    out
}

/// Il testo dentro `<name>...</name>`, senza spazi ai lati; `None` se manca.
pub fn element_text(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(decode_entities(xml[start..end].trim()))
}

/// Gli elementi `<name attr...>testo</name>` con i loro attributi e il testo.
pub fn elements(xml: &str, name: &str) -> Vec<(HashMap<String, String>, String)> {
    let close = format!("</{name}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(tag) = tags(rest, name).first().copied() {
        let at = rest.find(tag).unwrap_or(0);
        let body_end = at + tag.len() + 1;
        let Some(after) = rest.get(body_end..) else { break };
        let Some(close_at) = after.find(&close) else { break };
        out.push((attributes(tag), decode_entities(after[..close_at].trim())));
        rest = &after[close_at + close.len()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_attributes_with_both_quotes_and_entities() {
        let a = attributes(r#" call="W1AW" note='a &amp; b' snr="-12" empty="" "#);
        assert_eq!((a["call"].as_str(), a["note"].as_str(), a["snr"].as_str(), a["empty"].as_str()), ("W1AW", "a & b", "-12", ""));
    }

    #[test]
    fn a_value_may_contain_a_closing_bracket() {
        let found = tags(r#"<x a="1>2" b="3"/><x a="4"/>"#, "x");
        assert_eq!(found.len(), 2);
        assert_eq!(attributes(found[0])["a"], "1>2");
    }

    #[test]
    fn tags_do_not_match_a_longer_name() {
        assert_eq!(tags("<receptionReports><receptionReport a=\"1\"/>", "receptionReport").len(), 1);
    }

    #[test]
    fn reads_the_text_of_an_element() {
        assert_eq!(element_text("<solar><solarflux> 101 </solarflux><aindex></aindex></solar>", "solarflux").as_deref(), Some("101"));
        assert_eq!(element_text("<solar><aindex></aindex></solar>", "aindex").as_deref(), Some(""));
        assert_eq!(element_text("<solar></solar>", "kindex"), None);
    }

    #[test]
    fn reads_elements_with_attributes_and_text() {
        let xml = r#"<c><band name="80m-40m" time="day">Fair</band><band name="30m-20m" time="night">Good</band></c>"#;
        let bands = elements(xml, "band");
        assert_eq!(bands.len(), 2);
        assert_eq!((bands[1].0["name"].as_str(), bands[1].0["time"].as_str(), bands[1].1.as_str()), ("30m-20m", "night", "Good"));
    }

    #[test]
    fn truncated_input_does_not_panic() {
        assert!(tags("<receptionReport a=\"1", "receptionReport").is_empty());
        assert!(elements("<band name=\"x\">Fair", "band").is_empty());
        assert!(attributes("a=").is_empty());
    }
}
