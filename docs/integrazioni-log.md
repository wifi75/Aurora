# Integrazione di Aurora con altri log e utility (UDP, ADIF, TCP, servizi in rete)

Ricerca del 2026-11-01: questo documento dice **cosa Aurora fa oggi**, **con chi puo' parlare** e **cosa manca**. Aggiornato lo stesso giorno con l'**invio diretto dei QSO** a Wavelog/Cloudlog, ClubLog, QRZ.com Logbook e LoTW (sezione 6).
Le righe marcate **(non verificato)** non hanno una fonte ufficiale o sono state ricavate da forum e guide di terzi: vanno provate dal vivo.
Nei programmi e nei nominativi di esempio si usano valori di fantasia (K1ABC, locatore FN42).

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
| 3 | Clear | Out e In | **si**, alla partenza (come il costruttore di JTDX) | **no** | In WSJT-X il Clear in ingresso porta la «finestra» (quale pannello svuotare) |
| 4 | Reply | In | - | **si**, solo con `AURORA_UDP_ACCEPT=1` e per i messaggi ammessi da `AURORA_UDP_ACCEPT_REPLY` (1 solo CQ, 2 CQ e 73, 3 tutti) | Ritrova il decode tra gli ultimi 300 per testo, frequenza e ora; i modificatori non sono usati |
| 5 | QSO Logged | Out | **si**, alla registrazione | - | Ordine dei campi di JTDX (fino al «nominativo e locatore propri»); mancano «Exchange sent/received» che WSJT-X 2.x aggiunge |
| 6 | Close | Out (in WSJT-X il client lo gestisce anche in ingresso: **da verificare**) | **si**, alla chiusura | **no** | |
| 7 | Replay | In | - | **si**, sempre (anche con `AURORA_UDP_ACCEPT=0`) | Rimanda i decode correnti (non nuovi) e lo Status |
| 8 | Halt Tx | In | - | **si**, con `AURORA_UDP_ACCEPT=1` | In Aurora fa sempre lo STOP; il flag «solo Abilita Tx» e' letto ma non distinto |
| 9 | Free Text | In | - | **si**, con `AURORA_UDP_ACCEPT=1` | Con `send` trasmette |
| 10 | WSPR Decode | Out | no (Aurora non decodifica WSPR) | - | Non serve |
| 11 | Location | In | - | **no** | Servirebbe per far impostare il locatore da un GPS o da un programma esterno |
| 12 | Logged ADIF | Out | **si**, dopo il QSO Logged, **solo** con `AURORA_UDP_ADIF=1` (spento di fabbrica) | - | Intestazione ADIF di JTDX (`programid JTDX`) davanti al record |
| 13 | Highlight Callsign | In | - | **no** | Lo mandano CQRLOG, GridTracker, JTAlert (non verificato per tutti) per colorare le righe; Aurora ha una colorazione propria ma non legge questa |
| 14 | Switch Configuration | In | - | **no** | Cambio di configurazione per nome |
| 15 | Configure | In | - | **no** | Modo, frequenza Tx, DX call, ecc. da programma esterno |
| 50 | (JTDX) Set Tx Freq | In | - | **si**, con `AURORA_UDP_ACCEPT=1`, solo 1..5000 Hz | Tipo non standard di JTDX |
| 51 | (JTDX) Trigger CQ | In | - | **si**, con `AURORA_UDP_ACCEPT=1`, solo se `send` e' vero | Tipo non standard di JTDX |

Correzione rispetto a una convinzione comune nel progetto: il **Logged ADIF (12) c'e' gia'**, e' solo spento di fabbrica. Manca invece l'**ascolto** di 11, 13, 14, 15 e del Clear in ingresso.

Particolarita' dell'emettitore che contano per le integrazioni:
- un solo socket UDP con porta effimera per tutti i destinatari (i comandi tornano a quella porta, come con JTDX): i comandi di piu' programmi collegati arrivano tutti allo stesso ascoltatore;
- l'identificativo e' sempre `JTDX`: i comandi con un altro identificativo si scartano;
- non c'e' iscrizione a gruppi multicast ne' impostazione di TTL o interfaccia: un indirizzo multicast come destinatario non e' trattato in modo esplicito **(non verificato nel codice di rete)**;
- ogni destinatario riceve lo stesso identico flusso (nessuna opzione per destinatario).

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
| PSK Reporter | IPFIX su UDP `report.pskreporter.info:4739`; lettura su `retrieve.pskreporter.info` | - | **OK** (gia' in Aurora, invio e lettura) | - |
| HamQTH | API per sviluppatori: caricamento del log ADIF e QSO singolo in tempo reale | - | **lavoro** | Invio in rete |
| DXHeat | Nessuna documentazione di sviluppo trovata | - | **?** | Verificare |
| HamAlert | Avvisi via app, Telnet (`set/json` per JSON), URL GET/POST | - | **poco lavoro** come sorgente (Aurora ha gia' un client telnet cluster) | Voce nei server telnet, filtri |
| OpenHamClock | Ascolta 2237 (segnalazione su GitHub); alla ricezione del traffico FT8 mostra gli spot | - | **OK** (non verificato dal vivo) | Multicast se 2237 e' occupata |
| SDR-Control e simili | Nessuna fonte trovata | - | **?** | Verificare |

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
| 1 | **Multicast** per `AURORA_UDP_FORWARD` (iscrizione al gruppo, TTL, interfaccia di loopback) e come ascolto | Un solo programma per volta puo' ascoltare 2237: con il multicast GridTracker, JTAlert, QLog, MacLoggerDX, CQRLOG, N1MM+, HRD condividono il flusso senza altro | Basso: solo rete, nessun messaggio nuovo | Quasi tutti |
| 2 | **Ascolto di Highlight Callsign (13)**, e colorazione delle righe di Aurora (o almeno memorizzazione) | CQRLOG, N1MM+ (dupe/moltiplicatori) e GridTracker colorano le righe cosi' | Medio: decodifica del `QColor`, mappa nominativo -> colore, UI | CQRLOG, N1MM+, GridTracker |
| 3 | ~~**Invio diretto** a Wavelog/Cloudlog, ClubLog e QRZ Logbook~~ **fatto** (sezione 6) | - | - | - |
| 4 | **Impostazioni per destinatario** (flusso binario si/no, ADIF si/no, schema, identificativo) e Logged ADIF acceso per i destinatari che lo vogliono | Evita doppioni (Log4OM, HRD, JTAlert) e permette destinazioni diverse | Basso-medio | Tutti i log locali |
| 5 | Ascolto di **Location (11)**, **Configure (15)**, **Switch Configuration (14)**, **Clear (3) in ingresso** | Locatore da GPS/programmi, impostazione di modo e frequenza da fuori, cambio di configurazione | Medio | N1MM+, JTAlert, programmi di controllo |
| 6 | Broadcast **N1MM XML `contactinfo`** (porta 12060) | HRD, programmi che imitano N1MM; molti traduttori lo accettano | Medio: XML con ID del QSO | HRD, gestionali di gara |
| 7 | ~~**LoTW** via TQSL~~ **fatto** (sezione 6) | - | - | - |
| 8 | Campi **Exchange sent/received** nel QSO Logged; flag «solo Abilita Tx» del Halt Tx | Completezza del protocollo di WSJT-X 2.x | Basso | Programmi di gara |
| 9 | **HamQTH**, **HamAlert** (telnet come sorgente) | Utili ma secondari | Basso-medio | Chi li usa |

**Raccomandazione sul primo passo**: il multicast (priorita' 1) perche' sblocca in una volta sola l'uso di piu' programmi sulla stessa porta senza toccare il protocollo; subito dopo l'ascolto di Highlight (13). Gli invii diretti (priorita' 3 e 7) sono fatti: sezione 6.

## 5. Impostazioni consigliate, riepilogo

| Obiettivo | Impostazione in Aurora |
|---|---|
| Mappa e avvisi di GridTracker 2 | `AURORA_UDP_FORWARD=127.0.0.1:2333` (o la porta di GridTracker), `AURORA_UDP_ACCEPT=1` |
| Log4OM 2 (QSO) | `AURORA_EXPORT_UDP=127.0.0.1:2237` |
| WaveLogGate | `AURORA_EXPORT_UDP=127.0.0.1:2333` |
| MacLoggerDX / QLog / CQRLOG | `AURORA_UDP_FORWARD=<indirizzo>:2237` (una sola applicazione per porta; con piu' applicazioni serve il multicast) |
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
- HamAlert: https://hamalert.org/about e https://forum.hamalert.org/t/how-to-make-this-work-on-a-computer/191
- OpenHamClock, segnalazione sulla porta 2237: https://github.com/accius/openhamclock/issues/577
- Codice di Aurora letto per questo documento: `core/crates/udp-bridge`, `core/crates/udp-emitter`, `core/crates/qso-export`, `core/crates/reporting`, `docs/udp-flussi.md`, `.env.example`.
