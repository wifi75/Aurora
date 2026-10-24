# API di Aurora (v1)

Tutte le rotte stanno sotto `/api/v1`. La descrizione OpenAPI è servita da `GET /api/v1/openapi.json`.
L'autenticazione usa il cookie `aurora_session` (`HttpOnly; SameSite=Strict`, con `Secure` su HTTPS); sul server resta solo l'hash SHA-256 del token.

| Rotta | Auth | Descrizione |
|---|---|---|
| `GET /health` | no | `{status, name, version, author}` |
| `POST /pair/start` | solo da loopback | Genera il codice di 8 cifre (vale 120 s) e il QR. Corpo facoltativo `{"base_url": "http://192.168.1.10:5173"}` per far puntare il QR alla pagina di associazione |
| `POST /pair/finish` | no | `{code, device_name}` → `204` + cookie. Errori: `400 bad_code`/`bad_name`, `410 expired`, `429 locked` con `Retry-After` |
| `GET /devices` | sì | Dispositivi associati `[ {id, name, created_at} ]` |
| `DELETE /devices/{id}` | sì | Revoca un dispositivo (`204`, oppure `404`) |
| `POST /commands/halt-tx` | sì | Ferma la trasmissione. Idempotente e **mai bloccato** dalla policy di trasporto |
| `POST /commands/reply` | sì | `{decode_id}` → risponde a un decode noto. `403 tx_not_allowed_on_transport`, `404 unknown_decode` |
| `GET /stream` | sì | WebSocket: eventi e spettro |
| `GET /openapi.json` | no | Descrizione OpenAPI |

## Flusso `/stream`

- **Testo (JSON)**: un evento per messaggio, con il campo `type`: `status`, `decode`, `qso_logged`, `tx_halted`.
  Alla connessione arriva subito l'ultimo `status`. Un client in ritardo riceve di nuovo lo stato corrente, non gli arretrati.
- **Binario (spettro)**: `[0x01][seq u64][f_min u32][f_max u32][bins…]`, tutto big-endian; bin `u8` (1–4096), oggi 1024 bin su 0–3000 Hz,
  circa 15 righe al secondo.
- Il server chiude il flusso se il dispositivo viene revocato o allo spegnimento.

## Policy di trasmissione

Il trasporto si ricava dalle intestazioni del proxy: `cf-ray` o `cf-connecting-ip` ⇒ Cloudflare; `tailscale-user-login` ⇒ Tailscale; altrimenti rete locale.
Predefinito: i comandi che fanno trasmettere (`reply`) sono ammessi da rete locale e Tailscale, **non** da Cloudflare. Fermare la trasmissione è sempre ammesso.

## Variabili d'ambiente del programma `aurora`

`AURORA_BIND` (predefinito `127.0.0.1:8787`), `AURORA_DATA_DIR` (`data`), `AURORA_UI_DIR` (interfaccia compilata, facoltativa),
`AURORA_ENGINE` (`sim`), `AURORA_SIM_CYCLE_MS` (`15000`), `AURORA_SECURE_COOKIE` (`0`; `1` quando si arriva in HTTPS).
