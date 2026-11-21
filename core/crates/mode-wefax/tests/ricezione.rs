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
}

fn ricevi(cfg: DecoderConfig, audio: &[f32], chunk: usize) -> Esito {
    let mut d = Decoder::new(cfg);
    let mut e = Esito { fatte: Vec::new(), scartate: Vec::new(), righe: Vec::new(), avviate: 0 };
    let gestisci = |evs: Vec<DecoderEvent>, e: &mut Esito| {
        for ev in evs {
            match ev {
                DecoderEvent::Started { .. } => e.avviate += 1,
                DecoderEvent::Rows { first, count } => e.righe.push((first, count)),
                DecoderEvent::Finished(done) => e.fatte.push(done),
                DecoderEvent::Discarded { reason } => e.scartate.push(reason),
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
