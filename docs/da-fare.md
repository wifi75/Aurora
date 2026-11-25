# Da fare (breve)

Si aggiorna a ogni chiusura: `[x]` = fatto. Elenco lungo e tecnico: `docs/mancante.md`.

## Adesso
- [x] 0.11.315 pubblicata (rotore fuori dalla barra, «Mostra in home» nelle Impostazioni): da guardare a schermo
- [ ] Controllo licenze fatto il 2026-11-20 (sola lettura): crediti, `LICENSE`, licenze di Hamlib e attribuzioni delle mappe a posto; corretto `qso-native` (intestazione) e `THIRD-PARTY-NOTICES.md` nelle risorse dell'installatore (esce con la prossima versione); da verificare a mano: condizioni d'uso dei dati (cty.dat, OpenStreetMap)
- [x] 0.11.320 pubblicata (IC-820H sulla banda giusta, licenze nell'installatore) e messaggio `aggiorna-0.11.320` mandato per l'ultima volta (poi niente piu' messaggi di aggiornamento)
- [ ] Messaggio `aggiorna-0.11.320`/`aggiorna-0.11.319`: guardare `messaggi_visti`, `messaggi_clic` e le installazioni su 0.11.319+ (verso le 19-20 UTC) per vedere quanti si allineano
- [x] Casella «Aggiorna da solo quando Aurora e' ferma» pronta (spenta di base), importatore macro fldigi e pulsante «rigctld gia' acceso»: commit locali, NON pubblicati, da raggruppare nella prossima versione; manca la Guida (sei lingue + nl/ru) e guardare a schermo la finestra di importazione
- [x] 0.11.319 pubblicata (etichetta dei satelliti col conto alla rovescia sempre visibile, messaggio se la finestra non si apre per WebView2): chiudere #82 quando ct1bxx conferma
- [ ] Messaggio di WebView2 in olandese e russo (ora ricade su un'altra lingua): controllare come `agent::ui_lang()` sceglie la lingua
- [x] 0.11.318 pubblicata (IC-820H: trasmissione sul VFO della sua banda): attendo il test di Vladimir con ISS o RS-44 e la sua risposta su quale banda sta su Main
- [x] #86 M8GVC chiusa: il CQ parte con AutoTX spento
- [x] 0.11.317 pubblicata (CQ con AutoTX spento #86, Guida aggiornata): risposta data a M8GVC, da provare
- [ ] #83 IC-820H con SAT acceso: la radio scambia da sola principale e secondario, Aurora legge 145 e 435 MHz a turno (212 «cambi di banda»): serve il test di Vladimir; poi decidere come leggere solo il VFO giusto
- [ ] #85 Luc: risposta data (watchdog di trasmissione), attendo i suoi minuti
- [x] 0.11.316 pubblicata: diagnosi della trasmissione (pallini, «Prova senza trasmettere», registro): da guardare a schermo e con una radio vera
- [ ] Diagnosi della trasmissione, resta: prove con il finto rigctld, confronto del modo con quello richiesto (`docs/piano-diagnosi-trasmissione.md`)
- [x] 0.11.314 pubblicata (etichetta satelliti opzionale e al centro della barra, locandina senza barra del download): da guardare a schermo
- [x] 0.11.313 pubblicata (WEFAX ricorda la frequenza e la banda di prima, menu 19 rem, locandina senza striscia, divisore cascata): da guardare a schermo
- [x] Locandina di aggiornamento senza l'avanzamento del download (0.11.314)
- [x] 0.11.311 pubblicata con il russo (da far rivedere a Vladimir; manca nell'installatore e nelle schede del tray)
- [x] 0.11.310 pubblicata (locandina diretta, un ingranaggio, WEFAX su una riga e senza TUNE)
- [x] 0.11.309 pubblicata (satelliti, ADIF automatico, conferma di chiamata, ROS 2,0, ora automatica)
- [ ] #83 Vladimir: pubblicare la risposta in russo (pronta) e chiedere la prova a mano V Sub/V Main
- [ ] Rispondere a Pascal (F4LHL): ROS 2,0 nella scheda Radio, aurora_log.adi gia' scritto a ogni QSO, cartella ADIF a scelta, ALC (pacchetto)
- [ ] Caso XX9BB: serve il pacchetto con la nuova riga «in onda» al prossimo caso
- [x] WEFAX: pagina nell'interfaccia fatta
- [x] WEFAX: menu delle frequenze, finestra staccata e Guida fatti (0.11.307)
- [ ] WEFAX: guardarla a schermo e provarla con un segnale vero
- [x] Pubblicata la 0.11.307 (TUNE, SSTV 20 m, avvisi, WEFAX, olandese)
- [ ] Avvisi di aggiornamento: provarli dal vivo (finestrella «Vuoi aggiornare?», cartolina del tray)
- [ ] Finestrella di aggiornamento nativa prima dell'apertura di Aurora (solo se la vuoi)

## Utenti in attesa
- [ ] #83 Vladimir (IC-820H, ex #72): con la 0.11.306 il Doppler della ricezione va, ma la trasmissione (uplink) no; il modo non cambia (RX USB, TX LSB, a mano) e il SUB non si accende. Pacchetto B8024BF8 da leggere
- [ ] #72 Hernan (IC-707 senza CAT, macOS): attende il suo pacchetto di assistenza
- [ ] #81 Luc: prova Wait and Pounce
- [ ] #80 HB9TZU: quali comandi FLDigi mancano
- [ ] #82 ct1bxx: etichetta satelliti pubblicata nella 0.11.312, da provare con un satellite vero; avvisarlo e chiudere
- [x] F1DSZ: risposta inviata; conferma di chiamata corretta su `main` (esce nella 0.11.312); gli ho dato un percorso sbagliato: l'interruttore e' in Modi › FT8 · FT4 › Messaggi e filtri › Schermo
- [x] 0.11.312 pubblicata (conferma di chiamata opzionale, WEFAX, gruppo PSK, russo installatore e tray, etichetta satelliti #82, pulsanti temi scuri): da guardare a schermo
- [ ] Russo: far rivedere a Vladimir; provare l'installatore in russo e le cartoline del tray (non provati)
- [ ] #74 Luc: risposta sui contest, poi importare gli altri contest come gli ARRL

## Prove con la radio (le fai tu)
- [ ] Display, TUNE, clic misuratori TS-850, ingresso audio Windows 11
- [ ] FST4/WSPR/MSK144, JT65, OmniRig, Cat4OM, TCI, SSTV
- [ ] Spot dal tuo cluster con passaggio al server successivo

## Poi
- [ ] WEFAX: ogni 6 mesi rileggere l'elenco NOAA (rfax.pdf) e gli orari DWD e aggiornare `app/src/lib/wefaxStations.ts`
- [x] Contest: Cabrillo dal log, finestra scaduta, date (0.11.308)
- [ ] Contest: `.dom` JIDX (servono i dati veri), schede mancanti, altri contest in cartella; ricontrollare le date delle regole nuove
- [x] Satelliti: preavviso rotore, suoni, schede (0.11.308; spazi vuoti da guardare a schermo)
- [ ] Stazione stretta (0.11.308): guardarla a schermo con radio e dati veri
- [x] Satelliti: parcheggio del rotore a fine passaggio fatto (0.11.309, opzionale: resta, parcheggio, prossimo passaggio)
- [x] Satelliti: intervallo e soglia dell'uplink configurabili (Impostazioni dei satelliti, solo col ripiego a due VFO, sperimentale)
- [ ] IC-820H con SAT acceso: nel pacchetto 7A547090 (AO-7) la lettura `f` del VFO principale salta tra 145,95 e 445,95 MHz e Aurora la prende per cambi di banda (azzera la TX); `F` sul VFO secondario accettato (RPRT 0) ma una volta rifiutato (-9) e il ripiego si spegne; attende il test a mano di Vladimir
- [ ] Cascata: riconoscimento modo, BIGScan regolabile
- [ ] Modi in grigio: Olivia/Contestia (alta: codificatore, trasmettitore e ricevitore in Rust fatti e provati contro fldigi; manca il collegamento al programma: modo nel menu dei modi testuali, finestre RX/TX, trasmissione, Guida, «Informazioni»; poi la prova in onda), MT63, Hell, WEFAX fatto, Packet
- [x] Panic `tao`: prima correzione sulla chiusura (0.11.308)
- [x] Panic `tao` verificato sul pacchetto vero (F014CB6C, FTDX-101MP): i due panic sono di Aurora 0.11.179 (7/11) e 0.11.212 (9/11), nessuno nelle versioni dopo; niente altro da fare salvo nuove segnalazioni
- [x] Olandese completato (da rileggere: «Instellingen › Weergave», «Opdrachten»)
- [ ] Antivirus: invio dei file per i falsi positivi (tuo)

## Scartato
- VARA: a pagamento, non si integra
- [ ] Ricevere i QSO registrati in un altro log (messaggio «QSO Logged» di WSJT-X o ADIF su UDP) e aggiungerli al log di Aurora senza rimandarli fuori (richiesta di Luc, #88; in attesa della sua risposta su quale programma usa).
- [ ] Contest da aggiungere come gli ARRL, in ordine di Luc (#88): ARRL CW/RTTY, CQ WW CW/RTTY (prossimo fine settimana: CQ WW CW, senza promessa), UBA, REF CW, EU DX, WAE RTTY/CW. Log da cui ricevere i QSO: Log4OM, HRD Logbook, OpsLog, N1MM+ (chiesta a Luc la porta UDP/ADIF).
- [ ] Due radio collegate insieme (richiesta di Vladimir, #83): IC-820H per i satelliti su una porta COM e TS-570S per le onde corte su un'altra; oggi Aurora ne usa una per volta e si cambia con i profili (Impostazioni > Radio CAT).

- [ ] Nome «Aurora» e FlexRadio (2026-11-22): FlexRadio chiama AURORA™ la sua serie di ricetrasmettitori (AU-510, AU-520, presentata a Hamvention 2025; i manuali dichiarano il marchio, ma nei registri americani consultabili da siti di terzi non risulta una registrazione a suo nome). Il progetto e' nato il 24/10/2026 (prima versione pubblica il 5/11/2026). Fatto: riga «non collegata a FlexRadio» nei sei README. Da fare: controllare i registri ufficiali (EUIPO eSearch, WIPO Global Brand Database, USPTO TSDR) per «AURORA» nelle classi 9 e 38 e per FlexRadio come titolare; riga analoga in «Informazioni»; eventualmente una consulenza breve di un avvocato di marchi. Il nome non si cambia senza decisione del proprietario.

- [x] Gia' fatti e pubblicati (verificato il 2026-11-23): pulsante «Calibra la scheda audio» nel WEFAX, AFC del WEFAX che corregge la radio (soglia 30 Hz, spenta di base, `afc_radio`), Doppler dei satelliti che segue la manopola (`follow_knob`, 0.11.330). ALC di Pascal: fatta la lettura facoltativa dell'ALC in trasmissione (casella in Impostazioni › Radio, spenta di base; il pannello grande e' stato tolto), in commit locali non pubblicati. Resta: calibrazione con WWV/CHU (serve la scelta della pagina e una registrazione vera di WWV o CHU).

## Richieste da mostrare al proprietario

- BUG da correggere (Pere EA3CUU, #92, 2026-11-25, pacchetto E968DC46, 0.11.348): con la decodifica «Profonda» il periodo si decodifica finito a +1,6 s e la trasmissione parte a +0,8 s, quindi la risposta della controparte (-20 alle 07:54:45) arriva tardi: Aurora ripete il JN12 e risponde R-18 un ciclo dopo. Da studiare: aspettare o recuperare la decodifica tardiva (`engine_native`, tempi di `tx_native::scheduler`); verificare con REA come lo gestisce WSJT-X/JTDX. Risposto a Pere (non corretto, non pubblicato). Gli ho chiesto quale riga vede fuori posto.
- ab7r (Greg), #105 (2026-11-24, 22:06 UTC): RTTY, pulsante TX con la casella vuota: oggi non trasmette (le macro vogliono testo dal decodificatore); come MMTTY vuole che TX metta subito in trasmissione e mandi i diddle (LTRS) finche' non si preme RX. 🟡 da decidere (il diddle in attesa di testo c'e' gia' in `tx_stream.rs`, manca l'avvio senza testo).
- ab7r (Greg), #106: CWX del Flex via TCI: DECISO e FATTO in locale (2026-11-25), uscita CW «TCI (CWX della radio)», non pubblicata e non provata con radio vera.
- #105 TX a casella vuota con diddle: DECISO dal proprietario (2026-11-25): opzione «Diddle in attesa» scegliibile, FATTA in locale, non pubblicata. NOTA: leggendo il codice, il tasto RX/TX in basso (e Pausa) con la casella vuota apre gia' la trasmissione e manda i diddle (`DigiService::tx_open("")`, `idle()` RTTY = LTRS). La risposta data a Greg («non parte») e' probabilmente sbagliata: in attesa di decidere se correggerla o aggiungere comunque un'opzione.
- ab7r (Greg), #105: un log completo che segua i diplomi (DXCC, WAS...) al posto di OpsLog/SliceMaster. 🟡 grande, non previsto; per ora usa il server TCP verso il suo log.
- ab7r (Greg), #105 (2026-11-24, 21:30-21:46 UTC): vuole che il clic su uno spot di qualunque modo apra la pagina di quel modo (CW nella pagina PSK/RTTY/SSTV con il modo CW scelto) e metta la radio in quel modo. Oggi: FT4/FT8 cambiano il MODO nella stessa pagina, RTTY/PSK aprono la pagina dei modi testuali, CW e SSB spostano solo la frequenza (il CW resta li' per lo Split DX del doppio clic sulle DXpedition). DECISO dal proprietario (2026-11-24): si fa come scelta selezionabile, spenta di base. FATTO in locale (chip «Apri la pagina del modo» nel pannello DX Cluster), non pubblicato.
- ab7r (Greg), #104: DECISO dal proprietario (2026-11-24): non si fa. Aurora non legge il log SQLite di altri programmi; l'invio live dei QSO (UDP/ADIF) c'e' gia'. Nessun lavoro da fare.
- ab7r (Greg), #105: DECISO dal proprietario (2026-11-24): niente scrittura automatica del modo al clic su uno spot. Il modo della radio lo sceglie l'utente in «Modo scritto nella radio» (Nessuno/USB/Dati) e Aurora scrive solo quello; mai USB-D di default. Nessun lavoro da fare.
- Casella «Mostra gli spot del mio nominativo» nelle Impostazioni: DECISO dal proprietario (2026-11-24): non si fa, basta il pulsante «I miei spot» del pannello DX Cluster.
- Pere EA3CUU, #92: ordine righe Tx corretto nella 0.11.345 (ora dal motore, in attesa della sua prova); richiesta «perche' smette di trasmettere»: FATTA in locale (avviso `TxStopped`, 2026-11-24), non pubblicata.
- Luc (ic706mk2), #102: colonne SSTV in ricezione gia' pubblicate nella 0.11.345 (informazioni a sinistra, immagine grande al centro); in attesa della sua conferma.
- Filtro skimmer in blocco: FATTO in locale (2026-11-24, «Senza skimmer» nel pannello DX Cluster), non ancora pubblicato.
Funzioni nuove chieste da utenti (issue, rapporti): i giri orari non le implementano, le annotano qui; il proprietario le vuole vedere appena scrive. Formato: utente, issue, cosa chiede, idea in una riga.
- FATTO in locale (non pubblicato): PTT dati per Hamlib (`ptt_data`), confermato con Ghidra su Hamlib 4.7.2; resta la prova di pd0jor (#96) con la radio vera.
- FATTO in locale (non pubblicato; sottomenu «Ingresso audio» nel menu Configura dei modi testuali, scelto dal proprietario al posto della barra principale) - Luc (ic706mk2), #95 (2026-11-23, 19:07 UTC): un menu a tendina nella pagina principale per cambiare in fretta l'ingresso audio (per esempio tra l'audio della radio e un ricevitore SDR online con la linea virtuale), senza passare dalle Impostazioni. Idea: un selettore dell'ingresso audio nella barra in basso accanto al livello, con l'elenco dei dispositivi rilevati come nelle Impostazioni › Audio. Lo screenshot di Luc mostra la tendina dell'ingresso nelle Impostazioni › Audio (in olandese): «Systeemstandaard», «Line 2 (Virtual Audio Cable)», «Microfoon (USB Audio CODEC)», «Mic 1 / Line 1 (Virtual Audio Cable)»: vuole quella stessa lista nella pagina principale.
- fetuso, #94 (2026-11-23, 16:41 UTC): vuole cambiare i colori della cascata in CW per far risaltare il segnale ricevuto; ha trovato solo quelli dell'FT8. Oggi la cascata della pagina dei modi testuali (`DigiWaterfall.tsx`, palette fissa in `paletteColor`) non ha palette, guadagno ne' zero; la barra con palette/guadagno/zero (`WaterfallToolbar.tsx`) e' solo dell'FT8. Idea: stessa barra (o almeno palette e contrasto) anche nella cascata di CW, PSK e RTTY.
- Luc (ic706mk2), #93 (2026-11-23, 15:40 UTC): SSTV. (1) Il visore della ricezione e' troppo grande rispetto a MMSSTV (immagine 320x256 ingrandita, i disturbi sembrano piu' grossi): vuole la dimensione vera o un visore piu' piccolo. Idea: opzione «Dimensione 1:1» nel visore SSTV. (2) Ricezione in parallelo con MMSSTV: Aurora ha perso il segnale due volte e MMSSTV ha completato l'immagine (PROBLEMA da verificare: chiesti modo SSTV, versione e una registrazione WAV; screenshot non scaricato).
- Luc (ic706mk2), #88 (2026-11-22, 15:43 UTC): nella pagina Log (Esporta) all'apertura tutti i QSO sono selezionati e ci sono «Elimina selezionati» e «Svuota log»: lo trova pericoloso («come caricare una pistola e togliere la sicura»). Chiede di invertire: all'apertura NESSUN QSO selezionato, si sceglie per agire. Oggi la cancellazione chiede una conferma con il numero di QSO. Idea: partire senza selezione con «Seleziona tutti» a un clic per l'esportazione, oppure lasciare tutto selezionato ma con la conferma piu' forte per le cancellazioni grandi (scrivere il numero o «CANCELLA»). Da decidere: cambia il comportamento di base (l'esportazione dell'intero log oggi e' a un clic).
- Proprietario (2026-11-22), WEFAX: calibrazione della scheda audio come in SSTV (pulsante «Calibra la scheda audio» nelle impostazioni SSTV, `sstv.cal.*`): oggi il WEFAX ricava i ppm solo dal contenuto della cartina (inclinazione automatica e «ppm appreso»). Idea: stesso pulsante nelle impostazioni WEFAX, che porta nel campo dell'inclinazione i ppm misurati su una cartina ben ricevuta da una stazione di riferimento (DDK3).
- Proprietario (2026-11-22), tutti i modi: schermata «Calibra la scheda audio con WWV» (o altra emissione campione con il tic al secondo: WWV 5/10/15 MHz, CHU 3330/7850/14670 kHz): si ascolta il tic, si contano i campioni tra tic successivi per 1-2 minuti (non dipende dall'errore del quarzo della radio) e si ricava l'errore in ppm della scheda audio, che passa nel campo dell'inclinazione di SSTV e WEFAX. Oggi esiste solo il pulsante di SSTV che misura i ppm su un'immagine ricevuta bene.
- GIA' FATTO (follow_knob, 0.11.330, verificato 2026-11-23): Vladimir (un7en51-dev), #83 (chiusa, domanda dopo la chiusura): su RS-44 in SSB con SAB gira la manopola della radio per trovare una stazione, ma al passo seguente del Doppler Aurora riscrive la frequenza (quella scelta nel cursore «Downlink» della pagina Satelliti, `SatSettings.down_hz`) e la radio torna dov'era. Vuole che sintonizzando sulla stazione la frequenza di trasmissione la segua e il Doppler continui da li'. Idea: opzione «segui la manopola»: se la frequenza di ricezione letta dalla radio si scosta da quella scritta di piu' di una soglia (per esempio 50 Hz) fra due passi, si prende come nuovo `down_hz` senza riscriverla, e la trasmissione la segue con lo stesso scarto.
- Pascal F4LHL (email al proprietario, 2026-11-22): FATTI nei commit locali (non pubblicati): dB sul cursore «Lvl TX» e nel suggerimento (punto 1), livello di ricezione scritto in dB sotto la barra (punto 3), casella «Non toccare i volumi di Windows» (punto 4), casella «Ricorda la potenza per banda e modo» (controllo `pwr_band_memory`). RESTANO: (a) FATTO: livello separato per il Tune (casella nelle impostazioni audio); (b) finestra della frequenza piu' grande con potenza in trasmissione, ALC e SWR in grande (la sua immagine «Interface radio AURORA avec barre...» non l'ho: serve); (c) punto 5, ADIF con rapporti e distanza: esiste gia' (commento «FT8 Sent: -07 Rcvd: -12», e con «Distanza nei commenti» anche `Distance: 6656 km`), basta dirglielo (bozza di risposta in francese data al proprietario).
- Proprietario (2026-11-22), WEFAX: con il CAT collegato l'AFC potrebbe correggere anche la frequenza della radio (oggi sposta solo la sottoportante del decodificatore, +36 Hz non toccano il VFO). Idea: opzione «AFC corregge la radio» che sposta il VFO a passi di 10 Hz oltre una soglia, spenta di base; da decidere perche' cambia la frequenza registrata e il QSY a mano.
