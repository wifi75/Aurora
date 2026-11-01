# Compiti fissi a ogni versione

Ogni versione che aggiunge, modifica o corregge qualcosa si chiude con questi passi. Una versione non e' finita finche' non sono fatti tutti.

## Nel programma

1. **CHANGELOG.md**: le voci sotto `## Non rilasciato` diventano la sezione della nuova versione, con la data.
2. **Novita'** (`app/src/lib/novita.ts`): una voce per ogni funzione nuova, modificata o corretta, nelle sei lingue (it, en, de, fr, es, pt), con data e ora UTC del rilascio su GitHub. Restano le ultime 40: si tolgono le piu' vecchie. Dopo la pubblicazione si corregge l'ora con quella vera della release (`gh release view vX --json publishedAt`).
3. **Guida** (`app/src/i18n/guide.ts`): ogni funzione nuova o cambiata va spiegata nella Guida, nelle sei lingue, con passi numerati; i nomi di schede e pulsanti si citano con `[[chiave]]`. Le parti non piu' vere si correggono.

## Su GitHub

4. **Pagina web** <https://wifi75.github.io/Aurora/>: il sorgente e' `docs/index.html` (sei lingue). Le funzioni nuove vanno aggiunte anche li', poi si pubblica con `scripts/pubblica-sito.ps1` (aggiorna il ramo `gh-pages`, da cui GitHub Pages serve il sito; le modifiche a `main` non lo toccano). Dopo un minuto si controlla che la pagina risponda e che la distribuzione non sia in errore.
5. **README** nelle sei lingue (`README.md` in inglese, `README.it.md`, `.de`, `.fr`, `.es`, `.pt`): riga dello stato con la versione, badge con le versioni vere (Rust, librerie, numero dei test), funzioni nuove nella sezione Stato.
6. **Release**: installatore firmato, firma, `SHA256SUMS.txt` e la copia con nome fisso `Aurora-Setup.exe`, cosi' il link <https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe> scarica sempre l'ultima versione.

## Controlli prima di pubblicare

7. Nessun riferimento ad assistenti di scrittura del codice nei file, nei messaggi di commit, nelle note di release e nei nomi: `git log --all --format=%B | grep -iE 'cl[a]ude|anthr[o]pic|co-authored-by'` deve restare vuoto, come `git grep -iE 'cl[a]ude|anthr[o]pic'`.
8. Nessun dato personale vero: `git grep -iE "jn55te|gmail"` deve restare vuoto; il nominativo IU3WJQ solo come firma d'autore.
9. Il codice compila (`cargo check --workspace`, `npx tsc --noEmit`); le prove dal vivo con la radio le fa il proprietario.
