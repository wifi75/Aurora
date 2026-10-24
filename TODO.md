# TODO

Il dettaglio, con priorità e note, sta in `docs/mancante.md`. Qui solo lo stato a colpo d'occhio (2026-10-24, v0.10.3).

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
- [ ] Impostazioni del motore modificabili: provare «Applica e riavvia» con la radio (lo fa Tiziano) ed eventualmente aggiungere voci
- [x] Programma Windows (Tauri 2) con vassoio, avvio automatico, installatore personalizzato e aggiornamento firmato (chiave in `%USERPROFILE%\.aurora-signing`: **copiarla in un posto sicuro**)
- [ ] Provare l'installatore sul PC vero (provato in una cartella a parte) e un aggiornamento vero da una versione alla successiva: serve pubblicare i tre file in una release (`gh release upload`, vedi `docs/installer-windows.md`); per aggiornarsi dal repository privato serve il token in Impostazioni → Aggiornamenti
- [x] Mappa come GridTracker2 e pagina Propagazione da cruscotto (da rivedere con il tema chiaro)
- [ ] Telefono: riviste solo Stazione, testata e pulsanti; da rivedere una per una Storico, Propagazione, Mappa, Impostazioni, Automazioni (e provare sul telefono vero)
- [ ] Riordino delle pagine delle impostazioni: gruppi, colori, struttura chiara
- [ ] Rendere molto piu' veloce Abilita Tx (sessione PowerShell fissa): oggi il doppio clic puo' perdere il periodo giusto
- [ ] Log remoto via UDP (QSO nuovi da Log4OM2 su un altro PC)
- [ ] Prove con la radio (le fa Tiziano): selezione a mano del messaggio Tx durante un QSO, modo JT, doppio clic sul cluster, applicazione delle impostazioni, svuotamento dell'elenco al cambio banda, barre Rx/Tx in FT8
- [ ] Verificare il tema chiaro dopo le ultime modifiche grafiche (controllate soprattutto in scuro)
- [ ] Cascata: dire come la vuole Tiziano (velocita', contrasto, colori) ora che scorre di continuo
- [ ] Da `docs/logiche-jtdx.md`: limite di ritardo con cui JTDX avvia una trasmissione (serve una prova con la radio) e livelli AutoSeq 4-5
- [ ] Pubblicazione: 9 commit locali non pushati; il nominativo reale resta in due commit gia' su GitHub (`561c576`, `248b7f3`) e nei tag v0.6.0-v0.8.0: riscriverli richiede un force-push (decide Tiziano)
- [ ] Versione smartphone: flusso dati corretto (2026-10-24), da riprovare sul telefono vero per layout e lampeggio
- [ ] Leggere il log di Log4OM2 (`Documents\LOG4OM2\<nominativo>.sqlite`, 36484 QSO contro i 2060 del log di JTDX) per novita' e statistiche: da decidere con Tiziano
- [x] Mappa dei QSO e dei CQ (resta: linee verso la stazione scelta e la mappa senza Log4OM2, dal log ADIF)
- [ ] Trasporti sicuri: HTTPS locale, Tailscale, Cloudflare con Access
- [x] Controllo delle nuove versioni sul repository con avviso (barra, evento, telefono)
- [ ] Programma Windows (Tauri 2) con avvio automatico di Aurora e aggiornamento automatico (OTA): scarica e installa la release; serve firma dei pacchetti e un repository leggibile (pubblico o con token)
- [x] Lista dei desiderati con avvisi (pagina, suono, notifica del browser, telefono via ntfy); resta la notifica push nativa (richiede HTTPS) e la prova con il telefono vero
- [x] Licenza: GPL-3.0-or-later (file `LICENSE`)
- [x] Livello RX: indicatore e avvisi, cursore e AUTO sul volume di registrazione di Windows (da provare dal vivo con la radio: AUTO in ricezione, che non tocchi nulla in TX)
- [ ] Decidere cosa integrare da Decodium (fatti: livello RX, storico decode, nascondi gia' lavorate, spot del cluster sulla cascata, PSK Reporter + propagazione, diagnosi automatica, automazioni piu' sicure (filtro dei falsi decode, risposta ai desiderati, priorita' tra piu' CQ nello stesso ciclo; da provare dal vivo in prova a vuoto prima del modo reale), comandi a JTDX rapidi (sessione fissa), riapplicazione del modo della radio dopo il cambio banda (da provare con la radio); scelti: livello RX, storico decode + spot su cascata, PSK Reporter + propagazione, automazioni piu' sicure; ordine: prima queste, poi il programma Windows): elenco in `docs/confronto-decodium.md` (slider livello RX, storico decode, PSK 'chi mi ha sentito', propagazione, spot cluster sulla cascata...)
- [ ] Prove manuali ancora aperte: Reply con carico fittizio, H4 (JTDX ridotto a icona), installazione del telefono come PWA
