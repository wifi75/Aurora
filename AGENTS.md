# Aurora - istruzioni per chi sviluppa

Stazione digitale FT8/FT4 con automazione dei QSO e controllo remoto, per Windows (programma con vassoio e aggiornamenti) e per telefono.
**Aurora e' un'applicazione a se': non deve funzionare con l'ausilio di JTDX ne' di Decodium.** Il motore e' il suo (crate `core/crates/ft8`, `ft4`, `engine-native`): vedi `docs/2026-10-24-aurora-motore-ft8.md`. JTDX resta solo un motore alternativo temporaneo (`AURORA_ENGINE=jtdx`) e un termine di confronto.
Autore: Tiziano Cassone (IU3WJQ). Licenza: GPL-3.0 (solo). Repository: <https://github.com/wifi75/Aurora> (pubblico).

## Dove sta
- Cartella di lavoro: `D:\progetti\Aurora` (spostata da `C:\Users\Tiziano\Desktop\progetti\Aurora` il 2026-10-25; tutti i progetti sono in `D:\progetti`).
- Chiave di firma degli aggiornamenti: `%USERPROFILE%\.aurora-signing\aurora.key` (fuori dal repository, da tenere in un backup). La pubblica e' `desktop/updater.pub`.
- Dati del programma installato: `%APPDATA%\Aurora` (`aurora.env`, impostazioni, storico, `logs\aurora.log`); programma in `%LOCALAPPDATA%\Aurora`.

## Struttura
- `core/` workspace Rust (Tokio, Axum): `aurora` (avvio e collegamenti), `remote-api` (API e WebSocket), `qso-engine` (cervello delle automazioni), `engine-jtdx`, `rig-control` (OmniRig e comandi a JTDX), `audio-spectrum`, `input-level`, `decode-history`, `online-info`, `updater`, `clock-sync`, `ft8` (motore proprio, in costruzione) e altri.
- `app/` interfaccia React + TypeScript + Vite (cascata WebGPU/WebGL2, mappa Leaflet, telefono a schede).
- `desktop/` programma Windows (Tauri 2, workspace Cargo a parte): vassoio, finestra, avvio automatico, aggiornamento firmato, installatore NSIS (`desktop/src-tauri/installer`).
- `docs/` specifiche, piani, rapporti, analisi; `CHANGELOG.md`, `TODO.md`, `memory.md` (stato e decisioni), `docs/mancante.md` (cosa manca).

## Comandi
- Test core: `cd core && cargo test --workspace --no-fail-fast --target-dir target/testrun`; lint: `cargo clippy --workspace --all-targets --target-dir target/testrun -- -D warnings`.
- Test interfaccia: `cd app && npx tsc --noEmit && npx vitest run`; build: `npm run build`.
- Test programma Windows: `cd desktop/src-tauri && cargo test`.
- Rilascio Windows: alzare la versione in `core/Cargo.toml`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/package.json`, `app/package.json` e nei lock; `$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content "$env:USERPROFILE\.aurora-signing\aurora.key" -Raw); .\scripts\rilascio-windows.ps1`; poi tag `vX.Y.Z`, push e `gh release create` con installatore, `.sig` e `SHA256SUMS.txt` (guida in `docs/installer-windows.md`). Il nome dell'installatore deve finire in `-setup.exe`.
- Licenze delle dipendenze: `python scripts/licenze-terze-parti.py` (rigenera `docs/licenze-terze-parti.md`).

## Regole
- **Per capire e clonare le funzionalita' di JTDX e di Decodium si usa sempre REA** (<https://github.com/morluto/rea>, gia' installato con Ghidra): reverse dei programmi installati (`C:\JTDX64\159`, `C:\Users\Tiziano\AppData\Local\Programs\Decodium`), insieme ai loro sorgenti pubblici GPL 3. Un'analisi REA alla volta (memoria: `-Xmx` alto, timeout lunghi). Come si usa e cosa ha funzionato: `docs/fase0-rapporto.md` e `docs/analisi-decodium-reverse-rea.md`. Le funzioni si riscrivono in Rust/TypeScript in Aurora, citando la fonte; Aurora non dipende da quei programmi.
- Rust edition 2024. Niente `unwrap()`/`expect()` fuori dai test; errori come `Result`.
- Nessun segreto nel repository; usare `.env.example`. Mai il locatore vero ne' indirizzi email personali nei file o nei commit: l'email degli autori e' quella `users.noreply.github.com` gia' impostata. Nominativo reale solo come firma d'autore; nei test e negli esempi usare IU3ABC e IO91wm.
- Nessun riferimento a strumenti di assistenza nel repository, ne' nei messaggi di commit (niente trailer di attribuzione). Il file delle istruzioni si chiama `AGENTS.md`.
- Commit in italiano, atomici; il push e le release solo su richiesta del proprietario.
- Le prove manuali con radio e telefono le fa il proprietario; qui si eseguono i test automatici.
- Specifica e piani in `docs/`.
- Logiche di QSO, trasmissione e selezione dei messaggi: leggere `docs/logiche-jtdx.md` prima di toccarle. Quel documento descrive il comportamento di riferimento di JTDX da replicare nel motore proprio; con `AURORA_ENGINE=jtdx` Aurora legge ancora lo stato di JTDX (UDP, finestra), ma non e' piu' la strada principale.
- Programma Windows: ogni processo figlio (PowerShell, cmd) parte con `CREATE_NO_WINDOW`; l'installatore dell'aggiornamento parte staccato da Aurora (`cmd /C start`) e la riga di chiusura dell'installatore non deve uccidere i processi figli.
- La trasmissione nel programma Windows e' aperta di default (`AURORA_ALLOW_TX=0` la chiude): STOP TX, tetti, dead-man e automazioni spente di default restano.

## Dove riprendere
Leggere `memory.md` (stato e decisioni), `docs/mancante.md`, `CHANGELOG.md` e, per il motore proprio, `docs/2026-10-24-aurora-motore-ft8.md` e le analisi `docs/analisi-decodium-*.md` (CAT per tutte le radio, cascata piu' reattiva, banda satura).
