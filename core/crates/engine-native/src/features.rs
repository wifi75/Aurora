//! Funzioni di WSJT-X 3.x portate nel motore proprio: momento di avvio della decodifica FT8, full duplex, codice OTP del Fox (Hound classico e SuperFox)
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
/// `mainwindow.cpp`). Il formato SuperFox e' `parse_superfox_verify`.
pub fn parse_fox_otp(text: &str) -> Option<(String, String)> {
    let (call, code) = text.trim().split_once('.')?;
    let call_ok = (2..=6).contains(&call.len()) && call.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let code_ok = code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit());
    (call_ok && code_ok).then(|| (call.to_string(), code.to_string()))
}

/// La frequenza audio che WSJT-X da' alla riga di verifica di un SuperFox (`hz = 750` in `processSFoxVerification`).
pub const SUPERFOX_VERIFY_HZ: u32 = 750;

/// Il codice OTP di un SuperFox: la riga `$VERIFY$ CALL 123456` che il ricevitore aggiunge a ogni pacchetto (`sfox_unpack.f90`), con il nominativo
/// da 2 a 13 caratteri (anche con `/`) e spazi qualsiasi tra le parti (`verifyPayloadRe` di `processSFoxVerification`).
pub fn parse_superfox_verify(text: &str) -> Option<(String, String)> {
    let rest = text.trim().strip_prefix("$VERIFY$")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let mut parts = rest.split_whitespace();
    let (call, code) = (parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let call_ok = (2..=13).contains(&call.len()) && call.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'/');
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

/// Criterio di Wait and Pounce («CQ:» di WSJT-X, `AutoRespondPolicy.hpp`): a quale CQ rispondere da soli.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PouncePolicy {
    /// «Nessuno»: Wait and Pounce non si arma (WSJT-X chiede di scegliere un criterio).
    None,
    /// Il primo CQ decodificato.
    #[default]
    First,
    /// Il CQ con il locatore piu' lontano («Max Dist»; senza locatore non si sceglie).
    MaxDistance,
    /// Il CQ con il segnale piu' forte («Max dB»).
    MaxSignal,
    /// Il CQ con il segnale piu' debole («Min dB»).
    MinSignal,
}

impl PouncePolicy {
    pub fn parse(value: &str) -> PouncePolicy {
        match value {
            "none" => PouncePolicy::None,
            "max_dist" => PouncePolicy::MaxDistance,
            "max_db" => PouncePolicy::MaxSignal,
            "min_db" => PouncePolicy::MinSignal,
            _ => PouncePolicy::First,
        }
    }

    pub fn code(self) -> u8 {
        match self {
            PouncePolicy::None => 0,
            PouncePolicy::First => 1,
            PouncePolicy::MaxDistance => 2,
            PouncePolicy::MaxSignal => 3,
            PouncePolicy::MinSignal => 4,
        }
    }

    pub fn from_code(code: u8) -> PouncePolicy {
        match code {
            0 => PouncePolicy::None,
            2 => PouncePolicy::MaxDistance,
            3 => PouncePolicy::MaxSignal,
            4 => PouncePolicy::MinSignal,
            _ => PouncePolicy::First,
        }
    }
}

/// Wait and Pounce (`updateRespondTarget` di `widgets/mainwindow.cpp` e `AutoRespondScoring.hpp` di WSJT-X 3.3): fra i CQ del periodo sceglie quello
/// a cui rispondere. Come WSJT-X si scartano le stazioni gia' chiamate o collegate in questa sessione (`txLog`, `skip`) e, con «Max Dist», i CQ senza
/// locatore; a parita' vince il primo (il confronto e' stretto come `considerDistance`/`considerMaximumDb`). Restituisce l'indice del decode scelto.
pub fn pounce_pick(policy: PouncePolicy, decoded: &[(String, i32, u32)], my_grid: &str, skip: &dyn Fn(&str) -> bool) -> Option<usize> {
    use qso_native::message::{Message, parse};
    let mut best: Option<(usize, i32)> = None;
    for (i, (text, snr, _)) in decoded.iter().enumerate() {
        let Message::Cq { call, grid, .. } = parse(&text.replace(['<', '>'], "")) else { continue };
        if skip(&call) {
            continue;
        }
        let score = match policy {
            PouncePolicy::None => return None,
            PouncePolicy::First => return Some(i),
            PouncePolicy::MaxDistance => match grid {
                Some(g) => qso_native::history::grid_distance_km(my_grid, &g),
                None => continue,
            },
            PouncePolicy::MaxSignal => *snr,
            PouncePolicy::MinSignal => -*snr,
        };
        if best.is_none_or(|(_, b)| score > b) {
            best = Some((i, score));
        }
    }
    best.map(|(i, _)| i)
}

/// Lo scambio di contest in un messaggio diretto a me dal DX (`MYCALL HISCALL [R] scambio`), nelle forme dei messaggi FT8 di WSJT-X:
/// rapporto e stato/provincia o seriale del RTTY Roundup (`579 WI`, `559 0013`), classe e sezione di Field Day (`2A EMA`), rapporto+seriale e
/// locatore dell'EU VHF (`570123 JN65AB`). `None` se non e' uno scambio (rapporti, locatori, saluti).
pub fn contest_exchange(text: &str, my_call: &str, his_call: &str) -> Option<String> {
    let text = text.replace(['<', '>'], "").to_ascii_uppercase();
    let w: Vec<&str> = text.split_whitespace().collect();
    if w.len() < 4 || !w[0].eq_ignore_ascii_case(my_call) || !w[1].eq_ignore_ascii_case(his_call) {
        return None;
    }
    let rest: &[&str] = if w[2] == "R" { &w[3..] } else { &w[2..] };
    let [a, b] = rest else { return None };
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let letters = |s: &str| !s.is_empty() && s.len() <= 3 && s.chars().all(|c| c.is_ascii_alphabetic());
    let rst = a.len() == 3 && a.starts_with('5') && digits(a);
    let class = a.len() >= 2 && a.len() <= 3 && digits(&a[..a.len() - 1]) && a.ends_with(|c: char| ('A'..='F').contains(&c));
    let vhf = a.len() >= 5 && digits(a) && qso_native::message::is_valid_locator(b);
    ((rst && (letters(b) || (digits(b) && b.len() <= 4))) || (class && letters(b)) || vhf).then(|| format!("{a} {b}"))
}

/// Hound: il Fox trasmette sotto i 1000 Hz e i Hound chiamano fra 1000 e 3000 Hz (`QRandomGenerator::bounded (1000, 2999)` in `widgets/mainwindow.cpp`
/// di WSJT-X 3.3, «Hound randomized range: 1000-3000 Hz»). All'accensione di Hound una frequenza Tx sotto 1000 Hz si sposta in quell'intervallo;
/// `seed` sceglie il punto (WSJT-X lo sceglie a caso). Restituisce la frequenza da usare.
pub fn hound_tx_hz(tx_hz: u32, seed: u64) -> u32 {
    if tx_hz >= HOUND_MIN_TX_HZ { tx_hz } else { HOUND_MIN_TX_HZ + (seed % u64::from(HOUND_MAX_TX_HZ - HOUND_MIN_TX_HZ)) as u32 }
}

/// Una riga trasmessa da un Fox di WSJT-X in modo Fox/Hound: il messaggio DXpedition a due Hound (`K1ABC RR73; W9XYZ <KH1/KH7Z> -08`, `i3=0 n3=1`
/// di `lib/77bit/packjt77.f90`). Restituisce il nominativo del Fox. Le risposte multiple di MSHV sono messaggi standard separati e non lo sono.
pub fn fox_sender(text: &str) -> Option<String> {
    let (first, after) = text.split_once("; ")?;
    let first: Vec<&str> = first.split_whitespace().collect();
    let after: Vec<&str> = after.split_whitespace().collect();
    if first.len() != 2 || first[1] != "RR73" || after.len() != 3 {
        return None;
    }
    let fox = after[1].trim_start_matches('<').trim_end_matches('>');
    (!fox.is_empty() && fox != "...").then(|| fox.to_string())
}

/// L'intervallo di chiamata dei Hound (`[1000, 2999)` Hz come WSJT-X).
pub const HOUND_MIN_TX_HZ: u32 = 1000;
pub const HOUND_MAX_TX_HZ: u32 = 2999;

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

    /// Scambi di contest nei messaggi del DX: RTTY Roundup, Field Day, EU VHF; i messaggi normali no.
    #[test]
    fn contest_exchanges_are_read_from_the_dx_messages() {
        let x = |t: &str| contest_exchange(t, "IU3ABC", "K1ABC");
        assert_eq!(x("IU3ABC K1ABC 579 WI").as_deref(), Some("579 WI"));
        assert_eq!(x("IU3ABC K1ABC R 559 0013").as_deref(), Some("559 0013"));
        assert_eq!(x("<IU3ABC> K1ABC R 2A EMA").as_deref(), Some("2A EMA"));
        assert_eq!(x("IU3ABC K1ABC 570123 JN65AB").as_deref(), Some("570123 JN65AB"));
        assert_eq!(x("IU3ABC K1ABC -07"), None);
        assert_eq!(x("IU3ABC K1ABC FN42"), None);
        assert_eq!(x("IU3ABC W9XYZ 579 WI"), None, "non dal DX");
    }

    /// Le righe del Fox di WSJT-X si riconoscono; le risposte multiple standard (MSHV, V26JJ) no.
    #[test]
    fn fox_lines_are_recognised_and_mshv_replies_are_not() {
        assert_eq!(fox_sender("K1ABC RR73; W9XYZ <KH1/KH7Z> -08").as_deref(), Some("KH1/KH7Z"));
        assert_eq!(fox_sender("K1ABC RR73; IU3ABC <V26JJ> -12").as_deref(), Some("V26JJ"));
        assert_eq!(fox_sender("IU3WJQ V26JJ R+03"), None);
        assert_eq!(fox_sender("R2PA V26JJ RR73"), None);
        assert_eq!(fox_sender("CQ V26JJ FK97"), None);
    }

    /// Hound: sotto 1000 Hz si sale nell'intervallo dei Hound, sopra si resta dove si e'.
    #[test]
    fn hound_moves_the_tx_frequency_above_the_fox() {
        assert_eq!(hound_tx_hz(1500, 7), 1500);
        assert_eq!(hound_tx_hz(1000, 7), 1000);
        for seed in [0, 1, 1998, 1999, 123_456_789] {
            let hz = hound_tx_hz(600, seed);
            assert!((HOUND_MIN_TX_HZ..HOUND_MAX_TX_HZ).contains(&hz), "{hz}");
        }
    }

    /// Wait and Pounce: i quattro criteri di WSJT-X sugli stessi CQ, con le stazioni gia' chiamate scartate.
    #[test]
    fn wait_and_pounce_picks_the_cq_by_the_chosen_policy() {
        let d: Vec<(String, i32, u32)> = vec![
            ("W9XYZ K1ABC R-07".into(), 5, 900),
            ("CQ DL1AAA JO62".into(), -3, 1000),
            ("CQ JA1ZZZ PM95".into(), -18, 1200),
            ("CQ VK2AAA".into(), 2, 1400),
            ("CQ DX G4BBB IO91".into(), -20, 1600),
        ];
        let none = |_: &str| false;
        assert_eq!(pounce_pick(PouncePolicy::None, &d, "JN65", &none), None);
        assert_eq!(pounce_pick(PouncePolicy::First, &d, "JN65", &none), Some(1));
        assert_eq!(pounce_pick(PouncePolicy::MaxDistance, &d, "JN65", &none), Some(2), "il Giappone e' il piu' lontano; VK2AAA non ha locatore");
        assert_eq!(pounce_pick(PouncePolicy::MaxSignal, &d, "JN65", &none), Some(3));
        assert_eq!(pounce_pick(PouncePolicy::MinSignal, &d, "JN65", &none), Some(4));
        let worked = |c: &str| c == "DL1AAA" || c == "G4BBB";
        assert_eq!(pounce_pick(PouncePolicy::First, &d, "JN65", &worked), Some(2));
        assert_eq!(pounce_pick(PouncePolicy::MinSignal, &d, "JN65", &worked), Some(2));
        for p in [PouncePolicy::None, PouncePolicy::First, PouncePolicy::MaxDistance, PouncePolicy::MaxSignal, PouncePolicy::MinSignal] {
            assert_eq!(PouncePolicy::from_code(p.code()), p);
        }
        assert_eq!(PouncePolicy::parse("max_dist"), PouncePolicy::MaxDistance);
    }

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
        assert_eq!(parse_superfox_verify("$VERIFY$ VP2X/K1JT 920749"), Some(("VP2X/K1JT".into(), "920749".into())));
        assert_eq!(parse_superfox_verify("$VERIFY$   K8R  000000"), Some(("K8R".into(), "000000".into())));
        assert_eq!(parse_superfox_verify("$VERIFY$ K8R 92074"), None);
        assert_eq!(parse_superfox_verify("$VERIFY$K8R 920749"), None);
        assert_eq!(parse_superfox_verify("K8R.920749"), None);
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
