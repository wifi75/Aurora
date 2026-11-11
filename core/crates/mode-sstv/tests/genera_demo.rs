//! Strumento (non e' una prova): fa le immagini «ricevute» di esempio che finiscono nell'installatore.
//! Ogni foto di `desktop/src-tauri/demo-sstv/foto` passa dal vero codificatore SSTV, da un po' di rumore e dal vero decoder, come se fosse arrivata via radio.
//! Si lancia a mano: `cargo test -p mode-sstv --release --test genera_demo -- --ignored --nocapture`

use mode_sstv::{Decoder, DecoderConfig, DecoderEvent, RATE, SstvMode, encoder::encode_shifted, image_io::{decode_image, encode_png, fit_cover}};
use std::path::Path;

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

#[test]
#[ignore = "strumento: scrive i file di esempio"]
fn genera_ricevute_di_esempio() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/src-tauri/demo-sstv");
    // foto, modo, nome del modo nel file, rumore (sigma), nominativo del corrispondente di esempio, minuti fa
    let plan: [(&str, SstvMode, &str, f32, &str, u64); 5] = [
        ("terra-apollo17.jpg", SstvMode::Martin1, "martin1", 0.06, "IU3WJQ", 55),
        ("astronauta-apollo11.jpg", SstvMode::Scottie1, "scottie1", 0.10, "IU3WJQ", 40),
        ("galassie-hubble.jpg", SstvMode::Pd90, "pd90", 0.14, "IU3WJQ", 28),
        ("pilastri-creazione.jpg", SstvMode::Robot36, "robot36", 0.05, "IU3WJQ", 15),
        ("saturno.jpg", SstvMode::Martin2, "martin2", 0.18, "IU3WJQ", 6),
    ];
    let mut index = String::from("[\n");
    for (i, (foto, mode, name, sigma, call, ago)) in plan.iter().enumerate() {
        let spec = mode.spec();
        let src = decode_image(&std::fs::read(base.join("foto").join(foto)).expect("foto")).expect("decodifica");
        let img = fit_cover(&src, spec.width, spec.height);
        let mut audio = vec![0.0f32; RATE as usize / 2];
        audio.extend(encode_shifted(*mode, &img, RATE, 30.0).expect("codifica"));
        audio.extend(vec![0.0f32; RATE as usize]);
        let mut noise = Noise(100 + i as u64);
        for s in &mut audio {
            *s += sigma * noise.gauss();
        }
        let mut dec = Decoder::new(DecoderConfig::default());
        let done = audio
            .chunks(1024)
            .flat_map(|c| dec.push(c))
            .find_map(|e| match e {
                DecoderEvent::Finished(d) => Some(d),
                _ => None,
            })
            .expect("immagine finita");
        let file = format!("{:02}-{name}.png", i + 1);
        std::fs::write(base.join("ricevute").join(&file), encode_png(&done.image).expect("png")).expect("scrittura");
        index.push_str(&format!("  {{\"file\": \"{file}\", \"mode\": \"{}\", \"call\": \"{call}\", \"minutes_ago\": {ago}, \"sync_quality\": {:.2}}}{}\n", mode.id(), done.sync_quality, if i + 1 < plan.len() { "," } else { "" }));
        eprintln!("{file}: sincronismo {:.0} %, completa {}", done.sync_quality * 100.0, done.complete);
    }
    index.push_str("]\n");
    std::fs::write(base.join("ricevute").join("demo.json"), index).expect("indice");
}
