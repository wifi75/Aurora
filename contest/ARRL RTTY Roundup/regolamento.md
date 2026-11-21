# ARRL RTTY Roundup - regolamento riassunto

- **Fonte:** «ARRL RTTY Roundup Contest Rules», versione 3.0b (categorie 1.5 del 25 ago 2022, Rules for Entry 1.2 del 15 lug 2022): <https://contests.arrl.org/ContestRules/RTTY-RU-Rules.pdf>. Pagina: <https://www.arrl.org/rtty-roundup>.
- **Data di lettura:** 2026-11-21. Edizione: 2027.
- Le regole cambiano ogni anno: ricontrollare a ogni edizione.

## Date
- Primo fine settimana pieno di gennaio, mai il 1 gennaio; da sabato 18:00 a domenica 23:59 UTC (30 ore).
- Single op: 24 ore su 30, a partire dal primo QSO; una o due pause di almeno 30 minuti. Dopo le 24 ore si puo' continuare, ma quei QSO non contano. Multi-operatore: tutte le 30 ore.
- 2027: 2-3 gennaio; 2028: 8-9 gennaio (il 1 gennaio e' sabato). Date calcolate dalla regola: confermarle sul calendario ARRL.
- Log entro 7 giorni (alle 23:59 UTC), solo online su <https://contest-log-submission.arrl.org>.

## Bande e modo
- 80, 40, 20, 15, 10 m. **Solo RTTY** (nel Cabrillo `RY`); automazione vietata (avvio diretto da entrambi gli operatori). Gli altri modi digitali sono nell'ARRL International Digital Contest.

## Chi lavora chi
- Tutti lavorano tutti, una volta per banda.

## Scambio
- W/VE: RST + stato o provincia (sigle dell'elenco moltiplicatori ARRL). DX: RST + numero progressivo da 1.

## Punti e moltiplicatori
- 1 punto per QSO.
- Moltiplicatori **una volta sola, non per banda**: entita' DXCC (esclusi USA e Canada), stati USA + DC, province e territori canadesi + Labrador. Hawaii (KH6) e Alaska (KL7) contano come entita' DXCC. Tutte le stazioni contano gli stessi moltiplicatori.
- Punteggio = punti x moltiplicatori.

## Categorie
- Single Operator (QRP 5 W, Low 100 W, High 1500 W), Single Operator Unlimited (con spotting, stesse potenze), overlay Limited Antennas.
- Multioperator Single Transmitter (Low e High, 10 cambi di banda per ora), Two Transmitter (High, 6 cambi per ora per trasmettitore, il log dice il trasmettitore), Multitransmitter (High, un segnale per banda). Checklog.

## Penalita'
- Doppi tolti senza penalita'. Nominativo sbagliato o NIL: QSO tolto e penalita' pari ai punti del QSO.
- Mobili marittimi e aeronautici fuori da USA e Canada: solo credito di contatto (OPRG.5.1).

## Cosa fa Aurora e cosa no
- Punti, doppi per banda, moltiplicatori una volta sola (stati dallo scambio dei K e VE, paesi DXCC esclusi K e VE; la Sicilia e simili valgono come Italia), progressivo per i DX, Cabrillo `CONTEST: ARRL-RTTY`.
- Non fa: conteggio delle 24 ore e delle pause (resta il promemoria delle ore), penalita' NIL, limiti multi-operatore, il solo credito di contatto dei /MM.

## Differenze rispetto al vecchio `.contest`
1. Il vecchio file faceva il doppio senza guardare il modo e accettava tutti i modi digitali; ora solo RTTY (regolamento 3.0b).
2. Gli stati si prendevano dall'ultima parola dello scambio senza elenco: ora con l'elenco ARRL (`dati/stati.dom`).
3. Date del 2015: ora edizioni 2027 e 2028.

## Punti da verificare
- `CONTEST: ARRL-RTTY`: il PDF non lo scrive; e' il valore della specifica Cabrillo.
- Date 2027 e 2028 calcolate, non lette dal calendario.
