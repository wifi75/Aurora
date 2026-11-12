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

/// Tutti gli avvenimenti del decodificatore su un audio, a blocchi come dalla scheda.
fn events_of(audio: &[f32], cfg: DecoderConfig) -> Vec<DecoderEvent> {
    let mut dec = Decoder::new(cfg);
    audio.chunks(1024).flat_map(|c| dec.push(c)).collect()
}

fn add_noise(audio: &mut [f32], sigma: f32, seed: u64) {
    let mut noise = Noise(seed);
    for s in audio {
        *s += sigma * noise.gauss();
    }
}

fn starts_of(events: &[DecoderEvent]) -> Vec<SstvMode> {
    events.iter().filter_map(|e| if let DecoderEvent::Started { mode, .. } = e { Some(*mode) } else { None }).collect()
}

fn last_image(events: &[DecoderEvent]) -> Option<mode_sstv::DecodedImage> {
    events.iter().rev().find_map(|e| if let DecoderEvent::Finished(d) = e { Some(d.clone()) } else { None })
}

/// La PSNR migliore spostando le righe ricevute (l'immagine presa a meta' comincia da una riga che non si sa).
fn best_shifted_psnr(img: &RgbImage, got: &RgbImage) -> f64 {
    let row = (img.width * 3) as usize;
    (0..40u32)
        .filter(|&k| k < img.height)
        .map(|k| psnr(&RgbImage { width: img.width, height: img.height - k, data: img.data[k as usize * row..].to_vec() }, got))
        .fold(0.0, f64::max)
}

#[test]
fn fsk_id_round_trips_after_the_image_with_noise() {
    for (mode, sigma) in [(SstvMode::Robot36, 0.08), (SstvMode::Martin2, 0.3), (SstvMode::Scottie2, 0.2)] {
        let spec = mode.spec();
        let img = test_image(spec.width, spec.height);
        let mut audio = vec![0.0f32; RATE as usize / 2];
        audio.extend(mode_sstv::encoder::encode_with_id(mode, &img, RATE, 0.0, Some("iu3abc/p")).expect("codifica"));
        audio.extend(vec![0.0f32; RATE as usize * 2]);
        add_noise(&mut audio, sigma, 5);
        let events = events_of(&audio, DecoderConfig::default());
        let finished = events.iter().position(|e| matches!(e, DecoderEvent::Finished(_)));
        let id = events.iter().position(|e| *e == DecoderEvent::FskId("IU3ABC/P".into()));
        assert!(finished.is_some() && id.is_some(), "{}: immagine {finished:?}, FSK ID {id:?}", spec.name);
        assert!(id > finished, "{}: l'FSK ID viene dopo l'immagine", spec.name);
        assert_eq!(events.iter().filter(|e| matches!(e, DecoderEvent::FskId(_))).count(), 1);
    }
}

#[test]
fn fsk_id_is_read_with_a_mistuned_signal() {
    let mode = SstvMode::Robot36;
    let spec = mode.spec();
    let img = test_image(spec.width, spec.height);
    let mut audio = mode_sstv::encoder::encode_with_id(mode, &img, RATE, 70.0, Some("IU3ABC")).expect("codifica");
    audio.extend(vec![0.0f32; RATE as usize]);
    add_noise(&mut audio, 0.05, 9);
    let events = events_of(&audio, DecoderConfig::default());
    assert!(events.contains(&DecoderEvent::FskId("IU3ABC".into())), "{:?}", starts_of(&events));
}

#[test]
fn noise_alone_gives_no_image_and_no_fsk_id() {
    let mut audio = vec![0.0f32; RATE as usize * 90];
    add_noise(&mut audio, 0.3, 21);
    let events = events_of(&audio, DecoderConfig::default());
    assert!(events.is_empty(), "{:?}", starts_of(&events));
}

#[test]
fn every_mode_is_recognised_without_vis_from_the_line_period() {
    let mut bad = Vec::new();
    for spec in mode_sstv::MODES.iter() {
        let img = test_image(spec.width, spec.height);
        let audio = encode_shifted(spec.mode, &img, RATE, 0.0).expect("codifica");
        // si accende l'ascolto a immagine cominciata: niente VIS, si entra a meta' della sesta riga
        let skip = ((mode_sstv::modes::HEADER_SECONDS + spec.group_seconds() * 5.5) * f64::from(RATE)) as usize;
        let mut tail = audio[skip..].to_vec();
        // dopo l'immagine 8 s di solo rumore: la ricezione presa a meta' si chiude da sola (6 s senza sincronismo)
        tail.extend(vec![0.0f32; RATE as usize * 8]);
        add_noise(&mut tail, 0.08, 13);
        let events = events_of(&tail, DecoderConfig::default());
        let by_sync: Vec<SstvMode> = events.iter().filter_map(|e| if let DecoderEvent::Started { mode, by_sync: true, .. } = e { Some(*mode) } else { None }).collect();
        match (by_sync.as_slice(), last_image(&events)) {
            ([m], Some(d)) if *m == spec.mode && d.mode == spec.mode => {
                let q = best_shifted_psnr(&img, &d.image);
                eprintln!("senza VIS {}: PSNR {q:.1} dB, sincronismo {:.0} %, {} righe", spec.name, d.sync_quality * 100.0, d.image.height);
                if q < 18.0 {
                    bad.push(format!("{}: PSNR {q:.1}", spec.name));
                }
            }
            (s, d) => bad.push(format!("{}: partenze {s:?} ({:?}), finita {:?}", spec.name, starts_of(&events), d.map(|d| d.mode))),
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn recognition_without_vis_follows_the_locked_mode_and_can_be_turned_off() {
    let spec = SstvMode::Martin1.spec();
    let img = test_image(spec.width, spec.height);
    let audio = encode_shifted(spec.mode, &img, RATE, 0.0).expect("codifica");
    let skip = ((mode_sstv::modes::HEADER_SECONDS + spec.group_seconds() * 3.5) * f64::from(RATE)) as usize;
    let tail = &audio[skip..skip + RATE as usize * 20];
    let events = events_of(tail, DecoderConfig { lock: Some(SstvMode::Scottie1), ..DecoderConfig::default() });
    assert!(starts_of(&events).is_empty(), "{:?}", starts_of(&events));
    let events = events_of(tail, DecoderConfig { auto_sync: false, ..DecoderConfig::default() });
    assert!(starts_of(&events).is_empty(), "{:?}", starts_of(&events));
}

/// Il VIS di un modo (leader, bit di partenza, codice, arresto) da solo, come lo produce il codificatore.
fn vis_burst(mode: SstvMode) -> Vec<f32> {
    let spec = mode.spec();
    let img = test_image(spec.width, spec.height);
    let audio = encode_shifted(mode, &img, RATE, 0.0).expect("codifica");
    // dalla fine dei toni di calibrazione alla fine del VIS
    audio[(0.8 * f64::from(RATE)) as usize..(mode_sstv::modes::HEADER_SECONDS * f64::from(RATE)) as usize].to_vec()
}

#[test]
fn a_false_vis_in_a_disturbed_image_does_not_change_the_mode() {
    let mode = SstvMode::Martin1;
    let spec = mode.spec();
    let img = test_image(spec.width, spec.height);
    let mut audio = encode_shifted(mode, &img, RATE, 0.0).expect("codifica");
    // a meta' immagine due secondi di disturbo (le righe perdono il sincronismo) e dentro un VIS di Robot 36
    let at = ((mode_sstv::modes::HEADER_SECONDS + spec.group_seconds() * 100.0) * f64::from(RATE)) as usize;
    let burst_len = RATE as usize * 2;
    let mut noise = Noise(3);
    for s in &mut audio[at..at + burst_len] {
        *s = 0.6 * noise.gauss();
    }
    let vis = vis_burst(SstvMode::Robot36);
    let vis_at = at + burst_len - vis.len();
    audio[vis_at..vis_at + vis.len()].copy_from_slice(&vis);
    audio.extend(vec![0.0f32; RATE as usize]);
    add_noise(&mut audio, 0.05, 17);
    let events = events_of(&audio, DecoderConfig::default());
    assert_eq!(starts_of(&events), [mode], "{:?}", starts_of(&events));
    let done = last_image(&events).expect("immagine");
    assert!(done.complete && done.mode == mode);
}

#[test]
fn a_new_transmission_after_an_interrupted_image_starts_from_its_vis() {
    let m1 = SstvMode::Martin1.spec();
    let img1 = test_image(m1.width, m1.height);
    let first = encode_shifted(m1.mode, &img1, RATE, 0.0).expect("codifica");
    // la prima trasmissione si ferma al 40 %, mezzo secondo di silenzio, poi un Robot 36 intero
    let cut = ((mode_sstv::modes::HEADER_SECONDS + m1.image_seconds() * 0.4) * f64::from(RATE)) as usize;
    let mut audio = first[..cut].to_vec();
    audio.extend(vec![0.0f32; RATE as usize / 2]);
    let r36 = SstvMode::Robot36.spec();
    let img2 = test_image(r36.width, r36.height);
    audio.extend(encode_shifted(r36.mode, &img2, RATE, 0.0).expect("codifica"));
    audio.extend(vec![0.0f32; RATE as usize]);
    add_noise(&mut audio, 0.05, 19);
    let events = events_of(&audio, DecoderConfig::default());
    assert_eq!(starts_of(&events), [SstvMode::Martin1, SstvMode::Robot36], "{:?}", starts_of(&events));
    let done = last_image(&events).expect("immagine");
    assert_eq!(done.mode, SstvMode::Robot36);
    assert!(done.complete && psnr(&img2, &done.image) > 20.0);
}

#[test]
fn recognition_without_vis_estimates_the_tuning_offset() {
    let spec = SstvMode::Scottie1.spec();
    let img = test_image(spec.width, spec.height);
    let audio = encode_shifted(spec.mode, &img, RATE, 80.0).expect("codifica");
    let skip = ((mode_sstv::modes::HEADER_SECONDS + spec.group_seconds() * 20.5) * f64::from(RATE)) as usize;
    let mut tail = audio[skip..].to_vec();
    tail.extend(vec![0.0f32; RATE as usize * 8]);
    add_noise(&mut tail, 0.05, 23);
    let events = events_of(&tail, DecoderConfig::default());
    let offset = events.iter().find_map(|e| if let DecoderEvent::Started { offset_hz, by_sync: true, .. } = e { Some(*offset_hz) } else { None });
    assert!(offset.is_some_and(|o| (o - 80.0).abs() < 25.0), "scarto {offset:?}");
    let done = last_image(&events).expect("immagine");
    assert!(best_shifted_psnr(&img, &done.image) > 22.0);
}
