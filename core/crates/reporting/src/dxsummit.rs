//! Spot di una stazione su DXSummit: lo fa JTDX quando si preme il pulsante «DX Call» (se `AllowSpotsDXSummit` e' acceso), non a ogni decodifica.
//!
//! `MainWindow::on_pbSpotDXCall_clicked` (sorgente e `jtdx.exe` 2.2.159): una richiesta GET a `http://www.dxsummit.fi/SendSpot.aspx` con `callSign` (la
//! propria stazione), `dxCallSign`, `frequency` (kHz con un decimale, radio piu' tono di ricezione) e `info` (il modo e il testo scelto); attesa massima 10 s.

use std::time::Duration;

/// Indirizzo di DXSummit (quello di JTDX).
pub const URL: &str = "http://www.dxsummit.fi/SendSpot.aspx";
pub const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpotRequest {
    /// La propria stazione (`callSign`).
    pub my_call: String,
    /// La stazione segnalata (`dxCallSign`).
    pub dx_call: String,
    /// Frequenza reale in Hz.
    pub freq_hz: u64,
    /// Campo `info`: il modo (`FT8`) e, se c'e', un testo.
    pub info: String,
}


/// I valori con cui si espandono le macro del testo dello spot (`#G`, `#D`, `#R`, `#H`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Macros<'a> {
    /// Il proprio locatore e quello del DX (`#G` = `proprio<propagazione>DX`).
    pub my_grid: &'a str,
    pub dx_grid: &'a str,
    /// Distanza in km (`#D` = `1234km`); zero o meno = non nota, la macro sparisce.
    pub distance_km: i32,
    /// Il rapporto impostato (`#R` = `-12dB`); vuoto = la macro sparisce.
    pub report: &'a str,
}

/// Sostituisce ogni `pattern` (ASCII) senza badare alle maiuscole (`QString::replace(.., Qt::CaseInsensitive)`).
fn replace_ci(text: &str, pattern: &str, with: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text.get(i..i + pattern.len()).is_some_and(|s| s.eq_ignore_ascii_case(pattern)) {
            out.push_str(with);
            i += pattern.len();
        } else if let Some(c) = text[i..].chars().next() {
            out.push(c);
            i += c.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// Il campo `info` come lo compone `on_spotLineEdit_textChanged` di JTDX (l'anteprima sotto il testo dello spot, senza il suo prefisso «info: »):
/// il modo, poi, se il testo non ha `#G`, la propagazione, poi il testo con le macro espanse. Come JTDX, con testo vuoto e propagazione
/// vuota resta il solo modo; con testo non vuoto e senza propagazione restano i due spazi del sorgente.
pub fn compose_info(mode: &str, text: &str, prop: &str, macros: Macros<'_>) -> String {
    let (mode, prop) = (mode.trim(), prop.trim());
    if text.is_empty() {
        return if prop.is_empty() { mode.to_string() } else { format!("{mode} {prop}") };
    }
    let grids = format!("{}<{prop}>{}", macros.my_grid, macros.dx_grid);
    let distance = if macros.distance_km > 0 { format!("{}km", macros.distance_km) } else { String::new() };
    let report = if macros.report.is_empty() { String::new() } else { format!("{}dB", macros.report) };
    let mut expanded = replace_ci(text, "#G", &grids);
    expanded = replace_ci(&expanded, "#D", &distance);
    expanded = replace_ci(&expanded, "#R", &report);
    // `#H` e' la sigla Fox/Hound (F/H): Aurora non ha quella modalita'
    expanded = expanded.replace("#H", "");
    if text.to_ascii_uppercase().contains("#G") { format!("{mode} {expanded}") } else { format!("{mode} {prop} {expanded}") }
}

/// La frequenza come la scrive JTDX: kHz con un decimale (`QString::number(f, 'f', 1)`).
pub fn frequency_khz(freq_hz: u64) -> String {
    format!("{:.1}", freq_hz as f64 / 1000.0)
}

/// Manda lo spot (bloccante: si chiama da un thread a parte).
pub fn send(url: &str, req: &SpotRequest, timeout: Duration) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).build().into();
    let response = agent
        .get(url)
        .query("callSign", &req.my_call)
        .query("dxCallSign", &req.dx_call)
        .query("frequency", frequency_khz(req.freq_hz))
        .query("info", &req.info)
        .call()
        .map_err(|e| crate::eqsl::describe(&e))?;
    let status = response.status().as_u16();
    if (200..300).contains(&status) { Ok(()) } else { Err(format!("DXSummit ha risposto {status}")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frequency_is_in_khz_with_one_decimal() {
        assert_eq!(frequency_khz(14_075_084), "14075.1");
        assert_eq!(frequency_khz(7_074_000), "7074.0");
    }

    #[test]
    fn the_info_follows_the_spot_line_of_jtdx() {
        let m = Macros { my_grid: "JN45", dx_grid: "JO62", distance_km: 812, report: "-12" };
        // niente testo, niente propagazione: il solo modo; con la propagazione, modo e propagazione
        assert_eq!(compose_info("FT8", "", "", m), "FT8");
        assert_eq!(compose_info("FT8", "", "Es", m), "FT8 Es");
        // testo semplice senza #G: modo, propagazione (vuota: due spazi come nel sorgente) e testo
        assert_eq!(compose_info("FT8", "bella apertura", "", m), "FT8  bella apertura");
        assert_eq!(compose_info("FT4", "bella apertura", "Es", m), "FT4 Es bella apertura");
        // macro, anche in minuscolo; con #G la propagazione sta dentro i locatori e non si ripete
        assert_eq!(compose_info("FT8", "#g #d #r", "Es", m), "FT8 JN45<Es>JO62 812km -12dB");
        assert_eq!(compose_info("FT8", "#D", "", m), "FT8  812km");
        // dati non noti: le macro spariscono
        assert_eq!(compose_info("FT8", "#D #R", "", Macros::default()), "FT8   ");
        assert_eq!(compose_info("FT8", "x#Hy", "", m), "FT8  xy");
    }

}
