# API di Aurora (v1)

Tutte le rotte stanno sotto `/api/v1`. La descrizione OpenAPI è servita da `GET /api/v1/openapi.json`.
L'autenticazione usa il cookie `aurora_session` (`HttpOnly; SameSite=Strict`, con `Secure` su HTTPS); sul server resta solo l'hash SHA-256 del token.
Con l'**accesso senza codice** acceso (solo prove) chi è sulla rete di casa e non arriva da un proxy entra senza cookie.

## Accesso

| Rotta | Auth | Descrizione |
|---|---|---|
| `GET /health` | no | `{status, name, version, author}` |
| `POST /pair/start` | solo da loopback | Codice di 8 cifre e QR; vale `code_ttl_secs` (predefinito 600 s). Corpo facoltativo `{"base_url": "http://192.168.1.10:8787"}` |
| `POST /pair/finish` | no | `{code, device_name}` → `204` + cookie. Errori: `400 bad_code`/`bad_name`, `410 expired`, `429 locked` con `Retry-After` |
| `GET /devices` · `DELETE /devices/{id}` | sì | Dispositivi associati; revoca (`204` o `404`) |
| `GET /pairing-settings` | sì | `{code_ttl_secs, min_secs: 60, max_secs: 3600}` |
| `PUT /pairing-settings` | sì, solo dal PC | `{code_ttl_secs}`; `400 bad_ttl`, `403 local_only` |
| `GET /access` | no | `{open}`: accesso senza codice acceso o spento |
| `PUT /access` | solo dal PC | `{open}`; non resta al riavvio (si può avviare acceso con `AURORA_OPEN_ACCESS=1`) |

## Comandi

Quelli che fanno trasmettere richiedono la trasmissione aperta (`403 tx_disabled`) e una via consentita (`403 tx_not_allowed_on_transport`).

| Rotta | Descrizione |
|---|---|
| `POST /commands/halt-tx` | Ferma la trasmissione e spegne l'automazione. Idempotente, **mai bloccato** |
| `POST /commands/reply` | `{decode_id}` → risponde a un decode noto (come il doppio clic in JTDX). `404 unknown_decode` |
| `POST /commands/cq` | `{modifier?}` (`DX`...) → CQ di JTDX. Fa trasmettere |
| `POST /commands/free-text` | `{text, send}` → testo libero (13 caratteri: lettere, cifre, `+-./?`); con `send=true` trasmette. `400 bad_text` |
| `POST /commands/tx-frequency` | `{hz}` 200-3000, non trasmette |
| `POST /commands/rx-frequency` | `{hz}` 100-3000, vale anche in trasmissione |
| `POST /commands/band` | `{band, mode?}` → frequenza dalla tabella. `400 bad_band`, `409 transmitting`, `502 rig_error`, `503 rig_unavailable` |
| `POST /commands/mode` | `{mode}` `FT8`/`FT4`/`JT`. Stessi errori di `band` |
| `POST /commands/frequency` | `{hz}` → porta la radio su una frequenza esatta, solo dentro le bande dei radioamatori. `400 bad_frequency`, `409 transmitting`, `502 rig_error`. Lo split della radio si spegne |
| `GET /online` · `PUT /online` · `GET /online/psk` · `GET /online/solar` · `POST /online/refresh` | Servizi online **spenti finche' non si accendono** (`{psk, solar}`, salvati in `data/online.json`). Con `psk` acceso Aurora chiede ogni 5 minuti a `retrieve.pskreporter.info` chi ha ricevuto il nominativo di JTDX nelle ultime 3 ore (il sito vede nominativo e IP); con `solar` acceso scarica ogni 30 minuti `hamqsl.com/solarxml.php` (il sito vede solo l'IP). `psk` e `solar` rispondono `{enabled, psk|solar: {fetched_ms, ..., error}}`; `refresh` chiede subito, al massimo una volta al minuto per servizio, e dice `asked`. Spegnere un servizio dimentica i dati. Non si invia nulla di cio' che si riceve o si trasmette |
| `POST /update/install` | Installa la versione nuova trovata dal controllo: scarica `*-setup.exe` e `*-setup.exe.sig` dalla release (con il token se il repository e' privato), controlla la firma minisign e avvia l'installazione. Solo da loopback (`403 local_only`), mai in trasmissione (`409 transmitting`), `404 install_unavailable` se il programma non sa installare, `502 install_failed` se lo scaricamento o la firma falliscono. `GET /update` e `POST /update/check` hanno `can_install` |
| `GET /history` · `GET /history/stats` · `DELETE /history` | Storico dei messaggi ricevuti (non il log dei QSO). `GET` accetta `q` (parte di nominativo o testo), `band`, `mode`, `from_ms`, `to_ms`, `limit` (predefinito 100, massimo 500) e `offset`; risponde `{total, rows}` coi piu' recenti per primi, ogni riga `{id, ts_ms, band, mode, dial_hz, freq_hz, snr, dt, text, call, country, continent, tag}`. `stats` da' `{total, oldest_ms, calls}`. `DELETE` svuota tutto e risponde `{removed}`. `404 history_unavailable` se il file non si apre |
| `GET /input-level` · `PUT /input-level` | Volume di registrazione di Windows dell'ingresso audio usato per la cascata (lo stesso di JTDX): `{device, volume_pct 0-100, auto}`. `PUT` accetta `{volume_pct?, auto?}`: un volume scelto a mano spegne AUTO, salvo che lo stesso corpo lo riaccenda; `400 bad_input_level` se fuori scala o vuoto, `404 input_level_unavailable` senza ingresso regolabile, `502 input_level_device` se Windows non risponde. AUTO scende subito se il segnale satura (picco da 0,93 o campioni a fondo scala), sale al massimo del 12 % dopo 3 secondi sotto 0,5, non agisce in trasmissione ne' con ingresso muto |
| `GET /controls` | Stato degli interruttori di JTDX `{controls: {autoseq, single_qso, answer_b4, bypass, skip_tx1, tx_period, split, show_wanted, auto_tx, enable_tx}, tx}`; `tx` = `{messages: [sei testi Tx1-Tx6 come li ha il motore], selected: 1-6}` (il pallino selezionato, cioe' da dove trasmette), oppure `null` se la radio non li sa leggere. Una lettura sola, circa un secondo |
| `POST /commands/tx-message` | `{n}` da 1 a 6: seleziona il messaggio Tx da cui il motore parte (come il pallino), senza accendere ne' avviare la trasmissione; vale anche a trasmissione chiusa. `400 bad_message`, `502 rig_error`, `503 rig_unavailable` |
| `POST /commands/control` | `{id, on}` per un interruttore, `{id}` per un pulsante (`clear_dx`, `gen_msgs`, `rx_to_tx`, `tx_to_rx`). Accendere `enable_tx` o `auto_tx` richiede la trasmissione aperta (`403`); `400 bad_control` |
| `GET/PUT /output-level` | Livello audio di uscita di JTDX (cursore Pwr) come attenuazione `{attenuation_db}` 0-45; si regola anche a trasmissione chiusa (come in JTDX), non da una via non consentita. `400 bad_level` |

## Impostazioni e dati

| Rotta | Descrizione |
|---|---|
| `GET /station-info` | Come è avviata la stazione (sola lettura) |
| `GET/PUT /frequencies` | Frequenze di lavoro `[ {band, mode, hz} ]`; `400 bad_frequencies` |
| `GET/PUT /schedule` | Programmazione delle bande `{enabled, entries:[{hour, minute, band}]}`, al massimo 5 righe; `400 bad_schedule` |
| `GET/PUT /automation` | Stato, regole, limiti e registro; dal telefono solo prova a vuoto o spegnimento. `403 local_only`, `403 tx_disabled`, `400 bad_rules`/`notice_required`. Le regole hanno anche `reply_to_wanted` (predefinito `false`): risponde ai CQ della lista dei desiderati, con le stesse salvaguardie; e `prefer` (`rarity` predefinito o `snr`): con piu' CQ interessanti nello stesso ciclo si risponde a uno solo, la novita' piu' rara o il segnale piu' forte. Prima di rispondere a un CQ l'automazione scarta i falsi decode: messaggio segnato «?» dal motore, nominativo senza la forma di un nominativo, segnale sotto -23 dB di una stazione mai sentita nella sessione |
| `GET /stats` | Premi (DXCC, zone CQ e ITU, griglie, prefissi) e paesi mancanti, dal log ADIF |
| `GET /update`, `PUT /update` | Aggiornamenti: versione in uso, impostazioni `{enabled, repo, interval_hours, prereleases, has_token}` (il token non si restituisce mai; nel PUT `token` assente = si lascia, vuoto = si toglie), ultima release trovata e `available` se piu' nuova; `400 bad_update` |
| `POST /update/check` | Controlla subito sul repository (serve internet); risponde con lo stato, `error` spiega un fallimento |
| `GET /wishlist`, `PUT /wishlist` | Lista dei desiderati `{enabled, calls, prefixes, countries, grids, bands, cooldown_min, from_decodes, from_cluster, notify_url}` e avvisi recenti `{config, recent}`; il PUT valida e normalizza (`400 bad_wishlist`) e vale subito |
| `POST /wishlist/test` | Avviso di prova: compare nella pagina e, con `notify_url`, va al telefono; risponde `{phone: no_url|sent|failed}` |
| `GET /map/worked` | Quadrati locatore gia' lavorati per la mappa: `{squares: [{grid, count, bands}]}`, aggregati dal log di Log4OM2 (sola lettura); `404 map_unavailable`, `502 map_read` |
| `GET /log4om?limit=` | Ultimi QSO del log di Log4OM2 (sola lettura) e totale; `404 log4om_unavailable` |
| `GET /engine-config` | Impostazioni di JTDX lette dal suo `JTDX.ini` (solo voci elencate, nessuna password); `404 engine_config_unavailable` |
| `POST /engine-config` | `{changes: {chiave: valore}}` → chiude JTDX, scrive il file con copia di sicurezza, lo riavvia. Solo rete di casa (`403 local_only`), mai in trasmissione (`409 transmitting`); `400 bad_setting`, `502 engine_apply_failed` |
| `GET/PUT /clock` | Client NTP: `{config, last}`; `PUT` salva (`400 bad_clock`); accendere la correzione automatica solo dalla rete di casa |
| `POST /clock/check` · `POST /clock/correct` | Misura subito lo scarto; sposta l'ora di Windows (amministratore, solo rete di casa: `403 clock_needs_admin`/`local_only`) |
| `GET/PUT /dx-cluster?since=&limit=` | Impostazioni, stato dei collegamenti e spot recenti; `PUT` salva e rifa' i collegamenti (`400 bad_cluster`) |
| `GET /dx-cluster/catalog` · `POST /dx-cluster/test` | Server pubblici noti; prova di raggiungibilita' `{host, port}` → `{ok, ms, banner, error}` (non entra nel cluster) |
| `POST /dx-cluster/command` | `{server, text}` solo consultazione e impostazioni (`sh/`, `set/`, `unset/`, `accept/`, `reject/`, `clear/`); `400 bad_command`, `409 not_connected` |
| `GET /dxcc/{call}` | Paese, continente, prefisso e zone dal `cty.dat`; `404 dxcc_unknown` |
| `GET/PUT /ui-settings/{kind}` | Preferenze dell'interfaccia (`desktop` o `mobile`), chiavi `aurora-*`; `400 bad_kind`/`bad_settings` |
| `GET /stream` | WebSocket: eventi e spettro |

## Flusso `/stream`

- **Testo (JSON)**: un evento per messaggio, con il campo `type`: `status`, `decode`, `qso_logged`, `tx_halted`, `wanted`, `update_available` e `audio_level` (livello dell'ingresso audio ogni 250 ms: `peak`, `rms`, `clipped` da 0 a 1 e `state` = `silent`, `low`, `good`, `high` o `overdrive`; arriva solo se la cascata audio e' attiva).
  Alla connessione arriva subito l'ultimo `status`. Un client in ritardo riceve di nuovo lo stato corrente, non gli arretrati.
  `status` porta anche stazione DX, rapporto, frequenze Tx/Rx, messaggio in trasmissione e continente; `decode` porta paese, continente ed etichetta di novità.
- **Binario (spettro)**: `[0x01][seq u64][f_min u32][f_max u32][bins…]`, tutto big-endian; bin `u8` (1–4096), oggi 1024 bin su 0–3000 Hz, circa 15 righe al secondo.
- Il server chiude il flusso se il dispositivo viene revocato o allo spegnimento.

## Policy di trasmissione

Il trasporto si ricava dalle intestazioni del proxy: `cf-ray` o `cf-connecting-ip` ⇒ Cloudflare; `tailscale-user-login` ⇒ Tailscale; altrimenti rete locale.
Predefinito: i comandi che fanno trasmettere sono ammessi da rete locale e Tailscale, **non** da Cloudflare. Fermare la trasmissione è sempre ammesso.

## Variabili d'ambiente del programma `aurora`

Vedi `.env.example`. In breve: `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE`, `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_ALLOW_TX`,
`AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS` (giorni di storico dei messaggi, predefinito 30, 0 = per sempre), `AURORA_JTDX_INI`, `AURORA_LOG4OM_DB`, `AURORA_REPLAY_ON_CONNECT` (predefinito 0), `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_AUTO_*`.
