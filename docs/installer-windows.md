# Aurora per Windows: installazione, aggiornamenti, rilascio

Il programma Windows (cartella `desktop/`) e' Tauri 2: dentro c'e' il **core di Aurora** e una finestra che mostra la stessa pagina che si apre dal telefono. Resta nel **vassoio** di Windows, parte con Windows e si aggiorna quando esce una versione nuova.

## Per chi lo usa

**Installazione.** Si lancia `Aurora_X.Y.Z_x64-setup.exe`: installatore con logo, autore, licenza (GPL) e barra laterale, in italiano o inglese. Si installa per l'utente corrente (senza chiedere i permessi di amministratore) e mette Aurora nel menu Start.

**Primo avvio.** Aurora cerca da sola JTDX (`%LOCALAPPDATA%\JTDX\JTDX.ini`), il suo log, il file dei paesi (`cty.dat`), l'archivio di Log4OM2 e l'ingresso audio scelto in JTDX. Senza JTDX parte con il simulatore. Si mette tra i programmi che partono con Windows (si toglie dal vassoio).

**Trasmissione chiusa.** Come sempre: si apre solo con `AURORA_ALLOW_TX=1`. Le impostazioni si scrivono nel file `aurora.env` (dal vassoio: *Apri la cartella dei dati*; si crea al primo avvio, tutto commentato) e valgono al prossimo avvio. Le variabili d'ambiente `AURORA_*` hanno la precedenza sul file. Due voci utili: `AURORA_AUTOSTART=0` tiene Aurora fuori dai programmi che partono con Windows (al primo avvio ci si mette da sola), `AURORA_DATA_DIR` sceglie un'altra cartella dei dati.

| Dove | Cosa |
|---|---|
| `%APPDATA%\Aurora\` | i dati (log, dispositivi associati, impostazioni, storico), `aurora.env` e `logs\aurora.log` |
| vassoio, clic sinistro | apre la finestra |
| vassoio, menu | Apri Aurora, **Installa Aurora X** (si accende quando c'e' una versione nuova), Cerca aggiornamenti, Avvia con Windows, Apri la cartella dei dati, Esci |
| chiudere la finestra | la nasconde: Aurora resta nel vassoio, col core acceso |

Una seconda copia lanciata per errore non apre un secondo core: mostra la finestra della prima.

**Dal telefono.** L'indirizzo e' sempre `http://<IP del PC>:8787`. La finestra del programma sul PC entra da sola (nessuna associazione: vale solo per le richieste che arrivano da questo stesso PC, `AURORA_TRUST_LOCAL=0` la toglie); dal telefono l'accesso richiede il codice di associazione (l'accesso senza codice si accende solo con `AURORA_OPEN_ACCESS=1`).

## Aggiornamenti

1. Aurora controlla ogni tanto il repository dei rilasci (Impostazioni → Aggiornamenti: repository, ogni quante ore, token).
2. Quando c'e' una versione nuova arriva una **notifica di Windows**, la barra in alto nella pagina propone *Installa ora* e la voce del vassoio diventa *Installa Aurora X*.
3. *Installa*: scarica l'installatore e la **firma**, controlla la firma con la chiave pubblica di Aurora (`desktop/updater.pub`, incorporata nel programma) e **solo se e' valida** lo avvia; Aurora si chiude e si riapre da sola. Con una firma sbagliata o mancante non si installa nulla. Non si installa mai mentre si trasmette.

**Repository pubblico.** `wifi75/Aurora` e' pubblico dal 2026-10-24: le release si vedono e si scaricano senza token (verificato con `curl` senza credenziali). Il campo token in Impostazioni → Aggiornamenti serve solo per un repository privato (token di **sola lettura**, *Contents: read*; resta nel file `update.json` dei dati).

## Per chi rilascia

La chiave **privata** di firma sta fuori dal repository: `%USERPROFILE%\.aurora-signing\aurora.key` (senza password). **Va copiata in un posto sicuro**: se si perde, le copie gia' installate non potranno piu' aggiornarsi (bisognerebbe reinstallare a mano con una chiave nuova). La pubblica e' `desktop/updater.pub`.

Per fare una versione nuova:

1. Alzare il numero di versione in `core/Cargo.toml`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/package.json` e `app/package.json` (e, se cambia, `.\desktop\scripts\genera-immagini-installer.ps1 -Versione X.Y.Z` per la barra laterale dell'installatore).
2. Test, commit, tag `vX.Y.Z` e push (come per ogni rilascio).
3. `.\scripts\rilascio-windows.ps1`: controlla che le versioni coincidano, compila l'interfaccia e il programma, firma e scrive in `dist-release\` l'installatore, la sua `.sig` e `SHA256SUMS.txt`. Non pubblica nulla.
4. Pubblicare i tre file nella release: `gh release upload vX.Y.Z dist-release\Aurora_X.Y.Z_x64-setup.exe dist-release\Aurora_X.Y.Z_x64-setup.exe.sig dist-release\SHA256SUMS.txt` (il nome dell'installatore deve finire in `-setup.exe` e la firma deve avere lo stesso nome piu' `.sig`: e' cosi' che Aurora li trova).

`.\scripts\rilascio-windows.ps1 -SoloControllo` verifica soltanto le versioni.

## Come e' fatto

- `desktop/src-tauri`: workspace Cargo a parte (le dipendenze di Tauri non appesantiscono `cargo test --workspace` del core), che usa il crate `aurora` come libreria.
- `settings.rs`: rilevamento di JTDX/Log4OM2 e lettura di `aurora.env` (funzioni pure con prove).
- `installer.rs`: scarico, verifica della firma e avvio dell'installatore. Il core (`remote-api`) espone `POST /api/v1/update/install` solo da loopback e solo se il guscio gli ha dato un installatore (`can_install`).
- Prove: `cd desktop\src-tauri; cargo test` (comprese la firma del pacchetto appena fatto in `dist-release\` con la chiave incorporata e il rifiuto di un pacchetto alterato).

## Cosa e' stato provato (2026-10-24)

- Installatore da 6,8 MB, firmato; la firma torna con la chiave pubblica incorporata e un pacchetto alterato viene rifiutato.
- Installazione silenziosa (`/S`) in una cartella a parte: programma, interfaccia e disinstallatore al loro posto; voce nel registro con versione ed editore; il programma installato parte, mostra la finestra «Aurora» e serve l'interfaccia; la seconda copia non apre un secondo core; disinstallazione che toglie cartella, voce di registro e collegamenti.
- Non provato: l'aggiornamento vero da una versione alla successiva (serve una release con installatore e firma sul repository, e il token per il repository privato) e le voci del vassoio con il clic.
