# Aurora - istruzioni per chi sviluppa

Stazione digitale FT8/FT4 per Windows e telefono: motore proprio (`core/crates/engine-native`), CAT, cascata, automazione dei QSO, controllo remoto.
Autore: Tiziano Cassone (IU3WJQ). Licenza GPL-3.0. Repository pubblico <https://github.com/wifi75/Aurora>; cartella di lavoro `D:\progetti\Aurora`.
`core/` Rust (Tokio, Axum) · `app/` React + TypeScript + Vite · `desktop/` Tauri 2 (workspace Cargo a parte) · `docs/` documentazione.

## Regole
- Rust edition 2024. Niente `unwrap()`/`expect()` fuori dai test; errori come `Result`.
- Nessun segreto, locatore vero o email personale nei file e nei commit. Nominativo reale solo come firma d'autore; in test ed esempi IU3ABC e IO91wm.
- Nessun riferimento a strumenti di assistenza nel repository ne' nei commit (niente trailer). Il file delle istruzioni si chiama `AGENTS.md`.
- Commit in italiano, atomici. Push e release autorizzati.
- Si controlla solo che compili (`cargo check --workspace`, `npx tsc --noEmit`): le prove con radio e telefono le fa il proprietario.
- Logiche di QSO e trasmissione: leggere `docs/logiche-jtdx.md` prima di toccarle; il comportamento di JTDX/Decodium si verifica con REA, non si indovina.
- Ogni testo dell'interfaccia, della Guida e delle Novita' e' in sei lingue (it, en, de, fr, es, pt).
- A ogni versione si seguono i passi di `TASK.md`.
- Linux (non provata dall'autore, senza assistenza) usa lo stesso codice: ogni API di Windows va in `#[cfg(windows)]`; il controllo `Linux` di GitHub Actions deve restare verde (vedi `docs/sviluppo.md`).

## Per riprendere il lavoro
Leggi per primo `docs/ripresa.md`: stato attuale, lavoro in corso e prossimo passo.

## Documenti da aprire quando servono
- `docs/sviluppo.md`: percorsi, struttura dei crate, comandi di test e lint, regole del programma Windows.
- `docs/rilascio.md`: comandi per alzare la versione, firmare l'installatore e pubblicare la release.
- `memory.md` (stato e decisioni), `docs/mancante.md` (cosa manca), `CHANGELOG.md`.
- `docs/logiche-jtdx.md`, `docs/integrazioni-log.md`, `docs/api.md`, `docs/cat-radio.md`, `docs/analisi-*.md` per le singole aree.
