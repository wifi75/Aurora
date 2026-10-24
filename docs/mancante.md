# Cosa manca ad Aurora (elenco ordinato)

Stato al 2026-10-24, versione 0.4.0. Priorità: **A** indispensabile per sostituire JTDX nell'uso quotidiano, **M** utile, **B** rinviabile.
Fonti: specifica (`2026-10-24-aurora-design.md`), inventario di parità (`fase0-parita-funzioni.md`), rapporti dei piani.

## Già fatto

Core Rust (bus, hub UDP davanti a GridTracker2, motore JTDX, log SQLite con ADIF, associazione dispositivi, API e WebSocket), interfaccia a finestra larga e da telefono (cascata WebGPU/WebGL2, righello, pannelli, STOP TX, tema scuro/chiaro, associazione da link), cascata dall'audio (H2 confermata), trasmissione chiusa di default, contrasto automatico, paese e novita' dei decode (DXCC, banda, zona, griglia), avvisi sonori.

## Da fare, in ordine di lavoro

| # | Voce | Pri. | Note |
|---|---|---|---|
| 1 ✔ | **Cascata: contrasto automatico** | A | Il rumore di fondo reale e' alto (media 177/255): serve un livello di nero che segua il rumore, altrimenti la cascata e' tutta luminosa |
| 2 ✔ | **Dati DXCC**: paese/zone dei nominativi da `cty.dat` | A | Base per tutte le etichette e gli avvisi |
| 3 ✔ | **Lavorati**: DXCC, zone e griglie gia' collegati, dall'ADIF di JTDX (`wsjtx_log.adi`) | A | Letto all'avvio; si aggiorna a ogni QSO chiuso |
| 4 ✔ | **Etichette reali** nei decode: nuovo DXCC, nuova zona, nuova griglia, gia' lavorato per banda | A | Colori come in JTDX (CQ, DXCC, griglia, zona CQ/ITU, "sulla banda") |
| 5 ✔ | **Avvisi sonori** per le novità | A | Suoni generati dal browser, per categoria, con interruttore e volume |
| 6 | **Motore delle automazioni** (`qso-engine`): regole, prova a vuoto, tetto di cicli, dead-man switch, registro delle decisioni | A | Salvaguardie già decise in specifica §6; un Reply fa trasmettere |
| 7 | Pagina **Automazioni** nell'interfaccia | A | Regole, interruttori, cronologia con il motivo di ogni decisione |
| 8 | **Comando della radio** via rigctld (frequenza, banda, PTT) | A | H3 da verificare; JTDX potrebbe usare Hamlib NET rigctl |
| 9 | **TriggerCQ** e **SetTxDeltaFreq** (tipi 50 e 51) | M | Significato dei bool di `trigger_CQ` da confermare |
| 10 | **Trasporti sicuri**: HTTPS locale con certificato fissato dall'associazione, Tailscale, Cloudflare con Access | A | Oggi l'accesso da telefono e' HTTP sulla rete locale |
| 11 | **Programma Windows** (guscio Tauri 2) con avvio automatico di Aurora | A | Oggi l'interfaccia e' una pagina web |
| 12 | **Pagina Impostazioni** (dispositivi associati, revoca, trasmissione, audio, inoltro) | M | Oggi si configura da variabili d'ambiente |
| 13 | **Log e statistiche** di Aurora (da QsoLogged), premi DXCC/WAZ/griglie con avanzamento | M | Il log ufficiale resta Log4OM2 |
| 14 | **Wanted list** e avvisi push sul telefono (PWA) | M | |
| 15 | **Mappa** dei QSO e dei CQ con terminatore | M | Servono dati geografici offline |
| 16 | **Lingue** (italiano, inglese) | M | |
| 17 | **Modo SWL, FT4, JT65/JT9**: visualizzazione e comandi per modo | M | Oggi si legge il modo da JTDX |
| 18 | Riproduzione dei cicli decodificati, statistiche di propagazione | B | |
| 19 | Tipo `HighlightCallsign` (evidenziare i lavorati dentro JTDX) | B | |
| 20 | Il bool finale dello `Status` di JTDX: capire che cosa e' | B | |

## Limiti dichiarati (non risolvibili dall'esterno)

- JTDX 159 non accetta `Configure` via UDP: livello di decodifica, cicli FT8, filtri RX e altre impostazioni del motore restano nella finestra di JTDX.
- Un Reply fa trasmettere JTDX; i comandi di GridTracker2 passano dall'hub senza il cancello di Aurora.
