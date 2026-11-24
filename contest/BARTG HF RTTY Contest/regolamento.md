# BARTG HF RTTY Contest - regolamento riassunto

- **Fonte:** <https://bartg.org.uk/wp-content/uploads/2025/05/bartg-hf-rtty-rules-2025-v3.pdf> (BARTG, revisione del 19 maggio 2024, il regolamento piu' recente pubblicato). Calendario: <https://bartg.org.uk/?p=154>.
- **Data di lettura:** 2026-11-24. Edizione di riferimento: 2027 (date dal calendario BARTG).
- Le regole cambiano: ricontrollare a ogni edizione (non esiste un PDF 2026 o 2027).

## Date
- Terzo fine settimana pieno di marzo, da sabato 02:00 UTC a lunedi' 01:59 UTC (48 ore). 2026: 21-23 marzo (calcolata); 2027: 20-22 marzo; 2028: 18-20 marzo (dal calendario).
- Classi single operator (tranne SOAB6): al massimo 30 ore su 48, pause di almeno 3 ore. SOAB6: le prime 6 ore operate contano, il resto zero. Multi-operatore: tutte le 48 ore.
- Log entro 7 giorni dalla fine, in Cabrillo, al robot RSGB <http://bartg.rsgbcc.org/cgi-bin/hfenter.pl>. Log di Russia e Bielorussia: checklog (pagina del contest).

## Bande, modo e frequenze
- 80, 40, 20, 15, 10 m. Solo RTTY (Cabrillo `RY`).
- Segmenti (kHz): 80 m 3580-3615, 40 m 7040-7125, 20 m 14070-14125, 15 m 21070-21148, 10 m 28070-28189. Fuori segmento zero punti e moltiplicatori; su 14100 +/- 0,5 kHz (radiofaro) zero. Chi punta ai primi cinque posti deve registrare la frequenza vera di trasmissione (il MARK).

## Scambio
- RST + numero progressivo (tre o quattro cifre da 001, +1 a ogni contatto) + ora UTC di quattro cifre. Esempio: `G3XXX 599 156 1749`.

## Punti e moltiplicatori
- 1 punto per QSO completo; una stazione una volta per banda per punti e moltiplicatori.
- Moltiplicatori per banda: ogni paese DXCC (anche JA, W, VE, VK) e ogni area JA, W, VE, VK. Le stazioni USA fuori dall'area del proprio nominativo usano il suffisso (`W0XXX/5`, `KH6XXX/6`); senza suffisso il log diventa checklog.
- Continenti: moltiplicatore una volta sola, a prescindere dalla banda, al massimo 6.
- Punteggio = punti x moltiplicatori x continenti.

## Classi
- SOE (piu' radio), SOAB, SOAB100, SOABQRP, SOAB6, SS10/15/20/40/80 (una banda), MS (multi-op, una radio), MM (multi-op, piu' radio). Tutte assisted. Una radio sola: dopo un QSO su una banda non si cambia banda per 5 minuti (classi tutte le bande a radio singola).

## Cosa fa Aurora e cosa no
- Punti, doppi per banda, moltiplicatori per banda (paesi e aree), continenti a parte, progressivo, Cabrillo.
- Non fa: segmenti di frequenza per banda, ore e pause, cinque minuti per cambio banda, suffisso dei W, penalita'.

## Differenze rispetto al vecchio `.contest`
1. Date del 2023: ora edizioni 2026-2028 dal calendario; il vecchio file aveva un'ora d'inizio 02:00 e fine 01:59 di lunedi' gia' giusta.
2. Solo RTTY e doppi per banda dichiarati; scambio con l'ora UTC.

## Punti da verificare
- `CONTEST: BARTG-RTTY`: il regolamento non lo scrive; e' il valore della specifica Cabrillo.
- Il Cabrillo di esempio del sito non e' leggibile in automatico: formato della riga QSO (rst, numero, ora) da confrontare.
