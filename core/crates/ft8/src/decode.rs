//! La decodifica di un periodo intero (`ft8_decode.f90` e la parte di controllo di `ft8b.f90`, JTDX 2.2.159): passate con ricerca dei
//! candidati diversa a rotazione, sottrazione di ogni segnale appena decodificato, analisi a priori, filtri sui falsi.
//!
//! # Come lavora una passata
//!
//! 1. [`sync::find_candidates_ex`] cerca i candidati sull'audio di quel momento, con la forma dello spettro della passata (ampiezza,
//!    potenza, somma degli assoluti) e la soglia `syncmin` della passata (1,5; con [`Options::low_threshold`] 1,225 / 1,5 / 1,1 a rotazione).
//! 2. Per ogni candidato ([`try_candidate`]): discesa a 200 Hz, allineamento in ritardo e frequenza, controlli di sincronismo
//!    ([`filters::sync_screen`]), le serie di LLR (a, b, c, d), e per ognuna BP e poi OSD; poi i tipi a priori ([`ap`]).
//! 3. Una parola che passa il CRC deve avere `nharderrors + dmin < 60`, un testo che si legge e che non sembra un falso
//!    ([`filters::reject_message`]).
//! 4. Il segnale decodificato si toglie **subito** dall'audio ([`subtract`]), con il ritardo affinato: i candidati dopo vedono gia' il
//!    residuo. Se un candidato e' entro 50 Hz da un segnale appena tolto la FFT lunga si rifa, come `ft8_downsample` di JTDX.
//!
//! Il calendario delle passate e' quello di JTDX: 3, 6 o 9 secondo la profondita' ([`Options::depth`]); la quarta passata legge l'audio
//! a mezzo campione, la settima a mezzo campione dall'altra parte, la 5 e la 7 leggono i simboli a tempo invertito, dalla sesta non si
//! sottrae piu' (ne' nella terza di un ciclo da tre).
//!
//! # Differenze rispetto a JTDX
//!
//! - Un solo thread (JTDX divide la banda in fasce, una per thread).
//! - Non ci sono la cronologia dei periodi precedenti (`evencopy`/`oddcopy`: `ft8sd`, `ft8mf1`), le sotto-passate che mediano con il
//!   periodo prima (`isubp1` 3-11), il decoder `ft8s` del segnale di QSO ne' i candidati virtuali sulla frequenza di ricezione.
//! - Nessun controllo `chkgrid` (geografico) ne' `ALLCALL7.TXT`: vedi [`filters`].

use std::sync::{Arc, OnceLock};

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use crate::ap::{self, ApContext, ApOptions, NAPTYPES};
use crate::filters::{self, CallBook, MessageCheck};
use crate::message::{self, CallHashes};
use crate::subtract::{self, Reference};
use crate::sync::{self, Candidate, Complex, Downsampler, MetricsMode, PassCandidate, Spectrum, SyncKind};
use crate::{ldpc, modem, NN, SLOT_SAMPLES};

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    /// Frequenza del tono 0 (Hz): quella che JTDX scrive nella colonna delle frequenze.
    pub freq_hz: f32,
    /// Scarto di tempo come lo scrive JTDX (secondi).
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz (dB, intero come nell'elenco dei messaggi).
    pub snr_db: i32,
    pub text: String,
    /// Trovato con un'analisi a priori (`ap_type` diverso da 0): il marcatore `*` di JTDX.
    pub hint: bool,
}

/// Un messaggio decodificato con le grandezze con cui e' stato accettato (per le misure e per tarare i filtri).
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedInfo {
    pub decoded: Decoded,
    /// Passata (da 1) in cui e' uscito.
    pub pass: usize,
    /// Tipo a priori con cui e' stato trovato (0 = nessuno): vedi [`ap`].
    pub ap_type: u8,
    /// `nharderrors` e `dmin` della parola accettata.
    pub hard_errors: u32,
    pub dmin: f32,
    /// Sincronismo del candidato (`sync8`).
    pub sync: f32,
}

/// Come si toglie un segnale decodificato dall'audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subtraction {
    /// `subtractft8` di JTDX: filtro passa-basso (ampiezza e fase variabili nel tempo) e ritardo affinato.
    Adaptive,
    /// Una sola ampiezza e fase per tutto il messaggio, senza affinare il ritardo (come faceva Aurora prima): solo per confronto.
    Constant,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    /// Soglia di sincronismo dei candidati (1,5 normale; con `low_threshold` si usa invece la rotazione di JTDX).
    pub min_sync: f32,
    /// Numero massimo di candidati per passata.
    pub max_candidates: usize,
    /// Passate (JTDX: 3, 6 o 9, secondo "i cicli": vedi [`Options::depth`]).
    pub passes: usize,
    /// Iterazioni della propagazione di credenze.
    pub bp_iterations: usize,
    /// Ordine dell'OSD (0 = nessuno; JTDX usa `ndeep = 3`).
    pub osd_order: usize,
    /// `lft8lowth`: `syncmin` a rotazione 1,225 / 1,5 / 1,1 sulle passate (1, 4, 7), (2, 5, 8) e (3, 6, 9).
    pub low_threshold: bool,
    /// `lft8subpass`: seconda lettura dei simboli (a tempo invertito) per ogni candidato.
    pub subpass: bool,
    /// Modo SWL di JTDX (`nswl`): finestra di ricerca piu' ampia (+-86 passi), soglie di sincronismo come con la soglia bassa, la terza passata
    /// di un ciclo da 3 sottrae e la lettura a tempo invertito si fa sempre. La quarta sottopassata di SWL non e' portata.
    pub swl: bool,
    /// Come si tolgono i segnali decodificati.
    pub subtraction: Subtraction,
    /// Sottrarre subito dopo ogni decodifica (come JTDX) invece che a fine passata.
    pub immediate_subtraction: bool,
    /// Filtri sui falsi di JTDX (sincronismo, `nharderrors + dmin`, testo): vedi [`filters`]. Spenti, valgono solo CRC, lettura del
    /// testo e `nsync >= 7`.
    pub filters: bool,
    /// Ritardo medio delle stazioni sentite (`avexdt`, secondi): la finestra di ricerca del ritardo si sposta di `avexdt * 25` passi.
    pub dt_center_s: f32,
    /// Analisi a priori per il proprio nominativo e il QSO (`None`: solo il tipo CQ, se acceso).
    pub ap: Option<ApOptions>,
    /// Tipo a priori 1, `CQ ??? ???`: non richiede di sapere niente.
    pub ap_cq: bool,
    /// Elenco completo di nominativi (come `ALLCALL7.TXT`) per `chkflscall`; con un elenco incompleto si scartano stazioni vere.
    pub call_book: Option<Arc<CallBook>>,
}

impl Default for Options {
    fn default() -> Self {
        Options::depth(2)
    }
}

impl Options {
    /// Le profondita' di JTDX: 1 = veloce (3 passate), 2 = normale (6 passate), 3 = profonda (9 passate, soglia bassa e seconda lettura).
    /// La 1 non prova nessun tipo a priori e usa OSD di ordine 2; dalla 2 in su OSD e' d'ordine 3 come `ndeep = 3` di JTDX.
    pub fn depth(depth: u8) -> Options {
        let depth = depth.clamp(1, 3);
        Options {
            f_low_hz: 200.0,
            f_high_hz: 4000.0,
            min_sync: 1.5,
            max_candidates: 300,
            passes: 3 * usize::from(depth),
            bp_iterations: 30,
            osd_order: if depth == 1 { 2 } else { 3 },
            low_threshold: depth == 3,
            subpass: depth == 3,
            swl: false,
            subtraction: Subtraction::Adaptive,
            immediate_subtraction: true,
            filters: true,
            dt_center_s: 0.0,
            ap: None,
            ap_cq: depth > 1,
            call_book: None,
        }
    }
}

/// Mette nella tabella degli hash i nominativi dati (per esempio quelli dei `*_ALL.TXT` di JTDX o dei decodificati degli ultimi giorni),
/// cosi' un messaggio che porta solo l'impronta di un nominativo si legge subito invece di `<...>` (`fillhash`/`save_hash_call`).
/// Ritorna quanti ne sono stati presi.
pub fn seed_call_hashes<'a, I: IntoIterator<Item = &'a str>>(hashes: &mut CallHashes, calls: I) -> usize {
    let before = hashes.len();
    for call in calls {
        hashes.save_call(call);
    }
    hashes.len().saturating_sub(before)
}

/// L'audio di JTDX e' a 16 bit (±32768): i campioni normalizzati a ±1 si riportano a quella scala, su cui sono tarate le soglie.
const SCALE: f32 = 32768.0;
/// Due decodifiche a meno di questa distanza in frequenza e tempo sono lo stesso segnale (per saltare i candidati su un segnale tolto).
const SAME_FREQ_HZ: f32 = 3.0;
const SAME_DT_S: f32 = 0.1;
/// Un messaggio uguale a meno di questa distanza in frequenza e' lo stesso messaggio (`allmessages` di `decode`).
const DUPE_FREQ_HZ: f32 = 45.0;
/// Entro questa distanza da un segnale appena tolto la FFT lunga va rifatta.
const REFFT_HZ: f32 = 50.0;

/// Decodifica i 15 secondi di audio di un periodo (12 kHz, campioni normalizzati a ±1).
pub fn decode_slot(samples: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    decode_slot_detailed(samples, options, hashes).into_iter().map(|d| d.decoded).collect()
}

/// Un segnale decodificato in attesa di essere sottratto a fine passata.
struct Pending {
    tones: [u8; NN],
    hit_freq: f32,
    hit_dt: f32,
    cd: Vec<Complex>,
    i0: i64,
}

/// Come [`decode_slot`], con le grandezze di accettazione di ogni messaggio.
pub fn decode_slot_detailed(samples: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<DecodedInfo> {
    let mut audio: Vec<f32> = samples.iter().take(SLOT_SAMPLES).map(|&x| x * SCALE).collect();
    audio.resize(SLOT_SAMPLES, 0.0);
    let mut found: Vec<DecodedInfo> = Vec::new();

    let npass = options.passes.max(1);
    let cycles = if npass >= 7 { 3 } else if npass >= 4 { 2 } else { 1 };
    let ap_options = options.ap.clone().unwrap_or_else(|| ApOptions::new(""));
    let ap_ctx = ApContext::new(&ap_options);
    let mut qso_decoded = false;
    let mut audio_before_shift: Option<Vec<f32>> = None;
    let jz = |center: f32, base: i32| base + (center * 25.0) as i32;

    for ipass in 1..=npass {
        let subtract_on = !(ipass > 5 || (ipass == 3 && npass == 3 && !options.swl));
        // syncmin: 1,5 oppure, con la soglia bassa, 1,225 / 1,5 / 1,1 a rotazione
        let syncmin = if options.low_threshold || options.swl {
            match (ipass - 1) % 3 {
                0 => 1.225,
                1 => 1.5,
                _ => 1.1,
            }
        } else {
            options.min_sync
        };
        // l'audio a mezzo campione: passata 4 verso destra, passata 7 (solo con tre cicli) verso sinistra dall'audio di prima
        if ipass == 4 {
            if npass >= 9 {
                audio_before_shift = Some(audio.clone());
            }
            for i in 0..SLOT_SAMPLES - 1 {
                audio[i] = (audio[i] + audio[i + 1]) / 2.0;
            }
        } else if let (7, Some(saved)) = (ipass, &audio_before_shift) {
            audio[0] = saved[0];
            for i in 1..SLOT_SAMPLES {
                audio[i] = (saved[i - 1] + saved[i]) / 2.0;
            }
        }

        let half_window = if options.swl { 86 } else { 62 };
        let window = (jz(options.dt_center_s, -half_window), jz(options.dt_center_s, half_window));
        let candidates = sync::find_candidates_swl(&audio, options.f_low_hz, options.f_high_hz, syncmin, options.max_candidates, Spectrum::for_pass(ipass), window, options.swl);
        if candidates.is_empty() {
            continue;
        }
        let mut ds = Downsampler::new(&audio);
        let mut stale: Vec<f32> = Vec::new();
        let mut pending: Vec<Pending> = Vec::new();

        for cand in candidates {
            let c = cand.candidate;
            if found.iter().any(|d| (d.decoded.freq_hz - c.freq_hz).abs() < SAME_FREQ_HZ && (d.decoded.dt_s - c.dt_s).abs() < SAME_DT_S) {
                continue;
            }
            if stale.iter().any(|&f| (f - c.freq_hz).abs() < REFFT_HZ) {
                ds = Downsampler::new(&audio);
                stale.clear();
            }
            let mut ctx = PassContext { ipass, cycles, options, ap: &ap_ctx, qso_decoded, hashes: &mut *hashes };
            let Some(hit) = try_candidate(&ds, cand, &mut ctx) else { continue };

            if subtract_on {
                let freq = hit.info.decoded.freq_hz;
                if options.immediate_subtraction {
                    remove_signal(&mut audio, options, &hit.tones, freq, hit.info.decoded.dt_s, &hit.cd, hit.i0);
                    stale.push(freq);
                } else {
                    pending.push(Pending { tones: hit.tones, hit_freq: freq, hit_dt: hit.info.decoded.dt_s, cd: hit.cd, i0: hit.i0 });
                }
            }
            let info = hit.info;
            if found.iter().any(|d| d.decoded.text == info.decoded.text && (d.decoded.freq_hz - info.decoded.freq_hz).abs() < DUPE_FREQ_HZ) {
                continue;
            }
            if let Some(a) = &options.ap {
                let root = format!("{} {} ", a.mycall, a.hiscall);
                if a.mycall.len() >= 3 && a.hiscall.len() > 3 && info.decoded.text.starts_with(&root) {
                    qso_decoded = true;
                }
            }
            found.push(info);
        }
        for p in pending {
            remove_signal(&mut audio, options, &p.tones, p.hit_freq, p.hit_dt, &p.cd, p.i0);
        }
    }
    found.sort_by(|a, b| a.decoded.freq_hz.total_cmp(&b.decoded.freq_hz));
    found
}

/// Toglie un segnale dall'audio nel modo scelto dalle opzioni.
fn remove_signal(audio: &mut [f32], options: &Options, tones: &[u8; NN], freq_hz: f32, dt_s: f32, cd: &[Complex], i0: i64) {
    let dt_abs = f64::from(dt_s) + 0.5;
    match options.subtraction {
        Subtraction::Constant => subtract::subtract_constant(audio, tones, freq_hz, dt_abs),
        Subtraction::Adaptive => {
            let reference = Reference::new(tones);
            let correction = subtract::refine_dt(cd, &reference, i0);
            subtract::subtract(audio, &reference, freq_hz, dt_abs + f64::from(correction));
        }
    }
}

/// Lo stato che `try_candidate` legge e (per gli hash) aggiorna.
struct PassContext<'a> {
    ipass: usize,

    cycles: usize,
    options: &'a Options,
    ap: &'a ApContext,
    qso_decoded: bool,
    hashes: &'a mut CallHashes,
}

/// Un candidato decodificato, con quello che serve per sottrarlo.
struct Hit {
    info: DecodedInfo,
    tones: [u8; NN],
    cd: Vec<Complex>,
    i0: i64,
}

/// Indicatori sul segnale che decidono quali tipi a priori valgono la pena (`lcqsignal`, `lmycsignal`, `lqso73`...).
#[derive(Default)]
struct Flags {
    cq_signal: bool,
    cq_ratio: f32,
    my_signal: bool,
    my_ratio: f32,
    my_matches: usize,
    qso_type3: bool,
    qso_73: bool,
    qso_rr73: bool,
    qso_rrr: bool,
    cq_dx: bool,
}

/// FFT di 256 punti per l'indicatore di CQ e la sua rotazione di 3,125 Hz (`ctwk256`).
fn cq_fft() -> &'static (Arc<dyn Fft<f32>>, Vec<Complex>) {
    static PLAN: OnceLock<(Arc<dyn Fft<f32>>, Vec<Complex>)> = OnceLock::new();
    PLAN.get_or_init(|| {
        let plan = FftPlanner::<f32>::new().plan_fft_forward(256);
        let dphi = 2.0 * std::f64::consts::PI * 3.125 * 0.005;
        let twist = (0..256).map(|i| Complex32::new((dphi * i as f64).cos() as f32, (dphi * i as f64).sin() as f32)).collect();
        (plan, twist)
    })
}

/// `lcqsignal` di `ft8b`: gli otto simboli dopo il primo Costas, dove un CQ ha sempre il tono 0, danno una riga spettrale unica.
fn cq_signal(cd: &[Complex], ibest: i64, rscq: f32) -> bool {
    let (plan, twist) = cq_fft();
    let mut buf: Vec<Complex> = (0..256).map(|i| sync::cd_at(cd, ibest + 224 + i as i64) * twist[i]).collect();
    plan.process(&mut buf);
    let mut s256 = [0.0f32; 9];
    for (s, b) in s256.iter_mut().zip(buf.iter()) {
        *s = b.norm();
    }
    let argmax = |v: &[f32]| -> usize {
        let mut best = 0;
        for (i, &x) in v.iter().enumerate() {
            if x > v[best] {
                best = i;
            }
        }
        best
    };
    let ip = argmax(&s256);
    let mut cq = ip == 4 || rscq > 3.1;
    if (!cq && ip == 3) || ip == 5 {
        let mut wide = [0.0f32; 27];
        wide[..9].copy_from_slice(&s256);
        for (m, w) in wide[9..].iter_mut().enumerate() {
            *w = buf[8 + m].norm();
        }
        let ip2 = argmax(&wide);
        if ip2 == 3 || ip2 == 5 {
            cq = true;
        }
    }
    cq
}

/// Gli indicatori che decidono i tipi a priori, dal segnale e dalle opzioni.
fn signal_flags(ctx: &PassContext, cd: &[Complex], ibest: i64, s8: &[[f32; 8]; NN], f1: f32) -> Flags {
    let o = &ctx.ap.opts;
    let rscq = ap::cq_score(s8);
    let cq_signal = cq_signal(cd, ibest, rscq);
    let mut fl = Flags { cq_signal, cq_ratio: ap::expected_ratio(s8, &ctx.ap.cq_tones), ..Flags::default() };
    if ctx.ap.my_ok {
        fl.my_ratio = ap::expected_ratio(s8, &ctx.ap.my_tones);
        let nmic = ap::tone_matches(s8, &ctx.ap.my_tones, 0);
        fl.my_matches = nmic;
        fl.my_signal = o.my_call_active && nmic > 2;
    }
    let dfqso = o.rx_freq_hz.map_or(f32::MAX, |r| (r - f1).abs());
    let near_tx = o.tx_freq_hz.is_some_and(|t| (t - f1).abs() < o.width_hz);
    if ctx.ap.my_ok && ctx.ap.his_ok {
        if !ctx.qso_decoded && (dfqso < o.width_hz || near_tx) && o.my_call_active {
            let n19 = ap::tone_matches(s8, &ctx.ap.qso_tones[..19], 0);
            let n22 = n19 + ap::tone_matches(s8, &ctx.ap.qso_tones[19..22], 19);
            fl.qso_type3 = n22 > 3;
        }
        if dfqso < o.width_hz && (o.qso_progress == 3 || o.qso_progress == 4) {
            // i simboli 24-58 (da 1) del messaggio atteso: 73, RR73 o RRR
            let count = |t: &[u8; 58]| ap::tone_matches(s8, &t[23..], 23);
            let (n73, nrr73, nrrr) = (count(&ctx.ap.tones_73), count(&ctx.ap.tones_rr73), count(&ctx.ap.tones_rrr));
            let best = n73.max(nrr73).max(nrrr);
            if best > 6 {
                if n73 == best {
                    fl.qso_73 = true;
                } else if nrr73 == best {
                    fl.qso_rr73 = true;
                } else {
                    fl.qso_rrr = true;
                }
            }
        }
    }
    if ctx.ap.his_ok {
        // simboli 10-19 di dati: il nominativo del DX (`idtone56(1,k)`, k = 10..19)
        let ndxt = ap::tone_matches(s8, &ctx.ap.qso_tones[9..19], 9);
        fl.cq_dx = fl.cq_signal && ndxt > 3;
    }
    fl
}

/// Quali tipi a priori valgono la pena su questo candidato (le condizioni `cycle` di `ft8b` per nominativi standard).
fn ap_allowed(aptype: u8, isubp2: usize, ctx: &PassContext, fl: &Flags, cq_only: bool, f1: f32) -> bool {
    let o = &ctx.ap.opts;
    let his_given = o.hiscall.trim().len() >= 3;
    // con un nominativo del DX non standard restano solo CQ e "MYCALL ??? ???"
    if his_given && !ctx.ap.his_ok && !matches!(aptype, 1 | 2) {
        return false;
    }
    if aptype == 1 && !ctx.options.ap_cq {
        return false;
    }
    let far = o.rx_freq_hz.is_none_or(|r| (r - f1).abs() > o.width_hz) && o.tx_freq_hz.is_none_or(|t| (t - f1).abs() > o.width_hz);
    let qso_type = (3..31).contains(&aptype);
    if ctx.qso_decoded && qso_type {
        return false;
    }
    let lowth = ctx.options.low_threshold || ctx.options.subpass;
    if aptype == 2 {
        if !o.my_call_active || cq_only {
            return false;
        }
        // in un QSO si prova "MYCALL ??? ???" solo se almeno due dei primi nove simboli hanno il tono del proprio nominativo
        if o.qso_progress != 0 && fl.my_matches < 2 {
            return false;
        }
    }
    if !o.idle && aptype > 30 {
        return false;
    }
    if o.idle && qso_type {
        return false;
    }
    let no_his = !his_given;
    if aptype > 2 && no_his {
        return false;
    }
    if qso_type && far {
        return false;
    }
    match aptype {
        3 if !fl.qso_type3 => return false,
        4 if !fl.qso_rrr => return false,
        5 if !fl.qso_73 => return false,
        6 if !fl.qso_rr73 => return false,
        _ => {}
    }
    if aptype > 30 {
        if !o.dx_search || (!o.wide_dx_search && far) {
            return false;
        }
        if aptype == 31 && !fl.cq_dx {
            return false;
        }
        if aptype > 31 && cq_only {
            return false;
        }
        if aptype == 35 && !fl.qso_73 {
            return false;
        }
        if aptype == 36 && !fl.qso_rr73 {
            return false;
        }
    }
    // rapporti segnale/rumore dei simboli attesi, valutati alla prima lettura di ogni tipo
    match (aptype, isubp2) {
        (1, 20) if fl.cq_ratio < 1.0 && !fl.cq_signal => return false,
        (1, 21) if fl.cq_ratio < if lowth { 1.2 } else { 1.3 } && !fl.cq_signal => return false,
        (2, 17) if fl.my_ratio < 1.0 && !fl.my_signal => return false,
        (2, 18) if lowth && fl.my_ratio < 1.2 && !fl.my_signal => return false,
        (3, 5) if fl.my_ratio < 1.0 => return false,
        (3, 6) if fl.my_ratio < 1.2 => return false,
        _ => {}
    }
    true
}

/// Allinea un candidato, ne calcola le probabilita' dei bit e prova a decodificarlo (`ft8b`).
fn try_candidate(ds: &Downsampler, cand: PassCandidate, ctx: &mut PassContext) -> Option<Hit> {
    let options = ctx.options;
    let ipass = ctx.ipass;
    let c = cand.candidate;
    // la discesa a 200 Hz, con i ritocchi dei bordi per i segnali deboli (`lhighsens`)
    let highsens = c.sync < 1.9 || (matches!(ipass, 2 | 4 | 6) && c.sync < 3.15);
    let mut cd = ds.run(c.freq_hz, highsens);
    let refined: Candidate = sync::refine_ex(&cd, c, SyncKind::for_pass(ipass), cand.cq);
    sync::shift_frequency(&mut cd, -(refined.freq_hz - c.freq_hz));
    let ibest = (f64::from(refined.dt_s + 0.5) * 200.0).round() as i64;

    // lettura a tempo invertito: passate 5 e 7 con piu' cicli, la 2 con uno solo
    let reverse = if ctx.cycles < 2 { ipass == 2 } else { ipass == 5 || ipass == 7 };
    let first = sync::bit_metrics_ex(&cd, refined.dt_s, MetricsMode { reverse, second: false })?;

    // controlli di sincronismo prima di LDPC
    let o = &ctx.ap.opts;
    let on_rx = o.rx_freq_hz.is_some_and(|r| (r - refined.freq_hz).abs() < 2.0) && !o.idle;
    let screen = if options.filters {
        filters::sync_screen(&first.s8, refined.dt_s, cand.cq, !on_rx)
    } else if first.nsync >= 7 {
        filters::SyncScreen { pass: true, cq_only: false, skip_plain: false }
    } else {
        filters::SyncScreen { pass: false, cq_only: false, skip_plain: false }
    };
    if !screen.pass {
        return None;
    }

    let flags = signal_flags(ctx, &cd, ibest, &first.s8, refined.freq_hz);
    let nweak = if options.subpass || options.swl { 2 } else { 1 };
    let progress = usize::from(o.qso_progress.min(5));

    for isubp1 in 1..=nweak {
        let metrics = if isubp1 == 1 { first.clone() } else { sync::bit_metrics_ex(&cd, refined.dt_s, MetricsMode { reverse, second: true })? };
        let [llra, llrb, llrc, llrd] = metrics.llr;
        let apmag = llra.iter().fold(0.0f32, |m, v| m.max(v.abs())) * 1.01;

        for isubp2 in 1..=31usize {
            let (llrz, mask, aptype);
            if isubp2 < 5 {
                if screen.cq_only || screen.skip_plain || isubp2 == 4 {
                    continue;
                }
                llrz = match isubp2 {
                    1 => {
                        if ipass == 1 || isubp1 > 1 {
                            llrd
                        } else {
                            llra
                        }
                    }
                    2 => {
                        if isubp1 > 1 {
                            llra
                        } else {
                            llrb
                        }
                    }
                    _ => llrc,
                };
                mask = [false; 174];
                aptype = 0u8;
            } else {
                let t = NAPTYPES[progress][isubp2 - 5];
                if t == 0 || !ap_allowed(t, isubp2, ctx, &flags, screen.cq_only, refined.freq_hz) {
                    continue;
                }
                let base = [llra, llrb, llrc][ap::llr_index(isubp2, t)];
                let Some((z, m)) = ctx.ap.apply(t, &base, apmag) else { continue };
                llrz = z;
                mask = m;
                aptype = t;
            }

            // BP, e se non basta OSD
            let quality = match ldpc::bp_decode_masked(&llrz, &mask, options.bp_iterations) {
                Some(q) => q,
                None if options.osd_order > 0 => match ldpc::osd_decode_masked(&llrz, &mask, options.osd_order) {
                    Some(q) => q,
                    None => continue,
                },
                None => continue,
            };
            if options.filters {
                if !filters::accept_decode(quality.hard_errors, quality.dmin, isubp2 > 2) {
                    continue;
                }
            } else if quality.hard_errors > 45 {
                continue;
            }

            let mut bits77 = [0u8; 77];
            bits77.copy_from_slice(&quality.decoded.message[..77]);
            let (i3, n3) = message::message_type(&bits77);
            if i3 > 4 || (i3 == 0 && n3 > 5) {
                continue;
            }
            // il testo si legge su una copia degli hash: se il messaggio poi e' scartato la tabella non si sporca
            let mut trial = ctx.hashes.clone();
            let Ok(text) = message::unpack(&bits77, &mut trial) else { continue };
            if aptype == 1 && text.starts_with("CQ DE AA00") {
                continue;
            }
            let tones = modem::tones_from_codeword(&quality.decoded.codeword);
            let xsnr = filters::jtdx_snr(&metrics.s8, &tones, aptype != 0);
            if options.filters {
                let qual = 1.0 - (quality.hard_errors as f32 + quality.dmin) / 60.0;
                let check = MessageCheck { text: &text, i3, n3, ap_type: aptype, qual, snr: xsnr, dt_s: refined.dt_s };
                if filters::reject_message(&check, &o.mycall, options.call_book.as_deref()) {
                    return None;
                }
            }
            *ctx.hashes = trial;

            return Some(Hit {
                info: DecodedInfo {
                    decoded: Decoded { freq_hz: refined.freq_hz, dt_s: refined.dt_s, snr_db: xsnr.round() as i32, text, hint: aptype != 0 },
                    pass: ipass,
                    ap_type: aptype,
                    hard_errors: quality.hard_errors,
                    dmin: quality.dmin,
                    sync: c.sync,
                },
                tones,
                cd,
                i0: ibest,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use crate::SAMPLE_RATE;

    use super::*;

    /// Generatore pseudocasuale deterministico (nessuna dipendenza): uniforme in (0,1) e gaussiana (Box-Muller).
    struct Rng(u64);
    impl Rng {
        fn uniform(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f64 {
            (-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()
        }
    }

    /// Un periodo con rumore bianco e i messaggi dati (testo, frequenza, dt, SNR in 2500 Hz).
    fn slot(signals: &[(&str, f32, f32, f32)], seed: u64) -> Vec<f32> {
        let mut rng = Rng(seed);
        let sigma = 0.05f64; // ampiezza del rumore (normalizzata)
        let mut audio: Vec<f32> = (0..SLOT_SAMPLES).map(|_| (sigma * rng.gauss()) as f32).collect();
        let mut hashes = CallHashes::default();
        for &(text, freq, dt, snr) in signals {
            let wave = modem::encode_message(text, freq, &mut hashes).expect("messaggio valido");
            // potenza del rumore in 2500 Hz = sigma^2 * 2500 / 6000; l'onda ha picco a, potenza a^2 / 2
            let noise_2500 = sigma * sigma * 2500.0 / 6000.0;
            let amplitude = (2.0 * noise_2500 * 10f64.powf(f64::from(snr) / 10.0)).sqrt() as f32;
            let start = ((f64::from(dt) + 0.5) * SAMPLE_RATE as f64).round() as usize;
            for (i, &w) in wave.iter().enumerate() {
                if let Some(a) = audio.get_mut(start + i) {
                    *a += amplitude * w;
                }
            }
        }
        audio
    }

    fn decode(audio: &[f32]) -> Vec<Decoded> {
        decode_slot(audio, &Options::default(), &mut CallHashes::default())
    }

    #[test]
    fn a_clean_message_is_found_with_frequency_time_and_snr() {
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.3, -8.0)], 1);
        let out = decode(&audio);
        assert_eq!(out.len(), 1, "{out:?}");
        let d = &out[0];
        assert_eq!(d.text, "CQ K1ABC FN42");
        assert!((d.freq_hz - 1500.0).abs() < 1.0, "{}", d.freq_hz);
        assert!((d.dt_s - 0.3).abs() < 0.05, "{}", d.dt_s);
        assert!((d.snr_db - (-8)).abs() <= 3, "snr {}", d.snr_db);
    }

    #[test]
    fn several_signals_are_all_found() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.1, -6.0), ("IU3ABC K1ABC -15", 1500.0, 0.3, -9.0), ("K1ABC IU3ABC RR73", 2200.0, -0.2, -4.0)], 7);
        let texts: Vec<String> = decode(&audio).into_iter().map(|d| d.text).collect();
        for expected in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
            assert!(texts.iter().any(|t| t == expected), "manca {expected}: {texts:?}");
        }
        assert_eq!(texts.len(), 3, "nessun falso: {texts:?}");
    }

    #[test]
    fn swl_mode_finds_the_same_signals_and_invents_nothing() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.1, -6.0), ("IU3ABC K1ABC -15", 1500.0, 0.3, -9.0), ("K1ABC IU3ABC RR73", 2200.0, -0.2, -4.0)], 7);
        let swl = Options { swl: true, ..Options::default() };
        let texts: Vec<String> = decode_slot(&audio, &swl, &mut CallHashes::default()).into_iter().map(|d| d.text).collect();
        for expected in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
            assert!(texts.iter().any(|t| t == expected), "manca {expected}: {texts:?}");
        }
        assert_eq!(texts.len(), 3, "nessun falso: {texts:?}");
        for seed in [3, 4, 5] {
            let out = decode_slot(&slot(&[], seed), &swl, &mut CallHashes::default());
            assert!(out.is_empty(), "solo rumore, seme {seed}: {out:?}");
        }
        assert!(!Options::default().swl && !Options::depth(3).swl);
    }

    #[test]
    fn a_weak_signal_under_a_strong_one_appears_after_subtraction() {
        // i due toni sono a 12 Hz di distanza: il debole e' coperto dai fianchi del forte finche' questo non viene tolto
        let audio = slot(&[("CQ DX IU3ABC IO91", 1500.0, 0.2, 5.0), ("K1ABC W9XYZ -12", 1512.0, 0.4, -9.0)], 11);
        let texts: Vec<String> = decode(&audio).into_iter().map(|d| d.text).collect();
        assert!(texts.iter().any(|t| t == "CQ DX IU3ABC IO91"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "K1ABC W9XYZ -12"), "{texts:?}");
    }

    #[test]
    fn noise_alone_decodes_nothing() {
        for seed in [3, 4, 5] {
            let out = decode(&slot(&[], seed));
            assert!(out.is_empty(), "seme {seed}: {out:?}");
        }
    }

    #[test]
    fn a_short_or_empty_buffer_does_not_panic() {
        assert!(decode(&[]).is_empty());
        assert!(decode(&vec![0.0; 1000]).is_empty());
        assert!(decode(&vec![0.0; SLOT_SAMPLES]).is_empty());
    }

    #[test]
    fn le_profondita_scelgono_le_passate_e_clampano() {
        assert_eq!(Options::depth(1).passes, 3);
        assert_eq!(Options::depth(2).passes, 6);
        assert_eq!(Options::depth(3).passes, 9);
        assert_eq!(Options::depth(0).passes, 3);
        assert_eq!(Options::depth(9).passes, 9);
        assert!(Options::depth(3).low_threshold && !Options::depth(2).low_threshold);
    }

    #[test]
    fn il_segnale_e_il_nominativo_giusto_servono_all_analisi_a_priori_ma_uno_sbagliato_non_inventa_messaggi() {
        // un messaggio per noi a -22 dB con la profondita' normale: con il nominativo giusto l'AP lo recupera piu' spesso
        // di quanto lo faccia senza; con uno sbagliato non deve uscire niente che non sia nel segnale
        let mut con = 0;
        let mut senza = 0;
        let mut sbagliato = 0;
        for seed in 0..6u64 {
            let audio = slot(&[("IU3ABC K1ABC -15", 1200.0 + 100.0 * seed as f32, 0.2, -22.0)], 40 + seed);
            let ap = Options { ap: Some(ApOptions::new("IU3ABC")), ..Options::default() };
            con += decode_slot(&audio, &ap, &mut CallHashes::default()).iter().filter(|d| d.text == "IU3ABC K1ABC -15").count();
            senza += decode(&audio).iter().filter(|d| d.text == "IU3ABC K1ABC -15").count();
            let wrong = Options { ap: Some(ApOptions::new("W1AW")), ..Options::default() };
            let out = decode_slot(&audio, &wrong, &mut CallHashes::default());
            sbagliato += out.iter().filter(|d| d.text.starts_with("W1AW ")).count();
        }
        assert!(con >= senza, "con {con} senza {senza}");
        assert_eq!(sbagliato, 0, "un nominativo sbagliato non deve produrre messaggi per lui");
    }

    #[test]
    fn la_semina_degli_hash_scioglie_i_nominativi() {
        let mut hashes = CallHashes::default();
        let added = seed_call_hashes(&mut hashes, ["K1ABC", "IU3ABC", "K1ABC", "AB"]);
        assert_eq!(added, 2);
        assert_eq!(hashes.len(), 2);
    }
}
