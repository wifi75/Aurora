# Japan International DX Contest CW - regolamento riassunto

- **Fonte:** «Japan International DX Contest Rule», edizione «2026-10», <https://jidx.org/jidxrule-e.html>; Cabrillo e log per email: <https://jidx.org/jidxelog-e.html>; elenco delle prefetture: <https://jidx.org/jidxmult.lst>.
- **Data di lettura:** 2026-11-24. Edizione di riferimento: CW 2026 (11-12 aprile); le date 2027 e 2028 sono lette sulla stessa pagina.
- Le regole cambiano: ricontrollare a ogni edizione. Ultime modifiche dichiarate: 2023-04 (niente log su carta), 2022-02 (categoria M2).

## Date
- CW: secondo fine settimana pieno di aprile, da sabato 07:00 a domenica 13:00 UTC (30 ore). **2026: 11-12 aprile; 2027: 10-11 aprile; 2028: 8-9 aprile** (tutte lette).
- Log entro un mese dalla fine (CW 2026: 13 maggio), proroga fino a due settimane se richiesta per email prima della scadenza.

## Bande e modo
- 1,8, 3,5, 7, 14, 21, 28 MHz, senza le bande WARC. Solo CW per questo contest (CW e fonia sono due weekend separati).
- I JA su 1,8 e 3,5 MHz hanno porzioni riservate: 1800-1875 e 1907,5-1912,5 kHz; 3500-3580, 3599-3612, 3662-3687, 3702-3716, 3745-3770, 3791-3805 kHz.

## Scambio
- JA: RST + numero della prefettura (01-50). Gli altri: RST + zona CQ.
- La stessa stazione si lavora una volta per banda. Niente QSO fra modi o bande diverse e niente ripetitori.

## Punti per QSO
- Contano solo i QSO fra un JA e un DX (compresi i /MM). DX-DX, DX-/MM e JA-JA non valgono punti ne' moltiplicatori.
- 1,8 MHz 4 punti; 3,5 MHz 2; 7, 14 e 21 MHz 1; 28 MHz 2.

## Moltiplicatori (per banda)
- Per i JA: i paesi DXCC (esclusa JD1) e le zone CQ. Un /MM vale per i punti e la zona, non per il paese.
- Per gli altri: le prefetture giapponesi piu' le isole di JD1 (Ogasawara, Minami-Torishima, Okino-Torishima), al massimo 50 per banda (elenco `jidxmult.lst`).

## Formula
- Punti dei QSO x moltiplicatori totali. Esempio del regolamento: 100 punti x 59 moltiplicatori = 5900.

## Categorie
- Un solo ingresso per persona, qualunque nominativo; un solo segnale alla volta (tranne il multi-operatore). Self-spotting vietato; le reti di spot sono ammesse.
- Single operator High (oltre 100 W): AB (tutte le bande), 28, 21, 14, 7, 3.5, 1.8. Single operator Low (100 W o meno, QRP compreso): ABL, 28L...1.8L. Multi-operatore: M1 (un trasmettitore) e M2 (due). Maritime Mobile: MM (senza premi). Nessuna categoria QRP, e niente «/QRP» nel nominativo.
- Stazione fissa: non si cambia luogo durante il contest (tranne i MM). I JA che operano da un'altra area firmano «/#».

## Cabrillo
- `CONTEST: JIDX-CW` (da <https://jidx.org/jidxelog-e.html>: «either JIDX-PH or JIDX-CW»). Intestazione di quella pagina: `CATEGORY` con le abbreviazioni sopra, `CLAIMED-SCORE`, `OPERATORS`, `ENTITY`, `CREATED-BY`, `NAME`, `ADDRESS`, `SOAPBOX`, `EMAIL`, `SENTNR` (la propria zona CQ).
- Invio: caricamento web (<https://jidx.org/upload/uplog.html>) o email come allegato a cw@jidx.org con «JIDX» e il nominativo nell'oggetto; il file si chiama «nominativo.log».

## Cosa fa Aurora e cosa no
- Punti per banda, moltiplicatori (prefetture per i DX, paesi e zone per i JA), doppi per banda, /MM, Cabrillo con `CONTEST: JIDX-CW`.
- Non fa: controllo dei log degli altri, categorie M1/M2, porzioni di banda dei JA.

## Differenze rispetto al vecchio `.contest`
1. Il vecchio file valeva per un solo modo con le date del 2026; qui due cartelle (CW, fonia) con le date dal regolamento, anche 2027 e 2028.
2. Il `/MM` conta: un JA lo lavora per punti e zona. JD1 sta dalla parte del Giappone (prefetture 48-50 per i DX).
3. Le zone dei JA vengono dallo scambio ricevuto, non solo dal database dei paesi; le prefetture dei DX dallo scambio, con un elenco `.dom` di 50 voci.
4. Le macro non hanno piu' valori fissi: la zona o la prefettura propria si scrive nella scheda del contest.
5. Nome Cabrillo `JIDX-CW`: nel core (`nome_cabrillo` in `core/crates/contest/src/punti_testo.rs`) il vecchio nome della fonia e' `JIDX-PHONE`, mentre la pagina ufficiale scrive `JIDX-PH`. Da allineare quando il core sceglie il nome dalla cartella.

## Punti da verificare
- Un QSO JA-JD1 (o JD1-JA) vale punti? Il regolamento non lo dice: qui no.
