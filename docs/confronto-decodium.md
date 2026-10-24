# Confronto con Decodium 4.0 «Core Shannon»

Studio del 2026-10-24 del repository <https://github.com/iu8lmc/Decodium-4.0-Core-Shannon> (versione 1.0.705, scaricato in una cartella
temporanea, letto senza eseguire nulla). Quattro letture parallele (interfaccia, servizi e rete, radio/audio/sequenza, altri modi), con
file e righe a sostegno di ogni voce; dove il nome prometteva più del codice lo diciamo.

## Cos'è e cosa significa per Aurora

- Decodium è una stazione digitale completa in **Qt/QML + C++** (circa 3.100 file), figlia di WSJT-X: ha i **propri decoder** (FT8, FT4,
  FT2, Q65, MSK144, JT65/JT9/JT4, FST4, WSPR), CW, RTTY, SSB, SSTV, voice keyer, rotore, SDR, mappa, cluster, PSK Reporter.
- Aurora è un'altra cosa: **non decodifica**, usa JTDX come motore e lo comanda. Per questo tutto ciò che richiede un decoder o un
  modulatore propri (più flussi FT8 insieme, Q65, SSTV, CW...) è fuori portata o richiede un lavoro enorme; tutto ciò che sta **attorno**
  (livelli audio, mappa, avvisi, storico, servizi, interfaccia) si può replicare.
- **Licenza**: Decodium è GPL-3.0. Aurora è ora GPL-3.0-or-later, quindi il codice sarebbe compatibile; ma è C++/QML e non si porta
  riga per riga in Rust/TypeScript: si **riscrivono le idee** dopo averle capite, indicando la fonte quando serve.

## Lo slider RX/TX dell'intestazione

- **TX**: 450 → 0, cioè attenuazione da 45,0 a 0 dB, come il nostro Pwr (stessa scala di JTDX). Decodium in più ricorda il livello **per banda**
  (Impostazioni → Audio) e ha un overlay «TX Power %» con rotella sul pulsante Abilita Tx.
- **RX**: 0–100, è un **guadagno software** sul PCM (fino a +12 dB), non il volume di Windows. Ha il tasto **AUTO**: porta il picco a 0,70, scende
  subito se c'è clipping (picco ≥ 0,93, ≥ 2 campioni saturi o ≥ 0,1 %), sale piano dopo 3 s di segnale basso (+12 % al massimo per volta),
  si disattiva se lo muovi a mano. Un watchdog avvisa dopo 3 s di saturazione («abbassa il livello di Windows o della radio»).
- **Per Aurora**: JTDX non permette di regolare l'ingresso, e il suo livello Rx segnava 90+ dB (troppo alto). Si può fare lo stesso agendo sul
  **volume di registrazione di Windows del dispositivo che usa JTDX** (lo dice `SoundInName` in `JTDX.ini`), con il misuratore e l'avviso di
  overdrive calcolati dal nostro ingresso audio. Il clipping dell'ADC si corregge solo abbassando il volume nativo (non dopo).

## Cosa Aurora non ha: elenco completo

Difficoltà: **S** piccola, **M** media, **L** grande. «Fattibile» = realizzabile con JTDX/OmniRig come sono. Colonna *Consiglio*: **Sì** (vale la
pena), **Forse**, **No**.

### Audio e radio

| # | Funzione di Decodium | Cosa fa davvero | Per Aurora | Diff. | Consiglio |
|---|---|---|---|---|---|
| 1 | Misuratore e **avviso di overdrive/silenzio** | Picco ≥ 0,98 e rms ≥ 0,35 per 3 s = overdrive; ≥ 35 % campioni saturi; onda quadra; silenzio piatto 8 s | Dal nostro ingresso audio (già catturato per la cascata) | S | **Sì** |
| 2 | **Livello RX regolabile + AUTO** | Vedi sopra | Volume di registrazione Windows del dispositivo di JTDX (Core Audio) con regola AUTO e blocco durante il TX | M | **Sì** |
| 3 | Livello TX per banda | Ricorda la potenza per banda | Salvare il Pwr per banda e rimetterlo al cambio banda | S | **Sì** |
| 4 | Gestione **QSY** della radio | Dopo un salto ignora per 8 s le frequenze vecchie lette dalla radio, ripete il QSY fino a 2 volte, **riapplica il modo dopo 500 ms** (alcune radio tornano al modo memorizzato per banda) | Il nostro script di cambio banda può leggere la frequenza reale e ritentare/riapplicare il modo | S | **Sì** |
| 5 | Ritardo TX e slot | Un TX parte solo entro lead-in + 150 ms, altrimenti slitta; prima di un TX automatico aspetta ≥ 900 ms dal confine per finire il decode | Utile alle nostre automazioni; il limite di JTDX resta da misurare | S | Forse |
| 6 | Calibrazione ALC | Tune + lettura ALC fino al target, con tolleranza | Servono ALC/SWR dalla radio: **OmniRig li azzera** | M–L | No |
| 7 | Protezione SWR (blocca il TX oltre 2,5) | Richiede il sensore | Idem: con OmniRig non c'è | M | No |
| 8 | Amplificatore SPE Expert (telemetria) | Legge W, SWR, V, A, °C dalla porta seriale | Solo per chi ha l'amplificatore | S–M | No (per ora) |
| 9 | Rotore (controller seriale, rotctld, PSTRotator) | Punta l'antenna, anche sul DX lavorato | Aggiungibile come client rotctld | S | Forse |
| 10 | RTL-SDR, backend TCI/HRD/CAT4OM, DecoPort/Decolink | Radio in rete e SDR | Ridondante (OmniRig è già multi-client, la PWA fa da remoto) | L | No |
| 11 | Budget dei thread / modalità PC lento | Riduce carico su PC datati | Nel web: meno animazioni e niente WebGPU pesante | S | Forse |

### Sequenza dei QSO e automazioni

| # | Funzione | Cosa fa davvero | Per Aurora | Diff. | Consiglio |
|---|---|---|---|---|---|
| 12 | **Filtro anti-ghost** sui decode | Scarta decode falsi prima di usarli: forma ITU del nominativo, soglia −23 dB per nominativi mai sentiti, regole su distanza/locatore/«?», secondo decode per i deboli | Per le nostre automazioni, che rispondono ai messaggi | M | **Sì** |
| 13 | **Auto Call** | Risponde al CQ scelto (esclude i già lavorati su banda, priorità recente / SNR / distanza), tetto 5 QSO, si disarma allo stop | Aurora ha già automazioni con regole e tetti: confrontare e integrare le priorità | S | **Sì** |
| 14 | Direct Call con «DX-watch» | Arma il target e chiama solo quando compare; conta i TX a vuoto (max 10), timeout 90 s | Utile per i DX rari (si collega alla lista dei desiderati) | S–M | **Sì** |
| 15 | Auto CQ con cadenza | Dopo N CQ ascolta M cicli, mai due CQ di fila, tetto totale | Comandabile via `TriggerCQ` | S | Forse |
| 16 | Retry e chiusura QSO | Tetti per passo e per 73/RR73 per modo, logga al tetto, barriera «mai RR73 a chi non ha il mio report» | **Lo fa JTDX** (AutoSeq e contatori): Aurora può solo osservare | M | No |
| 17 | Coda chiamanti | FIFO fino a 20 | Con JTDX un QSO alla volta: utile solo come lista | S | Forse |
| 18 | Multi Answer Mode / DXpedition (fino a 10 flussi) | Trasmette più FT8 insieme con un modulatore proprio | Richiede decoder/modulatore propri: **JTDX non lo ha** | L | **No** |

### Elenco messaggi, mappa e statistiche

| # | Funzione | Cosa fa davvero | Per Aurora | Diff. | Consiglio |
|---|---|---|---|---|---|
| 19 | **Nascondi già lavorate** (banda, oggi, ieri, mai; Wait&Pounce) | Filtri dell'elenco | Aurora ha le etichette ma non nasconde | S | **Sì** |
| 20 | Colonne personalizzabili, riordino, **modalità compatta**, «più recenti in alto» | Menu sull'intestazione | Parte c'è (colonne a larghezza fissa); aggiungere scelta e compatto | S | Forse |
| 21 | **Suoni diversi per categoria** | CQ, MyCall, DXCC nuovo, DXCC su banda, griglia, continente, zone CQ/ITU, QSY | Oggi 3 suoni; i wav «stile JTDX» si possono sintetizzare | S | Sì |
| 22 | **Storico dei decode** (SQLite) | Tutti i RX con ricerca (nominativo, banda, modo, date), statistiche, pulizia a 30 giorni, export ADIF (attenzione: non sono QSO, non va nel log) | Nuovo crate e pagina | S–M | **Sì** |
| 23 | **Spot del cluster sulla cascata** | Segna gli spot sulla frequenza audio e un clic chiama | Aurora ha cluster e cascata, manca l'incrocio | S–M | **Sì** |
| 24 | QSY Quick Picker (F2) e scorciatoie | Frequenze di lavoro filtrabili per modo/banda/preferite; F1 monitor, F3 AutoSeq, F4 log, F11 | Piccola finestra e tasti | S | Sì |
| 25 | Mappa: **greyline**, proiezioni, preset, screenshot | Solo grafica | Si aggiunge alla nostra mappa | S–M | Forse |
| 26 | Mappa: ricezione PSK in tempo reale, POTA/IOTA, MUF/Es/aurora, VUCC per griglia, tab ROSTER/AWARDS/ALERTS | Molti strati, da fonti pubbliche | Un livello alla volta | M ciascuno | Forse |
| 27 | Premi (DXCC/WAZ/WAS/WAC/ITU/griglie) con obiettivo Worked/Confirmed e mancanti | Oltre le nostre statistiche | Estendere la pagina Statistiche | M–L | Forse |
| 28 | Logbook/Statistiche con filtri e confronto tra periodi, export JSON/CSV | Oltre il nostro log | Estensione | M | No (c'è Log4OM2) |
| 29 | Tema: luminosità e contrasto, densità, colori accessibili sui chiari | Sul tema | Si aggiunge ai nostri token | S | Forse |
| 30 | Reset layout, orologio mondiale, pannello «Active Stations» | Piccole comodità | Parte c'è | S | Forse |

### Servizi e rete

| # | Funzione | Cosa fa davvero | Dati che escono | Diff. | Consiglio |
|---|---|---|---|---|---|
| 31 | **PSK Reporter «chi mi ha sentito»** | HTTP `retrieve.pskreporter.info` (senderCallsign, ultime N ore): ricevente, locatore, SNR, frequenza, km, DXCC; in più feed live MQTT `mqtt.pskreporter.info:1883` (in chiaro) | Solo il tuo nominativo e l'IP; nessun account | S (HTTP), M (MQTT) | **Sì** |
| 32 | PSK Reporter: **invio spot** | Datagrammi IPFIX UDP a `report.pskreporter.info:4739` | Tutti i tuoi decode | S–M | **No**: JTDX lo fa già (`PSKReporter=true`) |
| 33 | **Propagazione** (SFI, A, K, raggi X, MUF, condizioni HF/VHF) | GET `hamqsl.com/solarxml.php` ogni 15 min | Solo l'IP; controllare i termini d'uso | S | **Sì** |
| 34 | **Autodiagnosi** (6 regole) | Offset orologio giudicato in proporzione allo slot (4 % avviso, 8 % problema), versione vecchia, nominativo mancante, zero decode dopo 3 minuti, troppi thread | Nulla | S | **Sì** |
| 35 | Orario di ripiego via HTTPS | Mediana dell'header `Date` di 5 grandi servizi quando l'NTP è bloccato | L'IP arriva a 5 aziende | S–M | Forse |
| 36 | **Elenco utenti LoTW** | CSV `lotw-user-activity.csv`, marca i nominativi che usano LoTW | Solo l'IP | S | Sì |
| 37 | AutoSpot **verificato** su cluster | Dopo un QSO manda lo spot e controlla che sia comparso | Rende pubblici nominativo, DX, frequenza | S–M | **No**: Aurora non manda mai spot (regola) |
| 38 | Callsign Intelligence (FCC, LoTW/eQSL/QRZ/Club Log) | Conferme e archivi | Password dei servizi | L | No (le conferme le gestisce Log4OM2) |
| 39 | Invio QSO a QRZ Logbook e Cloudlog | POST dell'ADIF | Il QSO | S ciascuno | No (lo fa Log4OM2) |
| 40 | **Aggiornamento automatico** | Controlla le release, scarica ed esegue l'installer: **non verifica firme né hash** | — | M | Da fare con **SHA-256/firma** (il nostro controllo c'è, manca l'installazione) |
| 41 | Segnalazione bug con diagnostica | Raccoglie sistema, audio, stato, ultime 30 righe di log; l'invio va su un forum di terzi | Nominativo e log | M | Sì, **ma solo come file da salvare** |
| 42 | Credenziali **cifrate** (DPAPI/Keychain) | Token e password non in chiaro | — | S | **Sì** (il nostro token e l'indirizzo ntfy oggi sono in un file) |
| 43 | Certificati `.dxcert` e «Decodium certificate» | HMAC con chiave scritta nel sorgente pubblico | — | — | **No** (non è sicurezza vera) |

### Altri modi (richiedono un decoder o un DSP proprio: JTDX non li ha)

| # | Modo | Cosa c'è in Decodium | Diff. per Aurora | Consiglio |
|---|---|---|---|---|
| 44 | **CW** | Decoder `ggmorse` (MIT), keyer a 4 vie (tono audio, DTR/RTS, WinKeyer, `send_morse` di Hamlib), macro F1–F12 | M | Interessante, ma è un progetto a sé |
| 45 | **RTTY** | DSP reale (filtri adattati, AFC, Viterbi sul codice Baudot), modulatore, log ADIF | M | Idem |
| 46 | **WSPR** | `wsprd` incorporato | M | Il più economico tra i nuovi modi |
| 47 | Q65, FST4, JT65/JT9 ecc. | Porting C++ dei sorgenti Fortran di WSJT-X | L ciascuno | No (JTDX ne ha già alcuni) |
| 48 | SSTV | 52 modi analogici, galleria, condivisione opt-in; solo 3 modi con prova indipendente | L | No (per ora) |
| 49 | Voice keyer, assistente vocale | Registra/riproduce 8 slot con PTT; riconoscimento (whisper) e sintesi (Piper) locali, modelli da 0,5-1,1 GB | S (keyer), L (assistente) | Solo il keyer, forse |
| 50 | Satelliti, EME, astronomia, telemetria meteo (messaggio 0.5 non standard) | TLE da CelesTrak, Doppler | L | No |

## Promesse non mantenute o codice morto in Decodium (da non copiare)

`QueuePanel` e `MessageAvgPanel` (mai usati), scorciatoie documentate F1–F7 ma diverse da quelle vere, `submitToGitHub` senza token,
certificati verificati con una chiave pubblica, `DecodiumAmplifier` solo in lettura, `AudioModeChangeCapturePolicy` che decide solo chi
possiede la cattura, Kalman/HTTPS/auto-calibrazione solo come monitor (la correzione usa l'offset NTP grezzo), assistente vocale che non
legge né la radio né il QSO.

## Cosa si impara dal loro CHANGELOG (utile anche a noi)

Un retry che azzera lo stato al passo sbagliato perde l'RR73 in arrivo; il watchdog di TX deve avere priorità sul tetto dei retry; due
stazioni nella stessa fase restano in stallo; rilasciare il PTT subito dopo l'apertura del CAT; una sonda CAT troppo insistente può
saturare il bus e incollare il PTT; il «già lavorato» va tenuto per nominativo, banda e modo; non ricreare la cattura audio al cambio modo.
