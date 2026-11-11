# Aurora - istruzioni per chi sviluppa

«Aurora — Multimode Ham Radio Station», stazione radioamatoriale multimodo (FT8, FT4, JT65, JT9, JT4, Q65, MSK144, FST4, FST4W, WSPR, PSK, RTTY, CW, SSTV, satelliti) per Windows e telefono: motore proprio (`core/crates/engine-native`), CAT, cascata, automazione dei QSO, controllo remoto.
Autore: Tiziano Cassone (IU3WJQ). Licenza GPL-3.0. Repository pubblico <https://github.com/wifi75/Aurora>; cartella di lavoro `D:\progetti\pc\Aurora`.
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

## Due rami: main (pubblico) e sviluppo (privato)
Questa sezione esiste solo nel ramo `sviluppo`. I modi nuovi (SSTV, RTTY, PSK, CW, menu nuovo...) non devono essere pubblici ne' scaricabili finche' il proprietario non lo dice.
- Remote: `origin` = github.com/wifi75/Aurora (pubblico), `sviluppo` = github.com/wifi75/Aurora-sviluppo (privato). Il ramo `sviluppo` va SOLO su `git push sviluppo sviluppo`, mai su `origin`.
- **«modifica / aggiungi / correggi ... per FT8»** (o FT4, Rotore, una correzione da pubblicare): si lavora su `main`. `git switch main`, `git pull`, modifiche, controlli, commit, `git push origin main`; poi si porta su sviluppo: `git switch sviluppo`, `git pull`, `git merge main`, `git push sviluppo sviluppo`.
- **«continua lo sviluppo»** (o SSTV, RTTY, menu nuovo, modi nuovi): si lavora su `sviluppo`. `git switch sviluppo`, `git pull`, lavoro, commit, `git push sviluppo sviluppo`.
- **«pubblica»**: release solo da `main` seguendo `TASK.md`; niente di `sviluppo` deve finirci.
- **«dammi la versione da installare»**: si compila SUL PC (non su GitHub: i minuti di Actions del repository privato sono pochi) dalla cartella del ramo sviluppo con `.\scripts\installatore-sviluppo.ps1 -Installa` (circa 10-20 minuti); l'installatore resta in `installer\` (solo l'ultimo: i vecchi si cancellano). Quella versione non si aggiorna da sola (`AURORA_SVILUPPO`). Il lavoro `Sviluppo` su GitHub resta solo a mano (`workflow_dispatch`) e non si usa senza chiederlo.
- Prima di ogni push controllare ramo e remote (`git status -sb`). Allineare il PC: `git switch main && git pull && git switch sviluppo && git pull` (oppure `/wifi75-git-start` sul ramo in uso).

## Per riprendere il lavoro
Leggi per primo `docs/ripresa.md`: stato attuale, lavoro in corso e prossimo passo.

## Documenti da aprire quando servono
- `docs/sviluppo.md`: percorsi, struttura dei crate, comandi di test e lint, regole del programma Windows.
- `docs/rilascio.md`: comandi per alzare la versione, firmare l'installatore e pubblicare la release.
- `memory.md` (stato e decisioni), `docs/mancante.md` (cosa manca), `CHANGELOG.md`.
- `docs/logiche-jtdx.md`, `docs/integrazioni-log.md`, `docs/api.md`, `docs/cat-radio.md`, `docs/analisi-*.md` per le singole aree.
