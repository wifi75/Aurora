# Aurora — Piano 4: dati DXCC, lavorati, etichette e avvisi

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (esecuzione nativa). Passi con `- [ ]`.

**Goal:** I decode reali mostrano il paese e le etichette **nuovo DXCC, DXCC su nuova banda, nuova zona CQ, nuova griglia**, ricavate da `cty.dat` e dal registro `wsjtx_log.adi` di JTDX, con avvisi sonori per categoria. Voci 2–5 di `docs/mancante.md`.

**Architecture:** Crate `dxcc` (lettura di `cty.dat` e ricerca per prefisso più lungo). Modulo `worked` in `engine-jtdx` (lettura dell'ADIF, insiemi dei lavorati per banda). Estrazione di nominativo e griglia dal testo FT8. `engine-jtdx` arricchisce ogni decode; i QSO nuovi aggiornano subito i lavorati. L'interfaccia mostra paese ed etichette e suona gli avvisi (WebAudio).

**Spec:** `docs/2026-10-24-aurora-design.md` · `docs/fase0-parita-funzioni.md` (colori e avvisi di JTDX) · `docs/mancante.md`

## Global Constraints

- Rust edition 2024, niente `unwrap`/`expect` fuori dai test, errori come `Result`.
- I dati dell'utente (`cty.dat`, `wsjtx_log.adi`) si **leggono** da percorso configurato; non si copiano nel repository. Nei test solo frammenti di fantasia (nominativi e paesi inventati o di esempio).
- Se i file mancano o sono illeggibili Aurora parte lo stesso: solo l'etichetta CQ, con un avviso nel log.
- Gli avvisi sonori partono solo dopo un'azione dell'utente (regola dei browser) e si spengono con un interruttore.
- Nessun nominativo reale di terzi nelle fixture; nessun riferimento a strumenti di assistenza nel repository.

## Review Focus

1. Nominativi strani: portatili (`DL/AA1AA`, `AA1AA/P`), con prefisso lungo, hashati (`<...>`), minuscoli: mai un panico, `None` se non riconoscibili.
2. `cty.dat` con righe lunghe su più righe, alias con `(zona)` e `[ITU]`, chiamate esatte con `=`: il prefisso più lungo vince, la chiamata esatta batte il prefisso.
3. ADIF con campi mancanti, lunghezze in byte (UTF-8), maiuscole/minuscole, record senza `<EOR>` finale: nessun panico, record incompleti ignorati.
4. Un QSO appena registrato non deve mostrare subito dopo "nuovo DXCC" per la stessa stazione.
5. I file grandi (centinaia di migliaia di righe) si leggono una volta all'avvio senza bloccare il motore.

---

### Task 1: crate `dxcc`

**Files:** `core/crates/dxcc/{Cargo.toml,src/lib.rs}`, `tests/dxcc.rs`

**Interfaces:**
- `pub struct Entity { pub name: String, pub prefix: String, pub cq_zone: u8, pub itu_zone: u8, pub continent: String }`
- `pub struct DxccDb`; `DxccDb::parse(text: &str) -> Result<DxccDb, DxccError>`; `DxccDb::from_file(path: &Path) -> Result<DxccDb, DxccError>`; `fn lookup(&self, call: &str) -> Option<Lookup>` con `pub struct Lookup { pub entity: usize, pub cq_zone: u8, pub itu_zone: u8 }`; `fn entity(&self, index: usize) -> &Entity`; `fn len(&self) -> usize`.
- Ricerca: chiamata esatta (`=CALL`) prima, poi il prefisso più lungo; le zone possono essere sovrascritte da `(CQ)` e `[ITU]` sull'alias; nominativi con `/`: si prova prima la parte che sembra un prefisso (più corta) poi l'altra; suffissi `/P /M /MM /QRP /A` ignorati; maiuscole indifferenti; `<...>` e stringhe senza cifre → `None`.

- [ ] **Step 1: Test** (frammento di `cty.dat` di fantasia nel test): `parses_entities_and_aliases`, `longest_prefix_wins`, `exact_call_beats_prefix`, `zone_overrides_cq_and_itu`, `portable_and_prefixed_calls`, `suffixes_are_ignored`, `case_insensitive`, `unknown_and_hashed_calls_are_none`, `multiline_alias_lists`, `rejects_a_file_without_entities`.
- [ ] **Step 2: Run** `cargo test -p dxcc` — FAIL. **Step 3: Implementare**. **Step 4: Run** — PASS. **Step 5: Commit** `feat(dxcc): lettura di cty.dat e ricerca del paese`.

### Task 2: nominativo e griglia dal testo FT8

**Files:** `core/crates/engine-jtdx/src/text.rs`, test unitari nello stesso file

**Interfaces:** `pub struct Parsed { pub sender: Option<String>, pub grid: Option<String>, pub is_cq: bool }`; `pub fn parse_message(text: &str) -> Parsed`.
Regole: `CQ`/`CQDX`/`QRZ` (+ modificatori alfabetici di 2–4 lettere come `DX`, `NA`, `EU`, `POTA`, `TEST`) seguiti dal nominativo del mittente e da un locatore facoltativo; negli altri messaggi (`DEST MITT ...`) il mittente e' il secondo elemento; un elemento `<...>` non e' un nominativo; il locatore e' l'ultimo elemento `[A-R]{2}[0-9]{2}` (4 caratteri, maiuscolo).

- [ ] Test: `cq_plain`, `cq_with_modifier`, `cq_pota`, `reply_with_report`, `reply_with_grid`, `rr73_has_no_grid`, `hashed_sender_is_none`, `lowercase_and_extra_spaces`, `garbage_gives_empty`. RED → implementare → GREEN. Commit `feat(engine-jtdx): nominativo e griglia dal testo dei messaggi`.

### Task 3: lavorati dall'ADIF

**Files:** `core/crates/engine-jtdx/src/worked.rs`

**Interfaces:** `pub struct WorkedLog`; `WorkedLog::from_adif(text: &str, db: &DxccDb) -> WorkedLog`; `fn add_qso(&mut self, call: &str, grid: &str, band: &str, db: &DxccDb)`; `pub enum Novelty { NewDxcc, NewDxccBand, NewZone, NewGrid, Worked, Nothing }`; `fn novelty(&self, call: &str, grid: Option<&str>, band: &str, db: &DxccDb) -> Novelty`; `fn len(&self) -> usize` (QSO letti).
Precedenza: `NewDxcc` > `NewDxccBand` > `NewZone` > `NewGrid` > `Worked` (stessa chiamata gia' lavorata su quella banda) > `Nothing`. Le griglie contano sui primi 4 caratteri.

- [ ] Test: lettura ADIF (campi `<CALL:5>`, `<BAND:3>`, `<GRIDSQUARE:6>`, con tipo `<X:3:S>`, minuscole, UTF-8 in `COMMENT`), `novelty_precedence`, `grid_is_four_chars`, `worked_on_other_band_is_new_band`, `new_qso_updates_immediately` (Review Focus 4), `incomplete_records_are_ignored`, `file_without_eor_is_ok`. RED → GREEN. Commit `feat(engine-jtdx): registro dei lavorati dall'ADIF`.

### Task 4: etichette nel dominio, nel motore e nell'interfaccia

**Files:** `aurora-core/src/event.rs`, `engine-jtdx/src/{lib,map}.rs`, `app/src/api/schemas.ts`, `app/src/components/DecodeList.tsx`, `app/src/state/selectors.ts`

**Interfaces:** `DecodeTag` guadagna `NewDxccBand`, `NewGrid` (JSON `new_dxcc_band`, `new_grid`); `DecodedMessage` guadagna `country: Option<String>`. `JtdxEngine::with_enrichment(self, db: Arc<DxccDb>, worked: WorkedLog) -> Self`. Il tag e' `NewDxcc|NewDxccBand|NewZone|NewGrid` se applicabile; altrimenti `Cq` per i CQ; altrimenti nessuno.
Interfaccia: etichette `NUOVO DXCC`, `DXCC NUOVA BANDA`, `NUOVA ZONA`, `NUOVA GRIGLIA` (ambra), paese accanto al testo; il pannello "Chiamate CQ" diventa "CQ e novità".

- [ ] Test Rust: decode `CQ ...` di un paese mai lavorato → `NewDxcc` con `country`; stesso paese su altra banda → `NewDxccBand`; QSO registrato dopo → il decode successivo e' `Worked`/`Cq`; senza dati → solo `Cq`. Test TS: schema accetta i nuovi tag e `country` nullo, badge e paese mostrati, `cqDecodes` include le novita'. RED → GREEN. Commit `feat: etichette nuovo DXCC, zona e griglia sui decode reali`.

### Task 5: collegamento nel binario

**Files:** `aurora/src/lib.rs`, `.env.example`, `scripts/avvia-sviluppo.ps1`

`AURORA_CTY_PATH`, `AURORA_WORKED_ADIF` (facoltativi); caricamento all'avvio (se manca o e' illeggibile: avviso e si prosegue); con `-Jtdx` lo script usa i file di JTDX se esistono. Test: config e avvio con file di fantasia; avvio con file mancante.

- [ ] RED → GREEN. Commit `feat(aurora): carica cty.dat e i lavorati all'avvio`.

### Task 6: avvisi sonori

**Files:** `app/src/lib/alerts.ts` (+test), `app/src/components/AlertSwitch.tsx`

`alertKind(tag): 'dxcc' | 'band' | 'zone' | 'grid' | 'cq' | null`; `newAlerts(previousIds: Set<number>, decodes): Alert[]` (solo decode nuovi, al massimo uno per categoria e per ciclo); suoni sintetizzati con WebAudio (toni distinti per categoria), interruttore e volume ricordati; nessun suono al primo caricamento.

- [ ] Test sulla logica pura (classificazione, nessun avviso per decode gia' visti, un avviso per categoria, silenzio al primo carico). RED → GREEN. Commit `feat(app): avvisi sonori per le novità`.

### Task 7: documentazione e verifica

Aggiornare `docs/mancante.md` (voci 2–5), guida, rapporto `piano4-rapporto.md`, versione 0.4.0; `cargo test`, `clippy`, `npm test`, `npm run build`; ricerca di nominativi reali e parole vietate.

## Self-review

Copertura: voci 2 (Task 1), 3 (Task 3), 4 (Task 4), 5 (Task 6) di `mancante.md`. Fuori: `HighlightCallsign` verso JTDX, wanted list e notifiche push, lettura del log di Log4OM2. Coerenza: `DxccDb::lookup` (Task 1) usato da `WorkedLog` (Task 3) e dal motore (Task 4); `Parsed` (Task 2) alimenta `novelty`.
