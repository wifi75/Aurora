# Holyland DX Contest - regolamento riassunto

Contest dell'IARC (Israel Amateur Radio Club) per far lavorare le stazioni di tutto il mondo con quelle israeliane. Dal 2023 e' un contest «WW»: tutti lavorano tutti. CW e SSB sono nello **stesso fine settimana** (una sola gara, una cartella).

- **Fonte:** pagina del contest <https://tools.iarc.org/wwhc/> («WWHC 2027», con le regole, l'elenco delle Aree e le istruzioni per il Cabrillo; il testo e' dentro la pagina, che e' un'applicazione web) e <https://www.iarc.org/holyland/>.
- **Data di lettura:** 2026-11-24. Le regole cambiano: ricontrollare a ogni edizione.
- Cabrillo: `CONTEST: HOLYLAND` (scritto nella pagina dei log).

## Date
- Terzo fine settimana pieno di aprile, **venerdi' 21:00 UTC - sabato 20:59 UTC** (24 ore). **2027: 16-17 aprile** (scritto sul sito). 2026: 17-18 aprile (stessa regola, bollettino di terzi).
- Log in Cabrillo 3.0 sul sito (<https://tools.iarc.org/wwhc/#/submitlog>) **entro 7 giorni** dalla fine; file `nominativo.log` o `nominativo.cbr`, orario UTC, tutti i numeri mandati e ricevuti nel log.

## Bande e modi
- 80, 40, 20, 15 e 10 m; CW, SSB o MIX. Nelle categorie MIX si lavora con la stessa stazione in CW e in SSB; **vietati i QSO in CW dentro i segmenti SSB e viceversa**.
- Web cluster e packet ammessi; l'auto-spot e' vietato; tutta la stazione entro un cerchio di 500 m; un solo segnale alla volta (nel MOST due segnali solo se il secondo lavora un nuovo moltiplicatore).

## Scambio
- Stazioni **non 4X/4Z**: RS(T) + numero progressivo da 001.
- Stazioni **4X/4Z**: RS(T) + **Area** di cinque caratteri (F15RH, E15TA...): quadrato di 10 x 10 km del sistema dell'Ufficio Topografico di Israele (lettera = ovest-est, numero = nord-sud) + sigla del distretto. Lo stesso quadrato puo' stare in piu' distretti (H08HF, H08HD, H08YZ). I 23 distretti: AK, AS, AZ, BL, BS, HB, HD, HF, HG, HS, JN, JS, KT, PT, RA, RH, RM, SM, TA, TK, YN, YZ, ZF.
- Mobili israeliani: fino a 10 Aree diverse, almeno mezz'ora per Area; ogni Area e' una stazione diversa con il suffisso `/1`, `/2`... `/0`.

## Punti per QSO
- **Fuori da Israele:** con una stazione 4X/4Z **8**; con una stazione di un altro continente **4**; dello stesso continente **2**; dello stesso paese DXCC **1**.
- **Stazioni 4X/4Z:** con il DX **8**; con una stazione dello stesso continente (Asia) **2**; fra 4X/4Z **1**.
- Le stazioni **/MM** valgono **4 punti** a ogni partecipante e non sono moltiplicatori.

## Moltiplicatori
- **Fuori da Israele:** ogni **Area** lavorata e ogni **paese DXCC** lavorato, **per banda**, a prescindere dal modo. I distretti da soli non sono moltiplicatori.
- **Stazioni 4X/4Z:** ogni paese DXCC (compreso Israele) per banda; ogni Area diversa a prescindere dal modo (il testo non dice «per banda»: Aurora le conta una volta sola).
- **Punteggio = somma dei punti dei QSO x somma di tutti i moltiplicatori (paesi + Aree) di tutte le bande.**

## Doppi
- Doppio: stessa stazione, stesso modo e stessa banda; zero punti, senza penalita', da tenere nel log. Se il primo QSO non e' valido, il secondo si accetta.

## Categorie
- SOAB-MIX (HP, LP sotto 100 W, QRP sotto 10 W), SOAB-CW e SOAB-SSB (HP, LP), SOSB per banda (80, 40, 20, 15, 10), MOST (multi-operatore, un trasmettitore, MIX), YN (sotto i 20 anni o al massimo 3 anni dalla prima licenza), SWL, portatili solo per 4X/4Z (`PORTABLE`, `M5`, `M10`).
- Premi (certificati digitali) solo con almeno 100 QSO validi.

## Controllo del log
- QSO a zero punti per: nominativo sbagliato, RST o numero sbagliati, NIL, differenza di orario oltre 5 minuti, errori di banda o modo. Un QSO con una stazione senza log conta se compare in almeno 10 log.

## Cosa fa Aurora e cosa no
- Punti per QSO (anche /MM), doppi per banda e modo, moltiplicatori per banda (paesi DXCC e Aree), regola delle 4X/4Z.
- Non fa: controllo dei segmenti CW/SSB, penalita' e QSO annullati dal controllo del log, regole del MOST (10 minuti), categorie, elenco chiuso dei paesi.

## Differenze rispetto al vecchio `.contest` («Holyland-DX Contest»)
1. Punteggio equivalente (8, 4, 2, 1) con in piu' le regole per le stazioni 4X/4Z, le `/MM` e le Aree controllate sull'elenco ufficiale (`dati/aree.dom`, 501 Aree); date dal sito, scambio e Cabrillo dichiarati.
2. Le macro mandano `<NRS>` (stazioni fuori da Israele) oppure `<MYSECT>` (macro «stazione 4X»): il numero e' progressivo, l'Area propria si scrive nella scheda del contest («La mia sezione / stato / club»), non nelle macro.

## Punti da verificare
- L'elenco delle Aree e' quello della tabella del sito, letto il 2026-11-24 (501 righe nel formato `F-15-RH`); un'Area scritta nello scambio ma non in elenco non e' un moltiplicatore.
- Il regolamento non dice se le Aree contano «per banda» per le stazioni 4X/4Z: Aurora le conta una volta sola.
- Per le `/MM` il motore riconosce il nominativo (4 punti) ma non puo' togliere il moltiplicatore del paese.
- La data del 2026 e' calcolata dalla regola (il sito mostra solo l'edizione 2027).
