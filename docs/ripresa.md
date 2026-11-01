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

- **Ultima versione pubblicata**: **0.11.60** (2026-11-01). `main` locale e GitHub sono allineati; sito https://wifi75.github.io/Aurora/ aggiornato con lo screenshot vero.
- **Cosa c'e' nella 0.11.60**: Impostazioni riordinate in sei gruppi (Servizi di log, Programmi collegati), «Log» nel menu in alto al posto di «Storico», display Yaesu FT-2000 e Icom IC-7610, DXpedition evidenziate nel DX Cluster con colore a scelta, barre Rx/Tx fluide, decodifica FT8 a7/a8 e multithread, protezioni in trasmissione, invio diretto a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP, diagnostica dei crash. Dettaglio in `CHANGELOG.md`.

## Lavoro in corso (non finito)

- **Ramo `resti-motore`** (su GitHub come ramo, NON unito a `main`; copia di lavoro `D:\build\wt-resti1`, ultimo commit di salvataggio `ce309ad`):
  - Hound automatico (si accende e spegne da solo secondo il DX), etichetta «FOX» sulle righe delle Fox di WSJT-X, avviso se la stazione chiamata non e' una Fox;
  - in Hound: Tx spostata sopra 1000 Hz, Split acceso, «Salta Tx1» mostrato disattivato; doppio clic su una riga della Fox prepara sempre Tx1;
  - Wait and Pounce;
  - ALL.TXT con la banda del periodo ricevuto, scambi di contest nei record ADIF;
  - misura della decodifica FT8 sulle registrazioni vere (WAV copiati in `D:\build\wav-prova`, fuori dal repository).
  Non si sa se e' completo: va controllato.

## Prossimo passo

1. Nel ramo `resti-motore`: `cargo check --workspace` e `cd app && npx tsc --noEmit`; leggere il diff rispetto a `main`; completare le parti mancanti con i test.
2. Unirlo a `main` (il CHANGELOG di `main` ha gia' la sezione 0.11.60: le voci del ramo vanno sotto «## Non rilasciato»; eventuali conflitti in `docs/tasks.md` si risolvono tenendo le voci ancora aperte).
3. Novita' in sei lingue, Guida, versione 0.11.61 e rilascio con `docs/rilascio.md` (installatore compilato da solo, senza altre compilazioni).
4. Poi le voci aperte di `docs/tasks.md`: TCI, Q65, MSK144, pulsanti dei modi, QMAP, pagina Fox (bozza `docs/mockup-fox.html`), SuperFox.

## Da provare dal vivo (proprietario)

Nuovo menu Impostazioni e pagina Log, barre Rx/Tx, display Yaesu/Icom, colore DXpedition nel cluster, a7/a8, stop per ROS alto, invii a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP.

## Dove stanno le cose

- Codice: `D:\progetti\Aurora` (repository pubblico github.com/wifi75/Aurora). Compilazione dell'installatore: `D:\build\Aurora-0.11.1` (cartella di rilascio, si porta sul commit con `git checkout --detach`).
- Analisi di WSJT-X 3: `D:\build\analisi-wsjtx` (sorgenti e binari), `docs/analisi-wsjtx-3.md`.
- Chiave di firma degli aggiornamenti: `%USERPROFILE%\.aurora-signing\aurora.key` (fuori dal repository).
- Le copie di lavoro `D:\build\wt-*` diverse da `wt-resti1` e `wt-superfox` sono gia' unite a `main` e si possono cancellare.
