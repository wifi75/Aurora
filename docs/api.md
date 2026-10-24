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
| `POST /commands/mode` | `{mode}` `FT8`/`FT4`. Stessi errori di `band` |
| `GET/PUT /output-level` | Livello audio di uscita di JTDX (cursore Pwr) come attenuazione `{attenuation_db}` 0-45; la scrittura richiede la trasmissione aperta e una via consentita. `400 bad_level` |

## Impostazioni e dati

| Rotta | Descrizione |
|---|---|
| `GET /station-info` | Come è avviata la stazione (sola lettura) |
| `GET/PUT /frequencies` | Frequenze di lavoro `[ {band, mode, hz} ]`; `400 bad_frequencies` |
| `GET/PUT /schedule` | Programmazione delle bande `{enabled, entries:[{hour, minute, band}]}`, al massimo 5 righe; `400 bad_schedule` |
| `GET/PUT /automation` | Stato, regole, limiti e registro; dal telefono solo prova a vuoto o spegnimento. `403 local_only`, `403 tx_disabled`, `400 bad_rules`/`notice_required` |
| `GET /stats` | Premi (DXCC, zone CQ e ITU, griglie, prefissi) e paesi mancanti, dal log ADIF |
| `GET /stream` | WebSocket: eventi e spettro |

## Flusso `/stream`

- **Testo (JSON)**: un evento per messaggio, con il campo `type`: `status`, `decode`, `qso_logged`, `tx_halted`.
  Alla connessione arriva subito l'ultimo `status`. Un client in ritardo riceve di nuovo lo stato corrente, non gli arretrati.
  `status` porta anche stazione DX, rapporto, frequenze Tx/Rx, messaggio in trasmissione e continente; `decode` porta paese, continente ed etichetta di novità.
- **Binario (spettro)**: `[0x01][seq u64][f_min u32][f_max u32][bins…]`, tutto big-endian; bin `u8` (1–4096), oggi 1024 bin su 0–3000 Hz, circa 15 righe al secondo.
- Il server chiude il flusso se il dispositivo viene revocato o allo spegnimento.

## Policy di trasmissione

Il trasporto si ricava dalle intestazioni del proxy: `cf-ray` o `cf-connecting-ip` ⇒ Cloudflare; `tailscale-user-login` ⇒ Tailscale; altrimenti rete locale.
Predefinito: i comandi che fanno trasmettere sono ammessi da rete locale e Tailscale, **non** da Cloudflare. Fermare la trasmissione è sempre ammesso.

## Variabili d'ambiente del programma `aurora`

Vedi `.env.example`. In breve: `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE`, `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_ALLOW_TX`,
`AURORA_AUDIO_DEVICE`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_AUTO_*`.
