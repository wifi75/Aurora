# ARI International DX Contest - regolamento riassunto

- **Fonte:** «A.R.I. International DX Contest 2026 - Rules for non-Italians participants», <https://www.ari.it/images/stories/ContestHF/ariinternationaldxcontest-dxrules2026.pdf> (A.R.I. HF Contest Committee); invio dei log su <https://contest.ari.it/hf_web/index_hf.php>. Il regolamento in inglese e' quello per i non italiani: per chi opera dall'Italia (anche Sicilia, Sardegna e Italia africana) valgono le regole ufficiali in italiano, che non sono state lette.
- **Data di lettura:** 2026-11-24. Edizione di riferimento: 2026 (2-3 maggio). Le date degli anni dopo sono calcolate.
- Il contest e' a **maggio** (non a gennaio): il vecchio `.contest` riportava date del 2016.
- Le regole cambiano: ricontrollare a ogni edizione.

## Date
- Primo fine settimana pieno di maggio, da sabato 12:00 UTC a domenica 11:59 UTC (24 ore). **2026: 2-3 maggio** (letto). 2027: 1-2 maggio (calcolato, da confermare).
- Log: solo Cabrillo, solo caricamento web, entro 6 giorni dalla fine (2359Z, 9 maggio nel 2026). Gli SWL entro 60 ore (2359Z del 5 maggio nel 2026).

## Bande e modi
- 80, 40, 20, 15, 10 m, senza le bande WARC, secondo i band plan IARU. CW, SSB e RTTY, anche in categoria mista.

## Scambio
- Stazioni italiane: rapporto + sigla di due lettere della provincia (107 province, `dati/province.dom`).
- Altre stazioni: rapporto + numero progressivo da 001, che NON riparte a ogni banda o modo (solo i multi-operatore multi-trasmettitore hanno un numero per banda).
- Tutti lavorano tutti, tranne che le stazioni italiane non si lavorano fra loro.

## Punti per QSO
- Con una stazione italiana (I, IS, IT9): 10 punti.
- Con il proprio paese: 0 punti (ma vale per i moltiplicatori).
- Stesso continente: 1 punto. Altro continente: 3 punti.
- Una stazione si lavora una volta per banda in ognuno dei modi SSB, CW, RTTY, ma solo il primo QSO vale per il moltiplicatore.

## Moltiplicatori (per banda)
- Ogni provincia italiana: 1. Ogni paese DXCC piu' elenco WAE: 1 (I, IS e IT9 non sono moltiplicatori di paese).
- Lo stesso moltiplicatore vale una sola volta per banda.

## Punteggio finale
- Somma dei punti dei QSO di tutte le bande x somma dei moltiplicatori di tutte le bande.

## Categorie
- Singolo operatore CW, SSB, RTTY o MIXED, alta potenza o bassa (100 W al massimo); multi-operatore con un trasmettitore (regola dei 10 minuti) o con piu' trasmettitori (un segnale per banda), solo misto e alta potenza; SWL.
- Solo per le stazioni italiane: Rookie (licenza da meno di 3 anni), Youth (meno di 25 anni), ARI IQ (sezioni ARI col nominativo IQ).
- Un solo segnale alla volta per il singolo operatore; cluster e skimmer ammessi; self-spotting vietato; doppio CQ sulla stessa banda vietato.
- SWL: si segnano il nominativo sentito e quello del corrispondente; un nominativo compare al massimo 3 volte per banda come corrispondente.

## Cabrillo
- `CONTEST: ARI-DX`. Intestazione: CALLSIGN, CLAIMED-SCORE, NAME, ADDRESS, SOAPBOX, CATEGORY-OPERATOR (SINGLE-OP, MULTI-OP), CATEGORY-TRANSMITTER (ONE, UNLIMITED, SWL), CATEGORY-ASSISTED (ASSISTED), CATEGORY-BAND (ALL), CATEGORY-POWER (HIGH, LOW), CATEGORY-MODE (SSB, CW, RTTY, MIXED), CATEGORY-STATION (FIXED), CATEGORY-OVERLAY (ROOKIE, YOUTH, solo italiani); LOCATION (sezione ARI) non serve ai DX.

## Cosa fa Aurora e cosa no
- Punti, doppi per banda e modo, province dallo scambio, paesi DXCC con le entita' WAE, esclusione di I, IS e IT9 dai paesi, QSO fra italiane non ammessi, Cabrillo.
- Non fa: controllo dei log degli altri, regola dei 10 minuti dei multi-operatore, SWL.

## Differenze rispetto al vecchio `.contest`
1. Date vere (maggio, 2026) al posto di quelle del 2016.
2. Le entita' WAE (come TA1 o GM/s) valgono come moltiplicatori; il vecchio file contava solo i paesi DXCC.
3. Le bande sono 80-10 m senza WARC anche nel controllo dei QSO (lo faceva gia' `bands=`), il proprio paese vale 0 punti (prima `same_dxcc=0` solo per i DX) e i doppi si fanno per banda e modo.
