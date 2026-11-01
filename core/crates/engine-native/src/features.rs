//! Funzioni di WSJT-X 3.x portate nel motore proprio: momento di avvio della decodifica FT8, full duplex, codice OTP del Fox (Hound classico)
//! e Wait and Reply / Wait and Call. Le regole sono quelle dei sorgenti GPL di WSJT-X 3.3.0-beta5 (citati funzione per funzione).

/// Momento di avvio della decodifica FT8 (`m_ft8DecoderStart` e `fixStop` di `widgets/mainwindow.cpp`). WSJT-X conta i mezzi simboli da 0,288 s
/// (`m_hsymStop`): «Presto» 48 (13,82 s), «Normale» 49 (14,11 s, il valore di fabbrica `m_ft8DecoderStart {3}`), «Tardi» 50 (14,40 s).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecoderStart {
    Early,
    #[default]
    Normal,
    Late,
}

/// Durata di un mezzo simbolo di WSJT-X (`hsym`), in microsecondi.
const HSYM_US: u64 = 288_000;

impl DecoderStart {
    /// Dal valore delle impostazioni (`early`, `normal`, `late`); altro = normale.
    pub fn parse(value: &str) -> DecoderStart {
        match value.trim().to_ascii_lowercase().as_str() {
            "early" => DecoderStart::Early,
            "late" => DecoderStart::Late,
            _ => DecoderStart::Normal,
        }
    }

    pub fn code(self) -> u8 {
        match self {
            DecoderStart::Early => 0,
            DecoderStart::Normal => 1,
            DecoderStart::Late => 2,
        }
    }

    pub fn from_code(code: u8) -> DecoderStart {
        match code {
            0 => DecoderStart::Early,
            2 => DecoderStart::Late,
            _ => DecoderStart::Normal,
        }
    }

    /// `m_hsymStop` di WSJT-X per questa scelta.
    pub fn hsym_stop(self) -> u64 {
        match self {
            DecoderStart::Early => 48,
            DecoderStart::Normal => 49,
            DecoderStart::Late => 50,
        }
    }

    /// Millisecondi dall'inizio del periodo FT8 in cui parte la decodifica.
    pub fn start_ms(self) -> u64 {
        self.hsym_stop() * HSYM_US / 1000
    }

    /// Millisecondi prima della fine del periodo FT8 (15 s) in cui il periodo si consegna al decoder.
    pub fn early_ms(self) -> u64 {
        15_000 - self.start_ms()
    }
}

/// Impostazioni della verifica del Fox (scheda Generali; `OTPUrl` e `ShowOTP` di `Configuration.cpp`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtpSettings {
    /// Mostra le righe OTP tra i decode (`ShowOTP`, di fabbrica spento).
    pub show: bool,
    /// Chiedi al servizio se il codice e' valido (in WSJT-X sempre, con Hound attivo).
    pub verify: bool,
    /// Indirizzo del servizio (`OTPUrl`, di fabbrica `https://www.9dx.cc`, `FOXVERIFIER_DEFAULT_BASE_URL`).
    pub url: String,
}

/// Indirizzo di fabbrica del servizio di verifica (`Network/FoxVerifier.hpp`).
pub const DEFAULT_OTP_URL: &str = "https://www.9dx.cc";

impl Default for OtpSettings {
    fn default() -> Self {
        OtpSettings { show: false, verify: true, url: DEFAULT_OTP_URL.to_string() }
    }
}

/// Un codice OTP ricevuto dal Fox, da verificare.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoxOtp {
    pub call: String,
    pub code: String,
    /// Frequenza audio del messaggio (per la riga «verified»).
    pub freq_hz: u32,
    /// Inizio del periodo in cui e' stato ricevuto (ms dal 1970, UTC): l'ora della richiesta.
    pub slot_start_ms: u64,
}

/// Il codice OTP del Fox classico: testo libero `CALL.123456` (nominativo da 2 a 6 caratteri, `stdPayloadRe` di `processSFoxVerification`,
/// `mainwindow.cpp`). Il formato SuperFox (`$VERIFY$ CALL 123456`) non e' qui: dipende dal ricevitore SuperFox, non ancora nel motore.
pub fn parse_fox_otp(text: &str) -> Option<(String, String)> {
    let (call, code) = text.trim().split_once('.')?;
    let call_ok = (2..=6).contains(&call.len()) && call.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let code_ok = code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit());
    (call_ok && code_ok).then(|| (call.to_string(), code.to_string()))
}

/// Cosa fare di una riga OTP in Hound: se si nasconde (`filtered` di WSJT-X: «Mostra messaggi OTP» spento o codice `000000`) e se si chiede la verifica
/// (codice diverso da `000000` e verifica accesa).
pub fn otp_action(code: &str, settings: &OtpSettings) -> (bool, bool) {
    let test = code == "000000";
    (!settings.show || test, !test && settings.verify)
}

/// La riga che WSJT-X scrive con l'esito (`FoxVerifier::formatDecodeMessage`): «CALL verified» o «CALL invalid»; `None` se la risposta non e' nessuna delle due.
pub fn verdict_of(body: &str) -> Option<bool> {
    let body = body.trim();
    if body.ends_with(" VERIFIED") {
        Some(true)
    } else if body.ends_with(" INVALID") {
        Some(false)
    } else {
        None
    }
}

/// Wait and Call: un messaggio del DX che dice che e' libero (`planWaitReplyCall` di `DecodedMessageReaction.cpp`: `CQ HIS`, `CQ DX HIS`, `HIS RR73`,
/// `HIS RRR`, `HIS 73`).
pub fn frees_the_dx(text: &str, his_call: &str) -> bool {
    let text = text.replace(['<', '>'], "");
    let padded = format!(" {text} ");
    [format!(" CQ {his_call} "), format!(" CQ DX {his_call} "), format!(" {his_call} RR73 "), format!(" {his_call} RRR "), format!(" {his_call} 73 ")]
        .iter()
        .any(|p| padded.contains(p.as_str()))
}

/// Wait and Reply: il DX mi scrive (`MYCALL HISCALL ...`) e non e' un saluto finale (`!text.contains("73 ")` di `planWaitReplyCall`).
pub fn dx_calls_me(text: &str, my_call: &str, his_call: &str) -> bool {
    let text = text.replace(['<', '>'], "");
    let padded = format!(" {text} ");
    padded.contains(&format!(" {my_call} {his_call} ")) && !format!("{text} ").contains("73 ")
}

/// Per quanti periodi Wait and Call continua a chiamare dopo che il DX si e' liberato (`StartWaitCallTimer`, 6,2 periodi: fino a tre chiamate).
pub const WAIT_CALL_PERIODS: f64 = 6.2;

/// Il periodo in cui si trasmette si segna qui (dall'ora dei blocchi di audio): senza «Full duplex» quel periodo non si decodifica (WSJT-X spegne
/// il monitor in trasmissione, `transmitDisplay`: `if (m_monitoring && !actionFull_Duplex_Mode) monitor (false)`).
#[derive(Debug, Default)]
pub struct TxPeriods {
    marked: Vec<u64>,
}

impl TxPeriods {
    /// Un blocco di audio arrivato a `at_ms` mentre si trasmetteva: il suo periodo e' «di trasmissione».
    pub fn note(&mut self, at_ms: u64, period_ms: u64) {
        if period_ms == 0 {
            return;
        }
        // l'ultimo secondo del periodo non conta: il PTT puo' chiudersi qualche istante prima che cominci il periodo di trasmissione
        if at_ms % period_ms + 1_000 > period_ms {
            return;
        }
        let start = at_ms / period_ms * period_ms;
        if !self.marked.contains(&start) {
            self.marked.push(start);
        }
        if self.marked.len() > 8 {
            self.marked.remove(0);
        }
    }

    /// Il periodo che comincia a `start_ms` ha avuto una trasmissione.
    pub fn had_tx(&self, start_ms: u64) -> bool {
        self.marked.contains(&start_ms)
    }

    pub fn clear(&mut self) {
        self.marked.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_decoder_start_follows_the_wsjtx_half_symbols() {
        assert_eq!(DecoderStart::Early.start_ms(), 13_824);
        assert_eq!(DecoderStart::Normal.start_ms(), 14_112);
        assert_eq!(DecoderStart::Late.start_ms(), 14_400);
        assert_eq!(DecoderStart::default(), DecoderStart::Normal);
        assert_eq!(DecoderStart::parse("LATE"), DecoderStart::Late);
        assert_eq!(DecoderStart::parse("boh"), DecoderStart::Normal);
        for d in [DecoderStart::Early, DecoderStart::Normal, DecoderStart::Late] {
            assert_eq!(DecoderStart::from_code(d.code()), d);
        }
    }

    #[test]
    fn only_the_classic_fox_code_is_recognised() {
        assert_eq!(parse_fox_otp("K8R.920749"), Some(("K8R".into(), "920749".into())));
        assert_eq!(parse_fox_otp("VP2X.000000"), Some(("VP2X".into(), "000000".into())));
        assert_eq!(parse_fox_otp("K8R.92074"), None);
        assert_eq!(parse_fox_otp("TOOLONG7.123456"), None);
        assert_eq!(parse_fox_otp("$VERIFY$ VP2X/K1JT 920749"), None);
        assert_eq!(parse_fox_otp("CQ K8R FN42"), None);
    }

    #[test]
    fn test_codes_are_hidden_and_never_checked() {
        let on = OtpSettings::default();
        assert_eq!(otp_action("000000", &on), (true, false));
        assert_eq!(otp_action("920749", &on), (true, true));
        let shown = OtpSettings { show: true, ..OtpSettings::default() };
        assert_eq!(otp_action("920749", &shown), (false, true));
        let off = OtpSettings { verify: false, ..OtpSettings::default() };
        assert_eq!(otp_action("920749", &off), (true, false));
    }

    #[test]
    fn the_service_answer_is_read_like_wsjtx() {
        assert_eq!(verdict_of("172100 -00  0.0  750 ~  K8R VERIFIED\n"), Some(true));
        assert_eq!(verdict_of("K8R INVALID"), Some(false));
        assert_eq!(verdict_of("errore"), None);
    }

    #[test]
    fn wait_triggers_match_the_wsjtx_plan() {
        assert!(frees_the_dx("CQ K1ABC FN42", "K1ABC"));
        assert!(frees_the_dx("CQ DX K1ABC FN42", "K1ABC"));
        assert!(frees_the_dx("W9XYZ K1ABC RR73", "K1ABC"));
        assert!(frees_the_dx("W9XYZ <K1ABC> 73", "K1ABC"));
        assert!(!frees_the_dx("W9XYZ K1ABC -07", "K1ABC"));
        assert!(!frees_the_dx("K1ABC W9XYZ RR73", "K1ABC"));
        assert!(dx_calls_me("IU3ABC K1ABC -07", "IU3ABC", "K1ABC"));
        assert!(!dx_calls_me("IU3ABC K1ABC RR73", "IU3ABC", "K1ABC"), "RR73 contiene «73 »: WSJT-X non riprende");
        assert!(dx_calls_me("IU3ABC K1ABC R-05", "IU3ABC", "K1ABC"));
        assert!(!dx_calls_me("IU3ABC K1ABC 73", "IU3ABC", "K1ABC"));
        assert!(!dx_calls_me("W9XYZ K1ABC -07", "IU3ABC", "K1ABC"));
    }

    #[test]
    fn transmit_periods_are_remembered_by_their_start() {
        let mut p = TxPeriods::default();
        p.note(1_005_400, 15_000);
        p.note(1_017_000, 15_000);
        // il PTT acceso a 14,5 s del periodo di ricezione non lo segna
        p.note(1_034_500, 15_000);
        assert!(!p.had_tx(1_020_000));
        assert!(p.had_tx(1_005_000));
        assert!(!p.had_tx(1_020_000));
        p.clear();
        assert!(!p.had_tx(1_005_000));
    }
}
