# Rilascio di una versione

L'elenco dei compiti da chiudere a ogni versione e' in `TASK.md`; qui i comandi.

1. Alzare la versione in `core/Cargo.toml`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/package.json`, `app/package.json` e nei lock, nei README (riga dello stato), riga 3 di `memory.md` e `docs/mancante.md`.
2. Le voci di `## Non rilasciato` del CHANGELOG diventano la sezione della versione. Novita' in `app/src/lib/novita.ts`: titolo e testo nelle sei lingue, data e ora UTC; restano le ultime 40 (una prova controlla che la prima sia della versione di `app/package.json`).
3. Compilazione: `cargo check --workspace` (core e `desktop/src-tauri`), `cd app && npx tsc --noEmit`. Commit e push.
4. Tag `vX.Y.Z`, push del tag e `gh release create vX.Y.Z --latest=false` con le note e **senza file allegati** (il lavoro `Windows` la segna come ultima solo dopo aver allegato installatore e `.sig`: prima le copie installate dicevano «la release non ha il pacchetto di installazione con la sua firma»). Dal 2026-11-10 gli installatori li fa GitHub
   alla pubblicazione della release: non si compila piu' sul PC e la chiave di firma non serve in locale.
   - **Windows** (`.github/workflows/windows.yml`): compila con Hamlib e `cty.dat`, firma gli aggiornamenti con la chiave nei segreti
     (`AURORA_TIMBRO_AGGIORNAMENTI_AURORA_KEY`) e allega `Aurora_X.Y.Z_x64-setup.exe`, il `.sig`, `Aurora-Setup.exe` e `SHA256SUMS.txt`.
     Se il PC ha gia' allegato un installatore, non lo tocca.
   - **Linux** e **macOS**: come prima, allegano AppImage, `.deb`, `.dmg` e i loro `SHA256SUMS`.
5. Aspettare 10-15 minuti e controllare che i tre lavori (Windows, Linux, macOS) siano verdi e che la release abbia `Aurora-Setup.exe` e il
   `.sig`: finche' mancano, Aurora che si aggiorna dice «la release non ha il pacchetto di installazione con la sua firma».
   (`scripts/rilascio-windows.ps1` resta per compilare e provare un installatore sul PC prima di pubblicare.)
6. Ora vera nelle Novita' (`gh release view vX.Y.Z --json publishedAt`), commit e push; pagina web con `scripts/pubblica-sito.ps1`.
7. File dei paesi: i lavori di GitHub (e `rilascio-windows.ps1` sul PC) scaricano da soli `cty.dat` da country-files.com in `desktop/src-tauri/cty/` e si fermano se non ci riescono.
8. Linux: niente da fare. Alla pubblicazione della release GitHub Actions (`Linux`) allega da sola `Aurora-Linux-x86_64.AppImage`, `Aurora-Linux-amd64.deb` e `SHA256SUMS-linux.txt`; si controlla solo che il lavoro sia verde.

## Velocita' della compilazione

- `D:\progetti` e `D:\build` sono esclusi da Windows Defender (impostazione permanente fatta dal proprietario). Senza esclusione la compilazione si bloccava in attesa del disco.
- Durante la compilazione dell'installatore non devono girare altre compilazioni (agenti, prove): si mettono in pausa. Si compila con 9 processi (`jobs = 9` in `~/.cargo/config.toml`): con 18 la memoria finiva e Aurora aperta si chiudeva (`memory allocation ... failed`, 2026-11-08).
- Prima di `git checkout` del commit nella cartella di compilazione `D:\build\Aurora-0.11.1` si annullano i file rigenerati (`git checkout -- .`, es. `Cargo.lock`) e si controlla che `HEAD` sia il commit giusto: un checkout fallito ricompila la versione vecchia.
