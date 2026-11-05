# TODO

Il dettaglio, con priorità e note, sta in `docs/mancante.md`; i lavori presi da WSJT-X 3.x stanno in `docs/tasks.md`. Stato a colpo d'occhio al 2026-11-01, v0.11.60.

## Aperti (2026-11-01)
- [ ] Rinominare il crate `engine-jtdx` (ora contiene solo i dati di Aurora) e i preset della cascata «JTDX»/«WSJT-X».
- [ ] Linux: alla prossima release controllare che il lavoro `Linux` di GitHub Actions alleghi AppImage e `.deb` e che i link della pagina web funzionino.
- [ ] **CAT4OM** (cat4om.com, dello stesso autore di Log4OM, IW3HMH): richiesto da un utente. Oggi dovrebbe funzionare con il ponte Hamlib/rigctld di CAT4OM (Aurora: Hamlib, radio «Hamlib NET rigctl», porta 127.0.0.1:4532, PTT CAT), ma non e' provato. Dalla 0.11.119 la porta 127.0.0.1:4532 si puo' scrivere nel campo Porta, prima il collegamento era impossibile. Da fare: provarlo (anche senza radio), poi aggiungerlo come scelta pronta in Impostazioni > Radio e nella Guida; valutare il protocollo nativo di CAT4OM con l'autore se da' piu' funzioni del ponte.
- [ ] Pagina dei rapporti degli errori nell'installatore (informativa e due caselle gia' spuntate); oggi c'e' solo l'avviso al primo avvio.
- [ ] Server UDP primario e secondario di fabbrica sulla stessa porta 127.0.0.1:2237 (QSO doppi nei log): decidere se spegnere il secondario.
- [ ] Tema del telefono condiviso con il PC (`aurora-theme` non e' tra le preferenze proprie del telefono): decidere se separarlo.
- [ ] Controllare che nessun file (ALL.TXT, registrazioni, log) venga scritto fuori dalla cartella di Aurora (fatto per i file audio).
- [ ] Riordino del menu Impostazioni (gruppi per funzione, pagina Servizi di log, via Avanzate, errori di salvataggio col nome del campo): in lavorazione
- [ ] Hound automatico, etichetta FOX, Tx sopra 1000 Hz e Salta Tx1 disattivato in Hound: in lavorazione
- [x] SuperFox: fatto come UltraFOX nella 0.11.70 (trasmissione, ricezione in Hound); manca la firma digitale.
- [ ] Lavori grandi da WSJT-X 3: TCI, Q65, MSK144, pulsanti dei modi, QMAP, pagina Fox (bozza approvata `docs/mockup-fox.html`)
- [ ] Prove dal vivo del proprietario: a7/a8, stop per ROS, invii a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP, display Yaesu/Icom, barre Rx/Tx

## Fatto
- [x] Fase 0: spike su JTDX (H1, H2, H5 confermate dal vivo)
- [x] Core: bus, hub UDP davanti a GridTracker2, motore JTDX, log SQLite, associazione dispositivi, API e WebSocket
- [x] Interfaccia: stazione, cascata WebGPU/WebGL2, finestre trascinabili, impostazioni a schede, sei lingue, tooltip
- [x] Dati: DXCC da `cty.dat`, QSO già fatti, etichette come JTDX, statistiche del log
- [x] Trasmissione: CQ, testo libero, frequenza Tx, risposta con doppio clic, STOP TX
- [x] Radio: cambio banda (OmniRig), cambio modo FT8/FT4, frequenza Rx
- [x] Automazioni con prova a vuoto, tetti e dead-man
- [x] Esportazione dei QSO verso log esterni
- [x] Guida in-app, pagina Statistiche, righe Tx nell'elenco
- [x] DX Cluster telnet (server pubblici e propri, LED di raggiungibilita', doppio clic = frequenza dello spot)
- [x] Client NTP configurabile con indicatore dell'orologio e DT mediano
- [x] Impostazioni del motore (JTDX) lette dal suo file e modificabili da Aurora (barra Applica e riavvia, copia di sicurezza)
- [x] Log di Log4OM2 in sola lettura, finestra Log
- [x] Modo JT, comandi di ricezione di JTDX, split della radio spento a ogni cambio banda
- [x] Messaggi Tx letti da JTDX (testi, pallino selezionato, Salta Tx1, periodo), selezionabili a mano
- [x] Dati subito dopo il riavvio (stato letto dalla finestra di JTDX, indirizzo UDP ricordato), cambio banda in meno di un secondo
- [x] Barre Rx/Tx larghe quanto il segnale, cascata continua, orologio UTC, opzione per svuotare l'attivita' di banda al cambio banda/modo
- [x] Analisi delle logiche di JTDX (`docs/logiche-jtdx.md`)

## Da fare
- [ ] **Doppio clic su R+rapporto dopo un QSO gia' registrato e la coda partita** (0.11.32): correzione in `reply_to_with` non riprodotta dal banco di prova; verificare in onda che resti RR73 e non torni al rapporto.
- [x] **Compensazione AGC (AGCc) nel motore proprio, FT8**: portata da JTDX e migliorata (vedi `memory.md`); da provare in onda.
- [x] **AGCc in FT4**: stessa compensazione di FT8 sul tratto decodificato (banco di prova: -16 dB 15→20/20, -17 dB 3→15/20; audio pulito e rumore pompato invariati).
- [ ] Test dell'interfaccia «Notifications > switches highlight and sound per kind» fallito una volta sotto carico (0.11.32): ricontrollare se e' instabile.
- [x] **Coda delle stazioni che chiamano** (tasto destro) e **Rispondi a tutti** (fatti il 2026-10-26, 0.11.23): vedi `memory.md`. Da provare con la radio.
- [x] **Hound e modo SWL nel motore proprio** (2026-10-26): Hound senza CQ, SWL con finestra, soglie e passate come JTDX (quarta sottopassata di SWL non portata, filtro +-290 Hz e salti di Hound non portati).
- [x] **JT65 e JT9 nel motore proprio** (0.11.24): selettore, ricezione, trasmissione, sequenza con RRR, frequenze di JTDX, `ALL.TXT`; da provare con la radio.
- [ ] **JT65 e JT9: parita' con JTDX sul segnale debole** e il resto (`docs/2026-10-26-jt65-jt9.md`): afc, subtract di JT9, avg65, dynhint, JT65+JT9 insieme, sottomodi B/C e JT9 veloci, messaggi brevi RO/RRR/73.
- [x] **Hound**: filtro +-290 Hz (0.11.24) e salti di frequenza dell'R+rapporto (0.11.25). Resta la sensibilita' fissa a 1, che riguarda `ft8s` (non portato).
- [x] **SWL**: filtro della sottrazione `NFILT2` (3400 punti a coseno, `cwfilter`/`subtractft8` di JTDX). Su segnali stabili il banco di prova non cambia (198/288 con l'uno e con l'altro filtro, `esplora_swl`); la differenza si vede con l'evanescenza.
- [ ] **SWL / sottopassate**: `ldeepsync` vale solo per la ridecodifica `evencopy`/`oddcopy` (`tonesd`, `ft8sd1`, `ft8mf1`, sottopassata `iqso=4` di `ft8b`), che Aurora non ha ancora: portarla tutta insieme.
- [ ] **Causa dell'arresto del 2026-10-26 (0.11.21)**: non trovata; dalla 0.11.22 il log ha i panic e le risorse del processo (`health.rs`): al prossimo arresto leggere `%APPDATA%\Aurora\logs\aurora.log`.
- [x] **Generali: Regione IARU e «Generazione dei messaggi per nominativi composti di tipo 2» funzionanti** (fatte il 2026-10-25, lette con REA in `jtdx.exe`: `docs/analisi-jtdx-reverse-rea.md`, sezione «Regione IARU e nominativi composti di tipo 2»): la regione filtra la tabella iniziale di JTDX (FT8 e FT4) e sceglie la riga di banda, cambio di modo e programmazione; il tipo 2 si salva e si legge ma, come in JTDX, vale solo per JT65/JT9/T10 e in FT8/FT4 non cambia nessun messaggio.
- [ ] **Frequenze: 8 m, 4 m, 70 cm e scelta delle alternative**: le righe di JTDX per 40,680 MHz (R1), 70 MHz (R1) e 432,174 MHz non ci sono perche' Aurora non ha quelle bande (`rig_control::bands`, ADIF, filtri); le righe non predefinite (14,071, 14,090...) restano nella tabella ma Aurora non ha una tendina per sceglierle.
- [x] **Scheda Impostazioni «Radio» = scheda «Radio CAT»** (fatta il 2026-10-25): una sola scheda «Radio» con collegamento, PTT, split e lettura, profili, stato e prova, condivisione e OmniRig; «Audio TCI» resta indicato come non disponibile.
- [x] **Riordino delle schede delle Impostazioni** (fatto il 2026-10-25, tutte tranne Audio e Segnalazioni): pagine larghe a riquadri e colonne con celle di altezza uguale, Notifiche come tabella vera, barra delle schede in due righe per tipo; componenti in `app/src/settings/layout/`, guida in `docs/impostazioni-layout.md`.
- [ ] **Riordino della scheda Segnalazioni** con lo stesso sistema (Audio fatta il 2026-10-25), appena finiti i lavori sul collegamento dei dispositivi audio e su log, spot e server TCP (passi in `docs/impostazioni-layout.md`).
- [x] **Interrompere la chiamata se il DX risponde a un altro: opzione dell'utente** (c'e': Impostazioni › Sequenza, `SeqHaltTxReplyOther`) (richiesta del proprietario): in Impostazioni -> Sequenza deve restare una voce chiara «Ferma la Tx se risponde un altro» che l'utente accende o spegne; spenta, Aurora insiste a chiamare finche' il DX risponde, anche se sta rispondendo ad altri (valore iniziale preso da JTDX.ini, chiave per chiave).
- [x] **Schede Sequenza, Generali e Notifiche come in JTDX** (fatte il 2026-10-25, restano in `docs/impostazioni-motore-proprio.md` le voci spente con il motivo) (richiesta del proprietario, da fare subito dopo la 0.11.17, con logiche e collegamenti presi da `jtdx.exe` con il reverse): *Sequenza* (messaggi per scenari brevi: rispondere al CQ di una chiamata senza risposta N volte, chiamata CQ con risposta ricevuta, rapporto di risposta al rapporto ricevuto, RR73 in risposta a RRR; ferma il Tx se l'operatore chiamato ha risposto ad un altro; operazione CQ rigorosa); *Generali* (descrizione della stazione e regione IARU, generazione dei messaggi per nominativi composti di tipo 2, filtro del log, opzioni dello schermo come distanza in miglia, nascondi i messaggi parzialmente trasmessi, nome del paese o prefisso, distanza e azimut nei messaggi con griglia, e di funzionamento come non monitorare all'avvio, decodifica a t = 52 s, cambi di frequenza Tx durante la trasmissione, periodo di monitoraggio e tempo per l'accordo, ID CW dopo 73); *Notifiche* (evidenzia nuova zona CQ/ITU, paese, griglia, prefisso, nominativo, gia' lavorati, per banda e per modo, suoni, colori dei messaggi e suggerimenti per la configurazione). Le schermate di riferimento le ha mandate il proprietario in chat (non nel repository, contengono il suo locatore).
- [x] **Scheda DXCC per banda e modo** (pagina Statistiche e blocco Stazione DX): sei etichette NEW ONE/BAND/MODE e NEW GRID/GRID BAND/GRID MODE, matrice paese x banda x modo (FT8, FT4, DIG), dal log interno + file ADIF + Log4OM2 senza doppioni; logica in `docs/scheda-dxcc.md`; esportazione e importazione ADIF del log interno (`GET /log/export.adi`, `POST /log/import`)
- [ ] Scheda DXCC: colore «per modo» sulle righe delle decodifiche (oggi solo nella scheda e nel blocco Stazione DX) e prova delle conferme con un log vero di Log4OM2 (nomi delle colonne QSL/LoTW/eQSL)
- [ ] Impostazioni del motore modificabili: provare «Applica e riavvia» con la radio (lo fa Tiziano) ed eventualmente aggiungere voci
- [x] Programma Windows (Tauri 2) con vassoio, avvio automatico, installatore personalizzato e aggiornamento firmato (chiave in `%USERPROFILE%\.aurora-signing`: **copiarla in un posto sicuro**)
- [ ] Provare l'installatore sul PC vero (provato in una cartella a parte) e un aggiornamento vero da una versione alla successiva: serve pubblicare i tre file in una release (`gh release upload`, vedi `docs/installer-windows.md`); per aggiornarsi dal repository privato serve il token in Impostazioni → Aggiornamenti
- [x] Mappa come GridTracker2 e pagina Propagazione da cruscotto (da rivedere con il tema chiaro)
- [ ] Telefono: riviste solo Stazione, testata e pulsanti; da rivedere una per una Storico, Propagazione, Mappa, Impostazioni, Automazioni (e provare sul telefono vero)
- [ ] Rendere molto piu' veloce Abilita Tx (sessione PowerShell fissa): oggi il doppio clic puo' perdere il periodo giusto
- [ ] Log remoto via UDP (QSO nuovi da Log4OM2 su un altro PC)
- [ ] Prove con la radio (le fa Tiziano): selezione a mano del messaggio Tx durante un QSO, modo JT, doppio clic sul cluster, applicazione delle impostazioni, svuotamento dell'elenco al cambio banda, barre Rx/Tx in FT8
- [ ] Verificare il tema chiaro dopo le ultime modifiche grafiche (controllate soprattutto in scuro)
- [x] Cascata e spettro come il Wide Graph di JTDX (finestra gaussiana, canali per pixel, N Avg, Scale, Flatten, tavolozze, Gain/Zero, grafico dello spettro, barre Rx/Tx): scheda Impostazioni > Cascata, vedi `docs/analisi-jtdx-reverse-rea.md`
- [ ] Cascata: provare dal vivo i segnali FT8 (devono essere sottili come in JTDX), la velocita' (N Avg 4-5 = JTDX) e le tavolozze; poi decidere se aggiungere il timestamp nella cascata (Timestamp di JTDX, non fatto)
- [ ] Da `docs/logiche-jtdx.md`: limite di ritardo con cui JTDX avvia una trasmissione (serve una prova con la radio) e livelli AutoSeq 4-5
- [ ] Pubblicazione: 9 commit locali non pushati; il nominativo reale resta in due commit gia' su GitHub (`561c576`, `248b7f3`) e nei tag v0.6.0-v0.8.0: riscriverli richiede un force-push (decide Tiziano)
- [ ] Versione smartphone: flusso dati corretto (2026-10-24), da riprovare sul telefono vero per layout e lampeggio
- [ ] Leggere il log di Log4OM2 (`Documents\LOG4OM2\<nominativo>.sqlite`, 36484 QSO contro i 2060 del log di JTDX) per novita' e statistiche: da decidere con Tiziano
- [x] Mappa dei QSO e dei CQ (resta: linee verso la stazione scelta e la mappa senza Log4OM2, dal log ADIF)
- [ ] Trasporti sicuri: HTTPS locale, Tailscale, Cloudflare con Access
- [x] Controllo delle nuove versioni sul repository con avviso (barra, evento, telefono)
- [ ] Programma Windows (Tauri 2) con avvio automatico di Aurora e aggiornamento automatico (OTA): scarica e installa la release; serve firma dei pacchetti e un repository leggibile (pubblico o con token)
- [x] Lista dei desiderati con avvisi (pagina, suono, notifica del browser, telefono via ntfy); resta la notifica push nativa (richiede HTTPS) e la prova con il telefono vero
- [x] Licenza: GPL-3.0 (solo, file `LICENSE`): il motore proprio porta algoritmi di JTDX e WSJT-X, che sono GPL 3
- [x] Livello RX: indicatore e avvisi, cursore e AUTO sul volume di registrazione di Windows (da provare dal vivo con la radio: AUTO in ricezione, che non tocchi nulla in TX)
- [ ] Decidere cosa integrare da Decodium (fatti: livello RX, storico decode, nascondi gia' lavorate, spot del cluster sulla cascata, PSK Reporter + propagazione, diagnosi automatica, automazioni piu' sicure (filtro dei falsi decode, risposta ai desiderati, priorita' tra piu' CQ nello stesso ciclo; da provare dal vivo in prova a vuoto prima del modo reale), comandi a JTDX rapidi (sessione fissa), riapplicazione del modo della radio dopo il cambio banda (da provare con la radio); scelti: livello RX, storico decode + spot su cascata, PSK Reporter + propagazione, automazioni piu' sicure; ordine: prima queste, poi il programma Windows): elenco in `docs/confronto-decodium.md` (slider livello RX, storico decode, PSK 'chi mi ha sentito', propagazione, spot cluster sulla cascata...)
- [ ] Prove manuali ancora aperte: Reply con carico fittizio, H4 (JTDX ridotto a icona), installazione del telefono come PWA

## Dai rapporti degli errori (2026-11-04)
- [x] (0.11.106: tempo massimo di 11 s per le passate FT8; resta da capire perche' quel PC e' cosi' lento) Decodifica lenta su un PC Windows 10 con IC-7300: «0 messaggi nel periodo in 66598 ms» (un periodo FT8 dura 15 s). Capire se e' la CPU o un blocco del motore (pacchetto DDB54164, anche un blocco della finestra di 20 s all'avvio).
- [ ] Avvio fallito su Windows 11 Dev (build 29698): WebView2 «handle della finestra non valido» (0x80070578), quattro volte di fila (pacchetto B04AE563).
- [ ] Avvio fallito 0.11.102: porta 8787 gia' in uso (seconda copia di Aurora o altro programma): dirlo in chiaro e proporre un'altra porta.
- [ ] «trasmissione non attiva: InvalidCall("")» su piu' installazioni: la guida del primo avvio (0.11.102) dovrebbe bastare, ricontrollare i rapporti.
- [ ] «l'audio ha perso un blocco di campioni»: 66 volte su una installazione; capire se basta un buffer piu' grande.

## Display grandi in testata (deciso il 2026-11-04, non ancora fatto)
- [x] (0.11.113) Testata: lingua, volume con cursore, tema e logo piccoli (16-18 px) in fila nella barra del menu, subito dopo «Novità»; il display grande occupa l'angolo in alto a destra, alto quanto le due righe di comandi.
- [ ] Opzioni del display prese da Hamlib per ogni modello (Get/Set functions, Get/Set level con i loro intervalli, VFO Ops, split): in Impostazioni > Radio compaiono solo quelle che la radio ha; il display mostra solo quelle spuntate.
- [x] (0.11.113, scelti tutti e sei) Display da scegliere tra le bozze `docs/mockup-display-grandi.html` (8) e `docs/mockup-display-icom.html` (Icom blu con banda passante e strumento multiplo). Niente spettro ne' cascata nel display.
- [ ] Provare dal vivo i display quasi quadrati in testata (compaiono solo con la radio collegata) e i clic sui tasti dei sei display nuovi.
- [ ] Le note della locandina degli aggiornamenti sono il testo della release (italiano): valutare note in sei lingue.
- [ ] Telefono: verificare con gli utenti che con la 0.11.120 si colleghino (QR con l'indirizzo vero, «Consenti Aurora» nel firewall).
- [ ] Display: opzioni lette da Hamlib per ogni modello (ancora aperto).
