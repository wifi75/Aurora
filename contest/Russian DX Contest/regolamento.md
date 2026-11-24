# Russian DX Contest - regolamento riassunto

Russian DX Contest dell'SRR (Unione dei radioamatori russi): CW e SSB nello stesso fine settimana di marzo, una sola gara.

- **Fonte:** regolamento in inglese <https://www.rdxc.org/rules_eng> (34a edizione, 2027); regolamento SRR 2026 <https://srr.ru/wp-content/uploads/2026/02/Reglament-RDXC-2026.pdf> (in russo, approvato il 3 febbraio 2026); elenco delle regioni <https://www.rdxc.org/info>.
- **Data di lettura:** 2026-11-24. Edizione 2026 (21-22 marzo, gia' svolta); la prossima, 2027 (20-21 marzo), e' scritta sul sito.
- Nome Cabrillo `RDXC` (con uno spazio dopo i due punti): da <https://www.rdxc.org/info>.
- Le regole cambiano: ricontrollare a ogni edizione. Differenze fra 2026 e 2027 trovate nei punti letti: nel 2026 il regolamento SRR vieta ai gruppi Classic i decodificatori automatici e le reti di segnalazione e prevede il tag `X-RUSOBL` per i russi (il testo 2027 parla di RDA in `LOCATION`); punti, scambio e moltiplicatori sono uguali.

## Date
- Terzo fine settimana pieno di marzo, da sabato 12:00 UTC a domenica 11:59 UTC (24 ore). **2026: 21-22 marzo; 2027: 20-21 marzo.**
- Log entro 14 giorni dalla fine (2027: 4 aprile 23:59 UTC); per i candidati ai primi tre posti (non SWL, non overlay) entro due giorni (22 marzo 23:59 UTC). Solo Cabrillo, solo caricamento web <http://ua9qcq.ru/submit_log.php>, file `nominativo.log` o `nominativo.cbr`.

## Bande e modi
- 160, 80, 40, 20, 15 e 10 m, senza WARC. CW e SSB.
- In Mixed la stessa stazione si lavora una volta in CW e una in SSB su ogni banda.

## Scambio
- Stazioni russe: RS(T) + sigla di due lettere della regione (MA, SP, NS...); l'elenco ufficiale (89 regioni piu' Franz Josef Land FJ e Antartide AN) e' in `dati/oblast.dom`.
- Tutte le altre: RS(T) + numero progressivo da 001, uno solo per tutte le bande (nei multi-operatore si raccomanda una serie per banda).
- Sigle diverse da quelle ufficiali mandate davvero dal corrispondente sono accettate come giuste (FAQ del sito).

## Punti per QSO
- Stazioni russe: Russia del proprio continente 2; Russia dell'altro continente 5; altro paese del proprio continente 3; altro continente 5.
- Tutte le altre: stazione russa 10 (qualunque continente); proprio paese 2; altro paese del proprio continente 3; altro continente 5.
- Kaliningrad (UA2F), Franz Josef Land (RI1FJ) e le stazioni russe in Antartide (RI1AN) sono un paese DXCC e una regione a parte (moltiplicatore doppio); i punti si danno come per la Russia europea.
- Mobili marittimi (/MM): 5 punti a tutti, mai moltiplicatori.
- Paesi e continenti secondo la lista DXCC.

## Moltiplicatori (per banda)
- Ogni paese (lista DXCC piu' lista WAE) piu' ogni regione russa, una volta per banda; sommati.

## Formula
- Punti dei QSO x (paesi + regioni).

## Doppi
- Stesso nominativo, stessa banda e stesso modo: 0 punti se il primo QSO e' valido. I doppi non si penalizzano e non si tolgono dal log.

## Categorie
- Single-Op tutte le bande: High, Low, QRP; Mixed, CW o SSB (QRP: Mixed, CW, SSB); versioni Senior (75 anni o piu', 12 ore dal primo QSO). Single-Op monobanda (160-10 m): High o Low, Mixed. Multi-Ops Single Transmitter, Two Transmitters, Multi Transmitters. SWL.
- Overlay: One Radio, Classic (un solo transceiver, niente decodificatori automatici ne' reti di segnalazione, testi CW standard ammessi), YL, Junior (fino a 19 anni), Single TX/RX (Multi-Ops Single Transmitter con un transceiver).
- Una stazione puo' entrare in due categorie monobanda con un solo log (`CATEGORY-BAND: 10M, 15M`); in una categoria monobanda i QSO sulle altre bande si registrano ma non contano (ma valgono per i corrispondenti).
- Multi-Ops Single Transmitter: regola dei 10 minuti per il cambio di banda, un secondo segnale solo su una banda e solo per moltiplicatori nuovi. Multi-Ops Two Transmitters: otto cambi di banda per ora e per trasmettitore. Multi Transmitters: un segnale per banda.
- Reti di segnalazione e decodificatori automatici ammessi a tutti tranne ai Classic; autosegnalazione ammessa non piu' di una volta ogni 10 minuti (testo 2026); antenne e apparati in un cerchio di 500 m; controllo da remoto ammesso se tutto sta in un solo luogo e il nominativo e' del paese della stazione.

## Penalita' del controllo dei log
- Penalita' di due volte il valore del QSO per nominativo sbagliato o numero sbagliato. Nessun conteggio e nessuna penalita' per differenza d'ora oltre 3 minuti, banda o modo diversi, violazioni della regola dei 10 minuti o dei cambi di banda. Se il risultato scende di oltre il 75 percento il log diventa Checklog.

## Cabrillo
- `CONTEST: RDXC`. Intestazione con la categoria e l'indirizzo postale completo; le stazioni russe indicano la regione in `LOCATION:`. Multi-operatore: il trasmettitore di ogni QSO (RUN = 0 e MULT = 1 per Single Transmitter; RUN1 = 0 e RUN2 = 1 per Two Transmitters). Per chi punta ai primi tre posti la frequenza di ogni QSO al kHz.
- Riga: `QSO: freq mo data ora call rst scambio call rst scambio t`, per esempio `QSO: 21010 CW 2004-03-20 1200 VK3ABC 59 001 RL3A 599 MA`.
- I valori delle righe `CATEGORY-*` non sono elencati dal regolamento: quelli dichiarati sono i valori generali del formato Cabrillo, da verificare sul caricamento web.

## Cosa fa Aurora e cosa no
- Punti (russi, altri, /MM), paesi DXCC/WAE e regioni per banda (regione presa dallo scambio e controllata sull'elenco, per Kaliningrad, Franz Josef Land e Antartide anche dal nominativo), doppi per banda e modo, Cabrillo con `CONTEST: RDXC`.
- Non fa: penalita', regola dei 10 minuti e dei cambi di banda, trasmettitore di ogni QSO, categorie Senior (12 ore), overlay, serie di numeri per banda nei multi-operatore. Una stazione russa in Antartide che opera dal suo nominativo russo conta come stazione dell'Antartide, non russa, per il proprio punto di vista (non distinguibile dal paese).

## Differenze rispetto al vecchio `.contest` («RUSSIAN DX Contest», CW e SSB)
1. Il vecchio file aveva le date del 2014: ora la finestra e' quella del regolamento (terzo fine settimana di marzo).
2. Franz Josef Land e le stazioni russe in Antartide non erano riconosciute come russe; ora lo sono (10 punti, regione FJ e AN, paese a parte).
3. Le stazioni /MM valgono 5 punti e non sono moltiplicatori; i paesi WAE (IT9, GM/s...) sono paesi a parte.
4. Le regioni sono controllate sull'elenco ufficiale (91 sigle); il vecchio elenco `russian.dom` aveva sigle non ufficiali (XTL, XVL, UO...) e mancavano TL, VL, DO, HE, LU, ZP.
5. I punti dei russi fra loro ora distinguono la Russia europea dall'asiatica (2 o 5).

## Punti da verificare
- Valori `CATEGORY-*` del Cabrillo.
- Nel 2026 la fonte e' un testo russo: tradotto dal russo, nessuna traduzione ufficiale letta.
