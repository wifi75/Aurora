# Report Cloudflare — 12 novembre 2026 (mezzogiorno)

Dati letti dal database D1 `aurora-rapporti` e dai pacchetti R2 `aurora-rapporti` (installazioni anonime, errori, pacchetti dopo un crash).
Nessun IP, nessun nominativo. Pacchetti scaricati solo sul PC, non nel repository.

## Installazioni
| | |
|---|---|
| Installazioni di sempre | **436** |
| Attive oggi (12/11, fino a mezzogiorno) | **64** (0.11.238: 24, 0.11.239: 23, 0.11.234: 6, altre) |
| Attive negli ultimi 7 giorni | **418** |
| Attive per giorno | 8/11: 92 · 9/11: 180 · 10/11: 139 · 11/11: 125 |

Paesi negli ultimi 7 giorni: Italia 88, Russia 44, Stati Uniti 39, Spagna 34, Regno Unito 24, Francia 23, Polonia 17, Germania 15,
Kazakistan 14, Belgio 12, Paesi Bassi 11, Cechia 9, Brasile 7, poi altri 35 paesi. Sistemi: Windows 11 e 10 quasi tutti, **20 Linux** dal 10/11.

## Crash e blocchi (pacchetti dal 11/11)
1. **WebView2 mancante** (pacchetto `9AE40316`, Windows 10 Pro 22H2, 0.11.239): sette panic in mezz'ora con
   `Could not find the webview runtime, make sure it is installed` (`tauri-2.12.5/src/app.rs:1444`). Aurora riparte e cade di nuovo.
   Il pacchetto non dice `webviewInstallMode` (quindi il bootstrapper scaricato all'installazione), che evidentemente non ha installato
   WebView2 o e' stato tolto dopo. **Da fare**: controllare WebView2 prima di avviare Tauri e, se manca, una finestra di Windows che spiega
   e porta al download (o lo installa), invece del panic a ripetizione; valutare `embedBootstrapper` nell'installatore.
2. **Riavvio dopo l'aggiornamento** (pacchetto `BD76D06F`, 0.11.239, Windows in russo): `Start-Process` dello script di riavvio non trova il
   file (`C:\...\AppData\Local\Aurora...`). **Da verificare** lo script di aggiornamento con percorsi non standard.
3. Blocchi `hang-*` e panic `tao ... event handler is re-entrant` su versioni vecchie (0.11.224-0.11.230): gia' superati.
4. `Failed to unregister class Chrome_WidgetWin_0` in stderr: avviso innocuo di WebView2.

## Errori sulle versioni 0.11.238-0.11.239
- **Trasmissione fallita: «riproduzione oltre la durata prevista»** (2 installazioni, solo 0.11.239, `tx-native/src/scheduler.rs:614`):
  nuovo con la 0.11.239. **Da indagare** (cambi della 0.11.239 su Tune e durata delle onde).
- **rigctld chiude la connessione** (una installazione russa 9 volte, altre in spagnolo, polacco, olandese, inglese) ed `errore di Hamlib (-20)`:
  rigctld che cade o radio che non risponde. Da guardare con il log dell'utente.
- Decodifica lenta (3 installazioni), porta del server gia' usata (2), FT-991(A) Link scelto per il PTT senza collegarlo (2), Cat4OM non
  raggiungibile (2), uscite audio non disponibili, un Xrun USB.
- **Testo dei rapporti ripulito troppo**: «nessuna `<personale>` Cat4OM collegata» e «Intel® Smart Sound Technology for `<nominativo>` Audio»:
  la pulizia dei dati personali toglie anche parole normali («radio», un nome di dispositivo). Da correggere nella pulizia.
- I testi del core dicono ancora «Impostazioni → Radio»: vanno portati ai percorsi nuovi delle Impostazioni ad albero.

## Segnalazioni su GitHub (12/11)
- **#46** (ON7KEC): in CW l'AFC non funziona. Passato al lavoro in corso su PSK e CW.
- **#27** (UN7MBV): ringrazia per la mappa; dice che col solo rumore l'RTTY stampa caratteri (risolto dallo squelch su S/N del nuovo
  ricevitore RTTY) e che il livello Rx e' scomodo da regolare; riporta che sul forum locale c'e' chi chiede di non caricare il programma di
  funzioni in piu'.

## Correzioni fatte lo stesso giorno (commit locali)
- WebView2 mancante: controllo prima di aprire la finestra, messaggio di Windows nelle sei lingue con il link all'installazione di Microsoft e
  uscita pulita, niente piu' crash a ripetizione (`desktop/src-tauri/src/webview_check.rs`).
- «Riproduzione oltre la durata prevista»: tolleranza del 2 % sulla durata delle onde lunghe (anche per il watchdog del PTT) e numeri nel
  messaggio (durata vera, durata prevista, frequenza di campionamento) per capire la causa dai prossimi rapporti.
- Riavvio dopo l'aggiornamento: se il programma non c'e' a fine installazione si riapre l'installatore con le sue finestre.
- Pulizia dei rapporti: i nomi generici (User, Radio, Admin...) non sono dati personali; i valori corti si tolgono solo come parole intere.
- Messaggi del motore con i percorsi nuovi delle Impostazioni (Stazione › Radio, Accesso e assistenza).
- rigctld che si chiude appena collegato: il messaggio lo dice e riporta le sue ultime righe.
