//! La Luna vista dalla stazione e il Doppler dell'eco (EME), come la finestra «Astro» di QMAP (WSJT-X 3, GPL 3): `astro.f90`,
//! `astro0.f90`, `moondop.f90`, `moon2.f90`, `dcoord.f90`, `geocentric.f90`, `toxyz.f90`, `grid2deg.f90` della cartella `qmap/libqmap`.
//!
//! QMAP calcola ogni secondo il Doppler della propria eco (`ndop00`, «SelfDop») per il centro della fetta in MHz interi e lo usa per
//! la frequenza «sked» di ogni messaggio (`q65b.f90`: `fsked = frx - ndop00/2`), cioe' dove trasmette chi corregge da se' il proprio
//! Doppler. Il locatore e' quello della stazione nelle impostazioni: qui arriva solo come argomento, non si salva.

use serde::Serialize;

const RAD: f64 = 57.295_779_513_082_3;
/// Il valore di `twopi` scritto in `moondop.f90` (6,2831**0**53...): tenuto uguale per dare gli stessi numeri di QMAP.
const TWOPI_MOONDOP: f64 = 6.283_105_307_179_59;
const TWOPI: f64 = std::f64::consts::TAU;
/// Velocita' della luce in km/s.
const C_KMS: f64 = 2.997_924_58e5;

/// Posizione della Luna e Doppler dell'eco per la stazione.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MoonInfo {
    /// Azimut ed elevazione topocentrici, in gradi.
    pub az_deg: f64,
    pub el_deg: f64,
    /// Doppler della propria eco (andata e ritorno) in Hz alla frequenza usata (`ndop00` di QMAP, arrotondato).
    pub self_doppler_hz: i32,
    /// Frequenza usata per il Doppler, in MHz interi come QMAP (`nfreq=(int)fcenter`).
    pub freq_mhz: u32,
    /// Ritardo dell'eco in secondi (`techo`).
    pub echo_delay_s: f64,
    /// Declinazione topocentrica della Luna, in gradi.
    pub dec_deg: f64,
}

/// Locatore Maidenhead (4 o 6 caratteri) -> (longitudine OVEST, latitudine) in gradi, come `grid2deg.f90`; `None` se non valido.
pub fn grid2deg(grid: &str) -> Option<(f64, f64)> {
    let g: Vec<u8> = grid.trim().bytes().collect();
    if g.len() < 4 {
        return None;
    }
    let (g1, g2) = (g[0].to_ascii_uppercase(), g[1].to_ascii_uppercase());
    if !(b'A'..=b'R').contains(&g1) || !(b'A'..=b'R').contains(&g2) || !g[2].is_ascii_digit() || !g[3].is_ascii_digit() {
        return None;
    }
    // senza i caratteri 5 e 6 (o non lettere) il centro del quadrato: «mm»
    let (g5, g6) = match (g.get(4).map(u8::to_ascii_lowercase), g.get(5).map(u8::to_ascii_lowercase)) {
        (Some(a @ b'a'..=b'x'), Some(b @ b'a'..=b'x')) => (a, b),
        _ => (b'm', b'm'),
    };
    let nlong = 180.0 - 20.0 * f64::from(g1 - b'A');
    let n20d = 2.0 * f64::from(g[2] - b'0');
    let xminlong = 5.0 * (f64::from(g5 - b'a') + 0.5);
    let dlong = nlong - n20d - xminlong / 60.0;
    let nlat = -90.0 + 10.0 * f64::from(g2 - b'A') + f64::from(g[3] - b'0');
    let xminlat = 2.5 * (f64::from(g6 - b'a') + 0.5);
    Some((dlong, nlat + xminlat / 60.0))
}

/// `dcoord.f90`: rotazione fra sistemi di coordinate sferiche.
fn dcoord(a0: f64, b0: f64, ap: f64, bp: f64, a1: f64, b1: f64) -> (f64, f64) {
    let (sb0, cb0, sbp, cbp, sb1, cb1) = (b0.sin(), b0.cos(), bp.sin(), bp.cos(), b1.sin(), b1.cos());
    let sb2 = sbp * sb1 + cbp * cb1 * (ap - a1).cos();
    let cb2 = (1.0 - sb2 * sb2).sqrt();
    let b2 = (sb2 / cb2).atan();
    let saa = (ap - a1).sin() * cb1 / cb2;
    let caa = (sb1 - sb2 * sbp) / (cb2 * cbp);
    let cbb = sb0 / cbp;
    let sbb = (ap - a0).sin() * cb0;
    let sa2 = saa * cbb - caa * sbb;
    let ca2 = caa * cbb + saa * sbb;
    let ta2o2 = if ca2 <= 0.0 { (1.0 - ca2) / sa2 } else { sa2 / (1.0 + ca2) };
    let mut a2 = 2.0 * ta2o2.atan();
    if a2 < 0.0 {
        a2 += std::f64::consts::TAU;
    }
    (a2, b2)
}

/// `geocentric.f90`: latitudine geocentrica e distanza dal centro della Terra (km) di un punto a quota `elev` metri.
fn geocentric(alat: f64, elev: f64) -> (f64, f64) {
    let f = 1.0 / 298.257;
    let a = 6_378_140.0;
    let c = 1.0 / (1.0 + (-2.0 + f) * f * alat.sin() * alat.sin()).sqrt();
    let arcf = (a * c + elev) * alat.cos();
    let arsf = (a * (1.0 - f) * (1.0 - f) * c + elev) * alat.sin();
    (arsf.atan2(arcf), 0.001 * (arcf * arcf + arsf * arsf).sqrt())
}

fn toxyz(alpha: f64, delta: f64, r: f64) -> [f64; 3] {
    [r * delta.cos() * alpha.cos(), r * delta.cos() * alpha.sin(), r * delta.sin()]
}

/// Posizione della Luna (`moon2.f90`).
struct Moon {
    ra: f64,
    dec: f64,
    top_dec: f64,
    lst: f64,
    az: f64,
    el: f64,
    dist: f64,
}

/// Giorni dal 31/12/1999 0h (con la frazione dell'ora), con le divisioni intere di Fortran come in `moon2.f90`.
fn day_number(y: i64, m: i64, day: i64, ut: f64) -> f64 {
    (367 * y - 7 * (y + (m + 9) / 12) / 4 + 275 * m / 9 + day - 730_530) as f64 + ut / 24.0
}

/// `moon2.f90`: coordinate della Luna per `lon` EST e `lat` in gradi.
fn moon2(y: i64, m: i64, day: i64, ut: f64, lon: f64, lat: f64) -> Moon {
    let r_ = |x: f64| x / RAD;
    let d = day_number(y, m, day, ut);
    let ecl = 23.4393 - 3.563e-7 * d;
    let nn = 125.1228 - 0.052_953_808_3 * d;
    let i = 5.1454;
    let w = (318.0634 + 0.164_357_322_3 * d + 360_000.0).rem_euclid(360.0);
    let a = 60.2666;
    let e = 0.054_900;
    let mm = (115.3654 + 13.064_992_950_9 * d + 360_000.0).rem_euclid(360.0);

    let mut ee = mm + e * RAD * r_(mm).sin() * (1.0 + e * r_(mm).cos());
    ee -= (ee - e * RAD * r_(ee).sin() - mm) / (1.0 - e * r_(ee).cos());
    ee -= (ee - e * RAD * r_(ee).sin() - mm) / (1.0 - e * r_(ee).cos());
    let xv = a * (r_(ee).cos() - e);
    let yv = a * ((1.0 - e * e).sqrt() * r_(ee).sin());
    let v = (RAD * yv.atan2(xv) + 720.0).rem_euclid(360.0);
    let r = (xv * xv + yv * yv).sqrt();

    let xg = r * (r_(nn).cos() * r_(v + w).cos() - r_(nn).sin() * r_(v + w).sin() * r_(i).cos());
    let yg = r * (r_(nn).sin() * r_(v + w).cos() + r_(nn).cos() * r_(v + w).sin() * r_(i).cos());
    let zg = r * (r_(v + w).sin() * r_(i).sin());
    let mut lonecl = (RAD * (yg / RAD).atan2(xg / RAD) + 720.0).rem_euclid(360.0);
    let mut latecl = RAD * (zg / RAD).atan2((xg * xg + yg * yg).sqrt() / RAD);

    // perturbazioni
    let ms = (356.0470 + 0.985_600_258_5 * d + 3_600_000.0).rem_euclid(360.0);
    let ws = 282.9404 + 4.70935e-5 * d;
    let ls = (ms + ws + 720.0).rem_euclid(360.0);
    let lm = (mm + w + nn + 720.0).rem_euclid(360.0);
    let dd = (lm - ls + 360.0).rem_euclid(360.0);
    let ff = (lm - nn + 360.0).rem_euclid(360.0);
    lonecl = lonecl - 1.274 * r_(mm - 2.0 * dd).sin() + 0.658 * r_(2.0 * dd).sin() - 0.186 * r_(ms).sin() - 0.059 * r_(2.0 * mm - 2.0 * dd).sin() - 0.057 * r_(mm - 2.0 * dd + ms).sin()
        + 0.053 * r_(mm + 2.0 * dd).sin()
        + 0.046 * r_(2.0 * dd - ms).sin()
        + 0.041 * r_(mm - ms).sin()
        - 0.035 * r_(dd).sin()
        - 0.031 * r_(mm + ms).sin()
        - 0.015 * r_(2.0 * ff - 2.0 * dd).sin()
        + 0.011 * r_(mm - 4.0 * dd).sin();
    latecl = latecl - 0.173 * r_(ff - 2.0 * dd).sin() - 0.055 * r_(mm - ff - 2.0 * dd).sin() - 0.046 * r_(mm + ff - 2.0 * dd).sin() + 0.033 * r_(ff + 2.0 * dd).sin() + 0.017 * r_(2.0 * mm + ff).sin();
    let r = 60.36298 - 3.27746 * r_(mm).cos() - 0.57994 * r_(mm - 2.0 * dd).cos() - 0.46357 * r_(2.0 * dd).cos() - 0.08904 * r_(2.0 * mm).cos() + 0.03865 * r_(2.0 * mm - 2.0 * dd).cos()
        - 0.03237 * r_(2.0 * dd - ms).cos()
        - 0.02688 * r_(mm + 2.0 * dd).cos()
        - 0.02358 * r_(mm - 2.0 * dd + ms).cos()
        - 0.02030 * r_(mm - ms).cos()
        + 0.01719 * r_(dd).cos()
        + 0.01671 * r_(mm + ms).cos();
    let dist = r * 6378.140;

    let xg = r * r_(lonecl).cos() * r_(latecl).cos();
    let yg = r * r_(lonecl).sin() * r_(latecl).cos();
    let zg = r * r_(latecl).sin();
    let xe = xg;
    let ye = yg * r_(ecl).cos() - zg * r_(ecl).sin();
    let ze = yg * r_(ecl).sin() + zg * r_(ecl).cos();
    let ra = (RAD * ye.atan2(xe) + 360.0).rem_euclid(360.0);
    let dec = RAD * ze.atan2((xe * xe + ye * ye).sqrt());

    // topocentriche
    let mpar = RAD * (1.0 / r).asin();
    let gclat = lat - 0.1924 * r_(2.0 * lat).sin();
    let rho = 0.99883 + 0.00167 * r_(2.0 * lat).cos();
    let gmst0 = (ls + 180.0) / 15.0;
    let lst = (gmst0 + ut + lon / 15.0 + 48.0).rem_euclid(24.0);
    let ha = 15.0 * lst - ra;
    let g = RAD * (r_(gclat).tan() / r_(ha).cos()).atan();
    let top_ra = ra - mpar * rho * r_(gclat).cos() * r_(ha).sin() / r_(dec).cos();
    let top_dec = dec - mpar * rho * r_(gclat).sin() * r_(g - dec).sin() / r_(g).sin();
    let mut ha = 15.0 * lst - top_ra;
    if ha > 180.0 {
        ha -= 360.0;
    }
    if ha < -180.0 {
        ha += 360.0;
    }
    let pi = 0.5 * TWOPI;
    let pio2 = 0.5 * pi;
    let (az, el) = dcoord(pi, pio2 - lat / RAD, 0.0, lat / RAD, ha * TWOPI / 360.0, top_dec / RAD);
    Moon { ra, dec, top_dec, lst, az: az * RAD, el: el * RAD, dist }
}

/// `moondop.f90`: velocita' radiale della Luna rispetto alla stazione (km/s) e posizione.
fn moondop(y: i64, m: i64, day: i64, uth: f64, lon: f64, lat: f64) -> (f64, Moon) {
    let dlat = lat / RAD;
    let (dlat1, erad1) = geocentric(dlat, 200.0);
    let dt = 100.0;
    let before = moon2(y, m, day, uth - dt / 3600.0, lon, lat);
    let rme0 = toxyz(before.ra / RAD, before.dec / RAD, before.dist);
    let now = moon2(y, m, day, uth, lon, lat);
    let rme = toxyz(now.ra / RAD, now.dec / RAD, now.dist);
    let phi = now.lst * TWOPI_MOONDOP / 24.0;
    let rae = toxyz(phi, dlat1, erad1);
    let radps = TWOPI_MOONDOP / (86_400.0 / 1.002_737_909);
    let rae_v = [-rae[1] * radps, rae[0] * radps, 0.0];
    let mut rma = [0f64; 3];
    let mut rma_v = [0f64; 3];
    for k in 0..3 {
        rma[k] = rme[k] - rae[k];
        rma_v[k] = (rme[k] - rme0[k]) / dt - rae_v[k];
    }
    let dtopo0 = (rma[0] * rma[0] + rma[1] * rma[1] + rma[2] * rma[2]).sqrt();
    let vr = (rma_v[0] * rma[0] + rma_v[1] * rma[1] + rma_v[2] * rma[2]) / dtopo0;
    (vr, now)
}

/// Anno, mese, giorno dal numero di giorni dal 1/1/1970 (calendario gregoriano).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// La Luna e il Doppler della propria eco all'istante `unix_ms` per la stazione in `grid`, alla frequenza `freq_hz` (si usano i
/// MHz interi come QMAP). `None` se il locatore non e' valido o la frequenza e' sotto 1 MHz.
pub fn moon(unix_ms: u64, grid: &str, freq_hz: u64) -> Option<MoonInfo> {
    let (west, lat) = grid2deg(grid)?;
    let nfreq = u32::try_from(freq_hz / 1_000_000).ok().filter(|&f| f > 0)?;
    let days = (unix_ms / 86_400_000) as i64;
    let uth = (unix_ms % 86_400_000) as f64 / 3_600_000.0;
    let (y, m, d) = civil_from_days(days);
    // `astro.f90`: lon = -elon (grid2deg da' la longitudine ovest)
    let (vr, now) = moondop(y, m, d, uth, -west, lat);
    let freq = match nfreq {
        2 => 1.8e6,
        4 => 3.5e6,
        f => f64::from(f) * 1e6,
    };
    // Doppler di una tratta; quello della propria eco e' il doppio (`doppler00=2.0*xdop(1)`)
    let doppler = -freq * vr / C_KMS;
    Some(MoonInfo { az_deg: now.az, el_deg: now.el, self_doppler_hz: (2.0 * doppler).round() as i32, freq_mhz: nfreq, echo_delay_s: 2.0 * now.dist / C_KMS, dec_deg: now.top_dec })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_to_degrees_like_wsjtx() {
        let (w, lat) = grid2deg("IO91wm").expect("valido");
        assert!((w - 0.125).abs() < 1e-9 && (lat - 51.52083).abs() < 1e-4, "{w} {lat}");
        let (w4, lat4) = grid2deg("FN42").expect("valido");
        assert!((w4 - 70.9583).abs() < 1e-3 && (lat4 - 42.5208).abs() < 1e-3, "{w4} {lat4}");
        assert!(grid2deg("ZZ99").is_none() && grid2deg("IO9").is_none());
    }

    #[test]
    fn dates_from_unix_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_513), (2026, 3, 1));
    }

    #[test]
    fn the_moon_is_plausible() {
        // 1 gennaio 2026 12:00 UTC, IO91wm, 144 MHz
        let info = moon(1_767_268_800_000, "IO91wm", 144_100_000).expect("luna");
        assert!((0.0..360.0).contains(&info.az_deg) && (-90.0..=90.0).contains(&info.el_deg));
        // eco fra 2,3 e 2,7 s; Doppler proprio entro qualche centinaio di Hz a 144 MHz
        assert!((2.3..2.75).contains(&info.echo_delay_s), "{info:?}");
        assert!(info.self_doppler_hz.abs() < 1_000, "{info:?}");
        assert_eq!(info.freq_mhz, 144);
        // doppio della frequenza, doppio del Doppler
        let at288 = moon(1_767_268_800_000, "IO91wm", 288_000_000).expect("luna");
        assert!((at288.self_doppler_hz - 2 * info.self_doppler_hz).abs() <= 2);
    }
}
