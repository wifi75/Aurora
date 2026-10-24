# Piano 3 — Rapporto di esecuzione (collegamento reale a JTDX)

Data: 2026-10-24 · Versione: 0.3.0 · Stato: **codice completo e verificato dai test automatici; il collegamento con JTDX reale lo prova Tiziano**

## Esito automatico

| Verifica | Esito |
|---|---|
| `cargo test --workspace` | 143 passati, 0 falliti, 0 ignorati |
| `cargo clippy --workspace --all-targets` | nessun avviso |
| `npm test` (interfaccia) | 59 passati in 13 file |
| `npm run build` | riuscita |

Nuovi crate: `udp-hub` (9 test), `engine-jtdx` (22: 10 unitari e 12 d'integrazione con un JTDX finto che riproduce pacchetti reali), `audio-spectrum` (8). Il binario `aurora` ha 14 test end-to-end, di cui 7 nuovi con JTDX finto, inoltro a un "GridTracker" finto e comandi di ritorno.

I cinque punti del Review Focus hanno un test ciascuno: datagrammi estranei né inoltrati né causa di errori; Reply bloccato con `allow_tx=false` e HaltTx sempre consegnato; JTDX riavviato con un'altra porta; destinazione di inoltro chiusa che non ferma le altre; decode con spazi finali ripulito per l'interfaccia ma rimandato intatto nel Reply.

## Decisioni prese durante l'esecuzione

- **Due cancelli indipendenti per la trasmissione:** l'API (`TxPolicy::live_tx`, risposta `403 tx_disabled`) e il motore (`JtdxConfig::allow_tx`). Entrambi chiusi per impostazione predefinita; con il simulatore l'API resta aperta perché non c'è radio.
- **`Hub::start` è asincrono** (il piano lo dava sincrono): il bind UDP di Tokio lo richiede.
- **Replay all'avvio**: il motore chiede a JTDX di rimandare i decode correnti la prima volta che lo vede (e quando si riavvia). I programmi a valle li ricevono di nuovo, marcati "non nuovi"; è disattivabile con `AURORA_REPLAY_ON_CONNECT=0`. I Replay già noti non vengono mostrati due volte.
- **Il motore si iscrive all'hub nel costruttore**, non in `run`: nessun datagramma si perde tra la creazione e l'avvio.
- **Cascata dall'audio**: se l'ingresso non c'è Aurora parte lo stesso, con un avviso chiaro e l'elenco dei dispositivi.
- **Messaggio d'errore nell'interfaccia** distinto per «trasmissione disattivata in Aurora» (test scritto prima).
- **Fixture**: i pacchetti reali sono anonimizzati (nominativi e locatori di fantasia, stessa lunghezza).
- La revisione finale del branch l'ho fatta io, non un revisore indipendente.

## Limiti noti

- **I comandi di GridTracker2 attraversano l'hub senza passare dal cancello di Aurora**, come già oggi: un doppio clic in GridTracker2 fa trasmettere JTDX anche con `AURORA_ALLOW_TX=0`.
- **L'hub si fida di chiunque scriva sulla sua porta** (resta su `127.0.0.1`).
- **Non provato con JTDX, GridTracker2 e Log4OM2 reali insieme.** Va fatto da Tiziano seguendo `docs/guida-collegamento-jtdx.md`.
- **L'audio in parallelo a JTDX (H2) non è verificato:** la cattura compila ed è coperta dai test dell'analizzatore, ma la prova con la scheda USB reale è manuale.
- Restano fuori: comando della radio (frequenza e PTT, H3), `TriggerCQ` e `SetTxDeltaFreq` (tipi 50 e 51, significato dei bool non confermato), automazioni dei QSO, HTTPS/Tailscale/Cloudflare, copia dei QSO in un log proprio.
- Il significato del bool finale dello `Status` di JTDX resta da identificare (vale 1 in 28 pacchetti su 29).

## Verifiche manuali per Tiziano

Seguire `docs/guida-collegamento-jtdx.md` in ordine:
1. Avvio di Aurora con `AURORA_ENGINE=jtdx`, inoltro a `127.0.0.1:2333`.
2. In JTDX cambiare **solo** la porta del server primario da 2333 a 2334.
3. Controllare: decode in Aurora, **decode ancora in GridTracker2**, QSO ancora in Log4OM2 alla chiusura del contatto.
4. Toccare un decode: deve comparire «La trasmissione è disattivata in Aurora»; STOP TX deve funzionare.
5. (Facoltativo, con carico fittizio) `AURORA_ALLOW_TX=1` e risposta a un CQ.
6. (Facoltativo) `AURORA_AUDIO_DEVICE='USB Audio'`: la cascata deve scorrere con i segnali veri mentre JTDX decodifica.

Esiti: _da compilare dopo le prove_.

## Prossimi passi

- Piano 4: comando della radio (frequenza e PTT via rigctld, H3), `TriggerCQ`/`SetTxDeltaFreq`, motore delle automazioni con prova a vuoto, tetto di cicli e dead-man switch.
- Poi trasporti (HTTPS locale, Tailscale, Cloudflare) e il tag `v0.3.0` dopo le tue verifiche.
