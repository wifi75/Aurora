//! Doppi, punti, moltiplicatori e punteggio di un log di contest secondo le regole; valore di una stazione decodificata; rate e previsione.
//! Funzioni pure: il log entra, i numeri escono.

use crate::regole::Regole;
use crate::{geo, liste};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

/// Lo stato di un QSO del log del contest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stato {
    #[default]
    Valido,
    /// Manca la conferma finale: il corrispondente ha ripetuto lo scambio dopo il nostro RR73 (rischio NIL).
    Incerto,
    /// Segnato a mano come non valido: resta nel log (EDI: riga `ERROR`) ma non da' punti.
    Errore,
}

/// Un QSO del log del contest.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContestQso {
    /// Numero d'ordine nel log (da 1).
    pub n: u32,
    /// Inizio del QSO (ms UTC).
    pub time_ms: i64,
    pub call: String,
    pub band: String,
    pub mode: String,
    pub freq_hz: u64,
    #[serde(default)]
    pub grid: String,
    #[serde(default)]
    pub rst_sent: String,
    #[serde(default)]
    pub rst_rcvd: String,
    #[serde(default)]
    pub exch_sent: String,
    #[serde(default)]
    pub exch_rcvd: String,
    #[serde(default)]
    pub stato: Stato,
    /// Corretto a mano dopo la registrazione.
    #[serde(default)]
    pub modificato: bool,
}

/// Il paese DXCC di un nominativo (prefisso principale di cty.dat, `None` se non si trova).
pub type PaeseFn = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// I dati della propria stazione e le scelte che toccano il punteggio.
#[derive(Clone, Default)]
pub struct Contesto {
    pub mio_grid: String,
    /// Potenza in watt (moltiplicatore di potenza del Field Day).
    pub potenza_w: f64,
    /// Punti bonus spuntati a mano (Field Day).
    pub bonus: u32,
    /// Ricerca del paese DXCC (moltiplicatori `dxcc`); senza, quei moltiplicatori non si contano.
    pub paese: Option<PaeseFn>,
}

impl std::fmt::Debug for Contesto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Contesto").field("mio_grid", &self.mio_grid).field("potenza_w", &self.potenza_w).field("bonus", &self.bonus).field("paese", &self.paese.is_some()).finish()
    }
}

/// Un QSO con i suoi numeri.
#[derive(Clone, Debug, Serialize)]
pub struct Valutato {
    #[serde(flatten)]
    pub qso: ContestQso,
    pub doppio: bool,
    pub punti: u32,
    /// Distanza in km (0 se non si conosce).
    pub km: f64,
    /// I moltiplicatori nuovi che questo QSO ha aperto (`IO`, `JN45`, `DL1`...).
    pub molt_nuovi: Vec<String>,
    /// Lo scambio ricevuto ha la forma che il regolamento chiede.
    pub scambio_ok: bool,
    /// Perche' lo scambio non va (vuoto se va bene): sezione, stato o provincia fuori lista, classe sbagliata...
    pub scambio_nota: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TotaleBanda {
    pub banda: String,
    pub qso: u32,
    pub punti: u32,
    pub molt: u32,
    /// Ultimo QSO (ms UTC).
    pub ultimo_ms: i64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Totali {
    /// QSO validi (senza doppi ed errori).
    pub qso: u32,
    pub doppi: u32,
    pub incerti: u32,
    pub punti: u32,
    pub molt: u32,
    pub punteggio: u64,
    pub km_max: f64,
    pub per_banda: Vec<TotaleBanda>,
}

/// Il gruppo di modo per i doppi: con `stesso_modo_per_doppi` tutti i digitali sono `DG`.
fn modo_doppi(r: &Regole, modo: &str) -> String {
    if r.modi.stesso_modo_per_doppi { "DG".to_string() } else { modo.to_ascii_uppercase() }
}

/// Il locatore del corrispondente: quello del QSO o quello dentro lo scambio ricevuto.
pub fn grid_ricevuto(q: &ContestQso) -> String {
    for t in q.exch_rcvd.split_whitespace().rev() {
        if geo::grid_valido(t) {
            return t.to_ascii_uppercase();
        }
    }
    if geo::grid_valido(&q.grid) { q.grid.to_ascii_uppercase() } else { String::new() }
}

fn chiave_doppio(r: &Regole, q: &ContestQso) -> String {
    let call = q.call.to_ascii_uppercase();
    let banda = q.band.to_ascii_lowercase();
    match r.doppi.chiave.as_str() {
        "banda+modo" => format!("{call}|{banda}|{}", modo_doppi(r, &q.mode)),
        "banda+griglia" => format!("{call}|{banda}|{}", geo::quadrato(&grid_ricevuto(q)).unwrap_or_default()),
        "log" => call,
        _ => format!("{call}|{banda}"),
    }
}

/// Lo scambio ricevuto ha la forma richiesta (un locatore dove serve, la classe e la sezione al Field Day...).
pub fn scambio_valido(r: &Regole, q: &ContestQso) -> bool {
    problema_scambio(r, q).is_none()
}

/// Cosa non va nello scambio ricevuto (`None` se va bene): la forma e, dove il regolamento usa una lista ufficiale (sezioni ARRL/RAC,
/// stati e province, province olandesi), il valore.
pub fn problema_scambio(r: &Regole, q: &ContestQso) -> Option<String> {
    let x: Vec<&str> = q.exch_rcvd.split_whitespace().collect();
    let forma = |ok: bool, cosa: &str| if ok { None } else { Some(format!("scambio incompleto: serve {cosa}")) };
    match r.messaggi.attivita.as_str() {
        "na-vhf" | "arrl-digi" | "ww-digi" => forma(!grid_ricevuto(q).is_empty(), "il locatore"),
        "field-day" => {
            if x.len() != 2 {
                return forma(false, "classe e sezione");
            }
            if !liste::classe_fd_valida(x[0]) {
                return Some(format!("classe del Field Day non valida: {}", x[0]));
            }
            if !x[1].eq_ignore_ascii_case("DX") && !liste::sezione_valida(x[1]) {
                return Some(format!("sezione ARRL/RAC sconosciuta: {}", x[1]));
            }
            None
        }
        "roundup" => {
            if x.len() != 2 {
                return forma(false, "rapporto e stato, provincia o numero");
            }
            let v = x[1];
            if v.eq_ignore_ascii_case("DX") || liste::seriale_valido(v) || liste::stato_valido(v) {
                None
            } else {
                Some(format!("stato USA o provincia canadese sconosciuti: {v}"))
            }
        }
        "eu-vhf" => forma(x.len() == 2 && geo::grid_valido(x[1]), "rapporto, numero e locatore"),
        _ => {
            // messaggi normali: basta il rapporto; se il regolamento chiede il locatore deve esserci
            let vuole_grid = r.scambio.ricevuto.iter().any(|c| c == "grid4" || c == "grid6");
            if q.rst_rcvd.trim().is_empty() {
                return forma(false, "il rapporto");
            }
            if vuole_grid && grid_ricevuto(q).is_empty() && r.contest.id != "ft-challenge" {
                return forma(false, "il locatore");
            }
            // PACC: chi manda una sigla di due lettere al posto del numero deve mandare una provincia olandese
            if r.scambio.ricevuto.iter().any(|c| c == "provincia")
                && let Some(p) = sigla_provincia(&x)
                && !liste::provincia_olanda_valida(p)
            {
                return Some(format!("provincia olandese sconosciuta: {p}"));
            }
            None
        }
    }
}

/// La sigla di due lettere nello scambio ricevuto (provincia del PACC), se c'e'.
fn sigla_provincia<'a>(x: &[&'a str]) -> Option<&'a str> {
    x.iter().rev().find(|t| t.len() == 2 && t.chars().all(|c| c.is_ascii_alphabetic())).copied()
}

/// I punti di un QSO (senza guardare i doppi) e la distanza.
pub fn punti_qso(r: &Regole, ctx: &Contesto, q: &ContestQso) -> (u32, f64) {
    let suo = grid_ricevuto(q);
    let km = geo::distanza_km(&ctx.mio_grid, &suo).unwrap_or(0.0);
    let p = &r.punti;
    let punti = match p.tipo.as_str() {
        "fisso" => p.fisso.max(1),
        "per-banda" => p.tabella.iter().find(|(b, _)| b.eq_ignore_ascii_case(&q.band)).map_or(1, |(_, v)| *v),
        // punti secondo il paese DXCC (prefisso principale di cty.dat): PACC 3 con le olandesi (`PA = 3`), `fisso` con le altre
        "per-paese" => ctx
            .paese
            .as_ref()
            .and_then(|f| f(&q.call))
            .and_then(|px| p.tabella.iter().find(|(k, _)| k.eq_ignore_ascii_case(&px)).map(|(_, v)| *v))
            .unwrap_or(p.fisso.max(1)),
        "distanza" => {
            if suo.is_empty() || ctx.mio_grid.is_empty() {
                // senza locatore (FT Challenge: ZZ00) vale solo la base
                p.base.max(1)
            } else {
                let passi = km / f64::from(p.passo_km.max(1));
                let extra = if p.arrotonda == "su" { passi.ceil() } else { passi.floor() } as u32;
                p.base + extra.max(p.minimo_extra)
            }
        }
        "km" => {
            let (Some(a), Some(b)) = (geo::centro(&ctx.mio_grid), geo::centro(&suo)) else { return (0, 0.0) };
            if geo::quadrato(&ctx.mio_grid) == geo::quadrato(&suo) && p.stesso_quadrato > 0 {
                p.stesso_quadrato
            } else {
                let kpg = p.km_per_grado.unwrap_or(111.2);
                (geo::angolo_gradi(a, b) * kpg).floor() as u32 + 1
            }
        }
        _ => 1,
    };
    (punti, km)
}

/// I valori dei moltiplicatori di un QSO: (chiave dell'ambito + tipo, valore).
/// Un valore fuori dalle liste ufficiali non apre moltiplicatori.
fn molt_di(r: &Regole, ctx: &Contesto, q: &ContestQso) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for m in &r.moltiplicatori {
        let ambito = match m.ambito.as_str() {
            "per-banda" => q.band.to_ascii_lowercase(),
            "per-banda-modo" => format!("{}|{}", q.band.to_ascii_lowercase(), modo_doppi(r, &q.mode)),
            _ => String::new(),
        };
        let x: Vec<&str> = q.exch_rcvd.split_whitespace().collect();
        let valore = match m.tipo.as_str() {
            "campo2" => geo::campo(&grid_ricevuto(q)).filter(|c| c != "ZZ"),
            "grid4" => geo::quadrato(&grid_ricevuto(q)),
            "prefisso" => Some(geo::prefisso_wpx(&q.call)).filter(|p| !p.is_empty()),
            "sezione" => x.last().filter(|s| liste::sezione_valida(s)).map(|s| s.to_ascii_uppercase()),
            "stato" => x.last().filter(|s| liste::stato_valido(s)).map(|s| s.to_ascii_uppercase()),
            "provincia-pa" => sigla_provincia(&x).filter(|s| liste::provincia_olanda_valida(s)).map(|s| s.to_ascii_uppercase()),
            // paesi DXCC: dove lo scambio porta gia' uno stato o una provincia (Roundup) quelli contano al posto del paese
            "dxcc" => {
                let ha_stato = x.last().is_some_and(|s| liste::stato_valido(s));
                // USA (K) e Canada (VE) non contano come paesi nemmeno con lo stato sbagliato
                if ha_stato { None } else { ctx.paese.as_ref().and_then(|f| f(&q.call)).filter(|p| p != "K" && p != "VE") }
            }
            _ => None,
        };
        if let Some(v) = valore {
            out.push((format!("{}|{ambito}", m.tipo), v));
        }
    }
    out
}

/// Il moltiplicatore di potenza (Field Day) per i watt dati.
pub fn molt_potenza(r: &Regole, watt: f64) -> u32 {
    let mut soglie = r.punteggio.potenza.clone();
    soglie.sort_by(|a, b| a.fino_a_w.total_cmp(&b.fino_a_w));
    soglie.iter().find(|s| watt <= s.fino_a_w).map_or(1, |s| s.molt)
}

fn punteggio(r: &Regole, ctx: &Contesto, punti: u32, molt: u32) -> u64 {
    let (p, m) = (u64::from(punti), u64::from(molt));
    match r.punteggio.formula.as_str() {
        "punti" => p,
        "punti*potenza+bonus" => p * u64::from(molt_potenza(r, ctx.potenza_w)) + u64::from(ctx.bonus),
        _ => {
            if r.moltiplicatori.is_empty() {
                p
            } else {
                p * m
            }
        }
    }
}

/// Il log valutato in ordine e i totali.
pub fn valuta_log(r: &Regole, ctx: &Contesto, log: &[ContestQso]) -> (Vec<Valutato>, Totali) {
    let mut visti: HashSet<String> = HashSet::new();
    let mut molt: BTreeSet<(String, String)> = BTreeSet::new();
    let mut tot = Totali::default();
    let mut bande: BTreeMap<String, TotaleBanda> = BTreeMap::new();
    let mut out = Vec::with_capacity(log.len());
    let mut ordinato: Vec<&ContestQso> = log.iter().collect();
    ordinato.sort_by_key(|q| (q.time_ms, q.n));
    for q in ordinato {
        let valido = q.stato != Stato::Errore && r.banda_ammessa(&q.band);
        let chiave = chiave_doppio(r, q);
        let doppio = valido && !visti.insert(chiave);
        let (mut punti, km) = punti_qso(r, ctx, q);
        let mut nuovi = Vec::new();
        if !valido || doppio {
            punti = 0;
        } else {
            for (k, v) in molt_di(r, ctx, q) {
                if molt.insert((k, v.clone())) {
                    nuovi.push(v);
                }
            }
        }
        let banda = bande.entry(q.band.to_ascii_lowercase()).or_insert_with(|| TotaleBanda { banda: q.band.to_ascii_lowercase(), ..Default::default() });
        if valido && !doppio {
            tot.qso += 1;
            tot.punti += punti;
            tot.km_max = tot.km_max.max(km);
            banda.qso += 1;
            banda.punti += punti;
            banda.molt += nuovi.len() as u32;
        }
        banda.ultimo_ms = banda.ultimo_ms.max(q.time_ms);
        if doppio {
            tot.doppi += 1;
        }
        if q.stato == Stato::Incerto {
            tot.incerti += 1;
        }
        let nota = problema_scambio(r, q);
        out.push(Valutato { qso: q.clone(), doppio, punti, km, molt_nuovi: nuovi, scambio_ok: nota.is_none(), scambio_nota: nota.unwrap_or_default() });
    }
    tot.molt = molt.len() as u32;
    tot.punteggio = punteggio(r, ctx, tot.punti, tot.molt);
    tot.per_banda = bande.into_values().collect();
    (out, tot)
}

/// Quanto vale chiamare adesso una stazione.
#[derive(Clone, Debug, Serialize)]
pub struct Valore {
    pub call: String,
    pub doppio: bool,
    pub punti: u32,
    pub km: f64,
    pub molt_nuovi: Vec<String>,
    /// Bande ammesse su cui la stazione non e' ancora stata lavorata.
    pub bande_utili: Vec<String>,
    /// Banda o modo non ammessi dal regolamento.
    pub fuori_regola: bool,
}

/// Il valore di una stazione decodificata (`grid` puo' essere vuoto) su `banda` e `modo`, rispetto al log.
pub fn valore(r: &Regole, ctx: &Contesto, log: &[ContestQso], call: &str, grid: &str, banda: &str, modo: &str) -> Valore {
    let lista = [(call.to_string(), grid.to_string(), banda.to_string(), modo.to_string())];
    valori(r, ctx, log, &lista).pop().unwrap_or(Valore { call: call.to_ascii_uppercase(), doppio: false, punti: 0, km: 0.0, molt_nuovi: Vec::new(), bande_utili: Vec::new(), fuori_regola: true })
}

/// Il valore di tante stazioni `(nominativo, locatore, banda, modo)`: doppi e moltiplicatori aperti si calcolano una volta sola sul log
/// (il consiglio di banda e la mappa valutano centinaia di stazioni a ogni giro).
pub fn valori(r: &Regole, ctx: &Contesto, log: &[ContestQso], stazioni: &[(String, String, String, String)]) -> Vec<Valore> {
    let validi: Vec<&ContestQso> = log.iter().filter(|q| q.stato != Stato::Errore).collect();
    let chiavi: HashSet<String> = validi.iter().map(|q| chiave_doppio(r, q)).collect();
    let mut aperti: BTreeSet<(String, String)> = BTreeSet::new();
    for q in &validi {
        aperti.extend(molt_di(r, ctx, q));
    }
    stazioni
        .iter()
        .map(|(call, grid, banda, modo)| {
            let prova = ContestQso { call: call.to_ascii_uppercase(), band: banda.to_ascii_lowercase(), mode: modo.to_ascii_uppercase(), grid: grid.to_string(), exch_rcvd: grid.to_string(), rst_rcvd: "-10".into(), ..Default::default() };
            let doppio = chiavi.contains(&chiave_doppio(r, &prova));
            let fuori = !r.banda_ammessa(banda) || !r.modo_ammesso(modo);
            let (punti, km) = punti_qso(r, ctx, &prova);
            let molt_nuovi = if doppio || fuori { Vec::new() } else { molt_di(r, ctx, &prova).into_iter().filter(|m| !aperti.contains(m)).map(|(_, v)| v).collect() };
            let bande_utili = r
                .bande
                .ammesse
                .iter()
                .filter(|b| {
                    let altra = ContestQso { band: b.to_ascii_lowercase(), ..prova.clone() };
                    !chiavi.contains(&chiave_doppio(r, &altra))
                })
                .cloned()
                .collect();
            Valore { call: prova.call, doppio, punti: if doppio || fuori { 0 } else { punti }, km, molt_nuovi, bande_utili, fuori_regola: fuori }
        })
        .collect()
}

/// I moltiplicatori gia' presi su una banda (per la mappa dei mancanti): (tipo, valori).
pub fn molt_lavorati(r: &Regole, ctx: &Contesto, log: &[ContestQso], banda: &str) -> Vec<(String, Vec<String>)> {
    let (valutati, _) = valuta_log(r, ctx, log);
    r.moltiplicatori
        .iter()
        .map(|m| {
            let mut set = BTreeSet::new();
            for v in valutati.iter().filter(|v| !v.doppio && v.qso.stato != Stato::Errore) {
                if m.ambito == "per-log" || v.qso.band.eq_ignore_ascii_case(banda) {
                    for (k, val) in molt_di(r, ctx, &v.qso) {
                        if k.starts_with(&format!("{}|", m.tipo)) {
                            set.insert(val);
                        }
                    }
                }
            }
            (m.tipo.clone(), set.into_iter().collect())
        })
        .collect()
}

/// Ritmo: QSO all'ora sugli ultimi 10 e 60 minuti e QSO per ora nelle ultime `ore` ore (dalla piu' vecchia).
#[derive(Clone, Debug, Default, Serialize)]
pub struct Ritmo {
    pub rate10: u32,
    pub rate60: u32,
    pub per_ora: Vec<u32>,
}

pub fn ritmo(valutati: &[Valutato], ora_ms: i64, ore: usize) -> Ritmo {
    let validi: Vec<i64> = valutati.iter().filter(|v| !v.doppio && v.qso.stato != Stato::Errore).map(|v| v.qso.time_ms).collect();
    let negli = |min: i64| validi.iter().filter(|t| **t > ora_ms - min * 60_000 && **t <= ora_ms).count() as u32;
    let mut per_ora = vec![0u32; ore];
    for t in &validi {
        let fa = (ora_ms - t) / 3_600_000;
        if (0..ore as i64).contains(&fa) {
            per_ora[ore - 1 - fa as usize] += 1;
        }
    }
    Ritmo { rate10: negli(10) * 6, rate60: negli(60), per_ora }
}

/// Ore di attivita' usate: la somma dei tratti fra QSO successivi piu' vicini della pausa minima (un tratto piu' lungo e' una pausa).
pub fn ore_attive(valutati: &[Valutato], pausa_min: u32) -> f64 {
    let mut t: Vec<i64> = valutati.iter().filter(|v| v.qso.stato != Stato::Errore).map(|v| v.qso.time_ms).collect();
    t.sort_unstable();
    let soglia = i64::from(pausa_min.max(1)) * 60_000;
    let ms: i64 = t.windows(2).map(|w| w[1] - w[0]).filter(|d| *d < soglia).sum();
    ms as f64 / 3_600_000.0
}

/// Il punteggio previsto a fine gara: i QSO crescono con il rate dell'ultima ora per le ore che restano, i punti con la media per QSO,
/// i moltiplicatori piu' lentamente (radice del rapporto fra QSO previsti e fatti).
pub fn previsto(r: &Regole, ctx: &Contesto, tot: &Totali, rate60: u32, ore_restanti: f64) -> u64 {
    if tot.qso == 0 || ore_restanti <= 0.0 {
        return tot.punteggio;
    }
    let qso_in_piu = f64::from(rate60) * ore_restanti;
    let media = f64::from(tot.punti) / f64::from(tot.qso);
    let qso_fine = f64::from(tot.qso) + qso_in_piu;
    let punti = (f64::from(tot.punti) + qso_in_piu * media).round() as u32;
    let molt = (f64::from(tot.molt) * (qso_fine / f64::from(tot.qso)).sqrt()).round() as u32;
    punteggio(r, ctx, punti, molt.max(tot.molt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regole::carica;

    fn regole(id: &str) -> Regole {
        carica(None).0.into_iter().find(|r| r.contest.id == id).expect("regole")
    }

    fn qso(n: u32, call: &str, band: &str, grid: &str) -> ContestQso {
        ContestQso { n, time_ms: i64::from(n) * 60_000, call: call.into(), band: band.into(), mode: "FT8".into(), grid: grid.into(), exch_rcvd: grid.into(), rst_rcvd: "-10".into(), ..Default::default() }
    }

    /// WW Digi: 1 + floor(km/3000); doppi per banda; campi per banda.
    #[test]
    fn ww_digi_punti_doppi_e_campi() {
        let r = regole("ww-digi");
        let ctx = Contesto { mio_grid: "FN42".into(), ..Default::default() };
        let log = vec![qso(1, "DL1ABC", "20m", "JO62"), qso(2, "DL1ABC", "20m", "JO62"), qso(3, "DL1ABC", "40m", "JO62"), qso(4, "W1AW", "20m", "FN31")];
        let (v, t) = valuta_log(&r, &ctx, &log);
        assert_eq!(v[0].punti, 3, "circa 6200 km: 1 + 2");
        assert!(v[1].doppio);
        assert_eq!(v[1].punti, 0);
        assert!(!v[2].doppio, "altra banda");
        assert_eq!(v[3].punti, 1);
        assert_eq!(t.qso, 3);
        assert_eq!(t.molt, 3, "JO su 20 e 40 m, FN su 20 m");
        assert_eq!(t.punteggio, u64::from(t.punti) * 3);
    }

    /// ARRL Digi: 1 + ceil(km/500) con almeno 1, niente moltiplicatori.
    #[test]
    fn arrl_digi_punti() {
        let r = regole("arrl-digi");
        let ctx = Contesto { mio_grid: "FN42".into(), ..Default::default() };
        let (v, t) = valuta_log(&r, &ctx, &[qso(1, "W1AW", "20m", "FN42")]);
        assert_eq!(v[0].punti, 2, "stesso quadrato: almeno 1 punto di distanza");
        assert_eq!(t.punteggio, 2);
    }

    /// IARU MGM: stesso quadrato 50 punti; i doppi restano senza punti.
    #[test]
    fn iaru_km() {
        let r = regole("iaru-r1-50-mgm");
        let ctx = Contesto { mio_grid: "IO91".into(), ..Default::default() };
        let mut a = qso(1, "G4ABC", "6m", "IO91");
        a.band = "6m".into();
        let (v, _) = valuta_log(&r, &ctx, &[a.clone(), ContestQso { n: 2, time_ms: 120_000, ..a }]);
        assert_eq!(v[0].punti, 50);
        assert!(v[1].doppio);
    }

    /// Field Day: 2 punti, moltiplicatore di potenza e bonus.
    #[test]
    fn field_day_potenza() {
        let r = regole("arrl-fd");
        let ctx = Contesto { mio_grid: "IO91".into(), potenza_w: 100.0, bonus: 50, ..Default::default() };
        let mut q = qso(1, "K1ABC", "20m", "");
        q.exch_rcvd = "2A EMA".into();
        let (_, t) = valuta_log(&r, &ctx, &[q]);
        assert_eq!(t.punteggio, 2 * 2 + 50);
        assert_eq!(molt_potenza(&r, 5.0), 5);
        assert_eq!(molt_potenza(&r, 500.0), 1);
    }

    /// Il valore di una stazione: doppio, punti e moltiplicatori nuovi.
    #[test]
    fn valore_stazione() {
        let r = regole("ww-digi");
        let ctx = Contesto { mio_grid: "IO91".into(), ..Default::default() };
        let log = vec![qso(1, "DL1ABC", "20m", "JO62")];
        let v = valore(&r, &ctx, &log, "DL1ABC", "JO62", "20m", "FT8");
        assert!(v.doppio);
        assert!(v.bande_utili.contains(&"40m".to_string()));
        let w = valore(&r, &ctx, &log, "JA1ABC", "PM95", "20m", "FT8");
        assert_eq!(w.molt_nuovi, vec!["PM".to_string()]);
        assert!(w.punti >= 3);
    }

    #[test]
    fn rate_e_ore() {
        let r = regole("ww-digi");
        let ctx = Contesto { mio_grid: "IO91".into(), ..Default::default() };
        let log: Vec<ContestQso> = (1..=12).map(|n| qso(n, &format!("K{n}ABC"), "20m", "FN42")).collect();
        let (v, _) = valuta_log(&r, &ctx, &log);
        let rt = ritmo(&v, 12 * 60_000, 3);
        assert_eq!(rt.rate60, 12);
        assert_eq!(rt.rate10, 10 * 6);
        assert!((ore_attive(&v, 60) - 11.0 / 60.0).abs() < 1e-9);
    }
}
