//! Punteggio e Cabrillo del contest attivo dei modi testuali (file `.contest`, `docs/contest-file.md`).
//!
//! I QSO sono quelli del log salvati dalla pagina dei modi testuali: gli scambi stanno in coda al commento (`NRS 001 NRR 015`).
//! Contano i QSO dentro la finestra del contest (le date del file portate nell'anno corrente) e di una categoria valida (CW, fonia,
//! digitale; nessuna scelta = tutte). Un doppio (stesso nominativo, stessa banda salvo `m_bDupeNoBand`, stesso modo salvo
//! `m_bDupeNoMode`) vale zero punti e nessun moltiplicatore.
//!
//! Punti: `m_iPointsMyDxcc` per lo stesso paese, `m_iPointsMyCont` per lo stesso continente, `m_iPointsDx` per gli altri; tutti a zero
//! = un punto per QSO. Moltiplicatori: `m_iMultipliers` e `m_iMultipliersOption` (si sommano), ognuno 0 nessuno, 1 paese DXCC,
//! 2 prefisso WPX, 3 zona CQ, 4 zona ITU, 5 dominio (dall'elenco `.dom`, o lo scambio ricevuto se l'elenco manca). Con `m_bBandMult`
//! i moltiplicatori si contano per banda e quelli di una banda accesa con peso valgono il peso. Punteggio = punti x moltiplicatori
//! (senza moltiplicatori: i punti).

use crate::calendario::{civil_from_days, data, days_from_civil, hhmm};
use crate::elenco::{ContestTesto, Dominio, data_ms};
use crate::geo;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Un QSO del log per il contest dei modi testuali.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct QsoTesto {
    pub call: String,
    /// `20m`.
    pub band: String,
    pub mode: String,
    pub freq_hz: u64,
    /// Inizio del QSO, ms UTC.
    pub time_ms: i64,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub exch_sent: String,
    pub exch_rcvd: String,
}

/// Paese, continente e zone di un nominativo (cty.dat).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfoPaese {
    /// Il prefisso principale del paese DXCC.
    pub dxcc: String,
    pub continente: String,
    pub cq: u8,
    pub itu: u8,
}

/// Gli scambi scritti in coda al commento dalla pagina dei modi testuali: `note NRS 001 NRR 015` -> (`001`, `015`).
pub fn scambi_da_commento(commento: &str) -> (String, String) {
    let parole: Vec<&str> = commento.split_whitespace().collect();
    let Some(i) = parole.iter().rposition(|p| *p == "NRS") else { return (String::new(), String::new()) };
    let resto = &parole[i + 1..];
    let Some(j) = resto.iter().position(|p| *p == "NRR") else { return (resto.join(" "), String::new()) };
    (resto[..j].join(" "), resto[j + 1..].join(" "))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Categoria {
    Cw,
    Phone,
    Digital,
}

/// La categoria di un modo: CW, fonia (SSB, AM, FM) o digitale (tutto il resto).
pub fn categoria(modo: &str) -> Categoria {
    match modo.trim().to_ascii_uppercase().as_str() {
        "CW" => Categoria::Cw,
        "SSB" | "USB" | "LSB" | "AM" | "FM" | "PHONE" => Categoria::Phone,
        _ => Categoria::Digital,
    }
}

/// Il modo conta per il contest? Nessuna categoria scelta = tutte.
pub fn categoria_valida(c: &ContestTesto, modo: &str) -> bool {
    if !c.valid_cw && !c.valid_phone && !c.valid_digital {
        return true;
    }
    match categoria(modo) {
        Categoria::Cw => c.valid_cw,
        Categoria::Phone => c.valid_phone,
        Categoria::Digital => c.valid_digital,
    }
}

fn anno_di(ms: i64) -> i64 {
    civil_from_days(ms.div_euclid(86_400_000)).0
}

fn sposta_anni(ms: i64, anni: i64) -> i64 {
    let giorno = ms.div_euclid(86_400_000);
    let (y, m, d) = civil_from_days(giorno);
    days_from_civil(y + anni, m, d) * 86_400_000 + ms.rem_euclid(86_400_000)
}

/// La finestra del contest nell'anno di `ora_ms` (le date del file valgono ogni anno), come nella pagina. `None` = nessuna data.
pub fn finestra(c: &ContestTesto, ora_ms: i64) -> Option<(Option<i64>, Option<i64>)> {
    let (a, b) = (data_ms(&c.start), data_ms(&c.end));
    let rif = a.or(b)?;
    let anni = anno_di(ora_ms) - anno_di(rif);
    Some((a.map(|x| sposta_anni(x, anni)), b.map(|x| sposta_anni(x, anni))))
}

/// I QSO che contano per il contest (finestra e categoria), dal piu' vecchio.
pub fn qso_del_contest(c: &ContestTesto, qsos: &[QsoTesto], ora_ms: i64) -> Vec<QsoTesto> {
    let w = finestra(c, ora_ms);
    let mut v: Vec<QsoTesto> = qsos
        .iter()
        .filter(|q| categoria_valida(c, &q.mode))
        .filter(|q| match w {
            None => true,
            Some((da, a)) => da.is_none_or(|x| q.time_ms >= x) && a.is_none_or(|x| q.time_ms <= x),
        })
        .cloned()
        .collect();
    v.sort_by_key(|q| q.time_ms);
    v
}

/// Il tipo di moltiplicatore (`m_iMultipliers`, `m_iMultipliersOption`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoMolt {
    Dxcc,
    Wpx,
    Cq,
    Itu,
    Dominio,
    /// Continente (solo dalle regole `[Score]`).
    Cont,
    /// Area numerica (solo dalle regole `[Score]`).
    Area,
    /// Le chiavi date dalla funzione `moltiplicatori(qso)` della sezione `[Script]`.
    Script,
    /// Il nominativo (solo dalle regole `[Score]`).
    Call,
}

impl TipoMolt {
    pub fn da_codice(n: i32) -> Option<TipoMolt> {
        match n {
            1 => Some(TipoMolt::Dxcc),
            2 => Some(TipoMolt::Wpx),
            3 => Some(TipoMolt::Cq),
            4 => Some(TipoMolt::Itu),
            5 => Some(TipoMolt::Dominio),
            _ => None,
        }
    }
}

/// I tipi di moltiplicatore del contest (senza ripetizioni).
pub fn tipi_molt(c: &ContestTesto) -> Vec<TipoMolt> {
    let mut v: Vec<TipoMolt> = [c.multipliers, c.multipliers_option].into_iter().filter_map(TipoMolt::da_codice).collect();
    v.dedup();
    v
}

/// Il peso dei moltiplicatori di una banda (`20m` o `20`): quello del file se la banda e' accesa e il peso e' positivo, altrimenti 1.
pub fn peso_banda(c: &ContestTesto, banda: &str) -> i64 {
    let b = banda.trim().trim_end_matches(['m', 'M']);
    c.bands.iter().find(|x| x.band == b && x.on && x.weight > 0).map_or(1, |x| i64::from(x.weight))
}

/// Un QSO valutato.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Valutato {
    pub qso: QsoTesto,
    pub doppio: bool,
    pub punti: i64,
    /// I moltiplicatori nuovi portati da questo QSO (`dxcc:I`, `cq:15`...).
    pub nuovi: Vec<String>,
    /// Non ammesso dalle regole: `band` o `valid` (zero punti, nessun moltiplicatore, non e' un doppio).
    pub escluso: Option<String>,
}

/// Una riga della statistica per banda.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RigaBanda {
    pub band: String,
    pub qso: u32,
    pub doppi: u32,
    pub punti: i64,
    pub molt: u32,
    pub cw: u32,
    pub phone: u32,
    pub digital: u32,
}

/// Il punteggio del contest.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Punteggio {
    /// QSO validi (senza i doppi) e doppi.
    pub qso: u32,
    pub doppi: u32,
    pub punti: i64,
    /// I moltiplicatori (somma dei pesi con i moltiplicatori per banda).
    pub molt: i64,
    pub punteggio: i64,
    /// I QTC mandati e ricevuti (contest WAE): contano come i punti.
    pub qtc: u32,
    /// I tipi di moltiplicatore usati.
    pub tipi: Vec<TipoMolt>,
    pub bande: Vec<RigaBanda>,
    /// QSO non ammessi (banda o paese fuori dal contest).
    pub esclusi: u32,
    /// Fattore finale in millesimi (1000 = 1; RAEM polare 1100).
    pub fattore_mille: i64,
    /// Il punteggio si moltiplica anche per i QSO validi (Alessandro Volta).
    pub per_qso: bool,
    /// I moltiplicatori lavorati, con le bande.
    pub griglia: Vec<CellaMolt>,
    /// I moltiplicatori che mancano dei tipi con elenco chiuso.
    pub mancanti: Vec<Mancanti>,
    /// Ritmo: QSO all'ora sugli ultimi 10 minuti, QSO nell'ultima ora, media all'ora.
    pub rate10: u32,
    pub rate60: u32,
    pub media_ora: f64,
    /// Gli ultimi QSO che hanno portato moltiplicatori nuovi (il piu' recente per primo, al massimo 10).
    pub recenti: Vec<NuovoMolt>,
    /// L'errore dello script della sezione `[Script]`: il punteggio e' stato calcolato solo con la sezione `[Score]`.
    pub errore_script: Option<String>,
    /// Lo script del contest (per ricalcolare il punteggio finale con i QTC) e lo scambio mandato nel primo QSO.
    #[serde(skip)]
    pub script: String,
    #[serde(skip)]
    pub primo_mandato: String,
    /// Il punteggio e' la somma, banda per banda, dei punti per i moltiplicatori (`band_score=1`, IPA).
    #[serde(skip)]
    pub per_banda: bool,
    /// Il paese della propria stazione e i QSO validi per paese (per `punteggio(totali)` dello script).
    #[serde(skip)]
    pub mio_dxcc: String,
    #[serde(skip)]
    pub mio_cont: String,
    #[serde(skip)]
    pub paesi: BTreeMap<String, crate::script::PerPaese>,
}

/// Un QSO che ha portato moltiplicatori nuovi.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct NuovoMolt {
    pub call: String,
    pub band: String,
    pub time_ms: i64,
    pub nuovi: Vec<String>,
}

impl Default for Punteggio {
    fn default() -> Self {
        Punteggio { qso: 0, doppi: 0, punti: 0, molt: 0, punteggio: 0, qtc: 0, tipi: Vec::new(), bande: Vec::new(), esclusi: 0, fattore_mille: 1000, per_qso: false, griglia: Vec::new(), mancanti: Vec::new(), rate10: 0, rate60: 0, media_ora: 0.0, recenti: Vec::new(), errore_script: None, script: String::new(), primo_mandato: String::new(), per_banda: false, mio_dxcc: String::new(), mio_cont: String::new(), paesi: BTreeMap::new() }
    }
}

/// Un moltiplicatore lavorato: tipo, valore e bande dove e' stato preso (vuoto se conta una volta sola).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CellaMolt {
    pub tipo: String,
    pub valore: String,
    pub bande: Vec<String>,
}

/// I valori che mancano di un tipo con un elenco chiuso (zone, continenti, aree, domini).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Mancanti {
    pub tipo: String,
    pub per_banda: bool,
    pub totali: u32,
    /// I valori mai presi (su nessuna banda).
    pub valori: Vec<String>,
}

/// Valuta i QSO del contest (gia' filtrati da [`qso_del_contest`], dal piu' vecchio) con le regole del file
/// ([`crate::regole_punti`]): ammessi, doppi, punti, moltiplicatori, punteggio, statistica per banda e griglia dei moltiplicatori.
/// `mio` = paese della stazione, `paese` = la ricerca nel database DXCC.
/// Con la sezione `[Script]` le sue funzioni sostituiscono le parti di `[Score]`; un errore dello script (anche il limite delle
/// operazioni) fa ricalcolare tutto con la sola `[Score]` e resta in [`Punteggio::errore_script`].
pub fn valuta(c: &ContestTesto, qsos: &[QsoTesto], mio: Option<&InfoPaese>, paese: &dyn Fn(&str) -> Option<InfoPaese>, dominio: Option<&Dominio>) -> (Vec<Valutato>, Punteggio) {
    if c.script.trim().is_empty() {
        return valuta_con(c, qsos, mio, paese, dominio, None).unwrap_or_default();
    }
    let errore = match crate::script::ScriptContest::compila(&c.script) {
        Ok(sc) => match valuta_con(c, qsos, mio, paese, dominio, Some(&sc)) {
            Ok((v, mut tot)) => {
                tot.script = c.script.clone();
                calcola(&mut tot);
                if tot.errore_script.is_none() {
                    return (v, tot);
                }
                tot.errore_script.unwrap_or_default()
            }
            Err(e) => e,
        },
        Err(e) => e,
    };
    tracing::warn!("script del contest {}: {errore}", c.id);
    let (v, mut tot) = valuta_con(c, qsos, mio, paese, dominio, None).unwrap_or_default();
    tot.errore_script = Some(errore);
    (v, tot)
}

fn valuta_con(c: &ContestTesto, qsos: &[QsoTesto], mio: Option<&InfoPaese>, paese: &dyn Fn(&str) -> Option<InfoPaese>, dominio: Option<&Dominio>, sc: Option<&crate::script::ScriptContest>) -> Result<(Vec<Valutato>, Punteggio), String> {
    use crate::regole_punti::{Ctx, RegoleScore, ammesso, peso, punti, valore_molt, valori_possibili};
    use crate::script::QsoScript;
    let molt_script = sc.is_some_and(|s| s.ha("moltiplicatori"));
    let r = match RegoleScore::del_contest(c) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("regole del punteggio di {} non valide: {e}", c.id);
            RegoleScore::dalle_chiavi(c)
        }
    };
    let mut tipi: Vec<TipoMolt> = if molt_script { vec![TipoMolt::Script] } else { r.molt.iter().filter(|d| d.aggiunta == 0).map(|d| d.tipo.tipo()).collect() };
    tipi.dedup();
    let mut griglia_script: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut visti: HashSet<(String, String, String)> = HashSet::new();
    let mut molt: HashSet<String> = HashSet::new();
    let mut griglia: BTreeMap<(usize, String), BTreeSet<String>> = BTreeMap::new();
    let (mut somma, mut prodotto) = (0i64, 0i64);
    let mut bande: BTreeMap<String, RigaBanda> = BTreeMap::new();
    let mut out = Vec::with_capacity(qsos.len());
    let mut tot = Punteggio { tipi: tipi.clone(), ..Punteggio::default() };
    for q in qsos {
        let call = q.call.trim().to_ascii_uppercase();
        let banda = q.band.to_ascii_lowercase();
        let info = paese(&call);
        let x = Ctx { q, call: &call, info: info.as_ref(), mio };
        let xs = QsoScript { q, call: &call, info: info.as_ref(), mio, wpx: geo::prefisso_wpx(&call) };
        let esito = match sc.map(|s| s.ammesso(&xs)).transpose()?.flatten() {
            Some(e) => e,
            None => ammesso(&r, &x).map_err(str::to_string),
        };
        if let Err(m) = esito {
            tot.esclusi += 1;
            out.push(Valutato { qso: q.clone(), doppio: false, punti: 0, nuovi: Vec::new(), escluso: Some(m) });
            continue;
        }
        let riga = bande.entry(banda.clone()).or_insert_with(|| RigaBanda { band: banda.clone(), ..RigaBanda::default() });
        let turno = sc.map(|s| s.turno(&xs)).transpose()?.flatten().unwrap_or_default();
        let chiave = (format!("{call}~{turno}"), if c.dupe_no_band { String::new() } else { banda.clone() }, if c.dupe_no_mode { String::new() } else { q.mode.to_ascii_uppercase() });
        if !visti.insert(chiave) {
            tot.doppi += 1;
            riga.doppi += 1;
            out.push(Valutato { qso: q.clone(), doppio: true, punti: 0, nuovi: Vec::new(), escluso: None });
            continue;
        }
        let mut p = match sc.map(|s| s.punti(&xs)).transpose()?.flatten() {
            Some(p) => p,
            None => punti(&r, &x),
        };
        let mut nuovi = Vec::new();
        if let Some(chiavi) = sc.filter(|_| molt_script).map(|s| s.moltiplicatori(&xs)).transpose()?.flatten() {
            for k in chiavi {
                if molt.insert(format!("s:{k}")) {
                    riga.molt += 1;
                    somma += 1;
                    // `DXCC:I@20` -> tipo `dxcc`, valore `I`, banda `20m`
                    let (corpo, banda_k) = k.split_once('@').map_or((k.as_str(), None), |(a, b)| (a, Some(b)));
                    let (tipo, valore) = corpo.split_once(':').unwrap_or(("SCRIPT", corpo));
                    let tipo = tipo.to_ascii_lowercase();
                    nuovi.push(format!("{tipo}:{valore}"));
                    let cella = griglia_script.entry((tipo, valore.to_string())).or_default();
                    if let Some(b) = banda_k {
                        cella.insert(format!("{}m", b.trim_end_matches(['m', 'M']).to_ascii_lowercase()));
                    }
                }
            }
        }
        for (i, d) in r.molt.iter().enumerate().filter(|_| !molt_script) {
            let Some(v) = valore_molt(d, &x, dominio) else { continue };
            let mut k = format!("{i}:{v}");
            if d.per_banda {
                k.push_str(&format!("@{banda}"));
            }
            if d.per_modo {
                k.push_str(&format!("#{}", q.mode.to_ascii_uppercase()));
            }
            if molt.insert(k) {
                nuovi.push(format!("{}:{v}", d.tipo.nome()));
                riga.molt += 1;
                let w = if d.per_banda { peso(&r, &banda) } else { 1 };
                if d.aggiunta != 0 {
                    p += d.aggiunta;
                } else if d.prodotto {
                    prodotto += w;
                } else {
                    somma += w;
                }
                let cella = griglia.entry((i, v)).or_default();
                if d.per_banda {
                    cella.insert(banda.clone());
                }
            }
        }
        tot.qso += 1;
        tot.punti += p;
        if let Some(i) = info.as_ref().filter(|i| !i.dxcc.is_empty()) {
            let pp = tot.paesi.entry(i.dxcc.clone()).or_default();
            pp.qso += 1;
            pp.punti += p;
            pp.continente = i.continente.clone();
            pp.bande.insert(banda.clone());
        }
        riga.qso += 1;
        riga.punti += p;
        match categoria(&q.mode) {
            Categoria::Cw => riga.cw += 1,
            Categoria::Phone => riga.phone += 1,
            Categoria::Digital => riga.digital += 1,
        }
        out.push(Valutato { qso: q.clone(), doppio: false, punti: p, nuovi, escluso: None });
    }
    let ha_somma = r.molt.iter().any(|d| !d.prodotto && d.aggiunta == 0);
    let ha_prodotto = r.molt.iter().any(|d| d.prodotto && d.aggiunta == 0);
    tot.molt = if molt_script {
        somma
    } else if tipi.is_empty() {
        0
    } else {
        (if ha_somma { somma } else { 1 }) * if ha_prodotto { prodotto } else { 1 }
    };
    tot.primo_mandato = qsos.iter().map(|q| q.exch_sent.clone()).find(|e| !e.trim().is_empty()).unwrap_or_default();
    tot.per_qso = r.per_qso;
    tot.per_banda = r.per_banda;
    tot.mio_dxcc = mio.map(|m| m.dxcc.clone()).unwrap_or_default();
    tot.mio_cont = mio.map(|m| m.continente.clone()).unwrap_or_default();
    tot.griglia = griglia.into_iter().map(|((i, valore), b)| CellaMolt { tipo: r.molt[i].tipo.nome().to_string(), valore, bande: ordina_bande(b.into_iter().collect()) }).collect();
    tot.griglia.extend(griglia_script.into_iter().map(|((tipo, valore), b)| CellaMolt { tipo, valore, bande: ordina_bande(b.into_iter().collect()) }));
    for d in r.molt.iter().filter(|_| !molt_script) {
        let tutti = valori_possibili(&d.tipo, dominio);
        if tutti.is_empty() || tot.mancanti.iter().any(|m| m.tipo == d.tipo.nome()) {
            continue;
        }
        let presi: HashSet<&str> = tot.griglia.iter().filter(|g| g.tipo == d.tipo.nome()).map(|g| g.valore.as_str()).collect();
        tot.mancanti.push(Mancanti { tipo: d.tipo.nome().to_string(), per_banda: d.per_banda, totali: tutti.len() as u32, valori: tutti.iter().filter(|v| !presi.contains(v.as_str())).cloned().collect() });
    }
    tot.recenti = out.iter().rev().filter(|v| !v.nuovi.is_empty()).take(10).map(|v| NuovoMolt { call: v.qso.call.trim().to_ascii_uppercase(), band: v.qso.band.to_ascii_lowercase(), time_ms: v.qso.time_ms, nuovi: v.nuovi.clone() }).collect();
    tot.bande = bande.into_values().collect();
    tot.bande.sort_by_key(|r| ordine_banda(&r.band));
    calcola(&mut tot);
    Ok((out, tot))
}

fn ordina_bande(mut v: Vec<String>) -> Vec<String> {
    v.sort_by_key(|b| ordine_banda(b));
    v
}

/// Punteggio = (punti + QTC) x moltiplicatori [x QSO validi] x fattore finale (senza moltiplicatori: i punti).
/// Con la funzione `punteggio(totali)` dello script e' lei a dare il totale (un errore la toglie e resta in `errore_script`).
fn calcola(tot: &mut Punteggio) {
    let base = tot.punti + i64::from(tot.qtc);
    let lordo = if tot.tipi.is_empty() {
        base
    } else if tot.per_banda {
        // banda per banda: punti x moltiplicatori della banda
        tot.bande.iter().map(|b| b.punti * i64::from(b.molt)).sum()
    } else {
        base * tot.molt
    };
    let lordo = if tot.per_qso { lordo * i64::from(tot.qso) } else { lordo };
    tot.punteggio = (lordo * tot.fattore_mille + 500) / 1000;
    if tot.script.trim().is_empty() || tot.errore_script.is_some() {
        return;
    }
    let t = crate::script::Totali { qso: tot.qso, doppi: tot.doppi, esclusi: tot.esclusi, punti: tot.punti, qtc: tot.qtc, molt: tot.molt, exch_sent: &tot.primo_mandato, my_dxcc: &tot.mio_dxcc, my_cont: &tot.mio_cont, paesi: &tot.paesi };
    match crate::script::ScriptContest::compila(&tot.script).and_then(|s| s.punteggio(&t)) {
        Ok(Some(p)) => tot.punteggio = p,
        Ok(None) => {}
        Err(e) => tot.errore_script = Some(e),
    }
}

/// Aggiunge i QTC (mandati e ricevuti) al punteggio: (punti + QTC) x moltiplicatori, come nel WAE.
pub fn aggiungi_qtc(tot: &mut Punteggio, qtc: u32) {
    tot.qtc = qtc;
    calcola(tot);
}

/// Il ritmo: QSO validi negli ultimi 10 minuti (riportati all'ora), nell'ultima ora e media all'ora dal primo all'ultimo QSO.
pub fn ritmo(tot: &mut Punteggio, valutati: &[Valutato], ora_ms: i64) {
    let validi: Vec<i64> = valutati.iter().filter(|v| !v.doppio && v.escluso.is_none()).map(|v| v.qso.time_ms).collect();
    let negli = |min: i64| validi.iter().filter(|t| **t > ora_ms - min * 60_000 && **t <= ora_ms).count() as u32;
    let media = match (validi.first(), validi.last()) {
        (Some(a), Some(b)) if b > a => validi.len() as f64 * 3_600_000.0 / (b - a) as f64,
        _ => validi.len() as f64,
    };
    tot.rate10 = negli(10) * 6;
    tot.rate60 = negli(60);
    tot.media_ora = (media * 10.0).round() / 10.0;
}

/// Le bande in ordine di frequenza (le sconosciute in fondo).
fn ordine_banda(b: &str) -> u32 {
    const ORDINE: [&str; 13] = ["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"];
    ORDINE.iter().position(|x| *x == b).map_or(99, |i| i as u32)
}

/// Il modo come lo vuole il Cabrillo: `CW`, `RY` (RTTY), `PH` (fonia), `DG` (gli altri digitali).
pub fn modo_cabrillo(modo: &str) -> &'static str {
    match modo.trim().to_ascii_uppercase().as_str() {
        "RTTY" => "RY",
        m => match categoria(m) {
            Categoria::Cw => "CW",
            Categoria::Phone => "PH",
            Categoria::Digital => "DG",
        },
    }
}

/// `CATEGORY-MODE` dai QSO: un solo tipo -> `CW`, `RTTY`, `SSB`, `DIGI`; piu' tipi -> `MIXED`.
fn categoria_modo(log: &[Valutato]) -> &'static str {
    let tipi: HashSet<&str> = log.iter().map(|v| modo_cabrillo(&v.qso.mode)).collect();
    if tipi.len() != 1 {
        return if tipi.is_empty() { "DIGI" } else { "MIXED" };
    }
    match tipi.into_iter().next() {
        Some("CW") => "CW",
        Some("RY") => "RTTY",
        Some("PH") => "SSB",
        _ => "DIGI",
    }
}

/// Lo scambio di una colonna: il rapporto davanti (se c'e' e lo scambio non lo contiene gia') e lo scambio; `-` se vuoto.
fn scambio(rst: &str, ex: &str) -> String {
    let (rst, ex) = (rst.trim().to_ascii_uppercase(), ex.trim().to_ascii_uppercase());
    let s = if rst.is_empty() || ex.split_whitespace().next() == Some(rst.as_str()) { ex } else { format!("{rst} {ex}") };
    let s = s.trim().to_string();
    if s.is_empty() { crate::export::MANCA.to_string() } else { s }
}

/// Il nome del contest per la riga `CONTEST:` (maiuscole, spazi in trattini).
fn nome_cabrillo(nome: &str) -> String {
    nome.split_whitespace().collect::<Vec<_>>().join("-").to_ascii_uppercase()
}

/// Il nominativo (con il suo paese `info`, e il paese della stazione `mio`) deve dare un terzo elemento dello scambio, oltre al
/// numero (sezione, club, stato)? Solo con le regole della `[Score]` e un moltiplicatore `exch` che lo riguarda; le vecchie chiavi
/// del file (`m_iMultipliers`) no, il loro scambio e' un campo solo. Con `info` della propria stazione dice se tocca a lei mandarlo.
pub fn scambio_atteso(c: &ContestTesto, q: &QsoTesto, call: &str, info: Option<&InfoPaese>, mio: Option<&InfoPaese>) -> bool {
    use crate::regole_punti::{Ctx, RegoleScore};
    if c.score.trim().is_empty() {
        return false;
    }
    let Ok(r) = RegoleScore::da_testo(&c.score) else { return false };
    crate::regole_punti::scambio_atteso(&r, &Ctx { q, call, info, mio })
}

/// Il file Cabrillo 3.0 del contest dei modi testuali. I doppi restano nel file (li toglie chi controlla i log).
/// Con i QTC (contest WAE) le righe `QTC:` seguono quelle `QSO:`.
pub fn cabrillo(c: &ContestTesto, call: &str, log: &[Valutato], tot: &Punteggio, versione: &str, qtc: &[crate::qtc::Qtc]) -> String {
    let mut out = String::new();
    let mut riga = |k: &str, v: &str| {
        if !v.trim().is_empty() {
            out.push_str(&format!("{k}: {}\r\n", v.trim()));
        }
    };
    let call = call.trim().to_ascii_uppercase();
    riga("START-OF-LOG", "3.0");
    riga("CONTEST", &nome_cabrillo(&c.name));
    riga("CALLSIGN", &call);
    riga("CATEGORY-OPERATOR", "SINGLE-OP");
    riga("CATEGORY-BAND", "ALL");
    riga("CATEGORY-MODE", categoria_modo(log));
    riga("CATEGORY-TRANSMITTER", "ONE");
    riga("CLAIMED-SCORE", &tot.punteggio.to_string());
    riga("CREATED-BY", &format!("Aurora {versione}"));
    for v in log {
        let khz = v.qso.freq_hz / 1000;
        let freq = if khz > 0 { khz.to_string() } else { geo::banda_cabrillo(&v.qso.band) };
        out.push_str(&format!(
            "QSO: {freq:>5} {} {} {} {:<13} {:<10} {:<13} {}\r\n",
            modo_cabrillo(&v.qso.mode),
            data(v.qso.time_ms),
            hhmm(v.qso.time_ms),
            call,
            scambio(&v.qso.rst_sent, &v.qso.exch_sent),
            v.qso.call.trim().to_ascii_uppercase(),
            scambio(&v.qso.rst_rcvd, &v.qso.exch_rcvd)
        ));
    }
    out.push_str(&crate::qtc::righe_cabrillo(qtc, &call));
    out.push_str("END-OF-LOG:\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elenco::leggi_dominio;

    fn q(call: &str, band: &str, mode: &str, min: i64, rcvd: &str) -> QsoTesto {
        QsoTesto {
            call: call.into(),
            band: band.into(),
            mode: mode.into(),
            freq_hz: if band == "20m" { 14_080_500 } else { 7_040_000 },
            time_ms: data_ms("20261114120000").unwrap_or(0) + min * 60_000,
            rst_sent: "599".into(),
            rst_rcvd: "599".into(),
            exch_sent: format!("{:03}", min + 1),
            exch_rcvd: rcvd.into(),
        }
    }

    fn paese(call: &str) -> Option<InfoPaese> {
        let (dxcc, cont, cq, itu) = match &call[..1] {
            "I" => ("I", "EU", 15, 28),
            "D" => ("DL", "EU", 14, 28),
            "W" | "K" => ("K", "NA", 5, 8),
            _ => return None,
        };
        Some(InfoPaese { dxcc: dxcc.into(), continente: cont.into(), cq, itu })
    }

    fn mio() -> InfoPaese {
        paese("IU3ABC").unwrap_or_default()
    }

    #[test]
    fn scambi_dal_commento() {
        assert_eq!(scambi_da_commento("bello NRS 001 NRR 599 015"), ("001".into(), "599 015".into()));
        assert_eq!(scambi_da_commento("nessuno"), (String::new(), String::new()));
        assert_eq!(scambi_da_commento("NRS MI NRR"), ("MI".into(), String::new()));
    }

    #[test]
    fn punti_per_paese_continente_dx_e_doppi() {
        let c = ContestTesto { points_my_dxcc: 1, points_my_cont: 2, points_dx: 3, ..ContestTesto::default() };
        let log = [q("IK2XYZ", "20m", "RTTY", 0, "1"), q("DL1ABC", "20m", "RTTY", 1, "2"), q("W1AW", "20m", "RTTY", 2, "3"), q("DL1ABC", "20m", "RTTY", 3, "4"), q("DL1ABC", "40m", "RTTY", 4, "5")];
        let (v, t) = valuta(&c, &log, Some(&mio()), &paese, None);
        assert_eq!((t.qso, t.doppi, t.punti, t.molt, t.punteggio), (4, 1, 1 + 2 + 3 + 2, 0, 8));
        assert!(v[3].doppio);
        assert_eq!(t.bande.len(), 2);
        assert_eq!(t.bande[0].band, "40m");
        // doppio su ogni banda
        let c2 = ContestTesto { dupe_no_band: true, ..c };
        let (_, t2) = valuta(&c2, &log, Some(&mio()), &paese, None);
        assert_eq!(t2.doppi, 2);
    }

    #[test]
    fn qtc_nel_punteggio() {
        let c = ContestTesto { multipliers: 1, ..ContestTesto::default() };
        let log = [q("DL1ABC", "20m", "RTTY", 0, ""), q("W1AW", "20m", "RTTY", 1, "")];
        let (_, mut t) = valuta(&c, &log, None, &paese, None);
        aggiungi_qtc(&mut t, 3);
        assert_eq!((t.qtc, t.punteggio), (3, (2 + 3) * 2));
    }

    #[test]
    fn senza_punti_uno_per_qso_e_modo() {
        let c = ContestTesto::default();
        let log = [q("DL1ABC", "20m", "RTTY", 0, ""), q("DL1ABC", "20m", "CW", 1, "")];
        let (_, t) = valuta(&c, &log, None, &paese, None);
        assert_eq!((t.qso, t.punti, t.punteggio), (2, 2, 2));
        let c2 = ContestTesto { dupe_no_mode: true, ..ContestTesto::default() };
        assert_eq!(valuta(&c2, &log, None, &paese, None).1.doppi, 1);
    }

    #[test]
    fn moltiplicatori_dxcc_wpx_e_zone() {
        let log = [q("DL1ABC", "20m", "RTTY", 0, ""), q("DL2XYZ", "20m", "RTTY", 1, ""), q("W1AW", "20m", "RTTY", 2, ""), q("DL1ABC", "40m", "RTTY", 3, "")];
        let c = ContestTesto { multipliers: 1, ..ContestTesto::default() };
        let (v, t) = valuta(&c, &log, None, &paese, None);
        assert_eq!((t.punti, t.molt, t.punteggio), (4, 2, 8));
        assert_eq!(v[0].nuovi, vec!["dxcc:DL".to_string()]);
        let c = ContestTesto { multipliers: 2, multipliers_option: 3, ..ContestTesto::default() };
        let (_, t) = valuta(&c, &log, None, &paese, None);
        // WPX: DL1, DL2, W1 ; zone CQ: 14, 5
        assert_eq!(t.molt, 5);
        let c = ContestTesto { multipliers: 4, ..ContestTesto::default() };
        assert_eq!(valuta(&c, &log, None, &paese, None).1.molt, 2);
    }

    #[test]
    fn moltiplicatori_per_banda_con_peso() {
        let mut c = ContestTesto { multipliers: 1, band_mult: true, ..ContestTesto::default() };
        for b in &mut c.bands {
            if b.band == "40" {
                b.on = true;
                b.weight = 2;
            }
        }
        let log = [q("DL1ABC", "20m", "RTTY", 0, ""), q("W1AW", "20m", "RTTY", 1, ""), q("DL1ABC", "40m", "RTTY", 2, "")];
        let (_, t) = valuta(&c, &log, None, &paese, None);
        // 20m: DL, K (peso 1); 40m: DL (peso 2)
        assert_eq!((t.molt, t.punteggio), (4, 12));
        assert_eq!(peso_banda(&c, "40m"), 2);
        assert_eq!(peso_banda(&c, "20m"), 1);
    }

    #[test]
    fn moltiplicatori_dominio() {
        let d = leggi_dominio("Province\nMilano = MI\nMI\nRoma = RM\n");
        let log = [q("IK2XYZ", "20m", "RTTY", 0, "599 MILANO"), q("IK2ABC", "20m", "RTTY", 1, "599 MI"), q("IK0AAA", "20m", "RTTY", 2, "599 ROMA"), q("IK0BBB", "20m", "RTTY", 3, "599 XX")];
        let c = ContestTesto { multipliers: 5, ..ContestTesto::default() };
        assert_eq!(valuta(&c, &log, None, &paese, Some(&d)).1.molt, 2);
        // senza elenco: l'ultima parola non numerica
        assert_eq!(valuta(&c, &log, None, &paese, None).1.molt, 4);
    }

    #[test]
    fn finestra_e_categorie() {
        let c = ContestTesto { start: "20201114110000".into(), end: "20201114130000".into(), valid_cw: true, valid_digital: false, ..ContestTesto::default() };
        let ora = data_ms("20261114125900").unwrap_or(0);
        let log = [q("DL1ABC", "20m", "CW", 0, ""), q("DL2ABC", "20m", "RTTY", 1, ""), q("DL3ABC", "20m", "CW", 90, "")];
        let v = qso_del_contest(&c, &log, ora);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].call, "DL1ABC");
        let (a, b) = finestra(&c, ora).unwrap_or((None, None));
        assert_eq!(a, data_ms("20261114110000"));
        assert_eq!(b, data_ms("20261114130000"));
    }

    #[test]
    fn cabrillo_righe() {
        let c = ContestTesto { name: "Test Contest RTTY".into(), ..ContestTesto::default() };
        let log = [q("DL1ABC", "20m", "RTTY", 0, "599 015"), q("W1AW", "40m", "CW", 1, "")];
        let (v, t) = valuta(&c, &log, None, &paese, None);
        let f = cabrillo(&c, "iu3abc", &v, &t, "0.11.0", &[]);
        assert!(f.starts_with("START-OF-LOG: 3.0\r\n"));
        assert!(f.contains("CONTEST: TEST-CONTEST-RTTY\r\n"));
        assert!(f.contains("CATEGORY-MODE: MIXED\r\n"));
        assert!(f.contains("CLAIMED-SCORE: 2\r\n"));
        assert!(f.contains("QSO: 14080 RY 2026-11-14 1200 IU3ABC        599 001    DL1ABC        599 015\r\n"), "{f}");
        assert!(f.contains("QSO:  7040 CW 2026-11-14 1201 IU3ABC        599 002    W1AW          599\r\n"), "{f}");
        assert!(f.ends_with("END-OF-LOG:\r\n"));
        assert_eq!(modo_cabrillo("PSK31"), "DG");
        let q = crate::qtc::Qtc { inviato: true, corrispondente: "K1ABC".into(), serie: 1, quanti: 1, time_ms: log[1].time_ms, band: "40m".into(), mode: "RTTY".into(), ora: "1200".into(), call: "DL1ABC".into(), nr: "015".into(), ..Default::default() };
        let f = cabrillo(&c, "iu3abc", &v, &t, "0.11.0", &[q]);
        assert!(f.ends_with("QTC:  7000 RY 2026-11-14 1201 K1ABC         1/1   IU3ABC        1200 DL1ABC        015\r\nEND-OF-LOG:\r\n"), "{f}");
        assert_eq!(modo_cabrillo("USB"), "PH");
    }
}
