# Integrazione di Aurora con altri log e utility (UDP, ADIF, TCP, servizi in rete)

Ricerca del 2026-11-01: questo documento dice **cosa Aurora fa oggi**, **con chi puo' parlare** e **cosa manca**. Aggiornato lo stesso giorno con l'**invio diretto dei QSO** a Wavelog/Cloudlog, ClubLog, QRZ.com Logbook e LoTW (sezione 6).
Ricerca del 2026-11-01, aggiornata con il lavoro dello stesso giorno (multicast, destinatari con le loro opzioni, Highlight Callsign, Location, Configure, Clear e Switch Configuration in ingresso, XML di N1MM, coda del QSO Logged di WSJT-X 2.x): vedi la sezione 0. Il documento dice **cosa Aurora fa**, **con chi puo' parlare** e **cosa manca**.
Le righe marcate **(non verificato)** non hanno una fonte ufficiale o sono state ricavate da forum e guide di terzi: vanno provate dal vivo.
Nei programmi e nei nominativi di esempio si usano valori di fantasia (K1ABC, locatore FN42).

## 0. Stato attuale (dopo l'integrazione)

Fatto, con il motore proprio (`AURORA_ENGINE=native`):

- **Riquadro «Integrazioni»** nella scheda Impostazioni > Segnalazioni: un elenco di destinatari UDP (al piu' 16), ciascuno con nome, indirizzo, porta, acceso/spento e le sue scelte: **flusso WSJT-X** (binario), **Logged ADIF (12)**, **ADIF su UDP** (testo, come il secondo server di JTDX), **XML N1MM** (`contactinfo`), **multicast** con **TTL** (1-255, di base 1) e **interfaccia di uscita** (indirizzo IPv4 di questo PC; vuota = sistema). Ogni voce ha il «?» e i testi in sei lingue; l'elenco si salva da solo come le altre impostazioni (voce `UdpDestinations` del file `settings-motore.json`, JSON) e vale subito: l'esportatore rilegge i destinatari a ogni QSO, l'emettitore si riavvia in pochi istanti.
- **Valori iniziali dall'ambiente**: alla prima apertura (o se la voce manca nel file) l'elenco nasce dagli indirizzi di `AURORA_UDP_FORWARD` oltre il primo (flusso WSJT-X, con il Logged ADIF se `EnableUDP1adifSending` e' acceso) e da `AURORA_EXPORT_UDP` (ADIF su UDP). Dopo vale il file: le variabili non si aggiungono piu' da sole (prima erano destinatari fissi che non si potevano togliere). Il primo indirizzo di `AURORA_UDP_FORWARD` resta il valore iniziale del server UDP primario.
- **Multicast**: per ogni combinazione di TTL e interfaccia l'emettitore apre un socket proprio (socket2: `IP_MULTICAST_TTL`, `IP_MULTICAST_IF`, ritorno locale acceso), scrive al gruppo e ascolta su quel socket i comandi che i programmi rimandano al mittente (come WSJT-X: nessuna iscrizione al gruppo da parte di chi trasmette). L'unicast di prima resta com'era (un socket per famiglia di indirizzi, sul loopback se tutti i destinatari unicast sono sul PC). Il server UDP primario puo' essere anche un gruppo multicast: in quel caso TTL 1 e interfaccia del sistema. IPv6: hop limit si', interfaccia no (decide il sistema). L'ADIF su UDP e l'XML verso un gruppo multicast partono con il TTL del sistema (1).
- **Comandi in ingresso** (tutti solo con «Accetta richieste UDP», `AcceptUDPRequests`, acceso; spento non arriva niente, quindi nessuna trasmissione da comandi non accettati):
  - **Highlight Callsign (13)**: `QColor` letto per intero (RGB, HSV, HSL, CMYK, RGB esteso convertiti in RGB); evento `highlight_callsign` sul bus verso la pagina (`call`, `background` e `foreground` come `#rrggbb` o `null`, `last_only`). La lista dei messaggi colora le righe che contengono il nominativo (anche tra `<>`), con «solo l'ultima» solo la riga piu' recente; il colore vince sui colori normali (anche sulla riga «mi chiama»). Due colori non validi (`QColor()`) tolgono l'evidenziazione. Le evidenziazioni in vigore (al piu' 500) si rimandano a chi apre la pagina dopo.
  - **Clear (3)**: evento `clear_decodes` (`window` 0, 1 o 2; senza il campo vale 0): la pagina svuota le liste come «Cancella» e il motore dimentica i messaggi ricordati per il doppio clic. Aurora ha una sola lista: la finestra indicata non cambia l'effetto.
  - **Location (11)**: come `MainWindow::locationChange` di WSJT-X (oltre sei caratteri serve il prefisso `GRID:`; poi 4, 6 o 8 caratteri Maidenhead). Vale per la sessione, non si salva: Status (`de_grid`), messaggi generati dalla sequenza, QSO Logged e Logged ADIF (`MY_GRIDSQUARE`) usano il nuovo locatore. Il file ADIF e l'ADIF su UDP dell'esportatore restano con il locatore delle impostazioni.
  - **Configure (15)**: come `MainWindow::remote_configure`: modo (solo FT8 e FT4, come il pulsante della pagina: la radio va sulla frequenza del modo nella stessa banda; mai in trasmissione), frequenza Rx (`Rx DF`), DX call e locatore (non abilitano la trasmissione), «genera i messaggi» (Gen.Msgs). Tolleranza, sottomodo, modo veloce e periodo T/R non esistono in FT8/FT4 di Aurora: una riga di log e niente altro. Il protocollo non ha la frequenza Tx: resta il tipo 50 di JTDX.
  - **Switch Configuration (14)**: Aurora non ha configurazioni con nome: una riga di log, nessun effetto.
- **Rimanda i QSO persi** (`resend` del destinatario, crate `qso-export`, modulo `waiting`): per i destinatari su 127.0.0.1 o ::1 con ADIF su UDP o XML N1MM, prima di mandare si guarda nella tabella delle porte UDP del sistema (Windows `GetExtendedUdpTable` escludendo il processo di Aurora, Linux `/proc/net/udp` e `udp6`; sola lettura, nessuna prova di bind). Porta chiusa: il datagramma gia' pronto va in coda (`log-in-attesa.json` nella cartella dei dati, escluso dalle copie di sicurezza); ogni 5 s si ricontrolla e dopo tre controlli di fila con la porta aperta si manda, 20 per volta a 50 ms l'uno dall'altro, nell'ordine. Mentre ci sono QSO in coda per un destinatario anche i nuovi vanno in coda (l'ordine resta). Opzione spenta o destinatario tolto: la coda parte subito come prima. Sette giorni al piu', 2000 QSO al piu'. Altri sistemi o indirizzi di altri PC: nessuna coda, invio subito. Non vale per il flusso binario di WSJT-X.
- **XML stile N1MM** (`contactinfo`, porta di base 12060) a ogni QSO, per i destinatari che lo vogliono: nominativi, banda (MHz di N1MM), frequenze in decine di Hz, modo, rapporti, locatore, nome, potenza, commento, `ID` di 32 cifre esadecimali stabile per lo stesso QSO, `app` = `Aurora`. **Non verificato dal vivo** con HRD Logbook.
- **QSO Logged (5)**: dopo il locatore proprio ora ci sono anche **Exchange sent**, **Exchange received** e **ADIF Propagation mode** dello schema di WSJT-X 2.x (vuoti: Aurora non fa gare). Chi legge il formato di JTDX ignora i campi in piu'; il lettore di Aurora accetta entrambi.

Da provare dal vivo (vedi anche il rapporto del lavoro): GridTracker 2 e QLog insieme su 239.255.0.1:2237 con interfaccia 127.0.0.1; colori da CQRLOG o GridTracker; Location da un programma GPS; Configure da N1MM+; XML verso HRD Logbook su 12060; MacLoggerDX su un altro computer con interfaccia di rete.

## 1. Come parla Aurora oggi

Tre canali distinti, come in JTDX (dettagli in `docs/udp-flussi.md`):

| Canale | Crate | Impostazione | Formato | Destinatari tipici |
|---|---|---|---|---|
| Flusso WSJT-X «primario» | `udp-emitter` (codifica in `udp-bridge`) | `AURORA_UDP_FORWARD` (elenco di indirizzi separati da virgola), `AURORA_UDP_EMIT`, `AURORA_UDP_ACCEPT`, `AURORA_UDP_ACCEPT_REPLY`, `AURORA_UDP_ADIF` | Datagrammi binari WSJT-X (numero magico `0xadbccbda`, schema 2 o 3, identificativo `JTDX`) | GridTracker, JTAlert, MacLoggerDX, QLog, CQRLOG, N1MM+, WaveLogGate... |
| ADIF «secondario» | `qso-export` | `AURORA_EXPORT_UDP` (elenco), `AURORA_EXPORT_FILE` | Record ADIF testuale in un datagramma, senza intestazione e senza numero magico (come `UDP2Server` di JTDX) | Log4OM 2 (127.0.0.1:2237), WaveLogGate (porta 2333), altri in ascolto ADIF |
| Server TCP del log | `reporting::tcp` | impostazioni della scheda Segnalazioni (`TCPServer`) | Un messaggio `<command:3>Log <parameters:N> <record ADIF> ` per QSO, una connessione nuova ogni volta, risposta `NAK` = rifiuto | Programmi che si aspettano il «server TCP» di JTDX (N1MM+ lo usa con JTDX) |

Altri invii verso internet gia' presenti (crate `reporting`): PSK Reporter (IPFIX, `report.pskreporter.info:4739`), DXSummit, eQSL (HTTPS) e, col motore proprio, l'invio diretto dei QSO a Wavelog/Cloudlog, ClubLog, QRZ.com Logbook e LoTW (sezione 6). In ingresso (crate `online-info`, `dx-cluster`, `logbook`): PSK Reporter in lettura, cluster DX in telnet, lettura del database di Log4OM2. Aurora scrive anche un proprio file ADIF (`aurora_log.adi`) e, col motore proprio, un file nel formato di `wsjtx_log.adi`: e' quello che osservano gli strumenti che sorvegliano un file (ad esempio ADIFPUSH per Wavelog/Cloudlog).

### 1.1 Messaggi WSJT-X: cosa emette Aurora

Numeri e nomi dallo schema di `NetworkMessage.hpp` di WSJT-X (direzione vista da WSJT-X: «Out» = lo manda il programma di decodifica, «In» = lo riceve). In Aurora i tipi sono costanti di `core/crates/udp-bridge/src/message.rs`.

| N. | Nome | Direzione nel protocollo | Aurora emette | Aurora accetta | Note |
|---|---|---|---|---|---|
| 0 | Heartbeat | Out e In | **si**, alla partenza e ogni 15 s, schema massimo 3, versione `2.2.159` | solo per adottare lo schema piu' alto dichiarato dal client | Non manda il campo «revisione» (come JTDX) |
| 1 | Status | Out | **si**, a ogni cambiamento (non si rimanda uno identico) | - | Campi fino a `fast_mode`, poi `special_op_mode`, tolleranza, periodo T/R, nome configurazione, messaggio Tx |
| 2 | Decode | Out | **si**, uno per messaggio, `nuovo = true`, modo `~` (FT8) o `:` (FT4) | - | Con `apply_filters` escono solo i messaggi che i filtri di testo lasciano vedere |
| 3 | Clear | Out e In | **si**, alla partenza (come il costruttore di JTDX) | **si**, con «accetta le richieste» | Svuota le liste come «Cancella»; la «finestra» (0, 1, 2) si legge ma Aurora ha una sola lista |
| 4 | Reply | In | - | **si**, solo con `AURORA_UDP_ACCEPT=1` e per i messaggi ammessi da `AURORA_UDP_ACCEPT_REPLY` (1 solo CQ, 2 CQ e 73, 3 tutti) | Ritrova il decode tra gli ultimi 300 per testo, frequenza e ora; i modificatori non sono usati |
| 5 | QSO Logged | Out | **si**, alla registrazione | - | Ordine dei campi di JTDX, poi «Exchange sent/received» e «ADIF Propagation mode» di WSJT-X 2.x (vuoti) |
| 6 | Close | Out (in WSJT-X il client lo gestisce anche in ingresso: **da verificare**) | **si**, alla chiusura | **no** | |
| 7 | Replay | In | - | **si**, sempre (anche con `AURORA_UDP_ACCEPT=0`) | Rimanda i decode correnti (non nuovi) e lo Status |
| 8 | Halt Tx | In | - | **si**, con `AURORA_UDP_ACCEPT=1` | In Aurora fa sempre lo STOP; il flag «solo Abilita Tx» e' letto ma non distinto |
| 9 | Free Text | In | - | **si**, con `AURORA_UDP_ACCEPT=1` | Con `send` trasmette |
| 10 | WSPR Decode | Out | no (Aurora non decodifica WSPR) | - | Non serve |
| 11 | Location | In | - | **si**, con «accetta le richieste» | Locatore della sessione (4, 6, 8 caratteri; `GRID:` oltre sei) |
| 12 | Logged ADIF | Out | **si**, dopo il QSO Logged, al server primario con «Invia il QSO in ADIF» e ai destinatari del riquadro «Integrazioni» che lo vogliono | - | Intestazione ADIF di JTDX (`programid JTDX`) davanti al record |
| 13 | Highlight Callsign | In | - | **si**, con «accetta le richieste» | Colori di sfondo e testo nella lista dei messaggi, «solo l'ultima», tolti con colori non validi |
| 14 | Switch Configuration | In | - | letto, **ignorato** con una riga di log | Aurora non ha configurazioni con nome |
| 15 | Configure | In | - | **si**, con «accetta le richieste» | Modo FT8/FT4, frequenza Rx, DX call e locatore, genera i messaggi; il resto si ignora |
| 50 | (JTDX) Set Tx Freq | In | - | **si**, con `AURORA_UDP_ACCEPT=1`, solo 1..5000 Hz | Tipo non standard di JTDX |
| 51 | (JTDX) Trigger CQ | In | - | **si**, con `AURORA_UDP_ACCEPT=1`, solo se `send` e' vero | Tipo non standard di JTDX |

Il Logged ADIF (12) c'era gia' (spento di fabbrica); ora si sceglie anche per destinatario. L'ascolto di 3, 11, 13, 14 e 15 e' fatto (sezione 0).

Particolarita' dell'emettitore che contano per le integrazioni:
- un socket UDP con porta effimera per i destinatari unicast di una famiglia di indirizzi e uno per ogni gruppo multicast con le sue opzioni (TTL, interfaccia); i comandi tornano alla porta del socket che ha scritto e arrivano tutti allo stesso ascoltatore;
- l'identificativo e' sempre `JTDX`: i comandi con un altro identificativo si scartano;
- ogni destinatario riceve cio' che ha scelto (flusso binario, Logged ADIF; ADIF testuale e XML di N1MM passano dall'esportatore).

## 2. Tabella riassuntiva dei programmi

Legenda stato: **OK** = compatibile oggi con la configurazione indicata (fondata su fonti, non ancora provata dal vivo salvo dove scritto), **poco lavoro** = serve una funzione piccola, **lavoro** = serve una funzione nuova, **?** = fonti insufficienti.

| Programma | Cosa legge (porta e formato di default) | Puo' comandare Aurora? | Stato con Aurora | Che cosa serve |
|---|---|---|---|---|
| GridTracker 2 / 1 | UDP WSJT-X binario, 127.0.0.1:2237 (configurabile, anche multicast); inoltra a Log4OM, N1MM+, ecc. | Reply, Highlight, Free text, Halt Tx (segue `AcceptUDPRequests`) | **OK** (e' il collegamento di oggi) | Highlight (13) per i colori; multicast |
| Log4OM 2 | Ascoltatori UDP configurabili: `JT_MESSAGE` (flusso binario) e `ADIF_MESSAGE` (ADIF testuale); di norma 2237 | non risulta | **OK** con ADIF su UDP (2237); flusso binario OK | niente di obbligatorio |
| JTAlert (HamApps) | Si aggancia al programma WSJT-X/JTDX in esecuzione e alla sua porta; rilancia i pacchetti ricevuti (porta a scelta, es. 2334); inoltra il QSO a DXKeeper, ACLog, Log4OM, HRD, N1MM+; carica su QRZ, eQSL, ClubLog, HRDLog | Reply, Highlight (colori), Halt Tx | **? / lavoro**: non e' chiaro se riconosce un'istanza senza il processo di JTDX/WSJT-X | Provare; probabile necessita' del file di configurazione/processo |
| DXLab Suite (DXKeeper, Commander, SpotCollector) | DXKeeper riceve i QSO da JTAlert; Commander e' il controllo radio per WSJT-X/JTDX (TCP, non UDP WSJT-X) | non risulta | **poco lavoro** via JTAlert; diretto **non documentato** | Verificare ricezione diretta; il controllo radio ha il proprio CAT |
| N1MM+ Logger+ | Finestra «Decode List»: collegamento a due vie, UDP 127.0.0.1:2237 (SO2R radio 2: 2239); con JTDX anche TCP (52001, radio 2: 52006); trasmette XML `contactinfo` su 12060 | Dupe/moltiplicatori come colori nel pannello, doppio clic = nominativo e griglia, Abilita Tx | **poco lavoro**: Status e Decode OK, TCP del log gia' presente | Highlight (13), Reply/Location; verificare l'avvio con percorso a `jtdx.exe` |
| HRD Logbook / Ham Radio Deluxe | «QSO Forwarding»: sezione UDP Receive su 2333 (ricezione QSO da WSJT-X/JTDX, N1MM su 12060); allarmi con multicast | non risulta | **? / poco lavoro**: formato dei QSO da verificare | Provare ADIF su UDP 2333 e QSO Logged; multicast |
| CQRLOG | «Remote mode for wsjt» (da v1.9.1): legge decode e QSO dal flusso UDP binario | Colori (Highlight), Reply, Free text, Halt Tx | **OK** per la lettura; i comandi richiedono 13 e `AcceptUDPRequests` | Highlight (13); porta da allineare nelle impostazioni |
| QLog | Ascolta 2237 il flusso WSJT-X; rilancia su una porta a scelta; notifiche JSON | non risulta | **OK** (non verificato dal vivo) | Multicast se 2237 e' occupata |
| MacLoggerDX | UDP WSJT-X su 127.0.0.1:2237 (modificabile con `defaults write ... wsjtx_udp_port`); registra QSO Logged (5) o Logged ADIF (12); spot da CQ e decode | no (il doppio clic e' di WSJT-X) | **OK** su macOS | Il motore di Aurora gira su Windows: serve rete (broadcast/multicast) per un Mac separato |
| Logger32 | «UDP BandMap» con registrazione automatica dei QSO da WSJT-X/JTDX (guida del 2018); porta non documentata | non risulta | **?** | Prova pratica |
| N3FJP (ACLog e altri) | Nessun UDP WSJT-X diretto: adattatori di terzi leggono UDP e parlano il suo TCP (127.0.0.1:1100) | no | **poco lavoro** con un adattatore (ascolta 2333) | Mandare il flusso/ADIF alla porta dell'adattatore |
| RumLogNG | non trovata una fonte | ? | **?** | Verificare |
| UCXLog | non trovata una fonte | ? | **?** | Verificare |
| Cloudlog / Wavelog (+ WaveLogGate) | WaveLogGate: UDP 2333 (QSO dal flusso primario e dall'ADIF secondario, doppioni entro pochi secondi uniti); invia a Wavelog via API; CAT; QSY da Wavelog | solo QSY verso la radio (HTTP 54321), non Aurora via UDP | **OK**: invio diretto via API (sezione 6) oppure ADIF su UDP verso WaveLogGate | - |
| ClubLog | HTTP POST (real-time: un record ADIF per richiesta); lotti con putlogs | - | **OK** (invio diretto, sezione 6) | Chiave API dell'applicazione da chiedere a Club Log |
| QRZ.com Logbook | `https://logbook.qrz.com/api`, azione INSERT, `KEY` del logbook, abbonamento richiesto, User-Agent obbligatorio | - | **OK** (invio diretto, sezione 6) | - |
| LoTW | Firma con TQSL (certificato) e invio: da riga di comando con `tqsl` | - | **OK** (TQSL lanciato da Aurora, sezione 6) | - |
| eQSL | HTTPS, `ADIFdata` | - | **OK** (gia' in Aurora) | - |
| PSK Reporter | IPFIX su UDP `report.pskreporter.info:4739`; lettura in tempo reale via MQTT su `mqtt.pskreporter.info:1883` (topic `pskr/filter/v2/+/+/NOMINATIVO/#`), unita alla richiesta HTTP a `retrieve.pskreporter.info` ogni 5 minuti | - | **OK** (gia' in Aurora, invio e lettura) | - |
| HamQTH | API per sviluppatori: caricamento del log ADIF e QSO singolo in tempo reale | - | **lavoro** | Invio in rete |
| DXHeat | Nessuna documentazione di sviluppo trovata | - | **?** | Verificare |
| HamAlert | Avvisi via app, Telnet (`set/json` per JSON), URL GET/POST | - | **poco lavoro** come sorgente (Aurora ha gia' un client telnet cluster) | Voce nei server telnet, filtri |
| OpenHamClock | Ascolta 2237 (segnalazione su GitHub); alla ricezione del traffico FT8 mostra gli spot | - | **OK** (non verificato dal vivo) | Multicast se 2237 e' occupata |
| SDR-Control e simili | Nessuna fonte trovata | - | **?** | Verificare |

## 2bis. Collegare i log via UDP: configurazione passo passo (Log4OM, LogHX3, BBLogger)

Stessa sostanza della voce «Collegare i programmi di log (UDP)» della Guida dell'app.

### Lato Aurora (Impostazioni > Programmi collegati)
- **Server UDP primario**: `UDPServer` 127.0.0.1, `UDPServerPort` 2237 (valori di fabbrica). Manda il flusso binario WSJT-X: Heartbeat, Status, Decode, Clear, QSO Logged (5), Close; con «Abilita l'invio del file ADIF per il log» (`EnableUDP1adifSending`, di fabbrica acceso) anche il Logged ADIF (12). Unicast o, se l'indirizzo e' un gruppo, multicast (TTL 1).
- **Server UDP secondario** (riquadro «Invia file ADIF del QSO per il log»): `EnableUDP2adifBroadcast` (di fabbrica acceso), `UDP2Server` 127.0.0.1, `UDP2ServerPort` 2237. Solo il record ADIF in testo, un datagramma per QSO, come `UDP2Server` di JTDX.
- **Attenzione**: di fabbrica primario e secondario puntano entrambi a 127.0.0.1:2237. Lasciare acceso solo quello che serve al log, o spostare l'altro su un'altra porta.
- **Riquadro «Integrazioni»**: destinatari in piu' (al piu' 16). Pulsanti: «Flusso WSJT-X» (127.0.0.1:2237, flusso acceso), «Gruppo multicast» (239.255.0.1:2237, interfaccia 127.0.0.1, TTL 1), «ADIF su UDP» (127.0.0.1:2333, nome Log4OM), «XML stile N1MM» (12060). Per ogni destinatario: flusso WSJT-X, Logged ADIF (12), ADIF su UDP, XML N1MM, multicast.
- **«Gia' lavorato» da Log4OM**: variabile d'ambiente `AURORA_LOG4OM_DB` con il percorso del database SQLite di Log4OM 2; lettura sola, il pannello Log mostra i QSO di Log4OM.

### Log4OM 2
1. Aurora: secondario acceso su 127.0.0.1:2237; primario spostato su un'altra porta (es. 2238 o quella di GridTracker) per lasciare 2237 a Log4OM. In alternativa «+ ADIF su UDP» (2333) con la stessa porta in Log4OM.
2. Log4OM: Settings > Program Configuration > Software Integration > Connections *(nome indicativo, controllare la versione)*.
3. Connessione UDP in entrata, tipo `ADIF_MESSAGE`, porta uguale ad Aurora, attiva; «Save and apply».
4. Oppure tipo `JT_MESSAGE` sulla porta del primario per avere anche lo stato. Una sola delle due strade.
5. La radio si collega a parte: CAT interface > Hamlib, «Connect to active HAMLIB instance» 127.0.0.1:4532 (CAT condiviso di Aurora).

### LogHX3
1. Aurora: «Integrazioni» > «+ Flusso WSJT-X», 127.0.0.1, porta 2237 o un'altra libera (es. 2239), flusso acceso, multicast spento. Se il QSO non arriva ma lo stato si', accendere anche «Logged ADIF (12)».
2. Nel log: impostazione dell'interfaccia WSJT-X/JTDX via UDP, ricezione attiva, 127.0.0.1 e la stessa porta, registrazione automatica dei QSO ricevuti *(nomi delle voci indicativi, non verificati: controllare la versione del programma)*.

### BBLogger
Log italiano gratuito per Windows 10/11, sito ufficiale https://www.aribusto.it/bblogger.htm (ARI Busto Arsizio); sviluppato da Giorgio IK2VIW fino alla 1.0.5.8, poi da Mauro IZ2BKT; supporto di Gigi IK2UVR. Versione 14.10 (settembre 2026); in prova BBLogger 2. Le domande vanno al newsgroup di BBLogger, non agli autori.

L'interfaccia per i modi digitali si chiama **Wsjt_I** (versione 2) e si configura in **«Configurazione interfaccia Wsjtx»**, che si apre con l'icona a ingranaggio nelle finestre «Nuovo QSO» e «Fast QSO» (fonte: PDF «Wsjt_I v. 2 - SETUP» su aribusto.it).
- **Modo UDP**: righe «Msg ID 5», «Msg ID 12» e «Msg ID ADIF», ognuna con IP e porta. Di base 127.0.0.1:2237 per 5 e 12, 127.0.0.1:52001 per ADIF. Con JTAlert le righe 5 e 12 vanno sulla porta di JTAlert, 2334.
- **Scelta del modo**: menu sotto «Auto-Save» in «Fast QSO»: Disable, UDP 5, UDP 12, UDP ADIF, poi i programmi del Modo ADIF (WSJT-X, JTDX, MSHV, FLDIGI e quelli aggiunti). «UDP 5» funziona con il messaggio QSO Logged; «UDP 12» riceve piu' dati ma vuole il Logged ADIF (12) acceso; «UDP ADIF» solo per pacchetti ADIF testuali (MultiPSK).
- **«Auto-Save»**: spuntata, il QSO va a log da solo a fine QSO.
- **Modo ADIF**: tabella «Descrizione / Percorso file ADIF» (lente, + verde): BBLogger legge il file quando il programma salva il QSO. Il PDF «Differenze tra UDP e ADIF» lo indica come il modo piu' sicuro; in UDP il QSO puo' perdersi se il ciclo non arriva fino in fondo.

Con Aurora:
1. **Rete (consigliato)**: Aurora, «Integrazioni» > «+ BBLogger» (127.0.0.1:2237, flusso WSJT-X e Logged ADIF accesi). BBLogger: righe Msg ID 5 e 12 su 127.0.0.1:2237, menu «UDP 12», «Auto-Save».
2. **File**: BBLogger, Modo ADIF con descrizione Aurora e percorso `%APPDATA%\Aurora\aurora_log.adi` (il motore lo scrive sempre), menu «Aurora», «Auto-Save».
3. Un metodo solo alla volta. Di fabbrica anche i server UDP primario e secondario di Aurora sono su 127.0.0.1:2237: con BBLogger possono dare QSO doppi.

Fonti: https://www.aribusto.it/bblogger.htm, PDF https://www.aribusto.it/BBL/Wsjt_I%20v.%202-SETUP_IT.pdf e https://www.aribusto.it/BBL/Differenze%20tra%20UDP%20e%20ADIF.pdf (letti il 2026-11-07). Non provato dal vivo con Aurora.

### Verifica e problemi tipici
- Prova: un QSO (anche su carico fittizio) e la riga deve comparire nel log.
- Una porta unicast la apre un solo programma: con JTDX, GridTracker o JTAlert aperti la 2237 e' spesso occupata. Soluzioni: un destinatario per programma con porte diverse, oppure il gruppo multicast se tutti i programmi lo ascoltano.
- QSO doppi: lo stesso log riceve per due strade (flusso + ADIF, o inoltro di GridTracker/JTAlert). Tenerne una.
- Log su un altro PC: indirizzo IP di quel PC e regola del firewall di Windows per UDP in entrata sulla porta.
- BBLogger ha il suo destinatario pronto («+ BBLogger») e il modo file ADIF; LogHX3 usa «Flusso WSJT-X» (porte e voci non verificate).

## 3. Programma per programma

### GridTracker 2 (e 1)
- **Legge**: flusso WSJT-X binario; IP 127.0.0.1, porta 2237 di fabbrica (Impostazioni > Generale), multicast possibile (da 224.0.0.0 a 239.255.255.255; due computer: 224.0.0.73). Richiede in WSJT-X/JTDX «Accept UDP requests».
- **Invia**: i comandi di ritorno (Reply al doppio clic, Highlight, Halt Tx, Free text) all'indirizzo da cui parte il flusso. Inoltra il QSO a Log4OM (porte 2236/2238 nella configurazione della stazione), N1MM+, Wavelog e altri; il suo inoltro UDP porta il flusso a un'altra applicazione (porta diversa).
- **Con Aurora**: `AURORA_UDP_FORWARD=127.0.0.1:2333` (o la porta scelta in GridTracker); `AURORA_UDP_ACCEPT=1` per accettare i comandi; `AURORA_UDP_ACCEPT_REPLY` come `AcceptUDPReplyMessages` di JTDX.
- **Mancano**: Highlight (13), Location (11), multicast.

### Log4OM 2
- **Legge**: ascoltatori UDP creati dall'utente; per JT ci sono due tipi di messaggio, `JT_MESSAGE` (flusso binario dello stato) e `ADIF_MESSAGE` (QSO testuale ADIF). Log4OM non e' legato a una porta: va bene qualunque porta uguale a quella del mittente. Per i QSO su UDP di JTDX il secondo server usa di norma 127.0.0.1:2237.
- **Con Aurora**: `AURORA_EXPORT_UDP=127.0.0.1:2237` (un solo invio per QSO). Non attivare l'ADIF anche dal flusso primario verso lo stesso Log4OM, ne' dal JTAlert: il QSO arriva due volte.
- **Letture locali**: Aurora legge in sola lettura il database SQLite di Log4OM2 (`AURORA_LOG4OM_DB`) per i nominativi gia' lavorati.
- **Mancano**: niente di obbligatorio. Upload diretto ai servizi: Log4OM li fa da se'.

### JTAlert (HamApps)
- **Legge**: la porta UDP di WSJT-X/JTDX; la porta di ascolto non e' configurabile e segue la configurazione attiva del programma; in versioni recenti riconosce anche multicast. Blocca la porta all'avvio, quindi nessun altro la puo' usare (soluzioni: rilancio dei pacchetti, multicast, seconda porta).
- **Invia**: Reply, colori dei nominativi e Halt Tx a WSJT-X/JTDX, con «Accept UDP requests»; rilancia i pacchetti ricevuti (solo ricevuti) su una porta a scelta, ad esempio 2334, per altri programmi; inoltra il QSO a DXKeeper, ACLog, Log4OM, HRD, N1MM+ (UDP «Last QSO API», porta 2333 nelle guide) e a ADIF.
- **Con Aurora**: **non verificato**. Il rischio e' che JTAlert cerchi il programma di decodifica (processo e file di configurazione). Prova consigliata: far puntare il flusso di Aurora alla porta che JTAlert ascolta; in alternativa far girare Aurora in mezzo con GridTracker come oggi.
- **Mancano**: Highlight (13); comprensione del riconoscimento dell'istanza.

### DXLab Suite (DXKeeper, Commander)
- **Letture**: nelle guide DXKeeper non riceve da WSJT-X direttamente: WSJT-X manda a JTAlert (UDP, «Last QSO API», 2333) e JTAlert scrive in DXKeeper. Commander fa da ponte radio per WSJT-X/JTDX (rig «DX Lab Suite Commander», TCP).
- **Con Aurora**: via JTAlert (vedi sopra); Aurora ha il proprio CAT e non ha bisogno di Commander.
- **Mancano**: una conferma che DXKeeper legga direttamente l'ADIF su UDP **(non verificato)**.

### N1MM+ Logger+
- **Legge**: «Decode List» a due vie su UDP, default 127.0.0.1:2237 (SO2R, radio 2: 2239); per JTDX anche una connessione TCP di registrazione (52001; radio 2: 52006) e il percorso di `jtdx.exe`. Multicast opzionale (es. 239.0.0.1) per condividere il flusso con JTAlert o GridTracker.
- **Trasmette** (altra direzione): messaggi XML su UDP, porta consigliata 12060: `contactinfo`, `contactreplace`, `contactdelete`, `lookupinfo`, `RadioInfo` (circa ogni 10 s), `spot`, `AppInfo`.
- **Invia a WSJT-X/JTDX**: dupe e moltiplicatori come colore nel pannello (grigio dupe, blu lavorabile, rosso nuovo moltiplicatore), nominativo e griglia al doppio clic, Abilita Tx.
- **Con Aurora**: flusso su 2237 e `AURORA_UDP_ACCEPT=1`; la connessione TCP si ricava dal server TCP della scheda Segnalazioni (stesso formato di JTDX, `Log <parameters:N> ...`). Provare dal vivo come N1MM+ avvia il programma e se accetta un'istanza gia' in funzione.
- **Mancano**: Highlight (13) per i colori; Location/Configure se N1MM li usa **(non verificato)**; il controllo radio passa dal CAT proprio di Aurora, non da Commander.
- **Diffusione ADIF su UDP verso N1MM+**: nessuna documentazione trovata; N1MM+ usa un proprio XML.

### HRD Logbook / Ham Radio Deluxe
- **Legge**: «QSO Forwarding», interfaccia UDP in entrata e uscita del Logbook: sezione UDP Receive; la guida ufficiale cita la porta 2333 per WSJT-X/JTDX e 12060 per N1MM; nelle impostazioni di WSJT-X si spunta «N1MM Logger + Broadcasts» a 127.0.0.1:2333. Per gli allarmi usa multicast con lo stesso indirizzo di WSJT-X.
- **Con Aurora**: provare `AURORA_EXPORT_UDP=127.0.0.1:2333` (ADIF testuale come il secondo server di JTDX); se non basta, verificare se il formato atteso e' l'XML `contactinfo` di N1MM **(non verificato)**.
- **Attenzione**: abilitare HRD e JTAlert insieme crea doppioni.
- **Mancano**: eventuale emissione XML stile N1MM.

### CQRLOG
- **Legge**: modalita' remota per WSJT-X (da v1.9.1), attivabile da File > «Remote mode for wsjt»: legge le righe decodificate (monitor CQ, mappa di banda, avvisi) e il QSO quando WSJT-X lo registra.
- **Invia**: colori del monitor CQ alla finestra «Band Activity» (richiede una versione di WSJT-X recente), messaggi standard, messaggio libero (pulsante Name73), richiesta di avvio QSO su doppio clic (sposta la frequenza e trasmette), stop Tx.
- **Con Aurora**: flusso binario verso la porta scelta in CQRLOG; per i comandi `AURORA_UDP_ACCEPT=1`. La documentazione citata non fornisce la porta: allinearla nelle preferenze.
- **Mancano**: Highlight (13) per il lato «colori»; il testo cita solo WSJT-X, JTDX e' un'ipotesi **(non verificato)**.

### QLog
- **Legge**: il flusso WSJT-X su 2237 (conferma di un utente nel forum del progetto); la porta nella scheda Network e' per rilanciare ad altri programmi.
- **Invia**: notifiche UDP JSON per QSO, spot DX, avvisi, CQ di WSJT-X (`wsjtxcqspot`) e stato della radio.
- **Con Aurora**: flusso su 2237 (ma 2237 e' spesso occupata da altri: ricorrere al multicast).
- **Mancano**: multicast.

### MacLoggerDX
- **Legge**: WSJT-X, JTDX e JS8Call su UDP 127.0.0.1:2237; QSO Logged (5) o Logged ADIF (12) a scelta (casella «WSJT-X Log ADIF»); CQ e decode diventano spot; ricerca del DX Call. Porta modificabile da Terminale. Per un Mac diverso: multicast (224.0.0.1) o indirizzo di broadcast di rete.
- **Invia**: nessun comando UDP; il doppio clic su uno spot risponde dal programma WSJT-X.
- **Con Aurora**: `AURORA_UDP_FORWARD=<indirizzo>:2237`. Per il Logged ADIF `AURORA_UDP_ADIF=1`. Pericolo di doppioni se si registra anche da GridTracker.
- **Mancano**: multicast/broadcast per un Mac separato.

### Logger32
- Mostra i decode in una «UDP BandMap» e registra in automatico i QSO di WSJT-X/JTDX con l'opzione «Allow automatic QSO logging» (guida 3.50 del 2018). La porta non e' documentata. **(non verificato)**

### N3FJP
- Nessuna ricezione UDP di WSJT-X documentata. Esistono adattatori di terzi (ad esempio `wsjtx_to_n3fjp`): ascoltano UDP (default 127.0.0.1:2333) e parlano il TCP di N3FJP (127.0.0.1:1100).
- **Con Aurora**: mandare flusso/ADIF alla porta dell'adattatore.

### RumLogNG, UCXLog
- Nessuna fonte utile trovata: da verificare con la documentazione dei programmi. Probabile che funzionino se capaci di leggere il protocollo WSJT-X su UDP **(non verificato)**.

### Cloudlog e Wavelog (WaveLogGate)
- **WaveLogGate** (programma di terzi): UDP in ingresso su 2333 (configurabile) per i QSO dal secondo server ADIF **e** dai messaggi binari del primo; QSO identici entro pochi secondi si registrano una volta; invia a Wavelog con URL che termina in `index.php` e chiave API; CAT con FLRig o Hamlib; richieste QSY da Wavelog su HTTP 54321; stato radio su WebSocket 54322; UDP in uscita (2334) dei QSO registrati su Wavelog.
- **Con Aurora**: `AURORA_EXPORT_UDP=127.0.0.1:2333` (ADIF testuale); in alternativa `AURORA_UDP_FORWARD=127.0.0.1:2333` con `AURORA_UDP_ADIF=1`. Non abilitare entrambe: doppioni.
- Altre vie documentate da Wavelog/Cloudlog: importazione manuale del file `WSJTX_LOG.ADI`, ADIFPUSH (sorveglia il file), GridTracker con la propria chiave API.
- **Invio diretto**: fatto (sezione 6), senza programmi in mezzo. Non attivare insieme WaveLogGate verso lo stesso log: doppioni.

### ClubLog

Dal 2026-11-13 si usa l'**API REST v1** (`https://clublog.org/api/v1`): il radioamatore crea un token in Club Log › Settings › API e lo incolla (voce `ClublogToken`, segreto `clublog-token`). Invio `POST /qso` con `Authorization: Bearer <token>` e JSON `{"call", "adif"}`; risposte 201 `created`, 200 `duplicate`/`updated`/`dropped`, errori `{"error":{"code","message"}}` (401 token non valido: invio fermo; 403/404 nominativo; 422 rifiutato; 429/5xx si riprova). «Prova» = `GET /me/usage?days=1`, non scrive niente. Limiti: 60 richieste/minuto, 10.000 invii di QSO al giorno. Se il token manca si usa il vecchio `realtime.php` descritto qui sotto.
- Documentazione per sviluppatori: per i QSO singoli «real-time», una richiesta HTTP con **un solo record ADIF** terminato da `<eor>` per chiamata; per i lotti un altro punto di accesso (putlogs). Esito 200 = registrato (anche doppione), 400 = rifiutato. Il programma non deve ripetere richieste con credenziali errate (blocco dell'indirizzo). Serve una chiave applicazione.
- **Invio diretto**: fatto (sezione 6).

### QRZ.com Logbook
- `https://logbook.qrz.com/api`, richiesta POST a coppie nome=valore: `KEY` (chiave del logbook), `ACTION=INSERT`, `ADIF` (nominativo proprio e collegato, data e ora, banda, modo); abbonamento richiesto; User-Agent identificabile fino a 128 caratteri; `OPTION=REPLACE` per sovrascrivere i doppioni. Risposta `RESULT=OK&LOGID=...&COUNT=1`.
- **Invio diretto**: fatto (sezione 6).

### LoTW (Logbook of The World)
- Il QSO deve essere firmato con il certificato della stazione: lo fa TQSL (`tqsl` da riga di comando: `-a` azione su doppioni/fuori data, `-u` carica, `-x` senza finestre, `-l` stazione). Aurora non conosce LoTW (gli indicatori di conferma sono solo letti dal log).
- **Invio**: fatto (sezione 6), con gestione dei codici di uscita.

### eQSL, PSK Reporter
- Gia' in Aurora: eQSL (`ADIFdata`, HTTPS), PSK Reporter in invio (IPFIX, non piu' di un pacchetto ogni cinque minuti salvo pacchetto pieno, descrittori nei primi tre pacchetti e poi ogni ora) e in lettura (`retrieve.pskreporter.info/query`, ultimi 100 record, al massimo 6 ore, non piu' spesso di ogni cinque minuti).

### HamQTH, DXHeat, HamAlert, OpenHamClock, SDR-Control
- **HamQTH**: caricamento del log ADIF e QSO singolo in tempo reale (pagina sviluppatori). Un invio simile a quello di eQSL.
- **DXHeat**: nessuna API documentata trovata.
- **HamAlert**: avvisi da cluster, RBN, SOTA, POTA, PSK Reporter; recapito via app, Telnet (con `set/json`), URL GET/POST. Come sorgente per Aurora: un client telnet (gia' presente nel crate `dx-cluster`) o un URL.
- **OpenHamClock**: ascolta il flusso WSJT-X su 2237 (una segnalazione di un utente racconta un malfunzionamento di visualizzazione con pacchetti ricevuti).
- **SDR-Control** e simili: nessuna fonte trovata.

## 4. Cosa manca ad Aurora, con priorita' consigliata

| Pri. | Cosa | Perche' | Sforzo | Chi ne beneficia |
|---|---|---|---|---|
| 1 | **Multicast** (fatto: TTL e interfaccia per destinatario, comandi di ritorno sul socket del gruppo) | Un solo programma per volta puo' ascoltare 2237: con il multicast GridTracker, JTAlert, QLog, MacLoggerDX, CQRLOG, N1MM+, HRD condividono il flusso senza altro | Basso: solo rete, nessun messaggio nuovo | Quasi tutti |
| 2 | **Ascolto di Highlight Callsign (13)** (fatto: righe colorate nella lista dei messaggi) | CQRLOG, N1MM+ (dupe/moltiplicatori) e GridTracker colorano le righe cosi' | Medio: decodifica del `QColor`, mappa nominativo -> colore, UI | CQRLOG, N1MM+, GridTracker |
| 3 | ~~**Invio diretto** a Wavelog/Cloudlog, ClubLog e QRZ Logbook~~ **fatto** (sezione 6) | - | - | - |
| 4 | **Impostazioni per destinatario** (fatto: flusso binario, Logged ADIF, ADIF su UDP, XML N1MM, multicast; restano da fare schema e identificativo per destinatario) | Evita doppioni (Log4OM, HRD, JTAlert) e permette destinazioni diverse | Basso-medio | Tutti i log locali |
| 5 | Ascolto di **Location (11)**, **Configure (15)**, **Switch Configuration (14)**, **Clear (3) in ingresso** (fatto) | Locatore da GPS/programmi, impostazione di modo e frequenza da fuori, cambio di configurazione | Medio | N1MM+, JTAlert, programmi di controllo |
| 6 | Broadcast **N1MM XML `contactinfo`** (porta 12060) (fatto, da provare con HRD) | HRD, programmi che imitano N1MM; molti traduttori lo accettano | Medio: XML con ID del QSO | HRD, gestionali di gara |
| 7 | ~~**LoTW** via TQSL~~ **fatto** (sezione 6) | - | - | - |
| 8 | Campi **Exchange sent/received** nel QSO Logged (fatto); flag «solo Abilita Tx» del Halt Tx (da fare) | Completezza del protocollo di WSJT-X 2.x | Basso | Programmi di gara |
| 9 | **HamQTH**, **HamAlert** (telnet come sorgente) | Utili ma secondari | Basso-medio | Chi li usa |

**Prossimo passo consigliato**: le prove dal vivo delle sezioni 0 (UDP) e 6 (invii diretti).

## 5. Impostazioni consigliate, riepilogo

| Obiettivo | Impostazione in Aurora |
|---|---|
| Mappa e avvisi di GridTracker 2 | `AURORA_UDP_FORWARD=127.0.0.1:2333` (o la porta di GridTracker), `AURORA_UDP_ACCEPT=1` |
| Log4OM 2 (QSO) | `AURORA_EXPORT_UDP=127.0.0.1:2237` |
| WaveLogGate | `AURORA_EXPORT_UDP=127.0.0.1:2333` |
| MacLoggerDX / QLog / CQRLOG | `AURORA_UDP_FORWARD=<indirizzo>:2237` o un destinatario del riquadro «Integrazioni» (una sola applicazione per porta; con piu' applicazioni un destinatario multicast, per esempio 239.255.0.1:2237 con interfaccia 127.0.0.1) |
| Piu' programmi sulla stessa porta | Riquadro «Integrazioni»: destinatario multicast 239.255.0.1:2237, TTL 1, interfaccia 127.0.0.1 (stesso PC) o l'indirizzo di rete (altri computer); nei programmi lo stesso gruppo |
| HRD Logbook (XML stile N1MM) | Riquadro «Integrazioni»: destinatario 127.0.0.1:12060 con «XML N1MM» |
| Logged ADIF sul flusso primario | `AURORA_UDP_ADIF=1` |
| Un secondo file ADIF | `AURORA_EXPORT_FILE=<percorso>` |

Regola per i doppioni: **un solo percorso di registrazione per ciascun log** (flusso primario con ADIF oppure ADIF secondario, mai i due insieme verso lo stesso programma; non attivare anche JTAlert).

## 6. Invio diretto dei QSO (fatto il 2026-11-01)

Col motore proprio ogni QSO messo a log (evento `QsoLogged`, dopo la guardia anti-doppione del servizio `reporting`) entra in una **coda**, con una voce per ogni servizio acceso. Codice: `core/crates/reporting/src/upload.rs` (coda, attivita', esiti), `wavelog.rs`, `clublog.rs`, `qrz.rs`, `lotw.rs`; impostazioni in `core/crates/remote-api/src/engine_settings.rs`; scheda Impostazioni > Segnalazioni in `app/src/settings/NativeSettings.tsx` e `app/src/lib/engineSettingsLayout.ts`. Con JTDX come motore gli invii li fa JTDX.

### Comportamento comune
- **Spento di fabbrica**; si accende solo con tutti i dati (altrimenti `400 bad_setting`: nessun invio finto).
- **Non blocca il motore**: ogni servizio ha un'attivita' sua; le richieste HTTP (attesa massima 20 s) e TQSL (al piu' 3 minuti) girano in thread a parte. Un servizio lento non ferma gli altri.
- **Coda su file** `invii-in-coda.json` nella cartella dei dati (scrittura atomica): contiene solo nominativo e record ADIF, **mai credenziali**. Resta dopo un riavvio.
- **Nuovi tentativi** se la rete manca, il tempo scade o il servizio risponde 408, 429 o 5xx: dopo 1, 2, 5, 10, 30 minuti, poi ogni ora. Dopo **sette giorni** in coda il QSO si scarta (errore nello stato e nel registro). Al piu' 2000 QSO in coda per servizio. Un servizio spento tiene in coda quello che aveva (riparte quando lo si riaccende).
- **Errore di credenziali** (chiave rifiutata, 401/403, accesso negato): quel servizio **si ferma** e i QSO restano in coda finche' non cambiano le sue impostazioni o una chiave/password, o finche' una «Prova» riesce; poi riparte subito. Serve soprattutto per ClubLog, che blocca l'indirizzo di chi ripete richieste con credenziali sbagliate.
- **QSO rifiutato** (dati non validi): esce dalla coda, con il motivo nello stato. **Doppione**: vale come riuscito.
- **Stato** (`status.<servizio>` di `GET /api/v1/engine-settings`): esito, ora, messaggio (in italiano, senza credenziali) e QSO in coda (`pending`); la scheda lo mostra sotto ogni servizio.
- **Record ADIF**: lo stesso del secondo server UDP di JTDX (`logbook::adif::jtdx_record`: banda, `STATION_CALLSIGN`, `MY_GRIDSQUARE`, nominativo, frequenza, modo con `SUBMODE` per FT4, date e ore di inizio e fine, rapporti, potenza, locatore, nome e commento).
- **Credenziali**: chiavi e password nella protezione dei segreti di Windows (DPAPI, come la password di eQSL), lette solo al momento dell'invio; il campo resta vuoto con la scritta «salvata». Sui sistemi senza DPAPI le voci con chiavi non sono disponibili (LoTW si', perche' le credenziali stanno in TQSL).
- **Prova** (`POST /api/v1/qso-upload/test`): usa le impostazioni gia' salvate; il pulsante si abilita quando le modifiche del riquadro sono state salvate (autosalvataggio).
- **User-Agent** di tutte le richieste: `Aurora/<versione>`.

### Wavelog e Cloudlog
- **Richiesta**: `POST {indirizzo}/index.php/api/qso`, `Content-Type: application/json`, corpo `{"key": "...", "station_profile_id": "3", "type": "adif", "string": "<record ADIF>"}`. L'indirizzo si accetta con o senza `/index.php` finale.
- **Fonti**: documentazione dell'API di Wavelog (https://docs.wavelog.org/developer/api/) e di Cloudlog (https://github.com/magicbug/Cloudlog/wiki/API); le risposte sono lette dal sorgente di `application/controllers/Api.php` dei due progetti (rami principali al 2026-11-01), perche' la documentazione non le descrive.
- **Risposte**: 201 `{"status":"created"}` = registrato; Wavelog 400 `{"status":"abort","messages":[...]}` con «Duplicate» = doppione, altrimenti rifiutato; Cloudlog 201 con `imported_count: 0` e un messaggio con «Duplicate» = doppione, con un altro messaggio = rifiutato; 401 chiave o stazione sbagliate, 403 chiave senza scrittura (Wavelog), 404 indirizzo sbagliato: servizio fermo; 429 e 5xx: nuovo tentativo.
- **Prova**: `GET {indirizzo}/index.php/api/station_info/{chiave}` (sola lettura) e ricerca della stazione tra quelle della chiave. **Non** verifica il permesso di scrittura: una chiave di sola lettura si scopre al primo QSO (403, servizio fermo).
- **Chiave**: menu utente, voce «API», chiave di lettura e scrittura. **Numero della stazione**: si legge nell'indirizzo quando si modifica la posizione di stazione.
- **Note**: la documentazione di Wavelog dice che i QSO arrivati dall'API non vengono rimandati in diretta a QRZ ne' cercati nei callbook. **Non verificato dal vivo**: il messaggio esatto di Cloudlog per un doppione (letto solo dal sorgente).

### ClubLog
- **Richiesta**: `POST https://clublog.org/realtime.php`, modulo `application/x-www-form-urlencoded` con `email`, `password`, `callsign`, `adif` (un solo record) e `api`.
- **Fonti**: articolo dell'assistenza «How To Upload QSOs In Real-Time» (https://clublog.freshdesk.com/support/solutions/articles/54906) e modulo d'esempio https://clublog.org/test_realtime.html (nomi dei campi verificati sul modulo).
- **Risposte** (dall'articolo): 200 «QSO OK», «QSO Duplicate» (da trattare come riuscito), «QSO Modified»; 400 rifiutato; 403 accesso negato = **fermarsi e mostrare l'errore** (Aurora ferma il servizio). 401 e 5xx non sono descritti: Aurora tratta 401 come 403 e 5xx come «riprova» **(non verificato)**.
- **Password**: consigliata una «Application Password» creata nelle impostazioni dell'account ClubLog, al posto della password dell'account.
- **Chiave API dell'applicazione**: ClubLog la rilascia agli autori dei programmi su richiesta all'assistenza (l'articolo sui lotti `putlogs.php` dice di chiederla per email all'helpdesk; che per l'invio in tempo reale valga la stessa procedura e' **da confermare con ClubLog**). Aurora **non ne contiene una**: va scritta in Impostazioni > Segnalazioni ed e' protetta come una password. Chi pubblica Aurora puo' chiederne una per il programma; finche' non c'e', ogni utente usa la propria.
- **Prova**: ClubLog non offre una verifica delle credenziali senza inviare un QSO: il pulsante controlla solo che i dati ci siano; l'esito vero arriva col primo QSO.

### QRZ.com Logbook
- **Richiesta**: `POST https://logbook.qrz.com/api`, modulo con `KEY`, `ACTION=INSERT`, `ADIF`; User-Agent riconoscibile (QRZ lo pretende, al piu' 128 caratteri). Aurora **non** usa `OPTION=REPLACE` (sovrascriverebbe anche QSO confermati).
- **Fonte**: https://www.qrz.com/docs/logbook/QRZLogbookAPI.html.
- **Risposte**: `RESULT=OK` (o `REPLACE`) con `LOGID` = registrato (l'esempio ufficiale usa `LOGIDS`: Aurora legge entrambi); `RESULT=FAIL` con «duplicate» nel `REASON` = doppione; `FAIL` che parla di chiave, autorizzazione o abbonamento, oppure `RESULT=AUTH` = servizio fermo; altri `FAIL` = rifiutato. Il testo esatto dei motivi non e' documentato: le parole cercate vengono da esempi pubblici **(da verificare dal vivo)**; anche `RESULT=AUTH` non e' nella documentazione.
- **Prova**: `ACTION=STATUS` con la chiave (non scrive niente); il contenuto di `DATA` non e' garantito dalla documentazione.
- **Abbonamento**: l'inserimento via API richiede un abbonamento a QRZ.

- **Nominativo registrato** (`QRZLogCall`, facoltativo): se compilato, nel record mandato a QRZ `STATION_CALLSIGN` diventa quel nominativo (aggiunto se manca), cosi' i QSO fatti come `IU3ABC/P` o `EA3/IU3ABC` entrano nel logbook di `IU3ABC` invece di essere scartati.

### HRDLOG.NET
- **Richiesta** (come la usano altri programmi di log): `POST https://robot.hrdlog.net/NewEntry.aspx`, modulo con `Callsign` (nominativo del log; vuoto nelle impostazioni = il proprio), `Code` (il codice di caricamento, «upload code», del profilo), `App` (il programma) e `ADIFData` (un record).
- **Risposte lette**: `<insert>N</insert>` con N >= 1 = registrato; `<insert>0</insert>` = gia' presente; `<error>` con «duplicat»/«already» = doppione, con «code», «token», «callsign», «user», «auth» = invio fermo per credenziali, altro = scartato; 408/429/5xx o risposta illeggibile = nuovo tentativo.
- **Non verificabile**: le pagine di aiuto di HRDLOG.NET (`Help/Default.aspx` e `robot.hrdlog.net`) davano errore o «manutenzione» il 2026-11-02, quindi indirizzo, nomi dei campi e testi delle risposte **non sono confermati dalla documentazione ufficiale**. Va provato dal vivo con un QSO vero.
- **Prova**: HRDLOG.NET non ha una verifica documentata senza invio: «Prova» controlla solo nominativo e codice presenti; il codice si verifica al primo QSO.

### LoTW (TQSL)
- **Comando**: `tqsl -d -u -a compliant -x -l <posizione> <file.adi>` (fonte: https://lotw.arrl.org/lotw-help/cmdline). I QSO in attesa (fino a 50) vanno in un solo file ADIF temporaneo accanto alla coda, cancellato a fine invio; i messaggi di TQSL vanno in un file temporaneo da cui si prende l'ultima riga per lo stato. Su Windows TQSL parte senza finestra di console.
- **TQSL**: percorso configurabile; vuoto = cercato in `%ProgramFiles(x86)%\TrustedQSL\tqsl.exe`, `%ProgramFiles%\TrustedQSL\tqsl.exe`, `%LOCALAPPDATA%\Programs\TrustedQSL\tqsl.exe` (altrove `/usr/bin/tqsl`, `/usr/local/bin/tqsl`, il pacchetto di macOS). Un percorso scritto e sbagliato non viene sostituito dalla ricerca.
- **Codici d'uscita** (fonte ARRL) e cosa fa Aurora: 0 inviato; 9 inviato con alcuni QSO saltati; 8 nessun QSO (doppioni o fuori date) = vale come gia' presente; 11 LoTW non raggiungibile, 3 risposta inattesa, 7 file d'uscita: nuovo tentativo; 2 rifiutato da LoTW, 6 file d'ingresso: rifiutato; 1 annullato, 4 e 5 errori di TQSL/TQSLlib, 10 riga di comando: servizio fermo; tempo scaduto (3 minuti): TQSL fermato e nuovo tentativo.
- **Prova**: TQSL trovato e posizione presente nel file `station_data` di TQSL (`%APPDATA%\TrustedQSL\station_data`, elementi `<StationData name="...">`: formato osservato su un'installazione reale, non documentato); non carica niente.
- **Non verificato dal vivo**: che TQSL con `-x` scriva i messaggi su stderr quando gira senza console; che con un certificato protetto da password TQSL esca con un errore (1 o 4) invece di aspettare: per sicurezza c'e' il tempo massimo. Per l'invio automatico il certificato **non deve avere password**.

### Cosa provare dal vivo
1. Wavelog (o Cloudlog): chiave di lettura e scrittura, numero della stazione, «Prova», poi un QSO; controllare che compaia nel log e che lo stato dica «riuscito».
2. Rete staccata durante un QSO: lo stato dice «nuovo tentativo alle ...» e «1 QSO in coda»; riattaccata, il QSO parte al tentativo successivo (o subito con «Prova»).
3. ClubLog con una Application Password e la chiave dell'applicazione: un QSO, poi lo stesso QSO di nuovo (atteso «QSO Duplicate» = riuscito).
4. QRZ: «Prova» (stato del logbook) e un QSO; lo stesso QSO inviato di nuovo deve risultare doppione.
5. LoTW: «Prova» con la posizione di TQSL, poi un QSO; controllare il messaggio nello stato e il QSO su LoTW.
6. Credenziali sbagliate (una per servizio): lo stato dice «invio fermo», i QSO restano in coda; corrette le credenziali, ripartono.
7. HRDLOG.NET: nominativo e codice, un QSO; controllare che compaia nel log e il testo della risposta nello stato (da confrontare con quanto scritto sopra).
8. QRZ con «Nominativo registrato» compilato e un QSO fatto da /P: deve entrare nel logbook del nominativo base.

## 7. Ricerca dei nominativi (QRZ.com XML e HamQTH)
- **Dove**: Impostazioni > Servizi online, riquadri «Ricerca QRZ.com» e «Ricerca HamQTH»; serve al riquadro che compare passando il mouse sul nominativo DE nel DX Cluster (rotta `GET /api/v1/callsign/{call}`).
- **QRZ.com XML** (fonte: https://www.qrz.com/docs/xml/current_spec.html, versione 1.34): accesso `https://xmldata.qrz.com/xml/current/?username=..&password=..&agent=..` -> `<Session><Key>`; ricerca `?s=KEY&callsign=..` -> `<Callsign>`. Senza `<Key>` nella risposta («Session Timeout», «Invalid session key») si rifa' l'accesso una volta. «Not found: ...» = non trovato. Con credenziali sbagliate `<Error>` senza `<Key>`.
- **Abbonamento**: senza l'abbonamento «XML Logbook Data» QRZ risponde `SubExp` = `non-subscriber` e `<Message>A subscription is required to obtain the complete data</Message>`; la documentazione dice solo che i campi sono ridotti, non quali: Aurora prende quello che arriva e lo dice nel riquadro («dati ridotti»).
- **HamQTH** (fonte: https://www.hamqth.com/developers.php, API v1): `https://www.hamqth.com/xml.php?u=..&p=..` -> `<session><session_id>` (vale un'ora); `?id=..&callsign=..&prg=..` -> `<search>`. Errori «Wrong user name or password», «Session does not exist or expired» (sessione rinnovata una volta), «Callsign not found». Si usa se QRZ e' spento o non trova il nominativo.
- **Dati tenuti**: solo nome, cognome, citta' e paese (QRZ `fname`, `name`, `addr2`, `country`; HamQTH `nick`/`adr_name`, `adr_city`/`qth`, `country`). Email, indirizzo, telefono e il resto non si leggono. Risultati (anche «non trovato») in memoria per tre giorni, al massimo 5000 nominativi; si dimenticano cambiando utente o password.
- **Spente di base**; spente, nessuna richiesta. Password nella protezione dei segreti (`qrz-xml`, `hamqth`), mai nel file.
- **Prova**: un accesso nuovo con utente e password salvati; per QRZ dice se c'e' l'abbonamento XML e fino a quando.


## Fonti

- Schema dei messaggi di WSJT-X, `NetworkMessage.hpp` (copia del sorgente pubblico): https://raw.githubusercontent.com/roelandjansen/wsjt-x/master/NetworkMessage.hpp
- Manuale WSJT-X: https://wsjt.sourceforge.io/wsjtx-doc/wsjtx-main-2.7.0.html
- GridTracker, appendice B (impostazioni UDP): https://docs.gridtracker.org/latest/Appendices/Appendix-B-Configuring-WSJT-X-and-JTDX-for-GridTracker.html
- N1MM+, Decode List (collegamento a due vie, porte): https://n1mmwp.hamdocs.com/manual-windows/wsjt-x-decode-list-window/
- N1MM+, messaggi UDP esterni: https://n1mmwp.hamdocs.com/appendices/external-udp-broadcasts/
- Guida N3PKJ, WSJT-X + JTAlert + N1MM+: http://qsl.net/n/n3pkj/wsjt-x/
- Guida N3PKJ, WSJT-X + JTAlert + DXKeeper: http://qsl.net/n/n3pkj//dxkeeper/
- HamApps (JTAlert): https://hamapps.com/
- HRD, QSO Forwarding: https://support.hamradiodeluxe.com/support/solutions/articles/51000052684
- HRD, multicast: https://support.hamradiodeluxe.com/support/solutions/articles/51000298966
- CQRLOG, aiuto «WSJT» (copia Debian 2.5.2): https://sources.debian.org/src/cqrlog/2.5.2-6/help/wsjt.html e https://raw.githubusercontent.com/ok2cqr/cqrlog/master/help/wsjt.html
- QLog, notifiche: https://github.com/foldynl/QLog/wiki/Notifications
- QLog, discussione sulla porta 2237: https://github.com/foldynl/QLog/discussions/1062
- MacLoggerDX e WSJT-X: https://www.dogparksoftware.com/MacLoggerDX%20Help/mldxfc_wsjtx.html
- Logger32, guida FT8 (PDF): https://www.lrg.lv/_faili/logger32/FT8_L32.pdf
- WaveLogGate: https://github.com/wavelog/WaveLogGate
- Wavelog, integrazione WSJT-X: https://docs.wavelog.org/user-guide/integrations/wsjt-x/
- Cloudlog, integrazione WSJT-X: https://github.com/magicbug/Cloudlog/wiki/WSJT-X-Integration
- Ponte WSJT-X a Cloudlog (porta 2333): https://github.com/int2001/wsjtx2cloudlog
- Adattatore WSJT-X a N3FJP: https://github.com/dslotter/wsjtx_to_n3fjp
- Traduttore UDP verso XML N1MM: https://github.com/akgordon/N7AKG-UDP-Translator
- OpsLog, connessioni (porte e formati): https://github.com/GregTroar/OpsLog/wiki/Connections
- Forum Log4OM (tipi `JT_MESSAGE` e `ADIF_MESSAGE`, porte): https://forum.log4om.com/viewtopic.php?p=23832 e https://forum.log4om.com/viewtopic.php?p=23921
- GridTracker 2 e inoltro verso piu' programmi (esperienza d'uso): https://community.worldradioleague.com/t/wsjt-x-gridtracker2-udp-messages-to-multiple-applications-solved/44906
- Club Log, invio real-time: https://clublog.freshdesk.com/support/solutions/articles/54906-how-to-upload-qsos-in-real-time
- Club Log, modulo d'esempio dell'invio real-time: https://clublog.org/test_realtime.html
- Club Log, invio a lotti e chiave API: https://clublog.freshdesk.com/support/solutions/articles/54905
- Wavelog, API: https://docs.wavelog.org/developer/api/ ; sorgente `application/controllers/Api.php`: https://github.com/wavelog/wavelog
- Cloudlog, API: https://github.com/magicbug/Cloudlog/wiki/API ; sorgente `application/controllers/Api.php`: https://github.com/magicbug/Cloudlog
- QRZ Logbook API: https://www.qrz.com/docs/logbook/QRZLogbookAPI.html
- LoTW, riga di comando di TQSL: https://lotw.arrl.org/lotw-help/cmdline
- PSK Reporter, guida per sviluppatori: https://pskreporter.info/pskdev.html
- HamQTH, per sviluppatori: https://www.hamqth.com/developers.php
- QRZ XML Logbook Data Specification 1.34: https://www.qrz.com/docs/xml/current_spec.html
- HRDLOG.NET (pagine di aiuto non raggiungibili il 2026-11-02): http://www.hrdlog.net/
- HamAlert: https://hamalert.org/about e https://forum.hamalert.org/t/how-to-make-this-work-on-a-computer/191
- OpenHamClock, segnalazione sulla porta 2237: https://github.com/accius/openhamclock/issues/577
- Codice di Aurora letto per questo documento: `core/crates/udp-bridge`, `core/crates/udp-emitter`, `core/crates/qso-export`, `core/crates/reporting`, `docs/udp-flussi.md`, `.env.example`.
