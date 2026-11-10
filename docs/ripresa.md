# Ripresa del lavoro (ultimo aggiornamento: 2026-11-10 mattina)

Questo file dice a chi riprende il lavoro, in una sessione nuova, **a che punto e' Aurora** e **cosa fare per primo**. Va aggiornato a ogni passaggio di sessione.

## Stato al 2026-11-10 pomeriggio (0.11.226 pubblicata)

- **0.11.226** (12:19 UTC): al primo avvio con un installatore vecchio prima la locandina dell'aggiornamento e poi la guida iniziale; Finestra RX con colonne larghe quanto il contenuto (messaggio mai tagliato); cartolina del tray senza i passi.

- **Versioni del giorno**: 0.11.220-0.11.225. Ultime: 0.11.223 eta' dei messaggi e messaggi nuovi in fondo, Filtri divisi per elenco, Automazioni che salvano «nuovo nominativo» e «griglia su nuova banda», agente del tray nella lingua di Aurora; 0.11.224 PTT Hamlib con risposte in ritardo, chiusura ordinata del TCI; 0.11.225 «CQ e novita'» (bandierina, nominativo, paese, due colori chiari, via dopo 15 minuti, scorrimento), riquadro RX che in trasmissione diventa «STOP TX (Esc)», Auto con conferma del modo reale sulla Stazione, suono del QSO a log a scelta, mappe che seguono la banda della radio, OmniRig mancante detto con il link.
- **Release**: si creano con --latest=false e diventano «ultime» da sole dopo installatore e firma (passo di windows.yml): provato con 0.11.223-0.11.225.
- **cty.dat**: copia di riserva in desktop/cty-riserva usata quando country-files.com non risponde.
- **Rapporti su Cloudflare**: letti e cancellati (errori e pacchetti) il 2026-11-10 pomeriggio. Da guardare: blocco della finestra sulla 0.11.224 (i3-1005G1, 36 s dopo «finestra in primo piano»).
- **Richieste degli utenti** (Luc ON7KEC e altri): le decide il proprietario, non vanno fatte tutte; prima si propongono.
- **In attesa**: Aurora in giapponese e cinese, da decidere solo se arrivano utenti dagli annunci (gli annunci si scrivono in italiano, inglese, giapponese e cinese).
## Stato al 2026-11-10 mattina (0.11.222 pubblicata)

- **0.11.222** (08:39 UTC): l'aggiornamento non resta piu' bloccato su «la release non ha il pacchetto di installazione con la sua firma».
- **Release mai vuota per le copie installate** (dopo l'errore «la release non ha il pacchetto di installazione con la sua firma» della 0.11.221 e 0.11.222): la release si crea SEMPRE con `gh release create vX.Y.Z --latest=false` e senza file; il lavoro `Windows` allega installatore e `.sig` e solo allora la segna come ultima (passo «Release segnata come ultima solo con installatore e firma» di `windows.yml`). Mai creare una release come ultima senza installatore e firma. Se il lavoro Windows fallisce, la release resta non ultima e le copie installate restano sulla versione precedente, completa. In piu', dalla 0.11.222, Aurora richiede la release a GitHub se al momento di scaricare manca il pacchetto (`updater::prepare_locked`).

- **0.11.221** (08:03 UTC): solo dipendenze aggiornate (`cargo update` nei due workspace; interfaccia con react-query 5.107, zustand 5.1, vite 8.3.7, vitest 5.1; servizio dei rapporti con wrangler 4.159 e workers-types 5). jsdom resta alla 30 e typescript del servizio alla 5.9.

- **0.11.220** (07:52 UTC): «Ripristina Hamlib» su Windows, trasmissione che riparte da sola se la scheda audio della radio manca all'avvio, finestre a parte aperte fuori dal thread della finestra, decodifica con un processore libero, primi messaggi dopo il cambio di banda, voce della Guida «Che computer serve», tipo di computer nei rapporti.
- **Installatore da GitHub**: la release si crea senza file; il lavoro `Windows` (`windows.yml`) compila l'installatore, lo firma con il segreto `AURORA_TIMBRO_AGGIORNAMENTI_AURORA_KEY` e allega `Aurora_x.y.z_x64-setup.exe`, `.sig`, `Aurora-Setup.exe` e `SHA256SUMS.txt`. Sul PC dell'ufficio la chiave di firma non c'e': `scripts/rilascio-windows.ps1` li' non si puo' usare.
- **Identita' dei commit**: nel repository `user.email` deve essere l'indirizzo noreply di GitHub; il 2026-11-10 autore e committer dei commit dopo `ef9510e9` sono stati riscritti (push forzato di main).
- **Prossimo passo**: prove del proprietario al DARC FT4 (2026-11-10 19:00 UTC); controllare i rapporti dopo la 0.11.220 (blocchi delle finestre a parte, PC lenti con il tipo di computer); poi i lavori di `docs/tasks.md`.

## Stato al 2026-11-09 notte (0.11.219 pubblicata)

- **Versioni del giorno**: 0.11.205-0.11.218. Ultime: 0.11.215 nuovo stato in Auto e pagina Contest ordinata, voce degli avvisi rifatta con Kokoro (licenza Apache 2.0, `scripts/genera-voce-kokoro.py`; le frasi di Edge/Ava sono state tolte); 0.11.216 correzioni dai rapporti di Cloudflare (PowerShell con percorso intero per l'installatore, niente falsi blocchi/panic in chiusura `crash::exiting`, rapporti senza «radio non collegata»); 0.11.217 paesi non confermati come DXCC nuovo (`newDXCCUnconfirmed`), Mappa DX con banda in uso, «solo chi mi sente», linea rossa in TX; 0.11.218 Contest da tastiera (F1 Auto CQ, F2-F4, Esc), «Come funziona», avvisi sopra le Impostazioni, ricarica che resta sulla pagina.
- **0.11.219** (23:00): mappa del pannello Log semplificata (sempre banda in uso, riga di comandi corta) e passaggio automatico al Log quando un QSO va a log; selettore banda/frequenza piu' leggibile; Contest spento pulito; correzione della pagina vuota ricaricando (mappa di calore su tela larga 0) e `PageGuard` contro gli errori di pagina; alleggerimenti (righe visibili prima, `content-visibility`, misure ferme con la finestra nascosta). Tentativo di tenere viva la Stazione nascosta **tolto**: dava la pagina nera (non riprovarlo senza prove serie).
- **Contest**: nel log del contest entrano solo i QSO avviati dalla pagina Contest (intestazione `x-aurora-origine: contest` su reply e CQ) e fatti durante la gara; chi non fa il contest entra a 0 punti (errore, fuori dal Cabrillo). Da provare dal vivo al DARC FT4 del 2026-11-10, 19:00 UTC.
- **Conteggio delle persone**: `rapporti::codice_persona` (SHA-256 del nominativo sul PC) nel segnale `/v1/attivo`; sul Worker HMAC con `PERSONA_SEGRETO` (impostato) e colonne `persona` su D1 (aggiunte). Rapporti degli errori svuotati il 2026-11-09 dopo la lettura.
- **Riordino**: in `D:\progetti` restano `Aurora` e `Aurora-archivio` (LEGGIMI dentro); copie di lavoro git chiuse (i rami restano), compilazioni vecchie di `D:uild` cancellate.
- **Test**: i test del motore toccati passano; nell'interfaccia 39 test falliscono gia' da prima del 2026-11-09 (elenco confrontato con la 0.11.212), da sistemare a parte.
- **In attesa**: issue #9 (TCI «Audio silent», avllucken: chieste versione e pacchetto); risposte di Luc sulle sue richieste; frase «Virus free» dei README da rivedere (VirusTotal non si fa piu').
- **Blocchi della finestra**: su Cloudflare arrivano «programma bloccato (finestra)» da altre installazioni subito dopo ogni release (uno «finito dopo 29 s»): non spiegati, servono un pacchetto di assistenza o il minidump.
- **Prossimo passo**: prove del proprietario al DARC FT4 (2026-11-10 19:00 UTC); controllare i rapporti dopo la 0.11.219; poi i lavori di `docs/tasks.md`.

## Stato al 2026-11-08 mattina (passaggio ad altro account)

- **Lavoro solo in locale**: il proprietario ha chiesto di NON pubblicare (niente push, tag, release, sito) finche' non dice «pubblica». Sul ramo `main` locale ci sono circa 17 commit non pubblicati dopo la 0.11.194 (versione nei file: 0.11.195).
- Installatore locale pronto: `D:\build\Aurora-0.11.1\dist-release\Aurora_0.11.195_x64-setup.exe` (09:20). Il proprietario lo installa e prova; la grafica la vede dal vivo copiando `app/dist` in `%LOCALAPPDATA%\Aurora\dist`.
- Fatto oggi: correzione del cambio banda che rimbalzava (0.11.194 pubblicata), JT4 A-G, Q65-30A, MSK144 + MSK40 + tempo reale, bande da 2200 m a 24 GHz, selettore delle frequenze (pulsanti delle bande tolti), accesso senza codice in casa di base, frequenza principale all'avvio, «R + locatore» come conferma, tasti del display che non diventano grigi, zoom del display opaco sopra il display fisso, spot a DXSummit nel registro. Dettagli in `memory.md` (sezione «Modi e bande nuovi») e `TODO.md`.
- Compilare con 9 processi (`jobs = 9` in `~/.cargo/config.toml`): con 18 la memoria finiva e Aurora si chiudeva.
- **Prossimo passo**: aspettare le prove del proprietario; poi Q65 (periodi e sottomodi selezionabili, media, AP, EME), FST4/FST4W e WSPR, sistemare le prove che falliscono gia' (elenco in `TODO.md`).
- File non tracciati da lasciare: `app/tsconfig.tsbuildinfo` (generato) e `docs/report-cloudflare-2026-11-06.md` (il proprietario non ha ancora deciso se committarlo).

## Prompt da incollare nella sessione nuova

```text
Riprendi il lavoro su Aurora nella cartella D:\progetti\pc\Aurora.
1. Leggi AGENTS.md, docs/ripresa.md, memory.md (sezione «Come riprendere»), TODO.md e docs/tasks.md.
2. Allineati: git fetch, controlla main e i rami in corso elencati sotto.
3. Dimmi in breve a che punto siamo, poi riprendi dal punto «Prossimo passo» di docs/ripresa.md.
Regole: niente riferimenti all'assistente nei file e nei commit; mai il locatore vero; il push e le release sono autorizzati solo quando il proprietario dice «pubblica» (dal 2026-11-08 si lavora in locale);
le prove dal vivo con la radio le faccio io.
```

## Stato

- **Ultima versione**: **0.11.191** (2026-11-07). Dal 2026-11-07 sera (risposte alle issue e agli utenti): 0.11.189 dimensione del testo (Impostazioni > Generali, Ctrl +/-/0, `lib/uiScale.ts`) e Impostazioni che non si chiudono con Salva (issue #3 e #4, chiuse con #1 e #2); 0.11.190 pagina Log con «Aggiungi QSO», «Elimina selezionati», «Svuota il log» (`POST`/`DELETE /api/v1/log/qsos`) e Guida di BBLogger rifatta dai PDF ufficiali; 0.11.191 bande senza CAT (`ManualDialHook`, `frequenza-a-mano.txt`). Da provare dal vivo: tutte e tre. Fino alla 0.11.188 il lavoro e' nel CHANGELOG.
- Prima, 0.11.151 (2026-11-05): pubblicata dal PC (Mappa DX come GridTracker2, finestre a parte senza cornice, display su un altro schermo, pagina Log, orologio con correzione a un clic). Su `main` dopo la 0.11.151, non ancora rilasciati: ingresso e uscita audio che si riprendono da soli se la radio sparisce durante l'uso, guardiano dei blocchi piu' preciso, correzioni Linux (Pwr, aggiornamenti, OmniRig), storico permanente delle installazioni nel servizio dei rapporti.
- Versioni precedenti, del 2026-11-04:
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
  - 0.11.121: OmniRig che non risponde spiegato in chiaro (`explain_omnirig` in `tx-native/src/ptt.rs`); DX Cluster nascosto sul telefono se spento.
  - 0.11.120: telefono (QR senza adattatori virtuali, `lan::lan_ipv4s_named`; `GET /api/v1/lan/check` e `POST /api/v1/lan/allow` per rete «Pubblica» e firewall, `NetworkChecklist.tsx`), finestra che si ricollega se il flusso resta muto 8 s (`api/stream.ts`), Novita' compatte, quasi quadrato a 120, splash una volta per avvio (sessionStorage), controllo della versione nuova 8 s dopo l'avvio.
  - 0.11.119: porta del CAT scrivibile (indirizzi di rete per Hamlib NET rigctl, avviso se la condivisione usa la stessa porta), paese leggibile nel pannello CQ, quasi quadrato sopra la testata (createPortal).
  - 0.11.118: pannelli da togliere in Impostazioni > Finestre, import ADIF fino a 256 MB.
  - 0.11.117: «gia' lavorato» solo per nominativi nel log, tilde dello S-meter stimato.
  - 0.11.116: conteggio anonimo delle installazioni attive (`Rapporti::segnala_attivo`, una volta al giorno, `count_active` acceso di fabbrica; Worker `POST /v1/attivo`, `GET /v1/attivi` con `ADMIN_TOKEN`, tabella D1 `attivi`).
  - 0.11.115: anteprime dei display estesi a 640x70 nel menu Radio.
  - 0.11.114: locandina della versione nuova all'apertura (`UpdatePoster.tsx`), «Piu' tardi» fino alla versione dopo.
  - 0.11.106: decodifica FT8 con tempo massimo (`ft8::decode::Options::deadline`, 11 s dal motore).
- **Rapporti degli errori**: letti e cancellati il 2026-11-04 (R2 e D1 vuoti): da li' sono nate la 0.11.105 e la 0.11.106; i casi ancora aperti sono in `TODO.md` («Dai rapporti degli errori»).
- **Storico pulito**: il 2026-11-02 lo storico di GitHub e' stato riscritto (git filter-repo, push forzato di main, gh-pages e tag) per togliere ogni riferimento all'autore delle tabelle del codice polare dell'UltraFOX. Regola: non citarlo mai (vedi memoria privata). Copia dello storico vecchio solo in locale: `D:\progetti\Aurora-backup-prima-pulizia-qpc.bundle`.

## Lavoro in corso

- **2026-11-05, non ancora rilasciato** (su `main`, voci in CHANGELOG «Non rilasciato»; ultima versione pubblicata 0.11.131):
  - audio: cattura con `cpal` feature `realtime` (MMCSS), buffer riusati nel thread audio, rapporto dei blocchi persi con causa (`Xrun`), minuti, blocchi al minuto, tipo di ingresso, Hz, canali, formato (`audio-spectrum/src/capture.rs`);
  - avvisi sullo schermo dal motore: `aurora_core::notice::notice(id, detail)` → `Event::Notice` → `DiagnosisBar` (`audio_loss`, `no_radio`, `omnirig`, `slow_pc`; lo stesso avviso non si ripete prima di 2 minuti);
  - «Verifica tutto» in Impostazioni › Assistenza (`app/src/components/CheckAll.tsx`): stazione, radio, audio, orologio, telefono;
  - OmniRig: terzo tentativo del cambio frequenza; porta del server occupata: il messaggio dice il programma (Windows);
  - nominativo o locatore da compilare: avviso, non piu' errore nei rapporti;
  - AGCc anche in FT4 (`ft8::sync::agc_compensate_len`, `ft4::decode::Options::agc`);
  - decodifica lenta: `engine-native/src/slow.rs` (4 periodi su 10 oltre il 70% → rapporto `engine_native::lento` e avviso `slow_pc`);
  - riquadro «Radio non collegata» dentro la riga in alto, con spazio sopra e sotto.

- Prototipi grafici in `docs/prototipo-*.html` e `docs/mockup-display-*.html` (anche sul Desktop del PC di casa, cartella `Aurora-prototipi`): il proprietario vuole sempre un prototipo prima del codice grafico.
- Rapporti su Cloudflare letti alle 18:00 del 2026-11-04: 16 installazioni attive; un pacchetto di assistenza (OmniRig non in linea). Le prove con il core di prova vanno fatte su un'altra porta (8799) se Aurora e' aperta: sulla 8787 il core di prova fallisce e manda un falso rapporto «porta in uso».
- Nessun ramo aperto. Il proprietario deve rimettere il PTT della sua stazione su RTS / COM9 in Impostazioni > Radio: dalla 0.11.102 non viene piu' da JTDX.ini.
- **Servizio dei rapporti** online: `https://aurora-rapporti.tizianocassone.workers.dev` (cartella `servizi/rapporti/`, R2 `aurora-rapporti`, D1 `aurora-rapporti`, segreto `ADMIN_TOKEN` impostato dal proprietario). Per leggerli: `npx wrangler d1 execute aurora-rapporti --remote --command "SELECT ..."` dalla cartella del servizio; i pacchetti si scaricano con `wrangler r2 object get aurora-rapporti/<chiave> --remote`.
- **Linux**: versione dallo stesso codice, non provata dall'autore e senza assistenza; `scripts/compila-linux.sh`, AppImage e `.deb` allegati da GitHub Actions a ogni release.
- Restano da fare: «Prova» per eQSL, PSK Reporter, DXSummit, NG3K, propagazione, NTP, UDP/TCP; NAME e QTH per i QSO registrati a mano; CAT4OM; pagina dei rapporti nell'installatore; tema del telefono separato da quello del PC (decisione del proprietario).
- I test li lancia il proprietario: prima di pubblicare solo `tsc` e `cargo check`.

## Prossimo passo

0. **2026-11-06, su `main` non rilasciato**: AP in FT4 e FT8 collegata al motore, elenco dei nominativi conosciuti, controllo del locatore (dettagli in `memory.md`, «Motori: AP…»). Dal PC di casa con REA: confronto sui WAV Aurora/JTDX e prova dal vivo di queste tre cose (voci in `TODO.md`).

0. Pubblicare la versione con il lavoro del 2026-11-05 (dal PC), poi guardare su Cloudflare se i blocchi audio persi calano e da che tipo di ingresso arrivano.
0. JT65/JT9 sul segnale debole e SWL (`NFILT2`, `ldeepsync`): da fare dal PC con JTDX e REA, non a memoria.
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

- Codice: `D:\progetti\pc\Aurora` (repository pubblico github.com/wifi75/Aurora). Compilazione dell'installatore: `D:\build\Aurora-0.11.1` (cartella di rilascio, si porta sul commit con `git checkout --detach`).
- Analisi di WSJT-X 3: `D:\build\analisi-wsjtx` (sorgenti e binari), `docs/analisi-wsjtx-3.md`.
- Chiave di firma degli aggiornamenti: nei segreti di GitHub (installatori compilati da GitHub); copia sul PC di casa in `%USERPROFILE%\.aurora-signing\aurora.key`, solo per prove in locale.
- Le copie di lavoro `D:\build\wt-*` non elencate sopra sono gia' unite a `main` e si possono cancellare.
