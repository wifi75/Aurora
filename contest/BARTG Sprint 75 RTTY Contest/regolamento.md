# BARTG Sprint75 Contest - regolamento riassunto

- **Fonte:** <https://bartg.org.uk/wp-content/uploads/2026/02/bartg-sprint75-rules-2026-v1.pdf> (BARTG, revisione del 24 febbraio 2026). Calendario: <https://bartg.org.uk/?p=159>.
- **Data di lettura:** 2026-11-24. Edizione di riferimento: 2027 (25 aprile). Il secondo Sprint75 dell'anno (settembre) e' stato sostituito da uno Sprint PSK63, fuori da questa cartella.

## Date
- Quarta domenica di aprile, 17:00-20:59 UTC (4 ore). 2026: 26 aprile; 2027: 25 aprile; 2028: 23 aprile (2029: 22 aprile, non il quarto fine settimana pieno: per questo le date sono scritte).
- Log entro 7 giorni dalla fine, in Cabrillo, al robot RSGB <http://bartg.rsgbcc.org/cgi-bin/hfenter.pl>. Log di Russia e Bielorussia: checklog.

## Bande, modo e frequenze
- 80, 40, 20, 15, 10 m. Solo RTTY a **75 baud** (Cabrillo `RY`).
- Segmenti (kHz): 80 m 3580-3615, 40 m 7040-7125, 20 m 14070-14125, 15 m 21070-21148, 10 m 28070-28189. Fuori segmento zero punti e moltiplicatori; su 14100 +/- 0,5 kHz zero.

## Scambio
- Solo il numero progressivo (tre o quattro cifre da 001, +1 a ogni QSO). Niente RST. Esempio: `G3XXX 156`.

## Punti e moltiplicatori
- 1 punto per QSO; una stazione una volta per banda per punti e moltiplicatori.
- Moltiplicatori una volta sola nel contest: paesi DXCC e aree JA, W, VE, VK. Continenti una volta sola, al massimo 6.
- Punteggio = punti x moltiplicatori x continenti. Stazioni USA fuori area: suffisso `/<area>`; senza suffisso il log diventa checklog.

## Classi
- Solo single operator: SOE (piu' radio), SOAB, SOAB100, SOABQRP. Tutte assisted. Radio singola: 5 minuti prima di cambiare banda.

## Cosa fa Aurora e cosa no
- Punti, doppi per banda, moltiplicatori una volta sola, continenti, progressivo senza RST, Cabrillo. Le regole di punteggio sono identiche a quelle dello Sprint di gennaio.
- Non fa: i 75 baud (li imposta la macro `<BAUDRATE:75>`), segmenti di frequenza per banda, cinque minuti per cambio banda, suffisso dei W.

## Differenze rispetto al vecchio `.contest`
1. Date del 2017 (17 settembre): ora il Sprint75 di aprile, quarta domenica, 17:00-20:59.
2. Solo RTTY, doppi per banda dichiarati, scambio senza RST.

## Punti da verificare
- `CONTEST: BARTG-SPRINT75`: non e' scritto ne' nel regolamento ne' nella specifica Cabrillo; da confrontare con l'esempio di log del sito.
- Data 2026 (26 aprile) calcolata dalla regola della quarta domenica, non letta dalla pagina (che mostra solo 2027-2032).
