# CQ WW 160-Meter Contest SSB - regolamento riassunto

- **Fonte:** «CQ WW 160-Meter Contest Rules» edizione 2026, <https://www.cq160.com/rules.htm> (nessuna data di revisione sulla pagina). Formato log: <https://www.cq160.com/cabrillo.htm>.
- **Data di lettura:** 2026-11-24. Edizione di riferimento: 2026 (SSB 27 febbraio - 1 marzo); date 2027 calcolate.
- Le regole cambiano ogni anno: ricontrollare a ogni edizione. Il CW (23-25 gennaio 2026) e' un contest separato, con la cartella `CQ WW 160 Meter Contest CW`; regole uguali, cambiano date e scadenza dei log.

## Date
- 48 ore, dalle 22:00 UTC del venerdi' alle 22:00 UTC della domenica. SSB 2026: **dalle 22:00 UTC del 27 febbraio alle 22:00 UTC del 1 marzo** (lette). 2027: 26-28 febbraio, calcolate (ultimo venerdi' di febbraio): da confermare.
- Single op: massimo 30 ore su 48; multi-operatore massimo 40 ore. Pause di almeno 30 minuti in ogni categoria.
- Log entro 5 giorni dalla fine (SSB 2026: 6 marzo 22:00 UTC), solo caricamento Cabrillo su cq160.com/logcheck.

## Banda e modo
- Solo 160 m: 1810-2000 kHz in Regione 1 ITU, 1800-2000 kHz nelle Regioni 2 e 3. Rispettare i limiti del proprio paese e i modi digitali (FT8) nella banda.
- Solo fonia (SSB) per questo fine settimana.

## Scambio
- RS piu': stato USA per le stazioni W, provincia per le canadesi, zona CQ per i DX. Le zone sono solo indicative e non sono moltiplicatori.

## Punti per QSO
- Stesso paese: 2. Stesso continente, altro paese: 5. Altro continente: 10. Marittimo mobile: 5 (senza valore di moltiplicatore).

## Moltiplicatori (una volta sola)
- Stati USA: 48 contigui piu' DC (1). Canada, 14: VO1, VO2 (separati per tradizione), NB, NS, PEI (VY2), VE2, VE3, VE4, VE5, VE6, VE7, VE8 (NT), VY1 (YT), VY0 (NU).
- DX: paesi DXCC, compresa la lista WAE (1A0, 3A, 4U1I, 4U1V, 9A, 9H, C3, CT, CU, DL, EA, EI, G, GM, GM/s, HA, I, IT (Sicilia, esclusi IG9/IH9), OH, OK, OM, OZ, SM, SP...): i paesi WAE valgono a parte dal paese DXCC (IT9 non e' I). K e VE non sono fra i paesi.

## Formula
- Punti dei QSO x somma dei moltiplicatori (stati + province + paesi).

## Categorie
- (A) Single Operator, 1500 W, senza aiuto, un solo segnale; (B) Low Power 100 W; (C) QRP 5 W (puo' usare l'aiuto); (D) Single Op Assisted High (aiuto e un ricevitore remoto entro 100 km dal trasmettitore); (E) Single Op Assisted Low; (F) Multi-Operatore High Power (un segnale alla volta, niente ricevitori remoti). Checklog.

## Regole generali
- Un solo sito (proprieta' contigua o raggio di 1500 m); una sola frequenza di CQ (niente alternanza fra due); niente autospot ne' conferme fuori dalla banda e dal modo (chat). Operazione remota ammessa secondo le regole del paese.
- Il regolamento non fissa un limite di QSO per stazione e non definisce i doppi: i QSO unici non sono penalizzati se non eccessivi.

## Cabrillo
- `CONTEST: CQ-160-SSB` (con i trattini, senza l'anno). `CATEGORY-OPERATOR` SINGLE-OP / MULTI-OP / CHECKLOG, `CATEGORY-ASSISTED` ASSISTED / NON-ASSISTED, `CATEGORY-BAND` 160M, `CATEGORY-POWER` HIGH/LOW/QRP, `CATEGORY-MODE` SSB, `CATEGORY-TRANSMITTER` ONE (multi-op). `LOCATION`: stato, provincia o sezione ARRL per USA e Canada, `DX` per gli altri.
- Riga: `QSO: freq mo data ora call rst scambio call rst scambio t`; la colonna `t` non e' usata. Servono frequenze esatte per chi concorre ai premi World, Zone, Continent e USA.

## Penalita' del controllo dei log
- QSO sbagliati tolti con una penalita' di due QSO equivalenti. Squalifica per violazioni delle regole, comportamento antisportivo o troppi contatti non verificati.

## Cosa fa Aurora e cosa no
- Punti per paese e continente, 5 punti ai /MM, moltiplicatori stato/provincia dallo scambio e paese DXCC/WAE (`punteggio.rhai`: IT9, GM/s, 4U1V... contano a parte dal paese genitore), scambio per W/VE e DX, Cabrillo con `CONTEST: CQ-160-SSB`.
- Non fa: penalita' sui QSO sbagliati, conteggio delle 30 ore e delle pause, i limiti della categoria Assisted.

## Differenze rispetto al vecchio `.contest` («CQ WW 160 Meter contest», CW e SSB insieme; l'alias sta sulla cartella CW)
1. Date del 2017 e nessun limite di modo: ora le edizioni 2026 e 2027 e solo SSB.
2. Il vecchio file non dava 5 punti ai /MM (il /MM senza paese valeva 10): ora 5 e senza moltiplicatore.
3. Elenco degli stati e delle province nella cartella (`dati/stati.dom`), scambio per paese del corrispondente.

## Punti da verificare
- Date 2027 calcolate, non ancora sul sito.
- La pagina non dice le sigle di stati e province, ne' il trattamento di Alaska e Hawaii (qui paesi, non stati: i soli stati elencati sono i 48 contigui e DC).
- L'elenco WAE del regolamento non e' stato confrontato voce per voce con il database dei paesi di Aurora: vale quello che il database porta come entita' WAE.
