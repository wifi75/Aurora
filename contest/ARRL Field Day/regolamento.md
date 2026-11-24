# ARRL Field Day - regolamento riassunto

Esercitazione annuale dell'ARRL (con RAC) di giugno: si lavora in portatile, con alimentazione d'emergenza, con attenzione alla preparazione alle emergenze. Aperto a tutti; le stazioni fuori dalle aree ARRL/RAC e della Regione 2 IARU si lavorano per punti e possono mandare un solo check-log.

- **Fonte:** «ARRL Field Day 2026 Rules», revisione 4/2026 (nessuna modifica dall'anno prima), PDF <https://www.arrl.org/files/file/Field-Day/2026/2026-Field-Day-Rules.pdf> e pagina <https://www.arrl.org/field-day-rules>. Sezioni: «ARRL / RAC Section Abbreviation List» (revisione 2025) <https://www.arrl.org/files/file/Field-Day/Generic/ARRL-RAC%20Section%20List.pdf>.
- **Data di lettura:** 2026-11-24. Le regole cambiano: ricontrollare a ogni edizione.
- Cabrillo: non e' richiesto e il regolamento non scrive il nome (`ARRL-FD` e' quello della specifica): **da verificare**.

## Date
- **Quarto fine settimana pieno di giugno**, dalle **18:00 UTC del sabato alle 20:59 UTC della domenica** (27 ore). **2026: 27-28 giugno.** 2027: 26-27 giugno (calcolato, da confermare).
- Classi A e B: chi comincia l'allestimento prima delle 18:00 UTC del sabato lavora al massimo 24 ore consecutive; nessun allestimento prima delle 00:00 UTC del venerdi', al massimo 24 ore in tutto.
- Iscrizione (web app o posta) **entro il martedi' 28 luglio 2026**; riepilogo ufficiale, elenco dei QSO per banda e modo (un Cabrillo puo' sostituirlo ma non e' un'iscrizione) e prove dei bonus. I log completi non si mandano, ma si tengono un anno.

## Bande e modi
- 160, 80, 40, 20, 15 e 10 m piu' tutte le bande dai 50 MHz in su (le WARC e i 60 m non sono ammesse); il satellite e' una «banda» a parte; niente QSO via ripetitore, niente QSO in cross-band (salvo satellite).
- Fonia, CW e digitale (non CW). Ogni contatto si avvia da tutti e due gli operatori; i QSO automatici e il multi-flusso (Fox e Hound, DXpedition, MSHV) sono vietati; un solo segnale per banda e modo alla volta.

## Scambio
- Stazioni ARRL/RAC: **classe + sezione** (numero di trasmettitori simultanei + lettera, per esempio `3A CT`; in fonia «3 Alpha Connecticut»).
- Stazioni **DX**: classe + **DX** (`2A DX`). La stazione GOTA manda lo stesso scambio della stazione principale.
- Classi: **A** club o gruppo (almeno 3 persone) in portatile, in un cerchio di 300 m; **A-Battery** come la A ma a 5 W o meno a batteria; **B** una o due persone in portatile (e **B-Battery**); **C** mobile; **D** casa con la rete; **E** casa con alimentazione d'emergenza; **F** centro operativo d'emergenza (EOC). GOTA (una stazione in piu' per A e F, con un altro nominativo, 100 W al massimo) e stazione VHF libera (sopra i 50 MHz).

## Punti
- **QSO:** fonia **1**, CW **2**, digitale (non CW) **2** (7.1). Tutta la fonia e' un modo solo, tutto il digitale un modo solo (6.6, 6.7); ogni stazione una volta per banda e modo (6.3).
- **Moltiplicatore di potenza** (7.2), dalla potenza massima usata da qualunque trasmettitore: **5** se tutti i contatti sono a 5 W o meno e con alimentazione che non e' la rete ne' un generatore a motore (batterie, solare...); **2** a 5 W o meno ma con rete o generatore, oppure fino a 100 W; **1** oltre 100 W (massimo 500 W PEP per A, B, C; 100 W per D, E, F).
- **Bonus** (7.3), aggiunti dopo il moltiplicatore, con prove: alimentazione d'emergenza al 100% (100 per trasmettitore, fino a 2000), stampa (100), luogo pubblico (100), tavolo informativo (100), messaggio al Section Manager (100), messaggi gestiti (10 l'uno, al massimo 100), QSO via satellite (100), alimentazione alternativa (100), bollettino W1AW (100), attivita' educativa (100), visita di un politico (100), visita di un ente servito (100), contatti GOTA (5 l'uno, non moltiplicati) e allenatore GOTA (100), iscrizione via web (50), giovani fino a 18 anni (20 l'uno, al massimo 100), social media (100), responsabile della sicurezza per la classe A (100), responsabile del sito (50).
- **Punteggio = punti dei QSO x moltiplicatore di potenza + bonus.**

## Cosa fa Aurora e cosa no
- Punti dei QSO per modo (fonia 1, CW e digitale 2), doppi per banda e modo, bande ammesse, scambio classe + sezione con elenco delle sezioni.
- **Non fa:** il moltiplicatore di potenza (5/2/1) e nessun bonus (il motore dichiara i bonus per QSO e i bonus del Field Day non dipendono dal log: sono dichiarazioni sul riepilogo; il contatto GOTA da 5 punti non si distingue dal log); il punteggio mostrato e' quindi la somma dei punti dei QSO. Non fa nemmeno: modo unico per tutta la fonia e per tutto il digitale nei doppi (SSB, USB e FT8, PSK31 contano come modi diversi), QSO satellite, classi.

## Differenze rispetto al vecchio `.contest` («ARRL Fieldday»)
1. Stessi punti per modo; ora con le date 2026, le bande ammesse (niente WARC), la lista delle sezioni (con GH e TER) e lo scambio dichiarato.
2. Il vecchio script moltiplicava per la potenza leggendo QRP, LP o HP nello scambio mandato: il regolamento non lo prevede nello scambio, quindi si e' tolto (il moltiplicatore va applicato sul riepilogo).
3. Le macro mandano `<NRS> <MYSECT>`: la classe si scrive in «Numero da mandare» (senza incremento) e la sezione («DX» dall'Italia) in «La mia sezione / stato / club», non nelle macro.

## Altri contest simili (non convertiti qui)
- `Fieldday Contest.contest` (nome interno «Fieldday Contest», 2 punti per QSO, un moltiplicatore per paese e banda) e `IARU Region 1 Field Day.contest` (Field Day della Regione 1 IARU, punti 2/3/4/5/6) sono altri contest: restano file.

## Punti da verificare
- Il nome `CONTEST:` del Cabrillo e le intestazioni: il regolamento non le scrive.
- Le date del 2027 sono calcolate dalla regola («sempre il quarto fine settimana pieno di giugno»).
