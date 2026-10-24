//! Probe usa-e-getta: apre un ingresso audio e stampa ogni secondo la frequenza di picco.
//! Serve a verificare che Aurora possa ascoltare la scheda audio mentre JTDX la usa (H2).
//! Uso: audio_tone [parte-del-nome-dispositivo]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, FftPlanner};

const BLOCK: usize = 8192;

/// Frequenza di picco (Hz) di un blocco di campioni, o `None` se il segnale e' silenzioso o troppo corto.
fn peak_hz(samples: &[f32], sample_rate: u32) -> Option<f32> {
    let n = samples.len();
    if n < 256 || samples.iter().all(|s| s.abs() < 1e-4) {
        return None;
    }
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let hann = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos();
            Complex::new(s * hann, 0.0)
        })
        .collect();
    FftPlanner::<f32>::new().plan_fft_forward(n).process(&mut buf);
    let (bin, _) = buf[1..n / 2]
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.norm().total_cmp(&b.1.norm()))?;
    Some((bin + 1) as f32 * sample_rate as f32 / n as f32)
}

fn level_db(samples: &[f32]) -> f32 {
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
    20.0 * rms.max(1e-9).log10()
}

fn dev_name(d: &cpal::Device) -> String {
    d.description().map(|x| x.name().to_string()).unwrap_or_else(|_| "(senza nome)".into())
}

fn list_devices(host: &cpal::Host) {
    eprintln!("dispositivi di ingresso disponibili:");
    match host.input_devices() {
        Ok(devices) => {
            for d in devices {
                eprintln!("  - {}", dev_name(&d));
            }
        }
        Err(e) => eprintln!("  impossibile elencarli: {e}"),
    }
}

fn main() -> std::process::ExitCode {
    let wanted = std::env::args().nth(1);
    let host = cpal::default_host();
    let device = match &wanted {
        None => host.default_input_device(),
        Some(part) => host.input_devices().ok().and_then(|mut it| {
            it.find(|d| dev_name(d).to_lowercase().contains(&part.to_lowercase()))
        }),
    };
    let Some(device) = device else {
        eprintln!("ingresso audio non trovato");
        list_devices(&host);
        return std::process::ExitCode::from(1);
    };
    let name = dev_name(&device);
    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{name}: configurazione non disponibile: {e}");
            list_devices(&host);
            return std::process::ExitCode::from(1);
        }
    };
    let rate = config.sample_rate();
    let channels = config.channels() as usize;
    let shared: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&shared);
    let on_error = |e: cpal::Error| eprintln!("errore del flusso audio: {e}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config.into(),
            move |data: &[f32], _| push(&sink, data.iter().step_by(channels).copied()),
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            config.into(),
            move |data: &[i16], _| push(&sink, data.iter().step_by(channels).map(|&s| s as f32 / 32768.0)),
            on_error,
            None,
        ),
        other => {
            eprintln!("{name}: formato campioni non gestito: {other:?}");
            return std::process::ExitCode::from(1);
        }
    };
    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{name}: apertura fallita (occupato da un'altra applicazione?): {e}");
            return std::process::ExitCode::from(1);
        }
    };
    if let Err(e) = stream.play() {
        eprintln!("{name}: avvio fallito: {e}");
        return std::process::ExitCode::from(1);
    }
    println!("ascolto su \"{name}\" a {rate} Hz, {channels} canali. Ctrl+C per fermare.");
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let block: Vec<f32> = match shared.lock() {
            Ok(g) => g.iter().rev().take(BLOCK).rev().copied().collect(),
            Err(_) => return std::process::ExitCode::from(1),
        };
        match peak_hz(&block, rate) {
            Some(hz) => println!("picco {hz:7.1} Hz   livello {:6.1} dB", level_db(&block)),
            None => println!("silenzio o campioni insufficienti ({} campioni)", block.len()),
        }
    }
}

fn push(sink: &Arc<Mutex<Vec<f32>>>, samples: impl Iterator<Item = f32>) {
    if let Ok(mut g) = sink.lock() {
        g.extend(samples);
        let len = g.len();
        if len > BLOCK * 2 {
            g.drain(..len - BLOCK * 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::peak_hz;

    fn sine(freq: f32, rate: u32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn peak_of_1500hz_sine_is_1500hz() {
        let rate = 12_000;
        let samples = sine(1500.0, rate, 8192);
        let peak = peak_hz(&samples, rate).expect("picco atteso");
        assert!((peak - 1500.0).abs() <= rate as f32 / 8192.0, "picco {peak}");
    }

    #[test]
    fn peak_of_silence_is_none() {
        assert_eq!(peak_hz(&vec![0.0; 8192], 12_000), None);
    }

    #[test]
    fn peak_of_too_short_block_is_none() {
        assert_eq!(peak_hz(&[0.1; 10], 12_000), None);
    }
}
