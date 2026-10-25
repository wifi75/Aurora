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
}
