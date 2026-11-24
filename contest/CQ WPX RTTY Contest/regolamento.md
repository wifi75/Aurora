# CQ WPX RTTY Contest - regolamento riassunto

- **Fonte:** <https://www.cqwpxrtty.com/rules.htm> (regolamento 2026), formato log <https://www.cqwpxrtty.com/cabrillo.htm>.
- **Data di lettura:** 2026-11-24. Edizione di riferimento: 2026 (14-15 febbraio); date 2027 calcolate.
- Le regole cambiano ogni anno: ricontrollare a ogni edizione.

## Date
- 48 ore, da sabato 00:00 a domenica 23:59 UTC. 2026: 14-15 febbraio (lette). 2027: 13-14 febbraio, calcolate (secondo fine settimana pieno di febbraio, ricavato dalla data 2026): da confermare.
- Single op: 30 ore su 48, pause di almeno 60 minuti senza QSO. Multi-operatore: tutte le 48 ore.
- Log entro 48 ore dalla fine (2026: 17 febbraio 23:59 UTC), solo caricamento web in Cabrillo.

## Bande e modo
- 3,5, 7, 14, 21, 28 MHz (80-10 m). Solo RTTY, 45,45 baud, 170 Hz.

## Scambio
- RST + numero progressivo da 001. Multi-Two, Multi-Unlimited e Multi-Distributed usano una sequenza per banda.

## Punti e moltiplicatori
- Punti (ogni stazione una volta per banda): 14, 21, 28 MHz altro continente 3, stesso continente altro paese 2, stesso paese 1; su 3,5 e 7 MHz 6, 4, 2.
- Moltiplicatore: prefissi (es. N8, HG19, LY1000), uno una volta sola a prescindere dalla banda. Portatili: conta il designatore portatile (PA/N8BJQ = PA0); senza numero si aggiunge zero dopo la seconda lettera. /MM, /M, /A, /E, /J, /P e simili non sono prefissi.
- Punteggio = punti x prefissi.

## Categorie
- Single Operator (QRP 5 W, Low 100 W, High 1500 W), tutte le bande o una banda. Overlay (nel Cabrillo CATEGORY-OVERLAY): TB-WIRES, ROOKIE, CLASSIC (24 ore su 48, niente alert), YOUTH.
- Multi-Operator solo tutte le bande: Multi-One (10 cambi di banda per ora), Multi-Two (8 per trasmettitore, il log dice il trasmettitore), Multi-Unlimited (cinque trasmettitori, uno per banda), Multi-Distributed. Checklog.

## Penalita' (controllo dei log)
- Doppi e scambio ricevuto sbagliato: QSO tolto, nessuna penalita'. Nominativo sbagliato o non trovato nell'altro log: QSO tolto piu' penalita' pari ai suoi punti.

## Cosa fa Aurora e cosa no
- Punti, doppi per banda, prefissi WPX una volta sola, progressivo, Cabrillo `CONTEST: CQ-WPX-RTTY`.
- Non fa: conteggio delle 30 ore e delle pause, limiti di cambi banda, penalita' NIL, sequenza di numeri per banda.

## Differenze rispetto al vecchio `.contest`
1. Date del 2014: ora edizioni 2026 e 2027. Punti e moltiplicatori erano gia' giusti.
2. Solo RTTY (il vecchio file accettava ogni modo digitale), doppi per banda esplicito.

## Punti da verificare
- Date 2027 calcolate. `CATEGORY-BAND` e `CATEGORY-OVERLAY` letti solo in parte dalla pagina Cabrillo (valori presi dal regolamento).
- Il regolamento letto non dice se «stesso paese» usi anche l'elenco WAE: Aurora usa il DXCC.
