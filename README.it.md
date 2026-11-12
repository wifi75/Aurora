# Aurora — Multimode Ham Radio Station

[![Download](https://img.shields.io/github/downloads/wifi75/Aurora/total?label=Download&color=2ea44f&logo=github)](https://github.com/wifi75/Aurora/releases)
[![Senza virus](https://img.shields.io/badge/Senza%20virus-0%20segnalazioni-2ea44f?logo=virustotal&logoColor=white)](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)
![Visite](https://visitor-badge.laobi.icu/badge?page_id=wifi75.Aurora&left_text=Visite&left_color=%23555&right_color=%238e7bff)
[![Dona](https://img.shields.io/badge/Dona-PayPal-00457C?logo=paypal&logoColor=white)](https://www.paypal.com/donate/?hosted_button_id=MY55Q6GDQEH8L)
[![Piattaforma](https://img.shields.io/badge/Piattaforma-Windows%20%7C%20Linux%20%7C%20macOS-0078D4?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.md)
[![Linux](https://github.com/wifi75/Aurora/actions/workflows/linux.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/linux.yml)
[![macOS](https://github.com/wifi75/Aurora/actions/workflows/macos.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/macos.yml)
[![Windows](https://github.com/wifi75/Aurora/actions/workflows/windows.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/windows.yml)
[![Rust](https://img.shields.io/badge/Rust-1.99%20%C2%B7%20edition%202024-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/API-Axum%200.8-1A1A1A)](https://github.com/tokio-rs/axum)
[![Tokio](https://img.shields.io/badge/Async-Tokio%201.54-1A1A1A)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/DB-SQLite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)
[![Tauri](https://img.shields.io/badge/Desktop-Tauri%202-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![React](https://img.shields.io/badge/UI-React%2019-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-7.0-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Build-Vite%208-646CFF?logo=vite&logoColor=white)](https://vite.dev)
[![WebGPU](https://img.shields.io/badge/Waterfall-WebGPU%20%2B%20WebGL2-005A9C)](https://developer.mozilla.org/docs/Web/API/WebGPU_API)
[![Lingue](https://img.shields.io/badge/Lingue-7-FF5C7A)](app/src/i18n)
[![Test](https://img.shields.io/badge/Test-1981%20Rust%20%C2%B7%201066%20UI-3FD6C8)](#sviluppo)
[![SemVer](https://img.shields.io/badge/SemVer-2.0.0-3F4551)](https://semver.org/lang/it/)
[![Keep a Changelog](https://img.shields.io/badge/Changelog-Keep%20a%20Changelog-E05735)](CHANGELOG.md)
[![Manutenzione](https://img.shields.io/badge/Mantenuto-s%C3%AC%20(2026)-brightgreen)](CHANGELOG.md)
[![Licenza](https://img.shields.io/badge/Licenza-GPL--3.0-blue)](LICENSE)

*Dedicato ad Aurora, mia figlia: la luce da cui prende il nome questo programma.* ♥

**Italiano** · [English](README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Español](README.es.md) · [Português](README.pt.md)

🌐 **[Sito](https://aurora.iu3wjq.com/it/)**

> ### Scarica Aurora
> [![Download](https://img.shields.io/badge/Download-Aurora--Setup.exe-1E90FF?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--x86__64.AppImage-FCC624?logo=linux&logoColor=black)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--amd64.deb-A81D33?logo=debian&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) [![Download](https://img.shields.io/badge/Download-Aurora--macOS.dmg-000000?logo=apple&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg) [![latest](https://img.shields.io/github/v/release/wifi75/Aurora?label=latest)](https://github.com/wifi75/Aurora/releases/latest)
>
> La versione 0.11.188 è stata analizzata con [VirusTotal](https://www.virustotal.com) (circa 70 antivirus) e Windows Defender: nessun antivirus la segnala ([risultato](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)).
>
> **[Installatore per Windows (ultima versione)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe)** · [Pagina dell'ultima release](https://github.com/wifi75/Aurora/releases/latest)
>
> [![Linux](https://img.shields.io/badge/Guida-Linux-E95420?logo=ubuntu&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.md)
>
> 🐧 **[Linux (non provata dall'autore, senza assistenza): AppImage, .deb o compilazione con un comando](https://github.com/wifi75/Aurora/blob/main/docs/linux.md)**
>
> 🍎 **[macOS (non provata dall'autore, senza assistenza): Aurora-macOS.dmg per Apple Silicon e Intel](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg)** — non firmata da Apple: al primo avvio tasto destro › Apri (oppure `xattr -cr /Applications/Aurora.app`). Per il CAT installa Hamlib con `brew install hamlib`. Gli aggiornamenti si installano dall’app («Installa e riavvia»), come su Windows: tieni Aurora in Applicazioni, non aperta dal .dmg; al primo avvio dopo un aggiornamento macOS potrebbe chiedere di nuovo conferma, perché Aurora non è firmata da Apple.

Stazione radioamatoriale multimodo di nuova concezione (FT8, FT4, JT65, JT9, JT4, Q65, MSK144, FST4, FST4W, WSPR, PSK31/PSK63, RTTY, CW, SSTV e satelliti): interfaccia d'impatto, automazione dei QSO e controllo remoto da telefono. Ha un motore proprio di decodifica e trasmissione; Aurora è cervello, motore e schermo.

Progettazione e sviluppo: Tiziano Cassone (IU3WJQ)

## Stato

Versione 0.11.240: motore proprio multimodo (FT8, FT4, JT65 A/B/C, JT9, JT4 A–G, Q65, MSK144, FST4, FST4W e WSPR) con ricezione e trasmissione, CAT per tutte le radio e display della radio, più le pagine PSK31/PSK63, RTTY, CW, SSTV e Satelliti. La trasmissione con la radio vera va ancora provata dal proprietario.

- **Stazione**: elenco dei messaggi come in JTDX (paese con bandiera, distanza, righe colorate per tipo, nominativo chiamato e chiamate verso di te in evidenza), cascata WebGPU/WebGL2 con le barre Rx/Tx trascinabili, Tx1–Tx6 con la fase del QSO, barra di bande e modo (FT8/FT4) con cambio dalla pagina.
- **Coda dei chiamanti**: tasto destro su una stazione che chiama la mette in coda (sigla «IN CODA» con il posto) e a QSO registrato Aurora risponde alla prima; l'interruttore «Risp. a tutti» accoda da solo chi ti chiama; «Canc DX» ferma subito la trasmissione. Solo motore proprio.
- **Modi**: dal selettore FT8, FT4, JT65 (sottomodi A/B/C con media dei periodi, per l'EME), JT9, JT4 (A–G), Q65, MSK144 (5/10/15/30 s e messaggi «Sh»), FST4 e FST4W (periodi da 15 s a 30 min), WSPR-2 (beacon con percentuale di Tx), più le pagine PSK31/PSK63, RTTY, CW, SSTV e Satelliti (tracciamento, Doppler, rotore); interruttore «JT65+JT9» per decodificarli insieme (dopo il cambio di modo servono da uno a due minuti); Fox/Hound (anche UltraFOX) con salti di frequenza e «Filtro»; Modo SWL come JTDX. Stato e parita' con JTDX in `docs/2026-10-26-jt65-jt9.md`.
- **WSPR e FST4W**: beacon con potenza, «Tx Pct» e invio facoltativo degli spot a WSPRnet, spento di fabbrica. Opzioni di decodifica di WSJT-X in Impostazioni › Filtri › Decodifica.
- **Comandi come JTDX**: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, frequenze Tx/Rx con copia, Canc DX, Gen.Msgs; il doppio clic su un messaggio imposta il DX e JTDX lo chiama al momento giusto (opzione in Impostazioni → Generali).
- **Trasmissione**: CQ, messaggio libero, frequenza Tx, livello Pwr verticale, STOP TX sempre attivo (discreto a riposo, pieno in trasmissione); il tasto Esc fa lo stesso e spegne AutoTX e Abilita Tx. Non ha opzioni: è sempre abilitata (restano STOP TX, i tetti e le automazioni spente di base).
- **Automazioni**: risposte ai CQ con novità, partono spente e in prova a vuoto, con tetti, dead-man e STOP.
- **Impostazioni per argomento**: sei gruppi (Stazione, Ricezione, Trasmissione e QSO, Avvisi, Rete e log, Sistema) con pagine come Radio, Cascata, Sequenza, Servizi di log e Programmi collegati; ricerca su tutte le voci, errori mostrati accanto al campo.
- **Sette lingue** (italiano, inglese, tedesco, francese, spagnolo, portoghese, olandese) e un tooltip su ogni controllo.
- **Guida e statistiche**: guida in-app breve nelle sette lingue; pagina Statistiche con premi, QSO per banda e paesi mancanti; i tuoi messaggi trasmessi compaiono come righe `Tx`.
- **DX Cluster, orologio, log**: finestra DX Cluster (server telnet pubblici o propri, doppio clic = la radio va sulla frequenza dello spot; Split DX: con «UP 5», «5UP» o «QSX» nel commento il doppio clic mette la radio in split sul VFO B; colonna Modo anche per CW e SSB, dal band plan IARU Regione 1 quando il commento non lo dice), client NTP con indicatore dell'orologio, finestra Log di Log4OM2 in sola lettura.
- **Strumenti della stazione**: livello dell'audio in ingresso con cursore e AUTO sul volume di registrazione di Windows, storico dei messaggi ricevuti (30 giorni, ricercabile), spot del cluster sulla cascata, diagnosi automatica (orologio, audio, nominativo), pagina Propagazione con PSK Reporter e hamqsl.com (**accesi di fabbrica, si spengono quando vuoi**) e la Mappa DX (sotto).
- **Mappa DX** (alla GridTracker): 15 sfondi, notte automatica, linea grigia con i crepuscoli, Sole e Luna, stazioni in onda colorate che sbiadiscono, linee ad arco animate, quadrati lavorati e confermati per banda e modo, clic con quadrato, distanza, paese e zone CQ/ITU, propagazione con mappa di calore PSK Reporter e linee verso chi ti riceve, linee «Chi chiama chi» personalizzabili, quadrati in onda QSX/CQ/CQDX/QRZ con legenda; si stacca su un altro schermo.
- **Radio**: display della radio in alto con il display esteso (secondo VFO, strumenti, PRE/ATT, AGC, NB/NR/notch, guadagni, RIT/XIT), ogni voce accendibile in Impostazioni > Generali; la cascata segue la larghezza del filtro della radio letta col CAT; un rigctld rimasto orfano si chiude da solo; su Windows «Ripristina Hamlib» riscarica rigctld se l'antivirus lo toglie.
- **DXpedition**: calendario NG3K (Servizi online, spento di base) accanto ai CQ: in onda, sentite adesso, in arrivo e storico; accanto al paese la via QSL (LoTW, OQRS, manager).
- **Impostazioni**: barra laterale con ricerca, «?» con la spiegazione su ogni voce, pagina Informazioni (versione, autore, licenza, collegamenti).
- **Telefono**: PWA sulla stessa rete, con la pagina Stazione a schede e pulsanti da dito (schermi fino a 640 px); associazione con codice (durata decisa da te) oppure accesso senza codice per le prove.
- **Log e integrazioni**: invio diretto a Wavelog/Cloudlog, ClubLog, QRZ.com e LoTW (TQSL); integrazioni UDP con multicast e opzioni per programma, XML di N1MM; pagina di esportazione ADIF con «Salva con nome».
- **Da WSJT-X 3**: decodifica a7/a8 e FT8 multithread, Wait and Reply/Call, cambio banda automatico, full duplex, verifica della Fox (OTP), blocco della Tx via UDP, stop con ROS alto, messaggi non codificabili bloccati, filtri di WSJT-X 3, Hamlib 4.7.2.
- **OmniRig e Cat4OM**: OmniRig si comanda direttamente via COM come WSJT-X (niente PowerShell, niente «accesso negato», PTT più veloce); Cat4OM nativo, Aurora e Log4OM comandano la stessa radio insieme (gruppo e porta trovati da soli dalla porta di gestione 17300, come Decodium) (Impostazioni › CAT); split Rig/Fake It anche con OmniRig e TCI; ritardo Tx regolabile (0-0,5 s); se la porta COM è occupata, Aurora dice da quale programma.
- **Decodificatori allineati a WSJT-X 3.3**: FT8 (precedenza alla frequenza Rx, OSD 4 per le risposte del QSO, analisi a priori per /P e nominativi composti), FT4 (ricerca dei candidati di WSJT-X), Q65 (media con la lista del QSO, correzione della deriva) JT65/JT9 (soglie di WSJT-X, finestra Rx), MSK144, FST4/FST4W e WSPR.
- **Installatore in sette lingue** (italiano, inglese, tedesco, francese, spagnolo, portoghese, olandese); anche l'interfaccia è in sette lingue: l'olandese è la prima versione, da far rivedere.
- **Display e cluster**: display della radio in stile Yaesu FT-2000 e Icom IC-7610; DXpedition NG3K evidenziate nel DX Cluster con il colore che scegli; anteprima dal vivo della cascata nelle impostazioni; diagnostica dei crash sempre accesa.
- **Contest (beta)**: pagina dedicata con 24 contest FT8/FT4 e VHF (regolamenti e date di ogni anno), sequenza contest come WSJT-X, doppi, punti, moltiplicatori e punteggio previsto in diretta, categoria e operatori, log con Cabrillo, EDI e ADIF.
- **UltraFOX**: implementazione propria in Rust con il codice polare AQP-127 (implementazione di Tiziano Cassone IU3WJQ), compatibile con SuperFox di WSJT-X (trasmissione e ricezione). Funzione sperimentale.

Cosa manca: `docs/mancante.md`. Esiti delle prove con JTDX: `docs/fase0-rapporto.md`.

## Installazione (Windows)

Il modo piu' semplice: l'installatore `Aurora_X.Y.Z_x64-setup.exe` (programma con vassoio, avvio con Windows e aggiornamenti firmati). Come si installa, dove stanno i dati e come si fa un rilascio: `docs/installer-windows.md`.

## Avvio rapido (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Motore
```

Avvia il core (porta 8787) e l'interfaccia (porta 5173) in una sola tab di Windows Terminal; con `-Motore` usa il motore di Aurora sulla scheda audio (`-Audio "USB Audio"` per sceglierla). Apri la pagina **dall'indirizzo IP del PC** (per esempio `http://192.168.1.225:8787`), non da `localhost`: è lo stesso indirizzo che usa il telefono.

Per avere anche la cascata imposta `AURORA_AUDIO_DEVICE` (parte del nome dell'ingresso audio, per esempio `USB Audio`): senza, la cascata resta vuota.

Per servire l'interfaccia compilata dal core (senza il server di sviluppo): `cd app && npm run build`, poi avvia Aurora con `AURORA_UI_DIR=app\dist` e `AURORA_BIND=0.0.0.0:8787`.

### Collegare un telefono

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

Mostra un codice di 8 cifre e un QR; il codice vale per il tempo che decidi in Impostazioni → Accesso (predefinito 10 minuti), è monouso e si blocca dopo cinque tentativi sbagliati. Nessuna password: l'accesso è per dispositivo associato e si revoca dall'elenco.

Durante le prove si può accendere l'**accesso senza codice** (Impostazioni → Accesso, oppure `AURORA_OPEN_ACCESS=1`): vale solo per chi è sulla rete di casa, mai per internet o proxy, e dal PC si spegne in qualunque momento.

### CAT: la radio per tutti i modelli

In Impostazioni → Radio CAT si sceglie come comandare la radio (frequenza, modo, PTT, split): **Hamlib** tramite `rigctld` (oltre 300 modelli, la scelta di base) OmniRig (direttamente via COM) o Cat4OM nativo; anche TCI e FT-991 Link. Profili con nome, rilevamento della radio e stato in tempo reale. Guida per l'utente e prove con la radio vera in `docs/cat-radio.md`.

## Configurazione

Variabili d'ambiente (elenco completo in `.env.example`): `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`native` o `sim`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
Le preferenze di visualizzazione (colori, filtri, lingua, bande, macro) stanno nel browser; frequenze, programmazione, regole dell'automazione, durata del codice e radio (`radio.json`: modello, porta, profili) stanno in `data/`.

## Struttura

```
core/crates/   aurora-core (tipi, bus)  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine (automazioni)  qso-export
               rig-control (CAT: Hamlib e OmniRig, frequenze, programmazione)  auth  remote-api (HTTP/WebSocket)  aurora (binario)
app/src/       pages  components  settings (una scheda per funzione)  lib  i18n (6 lingue)  render (cascata)  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          specifica, piani, rapporti, API, mancante.md
```

## Documentazione

- Specifica: `docs/2026-10-24-aurora-design.md` · API: `docs/api.md` · Flussi UDP: `docs/udp-flussi.md`
- Piani: `docs/2026-10-24-aurora-piano*.md` · Rapporti: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- Cosa manca: `docs/mancante.md` · Memoria di progetto: `memory.md` · Cronologia: `CHANGELOG.md`

## Sviluppo

```
cd core && cargo test --workspace      # 1686 test del core
cd app && npm install && npm test      # 839 test dell'interfaccia
cd app && npm run build                # build di produzione (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

Sul PC con Aurora accesa, i test del core si lanciano con `--target-dir target/testrun` (altrimenti Windows blocca `aurora.exe`).

## Sostieni Aurora

Aurora è gratuita e lo resta. Se ti è utile e vuoi aiutarne lo sviluppo, puoi fare una donazione con PayPal:

[![Dona con PayPal](https://img.shields.io/badge/Dona_con-PayPal-00457C?style=for-the-badge&logo=paypal&logoColor=white)](https://www.paypal.com/donate/?hosted_button_id=MY55Q6GDQEH8L)

Grazie di cuore a chi ha già donato: il vostro sostegno conta. I sostenitori che lo desiderano sono ringraziati in [SOSTENITORI.md](SOSTENITORI.md).

## Licenza

Aurora e' software libero, rilasciato con la **GNU General Public License versione 3** (`GPL-3.0-only`: il motore proprio contiene algoritmi di JTDX e WSJT-X, rilasciati con la GPL 3): testo completo in [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). Le librerie di terzi e i loro avvisi sono in [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) e [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md).

Se usi il codice o le tabelle di Aurora in un tuo progetto, ti chiedo per cortesia di farmelo sapere aprendo una segnalazione su GitHub e di citare l'autore, Tiziano Cassone IU3WJQ.

Chi ridistribuisce Aurora, anche modificata, deve farlo con la stessa licenza e rendere disponibili i sorgenti. Le librerie di terzi che l'interfaccia include (`leaflet` per la mappa, licenza BSD-2-Clause, `world-atlas` e `topojson-client`, licenza ISC;  le tessere dello sfondo della mappa sono di Esri e National Geographic, con la loro attribuzione sulla mappa, e le altre dipendenze elencate in `app/package.json` e `core/Cargo.toml`) mantengono le proprie licenze.
