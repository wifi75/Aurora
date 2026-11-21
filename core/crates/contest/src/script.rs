//! Gli script dei casi speciali di un contest, sezione `[Script]` del file `.contest` (`docs/contest-file.md`), in Rhai.
//!
//! Funzioni facoltative chiamate dal motore del punteggio:
//! - `punti(qso)` -> intero: sostituisce i punti del QSO;
//! - `moltiplicatori(qso)` -> elenco di chiavi (`["dxcc:I@20"]`): sostituisce i moltiplicatori di `[Score]`;
//! - `ammesso(qso)` -> `true`, `false` o `[false, "motivo"]`;
//! - `punteggio(totali)` -> intero: il punteggio finale;
//! - `turno(qso)` -> testo: il turno del QSO (la stessa stazione in un altro turno non e' un doppio).
//!
//! Sicurezza: niente moduli (`import`), niente `eval`, nessuna funzione di file, rete o processi (Rhai non ne ha); `print` e `debug`
//! non scrivono niente. Limiti di operazioni, profondita', chiamate e memoria: uno script sbagliato o con un ciclo infinito si ferma
//! con un errore e il motore torna alla sezione `[Score]`.

use crate::punti_testo::{InfoPaese, QsoTesto};
use crate::regole_punti::coordinate_raem;
use rhai::{AST, Array, CallFnOptions, Dynamic, Engine, INT, Map, Scope};

/// Operazioni massime per ogni chiamata di funzione dello script.
pub const MAX_OPERAZIONI: u64 = 200_000;

/// Uno script compilato.
pub struct ScriptContest {
    engine: Engine,
    ast: AST,
}

fn motore() -> Engine {
    let mut e = Engine::new();
    e.set_max_operations(MAX_OPERAZIONI);
    e.set_max_call_levels(32);
    e.set_max_expr_depths(64, 32);
    e.set_max_string_size(10_000);
    e.set_max_array_size(10_000);
    e.set_max_map_size(1_000);
    e.set_max_functions(200);
    e.set_max_variables(500);
    e.disable_symbol("eval");
    e.on_print(|_| {});
    e.on_debug(|_, _, _| {});
    // coords("001 57N85E") -> [57, 85], () se lo scambio non ha coordinate
    e.register_fn("coords", |s: &str| -> Dynamic {
        match coordinate_raem(s) {
            Some((a, b)) => Dynamic::from_array(vec![Dynamic::from_int(a), Dynamic::from_int(b)]),
            None => Dynamic::UNIT,
        }
    });
    e
}

/// Le informazioni per lo script di un QSO.
pub struct QsoScript<'a> {
    pub q: &'a QsoTesto,
    pub call: &'a str,
    pub info: Option<&'a InfoPaese>,
    pub mio: Option<&'a InfoPaese>,
    pub wpx: String,
}

fn s(v: &str) -> Dynamic {
    Dynamic::from(v.to_string())
}

impl QsoScript<'_> {
    fn mappa(&self) -> Map {
        let mut m = Map::new();
        let q = self.q;
        let ora = crate::calendario::hhmm(q.time_ms);
        m.insert("callsign".into(), s(self.call));
        m.insert("band".into(), s(&crate::regole_punti::banda_corta(&q.band)));
        m.insert("mode".into(), s(&q.mode.trim().to_ascii_uppercase()));
        m.insert("time".into(), Dynamic::from_int(q.time_ms));
        m.insert("hour".into(), s(&ora));
        m.insert("rst_sent".into(), s(&q.rst_sent));
        m.insert("rst_rcvd".into(), s(&q.rst_rcvd));
        m.insert("exch_sent".into(), s(&q.exch_sent));
        m.insert("exch_rcvd".into(), s(&q.exch_rcvd));
        let i = self.info.cloned().unwrap_or_default();
        m.insert("dxcc".into(), s(&i.dxcc));
        m.insert("cont".into(), s(&i.continente));
        m.insert("wae".into(), s(&i.wae));
        m.insert("wae_cont".into(), s(&i.wae_cont));
        m.insert("cq".into(), Dynamic::from_int(INT::from(i.cq)));
        m.insert("itu".into(), Dynamic::from_int(INT::from(i.itu)));
        m.insert("wpx".into(), s(&self.wpx));
        let mi = self.mio.cloned().unwrap_or_default();
        m.insert("my_dxcc".into(), s(&mi.dxcc));
        m.insert("my_cont".into(), s(&mi.continente));
        m.insert("my_wae".into(), s(&mi.wae));
        m.insert("my_wae_cont".into(), s(&mi.wae_cont));
        m.insert("my_cq".into(), Dynamic::from_int(INT::from(mi.cq)));
        m.insert("my_itu".into(), Dynamic::from_int(INT::from(mi.itu)));
        m
    }
}

/// I totali passati a `punteggio(totali)`.
pub struct Totali<'a> {
    pub qso: u32,
    pub doppi: u32,
    pub esclusi: u32,
    pub punti: i64,
    pub qtc: u32,
    pub molt: i64,
    /// Lo scambio mandato nel primo QSO (per le coordinate della propria stazione).
    pub exch_sent: &'a str,
    /// Il paese e il continente della propria stazione.
    pub my_dxcc: &'a str,
    pub my_cont: &'a str,
    /// I QSO validi per paese.
    pub paesi: &'a std::collections::BTreeMap<String, PerPaese>,
}

/// I QSO validi con un paese (campo `paesi` dei totali).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerPaese {
    pub qso: u32,
    pub punti: i64,
    pub continente: String,
    /// Le bande dove il paese e' stato lavorato.
    pub bande: std::collections::BTreeSet<String>,
}

impl ScriptContest {
    /// Compila lo script (`Err` con il messaggio di Rhai, riga e colonna).
    pub fn compila(testo: &str) -> Result<ScriptContest, String> {
        let engine = motore();
        let ast = engine.compile(testo).map_err(|e| format!("script: {e}"))?;
        Ok(ScriptContest { engine, ast })
    }

    /// Lo script definisce la funzione `nome` con un parametro?
    pub fn ha(&self, nome: &str) -> bool {
        self.ast.iter_functions().any(|f| f.name == nome && f.params.len() == 1)
    }

    fn chiama(&self, nome: &str, arg: Dynamic) -> Result<Dynamic, String> {
        let mut scope = Scope::new();
        self.engine
            .call_fn_with_options::<Dynamic>(CallFnOptions::new().eval_ast(false), &mut scope, &self.ast, nome, (arg,))
            .map_err(|e| format!("script, {nome}(): {e}"))
    }

    fn intero(nome: &str, v: Dynamic) -> Result<i64, String> {
        if let Ok(n) = v.as_int() {
            return Ok(n);
        }
        if let Ok(f) = v.as_float() {
            return Ok(f.round() as i64);
        }
        Err(format!("script, {nome}(): serve un numero intero, non {}", v.type_name()))
    }

    /// `punti(qso)`, se c'e'.
    pub fn punti(&self, x: &QsoScript) -> Result<Option<i64>, String> {
        if !self.ha("punti") {
            return Ok(None);
        }
        let v = self.chiama("punti", Dynamic::from_map(x.mappa()))?;
        Self::intero("punti", v).map(Some)
    }

    /// `moltiplicatori(qso)`, se c'e': le chiavi in maiuscolo, senza le vuote.
    pub fn moltiplicatori(&self, x: &QsoScript) -> Result<Option<Vec<String>>, String> {
        if !self.ha("moltiplicatori") {
            return Ok(None);
        }
        let v = self.chiama("moltiplicatori", Dynamic::from_map(x.mappa()))?;
        if v.is_unit() {
            return Ok(Some(Vec::new()));
        }
        let a: Array = v.try_cast::<Array>().ok_or_else(|| "script, moltiplicatori(): serve un elenco di chiavi".to_string())?;
        Ok(Some(a.into_iter().map(|k| k.to_string().trim().to_ascii_uppercase()).filter(|k| !k.is_empty()).collect()))
    }

    /// `ammesso(qso)`, se c'e': `Ok(Some(Err(motivo)))` per un QSO non ammesso.
    pub fn ammesso(&self, x: &QsoScript) -> Result<Option<Result<(), String>>, String> {
        if !self.ha("ammesso") {
            return Ok(None);
        }
        let v = self.chiama("ammesso", Dynamic::from_map(x.mappa()))?;
        if let Ok(b) = v.as_bool() {
            return Ok(Some(if b { Ok(()) } else { Err("script".into()) }));
        }
        if let Some(a) = v.try_cast::<Array>()
            && let Some(Ok(b)) = a.first().map(Dynamic::as_bool)
        {
            let motivo = a.get(1).map(|m| m.to_string()).filter(|m| !m.trim().is_empty()).unwrap_or_else(|| "script".into());
            return Ok(Some(if b { Ok(()) } else { Err(motivo) }));
        }
        Err("script, ammesso(): serve true, false o [false, \"motivo\"]".into())
    }

    /// `turno(qso)`, se c'e': il turno del QSO, che entra nella chiave dei doppi.
    pub fn turno(&self, x: &QsoScript) -> Result<Option<String>, String> {
        if !self.ha("turno") {
            return Ok(None);
        }
        let v = self.chiama("turno", Dynamic::from_map(x.mappa()))?;
        Ok(Some(if v.is_unit() { String::new() } else { v.to_string() }))
    }

    /// `punteggio(totali)`, se c'e'.
    pub fn punteggio(&self, t: &Totali) -> Result<Option<i64>, String> {
        if !self.ha("punteggio") {
            return Ok(None);
        }
        let mut m = Map::new();
        m.insert("qso".into(), Dynamic::from_int(INT::from(t.qso)));
        m.insert("doppi".into(), Dynamic::from_int(INT::from(t.doppi)));
        m.insert("esclusi".into(), Dynamic::from_int(INT::from(t.esclusi)));
        m.insert("punti".into(), Dynamic::from_int(t.punti));
        m.insert("qtc".into(), Dynamic::from_int(INT::from(t.qtc)));
        m.insert("molt".into(), Dynamic::from_int(t.molt));
        m.insert("exch_sent".into(), s(t.exch_sent));
        m.insert("my_dxcc".into(), s(t.my_dxcc));
        m.insert("my_cont".into(), s(t.my_cont));
        let mut paesi = Map::new();
        for (k, p) in t.paesi {
            let mut e = Map::new();
            e.insert("qso".into(), Dynamic::from_int(INT::from(p.qso)));
            e.insert("punti".into(), Dynamic::from_int(p.punti));
            e.insert("cont".into(), s(&p.continente));
            e.insert("bande".into(), Dynamic::from_int(p.bande.len() as INT));
            paesi.insert(k.as_str().into(), Dynamic::from_map(e));
        }
        m.insert("paesi".into(), Dynamic::from_map(paesi));
        let v = self.chiama("punteggio", Dynamic::from_map(m))?;
        Self::intero("punteggio", v).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso(call: &str, rcvd: &str) -> QsoTesto {
        QsoTesto { call: call.into(), band: "20m".into(), mode: "RTTY".into(), exch_sent: "001 45N12E".into(), exch_rcvd: rcvd.into(), ..QsoTesto::default() }
    }

    fn x<'a>(q: &'a QsoTesto, info: Option<&'a InfoPaese>) -> QsoScript<'a> {
        QsoScript { q, call: &q.call, info, mio: None, wpx: String::new() }
    }

    #[test]
    fn script_che_calcola() {
        let sc = ScriptContest::compila(
            r#"
            fn punti(qso) { if qso.cont == "EU" { 1 } else { 3 } }
            fn moltiplicatori(qso) { ["dxcc:" + qso.dxcc + "@" + qso.band] }
            fn ammesso(qso) { if qso.band == "160" { [false, "band"] } else { true } }
            fn punteggio(t) { t.punti * t.molt + coords(t.exch_sent)[0] }
            "#,
        );
        let Ok(sc) = sc else { panic!("non compila") };
        let i = InfoPaese { dxcc: "DL".into(), continente: "EU".into(), cq: 14, itu: 28, ..InfoPaese::default() };
        let q = qso("DL1ABC", "002");
        assert_eq!(sc.punti(&x(&q, Some(&i))), Ok(Some(1)));
        assert_eq!(sc.moltiplicatori(&x(&q, Some(&i))), Ok(Some(vec!["DXCC:DL@20".to_string()])));
        assert_eq!(sc.ammesso(&x(&q, Some(&i))), Ok(Some(Ok(()))));
        let q160 = QsoTesto { band: "160m".into(), ..q.clone() };
        assert_eq!(sc.ammesso(&x(&q160, Some(&i))), Ok(Some(Err("band".to_string()))));
        let paesi = std::collections::BTreeMap::new();
        let t = Totali { qso: 1, doppi: 0, esclusi: 0, punti: 10, qtc: 0, molt: 2, exch_sent: "001 45N12E", my_dxcc: "I", my_cont: "EU", paesi: &paesi };
        assert_eq!(sc.punteggio(&t), Ok(Some(65)));
    }

    #[test]
    fn senza_funzioni_nessun_effetto() {
        let Ok(sc) = ScriptContest::compila("let a = 1;") else { panic!("non compila") };
        let q = qso("DL1ABC", "002");
        assert_eq!(sc.punti(&x(&q, None)), Ok(None));
        assert_eq!(sc.moltiplicatori(&x(&q, None)), Ok(None));
    }

    #[test]
    fn script_con_errore() {
        assert!(ScriptContest::compila("fn punti(qso) { 1 + }").is_err());
        let Ok(sc) = ScriptContest::compila("fn punti(qso) { qso.nessuno.campo + 1 }") else { panic!("non compila") };
        let q = qso("DL1ABC", "002");
        let e = sc.punti(&x(&q, None));
        assert!(e.as_ref().is_err_and(|m| m.starts_with("script, punti()")), "{e:?}");
        let Ok(sc) = ScriptContest::compila(r#"fn punti(qso) { "tre" }"#) else { panic!("non compila") };
        assert!(sc.punti(&x(&q, None)).is_err());
    }

    #[test]
    fn ciclo_infinito_fermato() {
        let Ok(sc) = ScriptContest::compila("fn punti(qso) { let n = 0; loop { n += 1; } }") else { panic!("non compila") };
        let q = qso("DL1ABC", "002");
        let e = sc.punti(&x(&q, None));
        assert!(e.is_err(), "{e:?}");
        // ricorsione senza fine: fermata dal limite delle chiamate
        let Ok(sc) = ScriptContest::compila("fn giu(n) { giu(n + 1) } fn punti(qso) { giu(0) }") else { panic!("non compila") };
        assert!(sc.punti(&x(&q, None)).is_err());
    }

    #[test]
    fn niente_file_ne_eval() {
        assert!(ScriptContest::compila(r#"import "x" as y; fn punti(q) { 1 }"#).is_err());
        assert!(ScriptContest::compila(r#"fn punti(q) { eval("1") }"#).is_err());
    }
}
