//! Lettura (solo lettura) delle impostazioni di JTDX dal suo `JTDX.ini` (formato QSettings).
//! Si espongono solo le voci elencate in `SHOWN`: niente password, geometrie delle finestre o blocchi binari.

use std::{collections::BTreeMap, path::Path};

/// Le voci di JTDX che Aurora mostra, per nome (uniche tra le sezioni `Common` e `Configuration`).
pub const SHOWN: &[&str] = &[
    "MyCall", "MyGrid", "Language", "PromptToLog", "AutoQSOLogging", "ClearCallGrid", "TxWatchdogTimer", "TuneTimer",
    "Miles", "showDistance", "countryName", "UseDarkStyle", "MonitorOFF", "MonitorLastUsed", "Rig", "PTTport",
    "CATSerialPort", "CATSerialRate", "Polling", "CATForceDTR", "CATForceRTS", "TCIAudio", "SoundInName",
    "SoundOutName", "AudioInputChannel", "AudioOutputChannel", "SaveDir", "SaveWav", "UDPServer", "UDPServerPort",
    "AcceptUDPRequests", "AcceptUDPReplyMessages", "EnableUDP1adifSending", "UDP2Server", "UDP2ServerPort",
    "EnableUDP2adifBroadcast", "TCPServer", "TCPServerPort", "EnableTCPConnection", "udpWindowToFront",
    "udpWindowRestore", "ApplyFiltersToUDPmessages", "preventFalseUDPspots", "PSKReporter", "AllowSpotsDXSummit",
    "WriteDecodedALLTXT", "WriteDecodedDebugALLTXT", "AutoSequence", "SingleShotQSO", "SkipTx1", "AnswerWorkedB4", "CallPriorityCQ",
    "MaxDistance", "StrictDirectionalCQ", "SeqHaltTxReplyOther", "SeqAnswerCQCount", "SeqAnswerCQCounterValue",
    "SeqSentRReportCount", "SeqSentRReportCounterValue", "SeqSentRR7373Count", "SeqSentRR7373CounterValue",
    "AlertOnMyCall", "AlertOnNewDXCC", "AlertOnNewCQZ", "AlertOnDXCall", "BeepOnMyCall", "BeepOnNewDXCC",
    "BeepOnNewCQZ", "BeepOnNewITUZ", "BeepOnNewGrid", "BeepOnNewCall", "NDepth", "NFT4Depth", "NFT8Cycles",
    "NFT8SWLCycles", "FT8Sensitivity", "FT8DecoderEarlyStart", "FT8WideDXCallSearch", "nTrials", "Aggressive",
    "EnableHoundMode", "SWLMode", "AGCcompensation", "AutoErase", "HideFT8Dupes",
    "SeqAnswerInCallCount", "newCQZ", "newCQZBand", "newCQZBandMode", "newITUZ", "newITUZBand", "newITUZBandMode", "newDXCC",
    "newDXCCBand", "newDXCCBandMode", "newGrid", "newGridBand", "newGridBandMode", "newPx", "newPxBand", "newPxBandMode", "newCall",
    "newCallBand", "newCallBandMode", "newPotential", "workedColor", "workedStriked", "workedUnderlined", "workedDontShow", "txtColor",
    "OtherStandardMessagesMarker", "73RR73Marker", "redMarker", "blueMarker", "hidehintMarker", "BeepOnNewPx", "BeepOnFirstMsg",
    "AlertOnNewITUZ", "AlertOnNewGrid", "AlertOnNewPx", "AlertOnNewCQZOB", "AlertOnNewDXCCOB", "AlertOnNewITUZOB", "AlertOnNewGridOB",
    "AlertOnNewPxOB", "callsignLogFiltering", "gridLogFiltering", "InsertBlank", "Scroll", "HideTxMessages", "Tx2QSO", "countryPrefix",
    "showAzimuth", "AlignDistanceAzimuth", "Decode52", "TxQSYAllowed", "After73", "SeqAnswerInCallCounterValue", "AlignInt", "IDint",
    "timeFromLogFiltering",
];

/// Il tipo di ogni voce: bool, text, num oppure enum:<nome> (valori ammessi in `ini_edit`).
pub const FIELDS: &[(&str, &str)] = &[
    ("MyCall", "text"),
    ("MyGrid", "text"),
    ("Language", "text"),
    ("PromptToLog", "bool"),
    ("AutoQSOLogging", "bool"),
    ("ClearCallGrid", "bool"),
    ("TxWatchdogTimer", "num"),
    ("TuneTimer", "num"),
    ("Miles", "bool"),
    ("showDistance", "bool"),
    ("countryName", "bool"),
    ("UseDarkStyle", "bool"),
    ("MonitorOFF", "bool"),
    ("MonitorLastUsed", "bool"),
    ("Rig", "text"),
    ("PTTport", "text"),
    ("CATSerialPort", "text"),
    ("CATSerialRate", "num"),
    ("Polling", "num"),
    ("CATForceDTR", "bool"),
    ("CATForceRTS", "bool"),
    ("TCIAudio", "bool"),
    ("SoundInName", "text"),
    ("SoundOutName", "text"),
    ("AudioInputChannel", "text"),
    ("AudioOutputChannel", "text"),
    ("SaveDir", "text"),
    ("SaveWav", "enum:save"),
    ("UDPServer", "text"),
    ("UDPServerPort", "num"),
    ("AcceptUDPRequests", "bool"),
    ("AcceptUDPReplyMessages", "enum:reply"),
    ("EnableUDP1adifSending", "bool"),
    ("UDP2Server", "text"),
    ("UDP2ServerPort", "num"),
    ("EnableUDP2adifBroadcast", "bool"),
    ("TCPServer", "text"),
    ("TCPServerPort", "num"),
    ("EnableTCPConnection", "bool"),
    ("udpWindowToFront", "bool"),
    ("udpWindowRestore", "bool"),
    ("ApplyFiltersToUDPmessages", "bool"),
    ("preventFalseUDPspots", "bool"),
    ("PSKReporter", "bool"),
    ("AllowSpotsDXSummit", "bool"),
    ("WriteDecodedALLTXT", "bool"),
    ("WriteDecodedDebugALLTXT", "bool"),
    ("AutoSequence", "bool"),
    ("SingleShotQSO", "bool"),
    ("SkipTx1", "bool"),
    ("AnswerWorkedB4", "bool"),
    ("CallPriorityCQ", "bool"),
    ("MaxDistance", "bool"),
    ("StrictDirectionalCQ", "bool"),
    ("SeqHaltTxReplyOther", "bool"),
    ("SeqAnswerCQCount", "bool"),
    ("SeqAnswerCQCounterValue", "num"),
    ("SeqSentRReportCount", "bool"),
    ("SeqSentRReportCounterValue", "num"),
    ("SeqSentRR7373Count", "bool"),
    ("SeqSentRR7373CounterValue", "num"),
    ("AlertOnMyCall", "bool"),
    ("AlertOnNewDXCC", "bool"),
    ("AlertOnNewCQZ", "bool"),
    ("AlertOnDXCall", "bool"),
    ("BeepOnMyCall", "bool"),
    ("BeepOnNewDXCC", "bool"),
    ("BeepOnNewCQZ", "bool"),
    ("BeepOnNewITUZ", "bool"),
    ("BeepOnNewGrid", "bool"),
    ("BeepOnNewCall", "bool"),
    ("NDepth", "enum:depth"),
    ("NFT4Depth", "enum:depth"),
    ("NFT8Cycles", "num"),
    ("NFT8SWLCycles", "num"),
    ("FT8Sensitivity", "num"),
    ("FT8DecoderEarlyStart", "bool"),
    ("FT8WideDXCallSearch", "bool"),
    ("nTrials", "num"),
    ("Aggressive", "num"),
    ("EnableHoundMode", "bool"),
    ("SWLMode", "bool"),
    ("AGCcompensation", "bool"),
    ("AutoErase", "bool"),
    ("HideFT8Dupes", "bool"),
    ("SeqAnswerInCallCount", "bool"),
    ("newCQZ", "bool"),
    ("newCQZBand", "bool"),
    ("newCQZBandMode", "bool"),
    ("newITUZ", "bool"),
    ("newITUZBand", "bool"),
    ("newITUZBandMode", "bool"),
    ("newDXCC", "bool"),
    ("newDXCCBand", "bool"),
    ("newDXCCBandMode", "bool"),
    ("newGrid", "bool"),
    ("newGridBand", "bool"),
    ("newGridBandMode", "bool"),
    ("newPx", "bool"),
    ("newPxBand", "bool"),
    ("newPxBandMode", "bool"),
    ("newCall", "bool"),
    ("newCallBand", "bool"),
    ("newCallBandMode", "bool"),
    ("newPotential", "bool"),
    ("workedColor", "bool"),
    ("workedStriked", "bool"),
    ("workedUnderlined", "bool"),
    ("workedDontShow", "bool"),
    ("txtColor", "bool"),
    ("OtherStandardMessagesMarker", "bool"),
    ("73RR73Marker", "bool"),
    ("redMarker", "bool"),
    ("blueMarker", "bool"),
    ("hidehintMarker", "bool"),
    ("BeepOnNewPx", "bool"),
    ("BeepOnFirstMsg", "bool"),
    ("AlertOnNewITUZ", "bool"),
    ("AlertOnNewGrid", "bool"),
    ("AlertOnNewPx", "bool"),
    ("AlertOnNewCQZOB", "bool"),
    ("AlertOnNewDXCCOB", "bool"),
    ("AlertOnNewITUZOB", "bool"),
    ("AlertOnNewGridOB", "bool"),
    ("AlertOnNewPxOB", "bool"),
    ("callsignLogFiltering", "bool"),
    ("gridLogFiltering", "bool"),
    ("InsertBlank", "bool"),
    ("Scroll", "bool"),
    ("HideTxMessages", "bool"),
    ("Tx2QSO", "bool"),
    ("countryPrefix", "bool"),
    ("showAzimuth", "bool"),
    ("AlignDistanceAzimuth", "bool"),
    ("Decode52", "bool"),
    ("TxQSYAllowed", "bool"),
    ("After73", "bool"),
    ("SeqAnswerInCallCounterValue", "num"),
    ("AlignInt", "num"),
    ("IDint", "num"),
    ("timeFromLogFiltering", "text"),
];

/// Toglie le virgolette con cui QSettings protegge i valori con spazi o virgole.
fn unquote(value: &str) -> &str {
    value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value)
}

/// Legge le voci elencate in `SHOWN` dalle sezioni `Common` e `Configuration`; i valori binari (`@Variant`, `@ByteArray`) si saltano.
pub fn read_shown(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(parse_shown(&text))
}

pub fn parse_shown(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            inside = name == "Common" || name == "Configuration";
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), unquote(value.trim()));
        if SHOWN.contains(&key) && !value.starts_with('@') {
            out.insert(key.to_string(), value.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI: &str = "[MainWindow]\nRxFreq = 1\n[Common]\nMode = FT8\nNDepth = 3\nSWLMode = false\n[Configuration]\nUDPServer = 127.0.0.1\nUDPServerPort = 2334\nEQSLPasswd = segreta\nSoundInName = \"Microfono (2- USB Audio Device)\"\nPTTMethod = @Variant(\\0\\0)\nMyCall = AA1AA\n[LogQSO]\nMyCall = FUORI\n";

    #[test]
    fn reads_only_the_shown_keys_from_common_and_configuration() {
        let got = parse_shown(INI);
        assert_eq!(got["UDPServer"], "127.0.0.1");
        assert_eq!(got["UDPServerPort"], "2334");
        assert_eq!(got["NDepth"], "3");
        assert_eq!(got["MyCall"], "AA1AA", "quella della sezione LogQSO non conta");
        assert!(!got.contains_key("Mode"), "voce non elencata");
        assert!(!got.contains_key("RxFreq"), "altra sezione");
    }

    #[test]
    fn never_exposes_secrets_or_binary_blobs() {
        let got = parse_shown(INI);
        assert!(!got.contains_key("EQSLPasswd"));
        assert!(!got.contains_key("PTTMethod"));
        assert!(!SHOWN.iter().any(|k| k.to_lowercase().contains("pass") || k.to_lowercase().contains("pwd")));
    }

    #[test]
    fn strips_the_quotes_of_values_with_spaces() {
        assert_eq!(parse_shown(INI)["SoundInName"], "Microfono (2- USB Audio Device)");
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(read_shown(Path::new("non-esiste-12345.ini")).is_err());
    }
}
