# Ripresa del lavoro (ultimo aggiornamento: 2026-11-04)

Questo file dice a chi riprende il lavoro, in una sessione nuova, **a che punto e' Aurora** e **cosa fare per primo**. Va aggiornato a ogni passaggio di sessione.

## Prompt da incollare nella sessione nuova

```text
Riprendi il lavoro su Aurora nella cartella D:\progetti\Aurora.
1. Leggi AGENTS.md, docs/ripresa.md, memory.md (sezione «Come riprendere»), TODO.md e docs/tasks.md.
2. Allineati: git fetch, controlla main e i rami in corso elencati sotto.
3. Dimmi in breve a che punto siamo, poi riprendi dal punto «Prossimo passo» di docs/ripresa.md.
Regole: niente riferimenti all'assistente nei file e nei commit; mai il locatore vero; il push e le release sono autorizzati;
le prove dal vivo con la radio le faccio io; giro di commit ogni 30 minuti.
```

## Stato

- **Ultima versione**: **0.11.95** (2026-11-04, pubblicata). Dalla 0.11.92: PTT dalla radio senza JTDX (`CatPtt`/`AutoPtt` e `ptt_method_for` in `core/crates/aurora/src/lib.rs`: con `AURORA_PTT_METHOD` da JTDX vale quello, altrimenti si decide a ogni trasmissione da Impostazioni > Radio); configurazione guidata della radio (`app/src/lib/rigPresets.ts`); Guida riorganizzata in sei gruppi (`app/src/i18n/guide.ts`); pacchetto per l'assistenza (`core/crates/aurora/src/support.rs`, zip sul Desktop senza segreti); stderr.log senza righe innocue di WebView2.
- **Storico pulito**: il 2026-11-02 lo storico di GitHub e' stato riscritto (git filter-repo, push forzato di main, gh-pages e tag) per togliere ogni riferimento all'autore delle tabelle del codice polare dell'UltraFOX. Regola: non citarlo mai (vedi memoria privata). Copia dello storico vecchio solo in locale: `D:\progetti\Aurora-backup-prima-pulizia-qpc.bundle`.

## Lavoro in corso

- Non pubblicato su main: scheda Impostazioni > Assistenza (`SupportTab` in `app/src/settings/SupportBox.tsx`, campo email vuoto di fabbrica, «Manda per email» apre Gmail in Chrome via `links.rs`); bandiera e paese sul telefono seguono le preferenze; colonna dei km nell'attivita' di banda (vuota = nascosta, visibile anche sotto 40rem; in CQ e novita' resta nascosta).
- In lavorazione sul ramo `rapporti` (`D:\build\wt-rapporti`): rapporti degli errori, livello 1 dopo un crash e livello 2 automatico anonimo, entrambi spenti di fabbrica, verso un Worker Cloudflare del proprietario (`servizi/rapporti/`, da mettere online lui con wrangler); pulsante «Manda il pacchetto a chi sviluppa Aurora».
- Aperti: l'IC-7300 di un amico del proprietario non va in TX nemmeno con la 0.11.93 (serve il pacchetto/log); CAT4OM: proposto di collegarlo con il ponte Hamlib (modello «Hamlib NET rigctl», 127.0.0.1:4532), non provato; i server UDP primario e secondario di fabbrica mandano entrambi a 127.0.0.1:2237 (QSO doppi), da decidere; tema del telefono condiviso con il PC, da decidere.
- Nessun ramo aperto: `cluster-qrz`, `cascata-wsjtx` e `grafica-impostazioni` sono uniti a main; i worktree in `D:\build\wt-*` si possono togliere.
- Restano da fare: «Prova» per eQSL, PSK Reporter, DXSummit, NG3K, propagazione, NTP, UDP/TCP; NAME e QTH per i QSO registrati a mano.
- Decisione aperta: il tema scelto sul telefono oggi cambia anche il PC (`aurora-theme` non e' in `PHONE_OWN`); chiedere al proprietario se separarlo.
- Da riordinare: `app/src/styles/phone.css` ha molte regole aggiunte in coda che si sovrascrivono; la regola larga `.phone-live * { max-width:100% }` andrebbe ristretta ai contenitori.
- Da provare dal vivo: vista telefono e tablet su dispositivi veri; Android spesso non risolve `aurora.local` (il QR usa l'IP); le anteprime dal vivo nelle Impostazioni sono l'app vera (anche la trasmissione).
- I test li lancia il proprietario: prima di pubblicare solo `tsc` e `cargo check`.

## Prossimo passo

1. Finire la pubblicazione della 0.11.79 e l'ora vera nelle Novita'.
2. Unire `cluster-qrz`, poi gli altri due rami quando finiscono; mostrare al proprietario le immagini prima di pubblicare la 0.11.80.
3. Poi le voci aperte di `docs/tasks.md` (TCI, Q65, MSK144, pulsanti dei modi, QMAP).

## Da provare dal vivo (proprietario)

Scheda DXpedition con mappa (corretta nella 0.11.65 e 0.11.67), lente sul display (2x), pannello QSO con colori (viola acceso, rosso per Abilita Tx/AutoTX), doppio clic su spot FT4/FT8 che cambia modo, pagina Log. 
Pannello Comandi (DX scritto a mano, rapporto, Hold Tx Freq, tipo di CQ, Tune e Log QSO), pannello QSO in due colonne, Hound automatico e FOX, Wait and Pounce, chiusura con RR73 dopo un R+rapporto al Tx1, nuovo menu Impostazioni e pagina Log, barre Rx/Tx, display Yaesu/Icom, colore DXpedition nel cluster, a7/a8, stop per ROS alto, invii a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP.

## Dove stanno le cose

- Codice: `D:\progetti\Aurora` (repository pubblico github.com/wifi75/Aurora). Compilazione dell'installatore: `D:\build\Aurora-0.11.1` (cartella di rilascio, si porta sul commit con `git checkout --detach`).
- Analisi di WSJT-X 3: `D:\build\analisi-wsjtx` (sorgenti e binari), `docs/analisi-wsjtx-3.md`.
- Chiave di firma degli aggiornamenti: `%USERPROFILE%\.aurora-signing\aurora.key` (fuori dal repository).
- Le copie di lavoro `D:\build\wt-*` non elencate sopra sono gia' unite a `main` e si possono cancellare.
