# Rilascio di una versione

L'elenco dei compiti da chiudere a ogni versione e' in `TASK.md`; qui i comandi.

1. Alzare la versione in `core/Cargo.toml`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/package.json`, `app/package.json` e nei lock, nei README (riga dello stato), riga 3 di `memory.md` e `docs/mancante.md`.
2. Le voci di `## Non rilasciato` del CHANGELOG diventano la sezione della versione. Novita' in `app/src/lib/novita.ts`: titolo e testo nelle sei lingue, data e ora UTC; restano le ultime 40 (una prova controlla che la prima sia della versione di `app/package.json`).
3. Compilazione: `cargo check --workspace` (core e `desktop/src-tauri`), `cd app && npx tsc --noEmit`. Commit e push.
4. Installatore firmato:
   `$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content "$env:USERPROFILE\.aurora-signing\aurora.key" -Raw); .\scripts\rilascio-windows.ps1`
   (il nome dell'installatore deve finire in `-setup.exe`; guida in `docs/installer-windows.md`).
5. Copia `Aurora-Setup.exe` dell'installatore e `SHA256SUMS.txt` dei file; tag `vX.Y.Z`, push del tag, `gh release create` con installatore, `.sig`, `SHA256SUMS.txt` e `Aurora-Setup.exe` (il link `releases/latest/download/Aurora-Setup.exe` scarica sempre l'ultima).
6. Ora vera nelle Novita' (`gh release view vX.Y.Z --json publishedAt`), commit e push; pagina web con `scripts/pubblica-sito.ps1`.
7. Linux: niente da fare. Alla pubblicazione della release GitHub Actions (`Linux`) allega da sola `Aurora-Linux-x86_64.AppImage`, `Aurora-Linux-amd64.deb` e `SHA256SUMS-linux.txt`; si controlla solo che il lavoro sia verde.

## Velocita' della compilazione

- `D:\progetti` e `D:\build` sono esclusi da Windows Defender (impostazione permanente fatta dal proprietario). Senza esclusione la compilazione si bloccava in attesa del disco.
- Durante la compilazione dell'installatore non devono girare altre compilazioni (agenti, prove): si mettono in pausa. Da sola, con `CARGO_BUILD_JOBS=18`, dura pochi minuti.
- Prima di `git checkout` del commit nella cartella di compilazione `D:\build\Aurora-0.11.1` si annullano i file rigenerati (`git checkout -- .`, es. `Cargo.lock`) e si controlla che `HEAD` sia il commit giusto: un checkout fallito ricompila la versione vecchia.
