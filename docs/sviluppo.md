# Sviluppo di Aurora: percorsi, struttura, comandi

Dettagli che servono solo quando si lavora su quella parte. Le regole sempre valide sono in `AGENTS.md`.

## Dove sta
- Cartella di lavoro: `D:\progetti\pc\Aurora` (dal 2026-11-10 i progetti in `D:\progetti` sono divisi in `pc`, `smartphone`, `web` e `archivio`; prima `D:\progetti\Aurora`, e fino al 2026-10-25 `C:\Users\Tiziano\Desktop\progetti\Aurora`).
- Chiave di firma degli aggiornamenti: nei segreti di GitHub (`AURORA_TIMBRO_AGGIORNAMENTI_AURORA_KEY`), usata dal lavoro Windows che compila l'installatore; una copia sul PC di casa in `%USERPROFILE%\.aurora-signing\aurora.key` (fuori dal repository, da tenere in un backup), serve solo per provare installatori in locale. La pubblica e' `desktop/updater.pub`.
- Dati del programma installato: `%APPDATA%\Aurora` (`aurora.env`, impostazioni, storico, `logs\aurora.log`); programma in `%LOCALAPPDATA%\Aurora`.

## Struttura
- `core/` workspace Rust (Tokio, Axum): `aurora` (avvio e collegamenti), `remote-api` (API e WebSocket), `qso-engine` (cervello delle automazioni), `engine-native` (motore proprio), `engine-jtdx`, `rig-control` (CAT), `audio-spectrum`, `input-level`, `decode-history`, `online-info`, `logbook`, `reporting`, `updater`, `clock-sync`, `ft8` e altri.
- `app/` interfaccia React + TypeScript + Vite (cascata WebGPU/WebGL2, mappa Leaflet, telefono a schede).
- `desktop/` programma Windows (Tauri 2, workspace Cargo a parte): vassoio, finestra, avvio automatico, aggiornamento firmato, installatore NSIS (`desktop/src-tauri/installer`).
- `docs/` specifiche, piani, rapporti, analisi.

## Comandi
- Test core: `cd core && cargo test --workspace --no-fail-fast --target-dir target/testrun`; lint: `cargo clippy --workspace --all-targets --target-dir target/testrun -- -D warnings`.
- Test interfaccia: `cd app && npx tsc --noEmit && npx vitest run`; build: `npm run build`.
- Test programma Windows: `cd desktop/src-tauri && cargo test`.
- RTTY, prova di confronto del ricevitore (lenta, a mano): `cd core && cargo test --release -p mode-digi --test rtty_confronto -- --ignored --nocapture --test-threads=1` (tabella dei caratteri sbagliati per spostamento, velocità e SNR; `stretti` due filtri contro discriminatore; `studio` un punteggio solo per provare le varianti).
- RTTY con fldigi come arbitro (solo prove, mai nell'installatore): `cd core && nice cargo test -j 6 -p mode-digi --features fldigi-oracle -- --test-threads=2 --nocapture`. La feature `fldigi-oracle` compila con `cc` (MSVC o g++) il ricevitore di fldigi 4.2.13 dal sottomodulo `terze-parti/fldigi` (`src/filters/fftfilt.cxx` e gli header di `src/include`, GPL 3, Dave Freese W1HKJ e contributori) più `core/crates/mode-digi/oracle/fldigi_rtty_rx.cpp`, la parte di ricezione di `src/rtty/rtty.cxx` estratta senza FLTK. Serve il sottomodulo: `git submodule update --init terze-parti/fldigi`. La prova `tests/rtty_fldigi.rs` dà lo stesso audio ai due ricevitori e fallisce se il nostro sbaglia più di fldigi oltre 2 punti.
- Licenze delle dipendenze: `python scripts/licenze-terze-parti.py` (rigenera `docs/licenze-terze-parti.md`).

## Programma Windows
- Ogni processo figlio (PowerShell, cmd) parte con `CREATE_NO_WINDOW`; l'installatore dell'aggiornamento parte staccato da Aurora (`cmd /C start`) e la riga di chiusura dell'installatore non deve uccidere i processi figli.
- La trasmissione e' aperta di default (`AURORA_ALLOW_TX=0` la chiude): STOP TX, tetti, dead-man e automazioni spente di default restano.

## Linux (non provata dall'autore, senza assistenza)
- Stesso codice e stesso ramo di Windows: nessun ramo separato. Ogni uso di API di Windows (`std::os::windows`, `windows`, `windows-sys`, `cmd`, percorsi `C:\`) va dentro `#[cfg(windows)]`, con la sua alternativa `#[cfg(not(windows))]` se serve.
- Il controllo `Linux` di GitHub Actions (`.github/workflows/linux.yml`) compila a ogni push: se diventa rosso, quasi sempre e' una riga di Windows senza `#[cfg(windows)]`.
- Compilazione locale: `./scripts/compila-linux.sh`; impostazioni del pacchetto in `desktop/src-tauri/tauri.linux.conf.json` (solo Linux, si somma a `tauri.conf.json`). Guida per gli utenti: `docs/linux.md` (sei lingue).
- Su Linux mancano OmniRig, il PTT su RTS/DTR, il volume di registrazione e l'aggiornamento automatico (l'installatore risponde con un errore chiaro).

## Documenti per riprendere
`memory.md` (stato e decisioni), `docs/mancante.md` (cosa manca), `CHANGELOG.md`; per il motore proprio `docs/2026-10-24-aurora-motore-ft8.md` e le analisi `docs/analisi-decodium-*.md` (CAT, cascata, banda satura); per i log esterni `docs/integrazioni-log.md`.
