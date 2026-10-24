# Changelog

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
