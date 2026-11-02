# Ripresa del lavoro (ultimo aggiornamento: 2026-11-01 sera)

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

- **Ultima versione**: **0.11.79** (2026-11-02, in pubblicazione): display FT-2000 con RX/TX in alto dopo la frequenza e senza scala di sintonia, potenza riletta dopo − e + con avviso se la radio non la cambia (FT-2000: decide la manopola RF PWR), DX Cluster allineato, Aggiornamenti senza voci per sviluppatori, log senza i messaggi innocui di WebView2. Prima, 0.11.78: riquadro del paese con regione e mondo in miniatura (`components/CountryShape.tsx`, d3-geo), tooltip in sei lingue ovunque, telefono piu' comodo, Integrazioni con un pulsante per tipo di programma.
- **Storico pulito**: il 2026-11-02 lo storico di GitHub e' stato riscritto (git filter-repo, push forzato di main, gh-pages e tag) per togliere ogni riferimento all'autore delle tabelle del codice polare dell'UltraFOX. Regola: non citarlo mai (vedi memoria privata). Copia dello storico vecchio solo in locale: `D:\progetti\Aurora-backup-prima-pulizia-qpc.bundle`.

## Lavoro in corso (rami non ancora uniti a main)

- `cluster-qrz` (`D:\build\wt-qrz`, finito, da unire): ricerca QRZ.com XML e HamQTH (rotta `GET /api/v1/callsign/{call}`), suggerimento sul nominativo DE nel DX Cluster, invio a HRDLOG.NET, NAME e QTH nei QSO, LED di stato e «Prova» nei servizi, campi segreti con «Chiave salvata» e Cambia/Togli, un servizio per riga in Servizi di log e Servizi online. Prima di unirlo: `cargo test -p logbook -p aurora-core -p aurora -p reporting -p remote-api --target-dir target/testrun`. Mancano ancora le prove per eQSL, PSK Reporter, DXSummit, NG3K, propagazione, NTP, UDP/TCP.
- `cascata-wsjtx` (`D:\build\wt-cascata`, in lavorazione): cascata come WSJT-X 3.3 (righe per periodo, pixel netti, blocchi compatti), pulsanti «Stile WSJT-X / Stile JTDX», WSJT-X attivo da subito anche per chi aggiorna.
- `grafica-impostazioni` (`D:\build\wt-grafica`, in lavorazione): revisione grafica di tutte le pagine delle Impostazioni (testi tagliati, larghezze, allineamenti), screenshot prima/dopo.

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
