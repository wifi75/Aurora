# CQ WW RTTY DX Contest - regolamento riassunto

- **Fonte:** regolamento 2026 del CQ World Wide RTTY DX Contest, <https://www.cqwwrtty.com/rules.htm>. Cabrillo: <https://www.cqwwrtty.com/cabrillo.htm>.
- **Data di lettura:** 2026-11-21. Edizione 2026 (26-27 settembre, gia' passata); prossima 2027.
- Le regole cambiano ogni anno: ricontrollare a ogni edizione.

## Date
- Ultimo fine settimana pieno di settembre, da sabato 00:00:00 a domenica 23:59:59 UTC. **2026: 26-27 settembre** (regolamento). 2027: 25-26 settembre (calcolata, da confermare).
- Log entro 48 ore dalla fine (2026: 29 settembre 23:59 UTC), caricamento web, Cabrillo.

## Bande e modo
- 80, 40, 20, 15, 10 m (niente 160 m). Solo RTTY 45,45 baud, shift 170 Hz, ITA2.

## Scambio
- RST + zona CQ (599 05). Le stazioni degli USA contigui e del Canada aggiungono lo stato o la provincia.
- Cabrillo: rst, zona e QTH; fuori da W/VE il QTH e' `DX` (esempio della pagina Cabrillo).

## Punti per QSO (ogni stazione una volta per banda)
- Altro continente 3, stesso continente e paese diverso 2, stesso paese 1. Nessuna eccezione per il Nord America.

## Moltiplicatori (per banda)
- Zone CQ (standard WAZ); i mobili marittimi contano solo per la zona.
- Paesi: lista DXCC, lista WAE piu' IG9/IH9, confini continentali.
- QTH W/VE: 48 stati contigui + DC (sigle USPS) e 14 aree canadesi: NB (VE9), NS (VE1), QC (VE2), ON (VE3), MB (VE4), SK (VE5), AB (VE6), BC (VE7), NWT (VE8), NF (VO1), LB (VO2), NU (VY0), YT (VY1), PEI (VY2). Alaska e Hawaii solo come paesi.

## Formula
- Punti x (zone + paesi + QTH W/VE). Esempio del regolamento: 1000 x (30 + 70 + 35) = 135.000.

## Categorie
- Single Operator High (1500 W) / Low (100 W) / QRP (5 W), all band o single band; Single Operator Assisted con le stesse potenze.
- Overlay Classic (una radio, 24 ore su 48, pause di almeno 60 minuti), Rookie, Youth (riga `CATEGORY-OVERLAY`).
- Multi-Single High/Low (run + un segnale per moltiplicatori, 8 cambi di banda per ora per segnale), Multi-Two (1500 W, 8 cambi per ora per segnale), Multi-Multi (1500 W), Multi-Distributed (`CATEGORY-STATION: DISTRIBUTED`). Checklog.

## Penalita' del controllo dei log
- Scambio sbagliato: QSO tolto senza penalita'. Nominativo sbagliato o NIL: tolto, penalita' di due volte i punti del QSO. Doppi tolti senza penalita'.

## Cosa fa Aurora e cosa no
- Punti con il paese DXCC/WAE, zone dallo scambio (altrimenti dal database), paesi, QTH dei K e VE (`dati/qth.dom`, sinonimi NT, PE, NL), /MM solo zona, doppi per banda, Cabrillo `CONTEST: CQ-WW-RTTY`.
- Non fa: penalita' NIL, limiti delle categorie multi-operatore, `LOCATION` nell'intestazione, la parola `DX` al posto del QTH nelle righe (lo scambio mandato si scrive come lo si vuole nel file).

## Differenze rispetto al vecchio `.contest`
1. Punti con il paese DXCC soltanto: I e IT9 erano lo stesso paese (1 punto); ora sono due paesi (2 punti) e IT9 e' un moltiplicatore.
2. Zone dal database dei paesi: ora dallo scambio.
3. QTH W/VE senza elenco: qualsiasi parola valeva. Ora solo le sigle del regolamento (con NWT, NF, LB, PEI).
4. Date del 2015: ora regola ed edizioni 2026-2027.

## Punti da verificare
- Il regolamento non dice in chiaro se gli Stati Uniti e il Canada contano anche come **paese** (oltre ai QTH): Aurora li conta, perche' sono entita' DXCC.
- Le sigle che arrivano nei log reali (NT, PE, NL, VE8...): Aurora accetta NT, PE e NL come sinonimi; altre no.
