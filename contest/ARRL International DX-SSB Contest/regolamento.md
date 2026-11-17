# ARRL International DX-SSB Contest - regolamento riassunto

- **Fonte:** «ARRL International DX Contest Rules», versione 2.0 del documento (sezione categorie del 04 gen 2024, «Rules for Entry» 1.3 del 29 nov 2023): <https://contests.arrl.org/ContestRules/DX-Rules.pdf>. Pagina del contest: <https://www.arrl.org/arrl-dx>. Il PDF e' unico per CW e fonia e non porta l'anno.
- **Elenco delle sigle dei moltiplicatori:** <https://contests.arrl.org/contestmultipliers.php>.
- **Data di lettura:** 2026-11-17. Edizione: 2027 (la prossima: quella del 2026 e' gia' passata).
- Le regole cambiano ogni anno: ricontrollare a ogni edizione.

## Date
- Primo fine settimana pieno di marzo, da sabato 00:00 UTC a domenica 23:59 UTC. Nessun limite di ore operabili (regola 1.1).
- **2026: 7-8 marzo**, **2027: 6-7 marzo** (calendario ARRL, voce «International DX - Phone»: <https://www.arrl.org/contest-calendar> per il 2027, il PDF del calendario 2025-2026 per il 2026).
- Il log si invia entro 7 giorni dalla fine, alle 23:59 UTC, solo online su <https://contest-log-submission.arrl.org>.

## Bande e modi
- Solo 160, 80, 40, 20, 15 e 10 m (niente 30, 17, 12 m). Solo fonia (Phone, solo durante il fine settimana fonia): i QSO in CW non contano (regola 2.4). CW e fonia sono due gare separate, con log separati.

## Chi lavora chi
- **Le stazioni W/VE lavorano solo i DX, e i DX solo le stazioni W/VE** (2.3). W/VE = Stati Uniti (esclusi Alaska e Hawaii) e Canada (escluse Sable e St. Paul).
- **KH6 (Hawaii), KL7 (Alaska), CY9 (St. Paul) e CY0 (Sable) partecipano come stazioni DX.** Anche gli altri territori statunitensi (KP4, KG4...) sono DX.

## Scambio
- **W/VE:** rapporto + stato o provincia (sigle dell'elenco moltiplicatori ARRL; DC manda DC).
- **DX:** rapporto + potenza, numero o abbreviazione (esempi del regolamento: 500, KW).

## Punti per QSO
- 3 punti per ogni collegamento (5.1), anche fra stazioni dello stesso continente.

## Moltiplicatori (uno per banda, 5.2.1)
- **Stazioni W/VE:** entita' DXCC, esclusi Stati Uniti e Canada (5.2.2). Hawaii e Alaska valgono come entita' DXCC (5.2.3.1).
- **Stazioni DX:** stati USA, DC, province e territori canadesi piu' Labrador (LB); VO1 e VO2 si contano separati per tradizione (5.2.3).

## Formula
- Punteggio = punti dei QSO x numero totale dei moltiplicatori (5.3).

## Doppi
- Ogni stazione una sola volta per banda (2.2). I doppi si tolgono senza penalita' (LGCK.1).

## Categorie
- Single Operator (SO), Single Operator Unlimited (SOU, con spotting), anche Single Band (SOSB, SOUSB); Multi-Operator Single Transmitter (MS), Two Transmitter (M2), Multi-Transmitter (MM), solo All Band.
- Potenze: QRP (5 W PEP), Low (100 W), High (1500 W o il limite di licenza); MS solo Low e High, M2 e MM solo High.
- Overlay Limited Antennas (antenne a elemento singolo, al massimo 50 piedi dal suolo). Checklog.
- MS: al massimo 10 cambi di banda per ora d'orologio; M2: 6 per trasmettitore.

## Regole particolari
- Cerchio di 500 m, un solo segnale per banda per volta, vietato alternare CQ su due frequenze (OPRG.3).
- Il nominativo deve indicare l'entita' DXCC di operazione (6.2): un KL7 che opera dalla Florida usa KL7AA/W4.
- Mobili marittimi e aeronautici fuori da USA e Canada: solo credito di contatto (OPRG.5.1).
- Penalita' dei log: nominativo errato o NIL tolto con penalita' pari ai punti del QSO (3); scambio errato e doppi tolti senza penalita'.
- Multi-operatore: elenco degli operatori in `OPERATORS:` (ELOG.4); M2: il log dice quale trasmettitore ha fatto ogni QSO (ELOG.5).

## Differenze trovate rispetto alle regole che c'erano prima (`.contest` distribuito)
1. **Nome.** Il nome dentro il vecchio file (`m_sName`) coincide con la cartella: il `.contest` intatto e' sostituito.
2. **Modi.** Il vecchio file non limitava il modo (nota `SSB`, nessun modo ammesso = tutti): il regolamento (2.4) vuole solo fonia. Ora `modi.ammessi` lo dice.
3. **Bande.** Nel vecchio file la lista `bands=` era scavalcata dallo script `ammesso()`, che non guardava la banda: 30, 17 e 12 m risultavano ammesse. Ora lo script controlla anche le sei bande.
4. **Doppi.** Il vecchio file faceva il doppio per banda e modo; il regolamento dice una volta per banda (2.2): in fonia USB e LSB sono lo stesso modo. Ora `[doppi] per = "banda"`.
5. **Alaska e Hawaii.** Le regole vecchie erano gia' giuste (solo `K` e `VE` sono W/VE; KH6 e KL sono DX e paesi moltiplicatori). Nessun `KL7` nelle liste: nel database dei paesi l'Alaska ha il prefisso `KL`. Nell'elenco degli stati non ci sono AK e HI.
6. **Sigle.** L'elenco di partenza usava `NWT` (Territori del Nord-Ovest): la sigla ufficiale e' `NT`; `NWT` resta come sinonimo.
7. **Date.** Il vecchio file aveva le date del 2017; ora la regola e' un dato e le date 2026 e 2027 vengono dal calendario ARRL.
8. **Cabrillo.** Il PDF non scrive `CONTEST:` ne' i valori `CATEGORY-*`: `ARRL-DX-SSB` e i valori sono quelli della specifica Cabrillo, da ricontrollare sulla pagina ARRL.
