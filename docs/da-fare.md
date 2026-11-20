# Da fare (breve)

Si aggiorna a ogni chiusura: `[x]` = fatto. Elenco lungo e tecnico: `docs/mancante.md`.

## Adesso
- [x] 0.11.309 pubblicata (satelliti, ADIF automatico, conferma di chiamata, ROS 2,0, ora automatica)
- [ ] Russo come ottava lingua (interfaccia, Guida, Novita'), con prova dei testi lunghi per non uscire dallo schermo
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
- [ ] #82 ct1bxx: avviso satelliti su tutte le pagine (da progettare)
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
- [ ] Satelliti: il rotore non parcheggia a fine passaggio (decidere)
- [ ] Cascata: riconoscimento modo, BIGScan regolabile
- [ ] Modi in grigio: Olivia/Contestia (alta), MT63, Hell, WEFAX fatto, Packet
- [x] Panic `tao`: prima correzione sulla chiusura (0.11.308)
- [x] Panic `tao` verificato sul pacchetto vero (F014CB6C, FTDX-101MP): i due panic sono di Aurora 0.11.179 (7/11) e 0.11.212 (9/11), nessuno nelle versioni dopo; niente altro da fare salvo nuove segnalazioni
- [x] Olandese completato (da rileggere: «Instellingen › Weergave», «Opdrachten»)
- [ ] Antivirus: invio dei file per i falsi positivi (tuo)

## Scartato
- VARA: a pagamento, non si integra
