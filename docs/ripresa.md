# Ripresa del lavoro (ultimo aggiornamento: 2026-11-03)

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

- **Ultima versione**: **0.11.90** (2026-11-03, pubblicata). Telefono come telecomando rifinito dalla 0.11.85 alla 0.11.90: solo la pagina Stazione (`REMOTE_TOUCH` in `App.tsx`, niente testata, pie' di pagina e toast), ordine attivita' di banda > cluster (ultimi 5 spot, modo e spotter) > log (con bandiera) > messaggi Tx > cascata, blocchi con bordo e titolo colorati, stazione chiamata in giallo, doppio tocco su STOP = Canc DX, cambio tema e ⟳ in alto a destra, barra di stato iPhone piena (`apple-mobile-web-app-status-bar-style` = black). Aggiornamento automatico del telefono: `app/src/lib/freshUi.ts` confronta la versione dell'interfaccia (`__AURORA_UI_VERSION__` da `vite.config.ts`) con `/api/v1/health` e ricarica senza cache. Accesso senza codice dalla rete di casa salvato in `accesso-libero.txt`. Il telefono usa le preferenze del PC tranne quelle in `PHONE_OWN` di `lib/syncSettings.ts`.
- **Storico pulito**: il 2026-11-02 lo storico di GitHub e' stato riscritto (git filter-repo, push forzato di main, gh-pages e tag) per togliere ogni riferimento all'autore delle tabelle del codice polare dell'UltraFOX. Regola: non citarlo mai (vedi memoria privata). Copia dello storico vecchio solo in locale: `D:\progetti\Aurora-backup-prima-pulizia-qpc.bundle`.

## Lavoro in corso

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
