# Aurora - istruzioni per chi sviluppa

«Aurora — Multimode Ham Radio Station», stazione radioamatoriale multimodo (FT8, FT4, JT65, JT9, JT4, Q65, MSK144, FST4, FST4W, WSPR, PSK, RTTY, CW, SSTV, satelliti) per Windows e telefono: motore proprio (`core/crates/engine-native`), CAT, cascata, automazione dei QSO, controllo remoto.
Autore: Tiziano Cassone (IU3WJQ). Licenza GPL-3.0. Repository pubblico <https://github.com/wifi75/Aurora>; cartella di lavoro `%USERPROFILE%\Desktop\Progetti\Aurora`.
`core/` Rust (Tokio, Axum) · `app/` React + TypeScript + Vite · `desktop/` Tauri 2 (workspace Cargo a parte) · `docs/` documentazione.

## Regole
- Rust edition 2024. Niente `unwrap()`/`expect()` fuori dai test; errori come `Result`.
- Nessun segreto, locatore vero o email personale nei file e nei commit. Nominativo reale solo come firma d'autore (eccezione: nominativi dei sostenitori che hanno dato il consenso, solo in `SOSTENITORI.md`); in test ed esempi IU3ABC e IO91wm.
- Nessun riferimento a strumenti di assistenza nel repository ne' nei commit (niente trailer). Il file delle istruzioni si chiama `AGENTS.md`.
- Commit in italiano, atomici. Push e release autorizzati.
- Si controlla solo che compili (`cargo check --workspace`, `npx tsc --noEmit`): le prove con radio e telefono le fa il proprietario.
- Logiche di QSO e trasmissione: leggere `docs/logiche-jtdx.md` prima di toccarle; il comportamento di JTDX/Decodium si verifica con REA, non si indovina.
- Ogni testo dell'interfaccia, della Guida e delle Novita' e' in sei lingue nelle tabelle (it, en, de, fr, es, pt) piu' l'olandese (nl) nel dizionario a parte `app/src/i18n/nl/` (chiavi come le tabelle; Guida e Novita' per impronta del testo inglese). Se manca una voce olandese si usa l'inglese. Il dizionario si rivede dal foglio `docs/traduzione-olandese.csv`: `node scripts/export-nl.mjs` lo rigenera, `node scripts/importa-nl.mjs` rilegge le correzioni.
- A ogni versione si seguono i passi di `TASK.md`.
- Linux (non provata dall'autore, senza assistenza) usa lo stesso codice: ogni API di Windows va in `#[cfg(windows)]`; il controllo `Linux` di GitHub Actions deve restare verde (vedi `docs/sviluppo.md`).

## Ramo
Esiste solo `main` (il ramo `sviluppo` e il repository privato sono stati tolti). Si lavora, si fa commit e `git push origin main`; prima di ogni push controllare con `git status -sb`.

## Per riprendere il lavoro
Leggi per primo `docs/ripresa.md`: stato attuale, lavoro in corso e prossimo passo.

## Documenti da aprire quando servono
- `docs/sviluppo.md`: percorsi, struttura dei crate, comandi di test e lint, regole del programma Windows.
- `docs/rilascio.md`: comandi per alzare la versione, firmare l'installatore e pubblicare la release.
- `memory.md` (stato e decisioni), `docs/mancante.md` (cosa manca), `CHANGELOG.md`.
- `docs/logiche-jtdx.md`, `docs/integrazioni-log.md`, `docs/api.md`, `docs/cat-radio.md`, `docs/analisi-*.md` per le singole aree.
