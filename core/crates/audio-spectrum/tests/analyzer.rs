use audio_spectrum::{AnalyzerError, SpectrumAnalyzer, F_MAX_HZ, OUTPUT_BINS};

fn sine(freq: f32, amp: f32, rate: u32, seconds: f32) -> Vec<f32> {
    let n = (rate as f32 * seconds) as usize;
    (0..n).map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin()).collect()
}

fn mix(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

fn peak_bin(row: &[u8]) -> usize {
    row.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).unwrap()
}

fn bin_of(freq_hz: f32) -> f32 {
    freq_hz / F_MAX_HZ as f32 * OUTPUT_BINS as f32
}

#[test]
fn sine_1500hz_peaks_at_bin_512_plus_minus_2() {
    for rate in [12_000u32, 44_100, 48_000] {
        let mut a = SpectrumAnalyzer::new(rate).unwrap();
        let rows = a.push(&sine(1500.0, 0.01, rate, 1.0));
        assert!(!rows.is_empty(), "{rate} Hz: nessuna riga prodotta");
        let last = rows.last().unwrap();
        assert_eq!(last.len(), OUTPUT_BINS);
        let p = peak_bin(last) as f32;
        assert!((p - bin_of(1500.0)).abs() <= 2.0, "{rate} Hz: picco al bin {p}");
    }
}

#[test]
fn silence_gives_floor_values() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let rows = a.push(&vec![0.0; 48_000]);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.iter().all(|v| *v == 0)));
}

#[test]
fn two_tones_give_two_peaks() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let signal = mix(&sine(1000.0, 0.3, 48_000, 1.0), &sine(2000.0, 0.3, 48_000, 1.0));
    let row = a.push(&signal).pop().unwrap();
    let near = |f: f32| {
        let c = bin_of(f) as usize;
        (c - 3..=c + 3).map(|i| row[i]).max().unwrap()
    };
    let background = row[bin_of(1500.0) as usize];
    assert!(near(1000.0) > background + 60 && near(2000.0) > background + 60);
}

#[test]
fn row_rate_is_about_15_per_second() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let rows = a.push(&sine(1000.0, 0.5, 48_000, 5.0));
    // dopo il riempimento della finestra (circa 0,34 s) escono 15 righe al secondo
    assert!((65..=75).contains(&rows.len()), "righe in 5 s: {}", rows.len());
}

#[test]
fn rejects_unsupported_sample_rate() {
    assert!(matches!(SpectrumAnalyzer::new(4_000), Err(AnalyzerError::SampleRateTooLow(4_000))));
    assert!(SpectrumAnalyzer::new(0).is_err());
    assert!(SpectrumAnalyzer::new(8_000).is_ok());
}

#[test]
fn samples_in_small_chunks_equal_one_big_push() {
    let signal = sine(1234.0, 0.4, 48_000, 2.0);
    let big = SpectrumAnalyzer::new(48_000).unwrap().push(&signal);
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let mut chunked = Vec::new();
    for chunk in signal.chunks(997) {
        chunked.extend(a.push(chunk));
    }
    assert_eq!(big, chunked);
}

#[test]
fn very_loud_signal_saturates_at_255_without_overflow() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let rows = a.push(&sine(1500.0, 100.0, 48_000, 1.0));
    let row = rows.last().unwrap();
    assert_eq!(row[peak_bin(row)], 255);
}

#[test]
fn very_quiet_or_invalid_samples_never_produce_garbage() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let quiet = a.push(&sine(1500.0, 1e-6, 48_000, 1.0));
    assert!(quiet.last().unwrap().iter().all(|v| *v < 10));
    let mut b = SpectrumAnalyzer::new(48_000).unwrap();
    let mut bad = sine(1500.0, 0.5, 48_000, 1.0);
    bad[100] = f32::NAN;
    bad[200] = f32::INFINITY;
    let rows = b.push(&bad);
    assert!(!rows.is_empty(), "campioni non validi non devono bloccare l'analisi");
}
