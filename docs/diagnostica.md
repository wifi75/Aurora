# Diagnostica: log, blocchi e crash

Aurora per Windows registra da sola cosa succede, senza impostazioni da accendere. Tutto sta nella cartella dei dati,
di norma `%APPDATA%\Aurora` (con `AURORA_DATA_DIR` nell'ambiente, quella cartella).

## Dove guardare

| File | Cosa contiene |
|---|---|
| `logs\aurora.log` | il log del programma (oltre 5 MB diventa `aurora.vecchio.log`); ogni panic vi lascia una riga, ogni 10 minuti le risorse del processo |
| `crash\hang-AAAAMMGG-HHMMSS.txt` | il programma si e' bloccato: ora, versione, risorse, da quanti secondi tacciono i battiti del runtime e della finestra |
| `crash\hang-AAAAMMGG-HHMMSS.dmp` | minidump dello stesso blocco: stack e stato di tutti i thread, la memoria a cui puntano, maniglie e moduli (pochi MB) |
| `crash\crash-AAAAMMGG-HHMMSS.txt` / `.dmp` | eccezione non gestita (es. accesso a memoria non valida): codice, indirizzo, minidump con il thread colpevole |
| `crash\panic-AAAAMMGG-HHMMSS.txt` | panic Rust: messaggio, thread, punto nel sorgente, backtrace e indirizzi grezzi dello stack |
| `crash\stderr.log` | l'errore standard del processo: qui Rust scrive il motivo di ogni `abort` (allocazione di memoria fallita, panic che non puo' risalire, errori fatali del runtime). Ricomincia oltre 1 MB |

Si tengono gli ultimi 10 dump e gli ultimi 50 file di testo; i piu' vecchi si cancellano da soli.

La cartella si apre da **Impostazioni > Aggiornamenti > Diagnostica > «Apri la cartella dei crash»** (solo nel programma
Windows e dal PC stesso), oppure scrivendo `%APPDATA%\Aurora\crash` nella barra di Esplora risorse.

## Come funziona

- **Guardiano dei blocchi**: il runtime asincrono (dove gira il core) e il thread della finestra battono ogni 2 s. Un thread a
  parte controlla i battiti; se uno tace da oltre 20 s scrive il minidump e il `.txt` direttamente su disco (senza passare dal
  logger, che potrebbe essere fermo anche lui), poi una riga di errore nel log. Un solo dump per blocco: ne scrive un altro solo
  dopo che i battiti sono ripartiti.
- **Eccezioni non gestite**: un filtro (`SetUnhandledExceptionFilter`) scrive dump e testo, poi lascia proseguire Windows.
- **Panic**: oltre alla riga nel log, il file `panic-….txt`.
- **Prova dal vivo**: con `AURORA_PROVA_BLOCCO=1` nell'ambiente, circa un minuto dopo l'avvio la finestra si ferma per 30 s:
  deve comparire un `hang-….dmp` con il suo `.txt`.

## Cosa non si cattura: i fast-fail

Un arresto «BEX64» con codice `c0000409` e' un *fast-fail* (`__fastfail`): Windows chiude il processo senza passare da
nessun filtro, quindi nessun programma puo' scrivere un dump di se stesso in quel momento. Il sottocodice 7
(`FAST_FAIL_FATAL_APP_EXIT`) e' quello di `abort()`, che Rust usa per allocazioni fallite, panic in punti che non possono
risalire e errori fatali del runtime: il motivo, quando Rust lo scrive, finisce in `crash\stderr.log`. Se il fast-fail segue
un blocco (come il 1 novembre 2026: blocco alle 13:01, chiusura alle 13:06), il guardiano ha gia' scritto il `hang-….dmp`
del blocco.

Senza diritti di amministratore Windows non offre un dump locale dei fast-fail per un programma installato normalmente:
- `WerRegisterAppLocalDump` vale solo per le app impacchettate (MSIX); Aurora la chiama comunque e, se rifiutata, prosegue;
- `WerRegisterRuntimeExceptionModule` richiede una DLL a parte registrata nel registro di Windows Error Reporting;
- i `LocalDumps` di Windows Error Reporting si configurano in `HKLM` (serve l'amministratore).

Chi ha i diritti di amministratore puo' far tenere a Windows un dump completo anche dei fast-fail, una volta sola:

```
reg add "HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\aurora-desktop.exe" /v DumpFolder /t REG_EXPAND_SZ /d "%APPDATA%\Aurora\crash" /f
reg add "HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\aurora-desktop.exe" /v DumpType /t REG_DWORD /d 1 /f
reg add "HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\aurora-desktop.exe" /v DumpCount /t REG_DWORD /d 5 /f
```

(`%APPDATA%` va scritto con il percorso vero dell'utente se il comando gira da un altro account.) In ogni caso Windows tiene
il suo rapporto, senza dump, in `%LOCALAPPDATA%\Microsoft\Windows\WER\ReportArchive` (cartelle `AppCrash_aurora-desktop…`
con `Report.wer`), e il Visualizzatore eventi lo riporta in Registri di Windows > Applicazione (origine «Application Error»).

## Come mandarli

1. Apri la cartella dei crash dal pulsante o da Esplora risorse.
2. Seleziona i file con la data e l'ora del problema (`.txt` e `.dmp`), `stderr.log` e, dalla cartella `logs`, `aurora.log`.
3. Tasto destro > Comprimi in file ZIP e allega lo ZIP alla segnalazione su GitHub (Issues) o mandalo all'autore.

I dump contengono la memoria del programma: possono includere nominativi, locatore e impostazioni. Mandali solo a chi li
deve leggere, non in un'issue pubblica. I file `.txt` si possono leggere prima di mandarli.

Per leggere un dump: WinDbg (`!analyze -v`, `~*k` per gli stack di tutti i thread) o Visual Studio, con i simboli della stessa
build (la versione e' nel `.txt`).
