# Cosa manca ad Aurora (elenco ordinato)

Stato al 2026-11-04, versione 0.11.174. Priorità: **A** indispensabile per sostituire JTDX nell'uso quotidiano, **M** utile, **B** rinviabile.
Fonti: specifica (`2026-10-24-aurora-design.md`), inventario di parità (`fase0-parita-funzioni.md`), rapporti dei piani.

## Già fatto

Core Rust (bus, hub UDP davanti a GridTracker2, motore JTDX, log SQLite con ADIF, associazione dispositivi, API e WebSocket); interfaccia a finestra larga e da telefono con finestre trascinabili e ridimensionabili; cascata WebGPU/WebGL2 con barre Rx/Tx; paese con bandiera e novità dei decode (DXCC, zona CQ/ITU, griglia, prefisso, stazione, per banda); avvisi sonori e chiamate verso di te; trasmissione (CQ, testo libero, risposta con doppio clic, STOP TX) chiusa di default; cambio banda, modo FT8/FT4 e frequenza Rx; livello audio di uscita (Pwr); automazioni con prova a vuoto; esportazione dei QSO; impostazioni a 13 schede, sei lingue e tooltip; filtri, macro, frequenze, programmazione; codice di associazione di durata scelta e accesso senza codice per le prove. Dalla 0.11.41 alla 0.11.46: impostazioni a barra laterale con spiegazioni, pagina Informazioni, notifiche, cascata che segue il filtro della radio, display esteso con secondo VFO, calendario DXpedition (NG3K), coda sempre visibile, rigctld orfano chiuso da solo.

Da verificare dal vivo: display esteso e secondo VFO su radio reali (ogni modello da' a Hamlib dati diversi), lettura NG3K nel tempo (la pagina puo' cambiare formato), cascata oltre 3000 Hz.

## Da fare, in ordine di lavoro

| # | Voce | Pri. | Note |
|---|---|---|---|
| 1 ✔ | **Cascata: contrasto automatico** | A | Il rumore di fondo reale e' alto (media 177/255): serve un livello di nero che segua il rumore, altrimenti la cascata e' tutta luminosa |
| 2 ✔ | **Dati DXCC**: paese/zone dei nominativi da `cty.dat` | A | Base per tutte le etichette e gli avvisi |
| 3 ✔ | **Lavorati**: DXCC, zone e griglie gia' collegati, dall'ADIF di JTDX (`wsjtx_log.adi`) | A | Letto all'avvio; si aggiorna a ogni QSO chiuso |
| 4 ✔ | **Etichette reali** nei decode: nuovo DXCC, nuova zona, nuova griglia, gia' lavorato per banda | A | Colori come in JTDX (CQ, DXCC, griglia, zona CQ/ITU, "sulla banda") |
| 5 ✔ | **Avvisi sonori** per le novità | A | Suoni generati dal browser, per categoria, con interruttore e volume |
| 6 ✔ | **Motore delle automazioni** (`qso-engine`): regole, prova a vuoto, tetto di cicli, dead-man switch, registro delle decisioni | A | Salvaguardie già decise in specifica §6; un Reply fa trasmettere |
| 7 ✔ | Pagina **Automazioni** nell'interfaccia | A | Regole, interruttori, cronologia con il motivo di ogni decisione |
| 8 ✔ | **Comando della radio** via rigctld (frequenza, banda, PTT) | A | H3 da verificare; JTDX potrebbe usare Hamlib NET rigctl |
| 9 ✔ | **TriggerCQ** e **SetTxDeltaFreq** (tipi 50 e 51) | M | Significato dei bool di `trigger_CQ` da confermare |
| 10 | **Trasporti sicuri**: HTTPS locale con certificato fissato dall'associazione, Tailscale, Cloudflare con Access | A | Oggi l'accesso da telefono e' HTTP sulla rete locale |
| 11 ✔ | **Programma Windows** (guscio Tauri 2) con avvio automatico, installatore e aggiornamento OTA firmato | A | Fatto (`desktop/`, `docs/installer-windows.md`); da provare l'installazione e un aggiornamento vero tra due versioni |
| 12 ✔ | **Pagina Impostazioni** a schede come JTDX | M | Le funzioni di JTDX che non si comandano da fuori (decodifica, filtri RX) restano in JTDX e la scheda Avanzate lo spiega |
| 13 ✔ | **Log e statistiche** di Aurora, premi DXCC/WAZ/griglie con avanzamento | M | Pagina Statistiche (0.7.0) e storico dei messaggi ricevuti (pagina Storico). Il log ufficiale resta Log4OM2 |
| 14 ✔ | **Wanted list** e avvisi sul telefono (ntfy) | M | Lista dei desiderati con avvisi in pagina e sul telefono; l'automazione puo' rispondere ai loro CQ |
| 15 ✔ | **Mappa** dei QSO e delle stazioni ricevute | M | Pagina Mappa; manca il terminatore (greyline) |
| 16 ✔ | **Lingue** (italiano, inglese, tedesco, francese, spagnolo, portoghese) | M | Tabelle in `app/src/i18n/strings` |
| 17 ◐ | **Modo SWL, Hound, JT65/JT9**: visualizzazione e comandi per modo | M | Fatti nel motore proprio con la 0.11.24 (selettore, ricezione, trasmissione, RRR, frequenze di JTDX); resta la parita' con JTDX sul segnale debole, JT65+JT9 insieme e i sottomodi (`docs/2026-10-26-jt65-jt9.md`) |
| 18 | Riproduzione dei cicli decodificati, statistiche di propagazione | B | |
| 19 | Tipo `HighlightCallsign` (evidenziare i lavorati dentro JTDX) | B | |
| 20 | Il bool finale dello `Status` di JTDX: capire che cosa e' | B | |

| 21 | ~~Wiki in-app~~ | fatto | Pagina `#/guida`, 11 argomenti in sei lingue (0.7.0) |
| 22 | ~~Pagina Statistiche~~ | fatto | Pagina `#/statistiche` da `GET /stats` (0.7.0) |
| 24 ◐ | **Versione smartphone** | A | Dati mancanti corretti; pagina Stazione rifatta a schede con pulsanti da dito (sotto 640 px). Da rivedere Storico, Propagazione, Mappa, Impostazioni, Automazioni e da provare sul telefono vero |
| 25 ✔ | **Priorita' tra piu' CQ** nello stesso ciclo, riapplicazione del modo dopo il cambio banda, comandi a JTDX rapidi | M | Fatti (0.10.x); il modo della radio e la scelta del CQ vanno provati con la radio |
| 26 ✔ | **Coda delle stazioni** (tasto destro) e **Rispondi a tutti** | A | 0.11.23, solo motore proprio |
| 23 | ~~Righe Tx nell'elenco~~ | fatto | Un'unica riga per trasmissione, dal messaggio dello stato (0.7.0) |
| 27 ◐ | **Info QSL** (LoTW, eQSL, diretta, bureau, manager) accanto al nome della stazione | M | Fatto (0.11.125-127): lettere L E D B; QRZ.com senza abbonamento XML non le manda, si completano con HamQTH. Resta da valutare: liste pubbliche gratuite LoTW (ARRL `lotw-user-activity.csv`) ed eQSL per chi non ha HamQTH |
| 28 | **Capacita' Hamlib per modello** per le opzioni dei display | M | Mostrare nei display solo i comandi che il modello supporta |
| 29 | **Blocchi colorati del telefono** (stazione chiamata, chi mi chiama) troppo grandi | M | Il proprietario deve mandare uno screenshot dal telefono; poi prototipo |
| 31 | **CAT4OM nativo** (WebSocket JSON): 1) tipo di CAT «CAT4OM»; 2) scelta fra piu' radio dalla pagina; 3) audio RX/TX in rete | A | Protocollo in `docs/cat4om-protocollo.md`; lacune del manuale da chiedere all'autore (rilascio dell'ownership, frequenze audio, riconnessione) |
| 33 ✔ | **Finestre sul secondo schermo e dashboard a blocchi** | A | 0.11.134-138: pannelli, Mappa e Propagazione in finestre a parte o in una dashboard trascinabile e ridimensionabile; Impostazioni › Finestre per ritrovare tutto. Da provare dal vivo con due schermi |
| 32 | **HTTPS per il telefono** | B | Scelta del proprietario (2026-11-04): per ora http con il consiglio per Chrome su Android sotto il QR; l'HTTPS vero richiede un dominio |
| 30 | **Prove dal vivo** di CAT4OM, dell'associazione del telefono con gli utenti e del log dopo RRR senza 73 (0.11.124) | A | Le fa il proprietario |
| 34 | **Altri modi digitali**: JT4, Q65, MSK144, FST4, FST4W e WSPR | M | Richiesti dal proprietario (2026-11-06). Oggi ci sono FT8, FT4, JT65 e JT9. Q65 e MSK144 sono gia' descritti in `docs/tasks.md` (§21, §22); per gli altri servono codifica, modulazione, decodifica, periodi e pulsanti dei modi |

## Limiti dichiarati (non risolvibili dall'esterno)

- **Linux** (non provata dall'autore): mancano OmniRig (solo Windows), PTT diretto su RTS/DTR della seriale, regolazione del volume di registrazione e aggiornamento automatico. Il PTT su seriale si potrebbe portare con `ioctl(TIOCMSET)` se qualcuno lo chiede.
- JTDX 159 non accetta `Configure` via UDP: livello di decodifica, cicli FT8, filtri RX e altre impostazioni del motore restano nella finestra di JTDX.
- Un Reply fa trasmettere JTDX; i comandi di GridTracker2 passano dall'hub senza il cancello di Aurora.
