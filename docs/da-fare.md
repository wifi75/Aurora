# Da fare (breve)

Si aggiorna a ogni chiusura: `[x]` = fatto. Elenco lungo e tecnico: `docs/mancante.md`.

## Adesso
- [x] 0.11.315 pubblicata (rotore fuori dalla barra, «Mostra in home» nelle Impostazioni): da guardare a schermo
- [ ] Controllo licenze fatto il 2026-11-20 (sola lettura): crediti, `LICENSE`, licenze di Hamlib e attribuzioni delle mappe a posto; corretto `qso-native` (intestazione) e `THIRD-PARTY-NOTICES.md` nelle risorse dell'installatore (esce con la prossima versione); da verificare a mano: condizioni d'uso dei dati (cty.dat, OpenStreetMap)
- [x] 0.11.320 pubblicata (IC-820H sulla banda giusta, licenze nell'installatore) e messaggio `aggiorna-0.11.320` mandato per l'ultima volta (poi niente piu' messaggi di aggiornamento)
- [ ] Messaggio `aggiorna-0.11.320`/`aggiorna-0.11.319`: guardare `messaggi_visti`, `messaggi_clic` e le installazioni su 0.11.319+ (verso le 19-20 UTC) per vedere quanti si allineano
- [ ] Casella «Aggiorna da solo quando Aurora e' ferma» (da preparare, solo commit: scelta da fare, spenta o accesa di fabbrica)
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
- [ ] Modi in grigio: Olivia/Contestia (alta), MT63, Hell, WEFAX fatto, Packet
- [x] Panic `tao`: prima correzione sulla chiusura (0.11.308)
- [x] Panic `tao` verificato sul pacchetto vero (F014CB6C, FTDX-101MP): i due panic sono di Aurora 0.11.179 (7/11) e 0.11.212 (9/11), nessuno nelle versioni dopo; niente altro da fare salvo nuove segnalazioni
- [x] Olandese completato (da rileggere: «Instellingen › Weergave», «Opdrachten»)
- [ ] Antivirus: invio dei file per i falsi positivi (tuo)

## Scartato
- VARA: a pagamento, non si integra
