//! Prove di andata e ritorno: codifica, rumore, decodifica, qualita' (PSNR) per ogni modo.

use mode_sstv::{Decoder, DecoderConfig, DecoderEvent, RATE, RgbImage, SstvMode, encoder::encode_shifted};

/// Immagine di prova "quasi fotografica": sfumature morbide, un disco, una trama e qualche bordo netto.
fn test_image(w: u32, h: u32) -> RgbImage {
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (f64::from(x) / f64::from(w), f64::from(y) / f64::from(h));
            let d = ((fx - 0.6).powi(2) + (fy - 0.45).powi(2)).sqrt();
            let mut r = 40.0 + 180.0 * fx;
            let mut g = 60.0 + 150.0 * fy + 25.0 * (fx * 9.0).sin();
            let mut b = 200.0 - 140.0 * fx * fy;
            if d < 0.2 {
                r = 230.0 - 300.0 * d;
                g = 190.0;
                b = 60.0;
            }
            if (0.1..0.3).contains(&fx) && (0.6..0.85).contains(&fy) {
                r = 20.0;
                g = 20.0;
                b = 20.0;
            }
            data.extend([r, g, b].map(|v: f64| v.clamp(0.0, 255.0) as u8));
        }
    }
    RgbImage { width: w, height: h, data }
}

/// Rumore gaussiano ripetibile.
struct Noise(u64);

impl Noise {
    fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    fn gauss(&mut self) -> f32 {
        ((-2.0 * self.uniform().ln()).sqrt() * (std::f64::consts::TAU * self.uniform()).cos()) as f32
    }
}

fn psnr(a: &RgbImage, b: &RgbImage) -> f64 {
    let rows = a.height.min(b.height) as usize;
    let n = rows * a.width as usize * 3;
    let mse: f64 = a.data[..n].iter().zip(&b.data[..n]).map(|(&x, &y)| (f64::from(x) - f64::from(y)).powi(2)).sum::<f64>() / n as f64;
    10.0 * (255.0f64.powi(2) / mse.max(1e-9)).log10()
}

/// Sposta l'orologio dell'audio di `ppm` (interpolazione lineare): simula due schede audio con frequenze diverse.
fn skew(input: &[f32], ppm: f64) -> Vec<f32> {
    let step = 1.0 + ppm * 1e-6;
    let mut out = Vec::with_capacity(input.len());
    let mut t = 0.0f64;
    while (t as usize) + 1 < input.len() {
        let i = t as usize;
        let f = (t - i as f64) as f32;
        out.push(input[i] * (1.0 - f) + input[i + 1] * f);
        t += step;
    }
    out
}

fn run(mode: SstvMode, noise_sigma: f32, ppm: f64, shift: f32) -> (RgbImage, mode_sstv::DecodedImage) {
    run_with(mode, noise_sigma, ppm, shift, |_| {})
}

/// Evanescenza (QSB): l'ampiezza sale e scende piano, con variazioni fino a `spread_hz`, e nei minimi scende sotto il rumore.
fn qsb(audio: &mut [f32], spread_hz: f32) {
    let mut noise = Noise(11);
    let parts: Vec<(f32, f32)> = (0..4).map(|i| (spread_hz * (0.25 + 0.25 * i as f32), (noise.uniform() * std::f64::consts::TAU) as f32)).collect();
    for (n, s) in audio.iter_mut().enumerate() {
        let t = n as f32 / RATE as f32;
        let wave: f32 = parts.iter().map(|(hz, ph)| (std::f32::consts::TAU * hz * t + ph).sin()).sum::<f32>() / parts.len() as f32;
        *s *= (0.55 + 0.45 * wave).clamp(0.05, 1.0);
    }
}

/// Eco (multipath): lo stesso segnale arriva anche `delay_ms` dopo, con ampiezza `amp`.
fn multipath(audio: &mut [f32], delay_ms: f32, amp: f32) {
    let d = (delay_ms * RATE as f32 / 1000.0) as usize;
    for n in (d..audio.len()).rev() {
        audio[n] += amp * audio[n - d];
    }
}

fn run_with(mode: SstvMode, noise_sigma: f32, ppm: f64, shift: f32, impair: impl Fn(&mut [f32])) -> (RgbImage, mode_sstv::DecodedImage) {
    let spec = mode.spec();
    let img = test_image(spec.width, spec.height);
    let mut audio = vec![0.0f32; RATE as usize / 2];
    audio.extend(encode_shifted(mode, &img, RATE, shift).expect("codifica"));
    audio.extend(vec![0.0f32; RATE as usize]);
    if ppm != 0.0 {
        audio = skew(&audio, ppm);
    }
    impair(&mut audio);
    let mut noise = Noise(7);
    for s in &mut audio {
        *s += noise_sigma * noise.gauss();
    }
    let mut dec = Decoder::new(DecoderConfig::default());
    let mut started = None;
    let mut result = None;
    for chunk in audio.chunks(1024) {
        for ev in dec.push(chunk) {
            match ev {
                DecoderEvent::Started { mode, .. } => started = Some(mode),
                DecoderEvent::Finished(done) => result = Some(done),
                _ => {}
            }
        }
    }
    assert_eq!(started, Some(mode), "{}: VIS non riconosciuto", spec.name);
    let done = result.unwrap_or_else(|| panic!("{}: immagine non finita", spec.name));
    (img, done)
}

/// Soglie di PSNR: i modi in YUV (Robot, PD) amplificano il rumore della crominanza e i modi veloci hanno pochi campioni per pixel.
fn threshold(mode: SstvMode, clean: bool) -> f64 {
    let yuv = matches!(mode.spec().family, mode_sstv::modes::Family::Robot36 | mode_sstv::modes::Family::Robot72 | mode_sstv::modes::Family::Pd);
    match (clean, yuv) {
        (true, false) => 32.0,
        (true, true) => 25.0,
        (false, false) => 24.0,
        (false, true) => 20.0,
    }
}

fn check_all(sigma: f32, clean: bool) {
    let mut bad = Vec::new();
    for spec in mode_sstv::MODES.iter() {
        let (img, done) = run(spec.mode, sigma, 0.0, 0.0);
        assert!(done.complete, "{}: incompleta", spec.name);
        assert_eq!(done.image.height, spec.height);
        let q = psnr(&img, &done.image);
        eprintln!("rumore {sigma}: {}: PSNR {q:.1} dB, sincronismo {:.0} %", spec.name, done.sync_quality * 100.0);
        if q <= threshold(spec.mode, clean) || done.sync_quality <= 0.9 {
            bad.push(format!("{}: PSNR {q:.1} dB, sincronismo {:.2}", spec.name, done.sync_quality));
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn every_mode_round_trips_with_noise_and_good_psnr() {
    // rumore bianco su tutta la banda: circa 22 dB di SNR nei 3 kHz del segnale (un buon segnale in HF)
    check_all(0.08, false);
}

#[test]
fn clean_signal_gives_high_quality() {
    check_all(0.0, true);
}

#[test]
fn weak_signal_still_decodes() {
    // circa 10 dB di SNR nei 3 kHz: immagine rumorosa ma intera e allineata
    for mode in [SstvMode::Martin1, SstvMode::Scottie1, SstvMode::Pd120, SstvMode::Robot36] {
        let (img, done) = run(mode, 0.3, 0.0, 0.0);
        assert!(done.complete && done.sync_quality > 0.8, "{}: sincronismo {}", mode.spec().name, done.sync_quality);
        let q = psnr(&img, &done.image);
        eprintln!("debole {}: {q:.1}", mode.spec().name);
        assert!(q > 10.0, "{}: PSNR {q:.1}", mode.spec().name);
    }
}
#[test]
fn slant_is_corrected() {
    // 300 ppm: molto piu' di due schede audio comuni
    for mode in [SstvMode::Martin1, SstvMode::Pd90] {
        let (img, done) = run(mode, 0.05, 300.0, 0.0);
        let q = psnr(&img, &done.image);
        assert!((done.clock_ppm + 300.0).abs() < 60.0, "{}: orologio stimato {} ppm", mode.spec().name, done.clock_ppm);
        assert!(q > 24.0, "{}: PSNR {q:.1} dB con inclinazione", mode.spec().name);
    }
}

#[test]
fn mistuned_signal_is_detected_and_corrected() {
    let (img, done) = run(SstvMode::Scottie2, 0.05, 0.0, 90.0);
    assert!((done.offset_hz - 90.0).abs() < 15.0, "scarto stimato {}", done.offset_hz);
    assert!(psnr(&img, &done.image) > 24.0);
}

#[test]
fn locked_mode_ignores_other_vis() {
    let spec = SstvMode::Robot36.spec();
    let img = test_image(spec.width, spec.height);
    let audio = encode_shifted(SstvMode::Robot36, &img, RATE, 0.0).expect("codifica");
    let mut dec = Decoder::new(DecoderConfig { lock: Some(SstvMode::Martin1), ..DecoderConfig::default() });
    let events: Vec<_> = audio.chunks(2048).flat_map(|c| dec.push(c)).collect();
    assert!(events.iter().all(|e| !matches!(e, DecoderEvent::Started { .. })));
}

#[test]
fn forced_start_without_vis_finds_the_line_phase() {
    let mode = SstvMode::Martin2;
    let spec = mode.spec();
    let img = test_image(spec.width, spec.height);
    let audio = encode_shifted(mode, &img, RATE, 0.0).expect("codifica");
    // si entra a meta' di una riga, dieci righe dopo l'inizio dell'immagine: niente VIS
    let skip = ((mode_sstv::modes::HEADER_SECONDS + spec.group_seconds() * 10.5) * f64::from(RATE)) as usize;
    let mut tail = audio[skip..].to_vec();
    tail.extend(vec![0.0f32; RATE as usize * 8]);
    let mut dec = Decoder::new(DecoderConfig::default());
    dec.force_start(mode);
    let done = tail.chunks(1024).flat_map(|c| dec.push(c)).find_map(|e| match e {
        DecoderEvent::Finished(d) => Some(d),
        _ => None,
    });
    let done = done.expect("immagine finita");
    // le righe ricevute sono quelle dalla 11 in poi, spostate in alto di 11
    let row = (spec.width * 3) as usize;
    let shifted = RgbImage { width: spec.width, height: spec.height - 11, data: img.data[11 * row..].to_vec() };
    let q = psnr(&shifted, &done.image);
    assert!(q > 24.0, "PSNR {q:.1}");
}

#[test]
fn deep_fading_still_gives_an_image() {
    // QSB fino a 2 Hz (come la banda alta dell'HF), con un po' di rumore: i minimi cadono sotto il rumore ma l'immagine arriva intera
    for mode in [SstvMode::Martin1, SstvMode::Scottie1, SstvMode::Pd120] {
        let (img, done) = run_with(mode, 0.05, 0.0, 0.0, |a| qsb(a, 2.0));
        let q = psnr(&img, &done.image);
        eprintln!("QSB {}: PSNR {q:.1} dB, sincronismo {:.0} %", mode.spec().name, done.sync_quality * 100.0);
        assert!(done.complete && q > 14.0, "{}: PSNR {q:.1} dB con QSB", mode.spec().name);
    }
}

#[test]
fn echo_is_tolerated() {
    // eco a 2 ms e meta' ampiezza: il sincronismo regge, i colori restano riconoscibili
    for mode in [SstvMode::Martin1, SstvMode::Robot36] {
        let (img, done) = run_with(mode, 0.03, 0.0, 0.0, |a| multipath(a, 2.0, 0.5));
        let q = psnr(&img, &done.image);
        eprintln!("eco {}: PSNR {q:.1} dB, sincronismo {:.0} %", mode.spec().name, done.sync_quality * 100.0);
        assert!(done.complete && q > 14.0, "{}: PSNR {q:.1} dB con eco", mode.spec().name);
    }
}
