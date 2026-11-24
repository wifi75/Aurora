# HA-DX Contest - regolamento riassunto

Contest della Magyar Radioamator Szovetseg (MRASZ, societa' radioamatori ungherese): CW e SSB nello **stesso fine settimana**, una sola gara (una cartella).

- **Fonte:** pagina del regolamento <https://ha-dx.com/en/contest-rules> (edizione 2026) e pagina di invio del log <https://ha-dx.com/en/submit-log> (esempio di intestazione Cabrillo).
- **Data di lettura:** 2026-11-24. Le regole cambiano: ricontrollare a ogni edizione.
- Cabrillo: il regolamento non scrive il valore di `CONTEST:`; l'esempio della pagina di invio usa **HA-DX**.

## Date
- Terzo fine settimana pieno di gennaio, sabato 12:00 UTC - domenica 12:00 UTC (24 ore). **2026: 17-18 gennaio.** Il 2027 e' calcolato dalla regola (16-17 gennaio, da confermare): il regolamento non scrive le date degli anni dopo.
- Log in Cabrillo su <https://ha-dx.com/en/submit-log> (c'e' anche l'inserimento a mano) **entro 5 giorni** dalla fine; proroga solo se chiesta prima della scadenza e confermata per iscritto. Registrazione audio da mandare entro 5 giorni dalla richiesta (obbligatoria per chi concorre ai titoli mondiali e continentali).

## Bande e modi
- 160, 80, 40, 20, 15 e 10 m, nei limiti delle licenze e dei piani di banda IARU; le bande WARC non sono nominate.
- CW (A1A) e SSB (A3J). Nelle categorie MIX si usano tutti e due.
- Tutti lavorano tutti. Si puo' usare il DX cluster, anche l'auto-spot; ogni altro aiuto umano o automatico (anche intelligenza artificiale) e' vietato.

## Scambio
- Stazioni **HA**: RS(T) + sigla della **contea** (20 sigle, `dati/contee.dom`).
- Stazioni **non HA**: RS(T) + **numero progressivo**.

## Punti per QSO
- Con una stazione ungherese (HA): **10**.
- Con una stazione dello stesso continente: **2**.
- Con una stazione di un altro continente: **5**.
- Le stazioni **/MM e /AM** valgono sempre **2 punti**, qualunque paese, e non sono moltiplicatori.

## Moltiplicatori (per banda, CW e SSB insieme)
- Paesi **DXCC e WAE** (HA escluso) e le **20 contee ungheresi**, contati una volta per banda, a prescindere dal modo.
- Punteggio = somma dei punti x somma dei moltiplicatori di tutte le bande.

## Doppi
- Vale solo il primo QSO con una stazione su ogni banda; nelle categorie MIX e' ammesso un QSO in CW e uno in SSB con la stessa stazione sulla stessa banda. I doppi valgono zero, senza penalita', e non vanno tolti dal log.

## Categorie
- **SOSB**: un operatore, una banda (10, 15, 20, 40, 80, 160 m), nessun limite di potenza o modo. **SO3BAND**: tre bande a scelta (contano le tre migliori). **SOAB**: tutte le bande, CW, SSB o MIX, potenza HP (oltre 100 W), LP (fino a 100 W) e, solo MIX, QRP (fino a 5 W). **MS MIX**: multi-operatore, tutte le bande. **YOUTH12H MIX**: fino a 25 anni (YOTA), al massimo 12 ore con pause di almeno 60 minuti.
- Operatore singolo: cambio di banda e modo libero, un solo segnale alla volta, tutto da solo. Multi-operatore: la stazione RUN cambia banda solo dopo 10 minuti dal primo QSO sulla nuova banda; la stazione MULT lavora moltiplicatori solo su un'altra banda, non chiama CQ e fa 0 punti con i non moltiplicatori; al massimo due segnali (RUN e MULT).
- Tutta la stazione entro 500 m.

## Controllo del log
- Differenza fra gli orari dei due log al massimo 3 minuti. Un QSO sbagliato costa il doppio del suo valore. I QSO con stazioni che non mandano il log e non compaiono in almeno 3 log si tolgono senza penalita'; un moltiplicatore senza log deve comparire in almeno 10 log. I QSO che violano le regole MS sono segnati X-QSO nel Cabrillo, senza penalita'.

## Cosa fa Aurora e cosa no
- Punti per QSO, doppi per banda e modo, moltiplicatori per banda (paesi DXCC/WAE escluso HA e contee), bande ammesse.
- Non fa: penalita' per i QSO sbagliati, controllo dei log degli altri, regole MS (10 minuti, MULT), categorie, controllo dell'orario di 12 ore.

## Differenze rispetto al vecchio `.contest` («HA-DX Contest», CW e SSB)
1. Bande e punti uguali; ora anche le `/MM` e `/AM` (2 punti, niente moltiplicatore) e i moltiplicatori WAE (IT9, GM/s, TA1...) accanto ai paesi DXCC; il vecchio file non aveva le date aggiornate ne' l'elenco delle contee.
2. Le macro mandano `<NRS>` (stazioni DX) oppure `<MYSECT>` (macro «stazione HA»): il numero e' progressivo, la contea propria si scrive nella scheda del contest («La mia sezione / stato / club»), non nelle macro.

## Punti da verificare
- Il regolamento non scrive le date dopo il 2026: il 2027 e' calcolato.
- Il valore di `CONTEST:` del Cabrillo e' quello dell'esempio del sito (`HA-DX`); i valori di `CATEGORY-*` vanno controllati sulla pagina di invio.
- Per i /MM e /AM il motore riconosce il nominativo (`call:MM,AM`) ma il paese resta quello che il database assegna: lo script li esclude dai moltiplicatori.
