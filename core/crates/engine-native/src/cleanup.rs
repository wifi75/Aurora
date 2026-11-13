//! Pulizia dei file che Aurora scrive senza limite: i periodi salvati «come JTDX» (con «Salva tutti» circa 2 GB al giorno) e `ALL.TXT`.
//!
//! Regole, scelte per non toccare mai altro:
//! - si guarda **una cartella sola**, senza scendere nelle sottocartelle, e solo i file con il nome che crea Aurora: `aammgg_hhmmss.wav`
//!   ([`crate::jtdx_stem`]) nella cartella dei salvataggi, `AAAAMM_ALL.TXT` e `ALL-AAAAMMGG-hhmmss.TXT` nella cartella dei dati;
//! - l'eta' si legge dal **nome** (l'ora del periodo, il mese, il momento della rotazione), non dalla data del file: un file copiato o
//!   toccato non cambia sorte;
//! - `keep_days` = 0 vuol dire «per sempre»: non si cancella niente (`ALL.TXT` troppo grande si ruota lo stesso, senza perdere righe).
//!
//! `ALL.TXT` del mese in corso si ruota in `ALL-<data>.TXT` quando supera [`ALL_TXT_MAX_BYTES`] o quando la sua prima riga e' piu' vecchia
//! del limite; i file ruotati e quelli dei mesi finiti si tolgono quando sono piu' vecchi del limite. Il motore apre `ALL.TXT` a ogni
//! scrittura (niente handle tenuto aperto), quindi rinominarlo mentre gira e' sicuro: alla scrittura dopo si crea quello nuovo.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::alltxt::Utc;

/// Oltre questa dimensione `ALL.TXT` si ruota (circa un mese di FT8 su banda piena).
pub const ALL_TXT_MAX_BYTES: u64 = 50 * 1024 * 1024;
/// Giorni di base per i file salvati.
pub const DEFAULT_KEEP_DAYS: u32 = 7;
const DAY_MS: u64 = 86_400_000;

/// Quanto e' stato tolto (e ruotato) in un giro.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Freed {
    pub files: u64,
    pub bytes: u64,
    pub rotated: u64,
}

impl Freed {
    fn add(&mut self, other: Freed) {
        self.files += other.files;
        self.bytes += other.bytes;
        self.rotated += other.rotated;
    }
}

/// Giorni dal 1970 di una data civile (Howard Hinnant, l'inverso di `Utc::from_unix_ms`).
fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn digits(s: &str) -> Option<u32> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten()
}

/// Millisecondi dal 1970 di data e ora UTC (`None` se non e' una data valida).
fn unix_ms(year: i64, month: u32, day: u32, h: u32, m: u32, s: u32) -> Option<u64> {
    if h > 23 || m > 59 || s > 59 {
        return None;
    }
    let days = u64::try_from(days_from_civil(year, month, day)?).ok()?;
    Some(days * DAY_MS + u64::from(h * 3600 + m * 60 + s) * 1000)
}

/// `aammgg_hhmmss.wav` (il nome di [`crate::jtdx_stem`]): l'ora del periodo.
fn wav_time(name: &str) -> Option<u64> {
    let stem = name.strip_suffix(".wav")?;
    let (date, time) = stem.split_once('_')?;
    if date.len() != 6 || time.len() != 6 {
        return None;
    }
    let (yy, mm, dd) = (digits(&date[0..2])?, digits(&date[2..4])?, digits(&date[4..6])?);
    let (h, m, s) = (digits(&time[0..2])?, digits(&time[2..4])?, digits(&time[4..6])?);
    unix_ms(2000 + i64::from(yy), mm, dd, h, m, s)
}

/// `AAAAMM_ALL.TXT`: anno e mese.
fn month_file(name: &str) -> Option<(i64, u32)> {
    let ym = name.strip_suffix("_ALL.TXT")?;
    if ym.len() != 6 {
        return None;
    }
    let (y, m) = (digits(&ym[0..4])?, digits(&ym[4..6])?);
    (1..=12).contains(&m).then_some((i64::from(y), m))
}

/// `ALL-AAAAMMGG-hhmmss.TXT`: il momento della rotazione.
fn rotated_time(name: &str) -> Option<u64> {
    let body = name.strip_prefix("ALL-")?.strip_suffix(".TXT")?;
    let (date, time) = body.split_once('-')?;
    if date.len() != 8 || time.len() != 6 {
        return None;
    }
    let (y, mo, d) = (digits(&date[0..4])?, digits(&date[4..6])?, digits(&date[6..8])?);
    let (h, mi, s) = (digits(&time[0..2])?, digits(&time[2..4])?, digits(&time[4..6])?);
    unix_ms(i64::from(y), mo, d, h, mi, s)
}

fn rotated_name(now_ms: u64) -> String {
    let at = Utc::from_unix_ms(now_ms);
    format!("ALL-{:04}{:02}{:02}-{:02}{:02}{:02}.TXT", at.year, at.month, at.day, at.hour, at.minute, at.second)
}

/// I file normali della cartella (niente sottocartelle, niente collegamenti), con nome e dimensione.
fn files_in(dir: &Path) -> Vec<(String, PathBuf, u64)> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    read.filter_map(Result::ok)
        .filter_map(|e| {
            let meta = e.file_type().ok().filter(fs::FileType::is_file).and_then(|_| e.metadata().ok())?;
            Some((e.file_name().to_string_lossy().into_owned(), e.path(), meta.len()))
        })
        .collect()
}

fn remove(path: &Path, bytes: u64, freed: &mut Freed) {
    match fs::remove_file(path) {
        Ok(()) => {
            freed.files += 1;
            freed.bytes += bytes;
        }
        Err(e) => tracing::warn!("pulizia: {} non tolto: {e}", path.display()),
    }
}

/// Toglie dalla cartella dei salvataggi i periodi `aammgg_hhmmss.wav` piu' vecchi di `keep_days` (0: niente).
pub fn clean_saved_wavs(dir: &Path, keep_days: u32, now_ms: u64) -> Freed {
    let mut freed = Freed::default();
    if keep_days == 0 {
        return freed;
    }
    let limit = now_ms.saturating_sub(u64::from(keep_days) * DAY_MS);
    for (name, path, len) in files_in(dir) {
        if wav_time(&name).is_some_and(|t| t < limit) {
            remove(&path, len, &mut freed);
        }
    }
    freed
}

/// La data della prima riga di `ALL.TXT` (`AAAAMMGG_...`), per sapere da quando scrive.
fn first_line_time(path: &Path) -> Option<u64> {
    use std::io::Read;
    let mut head = [0u8; 8];
    fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    let s = std::str::from_utf8(&head).ok()?;
    unix_ms(i64::from(digits(&s[0..4])?), digits(&s[4..6])?, digits(&s[6..8])?, 0, 0, 0)
}

/// Ruota `ALL.TXT` del mese in corso se troppo grande o troppo vecchio e toglie i file ruotati e dei mesi finiti oltre il limite.
pub fn clean_all_txt(data_dir: &Path, keep_days: u32, now_ms: u64, max_bytes: u64) -> Freed {
    let mut freed = Freed::default();
    let limit = (keep_days > 0).then(|| now_ms.saturating_sub(u64::from(keep_days) * DAY_MS));
    let now = Utc::from_unix_ms(now_ms);
    for (name, path, len) in files_in(data_dir) {
        if let Some((year, month)) = month_file(&name) {
            if (year, month) == (now.year, now.month) {
                let too_old = limit.is_some_and(|l| first_line_time(&path).is_some_and(|t| t < l));
                if len > max_bytes || too_old {
                    let target = data_dir.join(rotated_name(now_ms));
                    if target.exists() {
                        continue;
                    }
                    match fs::rename(&path, &target) {
                        Ok(()) => {
                            freed.rotated += 1;
                            tracing::info!("ALL.TXT ruotato in {} ({:.1} MB)", target.display(), len as f64 / 1_048_576.0);
                        }
                        Err(e) => tracing::warn!("ALL.TXT non ruotato ({}): {e}", path.display()),
                    }
                }
            } else {
                // un mese finito: la sua ultima riga e' al piu' all'inizio del mese dopo
                let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
                if let (Some(l), Some(end)) = (limit, unix_ms(ny, nm, 1, 0, 0, 0))
                    && end < l
                {
                    remove(&path, len, &mut freed);
                }
            }
        } else if let (Some(l), Some(t)) = (limit, rotated_time(&name))
            && t < l
        {
            remove(&path, len, &mut freed);
        }
    }
    freed
}

/// Un giro intero: periodi salvati e `ALL.TXT`, con la riga nel registro.
pub fn run(save_dir: &Path, data_dir: &Path, keep_days: u32, now_ms: u64) -> Freed {
    let mut freed = clean_saved_wavs(save_dir, keep_days, now_ms);
    freed.add(clean_all_txt(data_dir, keep_days, now_ms, ALL_TXT_MAX_BYTES));
    if freed.files > 0 || freed.rotated > 0 {
        tracing::info!(
            "pulizia dei file salvati (tenuti {keep_days} giorni): tolti {} file, liberati {:.1} MB, ALL.TXT ruotati {}",
            freed.files,
            freed.bytes as f64 / 1_048_576.0,
            freed.rotated
        );
    } else {
        tracing::debug!("pulizia dei file salvati (tenuti {keep_days} giorni): niente da togliere");
    }
    freed
}

/// La pulizia all'avvio e poi una volta al giorno, in un thread a bassa importanza; cartella e giorni si rileggono a ogni giro dalla scheda
/// Audio (`SaveSettings`), cosi' un cambio vale dal giro dopo.
pub fn spawn_daily(save: std::sync::Arc<crate::SaveSettings>, data_dir: PathBuf) {
    let spawned = std::thread::Builder::new().name("aurora-pulizia".into()).spawn(move || {
        loop {
            let (_, dir) = save.get();
            let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
            run(&dir, &data_dir, save.keep_days(), now_ms);
            std::thread::sleep(std::time::Duration::from_secs(86_400));
        }
    });
    if let Err(e) = spawned {
        tracing::warn!("pulizia dei file salvati non avviata: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-pulizia-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cartella");
        dir
    }

    fn ms(y: i64, mo: u32, d: u32, h: u32) -> u64 {
        unix_ms(y, mo, d, h, 0, 0).expect("data")
    }

    #[test]
    fn the_dates_come_back_from_the_names() {
        let t = ms(2026, 11, 13, 10);
        assert_eq!(Utc::from_unix_ms(t).day, 13);
        assert_eq!(wav_time(&format!("{}.wav", crate::jtdx_stem(t))), Some(t));
        assert_eq!(wav_time("261113_100000.WAV"), None);
        assert_eq!(wav_time("261113_1000.wav"), None);
        assert_eq!(wav_time("261340_100000.wav"), None, "mese impossibile");
        assert_eq!(month_file("202611_ALL.TXT"), Some((2026, 11)));
        assert_eq!(month_file("ALL.TXT"), None);
        assert_eq!(rotated_time(&rotated_name(t)), Some(t));
    }

    #[test]
    fn only_old_aurora_wavs_go_and_nothing_else_is_touched() {
        let dir = temp("wav");
        let now = ms(2026, 11, 13, 12);
        let old = format!("{}.wav", crate::jtdx_stem(now - 8 * DAY_MS));
        let recent = format!("{}.wav", crate::jtdx_stem(now - 6 * DAY_MS));
        for (name, size) in [(old.as_str(), 1000usize), (recent.as_str(), 10), ("mio.wav", 10), ("note.txt", 10), ("200101_000000.txt", 10)] {
            fs::write(dir.join(name), vec![0u8; size]).expect("file");
        }
        // una sottocartella con un nome «da Aurora» non si tocca, e nemmeno quello che contiene
        let sub = dir.join("200101_000000.wav");
        fs::create_dir_all(&sub).expect("sotto");
        fs::write(sub.join("200101_000000.wav"), b"x").expect("file");

        assert_eq!(clean_saved_wavs(&dir, 0, now), Freed::default(), "0 = per sempre");
        let freed = clean_saved_wavs(&dir, 7, now);
        assert_eq!((freed.files, freed.bytes), (1, 1000));
        assert!(!dir.join(&old).exists());
        for kept in [recent.as_str(), "mio.wav", "note.txt", "200101_000000.txt"] {
            assert!(dir.join(kept).exists(), "{kept}");
        }
        assert!(sub.join("200101_000000.wav").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn all_txt_rotates_when_big_or_old_and_old_files_go() {
        let dir = temp("alltxt");
        let now = ms(2026, 11, 5, 12);
        // il mese in corso, piccolo e cominciato da poco: resta
        fs::write(dir.join("202611_ALL.TXT"), "20261101_000000  14.074 MHz  FT8\r\n").expect("file");
        // un mese finito da piu' di 7 giorni (settembre) e uno finito da poco (ottobre, 4 giorni e mezzo fa)
        fs::write(dir.join("202609_ALL.TXT"), vec![b'x'; 500]).expect("file");
        fs::write(dir.join("202610_ALL.TXT"), b"x").expect("file");
        // ruotati: uno vecchio, uno recente; e file non di Aurora
        fs::write(dir.join(rotated_name(now - 10 * DAY_MS)), vec![b'y'; 300]).expect("file");
        fs::write(dir.join(rotated_name(now - 2 * DAY_MS)), b"y").expect("file");
        for other in ["ALL.TXT", "aurora.log", "settings.json"] {
            fs::write(dir.join(other), b"z").expect("file");
        }

        assert_eq!(clean_all_txt(&dir, 0, now, ALL_TXT_MAX_BYTES), Freed::default(), "0 = per sempre");
        let freed = clean_all_txt(&dir, 7, now, ALL_TXT_MAX_BYTES);
        assert_eq!((freed.files, freed.bytes, freed.rotated), (2, 800, 0));
        assert!(dir.join("202611_ALL.TXT").exists() && dir.join("202610_ALL.TXT").exists());
        assert!(!dir.join("202609_ALL.TXT").exists());
        assert!(!dir.join(rotated_name(now - 10 * DAY_MS)).exists() && dir.join(rotated_name(now - 2 * DAY_MS)).exists());
        for other in ["ALL.TXT", "aurora.log", "settings.json"] {
            assert!(dir.join(other).exists(), "{other}");
        }

        // troppo grande: si ruota (anche con «per sempre», senza togliere niente)
        let freed = clean_all_txt(&dir, 0, now, 10);
        assert_eq!((freed.files, freed.rotated), (0, 1));
        assert!(!dir.join("202611_ALL.TXT").exists() && dir.join(rotated_name(now)).exists());

        // cominciato oltre il limite: si ruota
        fs::write(dir.join("202611_ALL.TXT"), "20261101_000000  14.074 MHz  FT8\r\n").expect("file");
        let later = ms(2026, 11, 20, 12);
        assert_eq!(clean_all_txt(&dir, 7, later, ALL_TXT_MAX_BYTES).rotated, 1);
        assert!(dir.join(rotated_name(later)).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn after_a_rotation_the_new_all_txt_starts_with_the_frequency_line() {
        let dir = temp("rotazione");
        let mut log = crate::alltxt::AllTxt::new(&dir, "Aurora");
        let t = ms(2026, 11, 13, 12);
        log.write_decodes(t, 14_074_000, "FT8", &[(-10, 0.1, 1500, "CQ IU3ABC IO91")]);
        assert_eq!(clean_all_txt(&dir, 0, t + 20_000, 1).rotated, 1);
        log.write_decodes(t + 15_000, 14_074_000, "FT8", &[(-12, 0.2, 1200, "CQ IU3ABC IO91")]);
        let text = fs::read_to_string(dir.join("202611_ALL.TXT")).expect("nuovo file");
        assert!(text.lines().next().is_some_and(|l| l.contains("MHz")), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }
}
