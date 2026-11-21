# WAE DX Contest CW - regolamento riassunto

Worked All Europe DX Contest del DARC, parte CW.

- **Fonte:** «Worked All Europe DX Contest - Rules 2025», <https://www.darc.de/der-club/referate/conteste/wae-dx-contest/en/wae-rules/> (pagina del 2025-06-17, con le date del 2026); valgono anche le regole generali dei contest DARC (dal 2014).
- **Data di lettura:** 2026-11-21.
- Nome Cabrillo `DARC-WAEDC-CW`: la pagina e quella di caricamento (dxhf2.darc.de) non lo scrivono; viene dall'elenco dei nomi Cabrillo di WA7BNM (<https://www.contestcalendar.com/cabnames.php>, fonte secondaria).
- Il meccanismo dei QTC e la finestra di Aurora sono descritti in `docs/contest-qtc.md`.

## Date
- Secondo fine settimana pieno di agosto, sabato 00:00 - domenica 23:59 UTC. **2026: 8-9 agosto** (regolamento). 2027: 14-15 agosto (calcolata).
- Singolo operatore: al massimo 36 ore su 48, pause di almeno 60 minuti senza QSO ne' QTC.
- Log Cabrillo entro sette giorni (2026: 17 agosto 23:59 UTC) sulle pagine di caricamento del DARC.

## Bande e modo
- 80, 40, 20, 15, 10 m, solo nei segmenti preferiti per i contest della Regione 1 IARU (niente contest a 3570-3800, 7040-7200, 14060-14350 kHz in CW).

## Chi lavora chi
- Valgono solo i QSO fra una stazione europea (elenco WAE) e una non europea (elenco DXCC).

## Scambio
- RST + numero progressivo da 001. Numero non mandato: si scrive 000, e il QSO vale solo se porta un moltiplicatore nuovo.
- Ogni stazione vale una volta per banda.

## Punti
- Ogni QSO e ogni QTC (mandato o ricevuto) vale 1 punto.

## QTC
- Un QTC e' il rapporto di un QSO gia' fatto (ora, nominativo, numero), passato da una stazione non europea a una europea. Ogni QSO una volta sola, mai alla stazione con cui e' stato fatto; al massimo 10 QTC fra due stazioni, a serie da 1 a 10 (`n/m`); di ogni serie si scrivono numero, ora e banda.

## Moltiplicatori (per banda, pesati)
- Stazioni non europee: ogni paese dell'elenco WAE.
- Stazioni europee: ogni paese DXCC non europeo; per W, VE, VK, ZL, ZS, JA, BY, PY e RA8, RA9, RA0 ogni area numerica (W1, K1, KA1 e K3../1 sono tutti W1).
- Peso: 80 m x4, 40 m x3, 20/15/10 m x2.

## Formula
- (QSO + QTC) x somma dei moltiplicatori pesati.

## Categorie
- Single-Op Low (100 W), Single-Op High, Multi-Op (cambi di banda ogni 10 minuti; altri trasmettitori solo per i moltiplicatori su altre bande). Tutto entro un cerchio di 2 km. Cluster ammesso.

## Cosa fa Aurora e cosa no
- Ammissione Europa / resto del mondo (continente dell'entita' WAE: TA1 in Europa, IG9 in Africa), 1 punto per QSO, doppi per banda, paesi WAE per i non europei, paesi e aree per gli europei, pesi delle bande, QTC dalla finestra QTC (`qtc = 1`: dalle stazioni non europee alle europee) sommati ai punti, righe `QTC:` nel Cabrillo con `CONTEST: DARC-WAEDC-CW`.
- Non fa: la regola del 000 (il QSO col numero 000 conta lo stesso), limite delle 36 ore, regola dei 10 minuti.

## Differenze rispetto al vecchio `.contest` («WAEDC (Worked All Europe DX Contest)», CW e SSB)
1. L'elenco WAE non era distinto dal DXCC (IT9, GM/s, TA1, JW/b, 4U1V contavano col paese DXCC): ora sono paesi a parte e TA1 conta come Europa.
2. Mancava la Russia asiatica fra i paesi con le aree (RA8, RA9, RA0).
3. Il vecchio file resta: ha lo stesso nome del WAE RTTY e vale ancora per la fonia (non in cartella), quindi la cartella non lo sostituisce.

## Punti da verificare
- Regola del 000: non applicata.
- IG9/IH9 (Africa italiana): nel database dei paesi e' un'entita' a parte del continente africano; non e' detto che sia nell'elenco WAE. Per un europeo Aurora la conta come paese non europeo col prefisso DXCC (I).
- Nome Cabrillo da fonte secondaria; date 2027 calcolate.
