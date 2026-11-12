//! Con quale formato si apre una scheda audio: canali, frequenza di campionamento e tipo dei campioni, in ordine di preferenza.
//!
//! Serve all'ingresso (`audio-spectrum`) e all'uscita (`tx-native`). Non dipende dalla libreria audio: il tipo dei campioni e' un parametro,
//! cosi' la scelta si prova senza una scheda.
//!
//! Il motivo: l'elenco dei formati «supportati» non dice come lavora la scheda. Su Linux con PulseAudio o PipeWire ogni dispositivo dichiara da
//! 1 a 31 canali e qualunque frequenza (il server converte tutto): il massimo dell'elenco non e' il formato vero, e aprire con quello da' un
//! flusso inutile o rifiutato. Ad Aurora bastano uno o due canali a 48 o 44,1 kHz.

/// Un formato con cui provare ad aprire il flusso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamChoice<F> {
    pub channels: u16,
    pub rate: u32,
    pub format: F,
}

/// Un intervallo di formati che il dispositivo dichiara: questi canali e questo tipo di campioni, a ogni frequenza tra le due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamRange<F> {
    pub channels: u16,
    pub min_rate: u32,
    pub max_rate: u32,
    pub format: F,
}

/// Le alternative sensate quando il formato predefinito non va: stereo a 48 kHz, stereo a 44,1 kHz, poi mono.
const FALLBACKS: [(u16, u32); 4] = [(2, 48_000), (2, 44_100), (1, 48_000), (1, 44_100)];

/// I formati da provare, nell'ordine. `default` e' quello che il sistema indica per il dispositivo, `ranges` quelli che dichiara di accettare,
/// `formats` i tipi di campioni che Aurora sa leggere o scrivere (il primo e' il preferito).
///
/// - Il predefinito viene per primo se ha uno o due canali e un tipo di campioni gestito: e' il formato con cui la scheda lavora davvero (su
///   Windows l'unico che la modalita' condivisa accetta).
/// - Poi stereo 48 kHz, stereo 44,1 kHz, mono 48 kHz, mono 44,1 kHz, se il dispositivo li dichiara.
/// - Un predefinito con piu' di due canali resta per ultimo: meglio di niente (una scheda che accetta solo quello).
pub fn stream_candidates<F: Copy + PartialEq>(default: Option<StreamChoice<F>>, ranges: &[StreamRange<F>], formats: &[F]) -> Vec<StreamChoice<F>> {
    let mut out: Vec<StreamChoice<F>> = Vec::new();
    let handled = |f: &F| formats.contains(f);
    let default = default.filter(|d| d.channels >= 1 && handled(&d.format));
    let simple_default = default.filter(|d| d.channels <= 2);
    if let Some(d) = simple_default {
        out.push(d);
    }
    // prima il tipo di campioni del predefinito (quello della scheda), poi gli altri nell'ordine di preferenza
    let mut order: Vec<F> = Vec::with_capacity(formats.len() + 1);
    if let Some(d) = default {
        order.push(d.format);
    }
    for f in formats {
        if !order.contains(f) {
            order.push(*f);
        }
    }
    for (channels, rate) in FALLBACKS {
        let format = order.iter().find(|f| ranges.iter().any(|r| r.channels == channels && r.format == **f && r.min_rate <= rate && rate <= r.max_rate));
        if let Some(format) = format {
            let choice = StreamChoice { channels, rate, format: *format };
            if !out.contains(&choice) {
                out.push(choice);
            }
        }
    }
    if let Some(d) = default
        && !out.contains(&d)
    {
        out.push(d);
    }
    out
}

/// I canali con cui il dispositivo si apre davvero (quelli del primo formato da provare), per l'elenco delle impostazioni; 1 se non si sa.
pub fn channels_in_use<F: Copy + PartialEq>(default: Option<StreamChoice<F>>, ranges: &[StreamRange<F>], formats: &[F]) -> u16 {
    stream_candidates(default, ranges, formats).first().map_or(1, |c| c.channels)
}

/// La grandezza del buffer da chiedere, in campioni per canale, nell'ordine in cui provarle: `None` e' «quella che decide il sistema».
/// `short_first` chiede prima un buffer di `millis` millisecondi (stretto tra `limits`, se il dispositivo li dichiara).
pub fn buffer_candidates(rate: u32, millis: u32, limits: Option<(u32, u32)>, short_first: bool) -> Vec<Option<u32>> {
    let mut out = Vec::with_capacity(2);
    if short_first {
        let frames = (u64::from(rate) * u64::from(millis) / 1000).clamp(1, u64::from(u32::MAX)) as u32;
        out.push(Some(match limits {
            Some((min, max)) if min <= max => frames.clamp(min, max),
            _ => frames,
        }));
    }
    out.push(None);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Fmt {
        F32,
        I16,
        U8,
    }

    const HANDLED: [Fmt; 2] = [Fmt::F32, Fmt::I16];

    fn choice(channels: u16, rate: u32, format: Fmt) -> StreamChoice<Fmt> {
        StreamChoice { channels, rate, format }
    }

    /// Come PulseAudio e PipeWire: da 1 a 31 canali, ogni frequenza, ogni tipo di campioni.
    fn pulse_ranges() -> Vec<StreamRange<Fmt>> {
        let mut ranges = Vec::new();
        for format in [Fmt::U8, Fmt::I16, Fmt::F32] {
            for channels in 1..32 {
                ranges.push(StreamRange { channels, min_rate: 1, max_rate: 384_000, format });
            }
        }
        ranges
    }

    #[test]
    fn a_stereo_default_comes_first_and_the_sensible_alternatives_follow() {
        let got = stream_candidates(Some(choice(2, 48_000, Fmt::I16)), &pulse_ranges(), &HANDLED);
        assert_eq!(got, vec![choice(2, 48_000, Fmt::I16), choice(2, 44_100, Fmt::I16), choice(1, 48_000, Fmt::I16), choice(1, 44_100, Fmt::I16)]);
    }

    #[test]
    fn the_31_channels_of_the_list_are_never_used() {
        let ranges = pulse_ranges();
        for default in [Some(choice(2, 44_100, Fmt::I16)), Some(choice(1, 48_000, Fmt::F32)), None] {
            let got = stream_candidates(default, &ranges, &HANDLED);
            assert!(!got.is_empty());
            assert!(got.iter().all(|c| c.channels <= 2 && [44_100, 48_000].contains(&c.rate)), "{got:?}");
        }
        assert_eq!(channels_in_use(Some(choice(2, 44_100, Fmt::I16)), &ranges, &HANDLED), 2, "l'etichetta dice stereo, non 31 canali");
        assert_eq!(channels_in_use(Some(choice(1, 48_000, Fmt::I16)), &ranges, &HANDLED), 1);
    }

    #[test]
    fn a_default_with_many_channels_is_tried_only_after_stereo_and_mono() {
        let got = stream_candidates(Some(choice(8, 96_000, Fmt::F32)), &pulse_ranges(), &HANDLED);
        assert_eq!(got.first(), Some(&choice(2, 48_000, Fmt::F32)));
        assert_eq!(got.last(), Some(&choice(8, 96_000, Fmt::F32)));
        assert_eq!(got.len(), 5);
        // una scheda che dichiara solo quel formato (modalita' condivisa di Windows): si apre come prima
        let only = [StreamRange { channels: 8, min_rate: 96_000, max_rate: 96_000, format: Fmt::F32 }];
        assert_eq!(stream_candidates(Some(choice(8, 96_000, Fmt::F32)), &only, &HANDLED), vec![choice(8, 96_000, Fmt::F32)]);
        assert_eq!(channels_in_use(Some(choice(8, 96_000, Fmt::F32)), &only, &HANDLED), 8);
    }

    #[test]
    fn a_windows_card_keeps_its_own_format_first_whatever_the_rate() {
        let ranges = [StreamRange { channels: 2, min_rate: 192_000, max_rate: 192_000, format: Fmt::F32 }];
        assert_eq!(stream_candidates(Some(choice(2, 192_000, Fmt::F32)), &ranges, &HANDLED), vec![choice(2, 192_000, Fmt::F32)]);
    }

    #[test]
    fn an_unhandled_sample_type_falls_back_to_one_aurora_can_use() {
        // il predefinito e' a 8 bit senza segno, che Aurora non gestisce: si apre in virgola mobile (il preferito) se il dispositivo lo accetta
        let got = stream_candidates(Some(choice(2, 48_000, Fmt::U8)), &pulse_ranges(), &HANDLED);
        assert_eq!(got.first(), Some(&choice(2, 48_000, Fmt::F32)));
        assert!(got.iter().all(|c| c.format != Fmt::U8));
        // e se accetta solo quello non c'e' niente da provare
        let only = [StreamRange { channels: 2, min_rate: 8_000, max_rate: 48_000, format: Fmt::U8 }];
        assert!(stream_candidates(Some(choice(2, 48_000, Fmt::U8)), &only, &HANDLED).is_empty());
        assert_eq!(channels_in_use(Some(choice(2, 48_000, Fmt::U8)), &only, &HANDLED), 1);
    }

    #[test]
    fn without_a_default_the_declared_formats_are_enough() {
        let ranges = [StreamRange { channels: 1, min_rate: 8_000, max_rate: 44_100, format: Fmt::I16 }, StreamRange { channels: 2, min_rate: 44_100, max_rate: 44_100, format: Fmt::I16 }];
        assert_eq!(stream_candidates(None, &ranges, &HANDLED), vec![choice(2, 44_100, Fmt::I16), choice(1, 44_100, Fmt::I16)]);
        assert!(stream_candidates::<Fmt>(None, &[], &HANDLED).is_empty());
    }

    #[test]
    fn the_short_buffer_is_asked_first_and_the_system_one_stays_as_a_fallback() {
        assert_eq!(buffer_candidates(48_000, 40, None, true), vec![Some(1920), None]);
        assert_eq!(buffer_candidates(44_100, 40, Some((64, 1024)), true), vec![Some(1024), None]);
        assert_eq!(buffer_candidates(48_000, 40, Some((4096, 8192)), true), vec![Some(4096), None]);
        assert_eq!(buffer_candidates(48_000, 40, None, false), vec![None]);
    }
}
