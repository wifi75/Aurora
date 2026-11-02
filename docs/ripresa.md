# Ripresa del lavoro (ultimo aggiornamento: 2026-11-01 sera)

Questo file dice a chi riprende il lavoro, in una sessione nuova, **a che punto e' Aurora** e **cosa fare per primo**. Va aggiornato a ogni passaggio di sessione.

## Prompt da incollare nella sessione nuova

```text
Riprendi il lavoro su Aurora nella cartella D:\progetti\Aurora.
1. Leggi AGENTS.md, docs/ripresa.md, memory.md (sezione «Come riprendere»), TODO.md e docs/tasks.md.
2. Allineati: git fetch, controlla main e il ramo resti-motore (anche nella copia di lavoro D:\build\wt-resti1).
3. Dimmi in breve a che punto siamo, poi riprendi dal punto «Prossimo passo» di docs/ripresa.md.
Regole: niente riferimenti all'assistente nei file e nei commit; mai il locatore vero; il push e le release sono autorizzati;
le prove dal vivo con la radio le faccio io; giro di commit ogni 30 minuti.
```

## Stato

- **Ultima versione pubblicata**: **0.11.73** (2026-11-02): pagina Informazioni centrata con il link al sito. Prima, 0.11.72: Abilita Tx al primo clic nel modo Fox, pagina Fox in una schermata con cascata ridimensionabile, FOX solo sotto 1000 Hz. Prima, 0.11.71: pagina Fox completa e ridisegnata (`pages/FoxPage.tsx`, `styles/fox.css`), niente 73 dopo il nostro RRR, forma del paese nella scheda DXpedition (`components/CountryShape.tsx`), avvisi GPL nella pagina Informazioni. Prima, 0.11.70: UltraFOX (`qpc.rs`, `ultrafox.rs`, `ultrafox_rx.rs`). Da provare in aria: Fox/UltraFOX; mancano firma digitale UltraFOX e rimozione dei segnali FT8 prima della decodifica UltraFOX.
- **Cosa c'e' nella 0.11.60**: Impostazioni riordinate in sei gruppi (Servizi di log, Programmi collegati), «Log» nel menu in alto al posto di «Storico», display Yaesu FT-2000 e Icom IC-7610, DXpedition evidenziate nel DX Cluster con colore a scelta, barre Rx/Tx fluide, decodifica FT8 a7/a8 e multithread, protezioni in trasmissione, invio diretto a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP, diagnostica dei crash. Dettaglio in `CHANGELOG.md`.

## Lavoro in corso (non finito)

- Il ramo `resti-motore` e' unito a `main` e pubblicato nella 0.11.61 (Hound automatico, FOX, Wait and Pounce, ALL.TXT, scambi di contest). Scelta del proprietario: l'R+rapporto subito dopo il nostro Tx1 chiude con RR73 come WSJT-X 3.3.
- **UltraFOX**: pubblicato nella 0.11.70 (compatibile con SuperFox); il ramo locale `superfox-rx` non serve piu'.

## Prossimo passo

2. Voci aperte di `docs/tasks.md`: TCI, Q65, MSK144, pulsanti dei modi, QMAP, pagina Fox (bozza `docs/mockup-fox.html`), SuperFox quando arriva il permesso.
3. Regola del proprietario: si parte sempre da WSJT-X 3.3 (`D:\build\analisi-wsjtx\sorgenti`); cio' che WSJT-X 3.3 non ha non si aggiunge (per questo «Sinc» di JTDX e' tolto).

## Da provare dal vivo (proprietario)

Scheda DXpedition con mappa (corretta nella 0.11.65 e 0.11.67), lente sul display (2x), pannello QSO con colori (viola acceso, rosso per Abilita Tx/AutoTX), doppio clic su spot FT4/FT8 che cambia modo, pagina Log. 
Pannello Comandi (DX scritto a mano, rapporto, Hold Tx Freq, tipo di CQ, Tune e Log QSO), pannello QSO in due colonne, Hound automatico e FOX, Wait and Pounce, chiusura con RR73 dopo un R+rapporto al Tx1, nuovo menu Impostazioni e pagina Log, barre Rx/Tx, display Yaesu/Icom, colore DXpedition nel cluster, a7/a8, stop per ROS alto, invii a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP.

## Dove stanno le cose

- Codice: `D:\progetti\Aurora` (repository pubblico github.com/wifi75/Aurora). Compilazione dell'installatore: `D:\build\Aurora-0.11.1` (cartella di rilascio, si porta sul commit con `git checkout --detach`).
- Analisi di WSJT-X 3: `D:\build\analisi-wsjtx` (sorgenti e binari), `docs/analisi-wsjtx-3.md`.
- Chiave di firma degli aggiornamenti: `%USERPROFILE%\.aurora-signing\aurora.key` (fuori dal repository).
- Le copie di lavoro `D:\build\wt-*` diverse da `wt-resti1` e `wt-superfox` sono gia' unite a `main` e si possono cancellare.
