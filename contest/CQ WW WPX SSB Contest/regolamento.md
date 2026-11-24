# CQ WW WPX Contest SSB - regolamento riassunto

- **Fonte:** «2026 CQ World-Wide WPX Contest», <https://www.cqwpx.com/rules.htm> (la pagina vale per CW e SSB; nessuna data di revisione). Formato log: <https://www.cqwpx.com/cabrillo.htm>.
- **Data di lettura:** 2026-11-24. Edizione di riferimento: 2026 (SSB 28-29 marzo); date 2027 calcolate.
- Le regole cambiano ogni anno: ricontrollare a ogni edizione.

## Date
- 48 ore, da sabato 00:00 a domenica 23:59 UTC. SSB 2026: **28-29 marzo**: la pagina scrive «March 28-29, 2025», un refuso (la scadenza dei log e' il 31 marzo 2026 e il 28-29 marzo 2026 e' sabato-domenica; in caso di dubbio si chiede al direttore del contest). 2027: 27-28 marzo, calcolate (ultimo fine settimana pieno di marzo): da confermare. Il CW ha la sua cartella (fine maggio).
- Single op: 36 ore su 48, pause di almeno 60 minuti senza QSO nel log. Multi-operatore: tutte le 48 ore.
- Log entro le 23:59 UTC del 31 marzo 2026 (2 giorni dopo la fine), solo caricamento web in Cabrillo; una proroga si chiede per email prima della scadenza.

## Bande e modo
- 1,8, 3,5, 7, 14, 21, 28 MHz (160-10 m), senza WARC; rispettare il band plan. Solo fonia (SSB) per questo fine settimana.
- Regione 1 ITU: niente sotto 1810 kHz nei fine settimana del contest e niente sopra 7200 kHz sui 40 m nel fine settimana della fonia (limite non controllato da Aurora, che non sa la regione).

## Scambio
- RS + numero progressivo (esempio del Cabrillo: 59 001) da 001. Multi-Two, Multi-Unlimited e Multi-Distributed usano una sequenza per banda. Tutti lavorano tutti.

## Punti per QSO (ogni stazione una volta per banda)
- Altro continente: 3 su 28/21/14 MHz, 6 su 7/3,5/1,8 MHz.
- Stesso continente, paese diverso: 1 e 2.
- Fra due stazioni del Nord America (paesi diversi): 2 e 4.
- Stesso paese: 1 punto su ogni banda.

## Moltiplicatori
- Prefissi: ogni prefisso conta una sola volta, a prescindere da banda e numero di QSO. Portatili: conta il designatore portatile (PA/N8BJQ = PA0); senza numero si aggiunge zero dopo la seconda lettera. Marittimo mobile, mobile, `/A`, `/E`, `/J`, `/P` e altri identificatori di classe di licenza non contano come prefissi.

## Formula
- Punti dei QSO x numero di prefissi diversi.

## Doppi
- Si tolgono nel controllo dei log, senza penalita'.

## Categorie
- Single Operator (nessun limite di cambi banda, un solo segnale): High 1500 W, Low 100 W, QRP 5 W, tutte le bande o una banda.
- Overlay del single operator (conteggio tutte le bande, riga `CATEGORY-OVERLAY`): TB-WIRES (un tribanda 10-20 m e antenne semplici 40-160 m), ROOKIE (licenza da tre anni o meno, data nel SOAPBOX), CLASSIC (una radio, fino a 24 ore, nessun avviso dei QSO), YOUTH (25 anni o meno all'inizio, data di nascita nel SOAPBOX).
- Multi-operatore, solo tutte le bande, 1500 W per segnale: Multi-One (un segnale, 10 cambi di banda per ora d'orologio, un'unica sequenza di numeri; High o Low), Multi-Two (due segnali su bande diverse, 8 cambi per trasmettitore, sequenza per banda, il log dice il trasmettitore), Multi-Unlimited (fino a sei segnali, uno per banda), Multi-Distributed (fino a sei segnali, stesso paese DXCC e stessa zona CQ; `CATEGORY-STATION: DISTRIBUTED`). Checklog.
- L'aiuto per trovare i QSO (cluster, skimmer) e' ammesso in tutte le categorie tranne Classic.

## Cabrillo
- `CONTEST: CQ-WPX-SSB` (con i trattini, senza l'anno). `CATEGORY-OPERATOR` SINGLE-OP / MULTI-OP / CHECKLOG, `CATEGORY-ASSISTED`, `CATEGORY-MODE` SSB, `CATEGORY-BAND` ALL o 160M...10M, `CATEGORY-POWER` HIGH/LOW/QRP, `CATEGORY-TRANSMITTER` ONE/TWO/UNLIMITED (multi-operatore), `CATEGORY-OVERLAY` CLASSIC/ROOKIE/TB-WIRES/YOUTH. Le stazioni USA mettono lo stato in `LOCATION`.
- Riga: `QSO: freq mo data ora call rst numero call rst numero t` (t solo per Multi-Two).

## Penalita' del controllo dei log
- Nominativo sbagliato: costa il doppio dei punti del QSO. Squalifica per comportamento antisportivo.

## Cosa fa Aurora e cosa no
- Punti per banda e continente, doppi per banda, prefissi WPX una volta sola, progressivo, Cabrillo con `CONTEST: CQ-WPX-SSB`.
- Non fa: conteggio delle 36 ore e delle pause, limiti di cambi banda, sequenza di numeri per banda, penalita' sui nominativi sbagliati, esclusione dei prefissi dei /MM.

## Differenze rispetto al vecchio `.contest` («CQ WW WPX SSB-CW CONTEST»)
1. Date del 2015 (e SSB e CW insieme): ora le edizioni 2026 e 2027 e solo fonia.
2. Punti e moltiplicatori erano gia' giusti.
3. Il vecchio file valeva per ogni modo («SSB + CW»): ora solo fonia, doppi per banda esplicito, e il CW ha la sua cartella.

## Punti da verificare
- Anno del 2026 sulla pagina (2025): letto come refuso. Date 2027 calcolate, non ancora sul sito.
- Il regolamento non dice nulla sul punteggio dei QSO con stazioni /MM; per il prefisso dice che non contano.
- La pagina dei Cabrillo elenca `CATEGORY-ASSISTED`, il regolamento 2026 no (dice che l'aiuto e' ammesso ovunque tranne Classic): campo facoltativo.
