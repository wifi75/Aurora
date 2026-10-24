//! Condizioni di propagazione da hamqsl.com (`solarxml.php`, di N0NBH): flusso solare, indici A e K, condizioni calcolate per banda.

use serde::Serialize;

use crate::xml;

pub const URL: &str = "https://www.hamqsl.com/solarxml.php";

/// Condizione calcolata di un gruppo di bande, di giorno o di notte.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BandCondition {
    /// Per esempio «30m-20m».
    pub name: String,
    /// `day` o `night`.
    pub time: String,
    /// `Good`, `Fair`, `Poor`...
    pub condition: String,
}

/// Un fenomeno VHF (E-Skip, aurora) con la sua zona e lo stato.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Phenomenon {
    pub name: String,
    pub location: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Solar {
    /// Quando il servizio ha aggiornato i dati, come lo scrive lui.
    pub updated: String,
    pub sfi: Option<u32>,
    pub a_index: Option<u32>,
    pub k_index: Option<u32>,
    pub xray: String,
    pub sunspots: Option<u32>,
    pub aurora: Option<u32>,
    pub solar_wind: Option<f32>,
    pub magnetic_field: Option<f32>,
    pub geomag: String,
    pub signal_noise: String,
    pub muf: String,
    pub bands: Vec<BandCondition>,
    pub vhf: Vec<Phenomenon>,
}

fn number<T: std::str::FromStr>(xml: &str, name: &str) -> Option<T> {
    xml::element_text(xml, name)?.trim().parse().ok()
}

fn text(xml: &str, name: &str) -> String {
    xml::element_text(xml, name).unwrap_or_default()
}

/// Legge la risposta; `None` se non e' il formato atteso (pagina d'errore, servizio non raggiungibile).
pub fn parse(body: &str) -> Option<Solar> {
    if !body.contains("<solardata>") {
        return None;
    }
    let bands = xml::elements(body, "band")
        .into_iter()
        .filter_map(|(a, condition)| Some(BandCondition { name: a.get("name")?.clone(), time: a.get("time")?.clone(), condition }))
        .collect();
    let vhf = xml::elements(body, "phenomenon")
        .into_iter()
        .filter_map(|(a, status)| Some(Phenomenon { name: a.get("name")?.clone(), location: a.get("location").cloned().unwrap_or_default(), status }))
        .collect();
    Some(Solar {
        updated: text(body, "updated"),
        sfi: number(body, "solarflux"),
        a_index: number(body, "aindex"),
        k_index: number(body, "kindex"),
        xray: text(body, "xray"),
        sunspots: number(body, "sunspots"),
        aurora: number(body, "aurora"),
        solar_wind: number(body, "solarwind"),
        magnetic_field: number(body, "magneticfield"),
        geomag: text(body, "geomagfield"),
        signal_noise: text(body, "signalnoise"),
        muf: text(body, "muf"),
        bands,
        vhf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<solar>
	<solardata>
		<source url="http://www.hamqsl.com/solar.html">N0NBH</source>
		<updated> 24 Oct 2026 1758 GMT</updated>
		<solarflux>101</solarflux>
		<aindex> 11</aindex>
		<kindex> 1</kindex>
		<xray>B4.3</xray>
		<sunspots>25</sunspots>
		<aurora> 2</aurora>
		<solarwind>408.7</solarwind>
		<magneticfield> -1.1</magneticfield>
		<calculatedconditions>
			<band name="80m-40m" time="day">Fair</band>
			<band name="30m-20m" time="night">Good</band>
		</calculatedconditions>
		<calculatedvhfconditions>
			<phenomenon name="E-Skip" location="europe">Band Closed</phenomenon>
		</calculatedvhfconditions>
		<geomagfield>VR QUIET</geomagfield>
		<signalnoise>S0-S1</signalnoise>
		<fof2></fof2>
		<muf>NoRpt</muf>
	</solardata>
</solar>"#;

    #[test]
    fn reads_the_indices_and_the_conditions() {
        let s = parse(SAMPLE).expect("formato atteso");
        assert_eq!((s.sfi, s.a_index, s.k_index, s.sunspots, s.aurora), (Some(101), Some(11), Some(1), Some(25), Some(2)));
        assert_eq!((s.xray.as_str(), s.geomag.as_str(), s.signal_noise.as_str(), s.muf.as_str(), s.updated.as_str()), ("B4.3", "VR QUIET", "S0-S1", "NoRpt", "24 Oct 2026 1758 GMT"));
        assert_eq!((s.solar_wind, s.magnetic_field), (Some(408.7), Some(-1.1)));
        assert_eq!(s.bands, vec![
            BandCondition { name: "80m-40m".into(), time: "day".into(), condition: "Fair".into() },
            BandCondition { name: "30m-20m".into(), time: "night".into(), condition: "Good".into() },
        ]);
        assert_eq!(s.vhf, vec![Phenomenon { name: "E-Skip".into(), location: "europe".into(), status: "Band Closed".into() }]);
    }

    #[test]
    fn a_missing_or_empty_value_is_none_not_an_error() {
        let s = parse("<solar><solardata><solarflux></solarflux><aindex>n/a</aindex></solardata></solar>").expect("c'e' solardata");
        assert_eq!((s.sfi, s.a_index, s.k_index), (None, None, None));
        assert!(s.bands.is_empty());
    }

    #[test]
    fn anything_else_is_refused() {
        assert!(parse("").is_none());
        assert!(parse("<html>Service Unavailable</html>").is_none());
    }
}
