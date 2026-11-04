# Ripresa del lavoro (ultimo aggiornamento: 2026-11-04)

Questo file dice a chi riprende il lavoro, in una sessione nuova, **a che punto e' Aurora** e **cosa fare per primo**. Va aggiornato a ogni passaggio di sessione.

## Prompt da incollare nella sessione nuova

```text
Riprendi il lavoro su Aurora nella cartella D:\progetti\Aurora.
1. Leggi AGENTS.md, docs/ripresa.md, memory.md (sezione «Come riprendere»), TODO.md e docs/tasks.md.
2. Allineati: git fetch, controlla main e i rami in corso elencati sotto.
3. Dimmi in breve a che punto siamo, poi riprendi dal punto «Prossimo passo» di docs/ripresa.md.
Regole: niente riferimenti all'assistente nei file e nei commit; mai il locatore vero; il push e le release sono autorizzati;
le prove dal vivo con la radio le faccio io.
```

## Stato

- **Ultima versione**: **0.11.117** (2026-11-04, pubblicata; «gia' lavorato» solo per nominativi nel log, tilde dello S-meter stimato). Le versioni del 2026-11-04:
  - 0.11.102: Aurora indipendente (non legge piu' nulla da JTDX o WSJT-X; `cty.dat` e Hamlib dentro l'installatore), guida al primo avvio (`app/src/components/FirstRun.tsx`, compare finche' manca il nominativo), servizi pubblici accesi di fabbrica, BBLogger, prima versione Linux.
  - 0.11.103: modo, periodo e frequenza si pubblicano anche senza ingresso audio (`engine_native::publish_status`, avviato dal core prima del motore).
  - 0.11.104: copia delle impostazioni (Impostazioni > Assistenza, `core/crates/aurora/src/backup.rs`: zip sul Desktop, ripristino in `<dati>\ripristino` applicato al prossimo avvio da `backup::apply_pending`); suono dello splash (`--autoplay-policy=no-user-gesture-required` in `desktop/src-tauri/src/lib.rs`).
  - 0.11.105: schede audio ritrovate anche con nomi vecchi (`match_device_name` / `is_default_name` in `audio-spectrum` e `tx-native`: `\xNN`, «2- », scheda tra parentesi, «Default Input Device»); OmniRig assente detto in chiaro.
  - 0.11.108: con due schede USB uguali vince il nome esatto (`match_device_name`); confermato dal vivo.
  - 0.11.109: «Clicca per configurare» al posto di display e log vuoti (`SetupCta`).
  - 0.11.110: niente decodifiche false nel rumore (picco del periodo < 0,02: a7/a8/a priori spenti; < 0,0005: periodo vuoto).
  - 0.11.111: avvisi della stazione in schede galleggianti in basso a destra (`DiagnosisBar`), i pannelli non si spostano.
  - 0.11.112: log vuoto -> Impostazioni > Programmi collegati con import ADIF e log esterni UDP nella stessa pagina.
  - 0.11.113: sei display nuovi (`ModernDisplays.tsx`): estesi 640x70 `icdual`/`icblue`/`ledline`, quasi quadrati 330x140 `icblueq`/`icblackq`/`yaesuq` (fissi nell'angolo, icone della testata piccole con `body.rd-square`); tutti i display estesi alti 70; misure accanto alle scelte.
  - 0.11.116: conteggio anonimo delle installazioni attive (`Rapporti::segnala_attivo`, una volta al giorno, `count_active` acceso di fabbrica; Worker `POST /v1/attivo`, `GET /v1/attivi` con `ADMIN_TOKEN`, tabella D1 `attivi`).
  - 0.11.115: anteprime dei display estesi a 640x70 nel menu Radio.
  - 0.11.114: locandina della versione nuova all'apertura (`UpdatePoster.tsx`), «Piu' tardi» fino alla versione dopo.
  - 0.11.106: decodifica FT8 con tempo massimo (`ft8::decode::Options::deadline`, 11 s dal motore).
- **Rapporti degli errori**: letti e cancellati il 2026-11-04 (R2 e D1 vuoti): da li' sono nate la 0.11.105 e la 0.11.106; i casi ancora aperti sono in `TODO.md` («Dai rapporti degli errori»).
- **Storico pulito**: il 2026-11-02 lo storico di GitHub e' stato riscritto (git filter-repo, push forzato di main, gh-pages e tag) per togliere ogni riferimento all'autore delle tabelle del codice polare dell'UltraFOX. Regola: non citarlo mai (vedi memoria privata). Copia dello storico vecchio solo in locale: `D:\progetti\Aurora-backup-prima-pulizia-qpc.bundle`.

## Lavoro in corso

- Prototipi grafici in `docs/prototipo-*.html` e `docs/mockup-display-*.html` (anche sul Desktop del PC di casa, cartella `Aurora-prototipi`): il proprietario vuole sempre un prototipo prima del codice grafico.
- Nessun ramo aperto. Il proprietario deve rimettere il PTT della sua stazione su RTS / COM9 in Impostazioni > Radio: dalla 0.11.102 non viene piu' da JTDX.ini.
- **Servizio dei rapporti** online: `https://aurora-rapporti.tizianocassone.workers.dev` (cartella `servizi/rapporti/`, R2 `aurora-rapporti`, D1 `aurora-rapporti`, segreto `ADMIN_TOKEN` impostato dal proprietario). Per leggerli: `npx wrangler d1 execute aurora-rapporti --remote --command "SELECT ..."` dalla cartella del servizio; i pacchetti si scaricano con `wrangler r2 object get aurora-rapporti/<chiave> --remote`.
- **Linux**: versione dallo stesso codice, non provata dall'autore e senza assistenza; `scripts/compila-linux.sh`, AppImage e `.deb` allegati da GitHub Actions a ogni release.
- Restano da fare: «Prova» per eQSL, PSK Reporter, DXSummit, NG3K, propagazione, NTP, UDP/TCP; NAME e QTH per i QSO registrati a mano; CAT4OM; pagina dei rapporti nell'installatore; tema del telefono separato da quello del PC (decisione del proprietario).
- I test li lancia il proprietario: prima di pubblicare solo `tsc` e `cargo check`.

## Prossimo passo

1. Opzioni del display lette da Hamlib per ogni modello (`TODO.md`, «Display grandi in testata»).
2. Provare dal vivo i sei display, soprattutto i quasi quadrati in testata, e la locandina alla prossima versione.
3. Guardare i rapporti e le installazioni attive su Cloudflare (`SELECT ... FROM attivi` o `GET /v1/attivi`). Download al 2026-11-04: 266 `Aurora-Setup.exe` (prime installazioni), 22 Linux, 374 exe con versione = aggiornamenti automatici.
4. Le voci di `TODO.md` «Dai rapporti degli errori»: avvio fallito con WebView2 su Windows 11 Dev, porta 8787 occupata, campioni audio persi.
5. Poi le voci aperte di `docs/tasks.md` (TCI, Q65, MSK144, pulsanti dei modi, QMAP).

## Da provare dal vivo (proprietario)

Del 2026-11-04: guida al primo avvio su un PC pulito, copia e ripristino delle impostazioni, suono dello splash, decodifica sul PC lento con l'IC-7300 (audio con due schede USB uguali confermato dal vivo con la 0.11.108).
Scheda DXpedition con mappa (corretta nella 0.11.65 e 0.11.67), lente sul display (2x), pannello QSO con colori (viola acceso, rosso per Abilita Tx/AutoTX), doppio clic su spot FT4/FT8 che cambia modo, pagina Log. 
Pannello Comandi (DX scritto a mano, rapporto, Hold Tx Freq, tipo di CQ, Tune e Log QSO), pannello QSO in due colonne, Hound automatico e FOX, Wait and Pounce, chiusura con RR73 dopo un R+rapporto al Tx1, nuovo menu Impostazioni e pagina Log, barre Rx/Tx, display Yaesu/Icom, colore DXpedition nel cluster, a7/a8, stop per ROS alto, invii a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP.

## Dove stanno le cose

- Codice: `D:\progetti\Aurora` (repository pubblico github.com/wifi75/Aurora). Compilazione dell'installatore: `D:\build\Aurora-0.11.1` (cartella di rilascio, si porta sul commit con `git checkout --detach`).
- Analisi di WSJT-X 3: `D:\build\analisi-wsjtx` (sorgenti e binari), `docs/analisi-wsjtx-3.md`.
- Chiave di firma degli aggiornamenti: `%USERPROFILE%\.aurora-signing\aurora.key` (fuori dal repository).
- Le copie di lavoro `D:\build\wt-*` non elencate sopra sono gia' unite a `main` e si possono cancellare.
