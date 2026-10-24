# Changelog

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
