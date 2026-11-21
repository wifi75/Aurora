# UBA DX SSB Contest - regolamento riassunto

- **Fonte:** «UBA DX Contest», <https://www.uba.be/en/hf/contest-rules/uba-dx-contest> (UBA; pagina senza data di revisione, edizione 2027).
- **Data di lettura:** 2026-11-21.
- Le regole cambiano: ricontrollare a ogni edizione. Domande: contest@uba.be.

## Date
- SSB: ultimo fine settimana di gennaio, sabato 13:00 - domenica 13:00 UTC (24 ore). **2027: 30-31 gennaio** (la sezione 2 della pagina scrive «January 30-31» con l'anno 2026, ma e' il calendario del 2027; le scadenze dei log sono del 2027). 2028: 29-30 gennaio (calcolata).
- Log entro 83 ore dalla fine (2027: 3 febbraio 23:59 UTC), caricamento su <https://contestupload.uba.be/>, solo Cabrillo.
- Il CW e' una gara a parte (ultimo fine settimana di febbraio, cartella «UBA DX CW Contest»).

## Bande e modo
- 80, 40, 20, 15, 10 m, nei segmenti contest IARU. Solo SSB, chiamata «CQ UBA».

## Scambio
- Belgi: RS + numero da 001 + sezione UBA (59001 ACC). Non soci: XXX. ON4UBA, ON4DIG, ON4YLC: UBA.
- Gli altri: RS + numero da 001.

## Punti per QSO
- Fuori dal Belgio: con un belga 10, con un paese UE 3, altri 1.
- Belgi: con un belga 1, con un paese UE 2, altri 3.
- Paesi UE del regolamento: 5B, 9A, 9H, CT, CT3, CU, DL, EA, EA6, EA8, EI, ES, F, FG, FM, FR, FY, HA, I, IS, LX, LY, LZ, OE, OH, OH0, OJ0, OK, OM, OZ, PA, S5, SM, SP, SV, SV5, SV9, SV/A, TK, YL, YO.
- Russia e Bielorussia escluse dalle attivita' UBA: 0 punti e 0 moltiplicatori.

## Moltiplicatori (per banda)
- Belgi: ogni paese DXCC e ogni sezione UBA (XXX no).
- Gli altri: ogni sezione UBA (XXX no), ogni prefisso belga (ON4, OO5, OP1, OQ, OR, OS, OT...), ogni paese UE dell'elenco. Un QSO belga puo' dare due moltiplicatori.

## Bonus (solo fuori dal Belgio)
- Punti dei QSO belgi x la loro percentuale sui QSO validi, sommato ai punti. Esempio del regolamento: 50 QSO belgi (500 punti) su 320 = 15,6% -> 78.

## Formula
- (punti + bonus) x moltiplicatori di tutte le bande.

## Categorie
- Belgi: A (6 ore), B (12 ore), C (24 ore), ognuna High o Low; D multi-operatore; E QRP 18 ore; BASE (ON3, 24 ore); F SWL.
- Gli altri: monobanda A10/A15/A20/A40/A80 High o Low, CHP/CLP singolo operatore tutte le bande, D multi-operatore, E QRP 18 ore, F SWL.
- Potenza: High oltre 100 W, Low 100 W, QRP 10 W in SSB. D: regola dei 10 minuti; la stazione dei moltiplicatori solo su un'altra banda.

## Cabrillo
- `CONTEST: UBA-DX-SSB`: la pagina mostra solo l'esempio CW (`UBA-DX-CW`); il nome SSB viene dall'elenco dei nomi Cabrillo di WA7BNM (<https://www.contestcalendar.com/cabnames.php>, fonte secondaria). Righe obbligatorie anche `MODE` (SSB) ed `EMAIL`; i belgi `CATEGORY-TIME` (6, 12 o 24 ore).

## Controllo dei log
- Scambio incompleto o sbagliato, QSO non nel log dell'altro: 0 punti, senza altra penalita'.

## Cosa fa Aurora e cosa no
- Punti, doppi per banda, sezioni (XXX esclusa), prefissi belgi (prefisso WPX), paesi UE e paesi DXCC per i belgi, bonus, Cabrillo con `CONTEST: UBA-DX-SSB`.
- Non fa: controllo incrociato dei log, limiti delle categorie (ore, 10 minuti), `CATEGORY-TIME`, `EMAIL` e potenza (a mano nell'intestazione), elenco chiuso delle sezioni UBA (vale la parola dello scambio).

## Differenze rispetto al vecchio `.contest` («UBA SSB Contest»)
1. Stesse regole del punteggio; ora con le date del 2027 e l'elenco dello scambio dichiarato.

## Punti da verificare
- Russia e Bielorussia: Aurora le tratta come QSO non ammessi, quindi non entrano nei QSO validi su cui si calcola la percentuale del bonus (il regolamento dice solo 0 punti e 0 moltiplicatori).
- Nessuna regola dei doppi scritta: si tiene una stazione per banda.
- Gli stranieri dell'elenco UE contano 3 punti anche nel proprio paese (un italiano con un italiano): il regolamento non lo esclude.
- Date 2028 calcolate.
- `CONTEST: UBA-DX-SSB` da fonte secondaria.
