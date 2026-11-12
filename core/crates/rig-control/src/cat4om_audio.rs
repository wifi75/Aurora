//! L'audio in rete di Cat4OM (manuale, sez. 8; riassunto in `docs/cat4om-protocollo.md`): la parte pura, senza rete.
//!
//! - lo stream si apre con `startAudioStream` (`direction` `rx` o `tx`, `sampleRateHz`, `channels`) e si usano i valori **restituiti**;
//! - l'audio viaggia sullo stesso WebSocket del Control in messaggi binari: intestazione di 32 byte little-endian (`C4OA`, versione 1,
//!   tipo 1 = rx / 2 = tx, codec 0 = PCM16, canali, `streamId`, frequenza, campioni per canale, `sequence`, `timestampUs`) e poi i
//!   campioni PCM a 16 bit interlacciati;
//! - in trasmissione si mandano blocchi da 20 ms (`rate / 50` campioni) con `sequence` da 1, alla frequenza negoziata (il server non
//!   ricampiona).
//!
//! Qui: codifica e lettura del blocco, conversione dei campioni, salti di `sequence`, scelta della frequenza, il ritmo dei blocchi da
//! trasmettere e i testi degli errori dello stream. Il collegamento li usa in `cat4om.rs`.

use std::time::Duration;

use serde_json::{Value, json};

/// Il nome del dispositivo nell'elenco degli ingressi e delle uscite della scheda Audio (presente quando il CAT e' Cat4OM).
pub const CAT4OM_AUDIO_NAME: &str = "Cat4OM (audio in rete)";

/// `C4OA`: i primi 4 byte di ogni blocco.
pub const MAGIC: [u8; 4] = *b"C4OA";
/// La versione del blocco descritta dal manuale.
pub const VERSION: u8 = 1;
/// L'intestazione: 32 byte.
pub const HEADER_LEN: usize = 32;
/// Il codec `pcm16` (l'unico del manuale).
pub const CODEC_PCM16: u8 = 0;
/// La frequenza chiesta per la ricezione: quella del decodificatore (12 kHz mono, circa 190 kbit/s).
pub const RX_PREFERRED_RATE: u32 = 12_000;
/// La frequenza di base del manuale: da chiedere se l'altra non e' accettata (`AUDIO_RATE_UNAVAILABLE`), e per la trasmissione.
pub const FALLBACK_RATE: u32 = 48_000;
/// La durata di un blocco da trasmettere (gli esempi del manuale).
pub const FRAME: Duration = Duration::from_millis(20);
/// Uno stream di trasmissione senza audio per 60 s (a radio non in trasmissione) lo chiude il server: oltre questo tempo senza audio
/// lo si riapre prima del PTT (un secondo `startAudioStream` sostituisce il vecchio, quindi non costa nulla).
pub const TX_IDLE_REOPEN: Duration = Duration::from_secs(50);
/// Al massimo tanti blocchi persi si rimpiazzano con silenzio (1 s a 20 ms): oltre, il buco resta e lo si scrive solo nel registro.
pub const MAX_FILL_FRAMES: u32 = 50;

/// La direzione di uno stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Rx,
    Tx,
}

impl Direction {
    /// Il valore di `direction` e di `streamKind`.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Rx => "rx",
            Direction::Tx => "tx",
        }
    }

    fn kind(self) -> u8 {
        match self {
            Direction::Rx => 1,
            Direction::Tx => 2,
        }
    }

    fn from_kind(kind: u8) -> Option<Direction> {
        match kind {
            1 => Some(Direction::Rx),
            2 => Some(Direction::Tx),
            _ => None,
        }
    }

    pub fn parse(text: &str) -> Option<Direction> {
        match text {
            "rx" => Some(Direction::Rx),
            "tx" => Some(Direction::Tx),
            _ => None,
        }
    }
}

/// Un blocco audio letto o da mandare. I campioni sono interlacciati (`channels` per istante).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFrame {
    pub direction: Direction,
    pub channels: u8,
    pub stream_id: u32,
    pub sample_rate: u32,
    pub sequence: u32,
    pub timestamp_us: u64,
    pub samples: Vec<i16>,
}

impl AudioFrame {
    /// I campioni per canale.
    pub fn sample_count(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }
}

/// Perche' un blocco si scarta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Meno di 32 byte.
    TooShort,
    /// Non comincia con `C4OA`.
    BadMagic,
    UnknownVersion(u8),
    /// Codec diverso da PCM16: va rifiutato (manuale).
    UnknownCodec(u8),
    BadKind(u8),
    BadChannels(u8),
    /// Il carico non corrisponde a campioni x canali x 2 byte.
    BadLength,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::TooShort => write!(f, "blocco audio troppo corto"),
            FrameError::BadMagic => write!(f, "blocco audio senza C4OA"),
            FrameError::UnknownVersion(v) => write!(f, "versione del blocco audio sconosciuta ({v})"),
            FrameError::UnknownCodec(c) => write!(f, "codec audio sconosciuto ({c})"),
            FrameError::BadKind(k) => write!(f, "tipo di stream audio sconosciuto ({k})"),
            FrameError::BadChannels(c) => write!(f, "numero di canali non valido ({c})"),
            FrameError::BadLength => write!(f, "lunghezza del blocco audio sbagliata"),
        }
    }
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&bytes[at..at + 4]);
    u32::from_le_bytes(b)
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(b)
}

/// Legge un messaggio binario del Control: controlla magia, versione, tipo, codec, canali e lunghezza.
pub fn decode_frame(bytes: &[u8]) -> Result<AudioFrame, FrameError> {
    if bytes.len() < HEADER_LEN {
        return Err(FrameError::TooShort);
    }
    if bytes[0..4] != MAGIC {
        return Err(FrameError::BadMagic);
    }
    if bytes[4] != VERSION {
        return Err(FrameError::UnknownVersion(bytes[4]));
    }
    let direction = Direction::from_kind(bytes[5]).ok_or(FrameError::BadKind(bytes[5]))?;
    if bytes[6] != CODEC_PCM16 {
        return Err(FrameError::UnknownCodec(bytes[6]));
    }
    let channels = bytes[7];
    if !(channels == 1 || channels == 2) {
        return Err(FrameError::BadChannels(channels));
    }
    let count = u32_at(bytes, 16) as usize;
    let payload = &bytes[HEADER_LEN..];
    if count.checked_mul(usize::from(channels) * 2) != Some(payload.len()) {
        return Err(FrameError::BadLength);
    }
    Ok(AudioFrame {
        direction,
        channels,
        stream_id: u32_at(bytes, 8),
        sample_rate: u32_at(bytes, 12),
        sequence: u32_at(bytes, 20),
        timestamp_us: u64_at(bytes, 24),
        samples: payload.chunks_exact(2).map(|p| i16::from_le_bytes([p[0], p[1]])).collect(),
    })
}

/// Scrive un blocco (intestazione e campioni interlacciati).
pub fn encode_frame(frame: &AudioFrame) -> Vec<u8> {
    let channels = frame.channels.max(1);
    let count = (frame.samples.len() / usize::from(channels)) as u32;
    let mut out = Vec::with_capacity(HEADER_LEN + frame.samples.len() * 2);
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    out.push(frame.direction.kind());
    out.push(CODEC_PCM16);
    out.push(channels);
    out.extend_from_slice(&frame.stream_id.to_le_bytes());
    out.extend_from_slice(&frame.sample_rate.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&frame.sequence.to_le_bytes());
    out.extend_from_slice(&frame.timestamp_us.to_le_bytes());
    for s in &frame.samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// I campioni mono in virgola mobile (-1..1): con due canali la media dei due.
pub fn to_mono(frame: &AudioFrame) -> Vec<f32> {
    let channels = usize::from(frame.channels.max(1));
    frame.samples.chunks_exact(channels).map(|c| c.iter().map(|s| f32::from(*s)).sum::<f32>() / (channels as f32 * 32_768.0)).collect()
}

/// Da virgola mobile a PCM a 16 bit (fuori da -1..1 si taglia).
pub fn to_pcm16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|s| (s.clamp(-1.0, 1.0) * 32_767.0).round() as i16).collect()
}

/// I campioni per canale di un blocco da 20 ms alla frequenza data (almeno 1).
pub fn frame_samples(rate: u32) -> usize {
    (rate as usize / 50).max(1)
}

/// Come arriva un blocco rispetto al precedente dello stesso stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceCheck {
    /// Il primo o quello dopo.
    InOrder,
    /// Mancano tanti blocchi (spesso il server ha scartato i piu' vecchi perche' il lettore era lento).
    Lost(u32),
    /// Gia' visto o arrivato in ritardo: si scarta.
    Old,
}

/// Segue la `sequence` di uno stream (contatore a 32 bit che ricomincia da 0 dopo 2^32 - 1).
#[derive(Debug, Clone, Copy, Default)]
pub struct SequenceTracker {
    last: Option<u32>,
}

impl SequenceTracker {
    pub fn check(&mut self, sequence: u32) -> SequenceCheck {
        let Some(last) = self.last else {
            self.last = Some(sequence);
            return SequenceCheck::InOrder;
        };
        let step = sequence.wrapping_sub(last);
        // un passo «negativo» (meta' alta del giro) o nullo: blocco vecchio o ripetuto
        if step == 0 || step > u32::MAX / 2 {
            return SequenceCheck::Old;
        }
        self.last = Some(sequence);
        if step == 1 { SequenceCheck::InOrder } else { SequenceCheck::Lost(step - 1) }
    }
}

/// I campioni di silenzio da mettere al posto dei blocchi persi (cosi' la cascata e il decodificatore restano a tempo); zero oltre
/// [`MAX_FILL_FRAMES`].
pub fn gap_fill(lost: u32, samples_per_frame: usize) -> usize {
    if lost > MAX_FILL_FRAMES { 0 } else { lost as usize * samples_per_frame }
}

/// Lo stream concesso dal server (`result` di `startAudioStream`): valgono questi valori, non quelli chiesti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamGrant {
    pub stream_id: u32,
    pub direction: Direction,
    pub sample_rate: u32,
    pub channels: u8,
}

/// Il risultato di `startAudioStream`. `codec` diverso da `pcm16` o valori mancanti/non validi = stream inutilizzabile. Se il server
/// non ripete frequenza o canali valgono quelli chiesti.
pub fn parse_grant(result: &Value, asked: Direction, asked_rate: u32, asked_channels: u8) -> Option<StreamGrant> {
    let stream_id = u32::try_from(result.get("streamId")?.as_u64()?).ok()?;
    if let Some(codec) = result.get("codec").and_then(Value::as_str)
        && !codec.eq_ignore_ascii_case("pcm16")
    {
        return None;
    }
    let direction = match result.get("direction").and_then(Value::as_str) {
        Some(d) => Direction::parse(d)?,
        None => asked,
    };
    let sample_rate = match result.get("sampleRateHz") {
        Some(v) => u32::try_from(v.as_u64()?).ok().filter(|r| (1000..=384_000).contains(r))?,
        None => asked_rate,
    };
    let channels = match result.get("channels") {
        Some(v) => u8::try_from(v.as_u64()?).ok().filter(|c| *c == 1 || *c == 2)?,
        None => asked_channels,
    };
    (direction == asked).then_some(StreamGrant { stream_id, direction, sample_rate, channels })
}

/// I parametri di `startAudioStream`: sempre mono.
pub fn start_params(direction: Direction, rate: u32) -> Value {
    json!({ "direction": direction.as_str(), "sampleRateHz": rate, "channels": 1 })
}

/// La frequenza da chiedere dopo un rifiuto: dopo `AUDIO_RATE_UNAVAILABLE` con una frequenza diversa da 48 kHz si riprova a 48 kHz
/// (manuale); altrimenti niente.
pub fn retry_rate(code: &str, asked: u32) -> Option<u32> {
    (code == "AUDIO_RATE_UNAVAILABLE" && asked != FALLBACK_RATE).then_some(FALLBACK_RATE)
}

/// Quanti blocchi vanno mandati ora: a ritmo di 20 ms da `elapsed` (tempo dall'inizio dell'onda), con `lead` di anticipo, meno quelli
/// gia' mandati.
pub fn frames_due(elapsed: Duration, sent: u64, lead: Duration) -> u64 {
    let ahead = elapsed + lead;
    let due = (ahead.as_micros() / FRAME.as_micros()) as u64 + 1;
    due.saturating_sub(sent)
}

// ---------------------------------------------------------------- messaggi

/// La versione di Cat4OM non dichiara `audioStreaming`.
pub const NO_FEATURE: &str = "Questa versione di Cat4OM non ha l'audio in rete: aggiorna Cat4OM o usa una scheda audio";
/// `NOT_MASTER` sullo stream di trasmissione (anche dopo aver chiesto il comando).
pub const NOT_MASTER: &str = "Cat4OM: per trasmettere l'audio in rete Aurora deve avere il comando della radio, che ora e' di un altro programma";
/// `AUDIO_TRANSMIT_IN_USE`.
pub const TX_IN_USE: &str = "Cat4OM: un altro programma sta gia' mandando l'audio di trasmissione a questa radio";
/// `AUDIO_NOT_AVAILABLE` o radio senza `audioCaptureAvailable` / `audioTransmitAvailable`.
pub const UNAVAILABLE: &str = "Cat4OM: audio in rete non configurato per questa radio (scegli i dispositivi audio della radio in Cat4OM)";
/// Uno stream concesso con un codec o valori che Aurora non sa usare.
pub const BAD_GRANT: &str = "Cat4OM ha aperto l'audio in rete in un formato che Aurora non conosce";

/// `AUDIO_RATE_UNAVAILABLE` anche a 48 kHz.
pub fn rate_text(rate: u32) -> String {
    format!("Cat4OM non accetta l'audio a {rate} Hz per questa radio")
}

/// `AUDIO_DEVICE_OPEN_FAILED`, `AUDIO_DEVICE_NOT_PRESENT`.
pub fn device_text(code: &str) -> String {
    format!("Cat4OM non riesce ad aprire il dispositivo audio della radio: controlla che sia collegato ({code})")
}

/// `audioStreamStopped` dal server, con il motivo scritto da Cat4OM.
pub fn stopped_text(reason: &str) -> String {
    format!("Cat4OM ha chiuso l'audio in rete: {reason}")
}

/// Il testo di un errore di `startAudioStream`, se e' uno di quelli dell'audio (gli altri li traduce il chiamante).
pub fn error_text(code: &str, rate: u32) -> Option<String> {
    Some(match code {
        "NOT_MASTER" => NOT_MASTER.to_string(),
        "AUDIO_TRANSMIT_IN_USE" => TX_IN_USE.to_string(),
        "AUDIO_RATE_UNAVAILABLE" => rate_text(rate),
        "AUDIO_NOT_AVAILABLE" => UNAVAILABLE.to_string(),
        "AUDIO_DEVICE_OPEN_FAILED" | "AUDIO_DEVICE_NOT_PRESENT" => device_text(code),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(direction: Direction, channels: u8, samples: Vec<i16>) -> AudioFrame {
        AudioFrame { direction, channels, stream_id: 7, sample_rate: 12_000, sequence: 1, timestamp_us: 123_456, samples }
    }

    #[test]
    fn a_frame_has_the_documented_layout() {
        let f = frame(Direction::Tx, 1, vec![1, -2, 32_767]);
        let bytes = encode_frame(&f);
        assert_eq!(bytes.len(), 32 + 6);
        assert_eq!(&bytes[0..8], &[b'C', b'4', b'O', b'A', 1, 2, 0, 1], "magia, versione, tipo tx, PCM16, mono");
        assert_eq!(&bytes[8..12], &7u32.to_le_bytes(), "streamId");
        assert_eq!(&bytes[12..16], &12_000u32.to_le_bytes(), "frequenza");
        assert_eq!(&bytes[16..20], &3u32.to_le_bytes(), "campioni per canale");
        assert_eq!(&bytes[20..24], &1u32.to_le_bytes(), "sequence");
        assert_eq!(&bytes[24..32], &123_456u64.to_le_bytes(), "timestampUs");
        assert_eq!(&bytes[32..], &[1, 0, 0xfe, 0xff, 0xff, 0x7f], "PCM16 little-endian");
        assert_eq!(decode_frame(&bytes), Ok(f));
    }

    #[test]
    fn the_example_of_the_manual_20_ms_stereo_at_48_khz() {
        let f = AudioFrame { direction: Direction::Rx, channels: 2, stream_id: 1, sample_rate: 48_000, sequence: 9, timestamp_us: 0, samples: vec![0; 960 * 2] };
        let bytes = encode_frame(&f);
        assert_eq!(bytes.len() - HEADER_LEN, 3840);
        let back = decode_frame(&bytes).expect("blocco");
        assert_eq!((back.sample_count(), back.channels, back.direction), (960, 2, Direction::Rx));
        assert_eq!(frame_samples(48_000), 960);
        assert_eq!(frame_samples(12_000), 240);
    }

    #[test]
    fn unreadable_frames_are_refused() {
        let good = encode_frame(&frame(Direction::Rx, 2, vec![1, 2, 3, 4]));
        assert_eq!(decode_frame(&good[..31]), Err(FrameError::TooShort));
        let with = |at: usize, value: u8| {
            let mut b = good.clone();
            b[at] = value;
            decode_frame(&b)
        };
        assert_eq!(with(0, b'X'), Err(FrameError::BadMagic));
        assert_eq!(with(4, 2), Err(FrameError::UnknownVersion(2)));
        assert_eq!(with(5, 0), Err(FrameError::BadKind(0)), "il tipo 0 non esiste");
        assert_eq!(with(6, 1), Err(FrameError::UnknownCodec(1)));
        assert_eq!(with(7, 3), Err(FrameError::BadChannels(3)));
        assert_eq!(decode_frame(&good[..good.len() - 1]), Err(FrameError::BadLength));
        let mut longer = good.clone();
        longer.extend_from_slice(&[0, 0]);
        assert_eq!(decode_frame(&longer), Err(FrameError::BadLength));
        let mut huge = good.clone();
        huge[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(decode_frame(&huge), Err(FrameError::BadLength), "nessun calcolo che trabocca");
    }

    #[test]
    fn samples_become_mono_floats_and_back() {
        let stereo = frame(Direction::Rx, 2, vec![16_384, -16_384, 32_767, 32_767, i16::MIN, i16::MIN]);
        let mono = to_mono(&stereo);
        assert_eq!(mono.len(), 3);
        assert!(mono[0].abs() < 1e-6, "la media dei due canali");
        assert!((mono[1] - 32_767.0 / 32_768.0).abs() < 1e-6);
        assert_eq!(mono[2], -1.0);
        assert_eq!(to_pcm16(&[0.0, 0.5, 1.0, -1.0, 2.0, -3.0]), vec![0, 16_384, 32_767, -32_767, 32_767, -32_767]);
    }

    #[test]
    fn sequence_gaps_wrap_and_late_frames() {
        let mut t = SequenceTracker::default();
        assert_eq!(t.check(1), SequenceCheck::InOrder);
        assert_eq!(t.check(2), SequenceCheck::InOrder);
        assert_eq!(t.check(5), SequenceCheck::Lost(2));
        assert_eq!(t.check(4), SequenceCheck::Old, "in ritardo");
        assert_eq!(t.check(5), SequenceCheck::Old, "ripetuto");
        let mut w = SequenceTracker::default();
        assert_eq!(w.check(u32::MAX - 1), SequenceCheck::InOrder);
        assert_eq!(w.check(u32::MAX), SequenceCheck::InOrder);
        assert_eq!(w.check(0), SequenceCheck::InOrder, "il contatore ricomincia da 0 dopo 2^32 - 1");
        assert_eq!(w.check(3), SequenceCheck::Lost(2));
        assert_eq!(gap_fill(2, 240), 480);
        assert_eq!(gap_fill(MAX_FILL_FRAMES + 1, 240), 0, "un buco lungo non si riempie");
    }

    #[test]
    fn the_granted_values_win_over_the_asked_ones() {
        let r = json!({ "streamId": 3, "radioId": "run", "direction": "rx", "sampleRateHz": 48000, "channels": 2, "codec": "pcm16" });
        assert_eq!(parse_grant(&r, Direction::Rx, 12_000, 1), Some(StreamGrant { stream_id: 3, direction: Direction::Rx, sample_rate: 48_000, channels: 2 }));
        assert_eq!(parse_grant(&json!({ "streamId": 4 }), Direction::Tx, 48_000, 1).map(|g| (g.sample_rate, g.channels)), Some((48_000, 1)));
        assert_eq!(parse_grant(&json!({ "streamId": 4, "codec": "opus" }), Direction::Rx, 12_000, 1), None, "codec sconosciuto");
        assert_eq!(parse_grant(&json!({ "streamId": 4, "direction": "tx" }), Direction::Rx, 12_000, 1), None);
        assert_eq!(parse_grant(&json!({ "streamId": 4, "channels": 6 }), Direction::Rx, 12_000, 1), None);
        assert_eq!(parse_grant(&json!({ "streamId": 4, "sampleRateHz": 0 }), Direction::Rx, 12_000, 1), None);
        assert_eq!(parse_grant(&Value::Null, Direction::Rx, 12_000, 1), None);
        assert_eq!(start_params(Direction::Rx, RX_PREFERRED_RATE), json!({ "direction": "rx", "sampleRateHz": 12000, "channels": 1 }));
        assert_eq!(retry_rate("AUDIO_RATE_UNAVAILABLE", 12_000), Some(48_000), "il manuale: si riprova a 48000");
        assert_eq!(retry_rate("AUDIO_RATE_UNAVAILABLE", 48_000), None);
        assert_eq!(retry_rate("NOT_MASTER", 12_000), None);
    }

    #[test]
    fn frames_leave_every_20_ms_with_a_small_lead() {
        let lead = Duration::from_millis(40);
        assert_eq!(frames_due(Duration::ZERO, 0, lead), 3, "all'inizio il blocco di adesso e due d'anticipo");
        assert_eq!(frames_due(Duration::from_millis(10), 3, lead), 0);
        assert_eq!(frames_due(Duration::from_millis(20), 3, lead), 1);
        assert_eq!(frames_due(Duration::from_millis(200), 4, lead), 9, "dopo un ritardo si recupera");
        assert_eq!(frames_due(Duration::from_secs(1), 53, Duration::ZERO), 0, "mai di piu' del tempo passato");
    }

    #[test]
    fn audio_errors_have_clear_texts() {
        assert_eq!(error_text("NOT_MASTER", 48_000).as_deref(), Some(NOT_MASTER));
        assert_eq!(error_text("AUDIO_TRANSMIT_IN_USE", 48_000).as_deref(), Some(TX_IN_USE));
        assert_eq!(error_text("AUDIO_RATE_UNAVAILABLE", 48_000), Some("Cat4OM non accetta l'audio a 48000 Hz per questa radio".into()));
        assert_eq!(error_text("AUDIO_NOT_AVAILABLE", 12_000).as_deref(), Some(UNAVAILABLE));
        assert!(error_text("AUDIO_DEVICE_NOT_PRESENT", 12_000).is_some_and(|t| t.ends_with("(AUDIO_DEVICE_NOT_PRESENT)")));
        assert_eq!(error_text("RADIO_CLAIMED", 12_000), None, "lo traduce il collegamento con il nome della radio");
        assert_eq!(stopped_text("idle for too long"), "Cat4OM ha chiuso l'audio in rete: idle for too long");
    }
}
