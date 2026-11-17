# Il pacchetto di assistenza

Il pacchetto e' lo zip che Aurora manda a chi la sviluppa (scheda **Impostazioni › Accesso e assistenza › Assistenza**, pulsante «Manda il pacchetto a chi
sviluppa Aurora», oppure dopo un crash). Chi lo manda dice a voce o per iscritto il **codice breve**; lo zip sta nell'archivio R2 `aurora-rapporti`
(`bundle/AAAA-MM-GG/<uuid>.zip`), il codice nella tabella `pacchetti` del database D1. Per scaricarlo: `npx wrangler r2 object get
aurora-rapporti/<chiave> --remote --file pacchetto.zip` (la chiave e' nella colonna `chiave`). Il codice e le sue regole: `servizi/rapporti/README.md`.

**Senza dati personali.** Prima dell'invio Aurora toglie nominativi, locatori, percorsi con il nome utente, indirizzi IP, email e codici (`rapporti::pulisci`).
I minidump, il log dei QSO e `ALL.TXT` restano sul PC. Qui sotto non si legge mai il nominativo ne' il locatore, ne' le chiavi dei servizi.

## Cosa c'e' dentro (schema 2, dalla 0.11.287)

| File | Cosa dice |
|---|---|
| `info.json` | Versione, sistema, antivirus, motore, radio, PTT, CAT, audio; l'elenco dei file di diagnostica; quanti minidump sono rimasti sul PC |
| `logs/aurora.log` | Il registro di Aurora, ripulito (avvisi, errori e le righe che non parlano di nominativi) |
| `installazione.log` | Come e' stato installato (WebView2, file non copiati, antivirus), se c'e' |
| `crash/*.txt` | I rapporti dei guasti e dei blocchi (non i `.dmp`) |
| `diagnostica/problemi.txt` | Il registro **riassunto**: avvisi ed errori che si ripetono (quante volte, da quando a quando), riavvii di Aurora, tempi di decodifica |
| `diagnostica/cat-comandi.log` | I **comandi mandati alla radio** (a `rigctld`) con le risposte e il tempo di ognuno: l'inizio dell'ultimo collegamento (primi 300) e gli ultimi 600; gli errori sono marcati con «!» |
| `diagnostica/radio.json` | Come e' impostata la radio (modello, porta, velocita', PTT, split, modo scritto, dati in piu'...), lo stato di adesso e il VFO secondo Hamlib |
| `diagnostica/ambiente.json` | Il PC (processore, core, RAM), Aurora (versione, da quanto e' accesa, motore), l'audio (dispositivi e quello in uso, errori di apertura), cosa sta facendo il motore (modo, frequenza, TX), le variabili `AURORA_*` |
| `diagnostica/interfaccia-pc.json`, `interfaccia-telefono.json` | Quello che la **pagina** sa: finestra, schermo, scala, tema, lingua, pagina aperta, scheda grafica, browser e **gli ultimi 30 errori della pagina** (anche quelli di React). Dell'archivio del browser solo i nomi e le dimensioni |

## Dove sta nel codice

- Il pacchetto lo compone `Rapporti::pacchetto_crash` (`core/crates/aurora/src/rapporti.rs`); i file di diagnostica li prepara `aurora::diagnostica::raccogli`
  (`core/crates/aurora/src/diagnostica.rs`), collegata con `Rapporti::imposta_diagnostica` all'avvio del server.
- La traccia dei comandi e' in `core/crates/rig-control/src/trace.rs`: `RigctldClient::ask_raw` e `vfo_info` la riempiono; anche l'apertura e la chiusura
  del collegamento e di `rigctld` vi lasciano una nota.
- L'interfaccia manda i suoi dati con `app/src/lib/uiDiagnostics.ts` (`POST /api/v1/diagnostics/ui-snapshot`, al massimo 16 KB, uno per origine) all'avvio,
  quando la finestra cambia, ogni 5 minuti e subito prima di mandare il pacchetto.

## Come leggerlo

1. `diagnostica/problemi.txt`: cosa si ripete e da quando; i riavvii spiegano i «dopo il riavvio succede...».
2. `diagnostica/radio.json` e `cat-comandi.log`: la radio, comando per comando. Cercare «!» e i comandi `V`, `S`, `F`, `M`, `X` attorno all'apertura del
   collegamento («# collegato a rigctld») e alla chiusura («# chiusura del collegamento»).
3. `diagnostica/interfaccia-*.json` per i problemi di schermo (finestra troppo piccola, tema, errori della pagina).

## Per aggiungere altro

Un file in piu' si aggiunge in `diagnostica::raccogli` (nome, testo): finisce da solo nel pacchetto sotto `diagnostica/`, ripulito dai dati personali,
con il limite di 8 MB per tutto il pacchetto. Non si leggono nominativo, locatore, chiavi o percorsi personali; se serve un dato che li contiene, va
scritto gia' senza (e provato: vedi `il_pacchetto_porta_la_diagnostica_ripulita`).
