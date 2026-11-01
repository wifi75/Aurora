//! Le DXpedition annunciate, dal calendario di NG3K (`ng3k.com/Misc/adxo.html`): data di inizio e fine, paese, nominativo,
//! QSL e note. La pagina e' HTML semplice, una riga `<tr class="adxoitem">` per attivita'.

use serde::Serialize;

pub const URL: &str = "https://www.ng3k.com/Misc/adxo.html";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dxpedition {
    /// Inizio e fine come `AAAA-MM-GG`.
    pub start: String,
    pub end: String,
    pub country: String,
    pub call: String,
    pub qsl: String,
    pub info: String,
}

/// Toglie i tag, unisce gli spazi e decodifica le poche entita' che la pagina usa.
fn text_of(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out.replace("&amp;", "&").replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// «2026 Oct01» → «2026-10-01».
fn iso_date(text: &str) -> Option<String> {
    let t = text.trim();
    let (year, rest) = t.split_once(' ')?;
    if year.len() != 4 || !year.chars().all(|c| c.is_ascii_digit()) || rest.len() < 5 {
        return None;
    }
    let month = match rest.get(..3)? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let day: u32 = rest.get(3..)?.trim().parse().ok()?;
    Some(format!("{year}-{month:02}-{day:02}"))
}

/// Le celle `<td ...>...</td>` di una riga, nell'ordine.
fn cells(row: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(start) = rest.find("<td") {
        let after = &rest[start..];
        let Some(open_end) = after.find('>') else { break };
        let body = &after[open_end + 1..];
        let Some(close) = body.find("</td>") else { break };
        out.push(&body[..close]);
        rest = &body[close + 5..];
    }
    out
}

/// Le DXpedition della pagina, nell'ordine in cui compaiono (per data di inizio).
pub fn parse(html: &str) -> Vec<Dxpedition> {
    let mut out = Vec::new();
    for chunk in html.split("<tr class=\"adxoitem\"").skip(1) {
        let row = chunk.split("</tr>").next().unwrap_or(chunk);
        let c = cells(row);
        if c.len() < 7 {
            continue;
        }
        let (Some(start), Some(end)) = (iso_date(&text_of(c[0])), iso_date(&text_of(c[1]))) else { continue };
        // la cella del nominativo ha anche il collegamento «[spots]»
        let call = text_of(c[3]).replace("[spots]", "").trim().to_string();
        if call.is_empty() {
            continue;
        }
        out.push(Dxpedition { start, end, country: text_of(c[2]), call, qsl: text_of(c[4]), info: text_of(c[6]) });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"<tr class="adxoitem" bgcolor="#FFDAB9"><td class="date">2026 Oct03</td><td class="date">2026 Nov20</td><td class="cty">Solomon Is</td><td><span class="call"><a href="https://www.qrz.com/db/H44MS">H44MS</a></span><br><span class="spots">
  <a href="http://x">[spots]</a>
  </span></td><td class="qsl">LoTW</td><td class="rep"><a href="http://x">OPDX</a><br>20260922</td><td class="info">By DL2GAC; 80-10m; mainly FT8, some SSB; QSL via DL2GAC (B&amp;d)</td></tr>
<tr class="adxoitem"><td class="date">bad</td><td>x</td><td>x</td><td>x</td><td>x</td><td>x</td><td>x</td></tr>"##;

    #[test]
    fn reads_the_ng3k_rows() {
        let list = parse(SAMPLE);
        assert_eq!(list.len(), 1);
        let d = &list[0];
        assert_eq!(d.start, "2026-10-03");
        assert_eq!(d.end, "2026-11-20");
        assert_eq!(d.country, "Solomon Is");
        assert_eq!(d.call, "H44MS");
        assert_eq!(d.qsl, "LoTW");
        assert!(d.info.contains("mainly FT8") && d.info.contains("B&d"));
    }

    #[test]
    fn converts_dates() {
        assert_eq!(iso_date("2026 Oct01").as_deref(), Some("2026-10-01"));
        assert_eq!(iso_date("2027 Jan15").as_deref(), Some("2027-01-15"));
        assert_eq!(iso_date("Oct01"), None);
    }
}
