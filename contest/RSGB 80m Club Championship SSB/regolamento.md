# RSGB 80m Club Championship SSB - regolamento riassunto

- **Fonte:** «3.5MHz Club Championship Contests» (RSGB HF Contest Committee), edizione 2026, <https://www.rsgbcc.org/hf/rules/2026/r80mcc.shtml>; calendario <http://www.rsgbcc.org/cgi-bin/hfreadcal.pl?year=2026> (e `year=2027`, provvisorio); Cabrillo <http://www.rsgbcc.org/hf/rules/Cabrillo/Cabrillo-Information.shtml#3.5CC>.
- **Data di lettura:** 2026-11-24. La serie si chiamava «80m Club Championship» (cosi' la chiama ancora il calendario).
- La serie ha quattro modi in quattro serie separate: CW, SSB, dati (PSK63 e RTTY) e, per i club, la somma. Questa cartella e' la serie **SSB**; la serie dati **non e' stata fatta** (due moduli, PSK63 e RTTY, con un doppio per modo e codici Cabrillo a parte); l'altra serie (CW) ha la sua cartella. Il vecchio `.contest` («CW & SSB») e' sostituito dalla cartella CW.
- Le regole cambiano (2026: i self-spot sono ammessi alle iscrizioni Assisted): ricontrollare a ogni serie.

## Date
- Sessioni di 90 minuti, solo di lunedi', mercoledi' o giovedi'; febbraio-marzo 20:00-21:30 UTC, aprile-luglio 19:00-20:30 UTC. QSO a partire dall'ora di fine non valgono.
- **SSB 2026:** Mon 2 Feb 2000-2130; Thu 26 Mar 2000-2130; Wed 22 Apr 1900-2030; Mon 11 May 1900-2030; Thu 25 Jun 1900-2030; Wed 15 Jul 1900-2030 (UTC). **SSB 2027 (provvisorio):** Mon 1 Feb 2000-2130; Thu 25 Mar 2000-2130; Wed 28 Apr 1900-2030; Mon 10 May 1900-2030; Thu 24 Jun 1900-2030; Wed 14 Jul 1900-2030.
- Log: Cabrillo sul robot <http://www.rsgbcc.org/cgi-bin/hfenter.pl> entro le 23:59:59 UTC del giorno dopo la sessione.

## Banda e modo
- 80 m, SSB 3600-3775 kHz. Una sola frequenza per chiamare CQ; un solo segnale alla volta.

## Sezioni
- 100 W o 10 W (potenza massima in uscita), Assisted o Unassisted (in SSB tutte le iscrizioni sono Assisted); si puo' cambiare sezione da una sessione all'altra. I multi-operatore sono sempre Assisted. Classifiche normalizzate: punteggio / miglior UK&CD della sezione x 1000.
- Iscrizioni per conto di societa' affiliate (solo UK&CD); categoria Locale (80 km dal punto di incontro virtuale) o Generale. Stazione in un solo luogo, antenne e apparati entro 500 m, niente ricezione remota.

## Scambio
- RS(T) + numero progressivo da 1 (un piccolo fuori sequenza non e' penalizzato); se non arriva niente si scrive 0.

## Punti
- 1 punto per QSO; una stazione UK&CD (Regno Unito e dipendenze della Corona: G, GM, GW, GI, GD, GJ, GU) deve stare a uno o a tutti e due i capi.
- Una stazione si lavora una volta per sessione. Un QSO con una stazione RSGB (G6XX o variante regionale) che manda un checklog vale 5 punti.
- Le stazioni non UK&CD compaiono nelle tabelle individuali ma non danno punti alle societa'. Un errore costa 1 punto QSO.
- Nessun moltiplicatore.

## Cabrillo
- `CONTEST: RSGB-80M-CC`; codici dei modi CW e PH (RY e PS per i dati); l'esempio non ha righe CATEGORY (la sezione e' chiesta dal robot); riga QSO `freq modo data ora proprio-nominativo RST nr nominativo RST nr`.

## Cosa fa Aurora e cosa no
- Punti (1 e 5 per il checklog), QSO ammessi (UK&CD a un capo), doppi per sessione, Cabrillo.
- Non fa: normalizzazione, classifiche delle societa', controllo dei segmenti di frequenza, penalita'.

## Differenze rispetto al vecchio `.contest`
1. Il vecchio file valeva per CW e SSB insieme (e il modo digitale); ora sono due cartelle separate, ognuna con le sue sei sessioni.
2. Il vecchio file aveva date del 2014 e un solo QSO libero; ora le date sono quelle del calendario RSGB.
3. Anche il vecchio file aveva il checklog da 5 punti e la regola UK&CD a un capo: restano.
