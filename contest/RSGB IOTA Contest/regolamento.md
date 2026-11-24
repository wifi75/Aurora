# RSGB IOTA Contest - regolamento riassunto

Contest dell'RSGB per le isole IOTA (Islands On The Air): CW e SSB nello stesso fine settimana, una sola gara. Le regole generali dei contest HF dell'RSGB non valgono per questo contest.

- **Fonte:** «RSGB IOTA Contest», edizione 2026, <https://www.rsgbcc.org/hf/rules/2026/riota.shtml> (RSGB Contest Committee) e FAQ <https://www.rsgbcc.org/hf/iotafaq.shtml>.
- **Data di lettura:** 2026-11-24. Il regolamento del 2027 non c'e' ancora: le date 2027 vengono da un calendario di terzi. Le regole cambiano: ricontrollare a ogni edizione. Domande: hf.query@rsgbcc.org.
- Nome Cabrillo `RSGB-IOTA` (elenchi dei nomi Cabrillo e calendario di terzi, il regolamento non lo scrive).

## Date
- Fine settimana di fine luglio, da sabato 12:00 UTC a domenica 12:00 UTC (24 ore; i QSO dall'ora finale in poi non valgono). **2026: 25-26 luglio.** 2027: 24-25 luglio (calendario di terzi, da confermare).
- Log Cabrillo entro 5 giorni dalla fine, sulla pagina di caricamento dei log HF dell'RSGB.

## Bande e modi
- 3,5, 7, 14, 21 e 28 MHz. CW e/o SSB. Ogni stazione una volta per banda e per modo (il regolamento non scrive i doppi: i moltiplicatori sono per banda e per modo e le FAQ dicono che i QSO sull'altro modo servono solo al controllo se si entra in un solo modo).

## Scambio
- Stazioni **World** (non su un'isola valida): RS(T) + numero progressivo da 1 (599 254).
- Stazioni **Island** (su un'isola IOTA): RS(T) + numero progressivo + riferimento IOTA (59 378 EU115).
- Una sola sequenza di numeri per CW e SSB insieme. Se non si riceve niente si scrive 0.
- La Gran Bretagna continentale conta come isola (EU-005); GI ed EI contano come EU-115 secondo un regolamento delle edizioni precedenti.

## Punti per QSO
- Da una stazione Island: con una World 5, con un'isola con lo stesso riferimento 5, con un'altra isola 15.
- Da una stazione World: con una World 2, con un'isola 15.

## Moltiplicatori
- I riferimenti IOTA diversi lavorati su ogni banda, in CW, piu' gli stessi in SSB (per banda e per modo).
- Punteggio = totale dei punti di tutte le bande x totale dei moltiplicatori di tutte le bande.

## Categorie
- Posizione: Island Fixed, Island DXpedition (isola raggiungibile solo per mare o aria, attrezzatura portata sul posto), World (nessun multi-operatore World: sarebbe un check log).
- Operatore: Single Operator Unassisted o Assisted (autospot permesso agli Assisted dal 2026, al massimo ogni 10 minuti); Island Multi-1 (stazione RUN piu' stazione moltiplicatori che non chiama CQ) e Island Multi-2 (due segnali su bande diverse), solo da un'isola, solo misto, solo Assisted, 24 ore. Overlay Newcomer (licenziato da meno di 3 anni).
- Modo: CW, SSB o Mixed (i multi-operatore solo Mixed). Tempo: 24 ore o 12 ore (nel 12 ore pause di almeno 60 minuti). Potenza: High (1500 W), Low (100 W), QRP (5 W); senza indicazione High.
- Tutta la stazione entro 500 m; remoto solo entro 100 km nella stessa entita' DXCC.
- Log di Russia e Bielorussia considerati check log fino a nuovo avviso (decisione RSGB del 2022).

## Penalita' e controllo
- Un errore (nominativo, numero, IOTA) costa 5 punti QSO. CAT molto consigliato: errori di frequenza e modo fanno perdere punti QSO.
- Un QSO perso con un moltiplicatore fa perdere anche il moltiplicatore, a meno che quel riferimento sia stato lavorato di nuovo (FAQ 20).
- Numeri sbagliati o doppi mandati: si registra quello mandato davvero (FAQ 19).

## Cosa fa Aurora e cosa no
- Punti per World e Island dal riferimento IOTA dello scambio mandato e ricevuto, moltiplicatori per banda e per modo, doppi per banda e per modo, Cabrillo con `CONTEST: RSGB-IOTA`.
- Non fa: controllo dei riferimenti sull'elenco ufficiale IOTA, penalita', categorie (ore, 12/24, multi), `CATEGORY-*` e potenza (a mano nell'intestazione).

## Differenze rispetto al vecchio `.contest` («IOTA CONTEST»)
1. Stesse regole di punteggio (5/15 da isola, 2/15 da World, moltiplicatori IOTA per banda e modo); ora con date, categorie e scambio dichiarati.
2. Le macro mandano `<NRS> <MYSECT>`: il numero e' progressivo e il proprio riferimento IOTA si scrive nella scheda del contest («La mia sezione / stato / club»), non nelle macro.

## Punti da verificare
- Il regolamento non scrive la regola dei doppi: si tiene una stazione per banda e per modo.
- Il valore `EU115` di EI e GI viene dal regolamento di anni precedenti: nell'edizione 2026 non e' ripetuto; vale il riferimento mandato dal corrispondente.
- Date 2027 e 2028 non confermate.
