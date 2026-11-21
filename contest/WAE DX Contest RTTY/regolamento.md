# WAE DX Contest RTTY - regolamento riassunto

Worked All Europe DX Contest del DARC, parte RTTY.

- **Fonte:** «Worked All Europe DX Contest - Rules 2025», <https://www.darc.de/der-club/referate/conteste/wae-dx-contest/en/wae-rules/> (pagina del 2025-06-17, con le date del 2026), in particolare il punto 11 per la RTTY; valgono anche le regole generali dei contest DARC.
- **Data di lettura:** 2026-11-21.
- Nome Cabrillo `DARC-WAEDC-RTTY`: non scritto dal DARC; viene dall'elenco dei nomi Cabrillo di WA7BNM (<https://www.contestcalendar.com/cabnames.php>, fonte secondaria).
- Il meccanismo dei QTC e la finestra di Aurora sono descritti in `docs/contest-qtc.md`.

## Date
- Secondo fine settimana pieno di novembre, sabato 00:00 - domenica 23:59 UTC. **2026: 14-15 novembre** (regolamento). 2027: 13-14 novembre (calcolata).
- Singolo operatore: al massimo 36 ore su 48, pause di almeno 60 minuti senza QSO ne' QTC.
- Log Cabrillo entro sette giorni (2026: 22 novembre 23:59 UTC).

## Bande e modo
- 80, 40, 20, 15, 10 m nei segmenti preferiti per i contest della Regione 1 IARU (in RTTY a 20 m il segmento contest comprende 14099-14101 kHz).

## Chi lavora chi
- Tutti lavorano tutti, senza limiti di continente.

## Scambio
- RST + numero progressivo da 001 (000 se non mandato: il QSO vale solo con un moltiplicatore nuovo). Ogni stazione una volta per banda.

## Punti
- Ogni QSO e ogni QTC (mandato o ricevuto) vale 1 punto.

## QTC
- Tutti possono mandarli e riceverli, ma solo fra continenti diversi. Fra due stazioni al massimo 10 QTC in tutto (mandati + ricevuti). Ogni QSO una volta sola, mai alla stazione con cui e' stato fatto; serie da 1 a 10.

## Moltiplicatori (per banda, pesati)
- I moltiplicatori europei e non europei del punto 7 valgono per tutte le stazioni: ogni paese dell'elenco WAE e ogni paese DXCC non europeo, con le aree numeriche per W, VE, VK, ZL, ZS, JA, BY, PY e RA8, RA9, RA0.
- Peso: 80 m x4, 40 m x3, 20/15/10 m x2.

## Formula
- (QSO + QTC) x somma dei moltiplicatori pesati.

## Categorie
- Single-Op Low (100 W), Single-Op High, Multi-Op (cambi di banda ogni 10 minuti). Tutto entro un cerchio di 2 km.

## Cosa fa Aurora e cosa no
- 1 punto per QSO, doppi per banda, paesi WAE (europei) e paesi DXCC o aree (non europei) per tutti, pesi delle bande, QTC in tutte e due le direzioni fra continenti diversi (`qtc = 2`) sommati ai punti, righe `QTC:` nel Cabrillo con `CONTEST: DARC-WAEDC-RTTY`.
- Non fa: la regola del 000, limite delle 36 ore, regola dei 10 minuti.

## Differenze rispetto al vecchio `.contest` («WAEDC (Worked All Europe DX Contest)», RTTY)
1. L'elenco WAE non era distinto (IT9, GM/s, TA1, JW/b, 4U1V col paese DXCC): ora sono paesi a parte.
2. Il vecchio file resta: ha lo stesso nome del file CW e SSB e una cartella non puo' sostituirne uno solo dei due.

## Punti da verificare
- Il proprio paese come moltiplicatore e i QSO nel proprio paese: il regolamento non ne parla (Aurora li conta, 1 punto e moltiplicatore).
- Regola del 000: non applicata.
- Nome Cabrillo da fonte secondaria; date 2027 calcolate.
