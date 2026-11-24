# All Asian DX Contest CW - regolamento riassunto

All Asian DX Contest della JARL, parte CW.

- **Fonte:** «The 66th ALL ASIAN DX CONTEST -- 2025», <https://www.jarl.org/English/4_Library/A-4-3_Contests/AA_rule_en.htm> (JARL; l'edizione 2025 e' l'ultima con il testo leggibile, l'edizione 2026 e' gia' svolta senza una nuova pagina). Formato Cabrillo: <https://jarl.org/English/4_Library/A-4-3_Contests/AADX_Cabrillo.htm>.
- **Data di lettura:** 2026-11-24. Le date 2026 e 2027 sono calcolate dalla regola, non lette su una pagina JARL dell'edizione.
- Nome Cabrillo `AADX-CW`: dalla pagina del formato Cabrillo della JARL (scritta per edizioni passate, da verificare con il caricamento web).
- Le regole sono cambiate nel 2025 («rinnovato»): ricontrollare a ogni edizione.

## Date
- Terzo sabato di giugno, da 00:00 UTC al 24:00 UTC del giorno dopo (48 ore). Calcolate: 2026 2026-06-20 - 2026-06-21; 2027 2027-06-19 - 2027-06-20.
- Log entro 10 giorni dalla fine: caricamento web <https://contest.jarl.org/upload-aa> (consigliato), posta elettronica (CW aacw@jarl.org) o carta (solo scritta a mano, al massimo 100 QSO).
- CW e fonia sono due gare con due log; non si fanno QSO fra CW e fonia ne' fra bande diverse.

## Bande e modo
- 160, 80, 40, 20, 15, 10 m, senza WARC. Solo CW.

## Chi lavora chi
- Gli asiatici lavorano tutti tranne il proprio paese DXCC.
- Tutti gli altri lavorano solo gli asiatici.
- Asia: l'elenco JARL di 55 entita' (Spratly, Vietnam, Giappone, Corea, Cina, India, Russia asiatica, Kazakistan, ...). Maldive e Turchia contano solo per la parte asiatica; JD1 Ogasawara e' Asia, JD1 Minami Torishima e' Oceania.
- Stazioni /MM in alto mare: solo i QSO con l'Asia, punti come fra asiatici, niente moltiplicatore, classifica a parte.

## Scambio
- Singolo operatore: RS(T) + eta' dell'operatore (`5925` in fonia, `59925` in CW); chi non la manda passa `01`.
- Multi-operatore: RS(T) + eta' media degli operatori all'inizio del contest.

## Punti per QSO (una volta per banda)
| Banda | Asiatico: QSO con asiatico | Asiatico: QSO con non asiatico | Non asiatico: QSO con asiatico |
|---|---|---|---|
| 160 m | 3 | 9 | 3 |
| 80 e 10 m | 2 | 6 | 2 |
| 40, 20, 15 m | 1 | 3 | 1 |
- Fra stazioni dello stesso paese DXCC: ne' punti ne' moltiplicatore.

## Moltiplicatori (per banda)
- Stazioni asiatiche: le entita' DXCC lavorate su ogni banda.
- Stazioni non asiatiche: i prefissi asiatici (regole WPX) lavorati su ogni banda.

## Formula
- Tutte le bande: (punti di tutte le bande) x (moltiplicatori di tutte le bande). Monobanda: punti x moltiplicatori della banda.

## Doppi
- Una stazione una volta per banda; il doppio non da' punti.

## Categorie
- Single Operator tutte le bande High/Low (SOABHP, SOABLP, anche 24 ore: SOABHP24, SOABLP24), monobanda 160-10 m High/Low, Multi-Op Single TX (MSHP, MSLP), Multi-Op Multi TX (MMHP, MMLP), solo per i JA SOJR (19 anni o meno) e SOSV (70 anni o piu'), Checklog. Low = 100 W.
- Niente cambio di postazione durante il contest; reti di segnalazione ammesse (nessun autospot); controllo da remoto solo se antenne, trasmettitori e ricevitori sono in una sola postazione.

## Cabrillo
- `CONTEST: AADX-CW`; `CATEGORY: operatore banda potenza` (SINGLE-OP, MULTI-ONE, MULTI-MULTI, CHECKLOG; ALL o 160M...10M; HIGH o LOW). Riga: `QSO: freq mo data ora call rst scambio call rst scambio t` con `mo` CW o PH.
- Multi-operatore: `OPERATORS:` con tutti i nominativi; eta' in `SOAPBOX:` per JA junior e senior; `LOCATION:` la prefettura se diversa dall'indirizzo della licenza.

## Cosa fa Aurora e cosa no
- Punti per banda e continente, ammissione (asiatici/non asiatici, banda, paese), moltiplicatori DXCC o WPX per banda, doppi per banda, Cabrillo con `CONTEST: AADX-CW`.
- Non fa: /MM in alto mare, stazioni ausiliarie militari, categorie JA, 24 ore, penalita'.

## Differenze rispetto al vecchio `.contest` («All Asian DX Contest», CW e SSB)
1. Il vecchio file valeva per CW e fonia insieme (e per i modi digitali) con date del 2015: ora due cartelle, con il modo e le date giuste.
2. Lo script controlla anche la banda, perche' sostituisce `bands=`.

## Punti da verificare
- Date 2026 e 2027: regola «terzo sabato di giugno» / «primo sabato di settembre» del 2025, nessuna pagina JARL con il testo dell'edizione.
- Nome Cabrillo e valori `CATEGORY` (pagina di formato scritta per edizioni passate).
