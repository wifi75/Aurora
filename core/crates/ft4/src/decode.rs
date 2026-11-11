//! La decodifica di un periodo FT4 intero (`ft4_decode` di JTDX 2.2.159, `lib/ft4_decode.f90`, e `subtractft4`): piu' passate, sottrazione dei
//! segnali gia' decodificati, elenco senza doppioni, stima dell'SNR.
//!
//! Ogni passata cerca i candidati ([`sync::find_candidates`]), per ognuno ne cerca ritardo e frequenza in tre finestre di tempo
//! ([`sync::search`]), calcola le tre serie di LLR e prova la propagazione di credenze e poi l'OSD su ciascuna; un messaggio vale solo se il
//! CRC (calcolato sul messaggio mescolato) torna e il testo si legge. I segnali decodificati si tolgono dall'audio con una stima di ampiezza e
//! fase (`subtractft4`) e la passata dopo trova quelli che erano nascosti sotto. Le passate continuano finche' l'ultima ha trovato qualcosa,
//! fino a [`Options::passes`] (3 in JTDX).
//!
//! # Convenzioni
//!
//! - `freq_hz` e' la frequenza del tono 0, a passi di 1 Hz (la correzione fine di JTDX e' intera); `dt_s` e' lo scarto rispetto a 0,5 s con
//!   passo di 1,5 ms; `snr_db` e' la stima di JTDX: dall'altezza del picco nello spettro medio, con il pavimento a -21 dB.
//! - L'elenco e' ordinato per frequenza (JTDX li emette nell'ordine di decodifica).
//!
//! # Differenze rispetto al Fortran
//!
//! - Analisi a priori (AP) con i tipi di FT8 ([`ft8::ap`]): se [`Options::ap`] (nominativi e QSO) o [`Options::ap_cq`] sono accesi, dopo le tre
//!   letture senza AP di ogni candidato si provano i tipi del punto del QSO. I bit noti stanno nel messaggio mescolato con [`RVEC`], quindi la
//!   maschera si porta nel dominio mescolato (segno invertito dove `RVEC` e' 1). Non ci sono i controlli sui toni attesi di `ft8b`: li
//!   sostituisce un limite ai bit dubbi tra quelli non noti, piu' stretto quanti piu' bit sono noti. Non sono portati i nominativi non standard.
//! - Filtri sui falsi: come in WSJT-X 3.3 (`ft4_decode.f90:443-481`), un messaggio senza AP vale se il CRC torna e il testo si legge, senza
//!   altri controlli (niente limite ai bit dubbi, niente controllo del locatore ne' dell'elenco dei nominativi). Solo i messaggi trovati con
//!   l'AP passano per i controlli di Aurora: il limite ai bit dubbi ([`ap_error_limit`]), [`filters::accept_decode`], il locatore
//!   ([`filters::grid_is_false`]) e [`filters::reject_message`] (`chkfalse8` di JTDX, con `chkflscall` se c'e' [`Options::call_book`]).
//!   WSJT-X non li ha nemmeno per l'AP (marca solo con `?` quelli con `qual < 0,17`, `decoder_callbacks.f90:755`), ma e' con i bit fissati
//!   in partenza che il rumore produce messaggi plausibili: si tengono come cintura. Niente nascondimento di messaggi di prova o di
//!   telemetria (`lhidetest`, `lhidetelemetry`), ne' il filtro di frequenza dell'interfaccia (`lfilter`: qui `f_low_hz`/`f_high_hz` alla
//!   ricerca). Non c'e' la media dello scarto di tempo (`avexdt` sempre 0).
//! - Messaggio di tipo 0.1 (DXpedition): JTDX lo spezza in due righe; qui esce com'e' dal decodificatore dei messaggi.
//! - Un controllo opzionale sui bit dubbi anche senza AP ([`Options::max_hard_errors`], spento di base come in WSJT-X e JTDX): un messaggio
//!   vero ha pochi bit diversi dalle decisioni dure del segnale, uno falso (CRC a caso) circa la meta'.
//! - Ricerca dei candidati con i valori di WSJT-X 3.3: `syncmin = 1,18` (`ft4_decode.f90:210`), al piu' 200 candidati (`MAXCAND`,
//!   `ft4_decode.f90:49`), i candidati entro 20 Hz dalla frequenza di ricezione per primi ([`Options::qso_freq_hz`], `nfqso`).
//! - Un limite di tempo opzionale ([`Options::deadline`]): passato, non si prova un altro candidato. In WSJT-X non c'e'.
//! - L'OSD e' quello di `ft8::ldpc` (stesso algoritmo di `osd4_174_91`, il codice e' identico).

use std::sync::Arc;

use ft8::ap::{ApContext, ApOptions, NAPTYPES};
use ft8::filters::{self, CallBook, GridCheck, MessageCheck};
use ft8::ldpc;
use ft8::message::{self, CallHashes};
use rustfft::{Fft, FftPlanner};

pub use ft8::decode::Decoded;

use crate::modem;
use crate::sync::{self, Candidate, Complex, Downsampler, SyncTables, DOWN_RATE, NDMAX, NSS};
use crate::{NMAX, NN, NSPS, RVEC, SAMPLE_RATE};

/// L'audio di JTDX e' a 16 bit (±32768): i campioni normalizzati a ±1 si riportano a quella scala.
const SCALE: f32 = 32768.0;
/// Sincronismo minimo dopo la ricerca fine (`if(smax.lt.1.2) cycle`, WSJT-X 3.3 `ft4_decode.f90:294`).
const MIN_ALIGN_SYNC: f32 = 1.2;
/// Bit di sincronismo giusti, su 32, sotto i quali il candidato si scarta (`nsync_qual.lt.20`, `ft4_decode.f90:322`).
const MIN_SYNC_BITS: usize = 20;

#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    /// Frequenza di ricezione (`nfqso`): i candidati entro 20 Hz si provano per primi (`getcandidates4.f90:70-81`).
    pub qso_freq_hz: Option<f32>,
    /// Soglia di altezza dei picchi nello spettro medio (`syncmin = 1.18`, WSJT-X 3.3 `ft4_decode.f90:210`).
    pub min_sync: f32,
    /// Numero massimo di candidati per passata (`MAXCAND = 200`, `ft4_decode.f90:49`).
    pub max_candidates: usize,
    /// Passate con sottrazione (JTDX ne fa tre; 1 = una sola, senza sottrazione).
    pub passes: usize,
    /// Toglie dall'audio i segnali decodificati (`dosubtract`).
    pub subtract: bool,
    /// Iterazioni della propagazione di credenze (40).
    pub bp_iterations: usize,
    /// Ordine dell'OSD (0 = nessuno; JTDX usa 3 a profondita' 3).
    pub osd_order: usize,
    /// Finestre di ricerca del ritardo piu' larghe (`swl`, ascolto): da -1,2 a +1,7 s invece che da -1,0 a +1,4 s.
    pub swl: bool,
    /// Scarta i messaggi senza AP con piu' di questi bit diversi dalle decisioni dure del segnale (`None` = nessun controllo, come WSJT-X:
    /// `ft4_decode.f90:443-481` accetta ogni messaggio con CRC giusto che si legge).
    pub max_hard_errors: Option<usize>,
    /// Compensazione AGC (pulsante AGCc): la stessa di FT8 ([`ft8::sync::agc_compensate_len`]) sul tratto del periodo che si decodifica.
    pub agc: bool,
    /// Analisi a priori per il proprio nominativo e il QSO (`None`: solo il tipo CQ, se acceso).
    pub ap: Option<ApOptions>,
    /// Tipo a priori 1, `CQ ??? ???`: non richiede di sapere niente.
    pub ap_cq: bool,
    /// Elenco dei nominativi veri (come `ALLCALL7.TXT`) per `chkflscall`: solo sui messaggi trovati con l'AP.
    pub call_book: Option<Arc<CallBook>>,
    /// Controllo del locatore sul paese del nominativo (vedi [`filters::grid_is_false`]): solo sui messaggi trovati con l'AP.
    pub grid_check: Option<GridCheck>,
    /// Oltre questo istante non si prova un altro candidato (i messaggi gia' trovati restano): un PC lento non deve restare indietro.
    pub deadline: Option<std::time::Instant>,
}

impl Default for Options {
    /// Profondita' 3 di JTDX: tre passate con sottrazione, BP e OSD.
    fn default() -> Self {
        Options::depth(3)
    }
}

impl Options {
    /// Le profondita' di JTDX (`ndepth`): 1 = una passata senza sottrazione ne' OSD; 2 = tre passate solo con BP; 3 = tre passate con BP e OSD.
    pub fn depth(ndepth: u8) -> Self {
        let mut o = Options {
            f_low_hz: 200.0,
            f_high_hz: 4000.0,
            qso_freq_hz: None,
            min_sync: 1.18,
            max_candidates: 200,
            passes: 3,
            subtract: true,
            bp_iterations: 40,
            osd_order: 3,
            swl: false,
            max_hard_errors: None,
            agc: false,
            ap: None,
            ap_cq: false,
            call_book: None,
            grid_check: None,
            deadline: None,
        };
        match ndepth {
            0 | 1 => {
                o.passes = 1;
                o.subtract = false;
                o.osd_order = 0;
            }
            2 => o.osd_order = 0,
            _ => {}
        }
        o
    }
}

/// Tetto del limite ai bit dubbi dei messaggi trovati con l'AP ([`ap_error_limit`]): un vero ne ha pochi, uno casuale circa la meta' (87).
const MAX_HARD_ERRORS: usize = 45;

/// Decodifica i 7,5 secondi di audio di un periodo (12 kHz, campioni normalizzati a ±1; si usano i primi 73728 campioni).
pub fn decode_slot(samples: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    let mut audio: Vec<f32> = samples.iter().take(NMAX).map(|&x| x * SCALE).collect();
    audio.resize(NMAX, 0.0);
    // AGCc: una volta sola prima delle passate; con il rumore uniforme non cambia niente
    if options.agc {
        ft8::sync::agc_compensate_len(&mut audio, options.f_low_hz, options.f_high_hz, NMAX);
    }
    let tables = SyncTables::new();
    let subtractor = Subtractor::new();
    let ap = (options.ap.is_some() || options.ap_cq).then(|| ApContext::new(&options.ap.clone().unwrap_or_else(|| ApOptions::new(""))));
    let mut found: Vec<Decoded> = Vec::new();
    let mut before_pass = 0;

    for pass in 0..options.passes.max(1) {
        // dalla seconda passata in poi si prosegue solo se la precedente ha trovato qualcosa
        if pass > 0 && found.len() == before_pass {
            break;
        }
        before_pass = found.len();

        let candidates = sync::find_candidates(&audio, options.f_low_hz, options.f_high_hz, options.min_sync, options.max_candidates, options.qso_freq_hz);
        if candidates.is_empty() {
            break;
        }
        let ds = Downsampler::new(&audio);
        for cand in candidates {
            if options.deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                break;
            }
            let Some(hit) = try_candidate(&ds, &tables, cand, options, ap.as_ref(), hashes) else { continue };
            // come nel Fortran la sottrazione precede il controllo dei doppioni
            if options.subtract {
                subtractor.subtract(&mut audio, &hit.tones, hit.decoded.freq_hz, hit.start);
            }
            if found.iter().any(|d| d.text == hit.decoded.text) {
                continue;
            }
            found.push(hit.decoded);
        }
    }
    found.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
    found
}

struct Hit {
    decoded: Decoded,
    tones: [u8; NN],
    /// Campione (a 666,67 Hz) del primo simbolo di Costas.
    start: i64,
}

/// Cerca ritardo e frequenza di un candidato nelle tre finestre di tempo e prova a decodificarlo.
fn try_candidate(ds: &Downsampler, tables: &SyncTables, cand: Candidate, options: &Options, ap: Option<&ApContext>, hashes: &mut CallHashes) -> Option<Hit> {
    let f0 = cand.freq_hz;
    let mut cd2 = ds.run(f0);
    sync::normalize(&mut cd2, NDMAX as f32);

    let mut smax1 = 0.0f32;
    for (iseg, &(ibmin, ibmax)) in sync::dt_windows(options.swl).iter().enumerate() {
        let al = sync::search(&cd2, tables, ibmin, ibmax);
        if iseg == 0 {
            smax1 = al.sync;
        }
        if al.sync < MIN_ALIGN_SYNC {
            continue;
        }
        if iseg > 0 && al.sync < smax1 {
            continue;
        }
        let f1 = f0 + al.df_hz as f32;
        if f1 <= 10.0 || f1 >= 4990.0 {
            continue;
        }
        let mut cb = ds.run(f1);
        sync::normalize(&mut cb, (NSS * NN) as f32);
        let cd = sync::frame(&cb, al.start);
        let Some(metrics) = sync::bit_metrics(&cd) else { continue };
        if metrics.sync_bits < MIN_SYNC_BITS {
            continue;
        }
        let snr = sync_to_snr(cand.sync);
        let dt = al.start as f32 / DOWN_RATE - 0.5;
        // prima le tre letture senza AP; se nessuna regge, i tipi a priori
        let found = decode_llrs(&metrics, options, hashes).or_else(|| ap.and_then(|ctx| decode_ap(&metrics, options, ctx, f1, hashes)));
        let Some(found) = found else { continue };
        if rejected(&found, options, snr, dt) {
            continue;
        }
        let tones = modem::tones_from_codeword(&found.codeword);
        return Some(Hit { decoded: Decoded { freq_hz: f1, dt_s: dt, snr_db: snr, text: found.text, hint: found.ap_type != 0 }, tones, start: al.start });
    }
    None
}

/// Un messaggio decodificato con quello che serve a giudicarlo.
struct Found {
    text: String,
    codeword: [u8; 174],
    i3: u8,
    n3: u8,
    /// Tipo a priori con cui e' stato trovato (0 = nessuno).
    ap_type: u8,
    hard_errors: u32,
    dmin: f32,
}

/// I filtri sui falsi, solo per i messaggi trovati con l'AP (che lavorano su bit decisi in partenza). Senza AP WSJT-X 3.3 accetta il messaggio
/// appena il CRC torna e `unpack77` lo legge (`ft4_decode.f90:443-481`), senza altri controlli: qui lo stesso.
fn rejected(found: &Found, options: &Options, snr: i32, dt_s: f32) -> bool {
    if found.ap_type == 0 {
        return false;
    }
    if filters::grid_is_false(&found.text, snr as f32, found.ap_type, options.grid_check.as_ref()) {
        return true;
    }
    let qual = 1.0 - (found.hard_errors as f32 + found.dmin) / 60.0;
    let check = MessageCheck { text: &found.text, i3: found.i3, n3: found.n3, ap_type: found.ap_type, qual, snr: snr as f32, dt_s };
    let my_call = options.ap.as_ref().map_or("", |a| a.mycall.as_str());
    filters::reject_message(&check, my_call, options.call_book.as_deref())
}

/// I tipi a priori da provare per un candidato a `freq_hz`, nell'ordine di `NAPTYPES` per il punto del QSO (ogni tipo una volta sola: in FT8 si
/// ripete per le sottoletture). Le condizioni sono quelle di `ap_allowed` di FT8 che non riguardano i toni attesi.
fn ap_types(options: &Options, ctx: &ApContext, freq_hz: f32) -> Vec<u8> {
    let o = &ctx.opts;
    let his_given = o.hiscall.trim().len() >= 3;
    let far = o.rx_freq_hz.is_none_or(|r| (r - freq_hz).abs() > o.width_hz) && o.tx_freq_hz.is_none_or(|t| (t - freq_hz).abs() > o.width_hz);
    let mut types: Vec<u8> = Vec::new();
    for &t in &NAPTYPES[usize::from(o.qso_progress).min(5)] {
        if t == 0 || types.contains(&t) {
            continue;
        }
        let qso_type = (3..31).contains(&t);
        let allowed = match t {
            1 => options.ap_cq,
            // `MYCALL ??? ???`: in ricezione a vuoto un messaggio "per me" trovato nel rumore e' quasi sempre un falso
            2 => o.my_call_active && ctx.my_ok,
            _ if !his_given => false,
            // con il nominativo del DX non standard restano solo CQ e `MYCALL ??? ???`
            _ if !ctx.his_ok => false,
            3..=30 => !o.idle && !far && ctx.my_ok && o.my_call_active,
            _ => o.idle && o.dx_search && (o.wide_dx_search || !far),
        };
        if allowed && !(qso_type && o.idle) {
            types.push(t);
        }
    }
    types
}

/// Limite ai bit dubbi tra quelli non noti: un messaggio vero ne ha pochi, un'estrazione a caso circa la meta'. Piu' bit sono noti, meno
/// bit liberi ci sono e piu' facile e' che il rumore combaci per caso, quindi il limite scende (45 al massimo, come senza AP).
fn ap_error_limit(mask: &[bool; 174]) -> u32 {
    let free = mask.iter().filter(|&&m| !m).count();
    ((free as f32 * 0.35) as u32).min(MAX_HARD_ERRORS as u32)
}

/// L'analisi a priori su un candidato: per ogni tipo e per ognuna delle tre serie di LLR la propagazione di credenze con i bit noti fissati
/// e, sulla prima serie, l'OSD. Il primo messaggio valido vince.
fn decode_ap(metrics: &sync::Metrics, options: &Options, ctx: &ApContext, freq_hz: f32, hashes: &mut CallHashes) -> Option<Found> {
    for t in ap_types(options, ctx, freq_hz) {
        for (k, base) in metrics.llr.iter().enumerate() {
            let apmag = base.iter().fold(0.0f32, |m, v| m.max(v.abs())) * 1.01;
            let Some((mut z, mask)) = ctx.apply(t, base, apmag) else { break };
            // i bit noti sono quelli del messaggio; sul filo c'e' il messaggio mescolato con RVEC
            for (i, &r) in RVEC.iter().enumerate() {
                if mask[i] && r == 1 {
                    z[i] = -z[i];
                }
            }
            let quality = match ldpc::bp_decode_masked(&z, &mask, options.bp_iterations) {
                Some(q) => Some(q),
                None if k == 0 && options.osd_order > 0 => ldpc::osd_decode_masked(&z, &mask, options.osd_order),
                None => None,
            };
            let Some(q) = quality else { continue };
            if q.hard_errors > ap_error_limit(&mask) || !filters::accept_decode(q.hard_errors, q.dmin, false) {
                continue;
            }
            let mut scrambled = [0u8; 77];
            scrambled.copy_from_slice(&q.decoded.message[..77]);
            if scrambled.iter().all(|&b| b == 0) {
                continue;
            }
            let bits = modem::scramble(&scrambled);
            let (i3, n3) = message::message_type(&bits);
            if i3 > 4 || (i3 == 0 && n3 > 5) {
                continue;
            }
            let Ok(text) = message::unpack(&bits, hashes) else { continue };
            let text = if (i3, n3) == (0, 0) { text } else { strip_braces(&text) };
            if text.trim().is_empty() || (t == 1 && text.starts_with("CQ DE AA00")) {
                continue;
            }
            return Some(Found { text, codeword: q.decoded.codeword, i3, n3, ap_type: t, hard_errors: q.hard_errors, dmin: q.dmin });
        }
    }
    None
}

/// Le tre serie di LLR, nell'ordine: per ognuna la propagazione di credenze e, se non basta, l'OSD. Il primo messaggio valido vince.
fn decode_llrs(metrics: &sync::Metrics, options: &Options, hashes: &mut CallHashes) -> Option<Found> {
    for llr in &metrics.llr {
        let mut code = ldpc::bp_decode(llr, options.bp_iterations);
        if code.is_none() && options.osd_order > 0 {
            code = ldpc::osd_decode(llr, options.osd_order);
        }
        let Some(code) = code else { continue };
        if code.message[..77].iter().all(|&b| b == 0) {
            continue;
        }
        if let Some(limit) = options.max_hard_errors {
            let hard_errors = code.codeword.iter().zip(llr.iter()).filter(|&(&bit, &l)| (l > 0.0) != (bit == 1)).count();
            if hard_errors > limit {
                continue;
            }
        }
        let mut scrambled = [0u8; 77];
        scrambled.copy_from_slice(&code.message[..77]);
        let bits = modem::scramble(&scrambled);
        let Ok(text) = message::unpack(&bits, hashes) else { continue };
        let (i3, n3) = message::message_type(&bits);
        let text = if (i3, n3) == (0, 0) { text } else { strip_braces(&text) };
        if text.trim().is_empty() {
            continue;
        }
        let hard_errors = code.codeword.iter().zip(llr.iter()).filter(|&(&bit, &l)| (l > 0.0) != (bit == 1)).count() as u32;
        return Some(Found { text, codeword: code.codeword, i3, n3, ap_type: 0, hard_errors, dmin: 0.0 });
    }
    None
}

/// Toglie le parentesi angolari al primo dei primi tre nominativi che le ha (`<IU3ABC>` diventa `IU3ABC`); `<...>` (hash sconosciuto) resta.
fn strip_braces(text: &str) -> String {
    if !text.contains('<') {
        return text.to_string();
    }
    let mut parts: Vec<String> = text.split(' ').map(str::to_string).collect();
    for p in parts.iter_mut().take(3) {
        if p.starts_with('<') && !p.starts_with("<.") {
            let inner = p.trim_start_matches('<').trim_end_matches('>');
            *p = inner.to_string();
            break;
        }
    }
    parts.join(" ")
}

/// L'SNR di JTDX: dall'altezza del picco (`snr = speak - 1`): `10 log10(snr) - 14,8` dB, con il pavimento a -21.
fn sync_to_snr(sync: f32) -> i32 {
    let snr = sync - 1.0;
    let xsnr = if snr > 0.0 { 10.0 * snr.log10() - 14.8 } else { -21.0 };
    xsnr.max(-21.0).round() as i32
}

// ---------------------------------------------------------------------------------------------------------------------
// subtractft4

/// Campioni del filtro passa-basso della sottrazione.
const NFILT: usize = 1400;

/// Toglie dall'audio un segnale decodificato (`subtractft4`): lo rigenera, ne stima ampiezza e fase complesse filtrando `audio * conj(riferimento)`
/// con una finestra coseno quadro di 1401 punti, e sottrae `2 Re(ampiezza * riferimento)`.
struct Subtractor {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    /// FFT del filtro, gia' divisa per la lunghezza.
    kernel: Vec<Complex>,
}

impl Subtractor {
    fn new() -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(NMAX);
        let inverse = planner.plan_fft_inverse(NMAX);
        // finestra cos^2(pi j / NFILT), j = -700..700, normalizzata a somma 1; il Fortran la ruota con cshift(NFILT/2 + 1): il centro finisce
        // sull'ultimo campione, cioe' il filtro guarda un campione avanti (si conserva)
        let half = (NFILT / 2) as i64;
        let weights: Vec<f64> = (-half..=half).map(|j| (std::f64::consts::PI * j as f64 / NFILT as f64).cos().powi(2)).collect();
        let sum: f64 = weights.iter().sum();
        let mut kernel = vec![Complex::new(0.0, 0.0); NMAX];
        for (idx, w) in weights.iter().enumerate() {
            let j = idx as i64 - half; // da -700 a 700
            let pos = (j - 1).rem_euclid(NMAX as i64) as usize;
            kernel[pos].re = (w / sum) as f32;
        }
        let mut scratch = vec![Complex::new(0.0, 0.0); forward.get_inplace_scratch_len()];
        forward.process_with_scratch(&mut kernel, &mut scratch);
        for k in kernel.iter_mut() {
            *k /= NMAX as f32;
        }
        Subtractor { forward, inverse, kernel }
    }

    /// `start` e' il campione (a 666,67 Hz) del primo simbolo di Costas: il riferimento parte un simbolo di rampa prima, come in
    /// `nstart = dt * 12000 + 1 - NSPS` con `dt = start / 666,67` (anche la stessa aritmetica in singola precisione).
    fn subtract(&self, audio: &mut [f32], tones: &[u8; NN], f0_hz: f32, start: i64) {
        let dt = start as f32 / DOWN_RATE;
        let nstart = (dt * SAMPLE_RATE as f32 + 1.0 - NSPS as f32) as i64 - 1; // da 0
        let cref = modem::gfsk_complex(tones, f0_hz);
        let zero = Complex::new(0.0, 0.0);
        let mut buf = vec![zero; NMAX];
        for (i, (b, r)) in buf.iter_mut().zip(&cref).enumerate() {
            let id = nstart + i as i64;
            if id >= 0 && (id as usize) < audio.len() {
                *b = r.conj() * audio[id as usize];
            }
        }
        let mut scratch = vec![zero; self.forward.get_inplace_scratch_len().max(self.inverse.get_inplace_scratch_len())];
        self.forward.process_with_scratch(&mut buf, &mut scratch);
        for (b, k) in buf.iter_mut().zip(&self.kernel) {
            *b *= k;
        }
        self.inverse.process_with_scratch(&mut buf, &mut scratch);
        for (i, (b, r)) in buf.iter().zip(&cref).enumerate() {
            let id = nstart + i as i64;
            if id >= 0 && (id as usize) < audio.len() {
                audio[id as usize] -= 2.0 * (*b * *r).re;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SLOT_SAMPLES;
    use crate::testutil::slot;

    fn decode(audio: &[f32]) -> Vec<Decoded> {
        decode_slot(audio, &Options::default(), &mut CallHashes::default())
    }

    fn texts(out: &[Decoded]) -> Vec<String> {
        out.iter().map(|d| d.text.clone()).collect()
    }

    /// L'AGC della radio che pompa: una stazione forte fa scendere il guadagno a un quarto per 2 s a meta' del messaggio.
    fn pumped(mut a: Vec<f32>) -> Vec<f32> {
        for x in &mut a[24_000..48_000] {
            *x *= 0.25;
        }
        a
    }

    #[test]
    fn l_agcc_non_cambia_l_audio_pulito_e_non_inventa_messaggi() {
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.3, -12.0)], 3, 0.05);
        let on = decode_slot(&audio, &Options { agc: true, ..Options::default() }, &mut CallHashes::default());
        assert_eq!(texts(&on), texts(&decode(&audio)), "rumore uniforme: stesso risultato");
        for seed in 40..50 {
            let noise = pumped(slot(&[], seed, 0.05));
            assert!(decode_slot(&noise, &Options { agc: true, ..Options::default() }, &mut CallHashes::default()).is_empty(), "solo rumore pompato: niente messaggi");
        }
    }

    #[test]
    #[ignore]
    fn esplora_agc_ft4() {
        for snr in [-14.0, -15.0, -16.0, -17.0] {
            let (mut off, mut on) = (0, 0);
            for seed in 0..20 {
                let audio = pumped(slot(&[("CQ K1ABC FN42", 1500.0, 0.1, snr)], 200 + seed, 0.05));
                off += decode(&audio).len();
                on += decode_slot(&audio, &Options { agc: true, ..Options::default() }, &mut CallHashes::default()).len();
            }
            println!("SNR {snr}: senza AGCc {off}/20, con AGCc {on}/20");
        }
    }

    #[test]
    fn un_messaggio_pulito_con_frequenza_tempo_e_snr() {
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.3, -5.0)], 1, 0.05);
        let out = decode(&audio);
        assert_eq!(out.len(), 1, "{out:?}");
        let d = &out[0];
        assert_eq!(d.text, "CQ K1ABC FN42");
        assert!((d.freq_hz - 1500.0).abs() <= 2.0, "{}", d.freq_hz);
        assert!((d.dt_s - 0.3).abs() < 0.02, "{}", d.dt_s);
        assert!((-21..=5).contains(&d.snr_db), "snr {}", d.snr_db);
    }

    #[test]
    fn i_tre_messaggi_di_prova_a_snr_medio() {
        for (seed, snr) in [(21u64, -8.0f32), (22, -11.0)] {
            for text in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
                let audio = slot(&[(text, 1234.0, 0.1, snr)], seed, 0.05);
                let out = decode(&audio);
                assert!(texts(&out).iter().any(|t| t == text), "{text} a {snr} dB (seme {seed}): {out:?}");
                assert_eq!(out.len(), 1, "nessun falso: {out:?}");
            }
        }
    }

    #[test]
    fn piu_segnali_nello_stesso_periodo() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.1, -6.0), ("IU3ABC K1ABC -15", 1500.0, 0.3, -9.0), ("K1ABC IU3ABC RR73", 2200.0, -0.2, -4.0)], 7, 0.05);
        let out = decode(&audio);
        let t = texts(&out);
        for expected in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
            assert!(t.iter().any(|x| x == expected), "manca {expected}: {out:?}");
        }
        assert_eq!(out.len(), 3, "nessun falso: {out:?}");
        for d in &out {
            let (f, dt) = match d.text.as_str() {
                "CQ K1ABC FN42" => (800.0, 0.1),
                "IU3ABC K1ABC -15" => (1500.0, 0.3),
                _ => (2200.0, -0.2),
            };
            assert!((d.freq_hz - f).abs() <= 2.0 && (d.dt_s - dt).abs() < 0.02, "{d:?}");
        }
    }

    #[test]
    fn un_debole_sotto_un_forte_compare_dopo_la_sottrazione() {
        // 25 Hz di distanza: il debole e' dentro la banda del forte (4 toni = 62 Hz) finche' questo non viene tolto
        let audio = slot(&[("CQ DX IU3ABC IO91", 1500.0, 0.2, 8.0), ("K1ABC W9XYZ -12", 1525.0, 0.35, -9.0)], 11, 0.05);
        let out = decode(&audio);
        let t = texts(&out);
        assert!(t.iter().any(|x| x == "CQ DX IU3ABC IO91"), "{out:?}");
        assert!(t.iter().any(|x| x == "K1ABC W9XYZ -12"), "{out:?}");
        // senza sottrazione il debole si perde (la prova controlla che la passata serva davvero)
        let single = decode_slot(&audio, &Options::depth(1), &mut CallHashes::default());
        assert!(!texts(&single).iter().any(|x| x == "K1ABC W9XYZ -12"), "senza sottrazione: {single:?}");
    }

    #[test]
    fn solo_rumore_non_decodifica_nulla() {
        for seed in [3u64, 4, 5, 6, 8, 9, 10, 12] {
            let out = decode(&slot(&[], seed, 0.05));
            assert!(out.is_empty(), "seme {seed}: {out:?}");
        }
    }

    #[test]
    fn rumore_senza_il_controllo_sui_bit_dubbi_non_da_falsi() {
        // come WSJT-X 3.3 (ft4_decode.f90:443-481) senza AP non c'e' nessun filtro dopo CRC e unpack77: sul rumore bianco basta il CRC
        let options = Options { max_hard_errors: None, ..Options::default() };
        for seed in [13u64, 14, 15, 16] {
            let out = decode_slot(&slot(&[], seed, 0.05), &options, &mut CallHashes::default());
            assert!(out.is_empty(), "seme {seed}: {out:?}");
        }
    }

    #[test]
    fn buffer_corti_o_vuoti_non_vanno_in_panico() {
        assert!(decode(&[]).is_empty());
        assert!(decode(&vec![0.0; 1000]).is_empty());
        assert!(decode(&vec![0.0; SLOT_SAMPLES]).is_empty());
        assert!(decode(&vec![f32::NAN; SLOT_SAMPLES]).is_empty());
        assert!(decode(&vec![1.0; SLOT_SAMPLES]).is_empty());
    }

    #[test]
    fn ritardi_ai_bordi_delle_finestre() {
        // da -0,2 s a +0,6 s: copre le tre finestre di ricerca del ritardo
        for (i, dt) in [-0.25f32, 0.0, 0.55, 0.8].into_iter().enumerate() {
            let audio = slot(&[("CQ K1ABC FN42", 1000.0, dt, -3.0)], 30 + i as u64, 0.05);
            let out = decode(&audio);
            assert!(texts(&out).iter().any(|t| t == "CQ K1ABC FN42"), "dt {dt}: {out:?}");
            if let Some(d) = out.first() {
                assert!((d.dt_s - dt).abs() < 0.02, "dt {dt}: {d:?}");
            }
        }
    }

    fn ap_with(mycall: &str, hiscall: &str, progress: u8) -> ApContext {
        let mut o = ApOptions::new(mycall);
        o.hiscall = hiscall.into();
        o.hisgrid = "FN42".into();
        o.qso_progress = progress;
        o.idle = false;
        o.rx_freq_hz = Some(1500.0);
        ApContext::new(&o)
    }

    #[test]
    fn i_bit_noti_dell_ap_sono_nel_dominio_mescolato() {
        // "IU3ABC K1ABC RR73" (tipo 6): tutti i 77 bit noti; i bit sul filo sono il messaggio mescolato con RVEC
        let ctx = ap_with("IU3ABC", "K1ABC", 3);
        let bits = message::pack("IU3ABC K1ABC RR73", &mut CallHashes::default()).expect("messaggio valido");
        let wire = modem::scramble(&bits);
        let (mut z, mask) = ctx.apply(6, &[0.0; 174], 10.0).expect("tipo 6");
        for (i, &r) in RVEC.iter().enumerate() {
            if mask[i] && r == 1 {
                z[i] = -z[i];
            }
        }
        assert_eq!(mask.iter().filter(|&&m| m).count(), 77);
        for i in 0..77 {
            assert_eq!(z[i] > 0.0, wire[i] == 1, "bit {i}");
        }
    }

    #[test]
    fn i_tipi_ap_seguono_il_punto_del_qso_e_le_condizioni() {
        let none = Options::default();
        let cq = Options { ap_cq: true, ..Options::default() };
        // in ricezione a vuoto (idle) niente tipi di QSO; il CQ solo se acceso
        let mut idle = ApOptions::new("IU3ABC");
        idle.hiscall = "K1ABC".into();
        let ctx = ApContext::new(&idle);
        assert!(!ap_types(&none, &ctx, 1500.0).contains(&1));
        assert!(ap_types(&cq, &ctx, 1500.0).contains(&1));
        assert!(!ap_types(&cq, &ctx, 1500.0).iter().any(|t| (3..31).contains(t)));
        // in QSO, sulla frequenza: i tipi 3-6; lontano dalla frequenza no
        let ctx = ap_with("IU3ABC", "K1ABC", 3);
        let near = ap_types(&cq, &ctx, 1510.0);
        assert!(near.contains(&3) && near.contains(&6) && near.contains(&5) && near.contains(&4), "{near:?}");
        assert!(!ap_types(&cq, &ctx, 2800.0).iter().any(|t| (3..31).contains(t)));
        // senza il nominativo del DX restano solo CQ e MYCALL
        let ctx = ap_with("IU3ABC", "", 0);
        assert!(ap_types(&cq, &ctx, 1500.0).iter().all(|t| matches!(t, 1 | 2)));
        // niente di niente senza dati
        assert!(ap_types(&none, &ApContext::new(&ApOptions::new("")), 1500.0).is_empty());
    }

    #[test]
    fn il_limite_ai_bit_dubbi_scende_con_i_bit_noti() {
        let ctx = ap_with("IU3ABC", "K1ABC", 3);
        let (_, few) = ctx.apply(1, &[0.0; 174], 10.0).expect("tipo 1");
        let (_, all) = ctx.apply(6, &[0.0; 174], 10.0).expect("tipo 6");
        assert_eq!(ap_error_limit(&few), 45);
        assert!(ap_error_limit(&all) < 35);
    }

    #[test]
    fn le_parentesi_angolari_dei_nominativi_si_tolgono() {
        assert_eq!(strip_braces("CQ DX K1ABC"), "CQ DX K1ABC");
        assert_eq!(strip_braces("<IU3ABC> K1ABC -15"), "IU3ABC K1ABC -15");
        assert_eq!(strip_braces("K1ABC <IU3ABC> RR73"), "K1ABC IU3ABC RR73");
        assert_eq!(strip_braces("<...> K1ABC -15"), "<...> K1ABC -15");
    }

    #[test]
    fn snr_di_jtdx() {
        assert_eq!(sync_to_snr(0.5), -21);
        assert_eq!(sync_to_snr(1.0), -21);
        assert_eq!(sync_to_snr(1.1), -21);
        assert!(sync_to_snr(100.0) > sync_to_snr(10.0));
    }

    #[test]
    fn la_sottrazione_toglie_il_segnale() {
        // 40 dB sopra il rumore: l'energia che resta dopo la sottrazione e' quasi tutta rumore
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.2, 40.0)], 40, 0.001);
        let mut scaled: Vec<f32> = audio.iter().map(|&x| x * SCALE).collect();
        scaled.resize(NMAX, 0.0);
        let before: f64 = scaled.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
        let tones = modem::tones_from_bits77(&message::pack("CQ K1ABC FN42", &mut CallHashes::default()).expect("messaggio valido"));
        let start = ((0.5f32 + 0.2) * DOWN_RATE).round() as i64;
        Subtractor::new().subtract(&mut scaled, &tones, 1500.0, start);
        let after: f64 = scaled.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
        assert!(after < before * 0.02, "energia residua {} su {}", after, before);
    }

    #[test]
    #[ignore = "misura dei tempi: cargo test -p ft4 --release -- --ignored --nocapture"]
    fn tempi_di_decodifica() {
        let signals: Vec<(String, f32, f32, f32)> = (0..12)
            .map(|i| {
                let call = format!("K{}ABC", (b'A' + (i % 26) as u8) as char);
                (format!("CQ {call} FN42"), 400.0 + 280.0 * i as f32, 0.1 + 0.03 * i as f32, -6.0 - (i % 4) as f32 * 2.0)
            })
            .collect();
        let refs: Vec<(&str, f32, f32, f32)> = signals.iter().map(|(t, f, d, s)| (t.as_str(), *f, *d, *s)).collect();
        let busy = slot(&refs, 77, 0.05);
        let noise = slot(&[], 78, 0.05);
        for (name, audio) in [("rumore", &noise), ("12 segnali", &busy)] {
            for options in [Options::depth(1), Options::depth(2), Options::depth(3)] {
                let t0 = std::time::Instant::now();
                let out = decode_slot(audio, &options, &mut CallHashes::default());
                println!("{name:>10} passate {} osd {}: {} messaggi in {:?}", options.passes, options.osd_order, out.len(), t0.elapsed());
            }
        }
    }

    #[test]
    fn sensibilita_a_meno_14_db_e_snr_riportato() {
        let mut ok = 0;
        for seed in 100..108u64 {
            let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.2, -14.0)], seed, 0.05);
            let out = decode(&audio);
            if let Some(d) = out.iter().find(|d| d.text == "CQ K1ABC FN42") {
                ok += 1;
                assert!((d.snr_db - (-14)).abs() <= 3, "snr {}", d.snr_db);
            }
        }
        assert!(ok >= 7, "decodificati {ok} su 8");
    }

    #[test]
    #[ignore = "misura: cargo test -p ft4 --release -- --ignored --nocapture sensibilita_curva"]
    fn sensibilita_curva() {
        let con = Options { max_hard_errors: Some(MAX_HARD_ERRORS), ..Options::default() };
        for snr in [-14.0f32, -16.0, -17.0, -18.0, -19.0] {
            let (mut ok, mut ok_senza) = (0, 0);
            for seed in 100..120u64 {
                let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.2, snr)], seed, 0.05);
                if decode_slot(&audio, &con, &mut CallHashes::default()).iter().any(|d| d.text == "CQ K1ABC FN42") {
                    ok += 1;
                }
                if decode(&audio).iter().any(|d| d.text == "CQ K1ABC FN42") {
                    ok_senza += 1;
                }
            }
            println!("SNR {snr}: {ok}/20 con il controllo sui bit dubbi, {ok_senza}/20 senza");
        }
    }

    #[test]
    #[ignore = "misura: cargo test -p ft4 --release -- --ignored --nocapture falsi_su_molti"]
    fn falsi_su_molti_periodi_di_rumore() {
        let con = Options { max_hard_errors: Some(MAX_HARD_ERRORS), ..Options::default() };
        let (mut con_controllo, mut senza_controllo) = (0, 0);
        for seed in 1000..1200u64 {
            let audio = slot(&[], seed, 0.05);
            con_controllo += decode_slot(&audio, &con, &mut CallHashes::default()).len();
            senza_controllo += decode(&audio).len();
        }
        println!("falsi su 200 periodi di solo rumore: {con_controllo} con il controllo, {senza_controllo} senza");
    }
}
