//! Prova di confronto del ricevitore PSK31/PSK63: segnali fatti con il modulatore di Aurora, rumore bianco gaussiano a vari rapporti
//! segnale/rumore (misurati in 3 kHz come nella prova RTTY, in fldigi e in MMTTY) e i casi difficili dell'analisi `docs/analisi-psk-cw.md`:
//! errore di frequenza, mezzo minuto di solo rumore prima del segnale, deriva, fading CCIR, stazione vicina piu' forte, errore d'orologio
//! della scheda audio, livello molto basso. Misura il tasso di caratteri sbagliati (distanza di Levenshtein fra il testo mandato e quello
//! ricevuto, divisa per la lunghezza del testo mandato); nel conto entrano anche i caratteri inventati dal rumore prima e dopo il testo.
//!
//! Il ricevitore lavora con le impostazioni di base del servizio (AFC acceso, squelch spento). Prove lente, si lanciano a mano:
//! - `confronto_tasso_di_errore`: la tabella completa;
//! - `rumore_e_squelch`: un minuto di solo rumore, caratteri falsi con lo squelch spento e acceso e quanto si allontana l'AFC;
//! - `studio`: un punteggio solo su pochi casi difficili, per provare in fretta una variante;
//! - `contro_fldigi` (con la feature `fldigi-oracle`): la stessa tabella con il ricevitore di fldigi 4.2.13 accanto al nostro.
//!
//! `cargo test --release -p mode-digi --test psk_confronto -- --ignored --nocapture --test-threads=1`
//! (con `PSK_DEBUG=1` stampa il testo mandato e quello ricevuto; con `PSK_SQUELCH=<soglia 0-100>` lo squelch e' acceso, anche in fldigi; con `AURORA_WAV=<cartella>` scrive i file WAV dei casi, da riascoltare
//! con fldigi).

mod common;

use common::{Rng, aggiungi_rumore, errore_orologio, fading_ccir, guadagno_complesso, levenshtein, salva_wav, sigma_per_snr, testo};
use mode_digi::psk::{self, PskDemodulator, PskMode};

const RATE: f64 = 12_000.0;
/// Di quanti punti percentuali (media di una riga) il nostro ricevitore puo' sbagliare piu' di fldigi nella prova `contro_fldigi`.
#[cfg(feature = "fldigi-oracle")]
const TOLLERANZA: f64 = 2.0;
const AMP: f64 = 0.3;
const FREQ: f32 = 1000.0;

#[derive(Clone, Copy, Debug)]
enum Prova {
    /// Solo rumore bianco.
    Awgn,
    /// Il trasmettitore e' fuori di questi Hz (l'AFC deve agganciarlo).
    Fuori(f32),
    /// Trenta secondi di solo rumore prima del segnale (l'AFC non deve andarsene). Quello che si stampa nel rumore non si conta (lo misura
    /// `rumore_e_squelch`): conta solo dall'ultimo secondo prima del segnale.
    Attesa,
    /// La portante scivola di questi Hz dall'inizio alla fine del testo.
    Deriva(f64),
    /// Fading CCIR «moderate»: due percorsi, 1 ms, 1 Hz di Doppler.
    Fading,
    /// Una stazione 6 dB piu' forte a 2,25 volte la velocita' di simbolo (70 Hz in PSK31).
    Vicina,
    /// La scheda audio sbaglia l'orologio di tante parti per milione.
    Orologio(f64),
    /// Scariche (QRN) di 1-3 ms, 15 volte la deviazione del rumore, quattro al secondo in media.
    Qrn,
    /// Segnale e rumore 40 dB piu' bassi (ingresso a basso volume).
    Livello,
}

/// Il ricevitore con le impostazioni di base del servizio; con `PSK_SQUELCH=<soglia>` anche lo squelch acceso.
fn ricevitore(mode: PskMode) -> PskDemodulator {
    let mut rx = PskDemodulator::new(mode, FREQ);
    rx.set_afc(true);
    rx.set_squelch(std::env::var("PSK_SQUELCH").ok().and_then(|v| v.parse().ok()));
    // con `PSK_HARD=1` la varicode si legge bit per bit come fldigi (per confrontare)
    if std::env::var_os("PSK_HARD").is_some() {
        rx.set_soft_decoding(false);
    }
    rx
}

fn baud(mode: PskMode) -> f64 {
    RATE / mode.symbol_samples() as f64
}

/// Il testo mandato, l'audio (12 kHz) di una prova e da quale campione si contano i caratteri ricevuti.
fn audio_di_prova(mode: PskMode, prova: Prova, snr_db: f64, seed: u64) -> (String, Vec<f32>, usize) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let sent = testo(&mut rng, 100).to_lowercase();
    let tx_freq = match prova {
        Prova::Fuori(hz) => FREQ + hz,
        _ => FREQ,
    };
    let mut sig = psk::modulate(sent.as_bytes(), tx_freq, mode, AMP as f32);
    match prova {
        Prova::Deriva(hz) => {
            let t_tot = sig.len() as f64 / RATE;
            sig = guadagno_complesso(&sig, |n| {
                let t = n as f64 / RATE;
                let ph = 2.0 * std::f64::consts::PI * hz * t * t / (2.0 * t_tot);
                (ph.cos(), ph.sin())
            });
        }
        Prova::Fading => sig = fading_ccir(&sig, 1.0, 1.0, &mut rng),
        Prova::Vicina => {
            let other = testo(&mut rng, 160).to_lowercase();
            let off = (2.25 * baud(mode)) as f32;
            let qrm = psk::modulate(other.as_bytes(), FREQ + off, mode, (2.0 * AMP) as f32);
            for (s, q) in sig.iter_mut().zip(qrm.iter()) {
                *s += *q;
            }
        }
        Prova::Orologio(ppm) => sig = errore_orologio(&sig, ppm),
        _ => {}
    }
    let pad_before = if matches!(prova, Prova::Attesa) { 30 * RATE as usize } else { RATE as usize };
    let mut audio = vec![0.0_f32; pad_before];
    audio.extend_from_slice(&sig);
    audio.extend(std::iter::repeat_n(0.0, RATE as usize));
    let sigma = sigma_per_snr(AMP, snr_db);
    aggiungi_rumore(&mut audio, sigma, &mut rng);
    if let Prova::Qrn = prova {
        // come nella prova CW: scariche di 1-3 ms, 15 volte la deviazione del rumore
        let mut n = 0usize;
        while n < audio.len() {
            n += (-(rng.uniform().ln()) * RATE / 4.0) as usize + 1;
            let len = 12 + (rng.next_u64() % 24) as usize;
            for k in n..(n + len).min(audio.len()) {
                audio[k] += (15.0 * sigma.max(0.02) * rng.gauss()) as f32;
            }
        }
    }
    if matches!(prova, Prova::Livello) {
        for s in audio.iter_mut() {
            *s *= 0.01;
        }
    }
    salva_wav(&format!("psk_{}_{prova:?}_{snr_db}dB_{seed}", mode.name()), RATE as u32, &audio);
    (sent, audio, pad_before - RATE as usize)
}

fn errori(chi: &str, sent: &str, got: &[u8]) -> usize {
    if std::env::var_os("PSK_DEBUG").is_some() {
        println!("TX        {sent:?}\nRX {chi:<6} {:?}", String::from_utf8_lossy(got));
    }
    levenshtein(sent.as_bytes(), got)
}

/// I caratteri ricevuti dal campione `from` in poi (quelli prima si decodificano ma non si contano).
fn decodifica(rx: &mut PskDemodulator, audio: &[f32], from: usize) -> Vec<u8> {
    for block in audio[..from].chunks(1024) {
        rx.process(block);
    }
    let mut got = Vec::new();
    for block in audio[from..].chunks(1024) {
        got.extend(rx.process(block));
    }
    got
}

/// Una prova con il ricevitore di Aurora: (errori, caratteri mandati).
fn una_prova(mode: PskMode, prova: Prova, snr_db: f64, seed: u64) -> (usize, usize) {
    let (sent, audio, from) = audio_di_prova(mode, prova, snr_db, seed);
    let got = decodifica(&mut ricevitore(mode), &audio, from);
    (errori("Aurora", &sent, &got), sent.len())
}

/// La stessa prova con il ricevitore di fldigi (AFC acceso, squelch come il nostro): (errori Aurora, errori fldigi, caratteri).
#[cfg(feature = "fldigi-oracle")]
fn prova_contro_fldigi(mode: PskMode, prova: Prova, snr_db: f64, seed: u64) -> (usize, usize, usize) {
    let (sent, audio, from) = audio_di_prova(mode, prova, snr_db, seed);
    let ours = errori("Aurora", &sent, &decodifica(&mut ricevitore(mode), &audio, from));
    let mut fl = mode_digi::fldigi_oracle::FldigiPsk::new(baud(mode), f64::from(FREQ), 8000.0, true, std::env::var("PSK_SQUELCH").ok().and_then(|v| v.parse().ok()));
    let low = common::a_8k(&audio);
    let from_8k = from * 2 / 3;
    for block in low[..from_8k].chunks(512) {
        fl.process(block);
    }
    let mut got = Vec::new();
    for block in low[from_8k..].chunks(512) {
        got.extend(fl.process(block));
    }
    (ours, errori("fldigi", &sent, &got), sent.len())
}

const SNRS: [f64; 7] = [5.0, 0.0, -5.0, -8.0, -10.0, -12.0, -15.0];

const CASI: [(Prova, &str); 10] = [
    (Prova::Awgn, "rumore"),
    (Prova::Fuori(4.0), "fuori +4 Hz"),
    (Prova::Fuori(-6.0), "fuori -6 Hz"),
    (Prova::Attesa, "30 s di rumore"),
    (Prova::Deriva(4.0), "deriva 4 Hz"),
    (Prova::Fading, "fading CCIR"),
    (Prova::Vicina, "vicina +6 dB"),
    (Prova::Orologio(200.0), "orologio 200ppm"),
    (Prova::Livello, "livello -40 dB"),
    (Prova::Qrn, "QRN"),
];

#[test]
#[ignore]
fn confronto_tasso_di_errore() {
    const RUNS: u64 = 2;
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        let mut totali = vec![(0usize, 0usize); SNRS.len()];
        for (prova, nome) in CASI {
            let mut riga = format!("{} {nome:<16}|", mode.name());
            for (k, &snr) in SNRS.iter().enumerate() {
                let (mut e, mut n) = (0, 0);
                for r in 0..RUNS {
                    let (ee, nn) = una_prova(mode, prova, snr, 500 + r * 31 + k as u64 * 7);
                    e += ee;
                    n += nn;
                }
                totali[k].0 += e;
                totali[k].1 += n;
                riga.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
            }
            println!("{riga}");
        }
        let mut media = format!("{} {:<16}|", mode.name(), "media");
        for (e, n) in totali {
            media.push_str(&format!(" {:>6.1}", 100.0 * e as f64 / n as f64));
        }
        println!("{}", "-".repeat(media.len()));
        println!("{media}");
    }
    println!("(colonne: SNR in 3 kHz {SNRS:?} dB; valori: caratteri sbagliati in %)");
}

/// La stessa tabella con fldigi 4.2.13 come arbitro: ogni casella e' «Aurora/fldigi» (caratteri sbagliati in %). Solo con la feature
/// `fldigi-oracle`: `cargo test --release -p mode-digi --features fldigi-oracle --test psk_confronto contro_fldigi -- --ignored --nocapture`.
#[cfg(feature = "fldigi-oracle")]
#[test]
#[ignore]
fn contro_fldigi() {
    const RUNS: u64 = 2;
    let mut peggiori = Vec::new();
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        let (mut tot_a, mut tot_f) = (0.0, 0.0);
        for (prova, nome) in CASI {
            let mut riga = format!("{} {nome:<16}|", mode.name());
            let (mut media_a, mut media_f) = (0.0, 0.0);
            for (k, &snr) in SNRS.iter().enumerate() {
                let (mut ea, mut ef, mut n) = (0, 0, 0);
                for r in 0..RUNS {
                    let (a, f, len) = prova_contro_fldigi(mode, prova, snr, 500 + r * 31 + k as u64 * 7);
                    ea += a;
                    ef += f;
                    n += len;
                }
                let (pa, pf) = (100.0 * ea as f64 / n as f64, 100.0 * ef as f64 / n as f64);
                media_a += pa / SNRS.len() as f64;
                media_f += pf / SNRS.len() as f64;
                riga.push_str(&format!(" {pa:5.1}/{pf:5.1}"));
            }
            tot_a += media_a / CASI.len() as f64;
            tot_f += media_f / CASI.len() as f64;
            println!("{riga} | media {media_a:5.1}/{media_f:5.1}");
            if media_a > media_f + TOLLERANZA {
                peggiori.push(riga);
            }
        }
        println!("{} media di tutte le righe: Aurora {tot_a:.1}% · fldigi {tot_f:.1}%", mode.name());
    }
    println!("(colonne: SNR in 3 kHz {SNRS:?} dB; caselle: Aurora/fldigi, caratteri sbagliati in %)");
    assert!(peggiori.is_empty(), "righe dove Aurora sbaglia piu' di fldigi (oltre {TOLLERANZA} punti di media):
{}", peggiori.join("
"));
}

/// Un minuto di solo rumore (forte e debole): quanti caratteri si stampano con lo squelch spento e acceso, e di quanto si sposta l'AFC.
#[test]
#[ignore]
fn rumore_e_squelch() {
    for mode in [PskMode::Psk31, PskMode::Psk63] {
        for sigma in [0.3, 0.003] {
            for squelch in [false, true] {
                let mut rng = Rng(4242);
                let mut audio = vec![0.0_f32; 60 * RATE as usize];
                aggiungi_rumore(&mut audio, sigma, &mut rng);
                let mut rx = ricevitore(mode);
                if squelch {
                    rx.set_squelch(Some(mode_digi::psk::SQUELCH_DEFAULT));
                }
                let mut n = 0;
                let mut lontano = 0.0_f32;
                for block in audio.chunks(1024) {
                    n += rx.process(block).len();
                    lontano = lontano.max((rx.frequency() - FREQ).abs());
                }
                println!("{} rumore sigma {sigma:<5} squelch {:<6}: {n:>4} caratteri falsi, AFC fino a {lontano:.1} Hz", mode.name(), if squelch { "acceso" } else { "spento" });
            }
        }
    }
}

/// Un punteggio solo (errori medi su pochi casi difficili), per provare in fretta le varianti del ricevitore.
#[test]
#[ignore]
fn studio() {
    let casi: [(PskMode, Prova, f64); 8] = [
        (PskMode::Psk31, Prova::Awgn, -8.0),
        (PskMode::Psk31, Prova::Awgn, -10.0),
        (PskMode::Psk31, Prova::Awgn, -12.0),
        (PskMode::Psk31, Prova::Fading, -5.0),
        (PskMode::Psk31, Prova::Vicina, -5.0),
        (PskMode::Psk31, Prova::Attesa, -8.0),
        (PskMode::Psk31, Prova::Orologio(200.0), -8.0),
        (PskMode::Psk63, Prova::Awgn, -8.0),
    ];
    let mut riga = String::new();
    let mut tot = 0.0;
    for (mode, prova, snr) in casi {
        let (mut e, mut n) = (0, 0);
        for r in 0..4 {
            let (ee, nn) = una_prova(mode, prova, snr, 77 + r);
            e += ee;
            n += nn;
        }
        let cer = 100.0 * e as f64 / n as f64;
        tot += cer;
        riga.push_str(&format!(" {cer:5.1}"));
    }
    println!("STUDIO media {:5.1} |{riga}", tot / casi.len() as f64);
}

/// Tutti i segnali (browser): 10 e 30 stazioni PSK31 fra 320 e 2800 Hz a -8 dB in 3 kHz ognuna, un minuto. Stampa canali aperti, nominativi
/// verificati e la CPU in percento di un core (tempo di calcolo diviso per il tempo dell'audio). Con `PSK_DEBUG=1` anche i canali.
#[test]
#[ignore]
fn tutti_i_segnali() {
    use mode_digi::skim::BandDecoder;
    use mode_digi::skim::psk::PskBandDecoder;
    for stations in [10_usize, 30] {
        let mut rng = Rng(171 + stations as u64);
        let n = 60 * RATE as usize;
        let mut band = vec![0.0_f32; n];
        let step = 2_450.0 / stations as f32;
        let calls: Vec<String> = (0..stations).map(|i| format!("IU{}A{}", i % 10, ["BC", "BD", "BE", "BF"][i / 10])).collect();
        for (i, call) in calls.iter().enumerate() {
            let hz = 330.0 + step * i as f32 + 3.0 * (i % 3) as f32;
            let text = format!("cq cq cq de {call} {call} {call} pse k   cq cq cq de {call} {call} {call} pse k");
            let wave = psk::modulate(text.as_bytes(), hz, PskMode::Psk31, AMP as f32);
            for (s, w) in band.iter_mut().skip(6_000 + 2_000 * i).zip(wave) {
                *s += w;
            }
        }
        aggiungi_rumore(&mut band, sigma_per_snr(AMP, -8.0), &mut rng);
        let mut b = PskBandDecoder::new(PskMode::Psk31, 150.0, 4_000.0, 40);
        // le stazioni finiscono prima del minuto e i loro canali si chiudono: contano gli spot (nominativi verificati) e i canali aperti
        // insieme al massimo
        let (mut spotted, mut most, mut secs) = (Vec::new(), 0, 0.0);
        for (k, block) in band.chunks(1_200).enumerate() {
            let t0 = std::time::Instant::now();
            b.process(block);
            secs += t0.elapsed().as_secs_f64();
            spotted.extend(b.take_spots().into_iter().map(|s| s.call));
            if k % 25 == 0 {
                let chans = b.channels();
                most = most.max(chans.len());
                if std::env::var_os("PSK_DEBUG").is_some() && k == 250 {
                    for ch in &chans {
                        println!("{:7.1} Hz {:5.1} dB {:?} | {}", ch.freq_hz, ch.snr_db, ch.calls.iter().map(|c| (c.call.clone(), c.verified())).collect::<Vec<_>>(), ch.text);
                    }
                }
            }
        }
        let found = calls.iter().filter(|c| spotted.contains(c)).count();
        let wrong = spotted.iter().filter(|s| !calls.contains(s)).count();
        println!("{stations} stazioni PSK31: canali insieme {most} · nominativi verificati {found}/{stations}, sbagliati {wrong} · CPU {:.1}% di un core", 100.0 * secs / 60.0);
    }
}
