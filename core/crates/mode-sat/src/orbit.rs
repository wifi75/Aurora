//! Orbite dei satelliti: propagazione SGP4 (libreria `sgp4`, licenza MIT) e passaggio nel cielo di chi guarda.
//!
//! Dal vettore posizione e velocita' nel sistema TEME si passa a quello terrestre (rotazione di Greenwich) e poi alle coordinate
//! locali (Est, Nord, Su) dell'osservatore: azimut, elevazione, distanza e velocita' radiale (positiva = il satellite si allontana).

use std::f64::consts::PI;

/// Velocita' della luce (km/s).
pub const C_KMS: f64 = 299_792.458;
const WGS84_A: f64 = 6378.137;
const WGS84_F: f64 = 1.0 / 298.257_223_563;
/// Velocita' angolare della Terra (rad/s).
const OMEGA: f64 = 7.292_115_0e-5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observer {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

/// Un satellite con i suoi elementi orbitali (TLE).
#[derive(Clone)]
pub struct Sat {
    pub name: String,
    pub norad: u64,
    constants: sgp4::Constants,
    /// Istante dell'epoca dei dati (secondi Unix).
    pub epoch_unix: f64,
    elements: sgp4::Elements,
}

impl Sat {
    /// Da un TLE a tre righe (nome, riga 1, riga 2).
    pub fn from_tle(name: &str, line1: &str, line2: &str) -> Result<Sat, String> {
        let elements = sgp4::Elements::from_tle(Some(name.trim().to_string()), line1.trim_end().as_bytes(), line2.trim_end().as_bytes()).map_err(|e| e.to_string())?;
        let constants = sgp4::Constants::from_elements(&elements).map_err(|e| e.to_string())?;
        let epoch = elements.datetime.and_utc();
        let epoch_unix = epoch.timestamp() as f64 + f64::from(epoch.timestamp_subsec_nanos()) / 1e9;
        Ok(Sat { name: name.trim().to_string(), norad: elements.norad_id, constants, epoch_unix, elements })
    }

    /// I giri al giorno (dal TLE).
    pub fn revs_per_day(&self) -> f64 {
        self.elements.mean_motion
    }

    pub fn inclination_deg(&self) -> f64 {
        self.elements.inclination
    }

    pub fn eccentricity(&self) -> f64 {
        self.elements.eccentricity
    }

    pub fn period_min(&self) -> f64 {
        1440.0 / self.elements.mean_motion.max(1e-6)
    }

    /// Posizione e velocita' TEME (km, km/s) all'istante dato.
    fn teme(&self, unix: f64) -> Option<([f64; 3], [f64; 3])> {
        let p = self.constants.propagate(sgp4::MinutesSinceEpoch((unix - self.epoch_unix) / 60.0)).ok()?;
        Some((p.position, p.velocity))
    }
}

/// Tempo siderale di Greenwich (radianti) a un istante Unix.
pub fn gmst_rad(unix: f64) -> f64 {
    let jd = unix / 86_400.0 + 2_440_587.5;
    let d = jd - 2_451_545.0;
    let t = d / 36_525.0;
    let deg = 280.460_618_37 + 360.985_647_366_29 * d + 0.000_387_933 * t * t - t * t * t / 38_710_000.0;
    deg.rem_euclid(360.0).to_radians()
}

fn observer_ecef(o: &Observer) -> [f64; 3] {
    let lat = o.lat_deg.to_radians();
    let lon = o.lon_deg.to_radians();
    let e2 = WGS84_F * (2.0 - WGS84_F);
    let n = WGS84_A / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    let h = o.alt_m / 1000.0;
    [(n + h) * lat.cos() * lon.cos(), (n + h) * lat.cos() * lon.sin(), (n * (1.0 - e2) + h) * lat.sin()]
}

/// Latitudine e longitudine geodetiche (gradi) e quota (km) di un punto ECEF.
fn geodetic(ecef: [f64; 3]) -> (f64, f64, f64) {
    let [x, y, z] = ecef;
    let e2 = WGS84_F * (2.0 - WGS84_F);
    let p = (x * x + y * y).sqrt();
    let lon = y.atan2(x);
    let mut lat = z.atan2(p * (1.0 - e2));
    let mut h = 0.0;
    for _ in 0..6 {
        let n = WGS84_A / (1.0 - e2 * lat.sin().powi(2)).sqrt();
        h = p / lat.cos() - n;
        lat = (z / (p * (1.0 - e2 * n / (n + h)))).atan();
    }
    (lat.to_degrees(), lon.to_degrees(), h)
}

/// Quello che si vede di un satellite da un punto, a un istante.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Look {
    pub az_deg: f64,
    pub el_deg: f64,
    pub range_km: f64,
    /// Velocita' radiale (km/s): positiva se il satellite si allontana.
    pub range_rate_kms: f64,
    /// Il punto sotto il satellite.
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_km: f64,
    /// Velocita' rispetto alla Terra (km/s).
    pub speed_kms: f64,
    /// Il satellite sta salendo (`true`) o scendendo verso l'orizzonte.
    pub rising: bool,
}

/// Dove sta il satellite e come si vede dall'osservatore. `None` se la propagazione non e' possibile (dati scaduti o fuori limiti).
pub fn look(sat: &Sat, obs: &Observer, unix: f64) -> Option<Look> {
    let (r, v) = sat.teme(unix)?;
    let th = gmst_rad(unix);
    let (c, s) = (th.cos(), th.sin());
    // TEME -> terrestre
    let re = [r[0] * c + r[1] * s, -r[0] * s + r[1] * c, r[2]];
    let ve = [v[0] * c + v[1] * s + OMEGA * re[1], -v[0] * s + v[1] * c - OMEGA * re[0], v[2]];
    let o = observer_ecef(obs);
    let rel = [re[0] - o[0], re[1] - o[1], re[2] - o[2]];
    let range = (rel[0] * rel[0] + rel[1] * rel[1] + rel[2] * rel[2]).sqrt();
    let lat = obs.lat_deg.to_radians();
    let lon = obs.lon_deg.to_radians();
    let east = -lon.sin() * rel[0] + lon.cos() * rel[1];
    let north = -lat.sin() * lon.cos() * rel[0] - lat.sin() * lon.sin() * rel[1] + lat.cos() * rel[2];
    let up = lat.cos() * lon.cos() * rel[0] + lat.cos() * lon.sin() * rel[1] + lat.sin() * rel[2];
    let rate = (rel[0] * ve[0] + rel[1] * ve[1] + rel[2] * ve[2]) / range;
    let (slat, slon, salt) = geodetic(re);
    Some(Look {
        az_deg: east.atan2(north).to_degrees().rem_euclid(360.0),
        el_deg: (up / range).clamp(-1.0, 1.0).asin().to_degrees(),
        range_km: range,
        range_rate_kms: rate,
        lat_deg: slat,
        lon_deg: slon,
        alt_km: salt,
        speed_kms: (ve[0] * ve[0] + ve[1] * ve[1] + ve[2] * ve[2]).sqrt(),
        rising: rate < 0.0,
    })
}

/// Un passaggio sopra l'orizzonte (o sopra `min_el`).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Pass {
    pub aos_unix: f64,
    pub los_unix: f64,
    pub max_el_deg: f64,
    pub max_el_unix: f64,
    pub aos_az_deg: f64,
    pub los_az_deg: f64,
    pub max_az_deg: f64,
    pub duration_s: f64,
}

/// I passaggi in `hours` ore da `start_unix`, con elevazione massima almeno `min_max_el` gradi. Il passaggio gia' in corso comincia da `start_unix`.
pub fn passes(sat: &Sat, obs: &Observer, start_unix: f64, hours: f64, min_max_el: f64) -> Vec<Pass> {
    let end = start_unix + hours * 3600.0;
    let step = (sat.period_min() * 60.0 / 90.0).clamp(10.0, 30.0);
    let el_at = |t: f64| look(sat, obs, t).map_or(-90.0, |l| l.el_deg);
    let mut out = Vec::new();
    let mut t = start_unix;
    let mut prev = el_at(t);
    // se siamo gia' dentro un passaggio, parte da adesso
    let mut aos: Option<f64> = (prev > 0.0).then_some(start_unix);
    while t < end {
        let next_t = (t + step).min(end);
        let cur = el_at(next_t);
        if prev <= 0.0 && cur > 0.0 {
            aos = Some(bisect(&el_at, t, next_t, true));
        } else if prev > 0.0 && cur <= 0.0 {
            let los = bisect(&el_at, t, next_t, false);
            if let Some(a) = aos.take() {
                out.extend(describe(sat, obs, a, los, min_max_el));
            }
        }
        prev = cur;
        t = next_t;
    }
    // il passaggio che finisce dopo la fine della finestra
    if let Some(a) = aos {
        let mut los = end;
        while el_at(los) > 0.0 && los < end + 1800.0 {
            los += 30.0;
        }
        out.extend(describe(sat, obs, a, bisect(&el_at, los - 30.0, los, false), min_max_el));
    }
    out
}

/// L'istante in cui l'elevazione attraversa lo zero fra `lo` e `hi` (a un secondo).
fn bisect(el_at: &dyn Fn(f64) -> f64, mut lo: f64, mut hi: f64, rising: bool) -> f64 {
    for _ in 0..16 {
        let mid = (lo + hi) / 2.0;
        let up = el_at(mid) > 0.0;
        if up == rising {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    (lo + hi) / 2.0
}

fn describe(sat: &Sat, obs: &Observer, aos: f64, los: f64, min_max_el: f64) -> Option<Pass> {
    let mut best = (f64::MIN, aos, 0.0);
    let mut t = aos;
    while t <= los {
        if let Some(l) = look(sat, obs, t)
            && l.el_deg > best.0
        {
            best = (l.el_deg, t, l.az_deg);
        }
        t += 2.0;
    }
    if best.0 < min_max_el {
        return None;
    }
    let az = |t: f64| look(sat, obs, t).map_or(0.0, |l| l.az_deg);
    Some(Pass { aos_unix: aos, los_unix: los, max_el_deg: best.0, max_el_unix: best.1, aos_az_deg: az(aos + 1.0), los_az_deg: az(los - 1.0), max_az_deg: best.2, duration_s: los - aos })
}

/// La traccia a terra: punti (latitudine, longitudine) ogni `step_s` secondi da `start_unix` per `minutes` minuti.
pub fn ground_track(sat: &Sat, start_unix: f64, minutes: f64, step_s: f64) -> Vec<(f64, f64)> {
    let obs = Observer { lat_deg: 0.0, lon_deg: 0.0, alt_m: 0.0 };
    let mut out = Vec::new();
    let mut t = start_unix;
    while t <= start_unix + minutes * 60.0 {
        if let Some(l) = look(sat, &obs, t) {
            out.push((l.lat_deg, l.lon_deg));
        }
        t += step_s;
    }
    out
}

/// Il raggio dell'impronta radio (km sulla superficie) di un satellite a `alt_km`, con orizzonte a `min_el` gradi.
pub fn footprint_radius_km(alt_km: f64, min_el_deg: f64) -> f64 {
    let re = 6371.0;
    let el = min_el_deg.to_radians();
    let lambda = (re / (re + alt_km) * el.cos()).clamp(-1.0, 1.0).acos() - el;
    re * lambda.max(0.0)
}

/// Il Sole: latitudine e longitudine del punto in cui sta allo zenit (formule semplici, circa 0,01 gradi).
pub fn sun_subpoint(unix: f64) -> (f64, f64) {
    let d = unix / 86_400.0 + 2_440_587.5 - 2_451_545.0;
    let g = (357.529 + 0.985_600_28 * d).to_radians();
    let q = 280.459 + 0.985_647_36 * d;
    let l = (q + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();
    let eps = (23.439 - 0.000_000_36 * d).to_radians();
    let ra = (eps.cos() * l.sin()).atan2(l.cos());
    let dec = (eps.sin() * l.sin()).asin();
    let lon = (ra - gmst_rad(unix)).to_degrees();
    (dec.to_degrees(), ((lon + 180.0).rem_euclid(360.0)) - 180.0)
}

/// La Luna: latitudine e longitudine del punto in cui sta allo zenit (formule semplici, circa 1 grado).
pub fn moon_subpoint(unix: f64) -> (f64, f64) {
    let d = unix / 86_400.0 + 2_440_587.5 - 2_451_545.0;
    let l0 = (218.316 + 13.176_396 * d).to_radians();
    let m = (134.963 + 13.064_993 * d).to_radians();
    let f = (93.272 + 13.229_350 * d).to_radians();
    let lon = l0 + 6.289_f64.to_radians() * m.sin();
    let lat = 5.128_f64.to_radians() * f.sin();
    let eps = 23.439_f64.to_radians();
    let ra = (lon.sin() * eps.cos() - lat.tan() * eps.sin()).atan2(lon.cos());
    let dec = (lat.sin() * eps.cos() + lat.cos() * eps.sin() * lon.sin()).asin();
    let lon_geo = (ra - gmst_rad(unix)).to_degrees();
    (dec.to_degrees(), ((lon_geo + 180.0).rem_euclid(360.0)) - 180.0)
}

/// Il Doppler in Hz (positivo = frequenza piu' alta) di un segnale di `freq_hz` mentre il satellite ha velocita' radiale `rate_kms`.
pub fn doppler_shift_hz(freq_hz: f64, rate_kms: f64) -> f64 {
    -freq_hz * rate_kms / C_KMS
}

/// La differenza fra due angoli (gradi) nell'intervallo -180..180.
pub fn angle_diff_deg(a: f64, b: f64) -> f64 {
    (b - a + 540.0).rem_euclid(360.0) - 180.0
}

/// Il locatore Maidenhead (4, 6 o 8 caratteri) come latitudine e longitudine del suo centro.
pub fn maidenhead_to_latlon(grid: &str) -> Option<(f64, f64)> {
    let g: Vec<char> = grid.trim().chars().collect();
    if g.len() < 4 || g.len() % 2 != 0 {
        return None;
    }
    let letter = |c: char, max: u32| {
        let v = c.to_ascii_uppercase() as i32 - 'A' as i32;
        (0..max as i32).contains(&v).then_some(f64::from(v))
    };
    let digit = |c: char| c.to_digit(10).map(f64::from);
    let mut lon = -180.0 + letter(g[0], 18)? * 20.0 + digit(g[2])? * 2.0;
    let mut lat = -90.0 + letter(g[1], 18)? * 10.0 + digit(g[3])?;
    let (mut w, mut h) = (2.0, 1.0);
    if g.len() >= 6 {
        lon += letter(g[4], 24)? * (2.0 / 24.0);
        lat += letter(g[5], 24)? * (1.0 / 24.0);
        w = 2.0 / 24.0;
        h = 1.0 / 24.0;
    }
    if g.len() >= 8 {
        lon += digit(g[6])? * (2.0 / 240.0);
        lat += digit(g[7])? * (1.0 / 240.0);
        w = 2.0 / 240.0;
        h = 1.0 / 240.0;
    }
    Some((lat + h / 2.0, lon + w / 2.0))
}

#[allow(dead_code)]
const _: f64 = PI;

#[cfg(test)]
mod tests {
    use super::*;

    /// L'ISS da un TLE pubblico di prova (epoca 2008, usato negli esempi della libreria SGP4).
    const ISS: (&str, &str, &str) = (
        "ISS (ZARYA)",
        "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927",
        "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537",
    );

    fn iss() -> Sat {
        Sat::from_tle(ISS.0, ISS.1, ISS.2).expect("TLE valido")
    }

    #[test]
    fn il_tle_si_legge() {
        let s = iss();
        assert_eq!(s.norad, 25544);
        assert!((s.revs_per_day() - 15.72125391).abs() < 1e-6);
        assert!((s.inclination_deg() - 51.6416).abs() < 1e-6);
        assert!((s.period_min() - 91.6).abs() < 0.3);
    }

    #[test]
    fn la_quota_e_la_velocita_sono_quelle_di_una_orbita_bassa() {
        let s = iss();
        let obs = Observer { lat_deg: 45.0, lon_deg: 11.0, alt_m: 100.0 };
        for k in 0..20 {
            let l = look(&s, &obs, s.epoch_unix + f64::from(k) * 300.0).expect("propagazione");
            assert!((320.0..400.0).contains(&l.alt_km), "quota {}", l.alt_km);
            assert!((7.0..7.8).contains(&l.speed_kms), "velocita' {}", l.speed_kms);
            assert!(l.lat_deg.abs() <= 51.8, "la latitudine non supera l'inclinazione: {}", l.lat_deg);
        }
    }

    #[test]
    fn il_punto_sotto_il_satellite_lo_vede_allo_zenit() {
        let s = iss();
        let t = s.epoch_unix + 1800.0;
        let first = look(&s, &Observer { lat_deg: 0.0, lon_deg: 0.0, alt_m: 0.0 }, t).expect("propagazione");
        let under = Observer { lat_deg: first.lat_deg, lon_deg: first.lon_deg, alt_m: 0.0 };
        let l = look(&s, &under, t).expect("propagazione");
        assert!(l.el_deg > 89.0, "elevazione {}", l.el_deg);
        assert!((l.range_km - l.alt_km).abs() < 15.0, "distanza {} quota {}", l.range_km, l.alt_km);
    }

    #[test]
    fn i_passaggi_hanno_senso() {
        let s = iss();
        let obs = Observer { lat_deg: 45.0, lon_deg: 11.0, alt_m: 100.0 };
        let list = passes(&s, &obs, s.epoch_unix, 24.0, 5.0);
        assert!(list.len() >= 3, "in 24 ore ci sono diversi passaggi: {}", list.len());
        for p in &list {
            assert!(p.los_unix > p.aos_unix);
            assert!((60.0..1000.0).contains(&p.duration_s), "durata {}", p.duration_s);
            assert!((5.0..=90.0).contains(&p.max_el_deg));
            assert!(p.max_el_unix >= p.aos_unix && p.max_el_unix <= p.los_unix);
            // all'inizio e alla fine l'elevazione e' vicina a zero
            let at_aos = look(&s, &obs, p.aos_unix + 0.5).expect("a").el_deg;
            let at_los = look(&s, &obs, p.los_unix - 0.5).expect("l").el_deg;
            assert!(at_aos.abs() < 1.0 && at_los.abs() < 1.0, "aos {at_aos} los {at_los}");
        }
        // in ordine di tempo
        assert!(list.windows(2).all(|w| w[0].los_unix <= w[1].aos_unix));
    }

    #[test]
    fn il_doppler_ha_il_segno_giusto() {
        // il satellite si avvicina (velocita' radiale negativa): la frequenza sale
        assert!(doppler_shift_hz(435_000_000.0, -7.0) > 0.0);
        assert!(doppler_shift_hz(435_000_000.0, 7.0) < 0.0);
        // a 7 km/s su 435 MHz: circa 10 kHz
        assert!((doppler_shift_hz(435_000_000.0, -7.0) - 10_155.0).abs() < 50.0);
        assert_eq!(doppler_shift_hz(145_000_000.0, 0.0), 0.0);
    }

    #[test]
    fn la_velocita_radiale_cambia_segno_al_passaggio() {
        let s = iss();
        let obs = Observer { lat_deg: 45.0, lon_deg: 11.0, alt_m: 100.0 };
        let p = passes(&s, &obs, s.epoch_unix, 24.0, 20.0).into_iter().next().expect("un passaggio alto");
        let early = look(&s, &obs, p.aos_unix + 20.0).expect("a");
        let late = look(&s, &obs, p.los_unix - 20.0).expect("l");
        assert!(early.range_rate_kms < 0.0, "all'inizio si avvicina: {}", early.range_rate_kms);
        assert!(late.range_rate_kms > 0.0, "alla fine si allontana: {}", late.range_rate_kms);
        assert!(early.rising && !late.rising);
    }

    #[test]
    fn l_impronta_radio_di_una_orbita_bassa() {
        // a 400 km l'orizzonte e' a circa 2250 km
        let r = footprint_radius_km(400.0, 0.0);
        assert!((2200.0..2300.0).contains(&r), "{r}");
        assert!(footprint_radius_km(400.0, 10.0) < r);
    }

    #[test]
    fn il_locatore_diventa_coordinate() {
        // IO91 e' vicino a Londra
        let (lat, lon) = maidenhead_to_latlon("IO91").expect("locatore");
        assert!((lat - 51.5).abs() < 0.6 && (lon - -1.0).abs() < 1.1, "{lat} {lon}");
        let (lat, lon) = maidenhead_to_latlon("io91wm").expect("locatore");
        assert!((lat - 51.52).abs() < 0.05 && (lon - -0.125).abs() < 0.05, "{lat} {lon}");
        assert!(maidenhead_to_latlon("ZZ99").is_none());
        assert!(maidenhead_to_latlon("IO9").is_none());
    }

    #[test]
    fn la_differenza_di_angoli_passa_per_nord() {
        assert!((angle_diff_deg(350.0, 10.0) - 20.0).abs() < 1e-9);
        assert!((angle_diff_deg(10.0, 350.0) + 20.0).abs() < 1e-9);
        assert_eq!(angle_diff_deg(90.0, 90.0), 0.0);
    }

    #[test]
    fn il_sole_sta_dove_deve() {
        // al solstizio di giugno il Sole e' sul tropico del Cancro, a mezzogiorno UTC sul meridiano zero
        let june = 1_782_043_200.0; // 2026-06-21 12:00 UTC
        let (lat, lon) = sun_subpoint(june);
        assert!((lat - 23.4).abs() < 0.5, "latitudine {lat}");
        assert!(lon.abs() < 3.0, "longitudine {lon}");
        let (mlat, _mlon) = moon_subpoint(june);
        assert!(mlat.abs() < 30.0);
    }
}
