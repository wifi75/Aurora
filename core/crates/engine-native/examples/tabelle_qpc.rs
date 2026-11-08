//! Calcolo delle tabelle del codice polare q-ario (127, 50) di UltraFOX con il metodo classico dei codici polari: si simula il canale
//! (128 toni FSK non coerenti con rumore gaussiano, la parola tutta a zero perche' il canale e' simmetrico), si decodifica a cancellazione
//! successiva con il «genio» (ogni decisione precedente gia' giusta) e si conta la probabilita' di sbagliare ogni posizione. Le posizioni si
//! ordinano dalla piu' affidabile alla meno affidabile: le prime 50 portano il messaggio, le altre restano congelate. La posizione 0 e'
//! quella del simbolo che non si trasmette.
//!
//! Uso: `cargo run --release --example tabelle_qpc -- <Es/N0 in dB> <parole> [seme]`

use std::{sync::Arc, thread};

use engine_native::qpc::{genie_errors, N, NP, Q};

/// Generatore pseudocasuale semplice e ripetibile (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn uniform(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    /// Due campioni gaussiani indipendenti a varianza 1 (Box-Muller).
    fn gauss2(&mut self) -> (f64, f64) {
        let r = (-2.0 * self.uniform().ln()).sqrt();
        let a = std::f64::consts::TAU * self.uniform();
        (r * a.cos(), r * a.sin())
    }
}

/// Le probabilita' dei 128 valori per ognuno dei 127 simboli, con il tono 0 trasmesso: energia di ogni tono normalizzata alla mediana,
/// poi `exp(Es/N0 / (1 + Es/N0) * energia)` come il ricevitore di Aurora.
fn channel(rng: &mut Rng, esn0: f64, out: &mut [f32]) {
    let amp = esn0.sqrt();
    let norm = esn0 / (1.0 + esn0);
    let mut e = [0f64; Q];
    for row in out.chunks_mut(Q) {
        for (v, ev) in e.iter_mut().enumerate() {
            let (a, b) = rng.gauss2();
            // rumore complesso a potenza 1 per tono, segnale di potenza Es/N0 sul tono 0
            let re = a / std::f64::consts::SQRT_2 + if v == 0 { amp } else { 0.0 };
            let im = b / std::f64::consts::SQRT_2;
            *ev = re * re + im * im;
        }
        let mut sorted = e;
        sorted.sort_by(f64::total_cmp);
        let base = sorted[Q / 2] / std::f64::consts::LN_2; // la mediana di un'esponenziale a media 1 e' ln 2
        let max = e.iter().fold(0f64, |m, &v| m.max(norm * v / base));
        let w: Vec<f64> = e.iter().map(|&v| (norm * v / base - max).exp()).collect();
        let total: f64 = w.iter().sum();
        for (o, x) in row.iter_mut().zip(&w) {
            *o = (x / total) as f32;
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let esn0_db: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6.0);
    let words: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20_000);
    let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let esn0 = 10f64.powf(esn0_db / 10.0);
    let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8) as u64;
    let shared = Arc::new(());
    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let _keep = Arc::clone(&shared);
            thread::spawn(move || {
                let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ (seed.wrapping_mul(1_000_003) + t + 1));
                let mut errors = [0f64; N];
                let mut py = vec![0f32; NP * Q];
                for _ in 0..words / threads {
                    channel(&mut rng, esn0, &mut py);
                    genie_errors(&py, &mut errors);
                }
                errors
            })
        })
        .collect();
    let mut total = [0f64; N];
    for h in handles {
        for (t, e) in total.iter_mut().zip(h.join().expect("thread")) {
            *t += e;
        }
    }
    // dalla piu' affidabile: meno errori; a parita' l'indice piu' alto (piu' «uni» nella trasformata), poi il piu' basso
    let mut order: Vec<usize> = (0..N).collect();
    order.sort_by(|&a, &b| (a == 0).cmp(&(b == 0)).then(total[a].total_cmp(&total[b])).then(a.cmp(&b)));
    println!("Es/N0 {esn0_db} dB, {} parole", words / threads * threads);
    println!("ordine: {order:?}");
    println!(
        "errori: {:?}",
        order.iter().map(|&i| format!("{:.2e}", total[i] / (words / threads * threads) as f64)).collect::<Vec<_>>()
    );
}
