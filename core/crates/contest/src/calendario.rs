//! Le date dei contest: dalla regola del file (`ultimo-weekend-completo:agosto`, `secondo-martedi:febbraio,maggio`...) agli istanti di
//! inizio e fine in millisecondi UTC. Niente librerie di date: bastano i giorni dall'epoca (algoritmo di Howard Hinnant).

use crate::regole::Calendario;

const DAY_MS: i64 = 86_400_000;

/// Giorni dal 1970-01-01 di una data civile.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// La data civile (anno, mese, giorno) di un giorno dall'epoca.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

/// Giorno della settimana: 0 domenica ... 6 sabato.
pub fn weekday(days: i64) -> u32 {
    (days + 4).rem_euclid(7) as u32
}

fn days_in_month(y: i64, m: u32) -> u32 {
    let next = if m == 12 { days_from_civil(y + 1, 1, 1) } else { days_from_civil(y, m + 1, 1) };
    (next - days_from_civil(y, m, 1)) as u32
}

const MESI: &[&str] = &["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"];
const GIORNI: &[&str] = &["domenica", "lunedi", "martedi", "mercoledi", "giovedi", "venerdi", "sabato"];

fn ordinale(s: &str) -> Option<i32> {
    Some(match s {
        "primo" => 1,
        "secondo" => 2,
        "terzo" => 3,
        "quarto" => 4,
        "ultimo" => -1,
        _ => return None,
    })
}

fn mesi(s: &str) -> Option<Vec<u32>> {
    if s == "tutti" {
        return Some((1..=12).collect());
    }
    s.split(',').map(|m| MESI.iter().position(|x| *x == m.trim()).map(|i| i as u32 + 1)).collect()
}

/// Una regola letta: `None` = «sempre» (date scelte dall'operatore).
#[derive(Clone, Debug, PartialEq)]
enum Regola {
    Sempre,
    /// N-esimo fine settimana (`completo`: sabato e domenica nello stesso mese).
    Weekend { n: i32, completo: bool, mesi: Vec<u32> },
    /// Giorni della settimana del mese scelti per ordinale (`primo,secondo-mercoledi`); vuoto = ogni settimana (`ogni-giovedi`).
    Giorno { n: Vec<i32>, giorno: u32, mesi: Vec<u32> },
}

fn leggi(regola: &str) -> Result<Regola, String> {
    let r = regola.trim().to_lowercase();
    if r == "sempre" {
        return Ok(Regola::Sempre);
    }
    let (sinistra, destra) = r.split_once(':').ok_or_else(|| format!("calendario.regola «{regola}»: manca «:mesi»"))?;
    let mesi = mesi(destra).ok_or_else(|| format!("calendario.regola «{regola}»: mese sconosciuto"))?;
    let parti: Vec<&str> = sinistra.splitn(2, '-').collect();
    let [ord, resto] = parti.as_slice() else { return Err(format!("calendario.regola «{regola}» non valida")) };
    // «ogni» = tutte le settimane (contest settimanali); piu' ordinali separati da virgola (`primo,secondo,terzo-mercoledi`)
    let n: Vec<i32> = if *ord == "ogni" {
        Vec::new()
    } else {
        ord.split(',').map(|o| ordinale(o.trim())).collect::<Option<Vec<i32>>>().ok_or_else(|| format!("calendario.regola «{regola}»: ordinale sconosciuto"))?
    };
    match *resto {
        "weekend-completo" | "weekend" => {
            let [n] = n.as_slice() else { return Err(format!("calendario.regola «{regola}»: per il fine settimana serve un solo ordinale")) };
            Ok(Regola::Weekend { n: *n, completo: *resto == "weekend-completo", mesi })
        }
        g => {
            let giorno = GIORNI.iter().position(|x| *x == g).ok_or_else(|| format!("calendario.regola «{regola}»: giorno sconosciuto"))? as u32;
            Ok(Regola::Giorno { n, giorno, mesi })
        }
    }
}

/// `sab 12:00` / `dom 11:59` / `lun 02:59` (giorni dal sabato) oppure `+0 19:00`: (giorni dal giorno della regola, minuti del giorno).
fn ora(spec: &str) -> Result<(i64, i64), String> {
    let (g, hm) = spec.trim().split_once(' ').ok_or_else(|| format!("orario «{spec}» non valido (es. «sab 12:00»)"))?;
    let giorno = match g {
        "sab" => 0,
        "dom" => 1,
        "lun" => 2,
        x if x.starts_with('+') => x[1..].parse::<i64>().map_err(|_| format!("orario «{spec}» non valido"))?,
        _ => return Err(format!("orario «{spec}»: giorno sconosciuto (sab, dom, lun o +N)")),
    };
    let (h, m) = hm.split_once(':').ok_or_else(|| format!("orario «{spec}» non valido"))?;
    let (h, m): (i64, i64) = (h.parse().map_err(|_| format!("orario «{spec}» non valido"))?, m.parse().map_err(|_| format!("orario «{spec}» non valido"))?);
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return Err(format!("orario «{spec}» fuori scala"));
    }
    Ok((giorno, h * 60 + m))
}

/// `2026-07-18T14:00Z` in millisecondi.
pub fn parse_iso(s: &str) -> Option<i64> {
    let s = s.trim().trim_end_matches('Z');
    let (d, t) = s.split_once('T')?;
    let mut dp = d.split('-');
    let (y, m, dd) = (dp.next()?.parse::<i64>().ok()?, dp.next()?.parse::<u32>().ok()?, dp.next()?.parse::<u32>().ok()?);
    let mut tp = t.split(':');
    let (h, mi) = (tp.next()?.parse::<i64>().ok()?, tp.next()?.parse::<i64>().ok()?);
    let sec = tp.next().and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    Some(days_from_civil(y, m, dd) * DAY_MS + (h * 3600 + mi * 60 + sec) * 1000)
}

/// Controllo del calendario (al caricamento del file).
pub fn controlla(c: &Calendario) -> Result<(), String> {
    leggi(&c.regola)?;
    ora(&c.inizio)?;
    ora(&c.fine)?;
    for o in &c.inverno {
        ora(o)?;
    }
    if !c.inverno.is_empty() && c.inverno.len() != 2 {
        return Err("calendario.inverno: servono inizio e fine".into());
    }
    for (anno, v) in &c.date_fisse {
        if v.len() != 2 || v.iter().any(|x| parse_iso(x).is_none()) {
            return Err(format!("calendario.date_fisse {anno}: servono due istanti come «2026-07-18T14:00Z»"));
        }
    }
    Ok(())
}

/// I giorni «di partenza» della regola in un anno (il sabato del fine settimana o il giorno scelto).
fn ancore(r: &Regola, anno: i64) -> Vec<i64> {
    let mut out = Vec::new();
    match r {
        Regola::Sempre => {}
        Regola::Weekend { n, completo, mesi } => {
            for &m in mesi {
                let primo = days_from_civil(anno, m, 1);
                let ultimo = primo + i64::from(days_in_month(anno, m)) - 1;
                let sabati: Vec<i64> = (primo..=ultimo).filter(|d| weekday(*d) == 6 && (!completo || *d < ultimo)).collect();
                let scelto = if *n < 0 { sabati.last().copied() } else { sabati.get((*n - 1) as usize).copied() };
                out.extend(scelto);
            }
        }
        Regola::Giorno { n, giorno, mesi } => {
            for &m in mesi {
                let primo = days_from_civil(anno, m, 1);
                let ultimo = primo + i64::from(days_in_month(anno, m)) - 1;
                let giorni: Vec<i64> = (primo..=ultimo).filter(|d| weekday(*d) == *giorno).collect();
                if n.is_empty() {
                    out.extend(giorni);
                    continue;
                }
                for &k in n {
                    let scelto = if k < 0 { giorni.last().copied() } else { giorni.get((k - 1) as usize).copied() };
                    out.extend(scelto);
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// Una finestra del contest: inizio incluso, fine esclusa (ms UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Finestra {
    pub inizio_ms: i64,
    pub fine_ms: i64,
}

/// Le finestre del contest in un anno. Per «sempre» nessuna (gli orari li sceglie l'operatore).
pub fn finestre(c: &Calendario, anno: i64) -> Vec<Finestra> {
    if let Some(v) = c.date_fisse.get(&anno.to_string())
        && let (Some(a), Some(b)) = (v.first().and_then(|x| parse_iso(x)), v.get(1).and_then(|x| parse_iso(x)))
    {
        return vec![Finestra { inizio_ms: a, fine_ms: b + 60_000 }];
    }
    let Ok(r) = leggi(&c.regola) else { return Vec::new() };
    let mut out = Vec::new();
    for giorno in ancore(&r, anno) {
        let (_, m, _) = civil_from_days(giorno);
        let invernale = c.inverno.len() == 2 && matches!(m, 11 | 12 | 1 | 2 | 3);
        let (si, sf) = if invernale { (c.inverno[0].as_str(), c.inverno[1].as_str()) } else { (c.inizio.as_str(), c.fine.as_str()) };
        let (Ok((gi, mi)), Ok((gf, mf))) = (ora(si), ora(sf)) else { continue };
        let inizio = (giorno + gi) * DAY_MS + mi * 60_000;
        let fine = (giorno + gf) * DAY_MS + mf * 60_000 + 60_000;
        out.push(Finestra { inizio_ms: inizio, fine_ms: fine });
    }
    out
}

/// La finestra in corso o la prossima dopo `ora_ms` (si guarda quest'anno e il prossimo). `None` per «sempre».
pub fn prossima(c: &Calendario, ora_ms: i64) -> Option<Finestra> {
    let (anno, _, _) = civil_from_days(ora_ms.div_euclid(DAY_MS));
    let mut tutte: Vec<Finestra> = (anno - 1..=anno + 1).flat_map(|a| finestre(c, a)).collect();
    tutte.sort_by_key(|f| f.inizio_ms);
    tutte.into_iter().find(|f| f.fine_ms > ora_ms)
}

/// Per quanto tempo dopo la fine un'edizione resta «quella attuale» (fase «finito», log e invio del file), se la prossima non e' gia' iniziata.
pub const DOPO_FINE_MS: i64 = 48 * 3_600_000;

/// L'edizione attuale: quella in corso, o quella finita da meno di [`DOPO_FINE_MS`] (se la prossima non e' cominciata), altrimenti la prossima.
pub fn attuale(c: &Calendario, ora_ms: i64) -> Option<Finestra> {
    let (anno, _, _) = civil_from_days(ora_ms.div_euclid(DAY_MS));
    let mut tutte: Vec<Finestra> = (anno - 1..=anno + 1).flat_map(|a| finestre(c, a)).collect();
    tutte.sort_by_key(|f| f.inizio_ms);
    let ultima = tutte.iter().rev().find(|f| f.inizio_ms <= ora_ms).copied();
    match ultima {
        Some(f) if ora_ms < f.fine_ms + DOPO_FINE_MS => Some(f),
        _ => tutte.into_iter().find(|f| f.fine_ms > ora_ms),
    }
}

/// La regola e' «sempre» (date scelte dall'operatore).
pub fn e_sempre(c: &Calendario) -> bool {
    matches!(leggi(&c.regola), Ok(Regola::Sempre))
}

/// `aaaa-mm-gg` di un istante.
pub fn data(ms: i64) -> String {
    let (y, m, d) = civil_from_days(ms.div_euclid(DAY_MS));
    format!("{y:04}-{m:02}-{d:02}")
}

/// `hhmm` UTC di un istante.
pub fn hhmm(ms: i64) -> String {
    let s = ms.div_euclid(1000).rem_euclid(86_400);
    format!("{:02}{:02}", s / 3600, s / 60 % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cal(regola: &str, inizio: &str, fine: &str) -> Calendario {
        Calendario { regola: regola.into(), inizio: inizio.into(), fine: fine.into(), inverno: Vec::new(), date_fisse: Default::default(), ore_massime: None, pausa_minima_min: None }
    }

    #[test]
    fn date_civili() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(days_from_civil(2026, 8, 29)), (2026, 8, 29));
        assert_eq!(weekday(days_from_civil(2026, 8, 29)), 6, "29 agosto 2026 e' sabato");
    }

    /// WW Digi 2026: 29-30 agosto, 12:00 - 11:59.
    #[test]
    fn ww_digi_2026() {
        let f = finestre(&cal("ultimo-weekend-completo:agosto", "sab 12:00", "dom 11:59"), 2026);
        assert_eq!(f.len(), 1);
        assert_eq!(data(f[0].inizio_ms), "2026-08-29");
        assert_eq!(hhmm(f[0].inizio_ms), "1200");
        assert_eq!(data(f[0].fine_ms - 1), "2026-08-30");
    }

    /// Field Day 2026: quarto fine settimana completo di giugno = 27-28 giugno.
    #[test]
    fn field_day_2026() {
        let f = finestre(&cal("quarto-weekend-completo:giugno", "sab 18:00", "dom 20:59"), 2026);
        assert_eq!(data(f[0].inizio_ms), "2026-06-27");
    }

    /// DARC FT4: secondo martedi' di novembre 2026 = 10 novembre.
    #[test]
    fn darc_ft4_novembre() {
        let f = finestre(&cal("secondo-martedi:febbraio,maggio,agosto,novembre", "+0 19:00", "+0 20:29"), 2026);
        assert_eq!(f.len(), 4);
        assert_eq!(data(f[3].inizio_ms), "2026-11-10");
        assert_eq!(hhmm(f[3].inizio_ms), "1900");
    }

    /// ARI: orari invernali da novembre a marzo.
    #[test]
    fn ari_orari_invernali() {
        let mut c = cal("primo-mercoledi:tutti", "+0 17:00", "+0 19:59");
        c.inverno = vec!["+0 18:00".into(), "+0 20:59".into()];
        let f = finestre(&c, 2026);
        assert_eq!(f.len(), 12);
        assert_eq!(hhmm(f[0].inizio_ms), "1800", "gennaio");
        assert_eq!(hhmm(f[5].inizio_ms), "1700", "giugno");
    }

    /// Finita la gara, per due giorni l'edizione attuale resta quella appena finita (non la prossima); poi si passa alla prossima.
    #[test]
    fn attuale_tiene_l_edizione_appena_finita() {
        let c = cal("secondo-martedi:febbraio,maggio,agosto,novembre", "+0 19:00", "+0 20:29");
        let nov = finestre(&c, 2026)[3];
        assert_eq!(attuale(&c, nov.inizio_ms + 60_000), Some(nov), "in corso");
        assert_eq!(attuale(&c, nov.fine_ms + 3_600_000), Some(nov), "appena finita");
        assert_ne!(prossima(&c, nov.fine_ms + 3_600_000), Some(nov));
        let dopo = attuale(&c, nov.fine_ms + DOPO_FINE_MS + 1).map(|f| f.inizio_ms);
        assert!(dopo.is_some_and(|i| i > nov.fine_ms), "poi la prossima");
        assert_eq!(attuale(&c, nov.inizio_ms - 1), Some(nov), "prima: la prossima");
    }

    #[test]
    fn sempre_non_ha_date() {
        let c = cal("sempre", "+0 00:00", "+0 23:59");
        assert!(e_sempre(&c));
        assert!(finestre(&c, 2026).is_empty());
    }
}
