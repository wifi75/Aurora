# TODO

Il dettaglio, con priorità e note, sta in `docs/mancante.md`. Qui solo lo stato a colpo d'occhio (2026-10-24, v0.8.0).

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

## Da fare
- [ ] Versione smartphone: flusso dati corretto (2026-10-24), da riprovare sul telefono vero per layout e lampeggio
- [ ] Leggere il log di Log4OM2 (`Documents\LOG4OM2\IU3CYV.sqlite`, 36484 QSO contro i 2060 del log di JTDX) per novita' e statistiche: da decidere con Tiziano
- [ ] Mappa dei QSO e dei CQ
- [ ] Trasporti sicuri: HTTPS locale, Tailscale, Cloudflare con Access
- [ ] Programma Windows (Tauri 2) con avvio automatico di Aurora
- [ ] Lista dei desiderati e avvisi push sul telefono
- [ ] Licenza del progetto (decisione di Tiziano)
- [ ] Prove manuali ancora aperte: Reply con carico fittizio, H4 (JTDX ridotto a icona), installazione del telefono come PWA
