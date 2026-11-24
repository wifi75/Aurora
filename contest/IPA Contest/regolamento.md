# IPA Contest - regolamento riassunto

Contest dell'International Police Association Radio Club (IPARC), aperto a tutti i radioamatori; i soci IPARC si riconoscono dallo scambio. CW e SSB sono nello **stesso fine settimana** ma in due giorni (una cartella).

- **Fonte:** sito dell'IPARC, pagina del contest <https://dl0ipa.darc.de/Contest.htm> con il PDF «IPARC-Contest Rules» (inglese, `IPARC-Contest _engl_ allgemein_25.pdf`, edizione 2025-2026; la versione tedesca del 2023 e' identica nei punti). La pagina dice «IPARC-Contest 2026: 07./08. November».
- **Data di lettura:** 2026-11-24. Le regole cambiano: ricontrollare a ogni edizione.
- Cabrillo: il regolamento non lo nomina («.txt- e .cbr-files are welcome, every log will be accepted»): **da verificare**; con `cabrillo_contest` vuoto Aurora scrive il nome della cartella.

## Date
- **Primo fine settimana pieno di novembre**: CW il **sabato 06:00-18:00 UTC**, SSB la **domenica 06:00-18:00 UTC**. 2026: 7-8 novembre. 2027: 6-7 novembre (calcolato, da confermare).
- Log per e-mail al responsabile del contest **entro il 30 novembre**; nome e punteggio vengono pubblicati.

## Bande e segmenti (kHz)
- 80, 40, 20, 15 e 10 m (piano di banda IARU Regione 1). **Fuori dai segmenti nessun QSO vale.**

| Banda | CW | SSB |
|---|---|---|
| 80 m | 3510-3560 | 3700-3800 |
| 40 m | 7000-7025 | 7060-7100, 7130-7200 |
| 20 m | 14000-14060 | 14125-14300 |
| 15 m | 21000-21070 | 21155-21300 |
| 10 m | 28000-28070 | 28320-28600 |

## Scambio
- RS(T) + **numero progressivo**, ogni modo da 001. I **soci IPARC** aggiungono «IPA»; quelli degli **USA** anche la sigla del loro stato. Chiamata: CW «CQ IPA Test», SSB «CQ IPA Contest».
- Con ogni stazione si scambia un solo controllo per banda e modo (si puo' fare un rapporto in piu' con l'operatore di una stazione di club per i diplomi, ma non entra nel contest ne' nel log).

## Punti
- Ogni modo si valuta a parte. **1 punto per QSO, 5 per ogni stazione IPARC.**
- **Moltiplicatori:** ogni paese DXCC con una stazione IPARC e ogni stato USA con una stazione IPARC, **1 per banda**.
- **Punteggio** di ogni banda = somma dei punti dei QSO x somma dei moltiplicatori della banda; risultato finale = somma delle bande.

## Doppi
- Una sola volta per stazione, banda e modo (il regolamento dice che con ogni stazione si scambia un controllo per banda e per modo).

## Categorie
- **A** club e stazioni speciali, **B** singoli con nominativo personale, **C** SWL; ognuna in CW e in SSB (A-CW, A-SSB, B-CW...). Un solo nominativo per modo; nessuna differenza fra soci e non soci. La classe deve risultare dal log. Ogni stazione che manda il log riceve un diploma con il piazzamento.

## Cosa fa Aurora e cosa no
- Punti per QSO (1 o 5), doppi per banda e modo, moltiplicatori per banda (paesi e stati con una stazione IPARC), punteggio banda per banda.
- Non fa: controllo dei segmenti di frequenza, distinzione di CW e SSB per giorno (la finestra va da sabato 06:00 a domenica 18:00), un numero che riparte da 001 a ogni modo, classi A/B/C.
- CW e SSB sono due classifiche: conviene tenere un log per modo (nello stesso log i moltiplicatori valgono per banda, non per modo).

## Differenze rispetto al vecchio `.contest` («IPA Contest»)
1. Stesse regole di punteggio (il vecchio script e' ripreso quasi uguale); ora con le date dell'edizione, lo scambio dichiarato, i segmenti annotati e un controllo in piu' sui QSO senza paese.
2. Le macro mandano `<NRS>` (non soci) oppure `<NRS> IPA <MYSECT>` (macro «socio IPARC»): il numero e' progressivo, lo stato USA dei soci si scrive nella scheda del contest («La mia sezione / stato / club»), non nelle macro.

## Punti da verificare
- Il nome `CONTEST:` del Cabrillo e il formato del log: il regolamento non li scrive.
- Per gli USA il regolamento dice «DXCC / US-State» e Aurora conta lo stato al posto del paese K: una stazione IPARC USA senza stato nello scambio vale 5 punti ma nessun moltiplicatore.
- Le date del 2027 sono calcolate dalla regola.
