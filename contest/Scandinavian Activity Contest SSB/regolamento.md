# Scandinavian Activity Contest SSB - regolamento riassunto

Contest dell'NRAU (Nordic Radio Amateur Union) per promuovere i contatti fra stazioni scandinave e non scandinave. La parte SSB e la parte CW sono due gare in due fine settimana diversi (cartelle «Scandinavian Activity Contest CW» e «Scandinavian Activity Contest SSB»).

- **Fonte:** «Scandinavian Activity Contest 2026 Rules», <https://www.sactest.net/blog/scandinavian-activity-contest-2026-rules/> (NRAU, comitato SAC); date 2027 dalla pagina <https://www.sactest.net/>.
- **Data di lettura:** 2026-11-24. Le regole cambiano: ricontrollare a ogni edizione. Domande: support@sactest.net.
- Nome Cabrillo `SAC-SSB`: il regolamento non lo scrive; viene dagli elenchi dei nomi Cabrillo (e dal calendario di terzi dxzone.com), da verificare sul modello Cabrillo 3.0 del sito.

## Date
- SSB: ottobre, da sabato 12:00 UTC a domenica 11:59 UTC (24 ore). **2026: 10-11 ottobre.** 2027: 9-10 ottobre (annunciate sul sito). 2028 calcolata dalla regola.
- Log solo elettronici (Cabrillo) sul robot <https://www.sactest.net/robot/>, entro 5 giorni dalla fine alle 23:59 UTC; i log in ritardo sono check log. Per un posto fra i primi tre serve la frequenza esatta di ogni QSO.
- Russia e Bielorussia: stazioni non ammesse nel 2026 (decisione NRAU).

## Bande, modo e frequenze
- 80, 40, 20, 15 e 10 m (le cinque bande storiche, niente WARC). Solo SSB. Segmenti (band plan IARU R1): 3600-3650, 3700-3800, 7060-7100, 7130-7200, 14125-14300, 21151-21450, 28320-29000 kHz.
- Tutti i trasmettitori e ricevitori entro un cerchio di 500 m di diametro.

## Chi lavora chi
- Contano solo i QSO fra una stazione **scandinava** e una **non scandinava**. Scandinavia: Norvegia (LA, LB, LC, LG, LI, LJ, LN), Finlandia (OF, OG, OH, OI; Aland OF0, OG0, OH0; Market Reef OJ0), Danimarca (5P, 5Q, OU, OV, OZ), Svezia (7S, 8S, SA-SM), Islanda (TF), Faroe (OW, OY), Groenlandia (OX, XP: scandinava anche se in Nord America), Svalbard (JW), Jan Mayen (JX).

## Scambio
- RS(T) + numero progressivo da 001 (poi 1000, 1001...). I multi-operatore possono avere una sequenza per banda. Una stazione una volta per banda; QSO fra bande o modi diversi non ammessi.

## Punti per QSO
- Stazioni scandinave: con un europeo non scandinavo 2, con un non europeo 3.
- Stazioni non scandinave: europee 1 per ogni QSO scandinavo; non europee 1 su 14, 21 e 28 MHz e 3 su 3,5 e 7 MHz.
- Un QSO conta solo con nominativo e scambio corretti.

## Moltiplicatori (per banda)
- Stazioni scandinave: ogni paese DXCC lavorato, una volta per banda.
- Stazioni non scandinave: ogni prefisso con cifra (0-9) dentro ogni paese scandinavo, una volta per banda: SI3, SK3, SL3, SM3, 7S3 e 8S3 sono un solo moltiplicatore. Senza cifra vale 0 (LA/G3XYZ -> LA0); con due o piu' cifre la prima (OZ150A -> OZ1).
- Punteggio = punti di tutte le bande x moltiplicatori di tutte le bande.

## Doppi
- Si mandano tutti i QSO, doppi compresi: il controllo li trova e li porta a zero punti.

## Categorie
- Single Operator All Band High (1500 W), Low (100 W) e QRP (5 W); Single Band (solo scandinavi, una banda); Low Band (solo non scandinavi, 80 e 40 m). Multi-operatore: MULTI-ONE (tutti; un segnale per banda, 10 minuti minimi su una banda salvo nuovo moltiplicatore) e MULTI-MULTI (solo scandinavi).
- Overlay del Single Operator All Band: ROOKIE (licenza da meno di 3 anni), CLASSIC (niente aiuto di segnalazione), TB-WIRES, WIRE-ONLY. Altre: EXPLORER, CHECKLOG.
- Il DX cluster, lo Skimmer e il RBN sono ammessi in tutte le categorie tranne CLASSIC; l'autospot no.

## Cosa fa Aurora e cosa no
- Punti per chi calcola (scandinavo o no, europeo o no), ammissione dei soli QSO scandinavo/non scandinavo, moltiplicatori per banda (paesi DXCC per gli scandinavi, prefisso con cifra per gli altri), doppi per banda, Cabrillo con `CONTEST: SAC-SSB`.
- Non fa: controllo dei segmenti di frequenza, categorie e limiti (500 m, 10 minuti), `CATEGORY-*` (a mano nell'intestazione), controllo incrociato dei log. I QSO con Russia e Bielorussia valgono come gli altri (il regolamento vieta solo i loro log).

## Differenze rispetto al vecchio `.contest` («Scandinavian Activity Contest», CW e SSB insieme)
1. Stesse regole di punteggio; ora con le date del 2026/2027, la sola modalita' SSB e lo scambio dichiarato. Il vecchio file valeva per tutti i modi: la cartella CW (Scandinavian Activity Contest CW) ha l'altra gara.
2. Le macro restano quelle del vecchio contest (solo numero progressivo): nessun valore fisso.

## Punti da verificare
- Gli scandinavi con un europeo: il regolamento dice «European stations outside Scandinavia»; i paesi europei di cty.dat (anche Russia europea) valgono 2, Russia asiatica e simili 3.
- Date 2028 calcolate.
