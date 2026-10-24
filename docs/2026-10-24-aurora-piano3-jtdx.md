# Aurora — Piano 3: collegamento reale a JTDX (hub UDP, motore JTDX, audio)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Far lavorare Aurora con JTDX 2.2.159 reale: ricevere stato e decode, rispondere ai CQ e fermare la trasmissione, **senza togliere il flusso a GridTracker2 né a Log4OM2**, con la cascata alimentata dalla scheda audio.

**Architecture:** Un crate `udp-hub` ascolta il server UDP primario di JTDX su una porta di Aurora, inoltra i datagrammi **identici** a GridTracker2 e rimanda a JTDX i comandi che arrivano da lì. Un crate `engine-jtdx` implementa `Engine` sopra `udp-hub` e `udp-bridge`: traduce Status/Decode/QsoLogged negli eventi di Aurora e i comandi (Reply, HaltTx) in pacchetti verso JTDX. La trasmissione è chiusa da due cancelli indipendenti (API e motore). Un crate `audio-spectrum` produce le righe della cascata dall'ingresso audio.

**Tech Stack:** Rust edition 2024 (rustc 1.98.0), `tokio 1.54.1`, `cpal 0.18.2`, `rustfft 6.4.1`, `udp-bridge` (workspace), `aurora-core`, `remote-api`. Versioni già fissate in `core/Cargo.toml` e verificate il 2026-10-24.

**Spec:** `docs/2026-10-24-aurora-design.md` (§3, §4 `udp-hub`, §6 salvaguardie) · `docs/udp-flussi.md` (mappa dei flussi) · `docs/fase0-rapporto.md` (formato dei messaggi, esito H1 e H5)

## Global Constraints

- Rust edition 2024; niente `unwrap()`/`expect()` fuori dai test; errori come `Result`; logging con `tracing`.
- Nessuna cartella locale degli strumenti di assistenza né riferimenti a tali strumenti nel repository, nemmeno nei trailer dei commit. Nessun nominativo reale di terzi nelle fixture.
- **Un Reply fa trasmettere** (provato dal vivo, `Tx2QSO=true`): per impostazione predefinita Aurora **non invia Reply** (`AURORA_ALLOW_TX=0`). HaltTx è invece sempre ammesso.
- JTDX ignora i comandi con identificativo diverso dal proprio: i comandi riusano l'`id` e lo `schema` dell'ultimo pacchetto ricevuto (oggi `"JTDX"`, schema 2).
- Il `Reply` rimanda il decode **originale** (ora, SNR, DT, DF, modo e testo identici, spazi finali compresi): JTDX lo ritrova per confronto del testo.
- Porte della stazione di Tiziano: GridTracker2 su `127.0.0.1:2333`, Log4OM2 su `2237` (direttamente da JTDX): Aurora non le occupa mai. Porta predefinita dell'hub: `127.0.0.1:2334`.
- Le prove con JTDX e la radio le fa Tiziano; gli agenti eseguono solo i test automatici (nessun comando verso JTDX reale).

## Review Focus

1. Datagrammi estranei (senza magico, troncati, di 65 KB) non fanno cadere l'hub né il motore e **non vengono inoltrati** a GridTracker2.
2. Con `allow_tx = false` nessun Reply arriva a JTDX; HaltTx arriva sempre, anche con `allow_tx = false`.
3. JTDX riavviato con un'altra porta di origine: i comandi seguono il nuovo indirizzo; senza JTDX noto, un comando dà un errore pulito, non un panico.
4. Una destinazione di inoltro chiusa o lenta non blocca l'hub né le altre destinazioni.
5. Decode con spazi finali o testo `<...>`: l'evento per l'interfaccia è ripulito, ma il Reply usa l'originale.

## File Structure

```
core/crates/
  udp-hub/       src/lib.rs                      # Hub, HubConfig, HubError
  engine-jtdx/   src/{lib,map,store}.rs          # JtdxEngine, mappatura, archivio dei decode recenti
  audio-spectrum/ src/{lib,analyzer,capture}.rs  # SpectrumAnalyzer (puro) + cattura cpal
  remote-api/    src/policy.rs  src/routes.rs    # modifica: cancello "live_tx"
  aurora/        src/lib.rs  src/main.rs         # modifica: EngineKind::Jtdx, variabili d'ambiente
docs/ guida-collegamento-jtdx.md  piano3-rapporto.md
```

---

### Task 1: `udp-hub`

**Files:** Create `core/crates/udp-hub/Cargo.toml`, `src/lib.rs`; Test `core/crates/udp-hub/tests/hub.rs`

**Interfaces:**
- Consumes: `udp_bridge::MAGIC` (solo per riconoscere i datagrammi validi: primi 4 byte).
- Produces:
  - `pub struct HubConfig { pub listen: SocketAddr, pub forward: Vec<SocketAddr> }`
  - `pub enum HubError { Bind(String), NoPeer, Send(String) }` (con `Display`)
  - `pub struct Hub` — `Hub::start(config: HubConfig) -> Result<Hub, HubError>`; `fn local_addr(&self) -> SocketAddr`; `fn jtdx_addr(&self) -> Option<SocketAddr>`; `fn subscribe(&self) -> broadcast::Receiver<Vec<u8>>` (i datagrammi validi arrivati da JTDX); `async fn send_to_jtdx(&self, bytes: &[u8]) -> Result<(), HubError>`; `fn dropped_count(&self) -> u64` (datagrammi scartati perché non validi); `fn stop(&self)`.
- Comportamento: ogni datagramma che inizia con il magico viene (a) pubblicato agli iscritti, (b) inoltrato **byte per byte** a ogni `forward` da un socket dedicato per destinazione, (c) l'indirizzo di origine diventa `jtdx_addr`. Ogni datagramma che arriva a uno dei socket di inoltro (comando di GridTracker2) viene girato a `jtdx_addr` dal socket di ascolto. Un datagramma senza magico è contato e scartato.

- [ ] **Step 1: Test** (`tests/hub.rs`, socket reali su 127.0.0.1 porta 0): `jtdx_packet_reaches_subscriber_and_forward_target_unchanged`, `command_from_forward_target_reaches_jtdx_via_hub`, `send_to_jtdx_without_peer_is_no_peer_error`, `datagram_without_magic_is_dropped_and_not_forwarded` (Review Focus 1, anche 65 000 byte), `jtdx_restart_with_new_source_port_redirects_commands` (Review Focus 3), `closed_forward_target_does_not_block_others` (Review Focus 4), `stop_releases_the_port`.
- [ ] **Step 2: Run** `cargo test -p udp-hub` — Expected: FAIL.
- [ ] **Step 3: Implementare** `Hub` con un task di ricezione per il socket di ascolto e uno per socket di inoltro; invii di inoltro con `try_send_to` (mai bloccanti); iscritti con `broadcast` di capacità 1024.
- [ ] **Step 4: Run** `cargo test -p udp-hub` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(udp-hub): hub UDP che inoltra il flusso di JTDX senza toglierlo agli altri programmi`.

---

### Task 2: `engine-jtdx`

**Files:** Create `core/crates/engine-jtdx/Cargo.toml`, `src/{lib,map,store}.rs`; Test `core/crates/engine-jtdx/tests/engine.rs`, `tests/fixtures` (copia dei pacchetti reali anonimizzati di `udp-bridge`)

**Interfaces:**
- Consumes: `Hub::{subscribe, send_to_jtdx}` (Task 1); `udp_bridge::{decode, encode_reply, encode_halt_tx, Message, Decode as WireDecode}`; `aurora_core::{Engine, EngineIo, Event, Command, StationStatus, DecodedMessage, DecodeTag, QsoRecord}`.
- Produces:
  - `pub struct JtdxConfig { pub allow_tx: bool, pub reply_max_age_ms: u64 }` (predefiniti `false` e `45_000`) e `pub trait Clock: Send + Sync { fn now_ms(&self) -> u64 }` con `SystemClock`.
  - `pub struct JtdxEngine` — `JtdxEngine::new(hub: Arc<Hub>, config: JtdxConfig, clock: Arc<dyn Clock>)`; `impl Engine`.
  - `map.rs`: `pub fn to_status(&udp_bridge::Status) -> StationStatus`; `pub fn to_decoded(id: u64, &WireDecode) -> DecodedMessage` (testo senza spazi finali; etichetta `Cq` se inizia con `CQ `, `CQDX ` o `QRZ`); `pub fn band_of(hz: u64) -> Option<&'static str>`; `pub fn to_qso(&udp_bridge::Message) -> Option<QsoRecord>`.
  - `store.rs`: `pub struct DecodeStore` — `insert(&mut self, wire: WireDecode, now_ms: u64) -> u64` (id progressivo, capacità 600), `get(&self, id: u64, now_ms: u64, max_age_ms: u64) -> Option<&WireDecode>`, `clear(&mut self)`.
- Comportamento: Status → `Event::Status`; Decode → `Event::Decode` (id assegnato dal motore, originale conservato nello store); `Clear` svuota lo store; `QsoLogged` → `Event::QsoLogged`; all'avvio e a ogni nuovo JTDX invia `Replay` (tipo 7) per ripopolare i decode correnti. `Command::HaltTx` → `encode_halt_tx(id, schema, false)` **sempre**. `Command::Reply{decode_id}` → solo se `allow_tx` **e** il decode e' nello store e non piu' vecchio di `reply_max_age_ms`: invia `encode_reply(id, schema, &originale, 0)`; altrimenti registra un avviso e non invia nulla.
- Nota di formato: `encode_replay(id, schema)` va aggiunto a `udp-bridge` (tipo 7, nessun campo) con il suo test in questo task.

- [ ] **Step 1: Test** `udp-bridge/tests/commands.rs`: `encode_replay_layout` (magico, schema, tipo 7, id, nessun altro byte). Run `cargo test -p udp-bridge --test commands` — FAIL; implementare; PASS.
- [ ] **Step 2: Test** `map.rs`/`store.rs` (unitari): `to_decoded_trims_trailing_spaces_and_tags_cq`, `to_decoded_keeps_angle_bracket_text`, `band_of_known_bands_and_none_outside`, `store_ids_are_progressive_and_capped_at_600`, `store_get_respects_max_age`, `store_clear_empties`.
- [ ] **Step 3: Test** `tests/engine.rs` (Hub reale + "JTDX finto" su UDP che riproduce le fixture reali): `status_fixture_becomes_station_status` (21 074 000 Hz, FT8, `ZZ9ZZZ`, decoding), `decode_fixture_becomes_decoded_message_with_cq_tag`, `clear_resets_reply_ability`, `halt_tx_always_reaches_jtdx_even_without_allow_tx` (Review Focus 2), `reply_not_sent_when_allow_tx_false` (nessun datagramma entro 300 ms), `reply_sent_with_original_untrimmed_decode_when_allowed` (i byte ricevuti dal JTDX finto sono identici a `encode_reply(..., &originale, 0)`, anche con testo con spazi finali — Review Focus 5), `reply_unknown_or_stale_decode_is_ignored`, `replay_is_requested_when_jtdx_first_seen`, `foreign_garbage_is_ignored_without_panic`.
- [ ] **Step 4: Run** `cargo test -p engine-jtdx` — Expected: FAIL.
- [ ] **Step 5: Implementare** `map`, `store`, `JtdxEngine::run` (un `select!` tra iscrizione all'hub e canale dei comandi).
- [ ] **Step 6: Run** `cargo test --workspace` — Expected: PASS.
- [ ] **Step 7: Commit** `feat(engine-jtdx): motore che traduce il flusso di JTDX negli eventi di Aurora`.

---

### Task 3: Cancello di trasmissione nell'API

**Files:** Modify `core/crates/remote-api/src/policy.rs`, `src/routes.rs`; Test `core/crates/remote-api/tests/api.rs`

**Interfaces:**
- Produces: `TxPolicy::default_safe()` resta; si aggiunge `pub fn with_live_tx(self, live: bool) -> TxPolicy` e `pub fn live_tx(&self) -> bool` (predefinito **falso**). `POST /commands/reply` risponde `403 {"error":"tx_disabled"}` se `live_tx` e' falso (controllato **prima** del trasporto e del decode). `halt-tx` non e' mai bloccato.

- [ ] **Step 1: Test:** `reply_is_403_tx_disabled_when_live_tx_is_off` (anche con decode noto), `reply_works_when_live_tx_is_on`, `halt_tx_works_when_live_tx_is_off`; adattare gli helper di test esistenti (`state()` attiva `live_tx` per i test di inoltro gia' presenti).
- [ ] **Step 2: Run** `cargo test -p remote-api --test api` — FAIL; implementare; PASS.
- [ ] **Step 3: Commit** `feat(remote-api): la trasmissione resta chiusa finche' non e' abilitata esplicitamente`.

---

### Task 4: Collegare tutto nel binario `aurora`

**Files:** Modify `core/crates/aurora/Cargo.toml`, `src/lib.rs`, `.env.example`; Test `core/crates/aurora/tests/e2e_jtdx.rs`

**Interfaces:**
- Consumes: `Hub`, `HubConfig`, `JtdxEngine`, `JtdxConfig`, `TxPolicy::with_live_tx`.
- Produces: `EngineKind::Jtdx`; `Config` guadagna `udp_listen: SocketAddr`, `udp_forward: Vec<SocketAddr>`, `allow_tx: bool`. Variabili d'ambiente: `AURORA_ENGINE=jtdx`, `AURORA_UDP_LISTEN` (predefinito `127.0.0.1:2334`), `AURORA_UDP_FORWARD` (elenco separato da virgole, predefinito vuoto), `AURORA_ALLOW_TX` (`0`). `RunningServer` espone `hub: Option<Arc<Hub>>`.

- [ ] **Step 1: Test** `config_reads_jtdx_variables_and_rejects_bad_ones` (indirizzi non validi, `AURORA_ALLOW_TX=2`). E2E con JTDX finto (fixture reali) su UDP: `e2e_jtdx_status_and_decode_arrive_over_websocket`, `e2e_forward_target_receives_identical_datagrams`, `e2e_reply_is_403_and_nothing_reaches_jtdx_by_default`, `e2e_reply_reaches_jtdx_only_with_allow_tx`, `e2e_halt_reaches_jtdx_by_default`, `e2e_command_from_forward_target_reaches_jtdx`.
- [ ] **Step 2: Run** `cargo test -p aurora` — FAIL; implementare; PASS (le prove con `sim` esistenti restano verdi).
- [ ] **Step 3: Commit** `feat(aurora): motore JTDX con hub UDP, inoltro e trasmissione chiusa per impostazione predefinita`.

---

### Task 5: `audio-spectrum`

**Files:** Create `core/crates/audio-spectrum/Cargo.toml`, `src/{lib,analyzer,capture}.rs`; Modify `core/crates/aurora/src/lib.rs`; Test `core/crates/audio-spectrum/tests/analyzer.rs`

**Interfaces:**
- Produces: `pub struct SpectrumAnalyzer` — `SpectrumAnalyzer::new(sample_rate: u32) -> Result<Self, AnalyzerError>` (FFT 4096 con finestra di Hann, passo ~66 ms, 1024 bin su 0–3000 Hz, valori `u8` dalla scala -110…-20 dB), `fn push(&mut self, samples: &[f32]) -> Vec<Vec<u8>>` (zero o piu' righe pronte). `pub fn run_capture(device_hint: Option<&str>, rows: broadcast::Sender<SpectrumRow>) -> Result<CaptureHandle, CaptureError>` con `cpal` (`CaptureHandle::stop`). `Config.audio_device: Option<String>` da `AURORA_AUDIO_DEVICE`: se presente (con qualunque motore) la cascata arriva dall'audio.

- [ ] **Step 1: Test** (segnali sintetici, nessun audio reale): `sine_1500hz_peaks_at_bin_512_plus_minus_2`, `silence_gives_floor_values`, `two_tones_give_two_peaks`, `row_rate_is_about_15_per_second`, `rejects_unsupported_sample_rate` (< 8000 Hz), `samples_in_small_chunks_equal_one_big_push`, `very_loud_signal_saturates_at_255_without_overflow`.
- [ ] **Step 2: Run** `cargo test -p audio-spectrum` — FAIL; implementare analizzatore con `rustfft`; PASS.
- [ ] **Step 3:** Implementare `run_capture` (cpal 0.18: `description().name()`, `build_input_stream(config.into(), …, None)`, mono dal primo canale, F32 e I16) e collegarlo in `aurora` con `AURORA_AUDIO_DEVICE`; verificare `cargo build` e `clippy`. La prova con la scheda audio reale (H2) e' manuale.
- [ ] **Step 4: Commit** `feat(audio-spectrum): cascata dall'ingresso audio`.

---

### Task 6: Documentazione e guida al collegamento

**Files:** Create `docs/guida-collegamento-jtdx.md`, `docs/piano3-rapporto.md`; Modify `README.md`, `CHANGELOG.md`, `.env.example`, `core/Cargo.toml` (versione 0.3.0)

- [ ] **Step 1:** Guida passo per passo per Tiziano: avviare Aurora con `AURORA_ENGINE=jtdx` e `AURORA_UDP_FORWARD=127.0.0.1:2333`; **solo dopo** cambiare in JTDX il server UDP primario da `2333` a `2334`; verificare in GridTracker2 che i dati continuino ad arrivare; prova a vuoto (Reply respinto); come abilitare `AURORA_ALLOW_TX=1` e `live_tx` **solo con carico fittizio**; come tornare indietro (rimettere `2333`).
- [ ] **Step 2:** Rapporto con decisioni, limiti e verifiche manuali; aggiornare README, CHANGELOG e versione.
- [ ] **Step 3: Commit** `docs: guida al collegamento con JTDX e rapporto del Piano 3 (v0.3.0)`.

---

### Task 7: Verifica finale

- [ ] **Step 1:** `cd core && cargo test --workspace && cargo clippy --workspace --all-targets` — Expected: tutto PASS, nessun avviso; `cd app && npm test && npm run build` — Expected: PASS.
- [ ] **Step 2:** Ricerca testuale dei nomi degli strumenti di assistenza e di nominativi reali di terzi sui file tracciati — Expected: nessun risultato.
- [ ] **Step 3:** Revisione del branch sui cinque punti del Review Focus; correzioni con test scritto prima.
- [ ] **Step 4: Commit**; il tag `v0.3.0` solo dopo le verifiche manuali di Tiziano.

---

## Self-review

- **Copertura:** hub davanti a GridTracker2 (specifica §4 `udp-hub`, `docs/udp-flussi.md`) → Task 1; Status/Decode/QsoLogged/Reply/HaltTx/Replay (rapporto Fase 0) → Task 2; salvaguardia "trasmissione chiusa" (specifica §6) → Task 2 e 3; cascata reale da audio (specifica §4 `audio-spectrum`) → Task 5; collegamento al binario → Task 4. **Fuori ambito, per scelta:** `rig-control` (frequenza/PTT via rigctld, H3), TriggerCQ e SetTxDeltaFreq (tipi 50/51, significato dei bool non confermato), automazioni (`qso-engine`), HTTPS/Tailscale/Cloudflare, scrittura sul log di Log4OM2 (resta a JTDX via porta 2237), copia dei QSO in un log proprio di Aurora.
- **Coerenza dei tipi:** `Hub::subscribe` (Task 1) → `JtdxEngine` (Task 2); `JtdxConfig.allow_tx` e `TxPolicy::live_tx` (Task 3) sono due cancelli distinti, entrambi impostati da `AURORA_ALLOW_TX` nel Task 4; `encode_replay` nasce in `udp-bridge` (Task 2) e serve solo al motore.
- **Proporzione:** il piano fissa firme, valori e test; i corpi restano all'implementatore.
- **Rischio dichiarato:** la prova completa richiede JTDX reale (Tiziano): i test automatici usano un "JTDX finto" che riproduce pacchetti reali; il comportamento con gli ascoltatori veri (GridTracker2) e l'inoltro dei suoi comandi va verificato a mano.
