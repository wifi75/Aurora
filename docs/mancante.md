# Cosa manca ad Aurora (elenco ordinato)

Stato al 2026-10-24 (aggiornato a fine giornata), versione 0.10.0. Priorità: **A** indispensabile per sostituire JTDX nell'uso quotidiano, **M** utile, **B** rinviabile.
Fonti: specifica (`2026-10-24-aurora-design.md`), inventario di parità (`fase0-parita-funzioni.md`), rapporti dei piani.

## Già fatto

Core Rust (bus, hub UDP davanti a GridTracker2, motore JTDX, log SQLite con ADIF, associazione dispositivi, API e WebSocket); interfaccia a finestra larga e da telefono con finestre trascinabili e ridimensionabili; cascata WebGPU/WebGL2 con barre Rx/Tx; paese con bandiera e novità dei decode (DXCC, zona CQ/ITU, griglia, prefisso, stazione, per banda); avvisi sonori e chiamate verso di te; trasmissione (CQ, testo libero, risposta con doppio clic, STOP TX) chiusa di default; cambio banda, modo FT8/FT4 e frequenza Rx; livello audio di uscita (Pwr); automazioni con prova a vuoto; esportazione dei QSO; impostazioni a 13 schede, sei lingue e tooltip; filtri, macro, frequenze, programmazione; codice di associazione di durata scelta e accesso senza codice per le prove.

## Da fare, in ordine di lavoro

| # | Voce | Pri. | Note |
|---|---|---|---|
| 1 ✔ | **Cascata: contrasto automatico** | A | Il rumore di fondo reale e' alto (media 177/255): serve un livello di nero che segua il rumore, altrimenti la cascata e' tutta luminosa |
| 2 ✔ | **Dati DXCC**: paese/zone dei nominativi da `cty.dat` | A | Base per tutte le etichette e gli avvisi |
| 3 ✔ | **Lavorati**: DXCC, zone e griglie gia' collegati, dall'ADIF di JTDX (`wsjtx_log.adi`) | A | Letto all'avvio; si aggiorna a ogni QSO chiuso |
| 4 ✔ | **Etichette reali** nei decode: nuovo DXCC, nuova zona, nuova griglia, gia' lavorato per banda | A | Colori come in JTDX (CQ, DXCC, griglia, zona CQ/ITU, "sulla banda") |
| 5 ✔ | **Avvisi sonori** per le novità | A | Suoni generati dal browser, per categoria, con interruttore e volume |
| 6 ✔ | **Motore delle automazioni** (`qso-engine`): regole, prova a vuoto, tetto di cicli, dead-man switch, registro delle decisioni | A | Salvaguardie già decise in specifica §6; un Reply fa trasmettere |
| 7 ✔ | Pagina **Automazioni** nell'interfaccia | A | Regole, interruttori, cronologia con il motivo di ogni decisione |
| 8 ✔ | **Comando della radio** via rigctld (frequenza, banda, PTT) | A | H3 da verificare; JTDX potrebbe usare Hamlib NET rigctl |
| 9 ✔ | **TriggerCQ** e **SetTxDeltaFreq** (tipi 50 e 51) | M | Significato dei bool di `trigger_CQ` da confermare |
| 10 | **Trasporti sicuri**: HTTPS locale con certificato fissato dall'associazione, Tailscale, Cloudflare con Access | A | Oggi l'accesso da telefono e' HTTP sulla rete locale |
| 11 ◐ | **Programma Windows** (guscio Tauri 2) con avvio automatico di Aurora, installer e aggiornamento OTA | A | Fatta la base dell'aggiornamento (scarico, firma minisign, `POST /update/install`, pulsante Installa ora). Mancano guscio, vassoio, avvio automatico, installer NSIS e script di rilascio |
| 12 ✔ | **Pagina Impostazioni** a schede come JTDX | M | Le funzioni di JTDX che non si comandano da fuori (decodifica, filtri RX) restano in JTDX e la scheda Avanzate lo spiega |
| 13 ✔ | **Log e statistiche** di Aurora, premi DXCC/WAZ/griglie con avanzamento | M | Pagina Statistiche (0.7.0) e storico dei messaggi ricevuti (pagina Storico). Il log ufficiale resta Log4OM2 |
| 14 ✔ | **Wanted list** e avvisi sul telefono (ntfy) | M | Lista dei desiderati con avvisi in pagina e sul telefono; l'automazione puo' rispondere ai loro CQ |
| 15 ✔ | **Mappa** dei QSO e delle stazioni ricevute | M | Pagina Mappa; manca il terminatore (greyline) |
| 16 ✔ | **Lingue** (italiano, inglese, tedesco, francese, spagnolo, portoghese) | M | Tabelle in `app/src/i18n/strings` |
| 17 ◐ | **Modo SWL, FT4, JT65/JT9**: visualizzazione e comandi per modo | M | FT8/FT4 si cambiano dalla pagina; restano SWL, Hound, JT65/JT9 |
| 18 | Riproduzione dei cicli decodificati, statistiche di propagazione | B | |
| 19 | Tipo `HighlightCallsign` (evidenziare i lavorati dentro JTDX) | B | |
| 20 | Il bool finale dello `Status` di JTDX: capire che cosa e' | B | |

| 21 | ~~Wiki in-app~~ | fatto | Pagina `#/guida`, 11 argomenti in sei lingue (0.7.0) |
| 22 | ~~Pagina Statistiche~~ | fatto | Pagina `#/statistiche` da `GET /stats` (0.7.0) |
| 24 ◐ | **Versione smartphone** | A | Dati mancanti corretti; pagina Stazione rifatta a schede con pulsanti da dito (sotto 640 px). Da rivedere Storico, Propagazione, Mappa, Impostazioni, Automazioni e da provare sul telefono vero |
| 25 | **Priorita' tra piu' CQ** nello stesso ciclo e riapplicazione del modo dopo il cambio banda | M | Dal confronto con Decodium (`confronto-decodium.md`) |
| 23 | ~~Righe Tx nell'elenco~~ | fatto | Un'unica riga per trasmissione, dal messaggio dello stato (0.7.0) |

## Limiti dichiarati (non risolvibili dall'esterno)

- JTDX 159 non accetta `Configure` via UDP: livello di decodifica, cicli FT8, filtri RX e altre impostazioni del motore restano nella finestra di JTDX.
- Un Reply fa trasmettere JTDX; i comandi di GridTracker2 passano dall'hub senza il cancello di Aurora.
