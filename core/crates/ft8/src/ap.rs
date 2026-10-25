//! Analisi a priori (AP) come `ft8apset.f90`, `ft8_mod1.f90` (tabella `naptypes`) e la parte AP di `ft8b.f90` di JTDX 2.2.159.
//!
//! Se del messaggio si conoscono alcuni bit (CQ, il proprio nominativo, il nominativo del DX in QSO, il suo locatore, RRR/RR73/73) li si
//! mettono nel decoder con un LLR enorme e li si dichiarano fissi (`apmask`): la propagazione di credenze e OSD cercano allora solo
//! tra le parole che li rispettano, e la soglia scende di qualche dB. Il prezzo e' il rischio di "trovare" il messaggio atteso nel
//! rumore, per questo ogni tipo ha le sue condizioni (toni attesi nei simboli noti, rapporto segnale/rumore, stato del QSO) e il
//! messaggio trovato passa poi per i filtri di `filters`.
//!
//! # Tipi (iaptype) portati, per nominativi standard
//!
//! | tipo | messaggio                         | bit noti |
//! |------|-----------------------------------|----------|
//! | 1    | `CQ ??? ???`                      | 29 + 3   |
//! | 2    | `MYCALL ??? ???`                  | 29 + 3   |
//! | 3    | `MYCALL DXCALL ???`               | 58 + 3   |
//! | 4    | `MYCALL DXCALL RRR`               | 77       |
//! | 5    | `MYCALL DXCALL 73`                | 77       |
//! | 6    | `MYCALL DXCALL RR73`              | 77       |
//! | 31   | `CQ DXCALL GRID` (ricerca del DX) | 77 (o 58 + 3 senza locatore) |
//! | 35   | `??? DXCALL 73`                   | 29 + 19  |
//! | 36   | `??? DXCALL RR73`                 | 29 + 19  |
//!
//! L'ordine con cui i tipi si provano dipende dall'avanzamento del QSO (`nQSOProgress`, tabella [`NAPTYPES`]).
//!
//! # Differenze dal Fortran
//!
//! - Non sono portati i nominativi non standard (tipi 11-14, 40-44), la modalita' Hound/Fox (21-24, 111) ne' le sotto-passate che
//!   mediano con il periodo precedente (`isubp1` 3-11, `ft8s`, `ft8sd`): servono la cronologia dei periodi e un motore di QSO.
//!   Con un nominativo non standard (proprio o del DX) i tipi che lo richiederebbero restano spenti.
//! - Le condizioni che in JTDX vengono da `tone8.f90` (`idtone56`, `idtonemyc`) sono ricavate qui dagli stessi messaggi
//!   (`MYCALL DXCALL -01`, `... RRR`, `... RR73`, `... 73`) con `pack` e la codifica.

use crate::message::{self, Bits77, CallHashes};
use crate::{COSTAS, crc, ldpc, modem};

/// Opzioni dell'analisi a priori. Con `Options::ap = None` resta il solo tipo 1 (CQ), che non richiede di conoscere niente.
#[derive(Debug, Clone, PartialEq)]
pub struct ApOptions {
    /// Il proprio nominativo.
    pub mycall: String,
    /// Il nominativo del DX in QSO o che si sta chiamando (vuoto: nessuno).
    pub hiscall: String,
    /// Il suo locatore a 4 caratteri (vuoto: ignoto).
    pub hisgrid: String,
    /// Avanzamento del QSO (`nQSOProgress`): 0 CQ, 1 prima chiamata (locatore), 2 rapporto, 3 rapporto con R, 4 RRR/RR73, 5 73.
    pub qso_progress: u8,
    /// Frequenza di ricezione scelta (`nfqso`, Hz), se c'e'.
    pub rx_freq_hz: Option<f32>,
    /// Frequenza di trasmissione (`nftx`, Hz), se c'e'.
    pub tx_freq_hz: Option<f32>,
    /// Larghezza attorno alle due frequenze entro cui valgono i tipi di QSO (`napwid`, Hz).
    pub width_hz: f32,
    /// `lapmyc`: i tipi che usano il proprio nominativo sono attivi (JTDX li spegne da 2-3 minuti dopo l'ultima trasmissione, perche'
    /// in ricezione a vuoto un messaggio "per me" trovato nel rumore e' quasi sempre un falso).
    pub my_call_active: bool,
    /// `stophint`: non si sta trasmettendo (ricezione libera). Allora si cercano CQ, 73 e RR73 del DX (tipi 31, 35, 36) e non i tipi 3-6.
    pub idle: bool,
    /// `lenabledxcsearch`: ricerca del DX (tipi 31, 35, 36) permessa.
    pub dx_search: bool,
    /// `lwidedxcsearch`: la ricerca del DX vale su tutta la banda e non solo attorno alla frequenza di ricezione.
    pub wide_dx_search: bool,
}

impl ApOptions {
    /// Opzioni per ricezione libera con il proprio nominativo (`my_call_active` acceso, nessun DX, ricerca del DX spenta).
    pub fn new(mycall: &str) -> ApOptions {
        ApOptions {
            mycall: mycall.trim().to_uppercase(),
            hiscall: String::new(),
            hisgrid: String::new(),
            qso_progress: 0,
            rx_freq_hz: None,
            tx_freq_hz: None,
            width_hz: 50.0,
            my_call_active: true,
            idle: true,
            dx_search: false,
            wide_dx_search: false,
        }
    }
}

/// `naptypes(nQSOProgress, 1:27)` di `ft8_mod1.f90`: per ognuno dei 27 posti dopo le quattro letture normali (`isubp2` da 5 a 31) il
/// tipo da provare, 0 = saltare.
pub const NAPTYPES: [[u8; 27]; 6] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 6, 6, 6, 5, 5, 5, 4, 4, 4, 0, 0, 0, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [3, 3, 3, 6, 6, 6, 5, 5, 5, 4, 4, 4, 2, 2, 2, 0, 0, 0, 31, 31, 31, 36, 36, 36, 35, 35, 35],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 1, 1, 31, 31, 31, 36, 36, 36, 35, 35, 35],
];

/// Gli ultimi 19 bit di RRR, 73 e RR73 (`mrrr`, `m73`, `mrr73` di `ft8_mod1.f90`): bit 59 (R), locatore/rapporto a 15 bit, `i3`.
const MRRR: [i8; 19] = [-1, 1, 1, 1, 1, 1, 1, -1, 1, -1, -1, 1, -1, -1, 1, -1, -1, -1, 1];
const M73: [i8; 19] = [-1, 1, 1, 1, 1, 1, 1, -1, 1, -1, -1, 1, -1, 1, -1, -1, -1, -1, 1];
const MRR73: [i8; 19] = [-1, 1, 1, 1, 1, 1, 1, -1, -1, 1, 1, 1, -1, 1, -1, 1, -1, -1, 1];

/// Dall'indice della lettura (`isubp2`, 5..=31) e dal tipo, quale serie di LLR usare: 0 = `llra`, 1 = `llrb`, 2 = `llrc`.
/// Ogni tipo prova prima `llrc`, poi `llrb`, poi `llra` (i tipi 4-6 usano `llrb` due volte, come nel Fortran).
pub fn llr_index(isubp2: usize, aptype: u8) -> usize {
    match (isubp2 - 5) % 3 {
        0 => 2,
        1 => 1,
        _ if (4..=6).contains(&aptype) => 1,
        _ => 0,
    }
}

/// I 58 toni di dati (`idtone`) di un messaggio: i simboli 8-36 e 44-72 dei 79.
pub fn data_tones(tones: &[u8; 79]) -> [u8; 58] {
    let mut out = [0u8; 58];
    out[..29].copy_from_slice(&tones[7..36]);
    out[29..].copy_from_slice(&tones[43..72]);
    out
}

/// Dai bit del messaggio ai 79 toni (CRC, codice LDPC, mappa Gray e Costas).
fn tones_of_bits(bits: &Bits77) -> [u8; 79] {
    modem::tones_from_codeword(&ldpc::encode(&crc::append_crc(bits)))
}

/// Impacchetta `text` e verifica che si rilegga uguale (come `ft8apset`), senza toccare le tabelle degli hash di chi decodifica.
fn bits_of(text: &str) -> Option<Bits77> {
    let mut scratch = CallHashes::default();
    let bits = message::pack(text, &mut scratch).ok()?;
    let back = message::unpack(&bits, &mut scratch).ok()?;
    (back == text).then_some(bits)
}

/// Un nominativo vale come standard se `CQ nominativo` si impacchetta nel tipo 1 (o 2, con /P).
fn is_standard(call: &str) -> bool {
    bits_of(&format!("CQ {call}")).is_some_and(|b| matches!(message::message_type(&b).0, 1 | 2))
}

fn signs(bits: &[u8]) -> Vec<i8> {
    bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).collect()
}

/// Tutto cio' che serve a provare i tipi AP, calcolato una volta per periodo dalle opzioni.
#[derive(Debug, Clone)]
pub struct ApContext {
    pub opts: ApOptions,
    /// Il proprio nominativo e' standard e c'e' (tipi 2-6 possibili).
    pub my_ok: bool,
    /// Il nominativo del DX e' standard e c'e' (tipi 3-6, 31, 35, 36 possibili).
    pub his_ok: bool,
    /// Il locatore del DX e' noto (quattro caratteri validi).
    pub his_grid_known: bool,
    /// `CQ` + 3 bit di tipo: i bit 1-29 e 75-77 del messaggio `CQ ...`.
    mcq: [i8; 29],
    /// I primi 58 bit di `MYCALL DXCALL RRR` (con il proprio nominativo al posto del DX se manca).
    apsym: [i8; 58],
    /// I 77 bit di `CQ DXCALL GRID` (se il locatore e' ignoto contano solo i primi 58 e i tre di tipo).
    apcq: [i8; 77],
    /// Toni di dati attesi per `CQ ...` nei primi nove simboli (`idtone25`): otto volte il tono 0 e poi il tono 1.
    pub cq_tones: [u8; 9],
    /// Toni di dati attesi per i primi nove simboli del proprio nominativo (`idtonemyc`).
    pub my_tones: [u8; 9],
    /// `idtone56(1,:)`: `MYCALL DXCALL -01`, i primi 25 simboli di dati (i 58 bit di nominativi e l'inizio del rapporto).
    pub qso_tones: [u8; 25],
    /// Toni di dati completi di `MYCALL DXCALL 73`, `RR73` e `RRR` (`idtone56(56|55|54,:)`).
    pub tones_73: [u8; 58],
    pub tones_rr73: [u8; 58],
    pub tones_rrr: [u8; 58],
}

impl ApContext {
    pub fn new(opts: &ApOptions) -> ApContext {
        let mut ctx = ApContext {
            opts: opts.clone(),
            my_ok: false,
            his_ok: false,
            his_grid_known: false,
            mcq: [-1; 29],
            apsym: [0; 58],
            apcq: [0; 77],
            cq_tones: [0, 0, 0, 0, 0, 0, 0, 0, 1],
            my_tones: [0; 9],
            qso_tones: [0; 25],
            tones_73: [0; 58],
            tones_rr73: [0; 58],
            tones_rrr: [0; 58],
        };
        // CQ: n28 = 2, quindi tutti zero tranne il bit 27 (da 1)
        ctx.mcq[26] = 1;
        let my = opts.mycall.trim();
        let his = opts.hiscall.trim();
        ctx.my_ok = my.len() >= 3 && is_standard(my);
        ctx.his_ok = his.len() >= 3 && is_standard(his);
        // i 58 bit "primo secondo": servono anche senza il proprio nominativo (per i tipi 35 e 36 contano solo quelli del DX)
        let first = if ctx.my_ok { my } else { "K1ABC" };
        let second = if ctx.his_ok { his } else { first };
        match bits_of(&format!("{first} {second} RRR")) {
            Some(bits) if message::message_type(&bits).0 == 1 => {
                ctx.apsym.copy_from_slice(&signs(&bits[..58]));
                let tones = data_tones(&tones_of_bits(&bits));
                ctx.my_tones.copy_from_slice(&tones[..9]);
            }
            _ => {
                ctx.my_ok = false;
                ctx.his_ok = false;
            }
        }
        if ctx.my_ok && ctx.his_ok {
            if let Some(bits) = bits_of(&format!("{my} {his} -01")) {
                ctx.qso_tones.copy_from_slice(&data_tones(&tones_of_bits(&bits))[..25]);
            }
            for (text, dst) in [("73", &mut ctx.tones_73), ("RR73", &mut ctx.tones_rr73), ("RRR", &mut ctx.tones_rrr)] {
                if let Some(bits) = bits_of(&format!("{my} {his} {text}")) {
                    *dst = data_tones(&tones_of_bits(&bits));
                }
            }
        }
        if ctx.his_ok {
            let grid = opts.hisgrid.trim();
            let with_grid = (grid.len() == 4).then(|| bits_of(&format!("CQ {his} {grid}"))).flatten();
            if let Some(bits) = with_grid {
                ctx.apcq.copy_from_slice(&signs(&bits));
                ctx.his_grid_known = true;
            } else if let Some(bits) = bits_of(&format!("CQ {his}")) {
                // senza locatore: i bit del nominativo e il tipo (CQ + nominativo + 'nessun locatore')
                ctx.apcq.copy_from_slice(&signs(&bits));
            } else {
                ctx.his_ok = false;
            }
        }
        ctx
    }

    /// Maschera e LLR a priori del tipo `aptype` sopra `llr` (che e' la serie scelta), con `apmag` come ampiezza dei bit noti.
    /// `None` se il tipo non si puo' costruire con questi dati.
    pub fn apply(&self, aptype: u8, llr: &[f32; 174], apmag: f32) -> Option<([f32; 174], [bool; 174])> {
        let mut z = *llr;
        let mut mask = [false; 174];
        let mut set = |from: usize, signs: &[i8], z: &mut [f32; 174]| {
            for (i, &s) in signs.iter().enumerate() {
                z[from + i] = apmag * f32::from(s);
                mask[from + i] = true;
            }
        };
        // i tre bit di tipo di un messaggio standard (i3 = 1: 001)
        let type_bits = [-1i8, -1, 1];
        match aptype {
            1 => {
                set(0, &self.mcq, &mut z);
                set(74, &type_bits, &mut z);
            }
            2 if self.my_ok => {
                set(0, &self.apsym[..29], &mut z);
                set(74, &type_bits, &mut z);
            }
            3 if self.my_ok && self.his_ok => {
                set(0, &self.apsym, &mut z);
                set(74, &type_bits, &mut z);
            }
            4..=6 if self.my_ok && self.his_ok => {
                set(0, &self.apsym, &mut z);
                let tail: &[i8; 19] = match aptype {
                    4 => &MRRR,
                    5 => &M73,
                    _ => &MRR73,
                };
                set(58, tail, &mut z);
            }
            31 if self.his_ok => {
                if self.his_grid_known {
                    set(0, &self.apcq, &mut z);
                } else {
                    set(0, &self.apcq[..58], &mut z);
                    set(74, &type_bits, &mut z);
                }
            }
            35 | 36 if self.his_ok => {
                set(29, &self.apsym[29..], &mut z);
                set(58, if aptype == 35 { &M73 } else { &MRR73 }, &mut z);
            }
            _ => return None,
        }
        Some((z, mask))
    }
}

/// Numero di simboli (su `expected`, a partire dal primo di dati) il cui tono piu' forte in `s8` e' quello atteso.
/// Il simbolo di dati `i` (da 0) sta nella colonna `i + 7` per i primi 29 e `i + 14` per gli altri.
pub fn tone_matches(s8: &[[f32; 8]; 79], expected: &[u8], from_data_symbol: usize) -> usize {
    let mut n = 0;
    for (k, &tone) in expected.iter().enumerate() {
        let i = from_data_symbol + k;
        let col = if i < 29 { i + 7 } else { i + 14 };
        if col < 79 && argmax8(&s8[col]) == usize::from(tone) {
            n += 1;
        }
    }
    n
}

/// Il tono piu' forte di un simbolo (il primo in caso di parita', come `maxloc`).
pub fn argmax8(v: &[f32; 8]) -> usize {
    let mut best = 0;
    for (i, &x) in v.iter().enumerate() {
        if x > v[best] {
            best = i;
        }
    }
    best
}

/// Rapporto tra l'energia dei toni attesi nei primi nove simboli di dati e la media degli altri sette (`scqnr`, `smycnr`).
pub fn expected_ratio(s8: &[[f32; 8]; 79], expected: &[u8; 9]) -> f32 {
    let mut level = 0.0f32;
    for (i, &t) in expected.iter().enumerate() {
        level += s8[i + 7][usize::from(t)];
    }
    let total: f32 = (7..16).map(|k| s8[k].iter().sum::<f32>()).sum();
    let mut noise = (total - level) / 7.0;
    if noise == 0.0 {
        noise = f32::MIN_POSITIVE;
    }
    level / noise
}

/// Punteggio "CQ" di `ft8b` (`rscq`): nei simboli 8-16 un CQ ha il tono 0 (e il tono 1 all'ultimo), e qualche conferma sugli altri.
pub fn cq_score(s8: &[[f32; 8]; 79]) -> f32 {
    let mut rscq = 0.0f32;
    for (k, row) in s8.iter().enumerate().take(16).skip(7) {
        let ip = argmax8(row);
        if (k < 15 && ip == 0) || (k == 15 && ip == 1) {
            rscq += 1.0;
        }
    }
    let ip = argmax8(&s8[16]);
    if ip == 0 || ip == 1 {
        rscq += 0.5;
    }
    let ip = argmax8(&s8[26]);
    if ip == 0 || ip == 1 {
        rscq += 0.5;
    }
    let ip = argmax8(&s8[32]);
    if ip == 2 || ip == 3 {
        rscq += 0.5;
    }
    rscq
}

/// Il tono atteso nel simbolo di Costas `k` di un blocco (serve ai controlli di sincronismo).
pub fn costas_tone(k: usize) -> usize {
    usize::from(COSTAS[k])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tones_for(text: &str) -> [u8; 79] {
        tones_of_bits(&bits_of(text).expect("messaggio valido"))
    }

    #[test]
    fn il_contesto_riconosce_nominativi_standard_e_costruisce_i_bit() {
        let mut o = ApOptions::new("iu3abc");
        o.hiscall = "K1ABC".into();
        o.hisgrid = "FN42".into();
        let ctx = ApContext::new(&o);
        assert!(ctx.my_ok && ctx.his_ok && ctx.his_grid_known);
        // i 58 bit di nominativi di "IU3ABC K1ABC RRR" sono quelli del messaggio vero
        let bits = bits_of("IU3ABC K1ABC RRR").expect("messaggio");
        let (z, mask) = ctx.apply(3, &[0.0; 174], 10.0).expect("tipo 3");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 58 + 3);
        for i in 0..58 {
            assert_eq!(z[i] > 0.0, bits[i] == 1, "bit {i}");
        }
        let (_, mask) = ctx.apply(6, &[0.0; 174], 10.0).expect("tipo 6");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 77);
        let (_, mask) = ctx.apply(31, &[0.0; 174], 10.0).expect("tipo 31");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 77);
        let (_, mask) = ctx.apply(35, &[0.0; 174], 10.0).expect("tipo 35");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 29 + 19);
        let (_, mask) = ctx.apply(1, &[0.0; 174], 10.0).expect("tipo 1");
        assert_eq!(mask.iter().filter(|&&m| m).count(), 29 + 3);
    }

    #[test]
    fn senza_dati_restano_solo_i_tipi_che_non_ne_chiedono() {
        let ctx = ApContext::new(&ApOptions::new(""));
        assert!(ctx.apply(1, &[0.0; 174], 10.0).is_some());
        for t in [2u8, 3, 4, 5, 6, 31, 35, 36] {
            assert!(ctx.apply(t, &[0.0; 174], 10.0).is_none(), "tipo {t}");
        }
        let mut dx = ApOptions::new("");
        dx.hiscall = "K1ABC".into();
        let ctx = ApContext::new(&dx);
        assert!(ctx.apply(31, &[0.0; 174], 10.0).is_some());
        assert!(ctx.apply(35, &[0.0; 174], 10.0).is_some());
        assert!(ctx.apply(3, &[0.0; 174], 10.0).is_none());
    }

    #[test]
    fn un_nominativo_non_standard_non_abilita_i_tipi_che_lo_richiedono() {
        let mut o = ApOptions::new("IU3ABC");
        o.hiscall = "PJ4/K1ABC".into();
        let ctx = ApContext::new(&o);
        assert!(ctx.my_ok && !ctx.his_ok);
        assert!(ctx.apply(3, &[0.0; 174], 10.0).is_none());
        assert!(ctx.apply(2, &[0.0; 174], 10.0).is_some());
    }

    #[test]
    fn i_toni_attesi_tornano_sul_segnale_ideale() {
        let mut o = ApOptions::new("IU3ABC");
        o.hiscall = "K1ABC".into();
        let ctx = ApContext::new(&o);
        // un periodo ideale per "IU3ABC K1ABC -01": il tono giusto a 10, gli altri a 1
        let tones = tones_for("IU3ABC K1ABC -01");
        let mut s8 = [[1.0f32; 8]; 79];
        for (row, &t) in s8.iter_mut().zip(tones.iter()) {
            row[usize::from(t)] = 10.0;
        }
        assert_eq!(tone_matches(&s8, &ctx.qso_tones[..19], 0), 19);
        assert_eq!(tone_matches(&s8, &ctx.my_tones, 0), 9);
        assert!(expected_ratio(&s8, &ctx.my_tones) > 5.0);
        // e un CQ ideale ha il punteggio CQ pieno
        let cq = tones_for("CQ K1ABC FN42");
        let mut s8cq = [[1.0f32; 8]; 79];
        for (row, &t) in s8cq.iter_mut().zip(cq.iter()) {
            row[usize::from(t)] = 10.0;
        }
        assert!(cq_score(&s8cq) >= 9.0, "{}", cq_score(&s8cq));
        assert_eq!(&data_tones(&cq)[..9], &ctx.cq_tones);
    }

    #[test]
    fn la_serie_di_llr_di_ogni_tipo_segue_l_ordine_di_jtdx() {
        // tipo 1: isubp2 20, 21, 22 -> llrc, llrb, llra
        assert_eq!([20, 21, 22].map(|i| llr_index(i, 1)), [2, 1, 0]);
        // i tipi 4-6 usano llrb due volte
        assert_eq!([8, 9, 10].map(|i| llr_index(i, 4)), [2, 1, 1]);
        assert_eq!(NAPTYPES[3][3], 6);
        assert_eq!(NAPTYPES[0][12], 2);
    }
}
