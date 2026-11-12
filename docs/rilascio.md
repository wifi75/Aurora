# Rilascio di una versione

L'elenco dei compiti da chiudere a ogni versione e' in `TASK.md`; qui i comandi.

1. Alzare la versione in `core/Cargo.toml`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/package.json`, `app/package.json` e nei lock, nei README (riga dello stato), riga 3 di `memory.md` e `docs/mancante.md`.
2. Le voci di `## Non rilasciato` del CHANGELOG diventano la sezione della versione. Novita' in `app/src/lib/novita.ts`: titolo e testo nelle sei lingue, data e ora UTC; restano le ultime 40 (una prova controlla che la prima sia della versione di `app/package.json`).
3. Compilazione: `cargo check --workspace` (core e `desktop/src-tauri`), `cd app && npx tsc --noEmit`. Commit e push.
4. **Windows sul PC** (dal 2026-11-12, chiave privata in `%USERPROFILE%\.aurora-signing\aurora.key`, senza password): `.\scripts
ilascio-windows.ps1`
   compila interfaccia e programma con Hamlib e `cty.dat`, firma gli aggiornamenti e mette in `dist-release\` l'installatore `Aurora_X.Y.Z_x64-setup.exe`,
   il suo `.sig` e `SHA256SUMS.txt`. Prima si controlla che la chiave sia quella giusta: la sua `aurora.key.pub` deve essere uguale a
   `desktop/updater.pub` (lo script controlla che `updater.pub` e `tauri.conf.json` coincidano). Se la chiave si perde, le copie installate
   non accettano piu' gli aggiornamenti: tenerne una copia di sicurezza fuori dal PC.
5. Tag `vX.Y.Z`, push del tag e `gh release create vX.Y.Z --title "Aurora X.Y.Z" --notes-file ...` **con i file** di `dist-release\` piu' una copia
   dell'installatore chiamata `Aurora-Setup.exe` (il nome fisso dei collegamenti del sito). Con installatore e `.sig` gia' allegati la release
   puo' essere subito l'ultima.
   - **Windows** (`.github/workflows/windows.yml`): il lavoro `pc` vede installatore e `.sig` gia' allegati e salta la compilazione.
     Senza (release creata vuota) compila come prima con la chiave nei segreti (`AURORA_TIMBRO_AGGIORNAMENTI_AURORA_KEY`).
   - **Linux** e **macOS**: allegano AppImage, `.deb`, `.dmg` e i loro `SHA256SUMS` (gratuiti: il repository e' pubblico).
   Controllare dopo 10-15 minuti che Linux e macOS siano verdi.
   **Note multilingua**: le leggono da tutto il mondo. In cima la sezione del CHANGELOG in inglese, poi un blocco
   `<details><summary>🇮🇹 Italiano</summary>` … `</details>` per italiano, tedesco, francese, spagnolo, portoghese e olandese.
6. Ora vera nelle Novita' (`gh release view vX.Y.Z --json publishedAt`), commit e push; pagina web con `scripts/pubblica-sito.ps1`.
7. File dei paesi: i lavori di GitHub (e `rilascio-windows.ps1` sul PC) scaricano da soli `cty.dat` da country-files.com in `desktop/src-tauri/cty/` e si fermano se non ci riescono.
8. Linux: niente da fare. Alla pubblicazione della release GitHub Actions (`Linux`) allega da sola `Aurora-Linux-x86_64.AppImage`, `Aurora-Linux-amd64.deb` e `SHA256SUMS-linux.txt`; si controlla solo che il lavoro sia verde.

## Velocita' della compilazione

- `D:\progetti` e `D:\build` sono esclusi da Windows Defender (impostazione permanente fatta dal proprietario). Senza esclusione la compilazione si bloccava in attesa del disco.
- Durante la compilazione dell'installatore non devono girare altre compilazioni (agenti, prove): si mettono in pausa. Si compila con 9 processi (`jobs = 9` in `~/.cargo/config.toml`): con 18 la memoria finiva e Aurora aperta si chiudeva (`memory allocation ... failed`, 2026-11-08).
- Prima di `git checkout` del commit nella cartella di compilazione `D:\build\Aurora-0.11.1` si annullano i file rigenerati (`git checkout -- .`, es. `Cargo.lock`) e si controlla che `HEAD` sia il commit giusto: un checkout fallito ricompila la versione vecchia.
