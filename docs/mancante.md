# Cosa manca ad Aurora (elenco ordinato)

Stato al 2026-11-14, versione 0.11.254 pubblicata. Priorità: **A** indispensabile per sostituire JTDX nell'uso quotidiano, **M** utile, **B** rinviabile.
Fonti: specifica (`2026-10-24-aurora-design.md`), inventario di parità (`fase0-parita-funzioni.md`), rapporti dei piani.

## Già fatto
- 2026-11-11 (0.11.235-0.11.237 e dopo, su `main`): **decodificatori allineati a WSJT-X 3.3.0-beta6** (FT8, FT4, Q65, JT65, JT9, MSK144, con sorgenti e binari); **FST4 e FST4W** (periodi 15 s-30 min, verificati con `jt9.exe`); **WSPR-2** (beacon, percentuale di Tx, verificato con `wsprd.exe`); **JT65 sottomodi B e C** con media dei periodi (EME); **MSK144 a 30 s** e «Sh» in trasmissione; caselle di decodifica di WSJT-X (Impostazioni › Filtri › Decodifica); **Cat4OM nativo** (Aurora e Log4OM sulla stessa radio); **OmniRig via COM** diretto, senza PowerShell; rotore unificato, colonne del Contest, icone, schermata di avvio, installatore in sette lingue (con l'olandese).
- 2026-11-08 (ramo `lavoro/otp`): verifica OTP del SuperFox in Hound (`$VERIFY$ CALL 123456`, esclusa dall'UDP), etichetta Hound/Super Hound rossa o verde, seme OTP della Fox salvato dal motore, Fox log su Club Log e QSO per continente nella pagina Fox.
- 2026-11-08 (dopo la 0.11.196): «Installa e riavvia» con barra di download (percentuale, MB, velocita', tempo); pannello Log stretto che toglie le colonne meno importanti; elementi nascosti con le finestre vicine che riempiono il posto.

Core Rust (bus, hub UDP davanti a GridTracker2, motore JTDX, log SQLite con ADIF, associazione dispositivi, API e WebSocket); interfaccia a finestra larga e da telefono con finestre trascinabili e ridimensionabili; cascata WebGPU/WebGL2 con barre Rx/Tx; paese con bandiera e novità dei decode (DXCC, zona CQ/ITU, griglia, prefisso, stazione, per banda); avvisi sonori e chiamate verso di te; trasmissione (CQ, testo libero, risposta con doppio clic, STOP TX) chiusa di default; cambio banda, modo FT8/FT4 e frequenza Rx; livello audio di uscita (Pwr); automazioni con prova a vuoto; esportazione dei QSO; impostazioni a 13 schede, sei lingue e tooltip; filtri, macro, frequenze, programmazione; codice di associazione di durata scelta e accesso senza codice per le prove. Dalla 0.11.41 alla 0.11.46: impostazioni a barra laterale con spiegazioni, pagina Informazioni, notifiche, cascata che segue il filtro della radio, display esteso con secondo VFO, calendario DXpedition (NG3K), coda sempre visibile, rigctld orfano chiuso da solo.

Da verificare dal vivo: display esteso e secondo VFO su radio reali (ogni modello da' a Hamlib dati diversi), lettura NG3K nel tempo (la pagina puo' cambiare formato), cascata oltre 3000 Hz.

## Da fare, in ordine di lavoro

- **B** Comandi senza pulsante nell'interfaccia (dal controllo del 2026-11-07): `gen_msgs` (Gen. Msgs), `show_wanted`, `jt`; testi rimasti dal vecchio pannello Comandi (`cmd.dxCall`, `cmd.cqKind`, `cmd.cap.*`, `ctl.clock`...): il proprietario ha deciso di lasciarli cosi' (2026-11-07).
- **M** Provare dal vivo la nuova chiusura del QSO (RRR, RR73 senza risposta, ripresa dopo «Canc DX») e controllarla nei log del giorno.

- **M** Stazione che si adatta da sola alla larghezza della finestra (oggi i blocchi hanno misure fisse e a destra si taglia se la finestra e' piu' stretta).

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
| 17 ◐ | **Modo SWL, Hound, JT65/JT9**: visualizzazione e comandi per modo | M | Fatti nel motore proprio con la 0.11.24 (selettore, ricezione, trasmissione, RRR, frequenze di JTDX); sottomodi JT65 B/C con media dei periodi fatti (2026-11-11). Resta la parita' con JTDX sul segnale debole (`docs/2026-10-26-jt65-jt9.md`) |
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
| 31 ✔ | **CAT4OM nativo** (WebSocket JSON): 1) tipo di CAT «CAT4OM» ✔ (0.11.235); 2) scelta fra piu' radio dalla pagina ✔ («Cerca»: gruppi dalla gestione, radio dal Control come osservatore, stato, emulazione che le tiene, audio; «Usa» riempie gruppo e radio); 3) audio RX/TX in rete ✔ (dispositivo «Cat4OM (audio in rete)»: ricezione 12 kHz mono o 48 kHz, trasmissione 48 kHz a blocchi da 20 ms, stream aperto prima del PTT e riaperto dopo 60 s senza audio) | A | Protocollo in `docs/cat4om-protocollo.md`. Da provare dal vivo con Log4OM/Cat4OM (soprattutto audio: latenza accettata, anticipo di 40 ms dei blocchi, `welcome` dell'osservatore `noRegister`); lacune del manuale da chiedere all'autore (rilascio dell'ownership, frequenze audio accettate, latenza, riconnessione) |
| 33 ✔ | **Finestre sul secondo schermo e dashboard a blocchi** | A | 0.11.134-138: pannelli, Mappa e Propagazione in finestre a parte o in una dashboard trascinabile e ridimensionabile; Impostazioni › Finestre per ritrovare tutto. Da provare dal vivo con due schermi |
| 32 | **HTTPS per il telefono** | B | Scelta del proprietario (2026-11-04): per ora http con il consiglio per Chrome su Android sotto il QR; l'HTTPS vero richiede un dominio |
| 30 | **Prove dal vivo** di CAT4OM, dell'associazione del telefono con gli utenti e del log dopo RRR senza 73 (0.11.124) | A | Le fa il proprietario |
| 34 ◐ | **Altri modi digitali**: JT4, Q65, MSK144, FST4, FST4W e WSPR | M | Richiesti dal proprietario (2026-11-06). Fatti: JT4 A-G, Q65 (periodi 15-300 s, sottomodi A-E), MSK144 (5/10/15/30 s, «Sh»), FST4 e FST4W (15 s-30 min, verificati con `jt9.exe`), WSPR-2 (verificato con `wsprd.exe`). Fatti anche il radiofaro FST4W e l'invio facoltativo a WSPRnet (spento di fabbrica). Manca: «Round Robin» fisso di FST4W (1/2, 2/3...) e salto di banda WSPR; prove dal vivo con la radio |
| 35 | **Traduzione olandese** dell'interfaccia (#43) | M | In lavorazione, come settima lingua in un dizionario a parte; l'installatore e' gia' in sette lingue (con l'olandese) |
| 36 | **Split DX** dal DX Cluster (UP/DN/QSX nel commento dello spot) | M | In lavorazione |
| 37 | **Prove dal vivo con la radio** di FST4/FST4W, WSPR, MSK144 vero, JT65 B/C, OmniRig via COM, Cat4OM, TCI, SSTV | A | Le fa il proprietario |

## Limiti dichiarati (non risolvibili dall'esterno)

- **Linux** (non provata dall'autore): mancano OmniRig (solo Windows), PTT diretto su RTS/DTR della seriale, regolazione del volume di registrazione e aggiornamento automatico. Il PTT su seriale si potrebbe portare con `ioctl(TIOCMSET)` se qualcuno lo chiede.
- **macOS** (non provata dall'autore, dal 2026-11-08): `.github/workflows/macos.yml` compila un `.dmg` universale (Apple Silicon e Intel) a ogni release, allegato come `Aurora-macOS.dmg`. Non firmato da Apple: al primo avvio serve tasto destro > Apri (o `xattr -cr /Applications/Aurora.app`). Come su Linux mancano OmniRig e PTT sulla seriale; l'aggiornamento dall'app c'e' (pacchetto `Aurora-macOS.app.tar.gz` con firma minisign, Aurora.app sostituita e riaperta, da provare su un Mac vero); Hamlib va installato a parte (`brew install hamlib`).
- JTDX 159 non accetta `Configure` via UDP: livello di decodifica, cicli FT8, filtri RX e altre impostazioni del motore restano nella finestra di JTDX.
- Un Reply fa trasmettere JTDX; i comandi di GridTracker2 passano dall'hub senza il cancello di Aurora.
