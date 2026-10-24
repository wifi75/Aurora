# Aurora — Piano 2: core e interfaccia Live su stazione simulata

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Avere Aurora funzionante end-to-end con una **stazione simulata**: cascata, decode, stato e STOP TX visibili da browser e da telefono, accesso protetto da associazione del dispositivo. Il motore reale (JTDX) si innesta dopo, senza toccare il resto.

**Architecture:** Il core Rust ruota attorno a un bus di eventi tipizzato (`tokio::sync::broadcast`) e a un canale di comandi. Un trait `Engine` produce eventi e consuma comandi: oggi `SimEngine`, poi `JtdxEngine` (Piano 3, dopo la Fase 0). `remote-api` (Axum) espone API versionata `/api/v1`, un flusso WebSocket (JSON per gli eventi, binario per le righe di spettro) e serve l'interfaccia. L'interfaccia è React + TypeScript con un renderer della cascata che sceglie WebGPU o WebGL2. Nuove integrazioni (MQTT/Home Assistant, webhook, PSK Reporter) saranno semplici consumatori del bus o client dell'API, senza modificare il core.

**Tech Stack:** Rust edition 2024 (rustc 1.98.0): `tokio 1.54.1`, `axum 0.8.9`, `tower 0.5.3`, `tower-http 0.7.1`, `tokio-tungstenite 0.30.0`, `futures-util 0.3.34`, `serde 1.0.229`, `serde_json 1.0.154`, `rusqlite 0.40.2`, `sha2 0.11.1`, `getrandom 0.4.3`, `hex 0.4.3`, `qrcode 0.14.1`, `utoipa 6.0.0`, `tracing 0.1.44`. Interfaccia: `react 19.3.1`, `react-dom 19.3.1`, `typescript 7.0.2`, `vite 8.3.5`, `@vitejs/plugin-react 6.1.2`, `vitest 5.0.5`, `jsdom 30.1.3`, `@testing-library/react 16.3.3`, `@testing-library/jest-dom 7.0.1`, `@tanstack/react-query 5.104.5`, `zustand 5.0.16`, `zod 4.6.7`, `@webgpu/types 0.1.74`. Versioni verificate sui registri il 2026-10-24.

**Spec:** `docs/2026-10-24-aurora-design.md` (§3 architettura, §4 componenti, §5 interfaccia e tema "Notte radio", §7 sicurezza, §9 stack)

## Global Constraints

- Rust edition 2024; niente `unwrap()`/`expect()` fuori dai test; errori come `Result` con tipi propri; logging con `tracing`, mai `println!` nel core.
- Nessun segreto nel repository; configurazione da variabili d'ambiente documentate in `.env.example`.
- Il repository non contiene la cartella locale degli strumenti di assistenza né riferimenti a tali strumenti, nemmeno nei trailer dei commit.
- Tema **Notte radio**: fondo `#0b0e14`, pannelli `#111722`, bordi `#243046`, testo `#cdd7e6`, secondario `#6f7f96`, accento ciano `#3fd6c8`, ambra `#f2a93b` per gli eventi rari, rosso `#ff5d5d` solo per STOP TX; raggio 4; dati in monospazio; una sola famiglia di icone; font self-hosted. Tutti i valori estetici vivono nei token (`app/src/styles/tokens.css`): nessun colore scritto a mano nei componenti.
- Movimento: solo `transform`/`opacity`, nessuna animazione su azioni ripetute; `prefers-reduced-motion` rispettato.
- Touch: bersagli ≥48 px, testo dei campi ≥16 px, layout provato in verticale e orizzontale.
- Piè di pagina in una riga letto dall'endpoint di stato: `Aurora vX.Y.Z — Progettazione e sviluppo: Tiziano Cassone` (`APP_NAME`, `APP_VERSION`, `APP_AUTHOR` definiti in un solo punto: `core/crates/aurora-core/src/version.rs`).
- Token di sessione in cookie `HttpOnly; SameSite=Strict` (e `Secure` tranne su loopback); mai in `localStorage`, mai nell'URL; sul server solo l'hash SHA-256.
- API versionata sotto `/api/v1`; spettro: 1024 bin `u8`, 0–3000 Hz, circa 15 righe al secondo.
- Le prove manuali (telefono, aspetto grafico) le fa Tiziano; gli agenti eseguono i test automatici.

## Review Focus

1. Client WebSocket lento o disconnesso a metà flusso: il server lo scarta senza bloccare il bus né gli altri client; un client in ritardo (`Lagged`) riceve lo stato corrente, non una cascata di arretrati.
2. Tentativi ripetuti di indovinare il codice di associazione: dopo 5 errori il codice si blocca per 60 secondi.
3. Riga di spettro vuota o con più di 4096 bin: rifiutata alla sorgente (errore, non panico, non invio al client).
4. `Reply` con un `decode_id` sconosciuto → 404 con messaggio chiaro; `HaltTx` mentre non si trasmette → idempotente (200).
5. Richiesta senza cookie, con cookie sconosciuto o revocato → 401, anche sul WebSocket; il cookie non compare mai nei log.

## File Structure

```
Aurora/
  core/crates/
    aurora-core/   src/{lib,version,event,command,bus,engine}.rs      # tipi di dominio, bus, trait Engine
    engine-sim/    src/lib.rs                                         # stazione simulata deterministica
    logbook/       src/{lib,adif}.rs                                  # SQLite: QSO, duplicati, ADIF
    auth/          src/lib.rs                                         # associazione dispositivi, token
    remote-api/    src/{lib,routes,ws,frames,policy}.rs               # Axum, WebSocket, OpenAPI
    aurora/        src/main.rs                                        # binario: collega tutto
  app/             package.json  vite.config.ts  tsconfig.json  index.html
    src/styles/tokens.css  src/api/{client,schemas,stream}.ts
    src/render/{colormap,ring,backend,webgl2,webgpu}.ts
    src/components/*.tsx  src/pages/{Live,Pair}.tsx  src/main.tsx
    public/{manifest.webmanifest,sw.js}
  scripts/avvia-sviluppo.ps1
```

---

### Task 1: `aurora-core` — tipi, bus, trait `Engine`

**Files:**
- Create: `core/crates/aurora-core/Cargo.toml`, `src/{lib,version,event,command,bus,engine}.rs`
- Test: `core/crates/aurora-core/tests/bus.rs`

**Interfaces:**
- Produces:
  - `version.rs`: `pub const APP_NAME: &str = "Aurora"; pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION"); pub const APP_AUTHOR: &str = "Tiziano Cassone";`
  - `event.rs`: `#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)] #[serde(tag = "type", rename_all = "snake_case")] pub enum Event { Status(StationStatus), Decode(DecodedMessage), QsoLogged(QsoRecord), TxHalted }`; `StationStatus { dial_hz: u64, mode: String, tx_enabled: bool, transmitting: bool, decoding: bool, de_call: String, de_grid: String }`; `DecodedMessage { id: u64, time_ms: u32, snr: i32, delta_time: f32, freq_hz: u32, text: String, tag: Option<DecodeTag> }`; `enum DecodeTag { Cq, NewDxcc, NewZone, Auto }`; `QsoRecord { call: String, grid: String, band: String, mode: String, freq_hz: u64, rst_sent: String, rst_rcvd: String, time_ms: u32 }`; `SpectrumRow { seq: u64, f_min_hz: u32, f_max_hz: u32, bins: Vec<u8> }` (non è un `Event`: viaggia su canale proprio).
  - `command.rs`: `pub enum Command { HaltTx, Reply { decode_id: u64 } }`.
  - `bus.rs`: `pub struct Bus` con `Bus::new(capacity: usize) -> Bus`, `fn events(&self) -> broadcast::Sender<Event>`, `fn spectrum(&self) -> broadcast::Sender<SpectrumRow>`, `fn subscribe_events(&self) -> broadcast::Receiver<Event>`, `fn subscribe_spectrum(&self) -> broadcast::Receiver<SpectrumRow>`, `fn commands(&self) -> mpsc::Sender<Command>`, `fn take_command_rx(&self) -> Option<mpsc::Receiver<Command>>` (una sola volta).
  - `engine.rs`: `pub struct EngineIo { pub events: broadcast::Sender<Event>, pub spectrum: broadcast::Sender<SpectrumRow>, pub commands: mpsc::Receiver<Command> }` e `pub trait Engine: Send + 'static { fn run(self, io: EngineIo) -> impl Future<Output = ()> + Send; }`.
  - `SpectrumRow::validate(&self) -> Result<(), CoreError>`: errore se `bins` è vuoto o più lungo di 4096, o `f_min_hz >= f_max_hz`.

- [ ] **Step 1: Test** in `tests/bus.rs`: `two_subscribers_both_receive_event`, `command_channel_delivers_in_order`, `take_command_rx_works_only_once`, `spectrum_validate_rejects_empty_and_oversized_and_inverted_range` (Review Focus 3), `event_serializes_with_type_tag` (`{"type":"tx_halted"}` e `{"type":"decode",...}`).
- [ ] **Step 2: Run** `cargo test -p aurora-core` — Expected: FAIL (crate/tipi non definiti).
- [ ] **Step 3: Implementare** i tipi e il bus come da Interfaces.
- [ ] **Step 4: Run** `cargo test -p aurora-core` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(aurora-core): tipi di dominio, bus e trait Engine`.

---

### Task 2: `engine-sim` — stazione simulata

**Files:**
- Create: `core/crates/engine-sim/Cargo.toml`, `src/lib.rs`
- Test: `core/crates/engine-sim/tests/sim.rs`

**Interfaces:**
- Consumes: `Engine`, `EngineIo`, `Event`, `Command`, `SpectrumRow` (Task 1).
- Produces: `pub struct SimConfig { pub seed: u64, pub cycle_ms: u64 }` (predefinito 15 000), `pub struct SimEngine`, `SimEngine::new(SimConfig)`, `impl Engine for SimEngine`.
- Comportamento: ogni ~66 ms una `SpectrumRow` (1024 bin, 0–3000 Hz, rumore deterministico più 6–10 tracce strette per ciclo); a ogni ciclo 5–12 `Decode` con testi realistici (es. `CQ DX JA1XYZ PM95`, `DL2ABC K1ABC FN42`), una quota con tag `Cq`/`NewDxcc`/`NewZone`; `Status` all'avvio e a ogni ciclo (20 m, 14 074 000 Hz, FT8, de_call `IU3CYV`, griglia `JN55`); `Command::HaltTx` → `Event::TxHalted` e `transmitting=false`; `Command::Reply` su un id noto → `Status{transmitting:true}` (simulato). Stesso `seed` ⇒ stessa sequenza di decode.

- [ ] **Step 1: Test** (con `tokio::time::pause`): `emits_status_on_start`, `emits_spectrum_rows_with_1024_bins_in_range`, `emits_decodes_each_cycle`, `same_seed_same_decode_texts`, `halt_tx_emits_tx_halted_and_clears_transmitting`, `reply_unknown_id_is_ignored_without_panic`.
- [ ] **Step 2: Run** `cargo test -p engine-sim` — Expected: FAIL.
- [ ] **Step 3: Implementare** `SimEngine` con un generatore pseudo-casuale proprio (xorshift) per restare deterministici e senza dipendenze.
- [ ] **Step 4: Run** `cargo test -p engine-sim` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(engine-sim): stazione simulata deterministica`.

---

### Task 3: `logbook` — QSO, duplicati, ADIF

**Files:**
- Create: `core/crates/logbook/Cargo.toml`, `src/{lib,adif}.rs`
- Test: `core/crates/logbook/tests/logbook.rs`

**Interfaces:**
- Consumes: `QsoRecord` (Task 1).
- Produces: `Logbook::open(path: &Path) -> Result<Logbook, LogError>`, `Logbook::open_in_memory()`, `fn add(&self, qso: &QsoRecord) -> Result<i64, LogError>`, `fn is_dupe(&self, call: &str, band: &str, mode: &str) -> Result<bool, LogError>`, `fn count(&self) -> Result<u64, LogError>`, `fn export_adif(&self) -> Result<String, LogError>`; migrazioni versionate con `PRAGMA user_version`.

- [ ] **Step 1: Test:** `add_then_count`, `dupe_same_call_band_mode`, `not_dupe_on_other_band`, `call_comparison_is_case_insensitive`, `adif_contains_call_band_mode_and_eoh_eor` (formato `<CALL:6>JA1XYZ ... <EOR>`), `adif_escapes_length_for_utf8` (la lunghezza ADIF è in byte), `reopening_keeps_data` (file temporaneo), `migration_sets_user_version_1`.
- [ ] **Step 2: Run** `cargo test -p logbook` — Expected: FAIL.
- [ ] **Step 3: Implementare** con `rusqlite 0.40.2` (feature `bundled`); chiamata normalizzata in maiuscolo; indice su `(call, band, mode)`.
- [ ] **Step 4: Run** `cargo test -p logbook` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(logbook): log QSO su SQLite con duplicati ed export ADIF`.

---

### Task 4: `auth` — associazione e token

**Files:**
- Create: `core/crates/auth/Cargo.toml`, `src/lib.rs`
- Test: `core/crates/auth/tests/auth.rs`

**Interfaces:**
- Produces: `trait Clock { fn now_secs(&self) -> u64 }` e `SystemClock`; `DeviceStore::open(path, clock) -> Result<DeviceStore, AuthError>` / `open_in_memory(clock)`; `fn start_pairing(&self) -> PairingCode` (`String` di 8 cifre, valido 120 s, un solo codice attivo); `fn finish_pairing(&self, code: &str, device_name: &str) -> Result<Token, AuthError>` (`Token(String)` di 64 caratteri esadecimali = 256 bit casuali da `getrandom`); `fn verify(&self, token: &str) -> Option<DeviceId>`; `fn list(&self) -> Result<Vec<DeviceInfo>, AuthError>`; `fn revoke(&self, id: DeviceId) -> Result<bool, AuthError>`. `AuthError::{BadCode, Expired, Locked { retry_after_secs: u64 }, Storage}`.
- Sicurezza: nel database solo `sha256(token)`; confronto in tempo costante; dopo 5 `BadCode` consecutivi → `Locked` per 60 s (Review Focus 2).

- [ ] **Step 1: Test:** `pairing_returns_64_hex_token_that_verifies`, `wrong_code_is_bad_code`, `code_expires_after_120s`, `code_is_single_use`, `five_wrong_codes_lock_for_60s_even_with_right_code` (poi sblocco a 61 s), `database_stores_only_hash` (cerca il token nel file: assente), `revoked_token_no_longer_verifies`, `unknown_token_is_none`, `list_never_returns_token_or_hash`.
- [ ] **Step 2: Run** `cargo test -p auth` — Expected: FAIL.
- [ ] **Step 3: Implementare** con `rusqlite`, `sha2`, `getrandom`, `hex`; orologio iniettato per testare scadenze.
- [ ] **Step 4: Run** `cargo test -p auth` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(auth): associazione dispositivi con token a hash e blocco anti-forza-bruta`.

---

### Task 5: `remote-api` — HTTP, WebSocket, OpenAPI

**Files:**
- Create: `core/crates/remote-api/Cargo.toml`, `src/{lib,routes,ws,frames,policy}.rs`
- Test: `core/crates/remote-api/tests/api.rs`, `tests/ws.rs`

**Interfaces:**
- Consumes: `Bus`, `Event`, `Command`, `SpectrumRow`, `version::*` (Task 1), `DeviceStore` (Task 4), `Logbook` (Task 3).
- Produces: `pub struct AppState { pub bus: Bus, pub auth: Arc<DeviceStore>, pub logbook: Arc<Logbook>, pub policy: TxPolicy, pub dev_origin_secure: bool }`, `pub fn build_router(state: AppState) -> Router`.
- Rotte (tutte sotto `/api/v1`):
  - `GET /health` (senza autenticazione) → `{"status":"ok","name","version","author"}`.
  - `POST /pair/start` (solo da loopback) → `{"code","qr_svg","expires_in":120}`; `POST /pair/finish {"code","device_name"}` → `204` + `Set-Cookie: aurora_session=…; HttpOnly; SameSite=Strict; Path=/; Max-Age=…` (+`Secure` se `dev_origin_secure`), errori `400 bad_code`, `410 expired`, `429 locked` con `Retry-After`.
  - `GET /devices`, `DELETE /devices/{id}` (autenticate).
  - `POST /commands/halt-tx`, `POST /commands/reply {"decode_id"}` (autenticate; sottoposte a `TxPolicy`); `reply` su id sconosciuto → `404`.
  - `GET /stream` (upgrade WebSocket, autenticato dal cookie): frame di testo `Event` in JSON; frame binari di spettro con layout `[0x01][seq u64 BE][f_min u32 BE][f_max u32 BE][bins…]`; client in ritardo → invio dello stato corrente e proseguimento (Review Focus 1).
  - `GET /openapi.json` generato con `utoipa`.
- `policy.rs`: `pub enum Transport { Lan, Tailscale, Cloudflare }`, `pub struct TxPolicy { pub allow_tx_on: HashSet<Transport> }`, `fn allows(&self, t: Transport) -> bool`; il trasporto di una richiesta viene dall'intestazione impostata dal proxy fidato o, in assenza, `Lan` (qui basta il modello; l'integrazione reale è in Fase 3).
- Toggle di sicurezza: cookie mai nei log (`tracing` con `skip` sui header sensibili).

- [ ] **Step 1: Test** `tests/api.rs` con `tower::ServiceExt::oneshot`: `health_is_public_and_has_author`, `protected_route_without_cookie_is_401`, `unknown_cookie_is_401`, `pair_flow_sets_httponly_samesite_cookie`, `pair_start_refuses_non_loopback`, `locked_pairing_returns_429_with_retry_after`, `halt_tx_is_idempotent_200`, `reply_unknown_decode_id_is_404`, `tx_command_denied_by_policy_on_cloudflare_is_403`, `revoked_device_gets_401`, `openapi_lists_health_and_stream`. `tests/ws.rs` con server reale su porta effimera e client `tokio-tungstenite`: `ws_without_cookie_is_rejected`, `ws_receives_json_event_and_binary_spectrum_frame_with_expected_layout`, `slow_client_does_not_block_fast_client` (Review Focus 1), `ws_closed_after_device_revoked`.
- [ ] **Step 2: Run** `cargo test -p remote-api` — Expected: FAIL.
- [ ] **Step 3: Implementare** il router, l'estrattore di autenticazione, il WebSocket (un `select!` tra eventi, spettro e chiusura) e `frames::encode_spectrum(&SpectrumRow) -> Vec<u8>`.
- [ ] **Step 4: Run** `cargo test -p remote-api` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(remote-api): API v1, flusso WebSocket e associazione dispositivi`.

---

### Task 6: Binario `aurora` e script di avvio

**Files:**
- Create: `core/crates/aurora/Cargo.toml`, `src/main.rs`, `scripts/avvia-sviluppo.ps1`; modify `.env.example`, `README.md`
- Test: `core/crates/aurora/tests/e2e.rs`

**Interfaces:**
- Consumes: tutto quanto sopra.
- Produces: `pub async fn run(config: Config, shutdown: impl Future<Output=()>) -> Result<RunningServer, StartError>` con `Config { bind: SocketAddr, data_dir: PathBuf, ui_dir: Option<PathBuf>, engine: EngineKind }`; `Config::from_env()` legge `AURORA_BIND` (predefinito `127.0.0.1:8787`), `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`sim`); il binario serve i file statici di `ui_dir` con ripiego su `index.html`.
- Script `avvia-sviluppo.ps1` secondo lo standard dell'utente: salta ciò che è già in ascolto, una sola tab di Windows Terminal con pannelli per core e interfaccia (titolo e colore fissi, `--suppressApplicationTitle`, `--` prima del comando), attesa dell'ascolto reale, riepilogo con i link e l'indirizzo di rete per il telefono.

- [ ] **Step 1: Test** `tests/e2e.rs` (server reale, porta effimera, `SimEngine`, orologio reale con cicli brevi): `e2e_pair_connect_receive_decode_and_spectrum_within_3s`, `e2e_halt_tx_emits_tx_halted_over_stream`, `e2e_reply_then_status_shows_transmitting`, `e2e_graceful_shutdown_closes_streams`, `e2e_decode_with_tag_roundtrips_to_json`.
- [ ] **Step 2: Run** `cargo test -p aurora --test e2e` — Expected: FAIL.
- [ ] **Step 3: Implementare** `run`, `Config` e `main` (tracing a console, chiusura con Ctrl+C).
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: tutto PASS.
- [ ] **Step 5: Commit** `feat(aurora): binario che collega bus, simulatore, log e API`.

---

### Task 7: Interfaccia — scaffolding, token e client API

**Files:**
- Create: `app/package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/styles/tokens.css`, `src/api/{schemas,client,stream}.ts`, `src/main.tsx`, `src/components/Footer.tsx`
- Test: `app/src/api/schemas.test.ts`, `app/src/components/Footer.test.tsx`

**Interfaces:**
- Produces:
  - `schemas.ts` (Zod 4): `HealthSchema`, `EventSchema` (unione discriminata su `type`: `status`, `decode`, `qso_logged`, `tx_halted`), tipi derivati `Health`, `AuroraEvent`.
  - `client.ts`: `getHealth(): Promise<Health>`, `postHalt(): Promise<void>`, `postReply(decodeId: number): Promise<void>`, `finishPairing(code: string, deviceName: string): Promise<void>`; tutte con `credentials: 'include'`.
  - `stream.ts`: `openStream(handlers: { onEvent(e: AuroraEvent): void; onSpectrum(row: SpectrumFrame): void; onState(s: 'connecting'|'open'|'closed'): void }): { close(): void }` con ricollegamento a ritardo crescente (500 ms → 8 s); `decodeSpectrumFrame(buf: ArrayBuffer): SpectrumFrame | null` (restituisce `null` su frame troncato o tipo ≠ `0x01`).
  - `Footer.tsx`: legge `/health` e mostra su una riga `Aurora v0.1.0 — Progettazione e sviluppo: Tiziano Cassone`.
  - `tokens.css`: variabili CSS per colori, raggio, spaziature (scala 4 px), altezze controllo (44/52 px con `pointer: coarse`), durate ed easing (`--ease-out: cubic-bezier(0.23,1,0.32,1)`), modalità chiara e scura.
- Vite: proxy di sviluppo `/api` → `http://127.0.0.1:8787` (anche `ws: true`).

- [ ] **Step 1: Test:** `EventSchema accepts a decode event`, `EventSchema rejects unknown type`, `decodeSpectrumFrame reads layout seq/fmin/fmax/bins`, `decodeSpectrumFrame returns null on truncated frame`, `decodeSpectrumFrame returns null on wrong type byte`, `Footer shows name version and author on one line` (fetch simulato).
- [ ] **Step 2: Run** `npm test` — Expected: FAIL.
- [ ] **Step 3: Implementare** scaffolding Vite/React/TS (`npm create`-style a mano, con le versioni fissate), schemi, client, stream, token, footer. Se `typescript 7.0.2` non funziona con Vite 8 o Vitest 5, usare l'ultima 6.x stabile e annotare la decisione nel rapporto.
- [ ] **Step 4: Run** `npm test && npm run build` — Expected: PASS e build senza errori.
- [ ] **Step 5: Commit** `feat(app): scaffolding, token Notte radio, client API e flusso`.

---

### Task 8: Renderer della cascata (WebGPU con ripiego WebGL2)

**Files:**
- Create: `app/src/render/{colormap,ring,backend,webgl2,webgpu}.ts`, `app/src/components/Waterfall.tsx`
- Test: `app/src/render/colormap.test.ts`, `ring.test.ts`, `backend.test.ts`

**Interfaces:**
- Produces:
  - `colormap.ts`: `export const NOTTE_RADIO_STOPS: ReadonlyArray<[number, [number, number, number]]>` (blu notte `[8,12,24]` → `[12,52,78]` a 0.35 → `[22,150,150]` a 0.6 → `[63,214,200]` a 0.82 → `[242,200,90]` a 0.93 → `[255,250,235]`); `buildLut(stops, size = 256): Uint8Array` (RGBA); `lutColor(lut: Uint8Array, v: number): [number, number, number]`.
  - `ring.ts`: `class RowRing { constructor(width: number, height: number); push(row: Uint8Array): void; readonly head: number; readonly filled: number; copyInto(target: Uint8Array): void }` — le righe più nuove sovrascrivono le più vecchie; una riga di larghezza diversa viene riscalata/ritagliata alla larghezza fissata.
  - `backend.ts`: `export interface WaterfallBackend { init(canvas: HTMLCanvasElement): Promise<void>; pushRow(row: Uint8Array): void; resize(w: number, h: number): void; destroy(): void; readonly kind: 'webgpu' | 'webgl2' }`; `chooseBackend(env: { hasWebGPU: boolean; hasWebGL2: boolean; forced?: 'webgpu'|'webgl2' }): 'webgpu'|'webgl2'|'none'`; `createBackend(canvas): Promise<WaterfallBackend>` (prova WebGPU, ripiega su WebGL2 se `requestAdapter()` fallisce).
  - `webgl2.ts`/`webgpu.ts`: texture ad anello `R8` larghezza 1024 × altezza 512 (≈34 s a 15 righe/s), scorrimento con un offset nella shader (nessuna ricopiatura), LUT come texture 1D; marcatori di frequenza sovrapposti opzionali.
  - `Waterfall.tsx`: monta il canvas, riceve righe da `openStream`, rispetta `prefers-reduced-motion` (nessun effetto aggiuntivo) e `devicePixelRatio`.

- [ ] **Step 1: Test:** `buildLut first and last stop match NOTTE_RADIO_STOPS`, `lutColor clamps below 0 and above 1`, `RowRing wraps and overwrites oldest`, `RowRing head and filled after more pushes than height`, `RowRing resamples a wrong-width row`, `chooseBackend prefers webgpu`, `chooseBackend falls back to webgl2`, `chooseBackend returns none when neither exists`, `chooseBackend honors forced backend only if available`.
- [ ] **Step 2: Run** `npm test` — Expected: FAIL.
- [ ] **Step 3: Implementare** la logica pura (colormap, ring, scelta backend) e i due renderer; per `webgpu.ts` usare `@webgpu/types`. I renderer richiedono un browser: non hanno test automatici, vedi la verifica manuale al Step 5.
- [ ] **Step 4: Run** `npm test && npm run build` — Expected: PASS e build senza errori di tipo.
- [ ] **Step 5: VERIFICA MANUALE (Tiziano):** con il core in esecuzione (`scripts/avvia-sviluppo.ps1`), aprire l'interfaccia su PC e telefono; atteso: cascata che scorre fluida con tracce nitide; indicare in `docs/piano2-rapporto.md` il backend usato (`kind`) e i fotogrammi al secondo percepiti.
- [ ] **Step 6: Commit** `feat(app): cascata con WebGPU e ripiego WebGL2`.

---

### Task 9: Pagina Live, associazione e PWA

**Files:**
- Create: `app/src/pages/{Live,Pair}.tsx`, `app/src/components/{StatusBar,DecodeList,StopButton}.tsx`, `app/src/state/station.ts`, `app/public/manifest.webmanifest`, `app/public/sw.js`
- Test: `app/src/state/station.test.ts`, `app/src/components/*.test.tsx`

**Interfaces:**
- Consumes: `openStream`, `postHalt`, `postReply`, `finishPairing` (Task 7), `Waterfall` (Task 8).
- Produces: `state/station.ts` (Zustand): `useStation` con `status: StationStatus | null`, `decodes: DecodedMessage[]` (massimo 200, i più recenti prima), `connection: 'connecting'|'open'|'closed'`, `apply(e: AuroraEvent): void`.
- `StopButton`: pulsante **STOP TX** sempre visibile (anche con collegamento caduto), rosso `--stop`, bersaglio ≥48 px, invia `postHalt()` e mostra l'esito; un solo tocco, nessuna conferma.
- `DecodeList`: righe (tempo, SNR, frequenza, testo) con etichette `NUOVO DXCC` (ambra), `AUTO` (ciano); tocco su una riga ⇒ `postReply(id)` con conferma visiva; stato vuoto con invito ("In ascolto: i messaggi compaiono qui") e stato di errore chiari.
- `Pair`: campo codice (8 cifre, `inputmode="numeric"`, ≥16 px) e nome del dispositivo; mostra l'errore e il conto alla rovescia su `429`.
- `Live`: layout a griglia `minmax(0,1fr)`; su schermi larghi cascata + lista decode a fianco, su telefono colonna singola con cascata in alto e STOP TX fisso in basso.
- PWA: `manifest.webmanifest` (nome Aurora, `display: standalone`, colori dai token, icone), `sw.js` con cache della sola shell statica, mai delle chiamate `/api`.

- [ ] **Step 1: Test:** `apply status updates station`, `apply decode keeps newest first and caps at 200`, `StopButton calls postHalt once per click`, `StopButton stays enabled when connection is closed`, `DecodeList shows NUOVO DXCC badge`, `DecodeList empty state invites listening`, `DecodeList row click calls postReply with id`, `Pair shows error and countdown on 429`, `Pair blocks submit while code has fewer than 8 digits` (con messaggio, senza disabilitare il pulsante).
- [ ] **Step 2: Run** `npm test` — Expected: FAIL.
- [ ] **Step 3: Implementare** stato, componenti, pagine e PWA; routing con `react-router 8.5.0` (aggiungerlo alle dipendenze); stati di caricamento e di errore su ogni chiamata.
- [ ] **Step 4: Run** `npm test && npm run build` — Expected: PASS e build senza errori.
- [ ] **Step 5: VERIFICA MANUALE (Tiziano) su telefono:** associazione tramite codice, installazione come app, cascata e decode in tempo reale, STOP TX; layout in verticale e in orizzontale; annotare in `docs/piano2-rapporto.md`.
- [ ] **Step 6: Commit** `feat(app): pagina Live, associazione dispositivo e PWA`.

---

### Task 10: Documentazione, versione e verifica finale

**Files:**
- Create: `docs/piano2-rapporto.md`, `docs/api.md`; modify `README.md`, `CHANGELOG.md`, `TODO.md`, `memory.md`, `core/Cargo.toml` (versione 0.2.0), `app/package.json` (0.2.0)

- [ ] **Step 1:** `docs/api.md` con rotte, formato dei frame del flusso e policy di trasmissione (estratto da `/api/v1/openapi.json`).
- [ ] **Step 2:** `README.md` con avvio rapido (`scripts/avvia-sviluppo.ps1`) e come associare un telefono; `CHANGELOG.md` 0.2.0; allineare la versione di `package.json` a quella del core.
- [ ] **Step 3: Verifica finale.** `cd core && cargo test --workspace && cargo clippy --workspace --all-targets` — Expected: tutto PASS, nessun avviso; `cd app && npm test && npm run build` — Expected: PASS; ricerca testuale dei nomi degli strumenti di assistenza sui file tracciati — Expected: nessun risultato.
- [ ] **Step 4: Commit** `docs: API, rapporto del Piano 2 e versione 0.2.0`; tag `v0.2.0` solo dopo le verifiche manuali di Tiziano.

---

## Self-review

- **Copertura della specifica:** §3 architettura (bus, `Engine`, API, interfaccia) → Task 1, 5, 6; §4 componenti `logbook`, `auth`, `remote-api` → Task 3, 4, 5, `engine` → Task 1–2; §5 cascata WebGPU/WebGL2, tema Notte radio, stati vuoti/errore, PWA → Task 7–9; §7 sicurezza (token a hash, cookie HttpOnly, anti-forza-bruta, policy per trasporto) → Task 4–5; §9 stack → intestazione. **Non coperti da questo piano, per scelta:** `udp-bridge` verso JTDX come motore reale (Piano 3, dopo la Fase 0), `audio-spectrum` reale, `rig-control`, `qso-engine`/automazioni, mappa globale, Tailscale/Cloudflare reali (la sola `TxPolicy` è modellata), avvisi sonori, inventario di parità.
- **Coerenza dei tipi:** `SpectrumRow` (Task 1) → `frames::encode_spectrum` (Task 5) → `decodeSpectrumFrame` (Task 7) con lo stesso layout `[0x01][seq][fmin][fmax][bins]`; `Event` con tag `type` snake_case in Rust (Task 1) e unione discriminata Zod (Task 7); `Command`/rotte di Task 5 usate da `client.ts` (Task 7).
- **Proporzione:** il piano dà firme, test e valori fissati dalla specifica e lascia ai corpi delle funzioni le scelte che i test determinano.
- **Rischio dichiarato:** TypeScript 7 e Vite 8 sono molto recenti; il ripiego a TypeScript 6.x è previsto al Task 7, Step 3.
- **Estensibilità (il tuo obiettivo "futuri sviluppi e nuove integrazioni"):** nuovi motori = nuova implementazione di `Engine`; nuove integrazioni (Home Assistant/MQTT, webhook, PSK Reporter, eQSL) = consumatori del bus o client dell'API `/api/v1` con OpenAPI; nuovi client (app nativa, Node-RED) = stesso flusso WebSocket. Nessuna modifica al core per aggiungerli.
