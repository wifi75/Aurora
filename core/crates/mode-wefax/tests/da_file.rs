//! Decodifica di una registrazione WAV con il servizio: lo stesso segnale sintetico dei test di ricezione (APT start, fasatura, immagine di
//! prova, APT stop) scritto in un WAV a 12000 e a 44100 Hz deve dare le stesse cartine nella cronologia, e un file che non e' un WAV deve dare
//! un errore pulito.

use std::{
    f64::consts::TAU,
    path::PathBuf,
    time::{Duration, Instant},
};

use mode_wefax::{GrayImage, WefaxError, WefaxService, image_io::decode_png};

const W: usize = 1809;
const H: usize = 60;

/// Immagine di prova: onde lente in orizzontale e in verticale, cosi' le righe vicine si assomigliano.
fn immagine() -> Vec<u8> {
    let mut v = Vec::with_capacity(W * H);
    for y in 0..H {
        for x in 0..W {
            let a = 0.5 + 0.5 * (x as f64 * 0.021 + y as f64 * 0.05).sin();
            let b = 0.6 + 0.4 * (y as f64 * 0.031).cos();
            let c = 0.5 + 0.5 * (x as f64 * 0.0043).sin();
            v.push((45.0 + 190.0 * (0.5 * a * b + 0.5 * c)).round().clamp(0.0, 255.0) as u8);
        }
    }
    v
}

/// Audio del segnale alla frequenza `rate`: un secondo di silenzio, APT start (5 s, 300 Hz), fasatura (30 righe), immagine a 120 righe al
/// minuto, APT stop (5 s, 450 Hz), un po' di silenzio.
fn genera(rate: u32, img: &[u8]) -> Vec<f32> {
    let r = f64::from(rate);
    let riga = r * 60.0 / 120.0;
    let mut out: Vec<f32> = Vec::new();
    let mut phase = 0.0f64;
    let mut tono = |out: &mut Vec<f32>, v: f64| {
        phase += TAU * (1500.0 + 800.0 * v / 255.0) / r;
        if phase > TAU {
            phase -= TAU;
        }
        out.push(0.5 * phase.sin() as f32);
    };
    out.extend(std::iter::repeat_n(0.0, rate as usize));
    for i in 0..(5.0 * r) as usize {
        tono(&mut out, if ((2.0 * 300.0 * i as f64 / r) as u64) % 2 == 0 { 255.0 } else { 0.0 });
    }
    for i in 0..(30.0 * riga) as usize {
        let frac = (i as f64 / riga).fract();
        tono(&mut out, if !(0.025..=0.975).contains(&frac) { 255.0 } else { 0.0 });
    }
    for i in 0..(H as f64 * riga) as usize {
        let t = i as f64 / riga;
        let (row, col) = (t as usize, (t.fract() * W as f64) as usize);
        tono(&mut out, f64::from(img[row.min(H - 1) * W + col.min(W - 1)]));
    }
    for i in 0..(5.0 * r) as usize {
        tono(&mut out, if ((2.0 * 450.0 * i as f64 / r) as u64) % 2 == 0 { 255.0 } else { 0.0 });
    }
    for _ in 0..rate as usize {
        tono(&mut out, 0.0);
    }
    out.extend(std::iter::repeat_n(0.0, 2 * rate as usize));
    out
}

/// Un WAV: `float` a 32 bit oppure PCM a 16 bit, mono o con lo stesso segnale sui due canali.
fn wav(rate: u32, samples: &[f32], float: bool, channels: u16) -> Vec<u8> {
    let bits: u16 = if float { 32 } else { 16 };
    let block = channels * bits / 8;
    let mut data = Vec::with_capacity(samples.len() * usize::from(block));
    for &s in samples {
        for _ in 0..channels {
            if float {
                data.extend_from_slice(&s.to_le_bytes());
            } else {
                data.extend_from_slice(&((s * 32767.0).round() as i16).to_le_bytes());
            }
        }
    }
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&(if float { 3u16 } else { 1u16 }).to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
    out.extend_from_slice(&block.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aurora-wefax-file-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("cartella di prova");
    dir
}

fn servizio(name: &str) -> (WefaxService, PathBuf) {
    let dir = temp(name);
    let svc = WefaxService::open(&dir);
    let mut s = svc.settings();
    s.lpm_576 = 120;
    // la decodifica da file salva sempre nella cronologia, anche con il salvataggio automatico spento
    s.auto_save = false;
    svc.apply_settings(s).expect("impostazioni");
    (svc, dir)
}

fn attendi_fine(svc: &WefaxService) {
    let start = Instant::now();
    while svc.snapshot().rx.decoding_file {
        assert!(start.elapsed() < Duration::from_secs(240), "decodifica troppo lunga");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Le cartine nella cronologia, dalla piu' vecchia.
fn cartine(svc: &WefaxService) -> Vec<GrayImage> {
    let mut e = svc.images();
    e.sort_by_key(|x| x.time_ms);
    e.iter().map(|x| decode_png(&svc.image_png(&x.id).expect("png")).expect("lettura")).collect()
}

fn pearson(a: &[u8], b: &[u8]) -> f64 {
    let n = a.len().min(b.len()) as f64;
    let ma = a.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
    let mb = b.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
    let (mut num, mut da, mut db) = (0.0, 0.0, 0.0);
    for (&x, &y) in a.iter().zip(b) {
        let (x, y) = (f64::from(x) - ma, f64::from(y) - mb);
        num += x * y;
        da += x * x;
        db += y * y;
    }
    num / (da.sqrt() * db.sqrt())
}

/// Correlazione con l'originale cercando le righe di fasatura rimaste in testa e uno scarto di colonne.
fn confronta(rx: &GrayImage, orig: &[u8]) -> f64 {
    let n = H - 4;
    let mut best = -1.0f64;
    for off in 0..=12usize {
        if rx.height as usize <= off + n {
            break;
        }
        for dc in -6i32..=6 {
            let (mut a, mut b) = (Vec::new(), Vec::new());
            for r in 0..n {
                let row = &rx.data[(off + r) * W..(off + r + 1) * W];
                for x in 0..W {
                    a.push(row[(x as i32 + dc).rem_euclid(W as i32) as usize]);
                    b.push(orig[r * W + x]);
                }
            }
            best = best.max(pearson(&a, &b));
        }
    }
    best
}

#[test]
fn lo_stesso_segnale_a_12000_e_a_44100_hz_da_le_stesse_cartine() {
    let img = immagine();
    // 12000 Hz: PCM a 16 bit mono; 44100 Hz: float a 32 bit con due canali uguali
    let (svc_a, dir_a) = servizio("a");
    let (svc_b, dir_b) = servizio("b");
    svc_a.decode_file(wav(12_000, &genera(12_000, &img), false, 1)).expect("12000 Hz");
    // mentre la prima decodifica gira, una seconda richiesta sullo stesso servizio e' rifiutata
    assert_eq!(svc_a.decode_file(wav(12_000, &[0.0; 100], false, 1)).expect_err("occupato"), WefaxError::Busy);
    svc_b.decode_file(wav(44_100, &genera(44_100, &img), true, 2)).expect("44100 Hz");
    attendi_fine(&svc_a);
    attendi_fine(&svc_b);
    let (a, b) = (cartine(&svc_a), cartine(&svc_b));
    assert_eq!((a.len(), b.len()), (1, 1), "una cartina per file");
    assert_eq!((a[0].width, a[0].height), (b[0].width, b[0].height), "stessa misura");
    assert_eq!(a[0].width as usize, W);
    let (ca, cb) = (confronta(&a[0], &img), confronta(&b[0], &img));
    let tra = pearson(&a[0].data, &b[0].data);
    eprintln!("da file: 12000 Hz {ca:.4}, 44100 Hz {cb:.4} con l'originale, tra le due {tra:.4}, {} righe", a[0].height);
    assert!(ca > 0.97, "12000 Hz: correlazione con l'originale {ca}");
    assert!(cb > 0.97, "44100 Hz: correlazione con l'originale {cb}");
    assert!(tra > 0.97, "le due decodifiche differiscono: {tra}");
    // a fine file lo stato e' pulito e l'errore di orologio ricordato della scheda non si e' toccato
    let rx = svc_a.snapshot().rx;
    assert!(!rx.decoding_file && !rx.receiving);
    assert_eq!(svc_a.settings().learned_ppm, 0.0);
    assert_eq!(svc_a.settings().learned_weight, 0.0);
    for d in [dir_a, dir_b] {
        let _ = std::fs::remove_dir_all(d);
    }
}

/// Un servizio che riceve dal vivo (salvataggio automatico acceso) con la registrazione accesa o spenta.
fn dal_vivo(name: &str, record: bool) -> (WefaxService, PathBuf) {
    let dir = temp(name);
    let svc = WefaxService::open(&dir);
    let mut s = svc.settings();
    s.lpm_576 = 120;
    s.auto_save = true;
    s.record_audio = record;
    svc.apply_settings(s).expect("impostazioni");
    svc.start_rx();
    (svc, dir)
}

/// Il segnale alla scheda come in ricezione, a blocchi di un quarto di secondo (stanno tutti nella coda: nessuno si perde).
fn trasmetti(svc: &WefaxService, samples: &[f32]) {
    for block in samples.chunks(3000) {
        svc.feed(12_000, block);
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn attendi(cosa: &str, mut fatto: impl FnMut() -> bool) {
    let start = Instant::now();
    while !fatto() {
        assert!(start.elapsed() < Duration::from_secs(240), "troppo lungo: {cosa}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// I file della cartella delle registrazioni (vuota se non c'e').
fn registrazioni(svc: &WefaxService) -> Vec<String> {
    let dir = PathBuf::from(svc.snapshot().audio_folder);
    std::fs::read_dir(dir).map(|l| l.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default()
}

#[test]
fn la_ricezione_dal_vivo_si_registra_in_un_wav_che_ridecodificato_da_la_stessa_cartina() {
    let img = immagine();
    let segnale = genera(12_000, &img);
    let (svc, dir) = dal_vivo("registra", true);
    trasmetti(&svc, &segnale);
    attendi("cartina con l'audio", || svc.images().first().is_some_and(|e| e.has_audio));
    assert_eq!(svc.snapshot().rx.dropped_blocks, 0);
    let entry = svc.images().remove(0);
    assert_eq!(entry.audio.as_deref(), Some(mode_wefax::recorder::audio_name_for(&entry.id).as_str()));
    let bytes = svc.image_audio(&entry.id).expect("wav della cartina");
    let w = mode_wefax::wav::parse(&bytes).expect("wav valido");
    assert_eq!((w.rate, w.channels), (12_000, 1));
    assert_eq!(bytes.len(), 44 + w.frames() * 2, "intestazione con le misure vere");
    let secondi = w.frames() as f64 / 12_000.0;
    eprintln!("registrazione: {secondi:.1} s su {:.1} s di segnale", segnale.len() as f64 / 12_000.0);
    // almeno fasatura e immagine (45 s) piu' parte del tono d'inizio, al massimo tutto il segnale
    assert!(secondi > 47.0 && secondi <= segnale.len() as f64 / 12_000.0, "durata {secondi}");
    assert_eq!(registrazioni(&svc), vec![mode_wefax::recorder::audio_name_for(&entry.id)], "niente file provvisori");
    // il WAV registrato, decodificato di nuovo, da' la stessa cartina
    let (svc2, dir2) = servizio("registra-di-nuovo");
    svc2.decode_file(bytes).expect("decodifica della registrazione");
    attendi_fine(&svc2);
    let (a, b) = (cartine(&svc), cartine(&svc2));
    assert_eq!((a.len(), b.len()), (1, 1));
    assert_eq!((a[0].width, a[0].height), (b[0].width, b[0].height), "stessa misura");
    let (ca, cb, tra) = (confronta(&a[0], &img), confronta(&b[0], &img), pearson(&a[0].data, &b[0].data));
    eprintln!("dal vivo {ca:.4}, dalla registrazione {cb:.4}, tra le due {tra:.4}");
    assert!(ca > 0.97 && cb > 0.97 && tra > 0.97, "{ca} {cb} {tra}");
    // cancellata la cartina, se ne va anche la registrazione
    svc.delete_image(&entry.id).expect("cancellata");
    assert!(registrazioni(&svc).is_empty());
    for d in [dir, dir2] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn la_cartina_scartata_cancella_la_registrazione_e_quella_spenta_non_scrive_niente() {
    let img = immagine();
    let segnale = genera(12_000, &img);
    // tono d'inizio e fasatura, poi l'immagine si butta a mano: il file provvisorio sparisce
    let (svc, dir) = dal_vivo("scartata", true);
    let fasatura = (1.0 + 5.0 + 10.0) as usize * 12_000;
    trasmetti(&svc, &segnale[..fasatura]);
    attendi("registrazione partita", || svc.snapshot().rx.state != "apt" && registrazioni(&svc).iter().any(|n| n.ends_with(".wav.part")));
    svc.cancel_rx();
    attendi("registrazione buttata", || registrazioni(&svc).is_empty());
    std::thread::sleep(Duration::from_millis(300));
    assert!(registrazioni(&svc).is_empty() && svc.images().is_empty());
    // registrazione spenta: la cartina si salva, nessuna cartella dell'audio
    let (off, dir_off) = dal_vivo("spenta", false);
    trasmetti(&off, &segnale);
    attendi("cartina", || !off.images().is_empty());
    std::thread::sleep(Duration::from_millis(300));
    assert!(!off.images()[0].has_audio);
    assert!(!PathBuf::from(off.snapshot().audio_folder).exists(), "con la registrazione spenta non si scrive niente");
    for d in [dir, dir_off] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn senza_poter_scrivere_l_audio_la_ricezione_continua() {
    let img = immagine();
    let segnale = genera(12_000, &img);
    let (svc, dir) = dal_vivo("disco", true);
    // un file al posto della cartella `audio`: la registrazione non si apre, come con il disco pieno o una cartella protetta
    let audio = PathBuf::from(svc.snapshot().audio_folder);
    std::fs::create_dir_all(audio.parent().expect("cartella delle cartine")).expect("cartella");
    std::fs::write(&audio, b"non una cartella").expect("file");
    trasmetti(&svc, &segnale);
    attendi("prima cartina", || svc.images().len() == 1);
    trasmetti(&svc, &segnale);
    attendi("seconda cartina", || svc.images().len() == 2);
    assert!(svc.images().iter().all(|e| !e.has_audio));
    assert_eq!(svc.snapshot().rx.dropped_blocks, 0);
    for c in cartine(&svc) {
        assert!(confronta(&c, &img) > 0.97);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn le_impostazioni_vecchie_hanno_la_registrazione_accesa_per_sette_giorni() {
    let s: mode_wefax::WefaxSettings = serde_json::from_str(r#"{"ioc":576,"lpm_576":120}"#).expect("impostazioni vecchie");
    assert!(s.record_audio);
    assert_eq!(s.audio_keep_days, 7);
}

#[test]
fn un_file_che_non_e_un_wav_da_un_errore_pulito_e_non_blocca_il_servizio() {
    let (svc, dir) = servizio("errore");
    for brutto in [b"questo non e' un file audio".to_vec(), Vec::new(), wav(5_000, &[0.0; 64], false, 1), wav(192_000, &[0.0; 64], false, 1)] {
        let e = svc.decode_file(brutto).expect_err("doveva rifiutare");
        assert_eq!(e.code(), "bad_audio", "{e}");
        assert!(!svc.snapshot().rx.decoding_file);
    }
    // dopo gli errori un file buono parte (e si ferma a comando)
    svc.decode_file(wav(12_000, &vec![0.0f32; 12_000 * 600], false, 1)).expect("file buono");
    assert!(svc.snapshot().rx.decoding_file);
    svc.stop_file();
    attendi_fine(&svc);
    assert!(svc.images().is_empty());
    assert_eq!(svc.snapshot().rx.file_percent, 0.0);
    let _ = std::fs::remove_dir_all(dir);
}
