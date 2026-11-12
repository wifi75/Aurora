use audio_spectrum::{AnalyzerError, SpectrumAnalyzer, F_MAX_HZ, OUTPUT_BINS};

/// Gli stessi valori del client (`app/src/render/jtdxRow.ts`) per tornare dal byte ai dB di ampiezza.
const DB_MIN: f32 = -40.0;
const DB_STEP: f32 = 80.0 / 255.0;

fn sine(freq: f32, amp: f32, rate: u32, seconds: f32) -> Vec<f32> {
    let n = (rate as f32 * seconds) as usize;
    (0..n).map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin()).collect()
}

fn mix(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

fn peak_bin(row: &[u8]) -> usize {
    row.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).unwrap_or(0)
}

/// Canale di una frequenza: 0,732 Hz per canale (12000/16384), come JTDX.
fn bin_of(freq_hz: f32) -> f32 {
    freq_hz * 16384.0 / 12000.0
}

fn db_of(byte: u8) -> f32 {
    DB_MIN + f32::from(byte) * DB_STEP
}

/// Larghezza in canali, a `drop_db` sotto il picco, attorno al picco.
fn width_bins(row: &[u8], drop_db: f32) -> usize {
    let p = peak_bin(row);
    let limit = db_of(row[p]) - drop_db;
    let mut lo = p;
    while lo > 0 && db_of(row[lo - 1]) >= limit {
        lo -= 1;
    }
    let mut hi = p;
    while hi + 1 < row.len() && db_of(row[hi + 1]) >= limit {
        hi += 1;
    }
    hi - lo + 1
}

#[test]
fn rows_have_the_channels_of_jtdx_0_to_5000_hz() {
    assert_eq!(F_MAX_HZ, 5000);
    assert_eq!(OUTPUT_BINS, 6827);
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let rows = a.push(&sine(1500.0, 0.1, 48_000, 0.5));
    assert!(rows.iter().all(|r| r.len() == OUTPUT_BINS));
}

#[test]
fn sine_1500hz_peaks_at_its_channel_whatever_the_sample_rate() {
    for rate in [12_000u32, 44_100, 48_000] {
        let mut a = SpectrumAnalyzer::new(rate).unwrap();
        let rows = a.push(&sine(1500.0, 0.1, rate, 2.0));
        assert!(!rows.is_empty(), "{rate} Hz: nessuna riga prodotta");
        let last = rows.last().unwrap();
        let p = peak_bin(last) as f32;
        assert!((p - bin_of(1500.0)).abs() <= 1.5, "{rate} Hz: picco al canale {p}, atteso {}", bin_of(1500.0));
    }
}

#[test]
fn a_steady_tone_is_narrow_like_in_jtdx() {
    // finestra gaussiana di JTDX: a 20 dB sotto il picco un tono fermo occupa pochi canali (qualche Hz), non decine
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let rows = a.push(&sine(1500.0, 0.1, 48_000, 3.0));
    let row = rows.last().unwrap();
    let w = width_bins(row, 20.0);
    assert!(w as f32 * 12000.0 / 16384.0 <= 8.0, "tono largo {w} canali a -20 dB");
}

#[test]
fn an_ft8_symbol_is_narrower_than_the_old_long_window_smear() {
    // un tono che cambia ogni 160 ms (come i simboli FT8): con la finestra di 0,144 s ogni riga ne mostra soprattutto uno,
    // con una finestra lunga mezzo secondo se ne vedrebbero tre o quattro insieme
    let rate = 48_000u32;
    let mut signal = Vec::new();
    for symbol in 0..40u32 {
        let tone = 1500.0 + 6.25 * ((symbol * 5) % 8) as f32;
        signal.extend(sine(tone, 0.1, rate, 0.16));
    }
    let mut a = SpectrumAnalyzer::new(rate).unwrap();
    let rows = a.push(&signal);
    let row = &rows[rows.len() - 8];
    let lit = row.iter().filter(|b| db_of(**b) > db_of(row[peak_bin(row)]) - 10.0).count();
    // 8 toni da 6,25 Hz coprirebbero 50 Hz = 68 canali (a -10 dB dal picco); una riga ne deve illuminare molti meno
    assert!(lit < 40, "canali accesi in una riga: {lit}");
}

#[test]
fn silence_gives_the_floor() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let rows = a.push(&vec![0.0; 48_000]);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.iter().all(|v| *v == 0)));
}

#[test]
fn two_tones_give_two_peaks() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let signal = mix(&sine(1000.0, 0.3, 48_000, 2.0), &sine(2000.0, 0.3, 48_000, 2.0));
    let row = a.push(&signal).pop().unwrap();
    let near = |f: f32| {
        let c = bin_of(f) as usize;
        (c - 3..=c + 3).map(|i| row[i]).max().unwrap()
    };
    let background = row[bin_of(1500.0) as usize];
    assert!(near(1000.0) > background + 100 && near(2000.0) > background + 100);
}

#[test]
fn row_rate_is_15_per_second() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let rows = a.push(&sine(1000.0, 0.5, 48_000, 5.0));
    assert_eq!(rows.len(), 75, "righe in 5 s: {}", rows.len());
}

#[test]
fn rejects_unsupported_sample_rate() {
    assert!(matches!(SpectrumAnalyzer::new(4_000), Err(AnalyzerError::SampleRateTooLow(4_000))));
    assert!(SpectrumAnalyzer::new(0).is_err());
    assert!(SpectrumAnalyzer::new(8_000).is_ok());
}

#[test]
fn low_sample_rate_leaves_the_channels_above_nyquist_empty() {
    let mut a = SpectrumAnalyzer::new(8_000).unwrap();
    let rows = a.push(&sine(1000.0, 0.3, 8_000, 1.0));
    let row = rows.last().unwrap();
    assert_eq!(row.len(), OUTPUT_BINS);
    assert!(row[OUTPUT_BINS - 1] == 0);
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
    let rows = a.push(&sine(1500.0, 100.0, 48_000, 2.0));
    let row = rows.last().unwrap();
    assert_eq!(row[peak_bin(row)], 255);
}

#[test]
fn level_is_the_amplitude_in_db_and_follows_the_signal() {
    // 20 dB in piu' di ampiezza = 20 dB in piu' sul byte
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let low = a.push(&sine(1500.0, 0.001, 48_000, 2.0));
    let mut b = SpectrumAnalyzer::new(48_000).unwrap();
    let high = b.push(&sine(1500.0, 0.01, 48_000, 2.0));
    let (l, h) = (low.last().unwrap(), high.last().unwrap());
    let diff = db_of(h[peak_bin(h)]) - db_of(l[peak_bin(l)]);
    assert!((diff - 20.0).abs() < 1.0, "differenza {diff} dB");
}

#[test]
fn quiet_or_invalid_samples_never_produce_garbage() {
    let mut a = SpectrumAnalyzer::new(48_000).unwrap();
    let quiet = a.push(&sine(1500.0, 1e-9, 48_000, 1.0));
    assert!(quiet.last().unwrap().iter().all(|v| *v < 10));
    let mut b = SpectrumAnalyzer::new(48_000).unwrap();
    let mut bad = sine(1500.0, 0.5, 48_000, 1.0);
    bad[100] = f32::NAN;
    bad[200] = f32::INFINITY;
    let rows = b.push(&bad);
    assert!(!rows.is_empty(), "campioni non validi non devono bloccare l'analisi");
}

/// CW manipolato duro (fronti istantanei, 25 WPM, a 800 Hz) nel rumore: i click fanno righe orizzontali lontano dal tono. Con
/// l'anti-click l'energia in piu' a 1,5-2,5 kHz deve quasi sparire e il tono restare com'era; accanto, per confronto, la sostituzione
/// delle celle con la mediana delle tre righe vicine nel tempo (che con la finestra di 0,14 s, larga cinque righe, non basta).
#[test]
fn anticlick_removes_key_clicks_and_keeps_the_signal() {
    let rate = 12_000u32;
    let unit = (1.2 / 25.0 * rate as f32) as usize;
    let mut x = 99u64;
    let mut noise = || {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 23) as f32 - 1.0) * 0.003
    };
    // 2 s di solo rumore (riferimento), poi 6 s di punti e linee
    let mut audio: Vec<f32> = (0..2 * rate as usize).map(|_| noise()).collect();
    let pattern = [1usize, 1, 3, 1, 1, 1, 3, 3, 1, 3];
    let mut on = Vec::new();
    while on.len() < 6 * rate as usize {
        for &p in &pattern {
            on.extend(std::iter::repeat_n(true, p * unit));
            on.extend(std::iter::repeat_n(false, unit));
        }
        on.extend(std::iter::repeat_n(false, 2 * unit));
    }
    on.truncate(6 * rate as usize);
    for &k in &on {
        let t = audio.len() as f32 / rate as f32;
        audio.push(noise() + if k { 0.5 * (2.0 * std::f32::consts::PI * 800.0 * t).sin() } else { 0.0 });
    }
    let rows_with = |anticlick: bool| {
        let mut a = SpectrumAnalyzer::new(rate).unwrap();
        a.set_anticlick(Some(anticlick));
        let rows = a.push(&audio);
        let clicks = a.recent_clicks().count();
        (rows, clicks)
    };
    let (off, clicks) = rows_with(false);
    let (on_rows, _) = rows_with(true);
    // la mediana di tre righe vicine nel tempo, cella per cella, sulle righe senza anti-click
    let median: Vec<Vec<u8>> = (0..off.len())
        .map(|r| {
            if r == 0 || r + 1 >= off.len() {
                return off[r].clone();
            }
            (0..off[r].len())
                .map(|k| {
                    let mut v = [off[r - 1][k], off[r][k], off[r + 1][k]];
                    v.sort_unstable();
                    v[1]
                })
                .collect()
        })
        .collect();
    let far = (bin_of(1500.0) as usize)..(bin_of(2500.0) as usize);
    let tone = bin_of(800.0).round() as usize;
    let mean_db = |rows: &[Vec<u8>], from: usize, to: usize, bins: std::ops::Range<usize>| {
        let (mut sum, mut n) = (0.0, 0.0);
        for row in &rows[from..to] {
            for k in bins.clone() {
                sum += db_of(row[k]);
                n += 1.0;
            }
        }
        sum / n
    };
    let rows_per_s = off.len() / 8;
    let (quiet, keyed) = ((rows_per_s / 2, 3 * rows_per_s / 2), (3 * rows_per_s, 8 * rows_per_s - 2));
    let excess = |rows: &[Vec<u8>]| mean_db(rows, keyed.0, keyed.1, far.clone()) - mean_db(rows, quiet.0, quiet.1, far.clone());
    let carrier = |rows: &[Vec<u8>]| mean_db(rows, keyed.0, keyed.1, tone..tone + 1);
    let (e_off, e_on, e_med) = (excess(&off), excess(&on_rows), excess(&median));
    println!("ultimi click trovati {clicks}; energia in piu' a 1,5-2,5 kHz: senza {e_off:.1} dB, anti-click {e_on:.1} dB, mediana di tre righe {e_med:.1} dB");
    println!("tono: senza {:.1} dB, anti-click {:.1} dB, mediana {:.1} dB", carrier(&off), carrier(&on_rows), carrier(&median));
    assert!(clicks > 20, "click trovati {clicks}");
    assert!(e_off > 6.0, "la prova non fa click visibili ({e_off:.1} dB)");
    assert!(e_on < 3.0, "con l'anti-click restano {e_on:.1} dB");
    assert!((carrier(&on_rows) - carrier(&off)).abs() < 1.5, "il tono cambia");
}

#[test]
fn anticlick_without_clicks_changes_nothing() {
    let audio = sine(800.0, 0.5, 12_000, 3.0);
    let mut a = SpectrumAnalyzer::new(12_000).unwrap();
    a.set_anticlick(Some(false));
    let mut b = SpectrumAnalyzer::new(12_000).unwrap();
    b.set_anticlick(Some(true));
    let (ra, rb) = (a.push(&audio), b.push(&audio));
    assert_eq!(b.recent_clicks().count(), 0);
    assert_eq!(ra, rb);
}
