//! Frame binari del flusso: `[0x01][seq u64 BE][f_min u32 BE][f_max u32 BE][bins...]`.

use aurora_core::SpectrumRow;

pub const FRAME_SPECTRUM: u8 = 0x01;

pub fn encode_spectrum(row: &SpectrumRow) -> Vec<u8> {
    let mut out = Vec::with_capacity(17 + row.bins.len());
    out.push(FRAME_SPECTRUM);
    out.extend_from_slice(&row.seq.to_be_bytes());
    out.extend_from_slice(&row.f_min_hz.to_be_bytes());
    out.extend_from_slice(&row.f_max_hz.to_be_bytes());
    out.extend_from_slice(&row.bins);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_type_seq_range_bins() {
        let frame = encode_spectrum(&SpectrumRow { seq: 258, f_min_hz: 10, f_max_hz: 3000, bins: vec![7, 8] });
        assert_eq!(frame.len(), 19);
        assert_eq!(frame[0], 1);
        assert_eq!(&frame[1..9], &258u64.to_be_bytes());
        assert_eq!(&frame[9..13], &10u32.to_be_bytes());
        assert_eq!(&frame[13..17], &3000u32.to_be_bytes());
        assert_eq!(&frame[17..], &[7, 8]);
    }
}
