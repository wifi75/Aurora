# Piano: pagina «Contest» di Aurora

Stato: fasi 0-6 fatte nel ramo `lavoro/contest` (2026-11-08); cosa resta in `docs/tasks.md`, voce 27. I file di regole stanno in `core/crates/contest/regole` (non in `data/contest`, che e' la cartella dei dati del programma) e il log del contest in un file JSON per edizione nella cartella dei dati (non in tabelle del `logbook`). Ricerca fatta l'8 novembre 2026 su fonti pubbliche (link in fondo e accanto a ogni scheda).
Le regole dei contest cambiano ogni anno: ogni scheda riporta l'anno della fonte letta e i punti da ricontrollare.

## 1. In breve

- Oggi chi fa un contest in FT8/FT4 usa **due o tre programmi**: WSJT-X o JTDX per la sequenza, N1MM Logger+ (o WriteLog con DigiRite) per doppi, moltiplicatori, punteggio e Cabrillo, a volte JTAlert o GridTracker per gli avvisi. Fra questi programmi passano pacchetti UDP che vanno configurati a mano, e i problemi noti nascono proprio lì: porte occupate, QSO persi, seriali fuori sequenza, nominativo sovrascritto.
- Aurora ha già quasi tutti i pezzi di base:
  - **il codec a 77 bit con tutti i messaggi di contest**: Field Day 0.3/0.4, RTTY Roundup tipo 3, EU VHF tipo 5 con due hash, `/P` e `/R` (`core/crates/ft8/src/message.rs`);
  - la lettura dello scambio ricevuto (`engine-native/src/features.rs`, `contest_exchange`);
  - i campi ADIF dei contest (`logbook/src/adif.rs`, `contest_fields`);
  - il pacchetto `contactinfo` per N1MM (`qso-export`, `n1mm_contact_info`);
  - un Cabrillo, per ora solo nel Fox log (`app/src/lib/foxLog.ts`);
  - il simulatore (`engine-sim`), il DX Cluster, le mappe e la pagina Fox.
- Manca ancora:
  - la **sequenza contest nel motore**: `exch_sent` resta vuoto, vedi `docs/tasks.md`;
  - un **log del contest** con doppi, punti e moltiplicatori;
  - i **file di regole**;
  - la **pagina**.
- L'obiettivo è un solo programma che faccia tutto: sequenza, doppi, moltiplicatori, punteggio, rate, controllo delle regole, Cabrillo/EDI e invio. N1MM resta collegabile per chi lo vuole.

## 2. Programmi a confronto

### 2.1 Tabella

| Programma | Sequenza contest | Doppi / moltiplicatori in diretta | Punteggio, rate | Export | Contest coperti | Note |
|---|---|---|---|---|---|---|
| **WSJT-X 2.7 / 3.x** | Sì: menu «Special operating activity» con NA VHF, EU VHF, Field Day, FT Roundup (RU), WW Digi, ARRL Digi, Fox, Hound, Q65 Pileup | Solo «già lavorato» a colori. Per ARRL Digi c'è la finestra «Active Stations» con i punti di ogni stazione | Contest log con il totale dei QSO; nessun punteggio vero, nessun rate (il rate c'è solo nel Fox log) | Cabrillo da File; ADIF con gli scambi (3.2.0-rc1); `cablog` per l'EME | I 6 formati di messaggio | Riferimento del protocollo. Niente nominativi composti nei messaggi di contest; niente multi-QSO in contest |
| **JTDX 2.2.159** | Stesse attività di WSJT-X | Colori lavorato/nuovo | No | ADIF | Come WSJT-X | Il decoder migliore sui segnali deboli (+6% nel test W1UE). Non è pensato per i contest |
| **JTDX_contest (CE3TSK)** v3.0.0-rc11 | Sì: WW Digi, ARRL Digi, YBDXPI FT8, International FT Challenge | Punti accanto a ogni decodifica. La scelta automatica privilegia chi vale più punti; i doppi restano visibili ma in fondo | Punteggio M×P=S in diretta | Cabrillo per ogni sponsor; un ADIF separato per contest | 4 HF | Fork GPL. Risponde soltanto, non avvia QSO. Niente Field Day, VHF, rate, band map, UDP verso N1MM |
| **MSHV (LZ2HV)** | Sì: WW Digi, ARRL Digi (ex «RTTY Roundup»), FT4/FT8 DX Contest, FT Challenge, ARRL EME | In parte | No | Cabrillo | HF più EME | Multi Answering fino a 5 segnali: vietato nei contest per chi gareggia a un solo segnale. Decodifica tardiva (4-5 s nel periodo dopo) nel test W1UE |
| **DigiRite + WriteLog** | Sì: sequenziatore proprio, «QSO in progress» con priorità, fino a 5 flussi | Sì, dal logger (giallo = mult, rosso = doppio); scelta del moltiplicatore dal pannello | Sì (WriteLog) | Cabrillo (WriteLog) | WW Digi, FT Roundup e i contest di WriteLog | L'integrazione più stretta fra sequenza e logger. Installer non più distribuito (si compila dal sorgente); disputa di licenza con WSJT-X; difficile da configurare |
| **N1MM Logger+** con WSJT-X/JTDX | No: la fa WSJT-X | Sì: finestra «WSJT-X Decode List» con colori (grigio doppio, blu lavorabile, rosso mult) | Sì: punteggio, rate, statistiche | Cabrillo e tutti i formati | NA VHF, EU VHF, WW Digi, Field Day, FT Roundup, ARRL Digi (log type ARRLIDC, WWDIGI, FTROUNDUP) | Lo standard di fatto. Servono porte UDP (2237, multicast 239.x se c'è anche JTAlert) e, per JTDX, una porta TCP. Non si scrive in N1MM mentre WSJT-X è attivo; seriali fuori sequenza; QSO persi segnalati nel 2026 |
| **DXLog.net** | No | Riceve i QSO di WSJT-X via UDP; spot locali dalle decodifiche | Sì | Cabrillo | Solo contest con la griglia come unico scambio | Gli spot dalle decodifiche possono sovraccaricarlo. Integrazione più stretta rinviata dagli autori |
| **Win-Test** | No | — | — | — | Nessuna traccia di supporto FT8 | Solo RTTY tramite MMTTY |
| **JTAlert** | No | Già lavorato in base al log; niente doppi di contest verificati | No | — | — | Occupa la porta UDP di WSJT-X: con N1MM serve il multicast, altrimenti i QSO vanno a log due volte |
| **GridTracker 2** | No | Mappa delle griglie lavorate e da lavorare, Call Roster con nuovi DXCC/stati/zone | No | — | — | Nella 1.20 c'erano «Contest Map Overlays»; nella 2 non è verificato |
| **Decodium 4** | Auto Seq, Auto CQ, Multi Answer, ma nessuna attività di contest documentata | No | No | ADIF | — | Nessuna funzione contest |
| **Log4OM 2** | No | Modalità contest con doppi (DUPE / DUPE BAND) | Parziale | ADIF (Cabrillo con un convertitore esterno) | — | Segnalato lento in gara |
| **Swisslog, RUMlogNG, Wavelog/Cloudlog** | No | Minimi | No | RUMlog: upload Cabrillo | — | Swisslog stesso consiglia N1MM per i contest |

### 2.2 Il meglio da prendere

Ogni voce dice da quale programma viene.

- **WSJT-X**:
  - i formati dei messaggi e la sequenza (riferimento: `widgets/mainwindow.cpp` di WSJT-X 3.0.1 in `D:\build\wsjtx-src`, `genStdMsgs` intorno alle righe 9480-9640: parola del CQ per attività, scambio inviato, `R` davanti, seriale a 4 cifre, EU VHF con due hash);
  - i tasti alternativi F1-F6 per reinviare RR73;
  - la finestra Active Stations (stazioni ordinate per punti) e la parola del CQ personalizzabile («Individual contest name»).
- **N1MM**: i colori dei doppi e dei moltiplicatori sulle decodifiche, il rate, il riepilogo per banda, Cabrillo per ogni sponsor.
- **DigiRite**: la coda dei «QSO in progress» con priorità; la risposta automatica solo ai non doppi; il CQ «quando libero» che si spegne dopo 5 minuti senza intervento umano (utile per la regola dell'avvio umano).
- **JTDX_contest**: i punti accanto a ogni decodifica e un log separato per ogni contest con il punteggio M×P=S.
- **GridTracker**: la mappa delle griglie e dei campi lavorati e da lavorare.
- **MSHV**: il supporto EME e Q65 nei contest. Il multi-risposta no, perché nei contest è vietato.

### 2.3 Lamentele ricorrenti degli utenti

| # | Problema | Fonte |
|---|---|---|
| 1 | **NIL al 5-6% in FT contro l'1-2% in CW/SSB/RTTY**: molti non mettono a log senza il «73» finale superfluo | W0YK, Contest University 05/2026; ww-digi.com/operating |
| 2 | RR73 non ricevuto: non si sa se l'altro ha messo a log; Tx3 ripetuto dal partner = RR73 perso | VE3VN; guida WSJT-X |
| 3 | «CQ WW» al posto di «CQ TEST» per un'impostazione di WSJT-X; si chiede una sola parola del CQ e una casella «contest» | soapbox ARRL Digi 2022 |
| 4 | Chi non fa il contest risponde con il rapporto in dB invece della griglia: lo scambio non vale | ww-digi.com; soapbox ARRL Digi |
| 5 | Doppi non visibili in WSJT-X: chiamate continue da stazioni già lavorate | soapbox ARRL Digi 2022 |
| 6 | Ritmo lento («quasi insopportabile»); serve scegliere bene fra FT8 e FT4 | soapbox; VA7ST; W0YK |
| 7 | Due programmi e l'UDP: porta 2237 contesa con JTAlert, QSO persi (101 arrivati su 190, luglio 2026), documentazione povera | HamApps groups.io; World Radio League; FlexRadio |
| 8 | Log rifiutati: griglia mancante, intestazione sbagliata (CONTEST), modo RY invece di DG nel CQ WW VHF | WRL 06/2026; cqww-vhf.com/ft8 |
| 9 | Nominativi composti o speciali esclusi dai messaggi di contest | guida WSJT-X |
| 10 | Field Day: sezioni mancanti nelle prime versioni; regola «una volta per banda e modo» (digitale = un solo modo) mal gestita nei doppi | wsjt-devel 2018 e 2025 |
| 11 | Formato EU VHF di WSJT-X (seriale + locatore a 6) diverso dalle regole IARU MGM (rapporto + 4 caratteri, senza seriale) | Handbook IARU R1 V10.03 |
| 12 | Regole sull'automazione: l'ARRL chiede un avvio umano da entrambe le parti; Fox/Hound e multi-flusso vietati | ARRL news 2019; ww-digi; ARI; DARC |
| 13 | Primo WW Digi: la sequenza non partiva, frequenza del contest assente dal menu, radio che tornava su 14074 | VE9KK 2020 |

## 3. Schede dei contest

Campi uguali per tutte le schede, già nella forma del file di regole (sezione 4). «Da ricontrollare» vuol dire che la fonte è di un anno precedente o non è stata letta per intero.

### 3.1 WW Digi DX Contest (WWROF)

| Campo | Valore |
|---|---|
| id | `ww-digi` |
| Quando | Ultimo fine settimana completo di agosto, 12:00:00 sabato - 11:59:59 domenica UTC (2026: 29-30 agosto) |
| Bande | 160, 80, 40, 20, 15, 10 m |
| Modi | FT4 e FT8 (una stazione una sola volta per banda, in FT4 o FT8) |
| Sottobande consigliate | FT8 da 1.844, 3.590, 7.090, 14.090, 21.090, 28.090; FT4 da 1.840, 3.580, 7.080, 14.080, 21.080, 28.080 MHz (JA 80 m: 3.570/3.573) |
| Messaggi | Attività WSJT-X `WW_DIGI`: `CQ WW IU3ABC IO91` / `IU3ABC K1ABC FN42` / `K1ABC IU3ABC R IO91` / `IU3ABC K1ABC RR73` |
| Scambio inviato | griglia a 4 caratteri |
| Scambio ricevuto | griglia a 4 caratteri |
| Doppi | per banda (FT4 e FT8 insieme) |
| Punti | 1 + floor(distanza / 3000 km) fra i centri dei quadrati a 4 caratteri, cammino corto (esempio ufficiale: 5541 km = 2) |
| Moltiplicatori | campi a 2 caratteri (324), contati per banda |
| Punteggio | somma dei punti × somma dei moltiplicatori di ogni banda |
| Categorie | SINGLE-ONE (HIGH 1500 W, LOW 100 W, QRP 5 W; tutte le bande o una), SINGLE-UNLIMITED, MULTI-ONE (8 cambi di banda per ora), MULTI-TWO, MULTI-UNLIMITED, CHECKLOG |
| Restrizioni | il primo contatto con ogni stazione lo deve avviare un operatore umano: niente robot. Multi-flusso e Fox/Hound vietati |
| Log | Cabrillo 3.0, `CONTEST: WW-DIGI`, `CATEGORY-MODE: DIGI`, modo QSO `DG`, frequenza in kHz o banda; `LOCATION` = sezione ARRL/RAC o `DX` |
| Invio | upload su ww-digi.com/logcheck entro il 1° settembre 2026 23:59 UTC |
| Fonti | https://ww-digi.com/rules/ · https://ww-digi.com/cabrillo.htm · https://ww-digi.com/operating.htm |

### 3.2 ARRL International Digital Contest («ARRL Digi»)

| Campo | Valore |
|---|---|
| id | `arrl-digi` |
| Quando | Primo fine settimana completo di giugno, 18:00 sabato - 23:59 domenica UTC (30 h) |
| Ore operative | single op: 24 su 30, pause di almeno 60 minuti in una o due volte; categoria Limited: 8 ore; multi op: 30 ore |
| Bande | 160, 80, 40, 20, 15, 10, 6 m |
| Modi | qualunque modo digitale tranne RTTY (in pratica FT8/FT4); una stazione una volta per banda, qualunque modo |
| Messaggi | Attività `ARRL_DIGI` (stesso formato di NA VHF): `CQ TEST IU3ABC IO91` / `IU3ABC K1ABC FN42` / `K1ABC IU3ABC R IO91` / `IU3ABC K1ABC RR73` / `K1ABC IU3ABC 73` |
| Scambio | griglia a 4 caratteri |
| Punti | 1 + ceil(distanza / 500 km), con almeno 1 punto di distanza (1565 km: 1 + 4 = 5) |
| Moltiplicatori | **nessuno**: il punteggio è la somma dei punti |
| Categorie | Single Op 1 radio e 2 radio (LP, QRP), Multi Op Single Transmitter (LP, QRP); niente High Power; Limited 8 h; squadre di 2-5 single op entro 175 miglia |
| Restrizioni | ogni contatto avviato da un operatore su entrambi i lati, niente automazione |
| Log | Cabrillo; `CONTEST: ARRL-DIGI` (da ricontrollare sul modulo ARRL); modo `DG` |
| Invio | contest-log-submission.arrl.org entro 7 giorni |
| Fonti | https://contests.arrl.org/ContestRules/Digital-Rules.pdf (v1.1) · https://www.arrl.org/arrl-digital-contest |

### 3.3 ARRL Field Day

| Campo | Valore |
|---|---|
| id | `arrl-fd` |
| Quando | Quarto fine settimana completo di giugno, 18:00 sabato - 20:59 domenica UTC (2026: 27-28 giugno). Classi A e B: 27 h se il montaggio inizia dopo le 18:00, altrimenti 24 h |
| Bande | HF e VHF/UHF, senza 60, 30, 17 e 12 m (da ricontrollare sul PDF completo) |
| Messaggi | Attività `FIELD_DAY`: `CQ FD IU3ABC IO91` / `IU3ABC K1ABC 2A EMA` / `K1ABC IU3ABC R 1D DX` / `IU3ABC K1ABC RR73` |
| Scambio | classe (numero di trasmettitori + lettera A-F) e sezione ARRL/RAC; le stazioni fuori USA/Canada mandano `DX` |
| Doppi | per banda e per modo (tutto il digitale conta come un modo solo) |
| Punti | 2 per QSO digitale (1 fonia, 2 CW) |
| Moltiplicatori | potenza: ×5 (≤5 W senza rete né generatore), ×2 (≤100 W), ×1 (>100 W) |
| Bonus | lista di 20 voci (energia d'emergenza, luogo pubblico, messaggio alla sezione, satellite, GOTA, invio via web app 50 punti e altre): da spuntare a mano |
| Log | modulo web field-day.arrl.org più dupe sheet o Cabrillo (`CONTEST: ARRL-FD`) |
| Invio | entro il 28 luglio 2026 |
| Note per Aurora | dall'Italia si fa poco (si manda `1D DX`); la regola che obbliga i non USA/Canada a mandare `DX` è già nel controllo dei messaggi (`docs/analisi-jtdx-reverse-rea.md`, voce 7) |
| Fonti | https://www.arrl.org/field-day-rules |

### 3.4 International FT Challenge (erede dell'FT Roundup)

| Campo | Valore |
|---|---|
| id | `ft-challenge` |
| Quando | Primo fine settimana di dicembre, 18:00 sabato - 23:59 domenica UTC; contano solo le prime 24 ore; pause di almeno 30 minuti. Nella pagina delle regole le date 2025 e 2026 (5-6 dicembre) non coincidono: da ricontrollare |
| Bande | 80, 40, 20, 15, 10 m (14.083 consigliato per l'FT4); evitare i beacon 14.100, 21.150, 28.500 |
| Modi | FT4 o FT8, una volta per banda |
| Messaggi | **messaggi normali di tutti i giorni**, senza attività speciale: griglia nel Tx1 e SNR nel rapporto. Chi non fa il contest dà punti lo stesso |
| Scambio | griglia a 4 caratteri + SNR. Griglia mancante = `ZZ00` (1 punto, nessun moltiplicatore) |
| Punti | 1 + floor(distanza / 3000 km) |
| Moltiplicatori | campi a 2 caratteri per banda (ZZ escluso) |
| Categorie | SO Low (≤100 W), SO QRP (≤5 W), overlay `CLASSIC` (una radio), Multi Op Low; oltre 100 W = checklog |
| Log | Cabrillo con `CATEGORY-POWER`; upload su rttycontesting.com |
| Note | L'**FT Roundup** (scambio rapporto + stato/provincia o seriale, `CQ RU`) è **chiuso**: il formato di messaggio RU di WSJT-X resta per compatibilità e per altri contest che lo usano |
| Fonti | https://www.rttycontesting.com/ft-challenge/rules/ · https://www.rttycontesting.com/ft-roundup/rules/ |

### 3.5 ARRL January, June e September VHF

| Campo | Valore |
|---|---|
| id | `arrl-vhf-jan`, `arrl-vhf-jun`, `arrl-vhf-sep` |
| Quando | Giugno: secondo fine settimana completo, 18:00 sabato - 02:59 lunedì UTC (gennaio e settembre: vedi calendario ARRL) |
| Chi | USA e Canada lavorano tutti; il resto del mondo lavora solo USA e Canada (rilevante per l'Europa solo sui 6 m in Es o TEP) |
| Bande | dai 50 MHz in su, qualunque modo |
| Messaggi | Attività `NA_VHF`: `CQ TEST K1ABC FN42` / `K1ABC W9XYZ EN37` / `W9XYZ K1ABC R FN42` / `K1ABC W9XYZ RRR` (o RR73) / `W9XYZ K1ABC 73`; `/R` per i rover |
| Scambio | griglia a 4 caratteri |
| Doppi | una volta per banda da ogni griglia: un rover che cambia griglia è un QSO nuovo |
| Punti | 50 e 144 MHz: 1; 222 e 432: 2; 902 e 1296: 4 a gennaio, 3 a giugno e settembre; 2,3 GHz e oltre: 8 a gennaio, 4 a giugno e settembre |
| Moltiplicatori | griglie diverse per banda. Rover: più 1 per ogni griglia da cui ha trasmesso |
| Categorie | SOLP, SOHP, Single Op Portable, 3 bande, FM-only, Classic/Limited/Unlimited Rover, Limited e Unlimited Multi; sottocategorie All Mode e Analog Only |
| Log | Cabrillo, `CONTEST: ARRL-VHF-JUN` (e `-JAN`, `-SEP`), modo `DG`, frequenza come banda (50, 144, 222, 432, 902, 1.2G...) |
| Invio | web app ARRL entro 10 giorni |
| Fonti | https://contests.arrl.org/ContestRules/JanJunSep-VHF-Rules.pdf · https://www.arrl.org/june-vhf |

### 3.6 CQ World Wide VHF, fine settimana digitale

| Campo | Valore |
|---|---|
| id | `cq-ww-vhf-digi` |
| Quando | Dal 2026 il fine settimana digitale è separato: 18-19 luglio 2026, dalle 14:00 sabato alle 14:00 domenica UTC (analogico il 4-5 luglio) |
| Bande | 6 m e 2 m |
| Modi | solo digitali (FT4, FT8, MSK144, Q65...) |
| Messaggi | NA VHF (`CQ TEST`) con griglia; `/R` per i rover |
| Scambio | griglia a 4 caratteri, senza rapporto (il rapporto non va messo a log) |
| Punti | 1 sui 6 m, 2 sui 2 m |
| Moltiplicatori | griglie diverse per banda (per i rover si ricontano a ogni cambio di griglia) |
| Categorie | Single Op HP (1500 W), LP (100 W), QRP (10 W), tutte le bande o una; Hilltopper (6 ore, portatile, ≤100 W); Rover; Multi-Op |
| Log | Cabrillo, `CONTEST: CQ-VHF`, modo **`DG`** (non RY), `LOCATION` per gli USA |
| Invio | upload su cqww-vhf.com/logcheck entro il 24 luglio 2026 |
| Fonti | https://cqww-vhf.com/rules/2026_cq_ww_vhf_rules.pdf · https://cqww-vhf.com/ft8.htm |

### 3.7 IARU Regione 1, contest MGM 50 e 70 MHz

| Campo | Valore |
|---|---|
| id | `iaru-r1-50-mgm`, `iaru-r1-70-mgm` |
| Quando | 50 MHz: terzo sabato di aprile; 70 MHz: terzo sabato di maggio; dalle 14:00 sabato alle 14:00 domenica UTC |
| Modi | MGM (FT8, FT4, MSK144, Q65...). I contest IARU 145 MHz e UHF di settembre e ottobre sono **solo CW, SSB e FM** |
| Messaggi | messaggi standard: basta lo scambio normale (rapporto e locatore a 4 caratteri), **senza seriale**. La guida IARU accetta altri protocolli purché lo scambio avvenga tutto via radio |
| Scambio | rapporto + locatore a 4 caratteri |
| Doppi | una volta per banda; i doppi vanno a log senza punti e segnati `D` |
| Punti | 1 punto per km (km troncati + 1) fra i centri dei quadrati grandi (calcolati come `XXnnMM`), coefficiente 111,2 km/grado; QSO nello stesso quadrato = 50 punti |
| Moltiplicatori | quadrati grandi (`IO91`) lavorati |
| Punteggio | km totali × quadrati lavorati |
| Sezioni | SO-MGM, MO-MGM, 6H-MGM (6 ore in al massimo 2 periodi, pausa oltre 2 ore) |
| Log | EDI (REG1TEST) per ogni banda; modo EDI **7** per ogni QSO MGM; intestazione minima PCall, PWWLo, PSect, PBand, RCall/MOpe, RHBBS, SPowe, SAnte |
| Invio | entro il primo mercoledì dopo il contest, 23:59:59 UTC |
| Altro | autospot sul cluster ammesso all'inizio su una frequenza nuova, poi non più di uno ogni 10 minuti (decisione IARU di gennaio 2026); vietato completare lo scambio fuori dalla radio |
| Fonti | IARU R1 VHF Managers Handbook V10.03, febbraio 2026: https://www.iaru-r1.org/wp-content/uploads/2026/02/VHF_Handbook_V10_03_final.pdf |

### 3.8 Attività digitali mensili VHF/UHF e ARI FT8 Activity Contest

| Campo | Valore |
|---|---|
| id | `ari-iac-ft8-144`, `ari-iac-ft8-432` (schema IARU «Monthly Digital Activity Contest», VGC 2020) |
| Quando | 144 MHz: primo mercoledì del mese; 432 MHz: secondo mercoledì. ARI: 17:00-20:00 UTC da aprile a ottobre, 18:00-21:00 UTC da novembre a marzo |
| Chi | ARI: tutti gli OM italiani; almeno un QSO con una stazione italiana |
| Frequenze | 144,174 e 432,174 MHz USB |
| Messaggi | **procedura FT8 standard**. **Vietati «Fox/Hound» e «Contest Mode»** |
| Scambio | rapporto + locatore a 4 caratteri |
| Punti | 1 per QSO completo |
| Moltiplicatori | locatori a 4 caratteri |
| Punteggio | punti × moltiplicatori per tornata; classifica annuale = somma × mesi di partecipazione (massimo 12) |
| Log | **ADIF** (si può caricare tutto il log: il robot estrae la tornata) su contest.ari.it entro 3 giorni |
| Validi | non valgono EME, satellite, ripetitori |
| Fonte | https://contest.ari.it/rules/regolamentoiacft8_2026.pdf |
| Gli altri contest ARI VHF | Trofei ARI V-U-SHF, Sezioni, Alpe Adria, 50 MHz: **i modi digitali non sono ammessi**. Lo scambio è RS(T) + progressivo + locatore a 6 (Sezioni: più il codice di sezione); log EDI. Aurora può comunque tenere il log EDI di chi li fa in CW/SSB, ma non è il suo campo |

### 3.9 EU VHF di WSJT-X (contest nazionali con seriale)

| Campo | Valore |
|---|---|
| id | `eu-vhf` (formato generico, da associare ai contest che lo chiedono) |
| Messaggi | Attività `EU_VHF`: `CQ TEST G4ABC IO91` / `G4ABC PA9XYZ JO22` / `<PA9XYZ> <G4ABC> 570123 IO91NP` / `<G4ABC> <PA9XYZ> R 580071 JO22DB` / `PA9XYZ G4ABC RR73`; `/P` ammesso su entrambi |
| Scambio | rapporto 52-59 + seriale 0001-4095 (6 cifre in tutto) + locatore a 6 caratteri (tipo 5, due hash a 12 e 22 bit) |
| Uso | contest nazionali che accettano i modi digitali con lo scambio «classico» IARU (rapporto + progressivo + locatore a 6). **Non** serve per i contest MGM IARU (3.7) né per le attività mensili (3.8) |
| Log | EDI con mode code 7, oppure Cabrillo secondo lo sponsor |

### 3.10 DARC FT4 Contest

| Campo | Valore |
|---|---|
| id | `darc-ft4` |
| Quando | Secondo martedì di febbraio, maggio, agosto, novembre, 19:00-20:29 UTC (2026: 10/2, 12/5, 11/8, 10/11) |
| Bande | 80 m (3576, 3579, 3582 kHz) e 40 m (7047,5 kHz) |
| Modo | FT4; **Fox vietato**; una volta per banda |
| Scambio | rapporto + locatore a 4 caratteri, con messaggi standard (rapporti veri, non sempre lo stesso) |
| Punti | 1 per QSO |
| Moltiplicatori | prefissi diversi per banda |
| Categorie | SO LP (100 W), SO QRP (5 W), Trainee (DN1-DN8 e /T, solo DL), checklog; DL e non DL in classifiche separate |
| Log | Cabrillo via web entro 7 giorni (niente /P o /QRP nel nominativo) |
| Fonte | https://www.darc.de/der-club/referate/conteste/ft4-contest/darc-ft4-contest-english-version/darc-ft4-rules |

### 3.11 Altri contest con FT8/FT4 da coprire

| Contest | Quando | Scambio | Note | Fonte |
|---|---|---|---|---|
| RSGB FT4 Series e FT4 International Activity Day | serie serali; IAD ad aprile (2026: 11-12 aprile, 12:00-12:00) | solo rapporto | l'RSGB pensa di togliere l'obbligo di `CQ RSGB`/`CQ TEST` per non perdere chi passa | rsgb.org Contest News 2026 |
| PACCdigi (VERON) | aprile (2026: 18 aprile, 07:00-19:00 UTC) | da ricontrollare | RTTY e FT4/FT8 (FT4 e FT8 sono lo stesso protocollo); moltiplicatori = 12 province olandesi per banda e per modo (massimo 24 per banda) | https://veron.nl/vereniging/commissies-en-werkgroepen/traffic-bureau/hf-contesten/paccdigi-rules/ |
| RSGB 432 MHz FT8 Activity | serate, 4 ore | rapporto + locatore a 4 | schema simile alle attività mensili | rsgb.org Contest News 7/12/2025 |
| ARRL RTTY Roundup | gennaio | — | **dal 2023 solo RTTY**: FT8/FT4 non valgono più | https://www.arrl.org/rtty-roundup |
| Winter Field Day | gennaio | — | **FT4 e FT8 non valgono** | https://winterfieldday.org/downloads/2025-rules.pdf |
| ARRL EME, contest Q65/JT65 | autunno | rapporto/«R» | Q65 a 77 bit con le stesse attività NA VHF; MSHV e WSJT-X (`cablog`) | wsjt.sourceforge.io Release Notes |

### 3.12 Calendario dei prossimi 12 mesi

Da novembre 2026. Le date vanno confermate ogni anno.

| Mese | Contest |
|---|---|
| novembre | DARC FT4 (10/11/2026) |
| dicembre | International FT Challenge (5-6/12/2026, da confermare) |
| gennaio | ARRL January VHF |
| febbraio | DARC FT4 |
| ogni mese, 1° e 2° mercoledì | ARI FT8 Activity 144/432 |
| aprile | IARU R1 50 MHz MGM (terzo sabato), PACCdigi, RSGB FT4 IAD |
| maggio | IARU R1 70 MHz MGM, DARC FT4 |
| giugno | ARRL Digi (primo fine settimana), ARRL June VHF, ARRL Field Day |
| luglio | CQ WW VHF digitale |
| agosto | DARC FT4, WW Digi (ultimo fine settimana) |
| settembre | ARRL September VHF |

## 4. File di regole

### 4.1 Scelte

- **TOML**, uno per contest, in `data/contest/<id>.toml`, distribuiti con Aurora e aggiornabili come gli altri dati (copia nella cartella dell'utente, che può modificarla o aggiungerne).
- **Dichiarativo**: niente script. Formule e moltiplicatori si scelgono da un elenco chiuso di tipi implementati nel core. Quello che non entra nell'elenco si aggiunge come nuovo tipo nel core, non come codice dentro il file.
- **Versionato per anno**: `edizione = 2026`. Ogni file porta la fonte (URL) e la data in cui è stato verificato; la pagina mostra «regole verificate il ...».
- Le **liste** (sezioni ARRL/RAC, stati, province canadesi, province olandesi, campi Maidenhead) stanno in `data/contest/liste/*.toml` e i file di regole le richiamano per nome. Le sezioni del Field Day e gli stati del Roundup sono già in `ft8/src/message.rs`: la lista dati e quella del codec devono restare uguali (prova al build).

### 4.2 Campi

| Sezione | Campo | Significato / valori |
|---|---|---|
| `[contest]` | `id`, `nome`, `sponsor`, `edizione`, `url_regole`, `verificato_il` | identità |
| | `famiglia` | `hf`, `vhf`, `field-day`, `attivita` |
| `[calendario]` | `regola` | `"primo-weekend-completo:giugno"`, `"ultimo-weekend-completo:agosto"`, `"terzo-sabato:aprile"`, `"mercoledi:1"`, `"secondo-martedi:febbraio,maggio,agosto,novembre"`, `"primo,secondo,terzo-mercoledi:tutti"` (piu' ordinali), `"ogni-giovedi:tutti"` (settimanale) |
| | `inizio`, `fine` | ora UTC e giorno relativo (`sab 18:00`, `dom 23:59`) |
| | `date_fisse` | eccezioni per anno (`2026 = ["2026-07-18T14:00Z", "2026-07-19T14:00Z"]`) |
| | `ore_massime_single`, `pausa_minima_min`, `pause_massime` | ARRL Digi 24/60/2; FT Challenge 24/30; IARU 6H |
| `[bande]` | `ammesse` | `["160m","80m",...]` |
| | `sottobande` | per banda e modo, frequenza di partenza e passo di salita (ARRL: +2 kHz; W0YK: +3 kHz) |
| | `escluse` | per esempio WARC |
| `[modi]` | `ammessi` | `["FT8","FT4"]`, `["FT8","FT4","MSK144","Q65"]` |
| | `stesso_modo_per_doppi` | `true` se FT4 e FT8 contano come un modo solo |
| `[messaggi]` | `attivita` | `ww-digi`, `arrl-digi`, `na-vhf`, `eu-vhf`, `field-day`, `roundup`, `standard` |
| | `parola_cq` | `WW`, `TEST`, `FD`, `RU` o vuota |
| | `chiusura` | `RR73` o `RRR` |
| | `settantatre_finale` | `consigliato`, `facoltativo`, `no` (riduce i NIL, consiglio di W0YK) |
| `[scambio]` | `inviato` | elenco di campi: `grid4`, `grid6`, `rst`, `snr`, `seriale`, `classe`, `sezione`, `stato`, `provincia` |
| | `ricevuto` | idem, con le regole di validazione (`lista = "sezioni-arrl"`, `alternativa = "DX"`) |
| | `seriale` | `per-log` o `per-banda`; partenza 1; cifre 3 o 4 |
| | `fuori_area` | per esempio Field Day: chi è fuori USA/Canada manda `DX` |
| `[doppi]` | `chiave` | `banda`, `banda+modo`, `banda+griglia-rover` |
| | `log_doppi` | `si-senza-punti` (IARU) o `no` |
| `[punti]` | `tipo` | `fisso`, `per-banda`, `per-modo`, `distanza` |
| | `fisso` / `tabella` | punti fissi, o per banda (VHF ARRL: `50MHz=1`, `432MHz=2`...), anche diversi per mese |
| | `distanza` | `base`, `passo_km`, `arrotonda` (`su`/`giu`), `minimo_extra`, `riferimento` (`centro-grid4`, `centro-grid6`), `stesso_quadrato`, `km_per_grado` (111,2 IARU) |
| `[moltiplicatori]` | elenco di `{ tipo, ambito }` | tipi: `campo2`, `grid4`, `sezione`, `stato`, `provincia`, `dxcc`, `prefisso-wpx`, `provincia-pa`. Ambiti: `per-banda`, `per-log`, `per-banda-modo` |
| | `moltiplicatore_potenza` | Field Day: `{ "<=5W-batteria" = 5, "<=100W" = 2, ">100W" = 1 }` |
| | `bonus` | elenco di voci da spuntare (Field Day) |
| `[punteggio]` | `formula` | `punti*molt`, `punti`, `km*quadrati`, `(punti*molt_potenza)+bonus` |
| `[categorie]` | elenco | nome, operatore, trasmettitori, potenza massima, bande, cambi di banda per ora, limiti di tempo, overlay |
| `[restrizioni]` | `avvio_umano` | `true` (ARRL, WW Digi): niente CQ ripetuto all'infinito senza intervento |
| | `fox_hound`, `multi_flusso` | `vietato` |
| | `assistenza` | `ammessa`, `vietata` |
| | `autospot` | `ogni-10-min` |
| | `cambi_banda_ora` | numero |
| `[export]` | `formato` | `cabrillo`, `edi`, `adif` |
| | `cabrillo_contest` | `WW-DIGI`, `ARRL-DIGI`, `ARRL-VHF-JUN`, `CQ-VHF`, `ARRL-FD` |
| | `cabrillo_modo` | `DG` |
| | `colonne_scambio` | ordine dei campi nella riga `QSO:` |
| | `frequenza` | `khz` o `banda` (VHF: `50`, `144`, `1.2G`) |
| | `edi_tname`, `edi_sezioni`, `edi_modo` (7), `adif_contest_id` | per EDI e ADIF |
| | `invio_url`, `scadenza` | `"7 giorni"`, `"primo-mercoledi-dopo 23:59:59"` |

### 4.3 Esempio di file (WW Digi)

```toml
[contest]
id = "ww-digi"
nome = "World Wide Digi DX Contest"
sponsor = "WWROF"
edizione = 2026
url_regole = "https://ww-digi.com/rules/"
verificato_il = "2026-11-08"

[calendario]
regola = "ultimo-weekend-completo:agosto"
inizio = "sab 12:00:00"
fine = "dom 11:59:59"

[bande]
ammesse = ["160m", "80m", "40m", "20m", "15m", "10m"]

[modi]
ammessi = ["FT8", "FT4"]
stesso_modo_per_doppi = true

[messaggi]
attivita = "ww-digi"
parola_cq = "WW"
chiusura = "RR73"
settantatre_finale = "consigliato"

[scambio]
inviato = ["grid4"]
ricevuto = ["grid4"]

[doppi]
chiave = "banda"

[punti]
tipo = "distanza"
distanza = { base = 1, passo_km = 3000, arrotonda = "giu", riferimento = "centro-grid4" }

[[moltiplicatori]]
tipo = "campo2"
ambito = "per-banda"

[punteggio]
formula = "punti*molt"

[restrizioni]
avvio_umano = true
fox_hound = "vietato"
multi_flusso = "vietato"

[export]
formato = "cabrillo"
cabrillo_contest = "WW-DIGI"
cabrillo_modo = "DG"
colonne_scambio = ["grid4"]
frequenza = "khz"
scadenza = "2026-09-01T23:59:59Z"
invio_url = "https://ww-digi.com/logcheck/"
```

## 5. Architettura nel core

### 5.1 Nuovo crate `contest`

Nome indicativo: `core/crates/contest`.

- **`regole`**: lettura e validazione dei TOML (campi sconosciuti → errore chiaro in italiano), calcolo delle date dal calendario, liste.
- **`punteggio`**: calcolo per QSO di punti, doppio sì/no, quali moltiplicatori nuovi apre; totali per banda e totali generali; ricalcolo completo quando si cambiano regole o categoria. Funzioni pure, facili da provare con i casi delle regole (5541 km = 2 punti in WW Digi; 1565 km = 5 in ARRL Digi; IARU: stesso quadrato = 50).
- **`valore`**: per ogni stazione decodificata, quanto vale chiamarla adesso: doppio, punti, quanti moltiplicatori nuovi, per quali bande è ancora utile. Usato dai colori e dal suggeritore.
- **`tempo`**: stato del contest (prima / in corso / pausa / finito), ore operative usate, pause valide, ore restanti, cambi di banda nell'ora, finestre «6 ore» IARU.
- **`rate`**: rate degli ultimi 10 QSO, 60 minuti e per ora; proiezione del punteggio finale (vedi 7.6).
- **`export`**: Cabrillo 3.0, EDI REG1TEST (una per banda, intestazione con i totali `CQSOs/CQSOP/CWWLs/CToSc/CODXC`, righe `ERROR` e `D`), ADIF con `CONTEST_ID`, `STX/SRX`, `STX_STRING/SRX_STRING`, `CLASS`, `ARRL_SECT`, `STATE` (si riusa `logbook::adif::contest_fields`); controllo del file prima dell'invio.

### 5.2 Sequenza contest nel motore (`qso-native`)

- Il sequenziatore riceve un'**attività** (`standard`, `na-vhf`, `arrl-digi`, `ww-digi`, `eu-vhf`, `field-day`, `roundup`, `q65-pileup`) e lo scambio da inviare.
- Messaggi Tx1-Tx5 generati come `genStdMsgs` di WSJT-X 3.0.1 (`widgets/mainwindow.cpp` 9480-9640):

  | Attività | Tx1 | Tx2 (risposta) | Tx3 (`R` + scambio) | Tx4 | CQ |
  |---|---|---|---|---|---|
  | NA VHF / ARRL Digi | `DX MY GRID4` | `DX MY GRID4` | `DX MY R GRID4` | `RRR` o `RR73` | `CQ TEST` |
  | WW Digi | idem | idem | idem | `RR73` | `CQ WW` |
  | Field Day | `DX MY` | `DX MY 1D DX` | `DX MY R 1D DX` | `RR73` | `CQ FD` |
  | Roundup (formato RU) | `DX MY` | `DX MY 559 0013` | `DX MY R 559 0013` | `RR73`; in FT4 `TU; ...` | `CQ RU` |
  | EU VHF | `DX MY GRID4` | `<DX> <MY> 570123 GRID6` | `<DX> <MY> R 570123 GRID6` | `RR73` | `CQ TEST` |

- **Seriale**: contatore persistente nel log del contest (per log o per banda), assegnato al primo Tx2/Tx3 del QSO e mai riusato. Un QSO non completato lascia un buco: in EDI resta come riga `ERROR`, come chiede la specifica.
- **Scambio ricevuto**: `features::contest_exchange` va spostato nel core e generalizzato con i campi delle regole (griglia, classe/sezione, rapporto+stato/seriale, rapporto+seriale+locatore a 6) e la validazione sulle liste.
- **Chiusura e log**: si mette a log quando si riceve RRR/RR73/73 o quando si manda RR73/73 (regola WSJT-X). Se il partner ripete il Tx3 dopo il nostro RR73, l'RR73 si rimanda **da solo** (è il tasto F4 alternativo di WSJT-X) e il QSO resta segnato «incerto» finché non arriva un segnale di chiusura o il partner passa ad altro.
- **«73 di cortesia»** configurabile dalle regole (`settantatre_finale`) per abbassare i NIL.
- **Chi non fa il contest**: se a un CQ di contest risponde qualcuno con il rapporto in dB invece della griglia, Aurora lo segnala (per WW Digi e ARRL Digi lo scambio non vale). L'operatore può rispondere con il Tx3 di contest, che spesso lo fa passare allo scambio giusto, oppure lasciar perdere.
- **Controlli prima del PTT**:
  - nominativo composto o non standard → avviso che i messaggi di contest non lo ammettono (solo `/P` e `/R`);
  - messaggio non rappresentabile → niente PTT (come fa WSJT-X 3.2.0-rc1).
- **Ordine di risposta**: in contest il «chi chiamare dopo» usa il `valore` del crate `contest` (punti + moltiplicatori), non solo SNR/distanza; i doppi vanno in fondo, come in JTDX_contest.
- **Avvio umano**: con `avvio_umano = true` il CQ automatico si ferma dopo N minuti senza intervento (DigiRite usa 5 minuti), e la risposta automatica ai chiamanti si può accendere solo come «conferma con un clic».
- **Prima di toccare la sequenza** si leggono `docs/logiche-jtdx.md` e i sorgenti di WSJT-X, come di regola.

### 5.3 Log del contest

- Tabelle nuove nel database del `logbook`:
  - `contest_sessione`: contest, edizione, categoria, nominativo, locatore, operatori, potenza, inizio e fine;
  - `contest_qso`: riferimento al QSO, banda, modo, frequenza, scambio inviato e ricevuto, seriale, punti, moltiplicatori aperti, stato `valido/doppio/incerto/errore`, trasmettitore (0/1), operatore.
- **Ogni QSO va anche nel log normale** di Aurora (ADIF, LoTW, QRZ, Wavelog come oggi), con `CONTEST_ID` e gli scambi. Il log del contest è una vista in più, non un secondo programma.
- **Modifica a mano** di un QSO del contest (correggere uno scambio copiato male): segnata nel log. Le regole IARU vietano le correzioni a posteriori «da database o registrazioni», quindi Aurora non propone mai di completare uno scambio da altre fonti.
- **Multi-op in rete**: più postazioni Aurora condividono la sessione attraverso `remote-api`, con i doppi e i moltiplicatori sincronizzati, come la rete di N1MM (fase finale).

### 5.4 Integrazione con N1MM e altri

- Si mantiene il pacchetto `contactinfo` già fatto e si aggiungono gli scambi del contest, così chi vuole N1MM come logger lo usa.
- Va verificato che il ponte UDP di Aurora parli il protocollo di WSJT-X abbastanza bene da far funzionare la «WSJT-X Decode List» di N1MM (decodifiche, Reply, HighlightCallsign), per chi vuole i colori di N1MM. Il multicast `239.x` evita la lotta per la porta 2237.
- Non serve: con la pagina Contest Aurora non ha bisogno di N1MM. È solo una porta aperta.

### 5.5 API e interfaccia

- `remote-api`: stato del contest (punteggio, rate, ore), valore di un nominativo, log del contest, export, elenco delle regole.
- Eventi sul bus: `ContestQso`, `ContestAvviso` (doppio, moltiplicatore nuovo, pausa, scadenza, violazione), `ContestStato`.

## 6. La pagina «Contest»

Prima il prototipo visibile sulla pagina vera; si pubblica solo dopo il sì del proprietario. Testi in 6 lingue.

### 6.1 Barra in alto (sempre visibile)

- Contest e categoria scelti (menu), stato (`tra 3 g 4 h` / `in corso` / `pausa` / `finito`), tempo restante e ore operative usate (per esempio «18 h 20 di 24»).
- Numeri grandi: **punteggio**, QSO, punti, moltiplicatori; **rate** 10 QSO / 60 min; **proiezione** a fine contest.
- Spie delle regole: banda ammessa, modo ammesso, sottobanda giusta, cambi di banda nell'ora, avvio umano, Fox/Hound spento.

### 6.2 Colonna sinistra: decodifiche

Le decodifiche di Aurora con i colori del contest (stessa grafica della finestra principale, pulsantini piccoli):

- grigio barrato = doppio;
- normale = punti;
- rosso = moltiplicatore nuovo;
- verde = doppio moltiplicatore;
- giallo = stazione che non fa il contest (manda rapporti invece della griglia).

Accanto a ogni nominativo: punti del QSO, moltiplicatori che apre e un segno «chiamami» per chi ha appena chiuso con qualcun altro. Clic = chiama; doppio clic = chiama e trasmetti.

### 6.3 Colonna centrale: QSO in corso

- I messaggi Tx1-Tx5 dell'attività, lo scambio inviato (con il seriale) e lo scambio ricevuto, validato in diretta: una sezione sconosciuta compare in rosso.
- Coda «chi chiamare dopo» ordinata per valore, con il motivo («nuovo campo JO su 20 m», «4 punti»).
- Tasti rapidi: reinvia RR73, manda 73, salta, segna incerto.

### 6.4 Colonna destra: moltiplicatori

- Mappa per banda dei campi (WW Digi, FT Challenge) o dei quadrati (VHF) lavorati e mancanti, con sopra le stazioni sentite adesso e i dati di PSK Reporter. Per Field Day e PACCdigi: tabella di sezioni o province.
- Riepilogo per banda: QSO, punti, moltiplicatori, rate dell'ultima ora, ultimo QSO.

### 6.5 Sotto

- Log del contest: tabella con ricerca, modifica, stato (valido / doppio / incerto); i QSO incerti in evidenza.
- Grafico del rate (per ora e per banda), stesso stile del grafico del ritmo della pagina Fox.
- Avvisi: «doppio ti chiama», «moltiplicatore nuovo su 15 m», «pausa valida dopo 60 min», «mancano 40 min alle 24 ore», «8 cambi di banda in quest'ora», «scadenza invio fra 2 giorni».

### 6.6 Schede laterali

- **Regole**: la scheda leggibile del contest (sezione 3) con il link ufficiale e la data di verifica.
- **Calendario**: i contest dei prossimi mesi, conto alla rovescia, «preparami»: imposta attività, bande, sottobande, categoria, locatore e parola del CQ, e fa un controllo prima della gara (orologio sincronizzato, CAT, nominativo compatibile con i messaggi di contest).
- **Esporta e invia**: Cabrillo/EDI/ADIF con anteprima, controllo del file, link alla pagina di upload dello sponsor e promemoria della scadenza. Niente invio automatico.
- **Analisi**: dopo il contest, rate nel tempo, moltiplicatori per ora, bande migliori, elenco dei QSO incerti (rischio NIL), confronto con l'edizione precedente.
- **Allenamento**: il simulatore (sezione 7.5).

### 6.7 Telefono

Una scheda compatta con punteggio, rate, prossimo moltiplicatore e avvisi, nella pagina telefono che Aurora ha già.

## 7. Funzioni che nessun altro ha

1. **Tutto in un programma**: sequenza, doppi, moltiplicatori, punteggio, rate, Cabrillo/EDI e controllo delle regole senza N1MM, senza UDP e senza porte da configurare. Elimina alla radice le lamentele 5, 7 e 8.
2. **Valore di ogni decodifica e suggerimento del moltiplicatore più utile**: non solo «nuovo» o «doppio», ma quanto vale adesso (punti + moltiplicatori), su quali bande quella stazione è ancora utile, e una coda «chi chiamare dopo» che lo usa. JTDX_contest mostra i punti; nessuno unisce punti, moltiplicatori e probabilità di riuscita (SNR, quanti la chiamano, se sta per chiudere).
3. **Mappa dei moltiplicatori mancanti con la propagazione**: per banda, campi e quadrati non lavorati sovrapposti a chi si sente adesso (decodifiche e PSK Reporter), con «su 15 m adesso ci sono 3 campi nuovi».
4. **Consiglio di banda e modo**: in base a rate, moltiplicatori in vista e affollamento suggerisce FT8 o FT4 e la banda, come raccomanda W0YK. Tiene conto dei cambi di banda ammessi nell'ora.
5. **Guardia anti-NIL**:
   - reinvio automatico di RR73 quando il partner ripete il Tx3;
   - 73 di cortesia configurabile;
   - QSO «incerti» segnati e mostrati a fine gara;
   - avviso quando il partner manda uno scambio non valido (rapporto invece della griglia).
6. **Simulatore di contest**: `engine-sim` già c'è. Si genera un pile-up realistico (chiamanti con SNR, tempi, errori, chi non fa il contest, doppi, chi non riceve l'RR73) per allenarsi senza trasmettere, con punteggio e rate. È un MorseRunner per FT8/FT4, e nessun programma lo ha.
7. **Controllo delle regole e degli orari in diretta**:
   - contest non ancora iniziato o finito;
   - ore operative e pause valide (ARRL Digi 24 su 30 con pause di 60 minuti, FT Challenge pause di 30, IARU 6 ore);
   - cambi di banda per ora;
   - banda o modo non ammessi, fuori sottobanda;
   - Fox/Hound e multi-flusso bloccati quando il contest li vieta;
   - avvio umano rispettato;
   - autospot IARU non più di uno ogni 10 minuti.
8. **Punteggio previsto**: proiezione a fine gara dal rate recente e dalla curva delle edizioni precedenti, con «obiettivo» impostabile e «ti servono 42 QSO/ora per arrivarci».
9. **Preparazione guidata del contest**: dal calendario un clic imposta tutto e fa i controlli (orologio, CAT, nominativo compatibile con i messaggi di contest, locatore, categoria). Prende i problemi del primo WW Digi (frequenza assente, radio che torna su 14074) e quelli dei nominativi composti.
10. **Export verificato prima dell'invio**: Cabrillo e EDI controllati contro le regole (intestazione `CONTEST` giusta, modo `DG` e non `RY`, griglia presente, `LOCATION`, sezioni EDI, righe `ERROR` e `D`), con l'elenco degli errori prima dell'upload.
11. **VHF completo**: Q65/MSK144/FT8 con NA VHF, CQ WW VHF e IARU MGM; punti in km dal vivo; direzione d'antenna per ogni stazione decodificata; EDI per banda. Nessun programma digitale lo fa senza un logger esterno.
12. **Attività mensili e piccoli contest senza «contest mode»**: per l'ARI FT8 Activity, l'FT Challenge, il DARC FT4 e l'RSGB FT4 (messaggi standard) Aurora conta punti e moltiplicatori dal log normale, filtra la tornata e prepara l'ADIF o il Cabrillo. Si fa il contest senza cambiare il modo di operare, e l'ARI vieta proprio il «Contest Mode».
13. **Fox/Hound e contest**: le regole lette (ARRL, WW Digi, DARC, ARI) **vietano** Fox/Hound e multi-flusso. Aurora lo spiega e blocca la pagina Fox mentre un contest è attivo. Per le attività che non sono contest (spedizioni, eventi speciali) la pagina Fox resta com'è. Non si propone un «Fox in contest» perché farebbe squalificare l'utente.
14. **Multi-op in rete**: più postazioni Aurora con un log comune e doppi sincronizzati, senza N1MM (fase finale).

## 8. Fasi di sviluppo

Stime in giornate di lavoro. Le prove dal vivo le fa il proprietario; da parte del codice solo `cargo check` e `tsc`. Ogni fase chiusa va tolta da `docs/tasks.md`.

| Fase | Contenuto | Stima |
|---|---|---|
| 0 | Prototipo grafico della pagina (dati finti) sulla pagina vera, con la filigrana PROVA LOCALE: si va avanti dopo il sì | 2 |
| 1 | Crate `contest`: lettura e validazione dei TOML, liste, calendario; i 12 file di regole della sezione 3 | 3 |
| 2 | Punteggio, doppi, moltiplicatori e valore di ogni stazione; prove con gli esempi delle regole | 3 |
| 3 | Log del contest nel `logbook`, seriali, stato incerto, legame con il log normale e l'ADIF | 3 |
| 4 | Sequenza contest nel motore per tutte le attività, scambio ricevuto, reinvio RR73, 73 di cortesia, controlli prima del PTT, avvio umano | 5 |
| 5 | Pagina Contest vera: barra, decodifiche colorate, QSO in corso, log, riepilogo per banda, 6 lingue | 5 |
| 6 | Export Cabrillo / EDI / ADIF con controllo; scheda «Esporta e invia» | 2 |
| 7 | Rate, proiezione, avvisi, controllo di regole e orari, preparazione guidata dal calendario | 3 |
| 8 | Mappa dei moltiplicatori, consiglio di banda e modo, coda «chi chiamare dopo» | 4 |
| 9 | Ponte N1MM (scambi in `contactinfo`, compatibilità con la Decode List) | 2 |
| 10 | Simulatore di contest per l'allenamento | 5 |
| 11 | Analisi del dopo-gara, scheda per il telefono | 2 |
| 12 | Multi-op in rete | 5 |
| | **Totale indicativo** | **circa 44 giornate** |

### 8.1 Ordine consigliato e prima consegna utile

1. Fasi 0-6 danno un primo rilascio utile prima del prossimo grande contest (FT Challenge di dicembre o ARRL Digi di giugno), con WW Digi, ARRL Digi, FT Challenge, attività ARI e IARU MGM.
2. Poi le fasi 7-8 (le funzioni che fanno la differenza).
3. Poi le fasi 9-12 secondo le richieste.

### 8.2 Prima di cominciare

- Rileggere i PDF ufficiali dell'anno: le schede segnate «da ricontrollare» sono Field Day (bande), FT Challenge (date), ARRL Digi (nome Cabrillo), PACCdigi (scambio).
- Leggere il sorgente GPL di JTDX_contest (GitHub `ce3tsk/jtdx_contest`) per come calcola il valore delle decodifiche e per i Cabrillo degli sponsor; per WSJT-X i sorgenti sono già in `D:\build\wsjtx-src`.
- Decidere se le regole aggiornate si scaricano dal sito di Aurora (come gli altri dati) o arrivano solo con le versioni.

## 9. Formati dei file da mandare

### 9.1 Cabrillo 3.0

- **Intestazione**:
  - prima riga `START-OF-LOG: 3.0`, poi `CONTEST:`, `CALLSIGN:`;
  - `CATEGORY-OPERATOR` (SINGLE-OP, MULTI-OP, CHECKLOG), `CATEGORY-BAND` (ALL, 160M...), `CATEGORY-MODE` (DIGI), `CATEGORY-POWER` (HIGH, LOW, QRP), `CATEGORY-TRANSMITTER` (ONE, TWO, UNLIMITED), `CATEGORY-STATION`, `CATEGORY-ASSISTED`, `CATEGORY-OVERLAY`;
  - `LOCATION` (sezione ARRL/RAC o `DX`), `GRID-LOCATOR`, `CLAIMED-SCORE`, `CLUB`, `OPERATORS` (obbligatorio per i multi), `NAME`, `EMAIL`, `CREATED-BY`, `SOAPBOX`;
  - ultima riga `END-OF-LOG:`.
- **Righe QSO**: `QSO: freq mo data ora nominativo scambio-inviato nominativo scambio-ricevuto [t]`. Frequenza in kHz in HF (o la banda: 7000) e come banda in VHF (50, 144, 222, 432, 902, 1.2G...); modo `DG` per FT8/FT4/Q65/MSK144; data `aaaa-mm-gg`; ora `hhmm` UTC; `t` = trasmettitore (0/1) per i Multi-Two.
- Esempio WW Digi: `QSO: 14074 DG 2026-08-29 1215 IU3ABC IO91 K1ABC FN42 0`.

### 9.2 EDI (REG1TEST, VHF IARU Regione 1)

- `[REG1TEST;1]`, poi le chiavi `TName`, `TDate` (aaaammgg;aaaammgg), `PCall`, `PWWLo` (fino a 6 caratteri), `PExch`, `PSect` (SO, MO, SO-MGM, MO-MGM, SO-6H...), `PBand` (50 MHz, 70 MHz, 145 MHz, 435 MHz, 1,3 GHz...), `PClub`, `RName`, `RCall`, `RHBBS` (email), `MOpe1/2`, `STXEq`, `SPowe`, `SRXEq`, `SAnte`, `SAntH`, i totali dichiarati `CQSOs`, `CQSOP`, `CWWLs`, `CWWLB`, `CExcs`, `CExcB`, `CDXCs`, `CDXCB`, `CToSc` e la migliore distanza `CODXC`.
- `[Remarks]` con le note.
- `[QSORecords;N]`, poi una riga per QSO con 15 campi separati da `;`:
  data `aammgg`; ora `hhmm`; nominativo; codice modo (MGM = 7); rapporto inviato; numero inviato; rapporto ricevuto; numero ricevuto; scambio ricevuto; locatore ricevuto; punti; nuovo scambio `N`; nuovo locatore `N`; nuovo DXCC `N`; doppio `D`.
- I QSO non validi restano come righe `ERROR` con il numero inviato, per tenere la sequenza; un file per banda; nome del file `NOMINATIVO.edi`.

### 9.3 ADIF

`CONTEST_ID`, `STX`/`SRX` (seriali), `STX_STRING`/`SRX_STRING`, `CLASS` e `ARRL_SECT` (Field Day), `STATE`/`VE_PROV` (Roundup), `GRIDSQUARE` e `MY_GRIDSQUARE`. Basta per l'ARI FT8 Activity, che accetta l'ADIF intero.

## 10. Fonti principali

Programmi:

- WSJT-X, guida (messaggi di contest): https://sourceforge.net/p/wsjt/wsjtx/ci/master/tree/doc/user_guide/en/make-qso.adoc?format=raw (copia locale in `D:\build\wsjtx-src\estratto\wsjtx-3.0.1\doc\user_guide\en\make-qso.adoc`)
- WSJT-X, Release Notes: https://wsjt.sourceforge.io/Release_Notes.txt
- N1MM+, WSJT-X Decode List: https://n1mmwp.hamdocs.com/manual-windows/wsjt-x-decode-list-window/
- N1MM+, contest digitali: https://n1mmwp.hamdocs.com/manual-supported/contests-setup/setup-digital-contests/
- DigiRite: https://writelog.com/digirite · https://rttycontesting.com/tutorials/writelog3/digirite/
- JTDX_contest (CE3TSK): https://github.com/ce3tsk/jtdx_contest/blob/main/README.md
- MSHV: https://cq.sk/pl/mshv/ · https://groups.io/g/MSHV/topic/new_ft8_ft4_contest/32256857
- DXLog.net: https://dxlog.net/docs/index.php/Additional_Information
- GridTracker: https://docs.gridtracker.org/latest/Introduction/What-is-GridTracker.html
- Decodium 4: https://github.com/iu8lmc/Decodium-4.0-Core-Shannon/wiki
- Log4OM: https://forum.log4om.com/viewtopic.php?p=39273
- Contest University 2025 (test W1UE dei decoder): https://contestuniversity.com/wp-content/uploads/2025/10/7.-Mini-CTU-Digital-Contesting2.pdf
- W0YK, «Tips to Improve Your FT8-4 Contest Performance» (CTU, maggio 2026): https://www.contestuniversity.com/wp-content/uploads/2026/05/Pg.-36-Tips-to-Improve-Your-FT8-4-Contest-Performance.pdf

Regole:

- WW Digi: https://ww-digi.com/rules/ · https://ww-digi.com/cabrillo.htm · https://ww-digi.com/operating.htm
- ARRL International Digital Contest: https://contests.arrl.org/ContestRules/Digital-Rules.pdf
- ARRL Field Day: https://www.arrl.org/field-day-rules
- ARRL VHF: https://contests.arrl.org/ContestRules/JanJunSep-VHF-Rules.pdf · https://www.arrl.org/june-vhf
- ARRL RTTY Roundup (solo RTTY): https://www.arrl.org/rtty-roundup
- ARRL, divieto dei contatti automatici: https://www.arrl.org/news/arrl-contest-and-dxcc-rules-now-prohibit-automated-contacts
- International FT Challenge: https://www.rttycontesting.com/ft-challenge/rules/ · FT Roundup (chiuso): https://www.rttycontesting.com/ft-roundup/rules/
- CQ WW VHF 2026: https://cqww-vhf.com/rules/2026_cq_ww_vhf_rules.pdf · https://cqww-vhf.com/ft8.htm
- IARU R1 VHF Managers Handbook V10.03: https://www.iaru-r1.org/wp-content/uploads/2026/02/VHF_Handbook_V10_03_final.pdf
- EDI, modello UKSMG: https://uksmg.org/contest/edi-file-format.php
- ARI FT8 Activity Contest 2026: https://contest.ari.it/rules/regolamentoiacft8_2026.pdf
- ARI, contest VHF senza modi digitali: https://contest.ari.it/rules/regoltrofeiarivhfeup2026.pdf · https://contest.ari.it/rules/regolcontestsezionivhfeup2026.pdf · https://contest.ari.it/rules/regolamentoalpeadriavhf2026.pdf
- DARC FT4: https://www.darc.de/der-club/referate/conteste/ft4-contest/darc-ft4-contest-english-version/darc-ft4-rules
- PACCdigi: https://veron.nl/vereniging/commissies-en-werkgroepen/traffic-bureau/hf-contesten/paccdigi-rules/
- Winter Field Day: https://winterfieldday.org/downloads/2025-rules.pdf
- RSGB, notizie contest: https://rsgb.org/main/blog/category/news/gb2rs/contest-news

Lamentele degli utenti:

- Soapbox ARRL Digi 2022: https://contests.arrl.org/dig/soaps/2022
- N1MM perde QSO (luglio 2026): https://community.worldradioleague.com/t/n1mm-integration-misses-some-qs/45918
- Cabrillo rifiutato per griglia mancante: https://community.worldradioleague.com/t/pass-contest-exchange-while-streaming-from-wsjt-x/45645
- JTAlert e porta UDP: https://hamapps.groups.io/g/Support/topic/jtalert_not_communicating/86382878
- RR73 e NIL: https://ve3vn.blogspot.com/2020/04/dxing-ethics-and-rr73.html
- Il ritmo dei contest FT: https://va7st.ca/2020/10/ft8-ft4-from-a-contesters-perspective/
- Primo WW Digi: https://www.amateurradio.com/my-first-digi-contest-was-very-frustrating/
- Field Day e doppi (wsjt-devel): https://sourceforge.net/p/wsjt/mailman/message/59202163
