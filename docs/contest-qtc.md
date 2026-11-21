# QTC del WAE DX Contest (DARC)

Riassunto del regolamento del WAE DX Contest del DARC (pagina «WAE Rules», edizione 2026) per la parte dei QTC, e di come Aurora li
gestisce nella pagina dei modi testuali.

## Le date

- CW: secondo fine settimana pieno di agosto, sabato 00:00 - domenica 23:59 UTC.
- SSB: secondo fine settimana pieno di settembre.
- RTTY: secondo fine settimana pieno di novembre.
- Bande 80, 40, 20, 15 e 10 m; le stazioni con un solo operatore lavorano al massimo 36 ore su 48 (pause di almeno 60 minuti senza QSO
  ne' QTC).

## Chi lavora chi

- CW e SSB: valgono solo i QSO fra una stazione europea (elenco WAE) e una non europea (elenco DXCC).
- RTTY: tutti lavorano tutti, senza limiti di continente.
- Scambio: RST e numero progressivo da 001. Se il corrispondente non manda il numero si scrive 000 (il QSO conta solo se porta un
  moltiplicatore nuovo). Una stazione conta una volta per banda.

## Cos'e' un QTC

Un QTC e' il rapporto di un QSO del contest gia' fatto, passato a un'altra stazione: **ora, nominativo e numero ricevuto** di quel QSO.

- CW e SSB: i QTC vanno solo da una stazione non europea a una europea (l'europea li riceve, la non europea li manda).
- RTTY: ogni stazione puo' mandarli e riceverli, ma solo fra **continenti diversi** (due stazioni dello stesso continente non scambiano
  QTC, anche se il QSO fra loro vale).
- Un QSO si passa come QTC **una volta sola** e **non torna** alla stazione con cui e' stato fatto.
- Fra due stazioni al massimo **10 QTC** in tutto; in RTTY il limite vale per la somma di quelli mandati e ricevuti fra le due. Ci si puo'
  sentire piu' volte (anche su bande diverse) per arrivare a 10.
- Ogni QTC passato bene vale **1 punto** a chi lo manda e 1 a chi lo riceve.

### La serie

I QTC partono a gruppi (serie) da 1 a 10 righe. L'intestazione e' `QTC nn/mm`: `nn` e' il numero della serie (progressivo di chi
manda, da 1), `mm` quante righe contiene. Esempio: `QTC 3/7` = terza serie mandata da quella stazione, sette QTC. Di ogni serie mandata o
ricevuta si scrivono numero, ora e banda: se ne manca uno la serie non conta.

Una riga e' `ora nominativo numero`, per esempio `1332 K5XYZ 112` (in RTTY si usa spesso il trattino: `1332-K5XYZ-112`).

## Punteggio

Punteggio = (QSO + QTC mandati e ricevuti) x somma dei moltiplicatori pesati.

Moltiplicatori per banda: le stazioni europee contano i paesi DXCC non europei (per W, VE, VK, ZL, ZS, JA, BY, PY, RA8, RA9 e RA0 contano
le aree numeriche, fino a dieci per paese); quelle non europee contano i paesi dell'elenco WAE. In RTTY valgono entrambi gli elenchi per
tutti. Peso: 80 m x4, 40 m x3, 20/15/10 m x2.

## Cabrillo

Le righe QTC seguono la sintassi del regolamento:

```
QTC: QRG MODE DATE TIME CALL-RX QTC-GRP CALL-TX TIME-QSO CALL-QSO NR-QSO
QTC: 21011 CW 2024-08-10 1028 DL1XYZ 7/10 K5XYZ 1332 S59XYZ 112
```

`CALL-RX` e' chi riceve la serie, `CALL-TX` chi la manda; `QTC-GRP` e' `serie/righe`. Data e ora sono quelle della serie; in RTTY il modo
e' `RY`.

## Il lavoro dell'operatore

Il modo pratico, in RTTY:

1. Finito il QSO, chi ha QTC chiede «QTC?» o li offre («I have QTC for you, you ready?»); chi non vuole risponde «SRI no QTC».
2. Chi riceve dice di essere pronto («QRV»).
3. Chi manda trasmette l'intestazione `QTC 3/7 QTC 3/7`, poi le righe (tutte di fila o una alla volta), e chiede «QSL?».
4. Chi riceve conferma («QSL ALL QTC» / «CFM ALL QTC») oppure chiede la riga che non ha letto («AGN 4», «AGN ALL», «AGN QTC» per
   l'intestazione); chi manda ripete solo quella riga.
5. Confermate, le righe si segnano come mandate (da una parte) e ricevute (dall'altra).

## In Aurora

- I contest in cartella `WAE DX Contest CW` e `WAE DX Contest RTTY` (`contest/`) lo dicono con `[contest] qtc` (1 e 2) e contano i paesi dell'elenco WAE (IT9, GM/s, TA1...) a parte, con i pesi delle bande anche per i moltiplicatori dello script; i vecchi file WAEDC restano (hanno lo stesso nome per CW/SSB e RTTY).
- Il file `.contest` dice se il contest ha i QTC con la chiave `m_iQtc` (0 nessuno, 1 come in CW e SSB: dalle stazioni non europee alle
  europee, 2 come in RTTY: in entrambe le direzioni fra continenti diversi). Senza la chiave, un contest con «WAE» nel nome usa 2 se e'
  RTTY e 1 altrimenti.
- I QTC stanno in `contest/qtc-<id>.json` nella cartella dei dati, ognuno con stazione, serie, ora, banda, modo e frequenza della serie e
  la riga (ora, nominativo, numero; per quelli mandati anche l'istante del QSO del log).
- La finestra **QTC** (macro `<QTCSEND>` e `<QTCREAD>`, anche con i nomi `<STATS:SendQTC>` e `<STATS:ReadQTC>`):
  - **Mandare**: per il nominativo del QSO propone i QSO del contest ancora mandabili (non suoi, non gia' mandati, con il numero
    ricevuto, al massimo quanti ne restano dei 10), con il numero di serie dopo l'ultimo mandato. Tasti per intestazione, tutte le righe,
    una riga, «QSL?»; ogni riga si spunta quando e' confermata e «Salva» la registra come mandata.
  - **Ricevere**: «QRV» chiede la serie, «Prendi dalla ricezione» legge dalla finestra RX l'intestazione `QTC n/m` e le righe, che si
    correggono a mano; «AGN n», «AGN ALL», «AGN QTC» chiedono le ripetizioni; «Salva» registra le righe e manda la conferma.
- Il punteggio aggiunge i QTC ai punti; il Cabrillo ha le righe `QTC:` dopo quelle `QSO:`.
- Moltiplicatori del WAE RTTY (sezione `[Score]`, `docs/contest-regole.md`): paesi DXCC per banda, con le aree numeriche al posto del
  paese per W, VE, VK, ZL, ZS, JA, BY, PY e Russia asiatica, pesi 4/3/2. L'elenco WAE (IT9, GM/Shetland, JW/Bear...) non e' ancora
  distinto: il punteggio e' una stima, quello vero lo calcola il DARC. I file WAE CW e SSB hanno ancora le vecchie chiavi.
