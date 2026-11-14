# Ripresa del lavoro (ultimo aggiornamento: 2026-11-14)

Questo file dice a chi riprende il lavoro, in una sessione nuova, **a che punto e' Aurora** e **cosa fare per primo**. Va aggiornato a ogni passaggio di sessione.

## Stato al 2026-11-14 notte (0.11.255 pubblicata, sito aggiornato)

- **0.11.255** (2026-11-14T14:08:57Z): contest nei modi testuali: segni di BIGScan dal contest attivo (`logStatus.ts`, `useContestLog.ts`, `contest_valuta`: DUPE solo nella finestra della gara, NEW/B/M/BM/MULT/DXCC; B4 fuori gara), `<QSONR>` in gara = ultimo numero del contest + 1 (`contestNextSerial`), `POST /log/qsos` con `X-Aurora-Origine` e `exch_sent`/`exch_rcvd` (prima i QSO dei modi testuali entravano nel contest per caso), macro `<IGNOREBEFORE>`/`<PERIOD>` (`logWindow.ts`); BIGScan RTTY sintonia fine ±15 Hz nei canali (`rtty_multi.rs`, ritardo 12 -> 9 s; banco allineato a `band.rs`: interruzioni 5 -> 7); display della radio acceso se la radio risponde (`RadioDisplayHeader.tsx`, #55); HRDLOG.NET nominativo maiuscolo e codice senza spazi (`upload.rs`); riga fra RX e TX (`maniglie.css`); Guida aggiornata.
- Prove fallite su BIGScan (fondo locale ±300 Hz, picchi ±1, storia 2,5 s, soglia -0,35): nessun miglioramento, tolte. Il riferimento a canale singolo del banco mescola le stazioni vicine: va reso piu' preciso prima di riprovare le deboli (RZ3Z, UC0A).
- Da fare: risposta a Luc (#55) e a F1DSZ (HRDLOG.NET) date al proprietario da mandare; icone Pausa e Cerca nella barra RX; registrazione CW per il banco.

## Stato al 2026-11-14 sera (0.11.254 pubblicata, sito aggiornato)

- **0.11.254** (2026-11-14T12:39:20Z): Aurora BIGScan piu' affidabile: CQ solo per il nominativo accanto a CQ/TEST/QRZ (`calls.rs`, `cq_count`), sostituisce un CQ solo un altro CQ letto 2 volte, rinfresco con pezzi del nominativo (`partial_sighting`, 180 s per i CQ), canali RTTY aperti piu' a lungo e memoria dei nominativi per frequenza (`CallCache`, 240 s), CQ mai sfrattato, radio spostata di poco = frequenze ricalcolate (`Msg::Shift`, `band_follow`); la stazione CQ letta nella finestra RX o nelle finestre RX in piu' compare sempre (`cqCallOf`); doppio clic su una stazione, segni NEW/B4 dal log (`logStatus.ts`). Banco di prova `tests/bigscan_misura.rs` (`AURORA_BIGSCAN_WAV`, `AURORA_BIGSCAN_MODE=cw`): su `tmp30515.wav` interruzioni 37 -> 13, solo banda 57 -> 17, ritardo mediano 7 -> 12 s. Macro: `<TX>` senza `<RX>` resta aperta, `<RXANDCLEAR>` pulisce a fine trasmissione, `<TXE>`/`<RXE>`, `<QSONR>` progressivo a 3 cifre (`contestSerial.ts`), `<NRS>`/`<NRR>`. Barra sopra la finestra RX (Copia, Salva, Pulisci); maniglie tutte uguali (`maniglie.css`).
- Prossimo: stazioni deboli accanto alle forti (RZ3Z, UC0A mai trovate: fondo locale ±300 Hz in `rtty_multi.rs`, correlazione su 2-3 s, toni vicini, AFC dei canali), ritardo 12 s da riportare sotto 7; icone Pausa e Cerca nella barra RX; registrazione CW per il banco.

## Stato al 2026-11-14 pomeriggio (0.11.253 pubblicata, sito aggiornato)

- **0.11.253** (2026-11-14T11:28:30Z): finestre RX in piu' (fino a 4, `PUT /api/v1/digi/extra-rx`, `rx_text` con `rx_id`, `ExtraRxWindow.tsx`, indicatori trascinabili) e menu del tasto destro sulla cascata; Aurora BIGScan di base solo chi chiama CQ (`radarAnswering`), ruoli dei contest (dopo TU, prima di TEST/QRZ), stazioni non attive tolte (`stale`/`gone`, `SUPERSEDED_S` 10 s, 60 s, 3 min), ripulitura al cambio banda o di 50 Hz (`BAND_CLEAR_HZ`), interruttore ON/OFF e Pulisci, etichette col colore della stazione; RTTY: caratteri uno alla volta, a capo dopo 1,5 s di pausa (`RX_GAP_MS`), a capo persi recuperati, AGC confermato dal rumore, banda 200-4000 Hz; clic su spot RTTY/PSK porta alla pagina dei modi testuali (`lib/spotMode.ts`); display «RADIO NON COLLEGATA» in testo (#55); Informazioni in tre colonne.
- In attesa: seconda registrazione RTTY del proprietario (`Desktop	mp30515.wav`, ~110 MB, era ancora aperta da un altro programma); testi dell'altro programma per il confronto; registrazioni CW vere.

## Stato al 2026-11-14 (0.11.252 pubblicata, sito aggiornato)

- **0.11.252** (2026-11-14T09:57:44Z): **Aurora BIGScan** (CW e RTTY: elenco delle stazioni sicure a destra delle finestre RX/TX, una riga per nominativo, colore per stazione, colonne spostabili, maniglia; `RadarList.tsx`, `lib/digiBand.ts` `radarStations`, `skim/rtty.rs`, `band_rtty`), in primo piano nelle Novita' (`featured`). CW di tutta la banda: rumore non ingannato da due vicine (`SIDE_OCCUPIED` 2,5; 30 stazioni 23 -> 30/30), recupero degli ultimi 2,5 s, frammenti e letture di un carattere nascosti (`shadows`, `shadow_across`), 64 ipotesi sui canali forti. RTTY: cancello interno a squelch spento (rumore 182 -> 18 car/min), AFC dalla fase (aggancio < 1 s fino a 50 Hz), riscalatura dopo un salto di guadagno. Niente piu' PowerShell (SetupAPI, IP Helper, WLAN/NLM/Firewall COM, registro, copia di aiuto `--applica-aggiornamento` con `aggiornamento.log`).
- Da provare con la radio: tutto, in particolare aggiornamento senza PowerShell, «Consenti Aurora», RTTY accanto all'altro programma sul file del proprietario (`tmp12856.wav`, 2 stazioni a 1976 e 2806 Hz di centro).
- Prossimo: confronto RTTY con l'altro programma sui suoi testi (i due file di testo forte e debole), fading veloce RTTY (confermare il calo sui canali del rumore), registrazioni CW vere.

## Stato al 2026-11-14 (0.11.251 pubblicata, sito aggiornato)

- **0.11.251** (2026-11-13T23:07:29Z): Club Log col token personale dell'API REST v1 (`ClublogToken`, `POST /api/v1/qso` con Bearer, prova con `GET /me/usage`; vecchio `realtime.php` se manca il token); messaggi di HRDLOG.NET tradotti; frasi del registro delle decisioni tradotte (continente escluso, Abilita Tx riacceso, stato nuovo); VFO B delle Yaesu senza `get_vfo_info` letto da `get_split_freq` (FT-2000, non provato con la radio); impostazioni allineate a sinistra sotto le etichette; doppione ADIF tolto da Programmi collegati. Compilazione: `tsc --singleThreaded` e `CARGO_BUILD_JOBS=4`, il PC finiva la memoria. Non provato il token con Club Log vero.
- Da fare: pagina Informazioni (proposte A/B/C in attesa); PowerShell nascosto sostituibile con API di Windows in 4 punti (proposta, non chiesta); risposte a #47, #52, #53 date all'utente.

- **0.11.250** (2026-11-13T19:09:28Z): spot dal cluster telnet a scelta al posto di DXSummit (`dx_cluster::SpotVia`, `send_spot` aspetta la ripetuta `DX de <noi>`), nome di accesso e password per server (password mai rimandata all'interfaccia, `has_password`); azimut e distanza nel riquadro del nominativo; `auto_connect` acceso al primo collegamento (`auto_connect_offered`); stato di Club Log con `code`/`params` tradotti; verifica DXSummit filtrata per DX. Non provato con un cluster vero con password. Bozza di risposta all'issue #47 data all'utente.

- **0.11.249** (2026-11-13T16:56:56Z): spot verificati sull'elenco pubblico di DXSummit dopo l'invio (`reporting/src/dxsummit.rs` `check`/`listed`; controlli a 10, 30, 60, 90 s; stato «spot pubblicato» o «accettato ma non pubblicato»). Non ancora provato con uno spot vero.

- **0.11.248** (15:24 UTC): titolo della finestra con nome e autore, radio spenta a 14 segmenti, Tune e Log QSO sempre visibili, log vuoto con Aggiungi QSO e Importa ADIF.

- **0.11.247** (2026-11-13T14:37:32Z): notch e passabanda in frequenza davanti ai ricevitori PSK/RTTY/CW (`mode-digi/src/notch.rs`, oltre 60 dB, ritardo circa 0,3 s solo con filtri accesi), AFC vincolato, display spento grigio, Spot DX nella barra del cluster. FT-2000 del proprietario: l'accensione via CAT non funziona nemmeno a mano su COM10 senza VSPE (porta CAT della radio non alimentata da spenta): serve una presa comandata.

- **0.11.246** (14:01 UTC): Nuovo log vuoto protetto (archivio controllato o cancellazione con doppia conferma e `confirm: DELETE`), Log QSO apre la finestra compilata senza QSO in corso (SWL), Accendi con Cat4OM (`powerOnRadio`) e RTS alto per le Yaesu (proprietario: FT-2000 con microHAM e VSPE, da provare), display spento con radio che non risponde, Attivita' di banda a colonne spostabili, opzioni del cluster in FT8, ricerca della Guida con risultati. In studio: notch e AFC dei modi testuali.

- **0.11.245** (12:38 UTC): spia di salute della stazione (`GET /api/v1/health/resources`, `HealthLight.tsx`), avvisi e tooltip su fondo bianco, avviso di ingresso debole solo se dura 90 s e mai in TX/TUNE, niente falsi crash da `stderr.log` (`rumore_stderr`), tabella delle frequenze con righe sbagliate tolte una per una (`keep_valid`), menu delle Impostazioni a tasselli, scheda DXpedition «scuro nitido», TX durante TUNE, «Spegni split» sempre visibile, messaggio vero sul database di Log4OM (#51, causa ancora da capire).


- **0.11.244** (09:23 UTC): «Spegni split» accanto al display (`POST /cat/split-off`, `force_split_off`); aggiornamento in un solo
  giro (niente `/R`, riapre solo lo script, l'agente nel tray aggiorna da se' con Aurora chiusa: si vede dall'aggiornamento SUCCESSIVO);
  ascolto SSTV e modi testuali con battito e conteggio delle pagine, satelliti e forme dei nominativi solo quando servono; modalita'
  «PC lento» (`dec.slowPc`, porta `aurora_core::idle`); CW di tutta la banda -40% di CPU e ricampionamento del motore 20 volte piu'
  leggero (risultati identici); file salvati tenuti `aud.keep` giorni (7) e `ALL.TXT` che ruota; guardiano del thread CAT; avvisi
  `LagWatch` sul bus; coda audio limitata a 30 s; frequenze CW/SSTV/RTTY/PSK dai piani ARI (`app/src/lib/bandplan.ts`); manuale
  completo (`guide-*.ts`, copertura in `docs/copertura-guida.md`); separatore dei periodi e Tx nella finestra RX durante il QSO
  (#38, #42); schede DXCC nuovi al massimo due con «Spegni questi avvisi»; avviso di aggiornamento nella lingua di Aurora/Windows.
- **Attivita' pianificata** `aurora-rapporti-cloudflare` (ogni 8 ore, a PC acceso): scarica e svuota i rapporti, corregge i
  difetti piccoli con commit senza push.
- **CPU**: il PC e' quello di lavoro del proprietario: al massimo 12 thread in tutto, agenti compresi (con tre agenti `-j 2` ciascuno).
- **Non provato**: tutto quanto sopra con la radio vera e a schermo; Linux solo compilato da GitHub.
- **Prossimo passo**: prove del proprietario; idee rimaste in `docs/prestazioni-motore.md` e `docs/prestazioni-interfaccia.md`
  (cascata e vetro da misurare in WebView2); QPSK; tabella ITU ufficiale per la convalida dei nominativi.

## Stato al 2026-11-12 tarda notte (0.11.243 pubblicata, sito aggiornato)

- **0.11.243** (22:42 UTC): lo split di FT8 non si riaccende alla riconnessione del CAT ne' al cambio di frequenza, lo Split DX si spegne
  lasciando i modi testuali, PSK/RTTY/CW/SSTV trasmettono in simplex (Hamlib, OmniRig, TCI, Cat4OM); «Accendi» con 75 s di attesa e la
  sequenza `PS1;` a due coppie (0 e 1,2 s, 3,7 e 5,2 s); cambi di frequenza ravvicinati che non si contendono la radio; «Rimanda i QSO
  persi» anche per il server UDP secondario (`UDP2Resend`, acceso di base); CW con AGC dal fondo del rumore (banco 26,5% → 23,4%);
  Linux: uscita audio con buffer di 40 ms e formato a 1-2 canali, registro dei cambi di scheda, testi del volume senza «Windows», pagina
  Radio a 1024 px; avviso sull'indirizzo CI-V in esadecimale; ritocchi dell'interfaccia dei modi testuali, del DX Cluster e dei menu.
- **Dal registro del proprietario** (FT-2000 su COM11, Hamlib, «Accendi» e «Spegni»): dopo l'aggiornamento il CAT e' rimasto 3,5 minuti
  senza collegarsi perche' rigctld veniva chiuso dopo 15 s a meta' accensione; il proprietario dice che «Accendi» non accende la radio.
  La sequenza nuova e' da provare dal vivo: se non accende ancora, servono le righe `rig_control::wake` del registro.
- **Non provato**: tutto quanto sopra con la radio vera e, per Linux, su Linux (compilato solo su Windows; aspettare la conferma
  dell'utente con la IC-7100).
- **Prossimo passo**: prove del proprietario; collegare il rilevatore dei click ai canali di tutta la banda; QPSK.

## Stato al 2026-11-12 notte (0.11.242 pubblicata, sito aggiornato)

- **0.11.242** (21:02 UTC): QSO del log da correggere e cancellare (#47); pannello Segnali con «Controllo» che si salva, Pulisci, altezza e
  nominativi sulla cascata che spariscono (#50); SSTV con cronologie a strisce e freccette, «Trasmetti» in cima con F1 ed Esc; testo
  trasmesso solo nella finestra TX, doppio Esc che spegne l'Auto CQ, tasto RX/TX che ferma; avvio sui Windows 10 prima della 2004
  (`WerRegisterAppLocalDump` cercata quando serve); avvisi della stazione non nei primi 20 s; commento intero nel DX Cluster.
- **In pausa**: ramo `lavoro/cw-riprova` (commit `wip`, non verificato): AGC, velocita' e QRN largo del CW; l'ultima media del banco letta
  era 23,6% contro 26,5%, ma senza la tabella riga per riga. Da riprendere: finire le varianti, controllare che nessuna riga peggiori.
- **Da provare con la radio** e a schermo: tutto quanto sopra e' passato solo dalle prove automatiche.

## Stato al 2026-11-12 sera (0.11.241 pubblicata, sito aggiornato)

- **0.11.241** (19:34 UTC): CW con decodificatore probabilistico, noise blanker, rumore dalle bande laterali, aggancio rapido, «TX alla
  velocita' ricevuta» e anti-click della cascata; tutta la banda CW e PSK nella pagina (pannello Segnali, nominativi sulla cascata, convalida
  dei nominativi a livelli, spot solo locali); PSK con probabilita' e blanker; RTTY oltre (tre catene, squelch statistico, finestra
  Impostazioni RTTY); trasmissione aperta; SSTV (FSK ID, modo senza VIS, log con ricerca); «Rimanda i QSO persi» per i log chiusi
  (`qso-export`, modulo `waiting`); correzioni dai rapporti (aggiornamento fermato dal Restart Manager, WebView2 installato da Aurora, rigctld
  che chiude il collegamento, PulseAudio); schermata chiara dell'installatore quando non riesce; dimensione del testo (#49).
- **Rami di lavoro**: tutto e' su `main`. In corso sul ramo `lavoro/cw-riprova` (da `a549a2fd`): nuovo tentativo su AGC a due inseguitori,
  velocita' robusta con antidisturbo e QRN largo, scartati nella 0.11.241 perche' peggioravano il banco. Entra solo se migliora.
- **Decisioni del proprietario**: gli spot dei decodificatori restano locali (niente invio in rete); verso Log4OM solo UDP con la coda,
  niente scrittura nel suo archivio.
- **Portale**: il ramo `dist` di `aurora-portal` vuole il messaggio di commit `sito: <sha di main>`, altrimenti `aurora-sito aggiorna` lo
  rifiuta; da Git Bash l'alias ssh `aurora-portale` si usa con `/c/Windows/System32/OpenSSH/ssh.exe`.
- **Da provare con la radio** (proprietario): CW a tutta banda, PSK, RTTY, SSTV, coda dei QSO con Log4OM; la schermata d'errore
  dell'installatore non e' stata vista girare.
- **Risposte da dare** (le scrive il proprietario dalle bozze): #46, #27, #40, #48, #49 e F1DSZ.
- **Prossimo passo**: unire `lavoro/cw-riprova` se i numeri migliorano; collegare il rilevatore dei click ai canali di tutta la banda; QPSK.

## Stato al 2026-11-12 pomeriggio (lavoro dopo la 0.11.239, solo commit locali)

- **Commit locali, non pubblicati** (il proprietario li prova prima con l'installatore di prova `Aurora-PROVA-impostazioni.exe` sul Desktop):
  Impostazioni ad albero; pagine Scansione e memorie (PSK·RTTY·CW) e Contest; voci doppie tolte; issue #40 (righe non confermate piu'
  chiare e a righe); avviso sulla frequenza Tx fuori da 200-2700 Hz; Guida aggiornata ai percorsi nuovi. **Niente push finche' il proprietario
  non dice che la prova e' andata bene.**
- `dist-release\` contiene l'installatore di prova, non quello della 0.11.239: alla prossima release si ricompila.
- Le Novita' delle voci nuove si scrivono al rilascio (la prova vuole la voce piu' recente della versione in uso).
- Risposta per la #40 (avllucken) da scrivere quando la versione esce: la dara' il proprietario.

## Stato al 2026-11-12 (0.11.239)

- **0.11.239**: revisione completa con le sue correzioni (sicurezza dell'accesso locale, Split DX, XIT, SQLite) e i punti lasciati
  (Tune subito, Clear Avg di JT65, testi del cluster, PTT con la porta occupata, coda di WSPRnet, pulizie); Linux per la #41
  (audio PulseAudio/PipeWire, CAT con l'Hamlib 4.5 di Debian 12, PTT del Dummy, porte /dev); mappa del mondo dei satelliti (#27).
- **Installatore Windows compilato e firmato sul PC** con `scripts/rilascio-windows.ps1`: la chiave (`%USERPROFILE%\.aurora-signing\aurora.key`,
  id `92a42c12993d4c27`) l'ha ritrovata il proprietario ed e' verificata contro `desktop/updater.pub`. Il lavoro Windows di GitHub salta la
  compilazione se la release ha gia' installatore e `.sig` (lavoro `pc`); Linux e macOS restano su GitHub (gratuiti, repository pubblico).
  Procedura in `docs/rilascio.md`.
- **Da provare con la radio** (proprietario e utenti): Split DX, WSPR/FST4W con split, Cat4OM, OmniRig; su Linux KE4OZD (IC-7100, FT-857D,
  audio) — la #41 resta aperta finche' non conferma.
- **Lavori di una volta**: la cartella di compilazione del Rust su `D:\build\aurora-target` (C: era pieno); `cargo` con `-j 12` e prove con
  `--test-threads=2`, una compilazione alla volta.

## Stato al 2026-11-11 mezzanotte (0.11.238 pubblicata, sito aggiornato)

- **Pubblicata**: 0.11.238 (tre lavori verdi, installatore firmato da GitHub; sito compilato sul PC e caricato, mostra la 0.11.238).
- **Su `main` dopo la 0.11.238**: riquadro del nominativo con LoTW solo se la stazione lo usa (verde, senza data), anche dal DX Cluster con l'elenco dell'ARRL (`lotw_last_upload` in `/api/v1/callsign`); voci per importare le macro da altri programmi tolte dal menu di PSK·RTTY·CW; barra di stato con un solo carattere; rotore meno della meta' nella sua pagina e nella finestra a parte; donazioni con PayPal (badge, sezione nei README, `SOSTENITORI.md`, `.github/FUNDING.yml`).
- **Chiave di firma degli aggiornamenti** (superato il 2026-11-12, vedi sopra): non e' sul PC (`%USERPROFILE%\.aurora-signing\aurora.key` manca) ed e' solo nei segreti di GitHub, che non si rileggono: l'installatore firmato lo fa il lavoro Windows di GitHub (gratis, repository pubblico). Il proprietario non vuole compilare su GitHub: serve la sua copia della chiave per compilare sul PC.
- **Attenzione, cartella principale**: in `Desktop\Progetti\Aurora` il ramo `main` e' fermo alla storia vecchia (`1bce05e4`, 692 commit diversi da GitHub); si lavora in `Aurora-fix-remoto` da `origin/main`. Allinearla solo con il via del proprietario (copia in `backup-pre-rewrite`).
- **Un'altra sessione pusha su `main`** (es. `d7847b06`): prima di ogni push `git fetch` e `git rebase origin/main`.

## Stato al 2026-11-11 notte tardi (0.11.237 pubblicata, 0.11.238 pronta su `main`)

- **Su `main`, per la 0.11.238**: Split DX dal cluster («UP 5», «5UP», «10up», «DN», «QSX»); Cat4OM con la scoperta automatica (porta di gestione 17300, come Decodium); olandese come settima lingua (interfaccia, Guida, Novita', installatore; al primo avvio Aurora parte nella lingua di Windows); radiofaro FST4W e invio facoltativo a WSPRnet (spento di fabbrica); porta COM occupata con il nome del programma; OmniRig lento come avviso; PC lento con «Passa a Normale»; modo degli spot dal band plan IARU Regione 1; via QSL nell'elenco DXpedition; barra di stato PSK/RTTY/CW senza data e ora; testi senza confronti con altri programmi; macro in tema chiaro.
- **In corso**: griglie evidenti e colonne spostabili nella Finestra RX e nella scheda Log (ramo `lav-griglie`, worktree nello scratchpad).
- **Fatto fuori dal codice**: i 22 tag v0.11.134-155 spostati sui commit della storia pulita (nessun ramo o tag su GitHub cita piu' l'assistente); su Cloudflare cancellate le righe d'errore fino alla 0.11.235.
- **Repository privato `wifi75/Aurora-sviluppo` cancellato** (2026-11-11). **Da fare**: pubblicare la 0.11.238 e il sito (compilato sul PC); prove dal vivo con la radio (WSPRnet, radiofaro FST4W, Cat4OM con la scoperta, porta occupata).

## Stato al 2026-11-11 notte (0.11.237 pubblicata, altro gia' su `main`)

Aurora si chiama ora «Aurora — Multimode Ham Radio Station» (it: «stazione radioamatoriale multimodo»): il README e la descrizione dell'installatore lo dicono.

- **Pubblicata**: 0.11.237.
- **Su `main` dopo la 0.11.237** (non ancora in una release): FST4 e FST4W (periodi 15 s-30 min, verificati con `jt9.exe`), WSPR-2 (beacon, percentuale di Tx, verificato con `wsprd.exe`; invio a WSPRnet facoltativo, spento di fabbrica), JT65 sottomodi B/C con media dei periodi, MSK144 a 30 s e «Sh», caselle di decodifica di WSJT-X, rotore unificato, colonne del Contest, icone nuove, schermata di avvio, tempi di accensione dell'FT-2000, campo del rapporto, clic destro su QRZ, testo dell'SSTV centrato, LoTW nel riquadro del nominativo, installatore in sette lingue (con l'olandese).
- **In lavorazione**: Split DX dal DX Cluster (UP/DN/QSX nel commento dello spot), interfaccia in olandese (settima lingua, dizionario a parte; resta da fare) e scoperta delle radio di Cat4OM dalla pagina.
- **Un solo ramo, `main`**: `sviluppo` e' stato fuso e non serve piu'; il repository privato `Aurora-sviluppo` si cancella quando il proprietario da' il via. Il merge di `main` in `sviluppo` non e' piu' un passo da fare.
- **Il sito** (aurora-portal) si compila sul PC e si carica a mano: i minuti di GitHub Actions dei repository privati sono finiti.
- **Prossimi passi**: chiudere Split DX, olandese e scoperta Cat4OM; beacon FST4W in trasmissione; prove dal vivo con la radio (FST4/FST4W, WSPR, JT65 B/C, MSK144, OmniRig via COM, Cat4OM, TCI, SSTV); `wip-fox` (Fox in Attivita' di banda) se ancora aperto.

## Stato al 2026-11-11 sera (0.11.236 pubblicata)

- **Release del 2026-11-11**: 0.11.233, 0.11.234, **0.11.235** (decodificatori allineati a WSJT-X 3.3.0-beta6, OmniRig via COM, Cat4OM nativo, split con XIT, Linux #41, ALSA, TCI) e **0.11.236** (DX Cluster con griglia e colonne spostabili, orologio, avvisi chiudibili con le Impostazioni aperte, riquadro QRZ leggibile, doppioni tolti). Per la 0.11.236 controllare che i lavori Windows, Linux e macOS siano verdi e che la release abbia gli installatori.
- **Lavoro interrotto**: ramo `wip-fox` nel repository privato (`sviluppo/wip-fox`, commit `0edd6517`): righe Fox (DXpedition) leggibili in Attivita' di banda e finestra Rx, e filtro `chkspecial8var` di WSJT-X sui messaggi Fox dubbi. Da finire, provare (`cargo test -p ft8`, vitest su DecodeList) e portare su `main`.
- **Ramo `sviluppo`** (privato): contiene tutto `main` (merge `8093bcaa`) piu' SSTV con la pagina nuova (spettro sottile, log compatto, impostazioni nella categoria SSTV con l'ingranaggio). L'installatore di sviluppo il proprietario lo compila a casa.
- **Da provare dal vivo** (proprietario): OmniRig via COM (FT-710, FTdx3000), Cat4OM con Log4OM (CT1BXX), split Fake It, TCI, MSK144 vero, pagina SSTV. Lo split dal cluster e' gia' provato e funziona.
- **Prossimi lavori**: finire `wip-fox`; poi JT65 sottomodi B/C e media `avg65`; FST4/FST4W (circa 3000 righe); WSPR (circa 2200 righe); MSK144 periodo 30 s e «Sh» in trasmissione; caselle del menu Decode di WSJT-X che mancano (Auto Clear Avg, Include averaging, Aggressive, Single decode, F Tol per JT65/JT9/MSK144); guida «Collegare la radio» per modello (FTdx101, FT-991A, IC-7300...); traduzione olandese (#43, file da preparare per ON7KEC).
- **Segnalazioni aperte con risposta da dare**: #38 (OmniRig, F1DSZ), #41 (Linux), #43 (olandese). Ogni risposta agli utenti chiude con il P.S. della stella su GitHub.

## Stato al 2026-11-10 notte (0.11.232 in pubblicazione, ramo sviluppo con la CW)

- **Release del 2026-11-10**: 0.11.229 (rotore, filtri «sulla banda»), 0.11.230 (ricerca nelle Impostazioni, schede in gruppi, griglie non confermate), **0.11.231** (Contest: TX e Auto CQ sempre accesi, «Salta Tx1» da solo con i messaggi standard), **0.11.232** (PTT: una lettura mancata della radio non scarta piu' il PTT e, se non parte, un avviso a schermo dice il motivo; menu dei contest in ordine di data; il «×N» non sposta la bandiera). La 0.11.232 e' stata creata alle 22:29 UTC: **controllare che i tre lavori (Windows, Linux, macOS) siano verdi e che la release abbia `Aurora-Setup.exe` e il `.sig`**, poi correggere l'ora vera nelle Novita' (`gh release view v0.11.232 --json publishedAt`), commit e push.
- **Origine del fix PTT**: commenti su Facebook di Jeff Cadwalader (IC non specificato, «cambia frequenza ma niente PTT») e Michał Siarkiewicz (IC-7300). Nel codice: `HamlibRig::set_ptt` scartava il PTT con `failures > 0` e l'errore di trasmissione finiva solo nel log. Resta da vedere il pacchetto di assistenza per sapere se era proprio quella la causa; risposta pronta ai due utenti (Impostazioni › Radio, «Test PTT», 0.11.232, pacchetto di assistenza).
- **Ramo `sviluppo`** (repository privato `wifi75/Aurora-sviluppo`, remote `sviluppo`; mai su `origin`): contiene tutto `main` piu' i modi nuovi. Va fuso con `git merge main` dopo ogni versione di `main` (non farlo mentre una compilazione e' in corso: un nuovo push la annulla; l'installatore si rifa a ogni push, circa 11 minuti, release `sviluppo-ultima`). Nome ambiguo: usare `refs/heads/sviluppo`.
- **Contenuto di sviluppo**: SSTV (`mode-sstv`), PSK31/63, RTTY e CW (`mode-digi`, CW in `cw.rs`), Satelliti (`mode-sat`), menu nuovo con i modi in alto (un solo pulsante «PSK · RTTY · SSTV»), pagina PSK/RTTY/CW (macro a blocchetti con i sottoinsiemi, selettore di banda IARU in `app/src/lib/bandplan.ts`, log dello stesso log del pannello Log, DX Cluster con «Spot DX», cascata con rombi RX/TX e scala in kHz veri, opzioni della Vista, tema chiaro e scuro). Il dettaglio e' in `docs/ripresa.md` sul ramo `sviluppo`.
- **Da provare dal vivo** (le prove con la radio le fa il proprietario): CW (trasmissione e decodifica), spot a DXSummit, log di Log4OM con frequenza/nome/QTH, split con «up N», rotori.
- **Da fare**: linguetta «Mappa» nella colonna di destra; opzioni della Vista rimaste fuori (Word mode, stick cursor, IMD); comandi di macro dell'SSTV; split vero della radio per «up» oltre 3,5 kHz; fondere `sviluppo` in `main` solo quando il proprietario lo decide.
- **Attenzione alla pagina PSK/RTTY**: la finestra e' `position: fixed; inset: 0` (`styles/digi-log-corta.css`) perche' `100dvh` non segue lo zoom di `lib/uiScale.ts`; non tornare a `100dvh`.

## Ramo sviluppo - stato al 2026-11-10 notte (PSK, RTTY, CW, SSTV, Satelliti)

- **Pagina «PSK · RTTY · SSTV»** (un solo pulsante in alto; il modo si cambia dal menu «Modo»): macro a blocchetti (4 righe x 12), selettore di banda con le frequenze IARU (`app/src/lib/bandplan.ts`), RX/TX con divisore regolabile, colonna di destra con linguette DX Cluster e Log (bordo trascinabile, colonne a scelta), cascata sotto RX/TX (rombi RX blu e TX rosso, scala in kHz veri, 0-5 kHz o 0-3,5 kHz dal menu Vista, altezza regolabile), opzioni della Vista, tema scuro e chiaro, tutta in una schermata (`styles/digi-log-corta.css`, finestra fissa).
- **Log della pagina** = lo stesso del pannello Log (`getLog4om`, fino a 500 QSO, numero = totale - posizione); freq/nome/QTH letti dalle colonne di Log4OM se ci sono; il log interno ha le colonne `name` e `qth` (schema v4).
- **CW**: `core/crates/mode-digi/src/cw.rs` (modulatore Morse a tono, decodificatore a soglia adattiva, 7 prove), `DigiMode::Cw`, `cw_wpm`, comando `<WPM:n>`. Da provare con la radio.
- **Macro**: editor con i sottoinsiemi (testo, programma, frequenza, Auto CQ, macro, modo, rotore, RTTY, SSTV, CW, HELL); eseguiti: `<FQ>`, `<SPLIT>`, `<ROTOR>`, `<EDITMACRO>`, `<ALARM>`, `<ACQ>`, `<WPM>`; gli altri sono in elenco come «non ancora».
- **Cluster**: PSK nei modi, «Solo nuovi», clic = sintonia sul tono e «up N» in split, pulsante «Spot DX» con la finestra «Invia spot» (rotta `/api/v1/spot/dxsummit` con `freq_khz` e `info`).
- **Da fare**: provare tutto dal vivo (CW, spot, log di Log4OM, split); linguetta «Mappa»; opzioni della Vista che mancano (Word mode, stick cursor, IMD); SSTV: comandi di macro; auto split con la radio per «up» oltre 3,5 kHz; fondere `sviluppo` in `main` solo quando il proprietario lo decide (main resta alla 0.11.231, con il fix PTT e il menu dei contest ordinato non ancora in una release).

## Stato al 2026-11-10 sera (0.11.228 pubblicata, rotori su main)

- **0.11.228** (15:23 UTC): cascata ferma in trasmissione e spettro spegnibile (UN7MBV), filtro «Spotter» del DX Cluster (Luc ON7KEC), MSK144 centrato, chiusura di tutte le finestre con la principale, consigli ARS/split per FT-991 e IC-9700.
- **Rotori (P3) su main, non pubblicati**: Hamlib `rotctld` (`rig-control/src/rotator.rs`), API `/api/v1/rotator`, pagina «Rotore» con rosa dei venti, scheda «Rotore» nelle Impostazioni, Guida. `rotctld.exe` aggiunto all'installatore. Da provare dal vivo prima della prossima versione.
- **Regola sugli altri modi (P2)**: da programmi chiusi solo funzioni e parametri, mai codice; il codice da QSSTV, fldigi, slowrx (GPL) o scritto da zero.
- **In attesa del proprietario**: avviso nel pannello Log sulla fonte (Log4OM) per Luc; blocchi delle finestre al cambio di primo piano.

## Stato al 2026-11-10 pomeriggio (0.11.227 pubblicata)

- **Menu nuovo deciso** (P4 in docs/tasks.md): modi in alto (FT8, VHF, CW, RTTY, PSK, SSTV, Satelliti), a destra le pagine del modo e Log · Mappa DX · Propagazione · menu ⚙, niente seconda riga, volume e tema nella barra in fondo. Prototipo approvato in docs/prototipo-menu-modi/index.html: e' il prossimo lavoro grosso, da fare a passi (prima FT8/VHF con le pagine di oggi spostate).
- **0.11.227**: eta' dei messaggi da quando compaiono (come GridTracker), righe Tx all'inizio del periodo, nominativo di chi trasmette in grassetto, Finestra RX con colonne fisse. Contest di prova: DARC FT4, 2026-11-10 19:00 UTC (Luc ON7KEC). Progetti grossi in docs/tasks.md: satelliti, altri modi, rotori; MSK144 a 30 s (issue #27).

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
Riprendi il lavoro su Aurora nella cartella D:\progetti\Aurora.
1. Leggi AGENTS.md, docs/ripresa.md (la sezione «Stato al 2026-11-10 notte» in cima), memory.md e docs/tasks.md.
2. Allineati: git fetch su origin e sul remote sviluppo; main e sviluppo erano allineati ai remoti (main 57890cee alla 0.11.232, sviluppo 6e1c1499).
3. Dimmi in breve a che punto siamo; poi, per prima cosa: controlla che l'installatore di sviluppo abbia la CW nel menu Modo e rispondi a chi segnalava il PTT (la 0.11.232 e' pubblicata); poi riprendi dai punti «Da fare» di docs/ripresa.md.
Regole: niente riferimenti all'assistente nei file e nei commit (niente trailer); mai il locatore vero ne' email personali; push e release autorizzati (AGENTS.md); un push su sviluppo annulla la compilazione in corso, quindi raggruppali; dopo ogni versione di main, git merge main in sviluppo; le prove dal vivo con la radio le faccio io; i testi sono in sei lingue.
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
