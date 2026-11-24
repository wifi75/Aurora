# Compiti fissi a ogni versione

Ogni versione che aggiunge, modifica o corregge qualcosa si chiude con questi passi. Una versione non e' finita finche' non sono fatti tutti.

## Nel programma

1. **CHANGELOG.md**: le voci sotto `## Non rilasciato` diventano la sezione della nuova versione, con la data.
2. **Novita'** (`app/src/lib/novita.ts`): una voce per ogni funzione nuova, modificata o corretta, nelle sei lingue (it, en, de, fr, es, pt), con data e ora UTC del rilascio su GitHub. Restano le ultime 40: si tolgono le piu' vecchie. Dopo la pubblicazione si corregge l'ora con quella vera della release (`gh release view vX --json publishedAt`).
3. **Guida** (`app/src/i18n/guide.ts`): ogni funzione nuova o cambiata va spiegata nella Guida, nelle sei lingue, con passi numerati; i nomi di schede e pulsanti si citano con `[[chiave]]`. Le parti non piu' vere si correggono.

## Su GitHub

4. **Portale web** <https://aurora.iu3wjq.com/> (senza `/it`): il progetto «Portale web Aurora» lo tiene aggiornato; a ogni versione con funzioni nuove gli si passano le novita' (testi in `app/src/lib/novita.ts` e `app/src/i18n/guide.ts`). Dopo la pubblicazione si controlla che la pagina risponda.
5. **README** nelle sei lingue (`README.md` in inglese, `README.it.md`, `.de`, `.fr`, `.es`, `.pt`): riga dello stato con la versione, badge con le versioni vere (Rust, librerie, numero dei test), funzioni nuove nella sezione Stato.
6. **Release**: tag e release su GitHub senza allegati; installatore, firma, `SHA256SUMS.txt` e la copia con nome fisso `Aurora-Setup.exe` li allega GitHub (lavoro Windows), cosi' il link <https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe> scarica sempre l'ultima versione. Si controlla che i tre lavori (Windows, Linux, macOS) finiscano verdi. Le note finiscono sempre con la riga `Official site, manuals and downloads: https://aurora.iu3wjq.com` (vedi `docs/rilascio.md`).

## Controlli prima di pubblicare

7. Nessun riferimento ad assistenti di scrittura del codice nei file, nei messaggi di commit, nelle note di release e nei nomi: `git log --all --format=%B | grep -iE 'cl[a]ude|anthr[o]pic|co-authored-by'` deve restare vuoto, come `git grep -iE 'cl[a]ude|anthr[o]pic'`.
8. Nessun dato personale vero: `git grep -iE "jn55te|gmail"` deve restare vuoto; il nominativo IU3WJQ solo come firma d'autore.
9. Il codice compila (`cargo check --workspace`, `npx tsc --noEmit`); le prove dal vivo con la radio le fa il proprietario.

## Dopo la pubblicazione

10. **Antivirus** (non a ogni versione, deciso il 2026-11-17): il controllo su <https://www.virustotal.com/> serve solo al badge «Virus free» del README, che dichiara la versione controllata, e non protegge nessuno. Si rifa' solo quando un utente segnala un antivirus o ogni tanto: si carica `Aurora_X.Y.Z_x64-setup.exe`, si guardano i risultati e, per ogni antivirus che lo segnala, si manda la segnalazione di falso positivo col suo modulo (Avast e AVG, che hanno lo stesso motore: <https://www.avast.com/false-positive-file-form.php>; Microsoft Defender: <https://www.microsoft.com/wdsi/filesubmission>; gli altri dal loro sito), con il link della release e del sorgente. Il badge si aggiorna con il link del controllo nuovo solo se i rilevamenti sono zero. Se i file di Hamlib finiscono in quarantena, l'avviso del programma e la sezione della Guida spiegano come escludere le cartelle.
