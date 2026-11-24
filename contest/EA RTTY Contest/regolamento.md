# EA RTTY Contest - regolamento riassunto

- **Fonte:** bases EARTTY 2026 di URE (Union de Radioaficionados Espanoles): <https://concursos.ure.es/en/eartty>. Valgono anche il Reglamento General de Concursos HF di URE (<https://concursos.ure.es/en/reglamento-general-concursos-hf/>).
- **Data di lettura:** 2026-11-24. Edizione 2026: 4-5 aprile (gia' passata); prossima 2027 (calcolata, da confermare).
- Le regole cambiano ogni anno: ricontrollare a ogni edizione.

## Date
- Dalle 12:00 UTC di sabato alle 11:59 UTC di domenica, primo fine settimana pieno di aprile. 2026: 4-5 aprile; 2027: 3-4 aprile (calcolata).
- Log entro il 20 aprile 2026 (regola generale HF: massimo 15 giorni); i log in ritardo diventano Checklog.

## Bande e modo
- 80, 40, 20, 15 e 10 m, segmenti IARU consigliati. Solo RTTY Baudot. Chiamata: «CQ EA TEST». Vietati i QSO in banda incrociata.

## Chi lavora chi
- Tutti lavorano tutti, una volta per banda. I QSO unici non sono validi: per valere un QSO deve comparire in almeno due log.

## Scambio
- Stazioni EA (tutte quelle che trasmettono dalla Spagna, qualunque prefisso): RST + sigla della provincia. Resto del mondo: RST + numero progressivo da 001. EA4URE (stazione speciale di URE): RST + `HQ`.

## Punti per QSO
- EA con EA (anche EA6, EA8, EA9): 2. EA con DX: 1. DX con DX: 1. DX con EA (anche EA6, EA8, EA9): 3.

## Moltiplicatori (per banda, per EA e DX)
- Entita' dell'elenco EADX-100 (trattate come entita' DXCC), le 52 province spagnole, `EA4URE` (HQ) per tutte le stazioni, aree numeriche (distretti) di USA, Canada, Giappone e Australia (VE3, W5, JA1).
- Ogni moltiplicatore una volta per banda. Il primo QSO su una banda con W, VK, VE o JA vale due moltiplicatori (paese e area); con EA, EA6, EA8 o EA9 due (paese e provincia).
- Punteggio finale: somma dei punti di tutte le bande x somma dei moltiplicatori di tutte le bande.

## Categorie
- Per stazioni EA e per DX, le stesse sei: single op multibanda High, Low (fino a 100 W), QRP (fino a 5 W), Youth (massimo 25 anni); single op monobanda (10, 15, 20, 40, 80 m); multi-multi multibanda. Alta potenza = limite legale del paese.
- Cluster ammesso in tutte le categorie; vietato autoannunciarsi o farlo fare ad altri. Single op: un solo segnale; multi: un segnale per banda, tutti gli operatori nel log. Un solo nominativo per tutta la gara. Monobanda: si possono fare QSO su altre bande ma passano a Checklog.

## Log
- Solo Cabrillo (`CONTEST: EARTTY`), caricato dal sito dei log di URE; conferma o errori per e-mail. Problemi: hf@ure.es (non per inviare log); il manager e' indicato nel regolamento ufficiale di URE.

## Premi
- Targhe ai campioni di ogni categoria; diplomi ai secondi e terzi, al primo di USA, VE, JA, VK; per il premio cartaceo servono almeno 150 QSO validi in multibanda o 50 in monobanda e un minimo di partecipanti. I vincitori SOAB e MM possono operare come EA4URE (HQ) l'anno dopo.

## Cosa fa Aurora e cosa no
- Punti 2/1/3/1, doppi per banda, moltiplicatori per banda (paesi, province o HQ dallo scambio delle stazioni EA, aree di W, VE, JA, VK), Cabrillo `CONTEST: EARTTY`. Chi opera dalla Spagna scrive la propria provincia in «Numero da mandare» della scheda del contest (incremento automatico spento); dall'estero il numero avanza da solo.
- Non fa: il controllo incrociato dei log (QSO in almeno due log), la banda incrociata.

## Dubbi
- Il regolamento non elenca le sigle delle province: `dati/province.dom` usa le sigle automobilistiche (52, con LO per La Rioja, PM Baleari, GC e TF Canarie, CE e ML) e va ricontrollato.
- Il regolamento non dice i valori di `CATEGORY-*` del Cabrillo: quelli del file sono gli usuali.

## Differenze rispetto al vecchio `.contest`
1. Il vecchio file aveva date del 2014 e nessun elenco delle province; ora date 2026 e 2027, categorie e scambio per paese.
2. Il punteggio e' lo stesso (2/1/3/1; paesi, province, aree per banda).
