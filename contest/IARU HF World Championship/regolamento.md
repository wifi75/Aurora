# IARU HF World Championship - regolamento riassunto

Campionato mondiale HF della IARU, organizzato dall'ARRL: CW e SSB nello stesso fine settimana, una sola gara.

- **Fonte:** «IARU HF World Championship Rules», versione 1.21, <https://contests.arrl.org/ContestRules/IARU-HF-Rules.pdf> (ARRL); pagina del contest <https://www.arrl.org/iaru-hf-world-championship>.
- **Data di lettura:** 2026-11-24. Edizione: 2026 (41a, 11-12 luglio, gia' svolta); la prossima, 2027, e' calcolata dalla regola.
- Nome Cabrillo `IARU-HF`: il regolamento non lo scrive; viene dall'elenco dei nomi Cabrillo della WWROF (<https://wwrof.org/cabrillo/cabrillo-v3-header/>), che lo riporta come esempio.
- Le regole cambiano: ricontrollare a ogni edizione.

## Date
- Secondo fine settimana pieno di luglio, da sabato 12:00 UTC a domenica 11:59 UTC (24 ore). Nessun limite di ore operabili in nessuna categoria. **2026: 11-12 luglio.** 2027: 10-11 luglio (calcolata, da confermare sul calendario ARRL).
- Log entro sette giorni dalla fine (11:59 UTC), solo caricamento web su <https://contest-log-submission.arrl.org> (dal 30 settembre 2026 niente piu' logs cartacei, secondo la pagina del contest).

## Bande e modi
- 1,8, 3,5, 7, 14, 21 e 28 MHz, senza WARC. CW e fonia. Ogni stazione una volta **per banda e per modo**.
- Si lavora nella parte di banda accettata per il modo e si rispettano i segmenti per contest dei band plan regionali.

## Scambio
- Tutti: rapporto (RS o RST) + zona ITU.
- Stazioni HQ delle societa' IARU: la sigla della societa' al posto della zona (W1AW manda ARRL); l'ufficio internazionale IARU (NU1AW) manda IARU.
- Membri del Consiglio Amministrativo IARU e dei comitati esecutivi regionali: AC, R1, R2, R3.
- Esempio dal regolamento: `G3BJ 59 8` / `59 27`; HQ: `N1SFE 599 ARRL` / `599 8`.

## Punti per QSO
- Stessa zona ITU: 1 (anche se in un altro continente). Stazione HQ o ufficiale IARU: 1.
- Stesso continente, altra zona: 3. Altro continente e altra zona: 5.

## Moltiplicatori (per banda, non per modo)
- Zone ITU lavorate su ogni banda piu' stazioni HQ delle societa' IARU lavorate su ogni banda. Gli ufficiali valgono al massimo quattro moltiplicatori per banda (AC, R1, R2, R3).
- HQ e ufficiali non contano come zona. Per essere moltiplicatori AC e comitati regionali devono essere operati dal titolare della licenza (singolo operatore o multi con un solo trasmettitore con partecipazione significativa del titolare).

## Formula
- Punti dei QSO x totale dei moltiplicatori.

## Doppi
- Una stazione una volta per banda e per modo; i doppi si tolgono senza altra penalita'.

## Categorie
- Single Operator (SO: niente aiuto di segnalazione, un solo segnale), Single Operator Unlimited (SOU: segnalazioni pubbliche ammesse), con potenza QRP (5 W), Low (100 W) o High (1500 W); ciascuna anche Mixed, solo CW o solo fonia; overlay Youth (25 anni o meno).
- Multi-operatore solo in misto: Multi Single Transmitter (MS, sempre High; 10 minuti su una banda e modo prima di cambiare; segnalazioni ammesse) e Multi Two Transmitter (M2, 100 W al massimo, due segnali su bande diverse, nessun limite di cambi; senza segnalazioni; il log dice quale trasmettitore ha fatto ogni QSO).
- Stazioni HQ di societa' IARU: un solo segnale per banda e modo, tutte nella stessa zona ITU, un solo nominativo HQ per societa' per banda e modo. Checklog.

## Penalita' del controllo dei log
- Doppi tolti senza penalita'. Nominativo sbagliato o NIL: QSO tolto e penalita' pari ai punti del QSO. Scambio ricevuto sbagliato: QSO tolto senza penalita'.

## Cabrillo
- `CONTEST: IARU-HF`; `CATEGORY-OPERATOR` SINGLE-OP / MULTI-OP / CHECKLOG, `CATEGORY-BAND` ALL, `CATEGORY-POWER`, `CATEGORY-MODE` MIXED / CW / SSB, `CATEGORY-STATION` HQ, `CATEGORY-OVERLAY` YOUTH. I valori delle categorie sono quelli generali del formato Cabrillo (il regolamento rimanda a wwrof.org/cabrillo): da verificare sull'applicazione di caricamento.
- Multi-operatore: elenco completo degli operatori in `OPERATORS:`; per M2 il trasmettitore di ogni QSO.

## Cosa fa Aurora e cosa no
- Punti (zona dallo scambio ricevuto, sigle HQ e ufficiali), moltiplicatori per banda (zone e sigle), doppi per banda e modo, Cabrillo con `CONTEST: IARU-HF`.
- Non fa: penalita' NIL e nominativi sbagliati, regola dei 10 minuti, limiti delle categorie multi-operatore, trasmettitore di ogni QSO per M2, limite dei segmenti di frequenza (la banda si controlla, il segmento no).

## Differenze rispetto al vecchio `.contest` («IARU HF World Championship», CW e SSB)
1. Il vecchio file aveva le date del 2016 e il doppio per tutte le chiavi: ora la finestra e' quella del regolamento e il doppio e' per banda e modo.
2. La propria zona ITU si legge dal numero scritto nello scambio mandato (se manca, dal database dei paesi): un W6 o un W8 conta la zona giusta.

## Punti da verificare
- Il nome `IARU-HF` e i valori `CATEGORY-*` di Cabrillo (non scritti dal regolamento).
- Date 2027 calcolate, non ancora sul calendario ARRL.
- Stazioni di soli ufficiali AC/R1/R2/R3 o HQ in una zona uguale alla propria: il regolamento dice 1 punto in ogni caso.
