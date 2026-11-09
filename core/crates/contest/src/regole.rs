//! Le regole di un contest: lettura e controllo dei file TOML (formato in `docs/piano-contest.md`, sezione 4).
//!
//! I file distribuiti con Aurora (`core/crates/contest/regole`) sono dentro il programma; quelli nella cartella dell'utente (`<dati>/contest/regole/*.toml`) li
//! sostituiscono (stesso `id`) o se ne aggiungono. Un campo sconosciuto o un valore fuori dall'elenco e' un errore con un messaggio chiaro.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// I file di regole distribuiti con Aurora.
const BUILTIN: &[(&str, &str)] = &[
    ("ww-digi", include_str!("../regole/ww-digi.toml")),
    ("arrl-digi", include_str!("../regole/arrl-digi.toml")),
    ("arrl-fd", include_str!("../regole/arrl-fd.toml")),
    ("ft-challenge", include_str!("../regole/ft-challenge.toml")),
    ("arrl-vhf-jan", include_str!("../regole/arrl-vhf-jan.toml")),
    ("arrl-vhf-jun", include_str!("../regole/arrl-vhf-jun.toml")),
    ("arrl-vhf-sep", include_str!("../regole/arrl-vhf-sep.toml")),
    ("cq-ww-vhf-digi", include_str!("../regole/cq-ww-vhf-digi.toml")),
    ("iaru-r1-50-mgm", include_str!("../regole/iaru-r1-50-mgm.toml")),
    ("iaru-r1-70-mgm", include_str!("../regole/iaru-r1-70-mgm.toml")),
    ("eu-vhf", include_str!("../regole/eu-vhf.toml")),
    ("ari-iac-ft8-144", include_str!("../regole/ari-iac-ft8-144.toml")),
    ("ari-iac-ft8-432", include_str!("../regole/ari-iac-ft8-432.toml")),
    ("darc-ft4", include_str!("../regole/darc-ft4.toml")),
    ("rsgb-ft4", include_str!("../regole/rsgb-ft4.toml")),
    ("paccdigi", include_str!("../regole/paccdigi.toml")),
    ("csmi-marconi-speed", include_str!("../regole/csmi-marconi-speed.toml")),
    ("eu-ft8club-ft8", include_str!("../regole/eu-ft8club-ft8.toml")),
    ("eu-ft8club-ft4", include_str!("../regole/eu-ft8club-ft4.toml")),
    ("ybdxpi-ft8", include_str!("../regole/ybdxpi-ft8.toml")),
    ("ft8activity-eu", include_str!("../regole/ft8activity-eu.toml")),
    ("ft8activity-na", include_str!("../regole/ft8activity-na.toml")),
    ("nccc-ft4", include_str!("../regole/nccc-ft4.toml")),
    ("personalizzato", include_str!("../regole/personalizzato.toml")),
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Regole {
    pub contest: Identita,
    #[serde(default)]
    pub sintesi: Sintesi,
    pub calendario: Calendario,
    pub bande: Bande,
    pub modi: Modi,
    pub messaggi: Messaggi,
    pub scambio: Scambio,
    pub doppi: Doppi,
    pub punti: Punti,
    #[serde(default)]
    pub moltiplicatori: Vec<Moltiplicatore>,
    pub punteggio: Punteggio,
    #[serde(default)]
    pub categorie: Vec<Categoria>,
    /// Le scelte che il regolamento lascia a chi gareggia oltre alla categoria (potenza, Assisted, classe del Field Day).
    #[serde(default)]
    pub opzioni_categoria: OpzioniCategoria,
    #[serde(default)]
    pub restrizioni: Restrizioni,
    pub export: Export,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Identita {
    pub id: String,
    pub nome: String,
    #[serde(default)]
    pub sponsor: String,
    pub edizione: u32,
    #[serde(default)]
    pub url_regole: String,
    #[serde(default)]
    pub verificato_il: String,
    /// `hf`, `vhf`, `field-day`, `attivita`.
    pub famiglia: String,
    /// Campi da ricontrollare sul regolamento dell'anno.
    #[serde(default)]
    pub da_verificare: Vec<String>,
}

/// Il regolamento in breve, in italiano e in inglese (le altre lingue mostrano l'inglese).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sintesi {
    #[serde(default)]
    pub it: String,
    #[serde(default)]
    pub en: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Calendario {
    /// `ultimo-weekend-completo:agosto`, `terzo-sabato:aprile`, `secondo-martedi:febbraio,maggio`, `primo-mercoledi:tutti`, `sempre`.
    pub regola: String,
    /// `sab 12:00` (giorni del fine settimana: sab, dom, lun) oppure `+0 19:00` (giorni dopo il giorno della regola).
    pub inizio: String,
    pub fine: String,
    /// Orari di novembre-marzo, se diversi (ARI).
    #[serde(default)]
    pub inverno: Vec<String>,
    /// Date fissate per anno (`2026 = ["2026-07-18T14:00Z", "2026-07-19T14:00Z"]`): valgono al posto della regola.
    #[serde(default)]
    pub date_fisse: BTreeMap<String, Vec<String>>,
    /// Ore massime di attivita' per chi opera da solo.
    #[serde(default)]
    pub ore_massime: Option<f64>,
    /// Pausa minima perche' conti come pausa, in minuti.
    #[serde(default)]
    pub pausa_minima_min: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Bande {
    pub ammesse: Vec<String>,
    /// Frequenze di partenza in kHz per banda e modo (`"20m" = { FT8 = 14090.0, FT4 = 14080.0 }`).
    #[serde(default)]
    pub frequenze: BTreeMap<String, BTreeMap<String, f64>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Modi {
    pub ammessi: Vec<String>,
    /// FT8 e FT4 (e gli altri digitali) contano come un modo solo per i doppi.
    #[serde(default = "si")]
    pub stesso_modo_per_doppi: bool,
}

fn si() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Messaggi {
    /// `standard`, `na-vhf`, `arrl-digi`, `ww-digi`, `eu-vhf`, `field-day`, `roundup`.
    pub attivita: String,
    #[serde(default)]
    pub parola_cq: String,
    /// `RR73` o `RRR`.
    #[serde(default = "rr73")]
    pub chiusura: String,
    /// `consigliato`, `facoltativo`, `no`.
    #[serde(default)]
    pub settantatre_finale: String,
}

fn rr73() -> String {
    "RR73".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scambio {
    /// Campi: `grid4`, `grid6`, `rst`, `snr`, `seriale`, `classe`, `sezione`, `stato`, `provincia`.
    pub inviato: Vec<String>,
    pub ricevuto: Vec<String>,
    /// Scambio fisso proposto (Field Day: `1D DX`).
    #[serde(default)]
    pub predefinito: String,
    /// `per-log` se lo scambio porta il numero progressivo.
    #[serde(default)]
    pub seriale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Doppi {
    /// `banda`, `banda+modo`, `banda+griglia`, `log`.
    pub chiave: String,
    /// `no` oppure `si-senza-punti` (IARU: il doppio va a log senza punti e segnato D).
    #[serde(default)]
    pub log_doppi: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Punti {
    /// `fisso`, `distanza`, `per-banda`, `per-paese`, `per-continente`, `km`.
    pub tipo: String,
    #[serde(default)]
    pub fisso: u32,
    #[serde(default)]
    pub base: u32,
    #[serde(default)]
    pub passo_km: u32,
    /// `giu` o `su`.
    #[serde(default)]
    pub arrotonda: String,
    /// Punti di distanza minimi (ARRL Digi: almeno 1).
    #[serde(default)]
    pub minimo_extra: u32,
    #[serde(default)]
    pub tabella: BTreeMap<String, u32>,
    /// Punti per un QSO nello stesso quadrato (IARU: 50).
    #[serde(default)]
    pub stesso_quadrato: u32,
    #[serde(default)]
    pub km_per_grado: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Moltiplicatore {
    /// `campo2`, `grid4`, `prefisso`, `sezione`, `stato`, `provincia-pa`, `dxcc` (paesi DXCC da cty.dat; dove lo scambio porta uno stato
    /// USA o una provincia canadese conta quello).
    pub tipo: String,
    /// `per-banda`, `per-log`, `per-banda-modo`.
    pub ambito: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Punteggio {
    /// `punti*molt`, `punti`, `punti*potenza+bonus`.
    pub formula: String,
    /// Moltiplicatore di potenza del Field Day: fino a `fino_a_w` watt vale `molt`.
    #[serde(default)]
    pub potenza: Vec<SogliaPotenza>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SogliaPotenza {
    pub fino_a_w: f64,
    pub molt: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Categoria {
    pub id: String,
    pub nome: String,
    /// Valori Cabrillo: `SINGLE-OP`, `MULTI-OP`, `CHECKLOG`, `ROVER`.
    pub operatore: String,
    /// `HIGH`, `LOW`, `QRP`.
    pub potenza: String,
    /// `ONE`, `TWO`, `UNLIMITED`.
    pub trasmettitori: String,
    #[serde(default = "all")]
    pub bande: String,
    #[serde(default)]
    pub assistenza: String,
    /// Sigla corta mostrata nella barra (vuota = l'`id`).
    #[serde(default)]
    pub sigla: String,
    /// La spiegazione breve in inglese (le lingue diverse dall'italiano la mostrano; vuota = `nome`).
    #[serde(default)]
    pub en: String,
    /// `CATEGORY-STATION` del Cabrillo: `FIXED`, `PORTABLE`, `MOBILE`, `ROVER`... (vuoto = `FIXED`, `ROVER` per i rover).
    #[serde(default)]
    pub stazione: String,
}

impl Categoria {
    /// La sigla corta (`SO-LP`, `MO`, `Rover`).
    pub fn sigla(&self) -> &str {
        if self.sigla.is_empty() { &self.id } else { &self.sigla }
    }
}

/// Le scelte lasciate dal regolamento oltre alla categoria.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OpzioniCategoria {
    /// Potenze tra cui scegliere (`HIGH`, `LOW`, `QRP`); vuoto = quella fissata dalla categoria.
    #[serde(default)]
    pub potenze: Vec<String>,
    /// Assisted si'/no a scelta.
    #[serde(default)]
    pub assistenza: bool,
    /// Field Day: la classe e' numero di trasmettitori + lettera della categoria (`1D`, `2A`).
    #[serde(default)]
    pub classe_fd: bool,
}

fn all() -> String {
    "ALL".into()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Restrizioni {
    /// Il primo contatto lo avvia un operatore: niente CQ automatico senza intervento.
    #[serde(default)]
    pub avvio_umano: bool,
    /// `vietato` o `ammesso`.
    #[serde(default)]
    pub fox_hound: String,
    #[serde(default)]
    pub multi_flusso: String,
    #[serde(default)]
    pub autospot: String,
    #[serde(default)]
    pub cambi_banda_ora: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Export {
    /// `cabrillo`, `edi`, `adif`.
    pub formato: String,
    #[serde(default)]
    pub cabrillo_contest: String,
    #[serde(default)]
    pub cabrillo_modo: String,
    #[serde(default)]
    pub colonne_inviate: Vec<String>,
    #[serde(default)]
    pub colonne_ricevute: Vec<String>,
    /// `khz` o `banda`.
    #[serde(default)]
    pub frequenza: String,
    #[serde(default)]
    pub edi_modo: Option<u32>,
    #[serde(default)]
    pub edi_tname: String,
    #[serde(default)]
    pub adif_contest_id: String,
    #[serde(default)]
    pub invio_url: String,
    #[serde(default)]
    pub scadenza: String,
}

const ATTIVITA: &[&str] = &["standard", "na-vhf", "arrl-digi", "ww-digi", "eu-vhf", "field-day", "roundup"];
const CAMPI: &[&str] = &["grid4", "grid6", "rst", "snr", "seriale", "classe", "sezione", "stato", "provincia", "scambio"];
const DOPPI: &[&str] = &["banda", "banda+modo", "banda+griglia", "log"];
const PUNTI: &[&str] = &["fisso", "distanza", "per-banda", "per-paese", "per-continente", "km"];
const MOLT: &[&str] = &["campo2", "grid4", "prefisso", "sezione", "stato", "provincia-pa", "dxcc"];
const AMBITI: &[&str] = &["per-banda", "per-log", "per-banda-modo"];
const FORMULE: &[&str] = &["punti*molt", "punti", "punti*potenza+bonus"];
const FORMATI: &[&str] = &["cabrillo", "edi", "adif"];

fn uno_di(campo: &str, valore: &str, ammessi: &[&str]) -> Result<(), String> {
    if ammessi.contains(&valore) { Ok(()) } else { Err(format!("{campo}: «{valore}» non ammesso (valori: {})", ammessi.join(", "))) }
}

impl Regole {
    /// Legge un file di regole e lo controlla.
    pub fn from_toml(testo: &str) -> Result<Regole, String> {
        let r: Regole = toml::from_str(testo).map_err(|e| format!("file di regole non valido: {e}"))?;
        r.valida()?;
        Ok(r)
    }

    /// I valori fuori dall'elenco chiuso dei tipi implementati sono errori.
    pub fn valida(&self) -> Result<(), String> {
        let id = &self.contest.id;
        let ctx = |e: String| format!("contest «{id}»: {e}");
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(ctx("id vuoto o con caratteri non ammessi (solo minuscole, cifre e -)".into()));
        }
        uno_di("messaggi.attivita", &self.messaggi.attivita, ATTIVITA).map_err(ctx)?;
        for c in self.scambio.inviato.iter().chain(&self.scambio.ricevuto).chain(&self.export.colonne_inviate).chain(&self.export.colonne_ricevute) {
            uno_di("campo dello scambio", c, CAMPI).map_err(ctx)?;
        }
        uno_di("doppi.chiave", &self.doppi.chiave, DOPPI).map_err(ctx)?;
        uno_di("punti.tipo", &self.punti.tipo, PUNTI).map_err(ctx)?;
        for m in &self.moltiplicatori {
            uno_di("moltiplicatori.tipo", &m.tipo, MOLT).map_err(ctx)?;
            uno_di("moltiplicatori.ambito", &m.ambito, AMBITI).map_err(ctx)?;
        }
        uno_di("punteggio.formula", &self.punteggio.formula, FORMULE).map_err(ctx)?;
        uno_di("export.formato", &self.export.formato, FORMATI).map_err(ctx)?;
        if self.punti.tipo == "distanza" && self.punti.passo_km == 0 {
            return Err(ctx("punti.passo_km deve essere maggiore di zero".into()));
        }
        if self.bande.ammesse.is_empty() || self.modi.ammessi.is_empty() {
            return Err(ctx("servono almeno una banda e un modo".into()));
        }
        for p in &self.opzioni_categoria.potenze {
            uno_di("opzioni_categoria.potenze", p, &["HIGH", "LOW", "QRP"]).map_err(ctx)?;
        }
        for c in &self.categorie {
            uno_di("categorie.operatore", &c.operatore, &["SINGLE-OP", "MULTI-OP", "CHECKLOG", "ROVER"]).map_err(ctx)?;
        }
        crate::calendario::controlla(&self.calendario).map_err(ctx)?;
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.contest.id
    }

    /// La banda e' ammessa dal regolamento.
    pub fn banda_ammessa(&self, banda: &str) -> bool {
        self.bande.ammesse.iter().any(|b| b.eq_ignore_ascii_case(banda))
    }

    pub fn modo_ammesso(&self, modo: &str) -> bool {
        self.modi.ammessi.iter().any(|m| m.eq_ignore_ascii_case(modo))
    }

    /// Fox/Hound vietato dal regolamento (di base si': nessun contest letto lo ammette).
    pub fn vieta_fox(&self) -> bool {
        self.restrizioni.fox_hound != "ammesso"
    }

    pub fn categoria(&self, id: &str) -> Option<&Categoria> {
        self.categorie.iter().find(|c| c.id == id)
    }
}

/// Tutte le regole: quelle distribuite con Aurora, sostituite o completate da quelle della cartella `dir` (se c'e'). Un file sbagliato
/// della cartella dell'utente non blocca gli altri: l'errore si restituisce a parte.
pub fn carica(dir: Option<&Path>) -> (Vec<Regole>, Vec<String>) {
    let mut out: Vec<Regole> = Vec::new();
    let mut errori = Vec::new();
    for (nome, testo) in BUILTIN {
        match Regole::from_toml(testo) {
            Ok(r) => out.push(r),
            Err(e) => errori.push(format!("{nome}.toml: {e}")),
        }
    }
    if let Some(dir) = dir
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect();
        paths.sort();
        for p in paths {
            let testo = match std::fs::read_to_string(&p) {
                Ok(t) => t,
                Err(e) => {
                    errori.push(format!("{}: {e}", p.display()));
                    continue;
                }
            };
            match Regole::from_toml(&testo) {
                Ok(r) => {
                    out.retain(|x| x.contest.id != r.contest.id);
                    out.push(r);
                }
                Err(e) => errori.push(format!("{}: {e}", p.display())),
            }
        }
    }
    (out, errori)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tutti i file distribuiti si leggono e passano il controllo.
    #[test]
    fn i_file_distribuiti_sono_validi() {
        let (regole, errori) = carica(None);
        assert!(errori.is_empty(), "{errori:?}");
        assert_eq!(regole.len(), BUILTIN.len());
        for (nome, _) in BUILTIN {
            assert!(regole.iter().any(|r| r.contest.id == *nome), "{nome}");
        }
    }

    /// Un campo sconosciuto e' un errore.
    #[test]
    fn un_campo_sconosciuto_e_un_errore() {
        let testo = BUILTIN[0].1.replace("[contest]", "[contest]\ncampo_strano = 1");
        assert!(Regole::from_toml(&testo).is_err());
    }

    /// Un tipo non implementato e' un errore chiaro.
    #[test]
    fn un_tipo_non_previsto_e_un_errore() {
        let testo = BUILTIN[0].1.replace("tipo = \"campo2\"", "tipo = \"dxcc-mondo\"");
        let e = Regole::from_toml(&testo).expect_err("errore");
        assert!(e.contains("moltiplicatori.tipo"), "{e}");
    }

    /// Il Field Day ha le classi A-F con la scelta del numero di trasmettitori; la sigla di base e' l'id.
    #[test]
    fn categorie_strutturate() {
        let (regole, _) = carica(None);
        let fd = regole.iter().find(|r| r.id() == "arrl-fd").expect("fd");
        assert!(fd.opzioni_categoria.classe_fd);
        let lettere: Vec<&str> = fd.categorie.iter().map(Categoria::sigla).collect();
        assert_eq!(lettere, ["A", "B", "C", "D", "E", "F"]);
        let ww = regole.iter().find(|r| r.id() == "ww-digi").expect("ww");
        assert_eq!(ww.categorie[0].sigla(), "SO-HIGH");
    }
}
