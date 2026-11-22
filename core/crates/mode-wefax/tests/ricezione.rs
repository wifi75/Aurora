//! Prove di ricezione con un segnale WEFAX sintetico generato qui: APT start, fasatura, righe di un'immagine di prova modulate in FM sulla
//! sottoportante, APT stop. Si controlla che l'immagine ricostruita abbia la larghezza giusta e sia molto correlata all'originale, anche con
//! un piccolo scarto di frequenza, rumore e un orologio che sbaglia di qualche centinaio di ppm.

use std::f64::consts::TAU;

use mode_wefax::{Decoder, DecoderConfig, DecoderEvent, Done, Filter, RATE, Source, width_of};

struct Segnale {
    /// Scarto di sintonia in Hz sulla sottoportante.
    offset_hz: f64,
    /// Ampiezza del rumore (deviazione standard) rispetto a un segnale di ampiezza 0,5.
    rumore: f32,
    /// Errore dell'orologio di chi trasmette: periodo di riga piu' lungo di tanti ppm.
    ppm: f64,
    lpm: u32,
    righe_fasatura: usize,
    apt_start_hz: f64,
    /// Tono di partenza e di stop presenti (false: si trasmette solo l'immagine).
    apt: bool,
}

impl Segnale {
    fn standard(ioc: u32, lpm: u32) -> Segnale {
        Segnale { offset_hz: 0.0, rumore: 0.0, ppm: 0.0, lpm, righe_fasatura: 30, apt_start_hz: if ioc == 288 { 675.0 } else { 300.0 }, apt: true }
    }
}

/// Immagine di prova: onde lente in orizzontale e in verticale (valori tra 45 e 235), cosi' le righe vicine si assomigliano e non ci sono
/// zone piatte.
fn immagine(w: usize, h: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let a = 0.5 + 0.5 * (x as f64 * 0.021 + y as f64 * 0.05).sin();
            let b = 0.6 + 0.4 * (y as f64 * 0.031).cos();
            let c = 0.5 + 0.5 * (x as f64 * 0.0043).sin();
            let val = 45.0 + 190.0 * (0.5 * a * b + 0.5 * c);
            v.push(val.round().clamp(0.0, 255.0) as u8);
        }
    }
    v
}

struct Rng(u64);
impl Rng {
    fn gauss(&mut self) -> f32 {
        let mut s = 0.0;
        for _ in 0..6 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            s += ((self.0 >> 33) as f32) / (u32::MAX >> 1) as f32;
        }
        (s - 3.0) * 0.8165
    }
}

/// Audio a 12 kHz del segnale: un secondo di silenzio, APT start (5 s), fasatura, immagine, APT stop (5 s), un po' di silenzio.
fn genera(s: &Segnale, img: &[u8], w: usize, h: usize) -> Vec<f32> {
    let rate = f64::from(RATE);
    let riga = rate * 60.0 / f64::from(s.lpm) * (1.0 + s.ppm * 1e-6);
    let mut out: Vec<f32> = Vec::new();
    let mut phase = 0.0f64;
    let mut rng = Rng(12345);
    // un campione di segnale con il valore di grigio (0..255) dato
    let mut tono = |out: &mut Vec<f32>, v: f64| {
        let f = 1500.0 + 800.0 * v / 255.0 + s.offset_hz;
        phase += TAU * f / rate;
        if phase > TAU {
            phase -= TAU;
        }
        out.push(0.5 * phase.sin() as f32 + s.rumore * rng.gauss());
    };
    out.extend(std::iter::repeat_n(0.0, RATE as usize));
    if s.apt {
        let n = 5 * RATE as usize;
        for i in 0..n {
            let alto = ((2.0 * s.apt_start_hz * i as f64 / rate) as u64) % 2 == 0;
            tono(&mut out, if alto { 255.0 } else { 0.0 });
        }
        // fasatura: 2,5% bianco, 95% nero, 2,5% bianco
        let n = (s.righe_fasatura as f64 * riga) as usize;
        for i in 0..n {
            let frac = (i as f64 / riga).fract();
            tono(&mut out, if !(0.025..=0.975).contains(&frac) { 255.0 } else { 0.0 });
        }
    }
    let n = (h as f64 * riga) as usize;
    for i in 0..n {
        let t = i as f64 / riga;
        let (r, col) = (t as usize, ((t.fract()) * w as f64) as usize);
        tono(&mut out, f64::from(img[r.min(h - 1) * w + col.min(w - 1)]));
    }
    if s.apt {
        let n = 5 * RATE as usize;
        for i in 0..n {
            let alto = ((2.0 * 450.0 * i as f64 / rate) as u64) % 2 == 0;
            tono(&mut out, if alto { 255.0 } else { 0.0 });
        }
        for _ in 0..RATE as usize {
            tono(&mut out, 0.0);
        }
    }
    out.extend(std::iter::repeat_n(0.0, 2 * RATE as usize));
    out
}

struct Esito {
    fatte: Vec<Done>,
    scartate: Vec<String>,
    /// Righe mandate con gli eventi `Rows`, in ordine: (first, count).
    righe: Vec<(u32, u32)>,
    avviate: usize,
    /// Cambi di IOC dal tono d'inizio: (da, a).
    ioc_cambiati: Vec<(u32, u32)>,
}

fn ricevi(cfg: DecoderConfig, audio: &[f32], chunk: usize) -> Esito {
    let mut d = Decoder::new(cfg);
    let mut e = Esito { fatte: Vec::new(), scartate: Vec::new(), righe: Vec::new(), avviate: 0, ioc_cambiati: Vec::new() };
    let gestisci = |evs: Vec<DecoderEvent>, e: &mut Esito| {
        for ev in evs {
            match ev {
                DecoderEvent::Started { .. } => e.avviate += 1,
                DecoderEvent::Rows { first, count } => e.righe.push((first, count)),
                DecoderEvent::Finished(done) => e.fatte.push(done),
                DecoderEvent::Discarded { reason } => e.scartate.push(reason),
                DecoderEvent::IocDetected { from, ioc, .. } => e.ioc_cambiati.push((from, ioc)),
                DecoderEvent::State(_) => {}
            }
        }
    };
    for c in audio.chunks(chunk) {
        let evs = d.push(c);
        gestisci(evs, &mut e);
    }
    e
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

/// Confronta l'immagine ricevuta con l'originale cercando l'offset di righe (le righe di fasatura rimaste in testa) e di colonne
/// (a capo) che da' la correlazione migliore. Ritorna (correlazione, righe in testa, colonne di scarto).
fn confronta(rx: &Done, orig: &[u8], w: usize, h: usize, max_col: i32) -> (f64, usize, i32) {
    assert_eq!(rx.image.width as usize, w);
    let mut best = (-1.0, 0usize, 0i32);
    let n = h - 4;
    for off in 0..=12usize {
        if rx.image.height as usize <= off + n {
            break;
        }
        for dc in -max_col..=max_col {
            let mut a = Vec::with_capacity(n * w);
            let mut b = Vec::with_capacity(n * w);
            for r in 0..n {
                let row = &rx.image.data[(off + r) * w..(off + r + 1) * w];
                for x in 0..w {
                    a.push(row[(x as i32 + dc).rem_euclid(w as i32) as usize]);
                    b.push(orig[r * w + x]);
                }
            }
            let c = pearson(&a, &b);
            if c > best.0 {
                best = (c, off, dc);
            }
        }
    }
    best
}

fn cfg(ioc: u32, lpm: u32) -> DecoderConfig {
    DecoderConfig { ioc, lpm_576: lpm, lpm_288: lpm, ..DecoderConfig::default() }
}

#[test]
fn ioc_576_pulito_a_120_righe_al_minuto() {
    let (w, h) = (width_of(576) as usize, 100);
    assert_eq!(w, 1809);
    let img = immagine(w, h);
    let audio = genera(&Segnale::standard(576, 120), &img, w, h);
    let e = ricevi(cfg(576, 120), &audio, 1024);
    assert_eq!(e.fatte.len(), 1, "scartate: {:?}", e.scartate);
    let rx = &e.fatte[0];
    assert!(rx.complete && rx.reason == "apt_stop", "finita per {}", rx.reason);
    assert_eq!(rx.ioc, 576);
    assert!((rx.lpm - 120.0).abs() < 0.2, "lpm {}", rx.lpm);
    let (c, off, dc) = confronta(rx, &img, w, h, 6);
    eprintln!("IOC 576 pulito: correlazione {c:.4}, {off} righe di fasatura in testa, scarto {dc} colonne, {} righe", rx.image.height);
    assert!(c > 0.98, "correlazione {c}");
    assert!(dc.abs() <= 3, "riga non allineata di {dc} colonne");
    assert!(rx.image.height as usize >= off + h - 4 && rx.image.height as usize <= off + h + 4, "altezza {}", rx.image.height);
    // gli eventi portano le righe una dopo l'altra dalla 0 senza buchi
    let mut next = 0;
    for &(first, count) in &e.righe {
        assert_eq!(first, next, "righe non consecutive");
        next += count;
    }
    assert!(next as usize >= h);
    assert_eq!(e.avviate, 1);
}

#[test]
fn ioc_576_con_scarto_di_frequenza_rumore_e_orologio_lento() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    let s = Segnale { offset_hz: 18.0, rumore: 0.05, ppm: 400.0, ..Segnale::standard(576, 120) };
    let audio = genera(&s, &img, w, h);
    let e = ricevi(cfg(576, 120), &audio, 700);
    assert_eq!(e.fatte.len(), 1, "scartate: {:?}", e.scartate);
    let rx = &e.fatte[0];
    let (c, off, dc) = confronta(rx, &img, w, h, 8);
    eprintln!("IOC 576 +18 Hz, rumore, +400 ppm: correlazione {c:.4}, {off} righe in testa, scarto {dc} colonne");
    assert!(c > 0.95, "correlazione {c}");
    assert!(dc.abs() <= 4);
    // la correzione automatica dell'inclinazione: senza (periodo nominale) la stessa immagine verrebbe molto peggio
    let mut c2 = cfg(576, 120);
    c2.auto_lpm = false;
    let e2 = ricevi(c2, &audio, 700);
    assert_eq!(e2.fatte.len(), 1);
    // con auto_lpm spento il periodo misurato si usa comunque se e' entro l'1,5%: stesso risultato
    assert!(confronta(&e2.fatte[0], &img, w, h, 8).0 > 0.95);
}

#[test]
fn ioc_576_con_filtro_wide_e_correzione_a_mano() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    let s = Segnale { ppm: -250.0, rumore: 0.03, ..Segnale::standard(576, 120) };
    let audio = genera(&s, &img, w, h);
    let mut c = cfg(576, 120);
    c.filter = Filter::Wide;
    let e = ricevi(c, &audio, 2048);
    assert_eq!(e.fatte.len(), 1, "scartate: {:?}", e.scartate);
    assert!(confronta(&e.fatte[0], &img, w, h, 6).0 > 0.95);
}

#[test]
fn ioc_576_a_90_righe_si_riconosce_dalla_fasatura() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    let audio = genera(&Segnale::standard(576, 90), &img, w, h);
    let e = ricevi(cfg(576, 90), &audio, 1024);
    assert_eq!(e.fatte.len(), 1, "scartate: {:?}", e.scartate);
    let rx = &e.fatte[0];
    assert!((rx.lpm - 90.0).abs() < 0.2, "lpm {}", rx.lpm);
    let (c, off, dc) = confronta(rx, &img, w, h, 6);
    eprintln!("IOC 576 a 90 lpm: correlazione {c:.4}, {off} righe in testa, scarto {dc}");
    assert!(c > 0.97);
}

#[test]
fn ioc_288_a_60_righe_al_minuto() {
    let w = width_of(288) as usize;
    assert_eq!(w, 904);
    let h = 100;
    let img = immagine(w, h);
    let s = Segnale { offset_hz: -10.0, rumore: 0.03, ppm: 200.0, ..Segnale::standard(288, 60) };
    let audio = genera(&s, &img, w, h);
    let e = ricevi(cfg(288, 60), &audio, 1024);
    assert_eq!(e.fatte.len(), 1, "scartate: {:?}", e.scartate);
    let rx = &e.fatte[0];
    assert_eq!(rx.ioc, 288);
    assert_eq!(rx.image.width, 904);
    assert!((rx.lpm - 60.0).abs() < 0.2, "lpm {}", rx.lpm);
    let (c, off, dc) = confronta(rx, &img, w, h, 6);
    eprintln!("IOC 288: correlazione {c:.4}, {off} righe in testa, scarto {dc} colonne");
    assert!(c > 0.97, "correlazione {c}");
    assert!(dc.abs() <= 3);
}

#[test]
fn modo_manuale_senza_apt_ne_fasatura_e_chiusura_a_mano() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    let s = Segnale { apt: false, rumore: 0.02, ..Segnale::standard(576, 120) };
    // il segnale e' tutto immagine: dopo un secondo di silenzio le righe cominciano e il silenzio finale e' bianco
    let audio = genera(&s, &img, w, h);
    let mut c = cfg(576, 120);
    c.manual = true;
    c.auto_center = false;
    let mut d = Decoder::new(c);
    let mut finite = Vec::new();
    for chunk in audio.chunks(1024) {
        for ev in d.push(chunk) {
            if let DecoderEvent::Finished(done) = ev {
                finite.push(done);
            }
        }
    }
    assert!(finite.is_empty(), "in manuale non finisce da sola");
    assert!(d.is_receiving());
    let mut fatte = Vec::new();
    for ev in d.finish() {
        if let DecoderEvent::Finished(done) = ev {
            fatte.push(done);
        }
    }
    assert_eq!(fatte.len(), 1);
    let rx = &fatte[0];
    assert_eq!(rx.image.width as usize, w);
    assert_eq!(rx.reason, "manual");
    // un secondo di silenzio (bianco) = 2 righe in testa, e la riga e' a capo di quanto il silenzio non sia multiplo: si cerca lo scarto
    let (corr, off, dc) = confronta(rx, &img, w, h, 40);
    eprintln!("manuale: correlazione {corr:.4}, {off} righe in testa, scarto {dc} colonne");
    assert!(corr > 0.95, "correlazione {corr}");
}

/// Confronta un'immagine che puo' cominciare in un punto qualsiasi della riga (squelch, «parti subito»): il flusso dei pixel e' lo stesso
/// dell'originale spostato di `d` pixel. Lo scarto di colonne si trova dal profilo medio delle colonne, quello di righe provando.
/// Ritorna (correlazione, errore medio assoluto nei livelli di grigio).
fn confronta_flusso(rx: &Done, orig: &[u8], w: usize) -> (f64, f64) {
    assert_eq!(rx.image.width as usize, w);
    let rows = rx.image.height as usize;
    assert!(rows > 20, "immagine di {rows} righe");
    let profilo = |data: &[u8], n: usize| -> Vec<f64> { (0..w).map(|x| (0..n).map(|r| f64::from(data[r * w + x])).sum::<f64>() / n as f64).collect() };
    let n = (rows - 8).min(orig.len() / w - 14);
    let prx = profilo(&rx.image.data[4 * w..], n);
    let por = profilo(orig, n);
    let (mrx, mor) = (prx.iter().sum::<f64>() / w as f64, por.iter().sum::<f64>() / w as f64);
    let dc = (0..w)
        .max_by(|&a, &b| {
            let c = |d: usize| (0..w).map(|x| (prx[x] - mrx) * (por[(x + d) % w] - mor)).sum::<f64>();
            c(a).total_cmp(&c(b))
        })
        .unwrap_or(0) as i64;
    let mut best = (-1.0, f64::MAX);
    for off in -3i64..=12 {
        let d = off * w as i64 + dc;
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for k in 2 * w..(rows - 2) * w {
            let j = k as i64 + d;
            if j >= 0 && (j as usize) < orig.len() {
                a.push(rx.image.data[k]);
                b.push(orig[j as usize]);
            }
        }
        if a.len() < 20 * w {
            continue;
        }
        let c = pearson(&a, &b);
        if c > best.0 {
            let mae = a.iter().zip(&b).map(|(&x, &y)| (f64::from(x) - f64::from(y)).abs()).sum::<f64>() / a.len() as f64;
            best = (c, mae);
        }
    }
    best
}

/// Filtro biquadratico (RBJ) per fare rumore con la forma del filtro SSB di una radio.
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    fn new(low: bool, hz: f64) -> Biquad {
        let w0 = TAU * hz / f64::from(RATE);
        let (s, c) = w0.sin_cos();
        let alpha = s / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let b = if low { [(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0] } else { [(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0] };
        let a0 = 1.0 + alpha;
        Biquad { b: [b[0] / a0, b[1] / a0, b[2] / a0], a: [-2.0 * c / a0, (1.0 - alpha) / a0], z: [0.0; 2] }
    }

    fn run(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

fn cfg_squelch() -> DecoderConfig {
    DecoderConfig { squelch: true, squelch_level: 30.0, auto_center: false, ..cfg(576, 120) }
}

/// Un tono fisso (Hz) per tanti secondi, con un po' di rumore.
fn tono_fisso(hz: f64, secondi: usize) -> Vec<f32> {
    let mut rng = Rng(99);
    (0..RATE as usize * secondi).map(|i| (0.5 * (TAU * hz * i as f64 / f64::from(RATE)).sin()) as f32 + 0.02 * rng.gauss()).collect()
}

#[test]
fn lo_squelch_fa_partire_l_immagine_col_segnale_e_la_chiude_nel_silenzio() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    // prima il tono fisso del nero (la stazione non ha ancora cominciato), poi solo righe d'immagine, poi 6 secondi di silenzio
    let s = Segnale { apt: false, rumore: 0.03, ..Segnale::standard(576, 120) };
    let mut audio = tono_fisso(1500.0, 8);
    audio.extend(genera(&s, &img, w, h));
    audio.extend(std::iter::repeat_n(0.0, 4 * RATE as usize));
    let mut d = Decoder::new(cfg_squelch());
    let (mut avviate, mut fatte, mut scartate) = (Vec::new(), Vec::new(), Vec::new());
    let mut livello_tono = 0.0f32;
    for (i, c) in audio.chunks(1024).enumerate() {
        for ev in d.push(c) {
            match ev {
                DecoderEvent::Started { source, .. } => avviate.push(source),
                DecoderEvent::Finished(done) => fatte.push(done),
                DecoderEvent::Discarded { reason } => scartate.push(reason),
                _ => {}
            }
        }
        if i * 1024 < 7 * RATE as usize {
            livello_tono = d.signal_level();
            assert!(avviate.is_empty(), "il tono fisso non fa partire l'immagine");
        }
    }
    eprintln!("squelch: livello del tono fisso {livello_tono:.1}, avviate {avviate:?}, scartate {scartate:?}");
    // il tono fisso e' segnale presente (sopra la soglia) ma non immagine
    assert!(livello_tono > 30.0);
    assert_eq!(avviate, vec![Source::Squelch]);
    assert_eq!(fatte.len(), 1, "scartate: {scartate:?}");
    let rx = &fatte[0];
    assert_eq!(rx.reason, "squelch");
    assert_eq!(rx.source, Source::Squelch);
    assert!(!d.is_receiving(), "dopo il silenzio l'immagine e' chiusa");
    // 50 secondi a 120 righe al minuto, piu' le 6 righe dei 3 secondi di silenzio prima della chiusura
    assert!((95..=112).contains(&rx.image.height), "altezza {}", rx.image.height);
    let (c, mae) = confronta_flusso(rx, &img, w);
    eprintln!("squelch: correlazione {c:.4}, errore medio {mae:.1}");
    assert!(c > 0.95, "correlazione {c}");
}

#[test]
fn con_lo_squelch_acceso_apt_e_fasatura_danno_una_sola_immagine_allineata() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    let mut audio = genera(&Segnale { rumore: 0.03, ..Segnale::standard(576, 120) }, &img, w, h);
    audio.extend(std::iter::repeat_n(0.0, 4 * RATE as usize));
    let e = ricevi(cfg_squelch(), &audio, 1024);
    // lo squelch non deve aggiungere immagini: il tono APT start (livello basso, l'energia e' sparsa) e la fasatura portano a una sola
    // cartina allineata; se lo squelch si aprisse sul tono, l'immagine che ne nasce si chiuderebbe senza farsi vedere
    assert_eq!(e.avviate, 1);
    assert_eq!(e.fatte.len(), 1);
    assert_eq!(e.fatte[0].source, Source::Phasing);
    assert_eq!(e.fatte[0].reason, "apt_stop");
    let (c, _, dc) = confronta(&e.fatte[0], &img, w, h, 6);
    assert!(c > 0.98 && dc.abs() <= 3, "correlazione {c}, scarto {dc}");
}

#[test]
fn lo_squelch_non_parte_con_il_solo_rumore_ne_con_il_silenzio() {
    let mut rng = Rng(4242);
    // rumore bianco, poi rumore passato da un filtro SSB (300-2700 Hz), poi silenzio
    let mut audio: Vec<f32> = (0..RATE as usize * 20).map(|_| 0.3 * rng.gauss()).collect();
    let mut f = [Biquad::new(false, 300.0), Biquad::new(false, 300.0), Biquad::new(true, 2700.0), Biquad::new(true, 2700.0)];
    audio.extend((0..RATE as usize * 20).map(|_| {
        let mut x = f64::from(0.6 * rng.gauss());
        for b in &mut f {
            x = b.run(x);
        }
        x as f32
    }));
    audio.extend(std::iter::repeat_n(0.0, RATE as usize * 10));
    let mut d = Decoder::new(cfg_squelch());
    let (mut avviate, mut livello_max) = (0, 0.0f32);
    for c in audio.chunks(1024) {
        for ev in d.push(c) {
            if let DecoderEvent::Started { .. } = ev {
                avviate += 1;
            }
        }
        livello_max = livello_max.max(d.signal_level());
    }
    eprintln!("rumore con lo squelch: livello massimo {livello_max:.1}, AFC {:.1} Hz", d.afc_hz());
    assert_eq!(avviate, 0);
    assert!(!d.is_receiving());
    assert!(livello_max < 20.0, "livello del rumore {livello_max}");
    // l'AFC non insegue il rumore
    assert_eq!(d.afc_hz(), 0.0);
}

/// Riceve e restituisce le immagini finite e la correzione finale dell'AFC.
fn ricevi_afc(cfg: DecoderConfig, audio: &[f32]) -> (Vec<Done>, f32) {
    let mut d = Decoder::new(cfg);
    let mut fatte = Vec::new();
    for c in audio.chunks(1024) {
        for ev in d.push(c) {
            if let DecoderEvent::Finished(done) = ev {
                fatte.push(done);
            }
        }
    }
    (fatte, d.afc_hz())
}

#[test]
fn l_afc_corregge_una_radio_sintonizzata_60_hz_fuori() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    let s = Segnale { offset_hz: 60.0, rumore: 0.03, ..Segnale::standard(576, 120) };
    let audio = genera(&s, &img, w, h);
    // senza AFC i grigi sarebbero 19 livelli piu' chiari (60 Hz su 800 Hz di escursione); con questo scarto la fasatura non si riconosce
    // nemmeno: nessuna immagine conta come immagine sbagliata
    let (senza, _) = ricevi_afc(DecoderConfig { afc: false, ..cfg(576, 120) }, &audio);
    let mae_senza = senza.first().map_or(f64::MAX, |rx| confronta_flusso(rx, &img, w).1);
    let (con, afc) = ricevi_afc(cfg(576, 120), &audio);
    assert_eq!(con.len(), 1);
    let (c, mae) = confronta_flusso(&con[0], &img, w);
    eprintln!("AFC +60 Hz: correzione {afc:.1} Hz, errore medio {mae:.1} (senza AFC: {} immagini), correlazione {c:.4}", senza.len());
    assert!((afc - 60.0).abs() < 6.0, "correzione {afc}");
    assert!(mae < 5.0, "errore medio con AFC {mae}");
    assert!(mae_senza > 12.0, "errore medio senza AFC {mae_senza}");
    assert!(c > 0.97);
    // anche dall'altra parte
    let s = Segnale { offset_hz: -60.0, rumore: 0.03, ..Segnale::standard(576, 120) };
    let (con, afc) = ricevi_afc(cfg(576, 120), &genera(&s, &img, w, h));
    assert_eq!(con.len(), 1);
    assert!((afc + 60.0).abs() < 6.0, "correzione {afc}");
    assert!(confronta_flusso(&con[0], &img, w).1 < 5.0);
}

#[test]
fn l_afc_insegue_il_tono_fisso_del_nero_entro_150_hz() {
    // il tono che la stazione manda prima dell'immagine: 1500 Hz, qui con la radio fuori di +45 Hz
    let (_, afc) = ricevi_afc(cfg(576, 120), &tono_fisso(1545.0, 10));
    eprintln!("tono fisso a 1545 Hz: correzione {afc:.1} Hz");
    assert!((afc - 45.0).abs() < 5.0, "correzione {afc}");
    // anche il bianco
    let (_, afc) = ricevi_afc(cfg(576, 120), &tono_fisso(2270.0, 10));
    assert!((afc + 30.0).abs() < 5.0, "correzione {afc}");
    // un tono lontano dal nero e dal bianco non si insegue (prudenza: al massimo 150 Hz)
    let (_, afc) = ricevi_afc(cfg(576, 120), &tono_fisso(1900.0, 10));
    assert_eq!(afc, 0.0);
}

#[test]
fn il_rumore_senza_cartina_non_produce_immagini() {
    let mut rng = Rng(777);
    let audio: Vec<f32> = (0..RATE as usize * 40).map(|_| 0.3 * rng.gauss()).collect();
    let e = ricevi(DecoderConfig::default(), &audio, 1024);
    assert!(e.fatte.is_empty());
    assert_eq!(e.avviate, 0);
}

// ---------------------------------------------------------------- inclinazione automatica

/// Cartina di prova con strutture verticali nette (linee di varia larghezza e grigio, a tratti), fasce orizzontali e riquadri: nessuna
/// pendenza propria, cosi' ogni inclinazione dell'immagine ricevuta viene dall'errore di orologio.
fn cartina(w: usize, h: usize) -> Vec<u8> {
    let mut rng = Rng(777);
    let mut next = move || {
        rng.0 = rng.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (rng.0 >> 33) as usize
    };
    let mut v = vec![215u8; w * h];
    // linee verticali a tratti
    for _ in 0..40 {
        let (x0, lw, grigio) = (next() % (w - 12), 2 + next() % 7, 20 + (next() % 120) as u8);
        let (y0, len) = (next() % h, 30 + next() % h);
        for y in y0..(y0 + len).min(h) {
            for x in x0..x0 + lw {
                v[y * w + x] = grigio;
            }
        }
    }
    // riquadri
    for _ in 0..10 {
        let (x0, bw, grigio) = (next() % (w - 120), 20 + next() % 100, 90 + (next() % 100) as u8);
        let (y0, bh) = (next() % h, 10 + next() % 60);
        for y in y0..(y0 + bh).min(h) {
            for x in x0..x0 + bw {
                v[y * w + x] = grigio;
            }
        }
    }
    // fasce orizzontali sottili
    for k in 0..(h / 40) {
        let y = k * 40 + 7;
        for x in 0..w {
            v[y * w + x] = v[y * w + x].saturating_sub(60);
        }
    }
    v
}

/// Riceve in modo manuale (senza fasatura che misurerebbe l'orologio) e confronta con l'originale: la riga di partenza e la colonna si
/// cercano sulle prime righe, poi la correlazione e' su tutte le righe.
fn ricevi_cartina(ppm: f64, auto_slant: bool, img: &[u8], w: usize, h: usize) -> f64 {
    ricevi_cartina_corretta(ppm, auto_slant, &[], img, w, h)
}

/// Come `ricevi_cartina`, e con `correzioni = [(righe, ppm totale)]` a quei punti della ricezione cambia la correzione a mano senza interrompere
/// l'immagine (come i pulsanti dell'interfaccia): le righe gia' scritte si devono raddrizzare e le nuove nascere dritte.
fn ricevi_cartina_corretta(ppm: f64, auto_slant: bool, correzioni: &[(u32, f32)], img: &[u8], w: usize, h: usize) -> f64 {
    let s = Segnale { apt: false, ppm, ..Segnale::standard(576, 120) };
    let audio = genera(&s, img, w, h);
    let mut c = cfg(576, 120);
    c.manual = true;
    c.auto_center = false;
    c.auto_slant = auto_slant;
    let mut d = Decoder::new(c.clone());
    let mut da_fare = correzioni.iter().copied();
    let mut prossima = da_fare.next();
    for chunk in audio.chunks(1024) {
        d.push(chunk);
        if let Some((righe, v)) = prossima
            && d.current().is_some_and(|(_, rows, _)| rows >= righe)
        {
            let mut nuova = c.clone();
            nuova.slant_ppm = v;
            d.set_config(nuova);
            prossima = da_fare.next();
        }
    }
    assert!(prossima.is_none(), "una correzione non e' mai partita");
    let rx = d.finish().into_iter().find_map(|e| if let DecoderEvent::Finished(done) = e { Some(done) } else { None });
    let rx = rx.expect("immagine non chiusa");
    assert_eq!(rx.image.width as usize, w);
    let rows = rx.image.height as usize;
    // riga di partenza (un secondo di silenzio iniziale = 2 righe in testa) e colonna: profilo delle righe 4..20
    let mut best = (-2.0f64, 0usize, 0usize);
    for off in 0..=4usize {
        for dc in (0..w).step_by(2) {
            let (mut a, mut b) = (Vec::new(), Vec::new());
            for r in 4..20usize {
                for x in (0..w).step_by(3) {
                    a.push(rx.image.data[(off + r) * w + (x + dc) % w]);
                    b.push(img[r * w + x]);
                }
            }
            let c = pearson(&a, &b);
            if c > best.0 {
                best = (c, off, dc);
            }
        }
    }
    let (_, off, dc) = best;
    let n = (rows - off).min(h) - 6;
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for r in 4..n {
        for x in 0..w {
            a.push(rx.image.data[(off + r) * w + (x + dc) % w]);
            b.push(img[r * w + x]);
        }
    }
    pearson(&a, &b)
}

#[test]
fn l_inclinazione_automatica_raddrizza_la_cartina_con_orologio_sbagliato() {
    let (w, h) = (1809usize, 260usize);
    let img = cartina(w, h);
    let mut senza_media = 0.0;
    for ppm in [1500.0, -1500.0, 300.0] {
        let senza = ricevi_cartina(ppm, false, &img, w, h);
        let con = ricevi_cartina(ppm, true, &img, w, h);
        eprintln!("cartina {ppm:+} ppm: correlazione senza {senza:.4}, con inclinazione automatica {con:.4}");
        // misurato: 0,932 (+1500), 0,934 (-1500), 0,944 (+300); senza errore 0,9475; senza correzione 0,08-0,17
        assert!(con > 0.92, "{ppm} ppm: correlazione con auto_slant {con}");
        assert!(con > senza + 0.1, "{ppm} ppm: con {con}, senza {senza}");
        senza_media += senza;
    }
    assert!(senza_media / 3.0 < 0.7, "senza correzione la cartina doveva uscire inclinata");
}

#[test]
fn la_correzione_a_mano_a_immagine_in_corso_raddrizza_le_righe_scritte_e_le_nuove() {
    let (w, h) = (1809usize, 260usize);
    let img = cartina(w, h);
    for ppm in [1500.0f32, -1200.0] {
        let senza = ricevi_cartina_corretta(f64::from(ppm), false, &[], &img, w, h);
        // a meta' immagine si dice quanto era sbagliato l'orologio: tutta l'immagine, anche quanto gia' scritto, deve venire dritta
        let con = ricevi_cartina_corretta(f64::from(ppm), false, &[(120, ppm)], &img, w, h);
        // una correzione data in due tempi (come premendo i pulsanti piu' volte): la seconda vale anche per le righe prima della prima
        let due = ricevi_cartina_corretta(f64::from(ppm), false, &[(70, ppm / 2.0), (170, ppm)], &img, w, h);
        // con l'inclinazione automatica accesa (di base), correzione a mano data prima della prima stima (riga 41): la stima vede il residuo e non
        // deve rovinare niente (dopo che l'automatica ha gia' corretto, una correzione a mano uguale si somma: e' voluto)
        let con_auto = ricevi_cartina_corretta(f64::from(ppm), true, &[(30, ppm)], &img, w, h);
        eprintln!("cartina {ppm:+} ppm: senza correzione {senza:.4}, a mano alla riga 120 {con:.4}, in due tempi {due:.4}, con l'automatica {con_auto:.4}");
        assert!(senza < 0.7, "{ppm} ppm: senza correzione doveva essere storta ({senza})");
        assert!(con > 0.9, "{ppm} ppm: con la correzione a mano a immagine in corso {con}");
        assert!(due > 0.9, "{ppm} ppm: correzione in due tempi {due}");
        assert!(con_auto > 0.9, "{ppm} ppm: correzione a mano con l'inclinazione automatica {con_auto}");
    }
}

#[test]
fn l_inclinazione_automatica_non_peggiora_una_cartina_senza_errore() {
    let (w, h) = (1809usize, 260usize);
    let img = cartina(w, h);
    let senza = ricevi_cartina(0.0, false, &img, w, h);
    let con = ricevi_cartina(0.0, true, &img, w, h);
    eprintln!("cartina senza errore: correlazione senza {senza:.4}, con {con:.4}");
    assert!(senza > 0.9);
    assert!(con > senza - 0.005, "con {con}, senza {senza}");
}

#[test]
fn l_inclinazione_automatica_non_muove_niente_su_immagini_uniformi_o_rumore() {
    let (w, h) = (1809usize, 140usize);
    // uniforme: nessuna struttura da seguire
    let uniforme = vec![128u8; w * h];
    let mut rng = Rng(99);
    let rumore: Vec<u8> = (0..w * h).map(|_| (128.0 + 60.0 * rng.gauss()).clamp(0.0, 255.0) as u8).collect();
    for (nome, img) in [("uniforme", &uniforme), ("rumore", &rumore)] {
        let s = Segnale { apt: false, ppm: 1000.0, ..Segnale::standard(576, 120) };
        let audio = genera(&s, img, w, h);
        let ricevi_con = |auto: bool| {
            let mut c = cfg(576, 120);
            c.manual = true;
            c.auto_center = false;
            c.auto_slant = auto;
            let mut d = Decoder::new(c);
            for chunk in audio.chunks(1024) {
                d.push(chunk);
            }
            let lpm = d.lpm();
            let done = d.finish().into_iter().find_map(|e| if let DecoderEvent::Finished(done) = e { Some(done) } else { None });
            (lpm, done.map(|d| d.image.data))
        };
        let (lpm_off, dati_off) = ricevi_con(false);
        let (lpm_on, dati_on) = ricevi_con(true);
        assert_eq!(lpm_off, lpm_on, "{nome}: il periodo di riga non doveva cambiare");
        assert_eq!(dati_off, dati_on, "{nome}: i pixel non dovevano cambiare");
    }
}

// ---------------------------------------------------------------- IOC automatico

/// Un'immagine di `ioc_tx` ricevuta con l'IOC impostato a `ioc_rx` (le righe al minuto di partenza sono quelle di `lpm`, uguali per i due).
fn ricevi_con_ioc_sbagliato(ioc_tx: u32, ioc_rx: u32, lpm: u32, auto_ioc: bool) -> (Esito, Vec<u8>, usize, usize) {
    let w = width_of(ioc_tx) as usize;
    let h = 100;
    let img = immagine(w, h);
    let s = Segnale { rumore: 0.03, ..Segnale::standard(ioc_tx, lpm) };
    let audio = genera(&s, &img, w, h);
    let c = DecoderConfig { auto_ioc, ..cfg(ioc_rx, lpm) };
    (ricevi(c, &audio, 1024), img, w, h)
}

#[test]
fn l_ioc_si_riconosce_dal_tono_d_inizio_in_tutti_e_due_i_versi() {
    for (ioc_tx, ioc_rx, lpm) in [(288u32, 576u32, 60u32), (576, 288, 120)] {
        let (e, img, w, h) = ricevi_con_ioc_sbagliato(ioc_tx, ioc_rx, lpm, true);
        assert_eq!(e.ioc_cambiati, vec![(ioc_rx, ioc_tx)], "IOC cambiato una volta sola");
        assert_eq!(e.fatte.len(), 1, "scartate: {:?}", e.scartate);
        let rx = &e.fatte[0];
        assert_eq!(rx.ioc, ioc_tx);
        assert_eq!(rx.image.width as usize, w);
        assert_eq!(e.avviate, 1);
        let (c, off, dc) = confronta(rx, &img, w, h, 6);
        eprintln!("IOC {ioc_tx} ricevuto con {ioc_rx} e riconoscimento acceso: correlazione {c:.4}, {off} righe in testa, scarto {dc} colonne");
        assert!(c > 0.97, "correlazione {c}");
        assert!(dc.abs() <= 3);
        // senza il riconoscimento l'immagine, se esce, non ha la larghezza giusta
        let (e, _, w, _) = ricevi_con_ioc_sbagliato(ioc_tx, ioc_rx, lpm, false);
        assert!(e.ioc_cambiati.is_empty());
        for rx in &e.fatte {
            assert_ne!(rx.image.width as usize, w, "con il riconoscimento spento l'IOC resta quello impostato");
            assert_eq!(rx.ioc, ioc_rx);
        }
        eprintln!("IOC {ioc_tx} ricevuto con {ioc_rx} e riconoscimento spento: {} immagini (larghezza {})", e.fatte.len(), e.fatte.first().map_or(0, |r| r.image.width));
    }
}

#[test]
fn l_ioc_non_cambia_in_modo_manuale_ne_senza_tono_d_inizio() {
    // senza tono d'inizio ne' fasatura (modo manuale): l'IOC resta quello impostato
    let (w, h) = (width_of(288) as usize, 60);
    let img = immagine(w, h);
    let s = Segnale { apt: false, ..Segnale::standard(288, 60) };
    let audio = genera(&s, &img, w, h);
    let mut c = cfg(576, 60);
    c.manual = true;
    let mut d = Decoder::new(c);
    for chunk in audio.chunks(1024) {
        for ev in d.push(chunk) {
            assert!(!matches!(ev, DecoderEvent::IocDetected { .. }));
        }
    }
    assert_eq!(d.config().ioc, 576);
    // con il tono dell'altro IOC ma in modo manuale (il tono e' solo segnale) niente cambia
    let mut c = cfg(576, 60);
    c.manual = true;
    let mut d = Decoder::new(c);
    for chunk in genera(&Segnale::standard(288, 60), &img, w, h).chunks(1024) {
        d.push(chunk);
    }
    assert_eq!(d.config().ioc, 576);
    // il rumore e il tono giusto non cambiano niente
    let (e, ..) = ricevi_con_ioc_sbagliato(576, 576, 120, true);
    assert!(e.ioc_cambiati.is_empty());
    assert_eq!(e.fatte.len(), 1);
}

#[test]
fn l_ioc_non_cambia_durante_un_immagine_e_resta_se_si_cambiano_altre_impostazioni() {
    // un'immagine IOC 288 gia' in corso: il tono dell'altro IOC a meta' (il segnale di un'altra stazione) non fa cambiare IOC
    let (w, h) = (width_of(288) as usize, 100);
    let img = immagine(w, h);
    let audio = genera(&Segnale { rumore: 0.03, ..Segnale::standard(288, 60) }, &img, w, h);
    let mut d = Decoder::new(DecoderConfig { auto_ioc: true, ..cfg(288, 60) });
    // 5 s di silenzio+tono, 30 righe di fasatura (30 s): dopo 45 s si e' nell'immagine
    let meta = 45 * RATE as usize;
    for chunk in audio[..meta].chunks(1024) {
        d.push(chunk);
    }
    assert!(d.is_receiving() || d.rx_state() == "image");
    let tono_altro = tono_fisso_onda_quadra(300.0, 4);
    for chunk in tono_altro.chunks(1024) {
        for ev in d.push(chunk) {
            assert!(!matches!(ev, DecoderEvent::IocDetected { .. }), "IOC cambiato dentro l'immagine");
        }
    }
    assert_eq!(d.config().ioc, 288);
    // dopo un riconoscimento un cambio di altre impostazioni non butta l'immagine ne' riporta l'IOC alle impostazioni
    let (w, h) = (width_of(288) as usize, 100);
    let img = immagine(w, h);
    let audio = genera(&Segnale { rumore: 0.03, ..Segnale::standard(288, 60) }, &img, w, h);
    let mut d = Decoder::new(cfg(576, 60));
    for chunk in audio[..45 * RATE as usize].chunks(1024) {
        d.push(chunk);
    }
    assert_eq!(d.config().ioc, 288);
    assert_eq!(d.rx_state(), "image");
    let mut altre = cfg(576, 60);
    altre.afc = false;
    d.set_config(altre);
    assert_eq!(d.config().ioc, 288, "l'IOC riconosciuto resta");
    assert_eq!(d.rx_state(), "image", "l'immagine in corso non si butta");
    // se invece si sceglie un altro IOC nelle impostazioni, vale quello
    d.set_config(cfg(288, 60));
    assert_eq!(d.config().ioc, 288);
    d.set_config(cfg(576, 60));
    assert_eq!(d.config().ioc, 576);
}

/// Onda quadra nero-bianco alla frequenza `hz` (come il tono d'inizio).
fn tono_fisso_onda_quadra(hz: f64, secondi: usize) -> Vec<f32> {
    let rate = f64::from(RATE);
    let mut phase = 0.0f64;
    (0..RATE as usize * secondi)
        .map(|i| {
            let alto = ((2.0 * hz * i as f64 / rate) as u64) % 2 == 0;
            phase += TAU * (1500.0 + if alto { 800.0 } else { 0.0 }) / rate;
            (0.5 * phase.sin()) as f32
        })
        .collect()
}

// ---------------------------------------------------------------- errore di orologio ricordato

/// Riceve una cartina in modo manuale con il decodificatore dato (che si tiene l'errore ricordato) e dice quanto bene combacia con
/// l'originale: (correlazione delle righe 4..30 come erano prima della prima correzione, correlazione su tutte).
fn ricevi_con(d: &mut Decoder, ppm: f64, img: &[u8], w: usize, h: usize) -> (f64, f64) {
    let s = Segnale { apt: false, ppm, ..Segnale::standard(576, 120) };
    let audio = genera(&s, img, w, h);
    // le righe come sono prima della prima correzione dell'inclinazione automatica (alla riga 41): una cartina che parte dritta lo e' gia' li'
    let mut presto: Option<Vec<u8>> = None;
    for chunk in audio.chunks(1024) {
        d.push(chunk);
        if presto.is_none()
            && let Some((cw, rows, data)) = d.current()
            && rows >= 36
        {
            assert_eq!(cw as usize, w);
            presto = Some(data[..36 * w].to_vec());
        }
    }
    let presto = presto.expect("meno di 36 righe");
    let rx = d.finish().into_iter().find_map(|e| if let DecoderEvent::Finished(done) = e { Some(done) } else { None }).expect("immagine non chiusa");
    let rows = rx.image.height as usize;
    // riga di partenza e colonna dalle prime righe (profilo delle righe 4..20), poi confronto sugli intervalli
    let mut best = (-2.0f64, 0usize, 0usize);
    for off in 0..=4usize {
        for dc in (0..w).step_by(2) {
            let (mut a, mut b) = (Vec::new(), Vec::new());
            for r in 4..20usize {
                for x in (0..w).step_by(3) {
                    a.push(rx.image.data[(off + r) * w + (x + dc) % w]);
                    b.push(img[r * w + x]);
                }
            }
            let c = pearson(&a, &b);
            if c > best.0 {
                best = (c, off, dc);
            }
        }
    }
    let (_, off, dc) = best;
    let per = |from: usize, to: usize| {
        let to = to.min((rows - off).min(h) - 6);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for r in from..to {
            for x in 0..w {
                a.push(rx.image.data[(off + r) * w + (x + dc) % w]);
                b.push(img[r * w + x]);
            }
        }
        pearson(&a, &b)
    };
    // le prime righe di prima della correzione, con lo stesso allineamento
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for r in 4..30usize {
        for x in 0..w {
            a.push(presto[(off + r) * w + (x + dc) % w]);
            b.push(img[r * w + x]);
        }
    }
    (pearson(&a, &b), per(4, usize::MAX))
}

fn decoder_manuale() -> Decoder {
    let mut c = cfg(576, 120);
    c.manual = true;
    c.auto_center = false;
    Decoder::new(c)
}

#[test]
fn la_seconda_cartina_parte_gia_dritta_con_l_errore_di_orologio_ricordato() {
    let (w, h) = (1809usize, 260usize);
    let img = cartina(w, h);
    let mut d = decoder_manuale();
    assert_eq!(d.learned_ppm(), 0.0);
    let (presto1, tutta1) = ricevi_con(&mut d, 1500.0, &img, w, h);
    let appreso1 = d.learned_ppm();
    let (presto2, tutta2) = ricevi_con(&mut d, 1500.0, &img, w, h);
    let appreso2 = d.learned_ppm();
    eprintln!("prima cartina +1500 ppm: righe 4-30 {presto1:.3}, tutta {tutta1:.3}, appreso {appreso1:.0}; seconda: righe 4-30 {presto2:.3}, tutta {tutta2:.3}, appreso {appreso2:.0}");
    assert!((appreso1 - 1500.0).abs() < 150.0, "appreso dopo la prima {appreso1}");
    assert!((appreso2 - 1500.0).abs() < 150.0, "appreso dopo la seconda {appreso2}");
    assert!(presto1 < 0.6, "la prima parte storta nelle prime righe: {presto1}");
    assert!(presto2 > 0.9, "la seconda parte gia' dritta: {presto2}");
    assert!(tutta2 > 0.9);
    // un'orologio dall'altra parte: ricordare +1500 non deve rovinare una cartina corretta di -1500 (la stima corregge il residuo)
    let (_, tutta3) = ricevi_con(&mut d, -1500.0, &img, w, h);
    assert!(tutta3 > 0.85, "cartina a -1500 con +1500 ricordato: {tutta3}");
    // il valore appreso si azzera a comando
    d.reset_learned();
    assert_eq!(d.learned_ppm(), 0.0);
    assert_eq!(d.learned_weight(), 0.0);
}

#[test]
fn un_immagine_uniforme_o_senza_strutture_non_cambia_il_valore_ricordato() {
    let (w, h) = (1809usize, 200usize);
    let uniforme = vec![128u8; w * h];
    let mut c = cfg(576, 120);
    c.manual = true;
    c.auto_center = false;
    c.learned_ppm = 1500.0;
    c.learned_weight = 3.0;
    let mut d = Decoder::new(c);
    let s = Segnale { apt: false, ppm: 1500.0, ..Segnale::standard(576, 120) };
    for chunk in genera(&s, &uniforme, w, h).chunks(1024) {
        d.push(chunk);
    }
    d.finish();
    assert_eq!(d.learned_ppm(), 1500.0);
    assert_eq!(d.learned_weight(), 3.0);
    // il valore di partenza e' limitato a +-5000 ppm
    let mut c = cfg(576, 120);
    c.learned_ppm = 90_000.0;
    assert_eq!(Decoder::new(c).learned_ppm(), 5000.0);
}

#[test]
fn con_la_fasatura_l_errore_ricordato_non_si_somma_alla_misura() {
    // con +1500 ppm e la fasatura il periodo e' gia' misurato: sommare l'errore ricordato lo raddoppierebbe
    let (w, h) = (1809usize, 260usize);
    let img = cartina(w, h);
    let audio = genera(&Segnale { ppm: 1500.0, ..Segnale::standard(576, 120) }, &img, w, h);
    let corr = |learned: f32| {
        let mut c = cfg(576, 120);
        // con l'AFC acceso: lo sfondo grigio uniforme (215) della cartina di prova non e' un tono fisso e non sposta la sottoportante
        c.learned_ppm = learned;
        c.learned_weight = 3.0;
        let mut d = Decoder::new(c);
        let (mut fatte, mut scartate) = (Vec::new(), Vec::new());
        for chunk in audio.chunks(1024) {
            for ev in d.push(chunk) {
                match ev {
                    DecoderEvent::Finished(done) => fatte.push(done),
                    DecoderEvent::Discarded { reason } => scartate.push(reason),
                    _ => {}
                }
            }
        }
        assert_eq!(fatte.len(), 1, "ricordato {learned}: scartate {scartate:?}");
        let (c, _, _) = confronta(&fatte[0], &img, w, h, 8);
        (c, d.learned_ppm())
    };
    let (senza, _) = corr(0.0);
    let (con, appreso) = corr(1500.0);
    eprintln!("fasatura +1500 ppm: correlazione senza ricordato {senza:.4}, con +1500 ricordato {con:.4}; ricordato dopo: {appreso:.0}");
    assert!(senza > 0.9, "senza {senza}");
    assert!((con - senza).abs() < 0.01, "con {con}, senza {senza}");
    assert!((appreso - 1500.0).abs() < 200.0, "ricordato {appreso}");
}

// ---------------------------------------------------------------- AFC con due limiti

/// Correzione dell'AFC dopo ogni secondo di audio (un blocco da 1 s alla volta; 12000 campioni).
fn afc_al_secondo(cfg: DecoderConfig, audio: &[f32]) -> Vec<f32> {
    let mut d = Decoder::new(cfg);
    audio
        .chunks(RATE as usize)
        .map(|c| {
            d.push(c);
            d.afc_hz()
        })
        .collect()
}

/// Un tono che sale piano da `da` Hz di `hz_al_secondo` Hz ogni secondo, per `secondi`.
fn tono_in_deriva(da: f64, hz_al_secondo: f64, secondi: usize) -> Vec<f32> {
    let rate = f64::from(RATE);
    let mut phase = 0.0f64;
    (0..RATE as usize * secondi)
        .map(|i| {
            phase += TAU * (da + hz_al_secondo * i as f64 / rate) / rate;
            (0.5 * phase.sin()) as f32
        })
        .collect()
}

#[test]
fn il_limite_locale_dell_afc_rallenta_un_salto_grande() {
    // il tono del nero con la radio fuori di +100 Hz: ogni passo e' al massimo 25 Hz, il salto si raggiunge in piu' misure
    let afc = afc_al_secondo(cfg(576, 120), &tono_fisso(1600.0, 12));
    eprintln!("AFC a +100 Hz, un valore al secondo: {afc:.1?}");
    let passi: Vec<f32> = std::iter::once(0.0).chain(afc.iter().copied()).collect::<Vec<_>>().windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    assert!(passi.iter().all(|&p| p <= 25.01), "passo oltre 25 Hz: {passi:?}");
    assert!(afc[0] > 20.0 && afc[0] <= 25.01, "primo passo {}", afc[0]);
    assert!(afc[2] <= 75.1, "dopo tre misure {} (senza il limite sarebbe gia' a 97)", afc[2]);
    assert!((afc[11] - 100.0).abs() < 6.0, "alla fine {}", afc[11]);
}

#[test]
fn il_limite_globale_dell_afc_ferma_la_correzione_e_la_deriva() {
    // la radio fuori di +150 Hz con un limite di 40: la correzione si ferma li', non va oltre
    let afc = afc_al_secondo(DecoderConfig { afc_range_hz: 40.0, ..cfg(576, 120) }, &tono_fisso(1650.0, 10));
    eprintln!("AFC a +150 Hz con limite 40: {:.2}", afc[9]);
    assert!(afc.iter().all(|&a| a <= 40.0), "oltre il limite: {afc:?}");
    assert!((afc[9] - 40.0).abs() < 0.01, "alla fine {}", afc[9]);
    // con il limite a 400 la stessa radio si segue fino in fondo
    let afc = afc_al_secondo(DecoderConfig { afc_range_hz: 400.0, ..cfg(576, 120) }, &tono_fisso(1650.0, 12));
    assert!((afc[11] - 150.0).abs() < 8.0, "limite 400: {}", afc[11]);
    // una deriva lenta (5 Hz al secondo per 40 secondi, 200 Hz in tutto): con il limite a 60 la correzione si ferma a 60, con 400 la segue
    let deriva = tono_in_deriva(1500.0, 5.0, 40);
    let corto = afc_al_secondo(DecoderConfig { afc_range_hz: 60.0, ..cfg(576, 120) }, &deriva);
    let lungo = afc_al_secondo(DecoderConfig { afc_range_hz: 400.0, ..cfg(576, 120) }, &deriva);
    eprintln!("deriva di 200 Hz: limite 60 -> {:.1}, limite 400 -> {:.1}", corto[39], lungo[39]);
    assert!(corto.iter().all(|&a| a <= 60.0), "oltre il limite: {corto:?}");
    assert!((corto[39] - 60.0).abs() < 0.01, "limite 60: {}", corto[39]);
    assert!(lungo[39] > 170.0, "limite 400: {}", lungo[39]);
}

#[test]
fn l_afc_dentro_l_immagine_e_piu_cauto_e_non_scambia_una_zona_uniforme_per_un_tono() {
    let (w, h) = (1809usize, 100usize);
    // barre nere e bianche di larghezza varia (nero e bianco veri: due picchi per l'AFC)
    let mut rng = Rng(31337);
    let img: Vec<u8> = (0..h)
        .flat_map(|_| {
            let mut riga = Vec::with_capacity(w);
            let mut nero = false;
            while riga.len() < w {
                let n = 8 + (rng.gauss().abs() * 40.0) as usize;
                riga.extend(std::iter::repeat_n(if nero { 0u8 } else { 255 }, n.min(w - riga.len())));
                nero = !nero;
            }
            riga
        })
        .collect();
    // in modo manuale si e' sempre dentro l'immagine: radio fuori di +60 Hz, passi di al massimo 8 Hz
    let s = Segnale { apt: false, offset_hz: 60.0, rumore: 0.02, ..Segnale::standard(576, 120) };
    let mut c = cfg(576, 120);
    c.manual = true;
    let afc = afc_al_secondo(c.clone(), &genera(&s, &img, w, h)[..40 * RATE as usize]);
    let passi: Vec<f32> = std::iter::once(0.0).chain(afc.iter().copied()).collect::<Vec<_>>().windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    eprintln!("AFC in immagine a +60 Hz: {:.1?}", &afc[..12]);
    assert!(passi.iter().all(|&p| p <= 8.01), "passo oltre 8 Hz: {passi:?}");
    assert!(afc[39] > 30.0, "l'AFC cauto segue comunque: {}", afc[39]);
    // un tono fisso in mezzo all'immagine non si insegue (potrebbe essere una zona uniforme della cartina)
    let afc = afc_al_secondo(c, &tono_fisso(1545.0, 10));
    assert!(afc.iter().all(|&a| a == 0.0), "tono fisso in immagine: {afc:?}");
}

#[test]
fn l_afc_corregge_di_piu_sul_tono_d_inizio_e_sulla_fasatura_prima_dell_immagine() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    let s = Segnale { offset_hz: 60.0, rumore: 0.03, ..Segnale::standard(576, 120) };
    let audio = genera(&s, &img, w, h);
    let mut d = Decoder::new(cfg(576, 120));
    let mut all_inizio = None;
    for c in audio.chunks(1024) {
        d.push(c);
        if all_inizio.is_none() && d.rx_state() == "image" {
            all_inizio = Some(d.afc_hz());
        }
    }
    let all_inizio = all_inizio.expect("l'immagine non e' partita");
    eprintln!("AFC a +60 Hz: all'inizio dell'immagine {all_inizio:.1} Hz");
    // l'AFC e' arrivato sul tono d'inizio e sulla fasatura (passi da 25 Hz al massimo): dentro l'immagine finisce il lavoro con piu' cautela
    assert!(all_inizio > 45.0 && all_inizio < 66.0, "all'inizio dell'immagine {all_inizio}");
}

/// Due trasmissioni attaccate: la stazione chiude la prima con il tono di fine e comincia subito la seconda (tono d'inizio, fasatura, immagine),
/// senza silenzio in mezzo. Si toglie il silenzio finale della prima e quello iniziale della seconda.
fn due_cartine_di_seguito(ioc_b: u32, lpm_b: u32, w_b: usize) -> (Vec<f32>, Vec<u8>, Vec<u8>, usize, usize) {
    due_cartine_con_pausa(ioc_b, lpm_b, w_b, 0)
}

/// Come sopra con `pausa_s` secondi di silenzio tra la fine della prima e l'inizio della seconda.
fn due_cartine_con_pausa(ioc_b: u32, lpm_b: u32, w_b: usize, pausa_s: usize) -> (Vec<f32>, Vec<u8>, Vec<u8>, usize, usize) {
    let (w_a, h) = (1809usize, 80usize);
    let img_a = immagine(w_a, h);
    let img_b = cartina(w_b, h);
    let a = genera(&Segnale { rumore: 0.03, ..Segnale::standard(576, 120) }, &img_a, w_a, h);
    let b = genera(&Segnale { rumore: 0.03, ..Segnale::standard(ioc_b, lpm_b) }, &img_b, w_b, h);
    let coda = 2 * RATE as usize; // il silenzio finale che `genera` aggiunge alla prima
    let mut audio = a[..a.len() - coda].to_vec();
    audio.extend(std::iter::repeat_n(0.0, pausa_s * RATE as usize));
    audio.extend_from_slice(&b[RATE as usize..]);
    (audio, img_a, img_b, w_a, h)
}

#[test]
fn con_il_tono_di_fine_e_subito_un_altra_cartina_escono_due_immagini_giuste() {
    let (audio, img_a, img_b, w_a, h) = due_cartine_di_seguito(576, 120, 1809);
    for (nome, cfg) in [("senza squelch", cfg(576, 120)), ("con squelch", cfg_squelch())] {
        let e = ricevi(cfg, &audio, 1024);
        eprintln!("{nome}: {} immagini, avviate {}, scartate {:?}", e.fatte.len(), e.avviate, e.scartate);
        assert_eq!(e.fatte.len(), 2, "{nome}: immagini {}, scartate {:?}", e.fatte.len(), e.scartate);
        let (c1, _, d1) = confronta(&e.fatte[0], &img_a, w_a, h, 6);
        let (c2, _, d2) = confronta(&e.fatte[1], &img_b, 1809, h, 6);
        eprintln!("{nome}: prima correlazione {c1:.4} (scarto {d1}), seconda {c2:.4} (scarto {d2}), motivi {} / {}", e.fatte[0].reason, e.fatte[1].reason);
        assert!(c1 > 0.95 && d1.abs() <= 3, "{nome}: prima {c1} scarto {d1}");
        assert!(c2 > 0.95 && d2.abs() <= 3, "{nome}: seconda {c2} scarto {d2}");
        assert_eq!(e.fatte[0].reason, "apt_stop", "{nome}: la prima si chiude dal tono di fine");
    }
}

#[test]
fn con_il_tono_di_fine_e_subito_una_cartina_con_l_altro_ioc_esce_la_seconda_giusta() {
    let w_b = width_of(288) as usize;
    let (audio, _img_a, img_b, _w_a, h) = due_cartine_di_seguito(288, 60, w_b);
    // righe al minuto di ciascun IOC come nelle impostazioni vere: 120 per il 576, 60 per il 288
    let e = ricevi(DecoderConfig { ioc: 576, lpm_576: 120, lpm_288: 60, ..DecoderConfig::default() }, &audio, 1024);
    eprintln!("IOC diverso: {} immagini, cambi IOC {:?}, scartate {:?}", e.fatte.len(), e.ioc_cambiati, e.scartate);
    assert_eq!(e.fatte.len(), 2, "immagini {}, scartate {:?}", e.fatte.len(), e.scartate);
    assert_eq!(e.fatte[1].ioc, 288);
    let (c2, _, d2) = confronta(&e.fatte[1], &img_b, w_b, h, 6);
    eprintln!("IOC diverso: seconda correlazione {c2:.4} (scarto {d2})");
    assert!(c2 > 0.95 && d2.abs() <= 3, "seconda {c2} scarto {d2}");
}

// ---------------------------------------------------------------- inclinazione con intestazione bianca

/// Righe di intestazione quasi bianca in testa alla cartina con intestazione.
const INTESTAZIONE: usize = 250;

/// Cartina meteo con `INTESTAZIONE` righe quasi bianche (due righe di "testo" e due linee orizzontali nere), poi griglia di meridiani e
/// paralleli e una costa.
fn cartina_con_intestazione(w: usize, h: usize) -> Vec<u8> {
    let mut rng = Rng(4242);
    let mut next = move || {
        rng.0 = rng.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (rng.0 >> 33) as usize
    };
    let mut v = vec![228u8; w * h];
    // testo: lettere come blocchetti scuri su due righe di scritta
    for &(y0, x_start) in &[(60usize, 300usize), (110, 500)] {
        let mut x = x_start;
        for _ in 0..30 {
            let (lw, lh) = (6 + next() % 8, 8 + next() % 6);
            for y in y0..y0 + lh {
                for xx in x..x + lw {
                    if next() % 3 != 0 {
                        v[y * w + xx] = 40;
                    }
                }
            }
            x += lw + 4 + next() % 6;
        }
    }
    for y in [200usize, 204] {
        for x in 0..w {
            v[y * w + x] = 10;
        }
    }
    // griglia
    for y in INTESTAZIONE..h {
        for x in 0..w {
            if x % 113 < 4 || (y - INTESTAZIONE) % 60 < 2 {
                v[y * w + x] = 30;
            }
        }
        // costa: curva lenta, spessa 5 pixel
        let xc = (w as f64 * (0.45 + 0.2 * ((y as f64) * 0.013).sin())) as usize;
        for x in xc..xc + 5 {
            v[y * w + x] = 20;
        }
    }
    v
}

/// Spostamento lineare (in pixel, la riga ricevuta prosegue nella successiva: una correzione a immagine in corso puo' spostare l'inizio della
/// riga) intorno a `base` con cui le righe `r0..r1` ricevute combaciano meglio con le stesse dell'originale.
fn spostamento_migliore(rx: &[u8], img: &[u8], w: usize, base: usize, r0: usize, r1: usize) -> i64 {
    let mut best = (-2.0f64, 0i64);
    for k in -(w as i64 / 2)..(w as i64 / 2) {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for r in (r0..r1).step_by(3) {
            for x in (0..w).step_by(3) {
                a.push(rx[(base as i64 + (r * w + x) as i64 + k) as usize]);
                b.push(img[r * w + x]);
            }
        }
        let c = pearson(&a, &b);
        if c > best.0 {
            best = (c, k);
        }
    }
    best.1
}

/// Riceve la cartina con intestazione con il decoder dato: correlazione della parte disegnata con l'originale e scarto in colonne tra
/// l'inizio e la fine della parte disegnata (0 se le linee verticali sono dritte).
fn ricevi_intestazione(d: &mut Decoder, ppm: f64, img: &[u8], w: usize, h: usize) -> (f64, i64) {
    let s = Segnale { apt: false, ppm, ..Segnale::standard(576, 120) };
    let audio = genera(&s, img, w, h);
    for chunk in audio.chunks(1024) {
        d.push(chunk);
    }
    let rx = d.finish().into_iter().find_map(|e| if let DecoderEvent::Finished(done) = e { Some(done) } else { None }).expect("immagine non chiusa");
    let data = &rx.image.data;
    let rows = rx.image.height as usize;
    // riga di partenza (circa): le due linee nere a 200 e 204
    let off = (1..=4usize)
        .max_by_key(|&o| {
            let m = |r: usize| data[(o + r) * w..(o + r + 1) * w].iter().map(|&v| 255 - u32::from(v)).sum::<u32>();
            m(200) + m(204)
        })
        .unwrap_or(1);
    let fine = (rows - off).min(h) - 6;
    let k_inizio = spostamento_migliore(data, img, w, off * w, INTESTAZIONE + 5, INTESTAZIONE + 45);
    let k_fine = spostamento_migliore(data, img, w, off * w, fine - 40, fine);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for r in INTESTAZIONE..fine {
        for x in 0..w {
            a.push(data[((off * w + r * w + x) as i64 + k_inizio) as usize]);
            b.push(img[r * w + x]);
        }
    }
    (pearson(&a, &b), k_fine - k_inizio)
}

#[test]
fn l_inclinazione_automatica_raddrizza_una_cartina_che_inizia_con_un_intestazione_bianca() {
    let (w, h) = (1809usize, 560usize);
    let img = cartina_con_intestazione(w, h);
    // misurato: senza errore 0,956; con errore senza correzione automatica la parte disegnata scorre di ~0,55 colonne per riga ogni 300 ppm
    let mut d = decoder_manuale();
    let (dritta, _) = ricevi_intestazione(&mut d, 0.0, &img, w, h);
    assert!(dritta > 0.94, "senza errore {dritta}");
    for ppm in [300.0, -300.0, 800.0, -800.0, 1500.0, -1500.0] {
        let mut c = cfg(576, 120);
        c.manual = true;
        c.auto_center = false;
        c.auto_slant = false;
        let (senza, scarto_senza) = ricevi_intestazione(&mut Decoder::new(c), ppm, &img, w, h);
        let mut d = decoder_manuale();
        let (corr, scarto) = ricevi_intestazione(&mut d, ppm, &img, w, h);
        let appreso = f64::from(d.learned_ppm());
        eprintln!(
            "intestazione bianca, {ppm:+} ppm: senza correzione {senza:.4} (scarto {scarto_senza}), con {corr:.4} (scarto {scarto} colonne), appreso {appreso:.0}"
        );
        assert!(corr > dritta - 0.02, "{ppm} ppm: correlazione {corr}");
        assert!(scarto.abs() <= 2, "{ppm} ppm: scarto {scarto} colonne");
        assert!((appreso - ppm).abs() < 0.05 * ppm.abs() + 20.0, "{ppm} ppm: appreso {appreso}");
    }
}

#[test]
fn l_inclinazione_automatica_e_stabile_su_una_cartina_lunga_e_non_insegue_il_rumore() {
    // cartina lunga con l'intestazione e rumore sul segnale: dopo la prima stima le correzioni successive restano piccole e non oscillano
    let (w, h) = (1809usize, 1000usize);
    let img = cartina_con_intestazione(w, h);
    for ppm in [800.0, -1500.0] {
        let s = Segnale { apt: false, ppm, rumore: 0.15, ..Segnale::standard(576, 120) };
        let audio = genera(&s, &img, w, h);
        let mut d = decoder_manuale();
        let mut lpm_visti: Vec<f64> = Vec::new();
        let mut ultima = 0u32;
        for chunk in audio.chunks(1024) {
            d.push(chunk);
            if let Some((_, rows, _)) = d.current()
                && rows >= ultima + 40
            {
                ultima = rows;
                lpm_visti.push(f64::from(d.lpm()));
            }
        }
        d.finish();
        // periodo di riga in ppm rispetto al nominale (120 righe al minuto), dalla riga 320 (dopo la prima stima) in poi
        let ppm_visti: Vec<f64> = lpm_visti.iter().skip(8).map(|&l| (120.0 / l - 1.0) * 1e6).collect();
        let salto = ppm_visti.windows(2).map(|c| (c[1] - c[0]).abs()).fold(0.0, f64::max);
        let appreso = f64::from(d.learned_ppm());
        eprintln!("cartina lunga {ppm:+} ppm con rumore: periodo stimato {:.0}..{:.0} ppm, salto massimo {salto:.1} ppm, appreso {appreso:.0}",
            ppm_visti.iter().copied().fold(f64::MAX, f64::min), ppm_visti.iter().copied().fold(f64::MIN, f64::max));
        assert!(salto < 40.0, "{ppm} ppm: salto tra stime successive {salto}");
        assert!(ppm_visti.iter().all(|v| (v - ppm).abs() < 60.0), "{ppm} ppm: stime {ppm_visti:?}");
        assert!((appreso - ppm).abs() < 0.05 * ppm.abs() + 20.0, "{ppm} ppm: appreso {appreso}");
    }
}

// ---------------------------------------------------------------- stazioni senza tono d'inizio e con la fasatura corta

/// Audio di una stazione che sta ferma sul bianco, manda solo `barre` righe di fasatura (95% nero, il resto bianco ai capi) senza tono
/// d'inizio e poi subito l'immagine, come DDK3 su 7880 kHz. `falso_apt_s`: secondi di un finto tono d'inizio all'inizio.
fn senza_tono_d_inizio(img: &[u8], w: usize, h: usize, bianco_s: usize, barre: usize, falso_apt_s: usize, bianco_nero: f64) -> Vec<f32> {
    let rate = f64::from(RATE);
    let riga = rate * 60.0 / 120.0;
    let mut out: Vec<f32> = Vec::new();
    let mut phase = 0.0f64;
    let mut tono = |out: &mut Vec<f32>, v: f64| {
        phase += TAU * (1500.0 + 800.0 * v / 255.0) / rate;
        if phase > TAU {
            phase -= TAU;
        }
        out.push(0.5 * phase.sin() as f32);
    };
    for i in 0..falso_apt_s * RATE as usize {
        let alto = ((2.0 * 300.0 * i as f64 / rate) as u64) % 2 == 0;
        tono(&mut out, if alto { 255.0 } else { 0.0 });
    }
    for _ in 0..bianco_s * RATE as usize {
        tono(&mut out, 255.0);
    }
    // barre di fasatura: nero per `bianco_nero` della riga (di norma 0,95), il resto bianco ai capi
    let n = (barre as f64 * riga) as usize;
    let capo = (1.0 - bianco_nero) / 2.0;
    for i in 0..n {
        let frac = (i as f64 / riga).fract();
        tono(&mut out, if !(capo..=1.0 - capo).contains(&frac) { 255.0 } else { 0.0 });
    }
    let n = (h as f64 * riga) as usize;
    for i in 0..n {
        let t = i as f64 / riga;
        let (r, col) = (t as usize, (t.fract() * w as f64) as usize);
        tono(&mut out, f64::from(img[r.min(h - 1) * w + col.min(w - 1)]));
    }
    for _ in 0..3 * RATE as usize {
        tono(&mut out, 255.0);
    }
    out
}

#[test]
fn una_fasatura_corta_senza_tono_d_inizio_fa_partire_e_allinea_la_cartina() {
    let (w, h) = (1809usize, 160usize);
    // fondo bianco con grandi blocchi neri (come il testo e le cornici di una cartina), che danno i passaggi bianco-nero di un'immagine
    let img: Vec<u8> = (0..w * h).map(|i| if ((i % w) / 60 + (i / w) / 7) % 5 == 0 { 0 } else { 215 }).collect();
    // 20 s di bianco, poi tre barre (90% nero, come DDK3) e subito l'immagine
    let audio = senza_tono_d_inizio(&img, w, h, 20, 3, 0, 0.90);
    // senza tono di fine l'immagine si chiude a mano
    let mut d = Decoder::new(cfg(576, 120));
    let mut fatte: Vec<Done> = Vec::new();
    let mut avviate = 0;
    for c in audio.chunks(1024) {
        for ev in d.push(c) {
            match ev {
                DecoderEvent::Started { .. } => avviate += 1,
                DecoderEvent::Finished(done) => fatte.push(done),
                _ => {}
            }
        }
    }
    for ev in d.finish() {
        if let DecoderEvent::Finished(done) = ev {
            fatte.push(done);
        }
    }
    assert_eq!(avviate, 1);
    assert_eq!(fatte.len(), 1);
    let e = Esito { fatte, scartate: Vec::new(), righe: Vec::new(), avviate, ioc_cambiati: Vec::new() };
    let (c, _off, dc) = confronta(&e.fatte[0], &img, w, h, 120);
    eprintln!("fasatura corta: correlazione {c:.4}, scarto {dc} colonne");
    assert!(c > 0.95, "correlazione {c}");
    assert!(dc.abs() <= 60, "riga non allineata: {dc} colonne");
}

#[test]
fn un_falso_tono_d_inizio_non_blocca_l_attesa_per_sempre() {
    let (w, h) = (1809usize, 100usize);
    let img = immagine(w, h);
    // un finto tono d'inizio di 3 s, poi 120 s di bianco, poi una cartina vera con il suo tono d'inizio e la fasatura
    let mut audio = senza_tono_d_inizio(&img, w, 1, 120, 0, 3, 0.95);
    audio.extend(genera(&Segnale::standard(576, 120), &img, w, h));
    let e = ricevi(cfg(576, 120), &audio, 1024);
    assert!(e.fatte.iter().any(|d| d.image.height as usize >= h), "nessuna cartina completa: {} immagini, scartate {:?}", e.fatte.len(), e.scartate);
}

#[test]
fn imposta_inizio_riga_sposta_le_righe_dell_immagine_in_corso_e_quelle_che_arrivano() {
    let (w, h) = (1809usize, 120usize);
    let img = immagine(w, h);
    let s = Segnale { apt: false, ..Segnale::standard(576, 120) };
    let audio = genera(&s, &img, w, h);
    let decodifica = |inizio: Option<(usize, u32)>| -> Vec<u8> {
        let mut d = decoder_manuale();
        let mut n = 0usize;
        let mut fatto = false;
        for chunk in audio.chunks(1024) {
            d.push(chunk);
            n += chunk.len();
            if let Some((dopo_s, x)) = inizio
                && !fatto
                && n >= dopo_s * RATE as usize
            {
                d.set_line_start(x);
                fatto = true;
            }
        }
        d.finish().into_iter().find_map(|e| if let DecoderEvent::Finished(done) = e { Some(done.image.data) } else { None }).expect("immagine chiusa")
    };
    let normale = decodifica(None);
    let spostata = decodifica(Some((20, 300)));
    // una riga lontana dal punto in cui si e' fatto lo spostamento: e' quella normale con la colonna 300 all'inizio
    let r = 100usize;
    let mut atteso = normale[r * w..(r + 1) * w].to_vec();
    atteso.rotate_left(300);
    let ottenuta = &spostata[r * w..(r + 1) * w];
    let uguali = atteso.iter().zip(ottenuta).filter(|(a, b)| a == b).count();
    assert!(uguali * 100 / w >= 98, "solo {uguali} colonne su {w} sono quelle attese");
}
