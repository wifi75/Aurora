//! `ALL.TXT`: il registro del traffico, nello stesso formato di JTDX (`docs/analisi-all-txt-e-aggiornamenti.md`).
//!
//! JTDX scrive un file al mese, `AAAAMM_ALL.TXT`, nella sua cartella dei dati: lo apre in aggiunta e testo, scrive e lo chiude a ogni evento
//! (niente handle tenuto aperto: si puo' leggere o cancellare mentre il programma gira), con fine riga CR LF. Le righe sono di quattro tipi:
//!
//! - **decodifica**: `AAAAMMGG_hhmmss` + SNR (4 colonne) + DT (5, un decimale) + frequenza audio (5) + ` ~ ` (FT8) o ` : ` (FT4) + messaggio
//!   (26 colonne, troncato) + indicatore di una colonna, senza spazi finali;
//! - **frequenza**: `AAAAMMGG_hhmmss  14.074 MHz  FT8 <programma>` quando la radio cambia frequenza (e prima della prima decodifica);
//! - **trasmissione**: `AAAAMMGG_hhmmss.mmm(scarto)  Transmitting 14.074 MHz + 1500Hz  FT8:  <messaggio a 37 colonne>`;
//! - **QSO registrato**: `AAAAMMGG_hhmmss.mmm(scarto)  QSO logged: <nominativo>`.
//!
//! Qui si scrivono le stesse righe dal motore proprio, in `ALL.TXT` mensile nella cartella dei dati di Aurora (mai in quella di JTDX).
//! `AURORA_ALL_TXT=0` spegne la scrittura (JTDX: casella «decodifiche» in Impostazioni, `WriteDecodedALLTXT`, accesa di base; le righe di
//! trasmissione non si possono spegnere).

use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Un'ora UTC scomposta, dai millisecondi dal 1970.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utc {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub milli: u32,
}

impl Utc {
    pub fn from_unix_ms(ms: u64) -> Utc {
        let days = (ms / 86_400_000) as i64;
        let rest = ms % 86_400_000;
        // giorni dal 1970 -> data civile (algoritmo di Howard Hinnant)
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let mut year = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        if month <= 2 {
            year += 1;
        }
        Utc {
            year,
            month,
            day,
            hour: (rest / 3_600_000) as u32,
            minute: (rest / 60_000 % 60) as u32,
            second: (rest / 1000 % 60) as u32,
            milli: (rest % 1000) as u32,
        }
    }

    /// `AAAAMMGG`.
    fn date(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }

    /// `hhmmss`.
    fn time(&self) -> String {
        format!("{:02}{:02}{:02}", self.hour, self.minute, self.second)
    }

    /// `AAAAMM_ALL.TXT`: il file del mese.
    pub fn file_name(&self) -> String {
        format!("{:04}{:02}_ALL.TXT", self.year, self.month)
    }
}

/// Un numero come lo scrive `QTextStream` (formato `%g`): `precision` cifre significative, zeri finali tolti, esponente `e-06` solo sotto 1e-4
/// o da `10^precision` in su. JTDX scrive cosi' la frequenza (12 cifre) e lo scarto dell'orologio (6).
pub fn format_g(value: f64, precision: usize) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if !value.is_finite() {
        return if value.is_nan() { "nan".into() } else if value > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let precision = precision.max(1);
    // l'esponente dopo l'arrotondamento a `precision` cifre
    let sci = format!("{:.*e}", precision - 1, value);
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if exp < -4 || exp >= precision as i32 {
        let mantissa = trim_zeros(mantissa);
        format!("{mantissa}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        let decimals = (precision as i32 - 1 - exp).max(0) as usize;
        trim_zeros(&format!("{value:.decimals$}")).to_string()
    }
}

fn trim_zeros(s: &str) -> &str {
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { s }
}

/// La frequenza della radio in MHz come nelle righe di JTDX (`14.074`, `7.074055`, `14.08`).
pub fn format_mhz(dial_hz: u64) -> String {
    format_g(dial_hz as f64 / 1_000_000.0, 12)
}

/// Riga di decodifica. `hhmmss` e' l'inizio del periodo; il messaggio e' troncato a 26 caratteri come in JTDX (`msg26`); `flag` e' l'indicatore
/// finale (spazio = nessuno). Modo `ft4` = `:`, altrimenti `~`.
pub fn decode_line(date: &str, hhmmss: &str, row: (i32, f32, i64), ft4: bool, text: &str, flag: char) -> String {
    let (snr, dt, freq_hz) = row;
    let msg: String = text.chars().take(26).collect();
    let line = format!("{date}_{hhmmss}{snr:>4}{dt:>5.1}{freq_hz:>5} {} {msg:<26}{flag}", if ft4 { ':' } else { '~' });
    line.trim().to_string()
}

/// Riga di frequenza con il nome del programma (`FT8 Aurora v0.1.0`), come JTDX quando la radio cambia frequenza.
pub fn frequency_line(at: &Utc, dial_hz: u64, mode: &str, program: &str) -> String {
    format!("{}_{}  {} MHz  {mode} {program}", at.date(), at.time(), format_mhz(dial_hz))
}

/// Riga di frequenza senza programma: quella che precede la prima decodifica dopo l'avvio o il cambio di giorno.
pub fn header_line(at: &Utc, dial_hz: u64, mode: &str) -> String {
    format!("{}_{}  {} MHz  {mode}", at.date(), at.time(), format_mhz(dial_hz))
}

/// Riga di trasmissione. Il messaggio va a 37 colonne (`msgsent` di JTDX); `offset_s` e' lo scarto dell'orologio applicato (`(0)`, `(-0.22)`).
pub fn tx_line(at: &Utc, offset_s: f32, dial_hz: u64, tx_df: u32, mode: &str, text: &str) -> String {
    format!("{}_{}.{:03}({})  Transmitting {} MHz + {tx_df}Hz  {mode}:  {text:<37}", at.date(), at.time(), at.milli, format_g(f64::from(offset_s), 6), format_mhz(dial_hz))
}

/// Riga di ritrasmissione (`guiUpdate` di JTDX, `m_restart`): il messaggio e' cambiato dopo l'avvio e la trasmissione riparte. Nota la forma: `+1500Hz`
/// senza spazio e il messaggio senza riempimento.
pub fn retransmit_line(at: &Utc, offset_s: f32, dial_hz: u64, tx_df: u32, mode: &str, text: &str) -> String {
    format!("{}_{}.{:03}({})  Retransmitting {} MHz +{tx_df}Hz  {mode}:  {text}", at.date(), at.time(), at.milli, format_g(f64::from(offset_s), 6), format_mhz(dial_hz))
}

/// Riga «QSO logged» (anche tutti gli altri messaggi di servizio hanno questa forma: ora con i millisecondi, scarto, due spazi, testo).
pub fn service_line(at: &Utc, offset_s: f32, text: &str) -> String {
    format!("{}_{}.{:03}({})  {text}", at.date(), at.time(), at.milli, format_g(f64::from(offset_s), 6))
}

/// Cosa si scrive di cio' che si decodifica (`WriteDecodedALLTXT`, `WriteDecodedDebugALLTXT`; `Configuration::read_settings` e `MainWindow::readFromStdout`).
/// Le righe di trasmissione e dei QSO registrati si scrivono sempre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AllTxtScope {
    /// Nessuna delle due caselle: solo i messaggi che contengono il proprio nominativo (JTDX: `t.indexOf(m_baseCall) >= 0`).
    Calls,
    /// «Messaggi decodificati»: tutti i messaggi.
    #[default]
    Decoded,
    /// «Messaggi decodificati e di debug»: tutti, piu' le righe di debug.
    Debug,
}

impl AllTxtScope {
    pub fn from_flags(decoded: bool, debug: bool) -> AllTxtScope {
        if debug {
            AllTxtScope::Debug
        } else if decoded {
            AllTxtScope::Decoded
        } else {
            AllTxtScope::Calls
        }
    }

    /// Il messaggio si scrive? `my_call`: il proprio nominativo (si cerca la sua parte base, senza `/P`).
    pub fn keeps(self, text: &str, my_call: &str) -> bool {
        match self {
            AllTxtScope::Decoded | AllTxtScope::Debug => true,
            AllTxtScope::Calls => {
                let base = qso_native::base_call(my_call);
                !base.is_empty() && text.contains(base.as_str())
            }
        }
    }
}

/// Il registro: un file al mese in `dir`, aperto e chiuso a ogni scrittura.
pub struct AllTxt {
    dir: PathBuf,
    enabled: bool,
    program: String,
    /// Prima della prossima decodifica va scritta la riga di frequenza (avvio, cambio di giorno, file cancellato).
    header_pending: bool,
    last_secs: Option<u32>,
    last_dial_hz: u64,
}

impl AllTxt {
    pub fn new(dir: &Path, program: &str) -> AllTxt {
        AllTxt { dir: dir.to_path_buf(), enabled: true, program: program.to_string(), header_pending: true, last_secs: None, last_dial_hz: 0 }
    }

    /// Come `new`, ma `AURORA_ALL_TXT=0` (o `false`, `no`, `off`) spegne la scrittura.
    pub fn from_env(dir: &Path, program: &str) -> AllTxt {
        let mut log = AllTxt::new(dir, program);
        if let Ok(v) = std::env::var("AURORA_ALL_TXT") {
            log.enabled = !matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off");
        }
        log
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Aggiunge le righe (CR LF) al file del mese di `at`. Errori di scrittura: solo un avviso, il traffico non si ferma per un registro.
    fn append(&self, at: &Utc, lines: &[String]) {
        if !self.enabled || lines.is_empty() {
            return;
        }
        let path = self.dir.join(at.file_name());
        let result = std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| {
            let mut buf = String::new();
            for l in lines {
                buf.push_str(l);
                buf.push_str("\r\n");
            }
            f.write_all(buf.as_bytes())
        });
        if let Err(e) = result {
            tracing::warn!("ALL.TXT non scritto ({}): {e}", path.display());
        }
    }

    /// Le decodifiche di un periodo. `slot_start_ms` e' l'inizio del periodo; la data e il file sono quelli del periodo (JTDX usa la data del
    /// momento della scrittura: la differenza si vede solo per i messaggi decodificati dopo mezzanotte). Prima della prima riga, e dopo ogni
    /// cambio di giorno, la riga di frequenza.
    pub fn write_decodes(&mut self, slot_start_ms: u64, dial_hz: u64, mode: &str, decoded: &[(i32, f32, i64, &str)]) {
        if !self.enabled || decoded.is_empty() {
            return;
        }
        let at = Utc::from_unix_ms(slot_start_ms);
        let secs = at.hour * 3600 + at.minute * 60 + at.second;
        if self.last_secs.is_some_and(|last| secs < last) {
            self.header_pending = true;
        }
        self.last_secs = Some(secs);
        let ft4 = mode == "FT4";
        let mut lines = Vec::with_capacity(decoded.len() + 1);
        if self.header_pending {
            lines.push(header_line(&at, dial_hz, mode));
            self.header_pending = false;
        }
        let (date, time) = (at.date(), at.time());
        for (snr, dt, freq, text) in decoded {
            lines.push(decode_line(&date, &time, (*snr, *dt, *freq), ft4, text, ' '));
        }
        self.append(&at, &lines);
    }

    /// La radio ha cambiato frequenza (a trasmissione ferma): riga con il nome del programma. La prima lettura vale come cambio, una frequenza
    /// nulla (radio muta) no.
    pub fn write_frequency_change(&mut self, now_ms: u64, dial_hz: u64, mode: &str) {
        if dial_hz == 0 || dial_hz == self.last_dial_hz {
            return;
        }
        self.last_dial_hz = dial_hz;
        let at = Utc::from_unix_ms(now_ms);
        self.append(&at, &[frequency_line(&at, dial_hz, mode, &self.program)]);
    }

    /// Una trasmissione e' cominciata.
    pub fn write_tx(&self, now_ms: u64, offset_s: f32, dial_hz: u64, tx_df: u32, mode: &str, text: &str) {
        let at = Utc::from_unix_ms(now_ms);
        self.append(&at, &[tx_line(&at, offset_s, dial_hz, tx_df, mode, text)]);
    }

    /// Una trasmissione e' ripartita con un messaggio diverso nello stesso periodo.
    pub fn write_retransmit(&self, now_ms: u64, offset_s: f32, dial_hz: u64, tx_df: u32, mode: &str, text: &str) {
        let at = Utc::from_unix_ms(now_ms);
        self.append(&at, &[retransmit_line(&at, offset_s, dial_hz, tx_df, mode, text)]);
    }

    /// Una riga di debug (solo con «Messaggi decodificati e di debug»): `writeToALLTXT` di JTDX, con data, ora e scarto dell'orologio.
    pub fn write_debug(&self, now_ms: u64, text: &str) {
        let at = Utc::from_unix_ms(now_ms);
        self.append(&at, &[service_line(&at, 0.0, text)]);
    }

    /// Un QSO e' stato registrato.
    pub fn write_qso_logged(&self, now_ms: u64, offset_s: f32, call: &str) {
        let at = Utc::from_unix_ms(now_ms);
        self.append(&at, &[service_line(&at, offset_s, &format!("QSO logged: {call}"))]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-10-18 04:36:00 UTC
    const T: u64 = 1_792_298_160_000;

    #[test]
    fn the_scope_decides_which_decodes_are_written() {
        assert_eq!(AllTxtScope::from_flags(true, false), AllTxtScope::Decoded);
        assert_eq!(AllTxtScope::from_flags(false, true), AllTxtScope::Debug);
        assert_eq!(AllTxtScope::from_flags(true, true), AllTxtScope::Debug);
        assert_eq!(AllTxtScope::from_flags(false, false), AllTxtScope::Calls);
        assert!(AllTxtScope::Decoded.keeps("CQ K1ABC FN42", "IU3ABC") && AllTxtScope::Debug.keeps("CQ K1ABC FN42", "IU3ABC"));
        assert!(!AllTxtScope::Calls.keeps("CQ K1ABC FN42", "IU3ABC"), "un CQ di altri non si scrive senza le caselle");
        assert!(AllTxtScope::Calls.keeps("IU3ABC K1ABC -10", "IU3ABC") && AllTxtScope::Calls.keeps("K1ABC IU3ABC RR73", "IU3ABC"));
        assert!(AllTxtScope::Calls.keeps("K1ABC IU3ABC RR73", "IU3ABC/P"), "con /P conta la parte base");
        assert!(!AllTxtScope::Calls.keeps("CQ K1ABC FN42", ""), "senza nominativo proprio non c'e' niente da cercare");
    }

    #[test]
    fn utc_from_unix_ms_is_the_civil_date() {
        let at = Utc::from_unix_ms(T + 17_739);
        assert_eq!((at.year, at.month, at.day, at.hour, at.minute, at.second, at.milli), (2026, 10, 18, 4, 36, 17, 739));
        assert_eq!(at.file_name(), "202610_ALL.TXT");
        // il 29 febbraio 2028 e il passaggio d'anno
        assert_eq!(Utc::from_unix_ms(1_835_395_200_000).date(), "20280229");
        let ny = Utc::from_unix_ms(1_767_225_599_000);
        assert_eq!((ny.date().as_str(), ny.time().as_str()), ("20251231", "235959"));
        assert_eq!(Utc::from_unix_ms(0).date(), "19700101");
    }

    #[test]
    fn numbers_are_written_like_qtextstream() {
        assert_eq!(format_mhz(7_074_000), "7.074");
        assert_eq!(format_mhz(7_074_055), "7.074055");
        assert_eq!(format_mhz(14_080_000), "14.08");
        assert_eq!(format_mhz(21_074_000), "21.074");
        assert_eq!(format_mhz(28_074_000), "28.074");
        assert_eq!(format_mhz(2), "2e-06");
        assert_eq!(format_mhz(0), "0");
        assert_eq!(format_g(-0.22, 6), "-0.22");
        assert_eq!(format_g(0.0, 6), "0");
        assert_eq!(format_g(1.5, 6), "1.5");
        assert_eq!(format_g(0.0001, 6), "0.0001");
        assert_eq!(format_g(0.00001, 6), "1e-05");
        assert_eq!(format_g(123456789.0, 6), "1.23457e+08");
    }

    #[test]
    fn a_decode_line_has_the_columns_of_jtdx() {
        // dai file veri: SNR a 4 colonne, DT a 5, frequenza a 5, tilde, messaggio a 26
        assert_eq!(decode_line("20261018", "043545", (-9, 0.1, 1908), false, "R8CEC SV2YC KN10", ' '), "20261018_043545  -9  0.1 1908 ~ R8CEC SV2YC KN10");
        assert_eq!(decode_line("20261018", "043545", (-3, -0.1, 911), false, "CQ RA5O KO91", ' '), "20261018_043545  -3 -0.1  911 ~ CQ RA5O KO91");
        assert_eq!(decode_line("20261018", "043545", (-8, 0.0, 1363), false, "CQ RC7KH KN75", '*'), "20261018_043545  -8  0.0 1363 ~ CQ RC7KH KN75             *");
        assert_eq!(decode_line("20260104", "071615", (-10, 0.2, 1294), false, "BG5JGG SP8AJK KO10", '^'), "20260104_071615 -10  0.2 1294 ~ BG5JGG SP8AJK KO10        ^");
        assert_eq!(decode_line("20260104", "085622", (-5, 0.0, 1371), true, "CQ WWA", '.'), "20260104_085622  -5  0.0 1371 : CQ WWA                    .");
        // 59 caratteri con l'indicatore; il messaggio oltre 26 e' troncato
        assert_eq!(decode_line("20261018", "043545", (-3, 0.1, 911), false, "CQ RA5O KO91", '^').len(), 59);
        assert_eq!(decode_line("20261018", "043545", (-3, 0.1, 911), false, "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123", ' ').len(), 58);
        // valori limite: SNR a tre cifre, frequenza a 4
        assert_eq!(decode_line("20261018", "043545", (0, 0.0, 2500), false, "73", ' '), "20261018_043545   0  0.0 2500 ~ 73");
    }

    #[test]
    fn tx_and_service_lines_have_the_exact_jtdx_shape() {
        let at = Utc::from_unix_ms(T + 17_739);
        assert_eq!(
            tx_line(&at, 0.0, 7_074_000, 1084, "FT8", "OA4ENG IU3ABC -14"),
            "20261018_043617.739(0)  Transmitting 7.074 MHz + 1084Hz  FT8:  OA4ENG IU3ABC -14                    "
        );
        assert_eq!(tx_line(&at, -0.22, 21_074_000, 1500, "FT4", "CQ DX IU3ABC IO91").len(), "20261018_043617.739(-0.22)  Transmitting 21.074 MHz + 1500Hz  FT4:  ".len() + 37);
        let logged = Utc::from_unix_ms(T + 195_044);
        assert_eq!(service_line(&logged, 0.0, "QSO logged: OA4ENG"), "20261018_043915.044(0)  QSO logged: OA4ENG");
        assert_eq!(frequency_line(&Utc::from_unix_ms(T - 9_000), 7_074_055, "FT8", "Aurora v0.1.0"), "20261018_043551  7.074055 MHz  FT8 Aurora v0.1.0");
        assert_eq!(header_line(&Utc::from_unix_ms(T + 8_000), 7_074_000, "FT8"), "20261018_043608  7.074 MHz  FT8");
    }

    #[test]
    fn the_file_is_monthly_crlf_and_appended_with_a_header_before_the_first_decode() {
        let dir = std::env::temp_dir().join(format!("aurora-alltxt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella");
        let mut log = AllTxt::new(&dir, "Aurora v0.1.0");
        log.write_frequency_change(T - 1_000, 14_074_000, "FT8");
        log.write_frequency_change(T - 500, 14_074_000, "FT8"); // uguale: niente
        log.write_decodes(T, 14_074_000, "FT8", &[(-9, 0.1, 1908, "R8CEC SV2YC KN10"), (-3, -0.1, 911, "CQ RA5O KO91")]);
        log.write_decodes(T + 15_000, 14_074_000, "FT8", &[(-12, 0.3, 700, "73")]);
        log.write_tx(T + 30_040, 0.0, 14_074_000, 1500, "FT8", "RA5O IU3ABC -09");
        log.write_qso_logged(T + 30_046, 0.0, "RA5O");
        let text = std::fs::read(dir.join("202610_ALL.TXT")).expect("file del mese");
        let text = String::from_utf8(text).expect("ascii");
        let expected = [
            "20261018_043559  14.074 MHz  FT8 Aurora v0.1.0",
            "20261018_043600  14.074 MHz  FT8",
            "20261018_043600  -9  0.1 1908 ~ R8CEC SV2YC KN10",
            "20261018_043600  -3 -0.1  911 ~ CQ RA5O KO91",
            "20261018_043615 -12  0.3  700 ~ 73",
            // il messaggio di trasmissione e' riempito di spazi fino a 37 colonne
            &format!("20261018_043630.040(0)  Transmitting 14.074 MHz + 1500Hz  FT8:  {:<37}", "RA5O IU3ABC -09"),
            "20261018_043630.046(0)  QSO logged: RA5O",
        ];
        assert_eq!(text, expected.iter().map(|l| format!("{l}\r\n")).collect::<String>());
        // il mese dopo, un altro file
        log.write_qso_logged(T + 20 * 86_400_000, 0.0, "K1ABC");
        assert!(dir.join("202611_ALL.TXT").exists());
        // passaggio di mezzanotte: nuova riga di frequenza prima della prima decodifica
        let mut log = AllTxt::new(&dir, "Aurora");
        let midnight = T - 4 * 3_600_000 - 36 * 60_000; // 2026-10-18 00:00:00
        log.write_decodes(midnight - 15_000, 14_074_000, "FT8", &[(-1, 0.0, 1000, "A")]);
        log.write_decodes(midnight, 14_074_000, "FT8", &[(-1, 0.0, 1000, "B")]);
        let day = std::fs::read_to_string(dir.join("202610_ALL.TXT")).expect("file");
        let tail: Vec<&str> = day.lines().rev().take(4).collect();
        assert_eq!(tail[1], "20261018_000000  14.074 MHz  FT8", "{tail:?}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_disabled_log_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("aurora-alltxt-off-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella");
        let mut log = AllTxt::new(&dir, "Aurora");
        log.enabled = false;
        log.write_decodes(T, 14_074_000, "FT8", &[(-9, 0.1, 1908, "CQ A B")]);
        log.write_tx(T, 0.0, 14_074_000, 1500, "FT8", "CQ A B");
        assert!(std::fs::read_dir(&dir).expect("cartella").next().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
