# Changelog

## Non rilasciato
- Menu in alto e schede delle Impostazioni: la voce in cui ci si trova e' piena e in grassetto (anche `aria-current`).
- Tema scuro: pannelli con fondo pieno, senza vetro sfocato dietro al testo, e tabelle del cluster e del log a corpo intero: i nominativi non risultano piu' sfuocati.

## 0.9.0 - 2026-10-24
- **DX Cluster** (crate `dx-cluster`): client telnet verso server pubblici (catalogo verificato) o propri, con accesso col nominativo, riconnessione, paese dal `cty.dat`, filtri della vista e comandi solo di consultazione (`sh/`, `set/`...): mai spot o messaggi pubblici. Finestra a sinistra di Comandi (DE, Freq, DX, bandiera, Paese, Modo, Commenti, UTC), doppio clic = la radio va sulla frequenza esatta dello spot (`POST /commands/frequency`, solo dentro le bande dei radioamatori, mai in trasmissione). Scheda DX Cluster nelle impostazioni con LED di raggiungibilita' dei server (`POST /dx-cluster/test`), salvataggio automatico e spento di default.
- **Client NTP** (crate `clock-sync`): scarto dell'orologio dai server NTP, controllo periodico, correzione dell'ora di Windows (amministratore, solo rete di casa), indicatore «Orologio» e DT mediano (il «Med» di JTDX) nella barra di stato, scheda Orologio.
- **Impostazioni di JTDX** lette dal suo `JTDX.ini` (`GET /engine-config`, `AURORA_JTDX_INI`, senza password): UDP, radio, audio, sequenza, avvisi, avanzate. Si modificano dall'interfaccia (`POST /engine-config`: chiude JTDX, scrive il file con copia di sicurezza, lo riavvia; mai in trasmissione, solo rete di casa) con una barra «Applica e riavvia» / «Annulla» e le voci cambiate evidenziate.
- **Log di Log4OM2** in sola lettura (`GET /log4om`, `AURORA_LOG4OM_DB`): finestra Log accanto all'attivita' di banda, numero di QSO scelto in Impostazioni.
- Preferenze dell'interfaccia salvate sul server (`data/ui-settings.json`, separate per PC e telefono). Le schede Cluster e Orologio si salvano da sole.
- **Radio**: cambiando banda lo split della FT-2000 si spegne (OmniRig `SetSimplexMode` lo lasciava acceso); il cursore Pwr si regola anche a trasmissione chiusa, in basso la potenza massima come in JTDX; modo JT accanto a FT8 e FT4; comandi di ricezione di JTDX (Hound, Modo SWL, Monitora, Sinc, AGCc, Filtro, Ferma, Decodifica, Cancella) e barra del periodo.
- Doppio clic: la stazione scelta compare subito e resta finche' JTDX non la conferma; avviso se JTDX accetta i Reply solo per CQ e 73. Doppio Esc: libera la stazione da chiamare (Canc DX). Aprendo Aurora non si rivedono piu' le stazioni di prima (`AURORA_REPLAY_ON_CONNECT` ora spento).
- Interfaccia: Attivita' di banda a colonne con griglia su un rigo solo, barra di stato essenziale, modi e bande sulla prima riga, contrasti dei temi chiaro e scuro, frequenze scritte `21.074,0`, piede di pagina sottile.
- L'interfaccia non nomina piu' JTDX: nei testi (sei lingue) e nel codice dell'app si parla di Aurora e di «motore»; la rotta delle impostazioni del motore e' ora `/engine-config` (era `/jtdx-config`).
- DX Cluster: i filtri Banda e Modo sono a scelta multipla (menu a caselle leggibile in entrambi i temi, anche «La mia banda»); le scelte si ricordano.
- Tema scuro, Stazione DX: righe Tx1-Tx6 neutre con una sola tinta per la riga in corso, testo piu' nitido.
- Cambio banda: OmniRig non rende scrivibile `Freq` sulla FT-2000 (il comando pareva riuscito ma la radio restava ferma): ora si scrive `FreqA` e si attende la frequenza nuova.
- Lo script `avvia-sviluppo.ps1` apre la trasmissione (con `-Chiusa` resta chiusa) e collega JTDX.ini, il log di Log4OM2 e OmniRig. Avvio pulito: chiude le copie dell'avvio precedente, apre una sola finestra di Windows Terminal e tiene una sola copia di JTDX; il server di sviluppo (5173) parte solo con `-Sviluppo`.
- Cambio banda: aspetta fino a circa 3 secondi che OmniRig sia in linea e ritenta una volta. Cluster: frequenza senza decimale. Attivita' di banda: distanza senza parentesi quadre. Valori di `AcceptUDPReplyMessages` corretti (1 = solo CQ, 2 = CQ e 73, 3 = qualsiasi). L'avviso «buffer audio perso» compare una volta a ogni 10, 100, 1000 occorrenze.

## 0.8.1 - 2026-10-24
- Tasto Esc: ferma la trasmissione e spegne AutoTX e Abilita Tx, in tutte le pagine; si spegne in Impostazioni → Generali.
- Il doppio clic su un messaggio accende anche Abilita Tx (con la trasmissione aperta): con «1 QSO» JTDX lo spegne a ogni QSO finito e non richiamava piu' la stazione scelta.
- Blocco DX della scheda Trasmissione piu' stretto.
- Cursore Pwr verticale reso uguale in ogni browser (un cursore orizzontale ruotato): su alcuni browser il verso era invertito e portava il livello al minimo invece che al massimo. In alto e' sempre il 100%.

## 0.8.0 - 2026-10-24
- Pannello Comandi come JTDX: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX e Abilita Tx; frequenze Tx/Rx con le frecce di copia e Tx/Rx Split; Canc DX e Gen.Msgs; CQ e messaggio libero. Nuove rotte `GET /controls` e `POST /commands/control` (elenco chiuso: niente accordo ne' log; Abilita Tx e AutoTX si accendono solo con la trasmissione aperta, spegnerli e' sempre permesso).
- Scheda Trasmissione in due blocchi (stazione DX con Tx1-Tx6, e interruttori del QSO) con il livello Pwr verticale a destra.
- Doppio clic su un messaggio: la stazione compare subito nel blocco DX e JTDX la chiama al momento giusto; l'opzione si spegne in Impostazioni → Generali.
- Corretto il cambio modo FT8/FT4: prima accendeva solo la casella senza far cambiare modo a JTDX; ora preme il pulsante e resta nella banda di prima, sulla frequenza che la tabella indica per il nuovo modo.
- Cambio banda e modo si vedono subito nella pagina, senza aspettare il prossimo stato di JTDX.
- STOP TX piu' piccolo e discreto a riposo, pieno in trasmissione; l'esito dei comandi e' una nota che sparisce da sola e non occupa spazio.
- Pulsanti di modo e di banda a misura fissa; FT8 viola e FT4 ambra, anche da spenti.

## 0.7.0 - 2026-10-24
- Guida in-app (`#/guida`): 11 argomenti brevi nelle sei lingue, raggiungibile dalla testata e dalla scheda Avanzate.
- Pagina Statistiche (`#/statistiche`): premi DXCC, zone CQ e ITU, quadranti, prefissi, QSO per banda e paesi mancanti, dal log ADIF.
- Elenco messaggi: i propri messaggi trasmessi compaiono come righe `Tx` (una per trasmissione), in ordine d'orario, senza risposta al doppio clic.

## 0.6.0 - 2026-10-24
- Sei lingue (italiano, inglese, tedesco, francese, spagnolo, portoghese) con selettore nella testata e un tooltip su ogni controllo; i testi stanno in tabelle per area (`app/src/i18n/strings`) e i test impediscono chiavi mancanti o segnaposto diversi.
- Impostazioni a 13 schede come JTDX: generali, radio, audio, sequenza, macro TX, segnalazioni, frequenze, notifiche, filtri, programmazione, avanzate, accesso, finestre, bande.
- Filtri dei messaggi (continenti, paesi, nominativi, solo CQ/RRR/RR73/73, non standard, proprio continente), preferenze di visualizzazione (miglia, paese, bandiera, distanza, DT), flag evidenzia/suono per ogni tipo di messaggio, macro TX (@ DX, & io, # rapporto).
- Frequenze di lavoro per banda e modo (FT8/FT4) modificabili e salvate; il cambio banda le usa. Programmazione delle bande per orario UTC (fino a 5 righe).
- Accesso: durata del codice di associazione decisa dall'operatore (1-60 minuti, predefinita 10, solo dal PC), accesso senza codice per le prove (solo rete di casa, mai da proxy, mai al riavvio, `AURORA_OPEN_ACCESS`), elenco e revoca dei dispositivi, codice con QR dalle impostazioni.
- Stazione: bandiere dei paesi, colonne a larghezza fissa che si adattano al pannello, etichette corte con il nome intero nel tooltip, stazione chiamata in evidenza, chiamate verso di te in rilievo con tacca rossa e avviso sonoro dedicato, doppio clic per rispondere (un clic seleziona soltanto), fase del QSO nel pannello Tx, DX e messaggio in trasmissione.
- Cascata ridimensionabile a mano anche da telefono; il trascinamento dei pannelli non li fa piu' sparire.
- Livello audio di uscita (cursore Pwr di JTDX) con il suggerimento sull'ALC: `GET/PUT /api/v1/output-level`, la modifica richiede la trasmissione aperta.
- Continente in ogni messaggio e nello stato; il simulatore e i test non contengono nominativi reali.
- Corretto: con l'accesso senza codice il flusso `/stream` veniva chiuso ogni secondo (il dispositivo virtuale non e' nell'archivio), cosi' sul telefono comparivano "Scollegato", lampeggio e nessun dato. Ora il flusso resta aperto finche' l'accesso senza codice e' acceso.

## 0.5.0 - 2026-10-24

## 0.5.0 - 2026-10-24
- `qso-engine`: automazione dei QSO (risponde ai CQ con novita'). Parte spenta e in prova a vuoto; un solo QSO attivo, tentativi per stazione, tetto di risposte di fila (si spegne), tetto giornaliero, dead-man senza dispositivi collegati, STOP TX la spegne. Registro delle decisioni con il motivo. API `GET/PUT /api/v1/automation`: dal telefono solo prova a vuoto o spegnimento, il modo reale solo dal PC con l'avviso confermato e `AURORA_ALLOW_TX=1`.
- Il motore JTDX annuncia `TxHalted` a ogni STOP TX (anche prima di conoscere JTDX).
- `qso-export`: ogni QSO registrato va nel log di Aurora e, se configurato, in ADIF su UDP (`AURORA_EXPORT_UDP`) o su file (`AURORA_EXPORT_FILE`).
- `GET /api/v1/station-info`: come e' avviata la stazione (sola lettura).
- Interfaccia: finestre (cascata, elenco, stazione, CQ) trascinabili in qualunque colonna o in basso e ridimensionabili, divisore tra colonne; disposizione ricordata. Elenco delle stazioni a sinistra come in JTDX (UTC, dB, DT, frequenza, messaggio, paese e distanza in km, righe colorate per tipo). Barra delle bande (solo visualizzazione). Aspetto di vetro traslucido con sfondo luminoso.
- Impostazioni a schede: Colori (le 17 categorie di JTDX piu' la risposta automatica, valori di partenza del tema scuro di JTDX), Finestre, Avvisi sonori, Trasmissione, Log esterni, Collegamenti. Pagina Automazioni.
- Comandi di trasmissione: CQ (TriggerCQ di JTDX, provato dal vivo: il bool a `true` abilita Tx), messaggio libero (13 caratteri; con invio solo a trasmissione aperta) e frequenza audio di trasmissione. API `commands/cq`, `commands/free-text`, `commands/tx-frequency`; cio' che trasmette richiede `AURORA_ALLOW_TX=1` e una via consentita. Pannello Trasmissione nell'interfaccia.
- Stato: stazione DX, rapporto, frequenze Tx/Rx e messaggio in trasmissione da JTDX; pannello con Tx1-Tx6 previsti (si accende quello in uso).
- Etichette complete come i colori di JTDX: zona CQ e ITU (anche per banda), griglia per banda, prefisso (WPX), nuova stazione, gia' lavorata.
- Finestre comprimibili con doppio clic sul titolo (la cascata si riduce alla sola barra).
- L'interfaccia compilata si serve da Aurora (`AURORA_UI_DIR`) anche sulla rete locale, per l'uso da smartphone.
- Cambio banda: `rig-control` comanda la radio via OmniRig (condiviso con JTDX, che lo usa come CAT), `POST /api/v1/commands/band` con le frequenze FT8 predefinite, mai in trasmissione; barra delle bande cliccabile. Si attiva con `AURORA_RIG=omnirig`. Provato dal vivo (15 m -> 20 m -> 15 m).
- Cambio modo FT8/FT4 (si preme il pulsante di JTDX con l'accessibilita' di Windows, `POST commands/mode`), frequenza Rx impostabile (`commands/rx-frequency`).
- Spettro con i colori di JTDX (blu, giallo, bianco) e le due barre verde (Rx) e rossa (Tx) trascinabili; clic sposta Rx, Maiusc+clic sposta Tx.
- Linguetta piccola a destra: rosso lampeggiante = trasmissione in onda, verde = ascolto. Barra delle bande filtrabile dalle impostazioni, banda in uso ben evidente.
- Statistiche del log (premi DXCC, zone CQ e ITU, griglie, prefissi, paesi mancanti), regole dell'automazione salvate.
- Simulatore e test senza nominativi reali.
- Test dell'interfaccia: cartelle temporanee uniche nei test e2e.

## 0.4.0 - 2026-10-24
- Cascata: contrasto automatico che segue il rumore di fondo.
- `dxcc`: paese e zone dei nominativi da `cty.dat` (prefissi piu' lunghi, eccezioni `=CALL`, `/P` e simili, `/MM` senza paese).
- Novita' sui decode: paese di chi trasmette e etichette NUOVO DXCC, DXCC NUOVA BANDA, NUOVA ZONA, NUOVA GRIGLIA, calcolate dai QSO gia' fatti (log ADIF di JTDX) e aggiornate a ogni QSO chiuso. Senza log o senza banda nota restano solo le etichette CQ.
- Nuove variabili: `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`; file mancanti non fermano Aurora.
- Interfaccia: pannello "CQ e novita'", paese sotto ogni messaggio, avvisi sonori (spenti all'inizio, suono diverso per nuovo paese e altre novita', volume ricordato).

## 0.3.0 - 2026-10-24
- Collegamento reale a JTDX 2.2.159: `udp-hub` (ripetitore del flusso UDP, inoltro identico a GridTracker2 e comandi di ritorno), `engine-jtdx` (Status, Decode e QSO negli eventi di Aurora; Reply e HaltTx verso JTDX).
- **Trasmissione chiusa per impostazione predefinita** (`AURORA_ALLOW_TX=0`): un Reply fa trasmettere JTDX davvero. HaltTx e STOP TX funzionano sempre.
- `audio-spectrum`: cascata dall'ingresso audio (`AURORA_AUDIO_DEVICE`).
- `udp-bridge`: Heartbeat di JTDX senza revisione, comando Replay; test con pacchetti reali anonimizzati.
- Documentazione: mappa dei flussi UDP (JTDX, GridTracker2, Log4OM2), guida al collegamento, esiti del reverse con REA e Ghidra.

## 0.2.0 - 2026-10-24
- Core: `aurora-core` (tipi, bus di eventi, trait `Engine`), `engine-sim` (stazione simulata), `logbook` (SQLite, duplicati, ADIF), `auth` (associazione dispositivi con token a hash e blocco anti-forza-bruta), `remote-api` (API v1, WebSocket, OpenAPI), programma `aurora`.
- Interfaccia: React 19 + TypeScript 7 + Vite 8, tema Notte radio, cascata WebGPU con ripiego WebGL2, pagina Live, associazione del dispositivo, installazione come app (PWA).
- Script `avvia-sviluppo.ps1` e `associa-telefono.ps1`.
- Reverse di `jtdx.exe` con REA + Ghidra: conferme su server UDP e Reply (vedi `docs/fase0-rapporto.md`).

## 0.0.2 - 2026-10-24
- `udp-bridge`: decodifica e codifica del protocollo UDP di JTDX.
- Strumenti di prova `udp_capture`, `udp_send`, `audio_tone`.
- Bozza dell'inventario delle funzioni di JTDX e guida alle prove manuali.

## 0.0.1 - 2026-10-24
- Struttura iniziale del repository e specifica di design.
