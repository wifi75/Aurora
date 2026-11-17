//! Importa ed esporta un contest dei modi testuali (`docs/contest-file.md`).
//!
//! Il pacchetto e' un `.zip` con il file `<nome>.contest` in cima, il suo file di macro in `macros/` e il suo elenco dei domini in
//! `dom/` (i percorsi scritti nel `.contest`). Si importa anche un `.contest` da solo. All'importazione il file si controlla
//! (sezioni `[Score]` e `[Script]`, percorsi relativi), non sovrascrive niente (nome nuovo se c'e' gia') e del pacchetto si
//! prendono solo i file nominati dal contest.

use crate::elenco::{ContestTesto, ElencoContest, percorso_relativo_ok, testo_da_byte};
use std::io::{Cursor, Read, Write};

/// Dimensione massima di un pacchetto e di ogni file dentro (un contest e' testo di pochi kB).
pub const MAX_BYTE: u64 = 2 * 1024 * 1024;
const MAX_VOCI: usize = 64;

/// Il nome del file del pacchetto di un contest (`<id>.zip`).
pub fn nome_pacchetto(id: &str) -> String {
    format!("{id}.zip")
}

fn leggi_voce(z: &mut zip::ZipArchive<Cursor<&[u8]>>, i: usize) -> Result<(String, Vec<u8>), String> {
    let f = z.by_index(i).map_err(|e| format!("pacchetto non leggibile: {e}"))?;
    let nome = f.name().map_err(|e| format!("pacchetto non leggibile: {e}"))?.replace('\\', "/");
    if f.size() > MAX_BYTE {
        return Err(format!("file troppo grande nel pacchetto: {nome}"));
    }
    let mut b = Vec::new();
    f.take(MAX_BYTE + 1).read_to_end(&mut b).map_err(|e| format!("pacchetto non leggibile: {e}"))?;
    if b.len() as u64 > MAX_BYTE {
        return Err(format!("file troppo grande nel pacchetto: {nome}"));
    }
    Ok((nome, b))
}

/// Un file del pacchetto da copiare: percorso relativo (`macros/x.mc`) e contenuto.
type File = (String, Vec<u8>);

/// Legge un pacchetto (o un `.contest` da solo): il contest controllato e i file di macro e di domini che nomina.
pub fn leggi_pacchetto(nome_file: &str, dati: &[u8]) -> Result<(ContestTesto, Vec<File>), String> {
    if dati.len() as u64 > MAX_BYTE {
        return Err("file troppo grande".into());
    }
    let stem = |n: &str| {
        let base = n.rsplit(['/', '\\']).next().unwrap_or(n);
        base.rsplit_once('.').map_or(base, |(a, _)| a).to_string()
    };
    if !dati.starts_with(b"PK") {
        // un .contest da solo
        let c = ContestTesto::da_testo(&stem(nome_file), &testo_da_byte(dati));
        if !testo_da_byte(dati).lines().any(|r| r.trim().eq_ignore_ascii_case("[contest]")) {
            return Err("non e' un file .contest (manca la sezione [Contest])".into());
        }
        c.controlla()?;
        return Ok((c, Vec::new()));
    }
    let mut z = zip::ZipArchive::new(Cursor::new(dati)).map_err(|e| format!("pacchetto non leggibile: {e}"))?;
    if z.len() > MAX_VOCI {
        return Err("troppi file nel pacchetto".into());
    }
    let mut voci = Vec::new();
    for i in 0..z.len() {
        let (nome, b) = leggi_voce(&mut z, i)?;
        if !nome.ends_with('/') {
            voci.push((nome, b));
        }
    }
    let mut contest = voci.iter().filter(|(n, _)| !n.contains('/') && n.to_ascii_lowercase().ends_with(".contest"));
    let (Some((nome, b)), None) = (contest.next(), contest.next()) else {
        return Err("il pacchetto deve contenere un solo file .contest in cima".into());
    };
    let c = ContestTesto::da_testo(&stem(nome), &testo_da_byte(b));
    c.controlla()?;
    let mut file = Vec::new();
    for (rel, cartella) in [(&c.macros, "macros/"), (&c.domain_file, "dom/")] {
        let rel = rel.trim().replace('\\', "/");
        if rel.is_empty() || !percorso_relativo_ok(&rel) || !rel.starts_with(cartella) {
            continue;
        }
        if let Some((_, b)) = voci.iter().find(|(n, _)| n.eq_ignore_ascii_case(&rel)) {
            file.push((rel, b.clone()));
        }
    }
    Ok((c, file))
}

/// Un percorso libero nella cartella dei contest: `macros/x.mc`, poi `macros/x (2).mc`... (lo stesso se il contenuto e' uguale).
fn percorso_libero(e: &ElencoContest, rel: &str, dati: &[u8]) -> String {
    let (base, ext) = rel.rsplit_once('.').map_or((rel.to_string(), String::new()), |(a, b)| (a.to_string(), format!(".{b}")));
    let mut cand = rel.to_string();
    let mut k = 2;
    loop {
        let p = e.dir().join(&cand);
        match std::fs::read(&p) {
            Err(_) => return cand,
            Ok(b) if b == dati => return cand,
            Ok(_) => {}
        }
        cand = format!("{base} ({k}){ext}");
        k += 1;
    }
}

impl ElencoContest {
    /// Importa un pacchetto (`.zip`) o un `.contest`: restituisce l'id del contest aggiunto (mai sovrascritto).
    pub fn importa(&self, nome_file: &str, dati: &[u8]) -> Result<String, String> {
        let (mut c, file) = leggi_pacchetto(nome_file, dati)?;
        for (rel, b) in file {
            let dove = percorso_libero(self, &rel, &b);
            let p = self.dir().join(&dove);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            std::fs::write(&p, &b).map_err(|e| e.to_string())?;
            if rel == c.macros.trim().replace('\\', "/") {
                c.macros = dove;
            } else {
                c.domain_file = dove;
            }
        }
        c.id = String::new();
        self.salva(c)
    }

    /// Il pacchetto `.zip` di un contest con le sue macro e il suo elenco dei domini.
    pub fn esporta(&self, id: &str) -> Result<Vec<u8>, String> {
        // un contest a cartella: il `.contest` convertito con le sue macro e il suo elenco dei domini
        let (c, da_cartella) = match self.file_del_pacchetto(id) {
            Some((c, file)) => (c, file),
            None => (self.leggi(id).ok_or_else(|| "contest sconosciuto".to_string())?, Vec::new()),
        };
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut metti = |nome: &str, b: &[u8]| -> Result<(), String> {
            z.start_file(nome, o).map_err(|e| e.to_string())?;
            z.write_all(b).map_err(|e| e.to_string())
        };
        metti(&format!("{}.contest", c.id), c.in_testo().as_bytes())?;
        for (rel, b) in &da_cartella {
            metti(rel, b)?;
        }
        for rel in [&c.macros, &c.domain_file].into_iter().filter(|_| da_cartella.is_empty()) {
            let rel = rel.trim().replace('\\', "/");
            if rel.is_empty() || !percorso_relativo_ok(&rel) {
                continue;
            }
            if let Ok(b) = std::fs::read(self.dir().join(&rel)) {
                metti(&rel, &b)?;
            }
        }
        Ok(z.finish().map_err(|e| e.to_string())?.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cartella(nome: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("aurora-pacchetto-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    const MC: &str = "[Macros]\r\nnItems=1\r\nName0=F1\r\nLabel0=CQ\r\nText0=CQ TEST <MYCALL>\r\n";

    fn prova(e: &ElencoContest) -> String {
        let _ = std::fs::write(e.dir().join("macros/Prova.mc"), MC);
        let c = ContestTesto {
            name: "Prova".into(),
            macros: "macros/Prova.mc".into(),
            score: "points=any=2\n".into(),
            script: "fn punti(qso) { 3 }\n".into(),
            ..ContestTesto::default()
        };
        e.salva(c).unwrap_or_default()
    }

    #[test]
    fn esporta_e_reimporta_con_le_macro() {
        let d = cartella("andata");
        let e = ElencoContest::new(&d);
        let id = prova(&e);
        let zip = e.esporta(&id);
        let Ok(zip) = zip else { panic!("{zip:?}") };
        // reimportato nella stessa cartella: nome nuovo, macro uguali riusate
        let nuovo = e.importa("Prova.zip", &zip);
        assert_eq!(nuovo.as_deref(), Ok("Prova (2)"));
        let c = e.leggi("Prova (2)").unwrap_or_default();
        assert_eq!(c.macros, "macros/Prova.mc");
        assert_eq!(c.script, "fn punti(qso) { 3 }\n");
        // in una cartella vuota arrivano anche le macro
        let d2 = cartella("ritorno");
        let e2 = ElencoContest::new(&d2);
        assert_eq!(e2.importa("Prova.zip", &zip).as_deref(), Ok("Prova"));
        assert_eq!(std::fs::read_to_string(d2.join("macros/Prova.mc")).unwrap_or_default(), MC);
        // macro diverse gia' presenti: non si sovrascrivono
        let _ = std::fs::write(d2.join("macros/Prova.mc"), "altro");
        assert_eq!(e2.importa("Prova.zip", &zip).as_deref(), Ok("Prova (2)"));
        assert_eq!(e2.leggi("Prova (2)").unwrap_or_default().macros, "macros/Prova (2).mc");
        assert_eq!(std::fs::read_to_string(d2.join("macros/Prova.mc")).unwrap_or_default(), "altro");
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&d2);
    }

    #[test]
    fn importa_un_contest_da_solo_e_rifiuta_quelli_sbagliati() {
        let d = cartella("solo");
        let e = ElencoContest::new(&d);
        let buono = "[Contest]\r\nm_sName=Solo\r\n\r\n[Score]\r\npoints=any=1\r\n";
        assert_eq!(e.importa("Solo.contest", buono.as_bytes()).as_deref(), Ok("Solo"));
        // [Score] sbagliata, script che non compila, percorso fuori dalla cartella, file che non e' un contest
        assert!(e.importa("a.contest", b"[Contest]\r\nm_sName=A\r\n[Score]\r\nmult=pippo\r\n").is_err());
        assert!(e.importa("b.contest", b"[Contest]\r\nm_sName=B\r\n[Script]\r\nfn punti(q) { 1 + }\r\n").is_err());
        assert!(e.importa("c.contest", b"[Contest]\r\nm_sName=C\r\nm_sMacrosFilename=../../x.mc\r\n").is_err());
        assert!(e.importa("d.txt", b"ciao").is_err());
        assert!(e.importa("e.zip", b"PK\x03\x04rotto").is_err());
        assert_eq!(e.elenco().len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }
}
