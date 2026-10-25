//! Canale d'ingresso come nella scheda Audio di JTDX (`AudioInputChannel`): Mono, Sinistro, Destro.
//!
//! In JTDX (`SoundInput::start`, `Detector::writeData`) «Mono» apre il dispositivo con un solo canale (il mixer del sistema fa la somma dei due)
//! mentre Sinistro e Destro lo aprono in stereo e tengono rispettivamente il primo o il secondo campione di ogni coppia. «Entrambi» in ingresso
//! non c'e' (la lista lo toglie): se compare in un vecchio file vale Sinistro. Qui il dispositivo si apre con i suoi canali e la scelta si
//! applica ai campioni, quindi si puo' cambiare a flusso aperto.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InputChannel {
    /// Media dei canali del dispositivo.
    #[default]
    Mono,
    Left,
    Right,
}

impl InputChannel {
    /// Il nome nel `JTDX.ini` (`Mono`, `Left`, `Right`); `Both` (non esiste in ingresso) vale Sinistro come in JTDX.
    pub fn parse(text: &str) -> Option<InputChannel> {
        match text.trim().to_ascii_lowercase().as_str() {
            "mono" => Some(InputChannel::Mono),
            "left" | "both" => Some(InputChannel::Left),
            "right" => Some(InputChannel::Right),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            InputChannel::Mono => "Mono",
            InputChannel::Left => "Left",
            InputChannel::Right => "Right",
        }
    }

    pub fn code(self) -> u8 {
        match self {
            InputChannel::Mono => 0,
            InputChannel::Left => 1,
            InputChannel::Right => 2,
        }
    }

    pub fn from_code(code: u8) -> InputChannel {
        match code {
            1 => InputChannel::Left,
            2 => InputChannel::Right,
            _ => InputChannel::Mono,
        }
    }

    pub fn load(cell: &AtomicU8) -> InputChannel {
        InputChannel::from_code(cell.load(Ordering::Relaxed))
    }
}

/// Dai campioni interlacciati di un dispositivo con `channels` canali ne ricava uno solo secondo la scelta. Con un solo canale la scelta non conta.
/// Con piu' di due canali Sinistro e Destro sono i primi due.
pub fn pick_channel(interleaved: &[f32], channels: usize, channel: InputChannel, out: &mut Vec<f32>) {
    let channels = channels.max(1);
    out.clear();
    out.reserve(interleaved.len() / channels);
    if channels == 1 {
        out.extend_from_slice(interleaved);
        return;
    }
    for frame in interleaved.chunks_exact(channels) {
        out.push(match channel {
            InputChannel::Mono => frame.iter().sum::<f32>() / channels as f32,
            InputChannel::Left => frame[0],
            InputChannel::Right => frame[1],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_right_and_mono_pick_the_right_samples_of_a_stereo_stream() {
        let stereo = [0.2f32, -0.4, 0.6, 0.0, -0.2, 0.2];
        let mut out = Vec::new();
        pick_channel(&stereo, 2, InputChannel::Left, &mut out);
        assert_eq!(out, vec![0.2, 0.6, -0.2]);
        pick_channel(&stereo, 2, InputChannel::Right, &mut out);
        assert_eq!(out, vec![-0.4, 0.0, 0.2]);
        pick_channel(&stereo, 2, InputChannel::Mono, &mut out);
        let expected = [-0.1f32, 0.3, 0.0];
        assert!(out.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-6), "{out:?}");
    }

    #[test]
    fn a_mono_device_ignores_the_choice_and_extra_channels_use_the_first_two() {
        let mono = [0.1f32, 0.2, 0.3];
        let mut out = Vec::new();
        for c in [InputChannel::Mono, InputChannel::Left, InputChannel::Right] {
            pick_channel(&mono, 1, c, &mut out);
            assert_eq!(out, mono.to_vec());
        }
        let four = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        pick_channel(&four, 4, InputChannel::Right, &mut out);
        assert_eq!(out, vec![0.2, 0.6]);
        // un blocco che finisce a meta' di un fotogramma non produce un campione inventato
        pick_channel(&[0.1, 0.2, 0.3], 2, InputChannel::Left, &mut out);
        assert_eq!(out, vec![0.1]);
    }

    #[test]
    fn the_names_are_the_ones_of_the_jtdx_ini() {
        assert_eq!(InputChannel::parse("Mono"), Some(InputChannel::Mono));
        assert_eq!(InputChannel::parse("left"), Some(InputChannel::Left));
        assert_eq!(InputChannel::parse("Both"), Some(InputChannel::Left), "come in JTDX: in ingresso vale il primo canale");
        assert_eq!(InputChannel::parse("Centro"), None);
        for c in [InputChannel::Mono, InputChannel::Left, InputChannel::Right] {
            assert_eq!(InputChannel::parse(c.name()), Some(c));
            assert_eq!(InputChannel::from_code(c.code()), c);
        }
    }
}
