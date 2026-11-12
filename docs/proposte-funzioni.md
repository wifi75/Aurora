# Proposte di funzioni per CW, RTTY, PSK e contest (2026-11-12)

Questo elenco confronta Aurora con fldigi, MMTTY, 2Tone, MRP40, DigiPan, MultiPSK e N1MM. Ogni riga dice chi ha la funzione, a cosa serve, a che punto è Aurora, quanto lavoro costa e quanto conviene farla.
Le priorità sono una proposta: la scelta è tua. Dei programmi chiusi ho usato solo manuali e pagine pubbliche. Gli impegni sono stime, non misure.
Nominativi d'esempio: IU3ABC e K1ABC. Sigle: «progetto RTTY» = `docs/progetto-rtty-oltre.md`, «progetto PSK-CW» = `docs/progetto-psk-cw-oltre.md`, «analisi» = `docs/analisi-psk-cw.md`.

## Cosa Aurora ha già (per non proporlo due volte)
- PSK31 e PSK63, RTTY e CW con ricezione e trasmissione, cascata, AFC, squelch, Snap, Seek, scansione, 20 memorie, macro a tasti, log interno con ADIF, rotore, DX Cluster con spot sulla pagina, split «up N», S/N e IMD in ricezione, riquadro QRZ.
- Ricevitore RTTY rifatto (fldigi + MMTTY), CW con FIR e AGC, PSK con filtro adattato (lavoro di `analisi-psk-cw.md`).
- Contest con regole in file, doppi, moltiplicatori e Cabrillo, ma solo per i modi a periodo (FT8, FT4, MSK144, Q65).
- Nel menu Modo restano in grigio «presto»: QPSK31, FSK31, MFSK, Olivia, Contestia, MT63, Throb, Hellschreiber, Packet, Pactor, Amtor.

Legenda stato: **assente** = non c'è e nessun progetto lo prevede · **parziale** = c'è una parte · **progetto X** = scritto in un documento ma non fatto.

---

## CW

| ID | Funzione | Chi la ha | Cosa serve all'utente | Stato in Aurora | Impegno | Priorità |
|---|---|---|---|---|---|---|
| CW-01 | Decodifica di tutti i segnali CW della banda audio insieme | fldigi (browser CW), MRP40 (uno solo) | Vedere chi chiama in tutta la banda senza girare la manopola | progetto PSK-CW («Più segnali insieme») | grande | alta |
| CW-02 | Decodificatore bayesiano a traliccio (statistica invece di soglia) | — | 3-5 dB in più sui segnali deboli, meno caratteri sbagliati | progetto PSK-CW (C1) | grande | alta |
| CW-03 | Mappa di banda: un cerchio per ogni decodificatore, etichetta col nominativo sulla cascata, «CQ»/«DE», «QRL?», «599», triangolo verde se la stazione sta trasmettendo, la etichetta segue chi cambia frequenza | DigiPan (nominativi sul segnale) | Leggere i nominativi direttamente sulla cascata e cliccare per sintonizzare | assente (nominativi solo per FT8 e spot del cluster) | medio | alta |
| CW-04 | Estrazione del nominativo dal testo: appare due volte = in grassetto; riconosce CQ, DE, TEST per capire se chiama o risponde (running / S&P) | — | Sapere chi sta chiamando CQ e chi sta rispondendo | progetto PSK-CW (nominativi con DXCC e «già lavorato») | medio | alta |
| CW-05 | Convalida dei nominativi a livelli (minimo, normale, aggressivo, paranoico): lista personale, Master.dta/SCP, schema tipico/insolito/sospetto, blocchi di prefissi non assegnati dall'ITU; colori diversi | N1MM (check parziale) | Non vedere nominativi inventati dal rumore | parziale (`dxcc` verifica i prefissi; SCP facoltativo nel progetto PSK-CW, I2) | medio | alta |
| CW-06 | Elenco dei nominativi verificati: ordine per frequenza, ora o nominativo; scadenza dopo 10 minuti; doppio clic sintonizza; salvato e ripreso al riavvio | — | Una lista breve di chi c'è sulla banda adesso | assente | piccolo | media |
| CW-07 | Colori nel testo ricevuto: nominativo rosso, RST rosa, parole chiave blu; caratteri incerti in grigio; simbolo per le pause lunghe | — | Capire a colpo d'occhio cosa è sicuro e cosa no | progetto PSK-CW (I1, caratteri con confidenza); i colori assenti | medio | media |
| CW-08 | Filtro CW regolabile da 20 a 700 Hz con mouse (rotella o trascinando il rettangolo sulla cascata) o Ctrl+su/giù; MRP40 usa 30 Hz che segue la velocità | MRP40, fldigi | Stringere il filtro sul DX nel pile-up | parziale (larghezza in Impostazioni, non dalla cascata) | piccolo | media |
| CW-09 | Noise blanker contro i disturbi a impulsi (scariche, linee elettriche) | — | Decodificare con QRN | progetto PSK-CW (C6) | piccolo | media |
| CW-10 | AGC che non «respira»: segue alto e basso della dinamica, si sentono i segnali deboli fra i punti di uno forte | MRP40 («Smart AGC») | Ricezione stabile con segnali molto diversi | parziale (AGC del CW già rifatto) | piccolo | bassa |
| CW-11 | Anti-click: toglie i «click» dei tasti dalla cascata | — | Cascata pulita dove i segnali forti sporcano i vicini | assente | piccolo | media |
| CW-12 | AFC del tono (±50 Hz), tolleranza al chirp, pulsante «vai al massimo» che porta il filtro sul segnale più forte | MRP40 (Smart AFC, GoToMax), progetto PSK-CW | Il segnale non si perde se la radio o l'altra stazione derivano; segnalazione #46 (ON7KEC) | progetto PSK-CW (C7); il difetto è segnalato | piccolo | alta |
| CW-13 | Velocità e «pugno»: durate di punto, linea e pause stimate in linea (tasto a mano, bug, peso, Farnsworth); Tx da 5 a 60 WPM con peso e rapporto regolabili | MRP40, fldigi | Leggere anche i tasti a mano e trasmettere con peso e spaziatura scelti | parziale (velocità a coppie fatta; modello del pugno nel progetto, C2) | medio | media |
| CW-14 | Manipolazione vera della radio: linea DTR/RTS della seriale, comando CAT (Icom, Yaesu, Kenwood), WinKeyer, Arduino/Nano | fldigi, MRP40 (RS232, WinKeyer), N1MM | Trasmettere in CW vero (modo CW della radio, filtro stretto, potenza piena) e non un tono in SSB | assente (Aurora manda solo un tono audio) | medio | alta |
| CW-15 | Tasto e paddle: keyer iambic A/B, ingresso da tasto dritto, decodifica di ciò che si sta mandando a mano | fldigi (nanoIO, WinKeyer), N1MM | Operare «a mano» dal programma | assente | grande | bassa |
| CW-16 | Break-in e sidetone locale: ritardo PTT, QSK semplice, tono di controllo sulle cuffie del PC | fldigi, MRP40, N1MM | Ascoltarsi e passare rapido fra Tx e Rx | assente | medio | bassa |
| CW-17 | Macro da contest per CW: numero progressivo, numeri accorciati (5NN, 1T), ripeti, aumenta/diminuisci WPM, «invio manda il messaggio giusto» | N1MM, fldigi | Fare un contest CW senza altri programmi | parziale (macro a tasti; contest CW assente) | medio | alta |
| CW-18 | Memorie di testo e beacon: 10 testi pronti, ripetizione con pausa regolabile | MRP40 | Chiamare CQ o fare un faro senza stare davanti | parziale (macro e CQ automatico) | piccolo | bassa |
| CW-19 | Modello del testo con grammatica dei nominativi e seconda passata all'indietro sui 10-30 secondi in memoria | progetto PSK-CW | Le prime parole di una stazione non vanno più perse; meno errori sui nominativi | progetto PSK-CW (C3 e C4) | medio | media |
| CW-20 | Banco di 2-3 filtri adattati alla velocità, scelta per verosimiglianza | progetto PSK-CW | 1-2 dB sopra 35 WPM e sotto 12 WPM | progetto PSK-CW (C5) | piccolo | bassa |
| CW-21 | Due stazioni sullo stesso tono (sperimentale) | progetto PSK-CW | Separare due chiamanti sovrapposti | progetto PSK-CW (C8) | grande | bassa |
| CW-22 | Decodificare solo nel segmento CW del piano di banda; numero massimo di decodificatori adattivo o fisso | — | Meno CPU e meno rumore stampato | parziale (il piano di banda c'è, non è usato per questo) | piccolo | bassa |
| CW-23 | Sintonia veloce da tastiera: freccia su/giù = prossima stazione, PgSu/PgGiù = salto, tasto «Tune» per scrivere la frequenza; panoramica della cascata trascinando la scala | — | Cambiare stazione senza toccare la radio | parziale (Seek e scansione) | piccolo | bassa |
| CW-24 | Ingresso I/Q largo (48-192 kHz) con bilanciamento I/Q automatico e calibrazione della frequenza (WWV) | — | Decodifica di un'intera banda con una radio SDR | assente (Aurora ascolta 3 kHz di audio; TCI c'è per il controllo) | grande | media |
| CW-25 | Allenamento: pile-up simulato per esercitarsi | dal sito) | Allenarsi per i contest | assente | grande | bassa |

## RTTY

| ID | Funzione | Chi la ha | Cosa serve all'utente | Stato in Aurora | Impegno | Priorità |
|---|---|---|---|---|---|---|
| RT-01 | Decodifica probabilistica: probabilità d'errore per bit e per carattere, carattere intero a massima verosimiglianza | — | 1-2 dB alla soglia; base di tutto il resto | progetto RTTY (B3-B4) | grande | alta |
| RT-02 | Squelch sulla probabilità e caratteri incerti mostrati in chiaro (grigi) | 2Tone (squelch da 1, 5 dB con memoria di 4 caratteri) | Niente caratteri inventati nel rumore (segnalazione #27) e si vede cosa non è sicuro | parziale (squelch su S/N fatto; probabilità nel progetto RTTY, B6) | medio | alta |
| RT-03 | Profili di decodifica: Standard, Fluttered, Multi-path, AA6YQ FIR, FFT, 23 Hz (MMTTY); Flat, Flutter, Selective, Spread (2Tone); fusione per carattere | MMTTY, 2Tone | Scegliere o lasciar scegliere al programma il metodo giusto per fading e multipath | parziale (alcuni ricevitori scelti in Impostazioni; fusione nel progetto RTTY, B5) | medio | media |
| RT-04 | Soppressore d'impulsi e limitatore sempre acceso | MMTTY | RTTY leggibile con QRN e AGC che pompa | progetto RTTY (B1) | piccolo | media |
| RT-05 | Notch e filtro LMS, filtro a due picchi AA6YQ, passa-banda con AFC | MMTTY | Togliere un portante vicino, stringere sui due toni | assente | medio | media |
| RT-06 | Ripetizioni combinate (Repeat Match): le parole ripetute del contest si sommano | 2Tone | +3 dB con due copie; ottimo per nominativi ripetuti | progetto RTTY (B7) | medio | media |
| RT-07 | Registro LTRS/FIGS come stato nascosto e Contest Mode (alfabeto ridotto, scambio noto) | 2Tone | Meno errori su numeri e nominativi durante la gara | progetto RTTY (B8) | medio | media |
| RT-08 | Più segnali RTTY insieme (coppie di picchi alla distanza dello shift), una catena per ogni segnale | — | Tutti i chiamanti RTTY in banda | progetto RTTY (B9) | grande | alta |
| RT-09 | Diversity: due ingressi combinati | 2Tone (Selective, in banda), progetto RTTY | Molto con fading selettivo | progetto RTTY (B10); serve un secondo ingresso | grande | bassa |
| RT-10 | FSK vera in Tx: seriale (TXD/UART con tempi giusti), FSK della radio via CAT, scelta AFSK/FSK | MMTTY, fldigi, N1MM | Segnale RTTY pulito come le radio vere, senza toni audio | assente (solo AFSK) | medio | alta |
| RT-11 | Diddle in trasmissione (LTR o BLK), attesa del diddle, USOS in Tx, «Rev» solo in ricezione, stop 1/1,5/2 | MMTTY, fldigi | Comportamento come RTTY contest standard | parziale (UOS in ricezione; il resto da verificare) | piccolo | media |
| RT-12 | Scope XY (due ellissi a croce) e indicatore di sintonia | MMTTY | Sintonia fine sicura a occhio, soprattutto in contest | assente | medio | alta |
| RT-13 | AFC: più algoritmi (per HAM, per FSK), shift fisso 170 o a 220/240 Hz | MMTTY | Seguire stazioni con shift e deriva diversi | parziale (AFC c'è) | piccolo | bassa |
| RT-14 | Baud e codice: 45,45 / 50 / 75 / 100 baud, shift 85-850 Hz, ASCII 7/8 bit, SITOR | fldigi, MultiPSK | Ricevere anche RTTY commerciale e vecchi | parziale (baud e shift scelti dal menu) | piccolo | bassa |
| RT-15 | Pulsante «NET» (segui la frequenza di un net) e riga di sintonia stile MMTTY | MMTTY | Stare su un net senza toccare nulla | assente | piccolo | bassa |

## PSK

| ID | Funzione | Chi la ha | Cosa serve all'utente | Stato in Aurora | Impegno | Priorità |
|---|---|---|---|---|---|---|
| PS-01 | Browser PSK: decodifica di tutti i segnali visibili insieme, con il testo di ognuno | DigiPan (40-50 stazioni), fldigi (viewpsk), MultiPSK, WinWarbler | Vedere chi è in aria, cliccare e rispondere | progetto PSK-CW (browser PSK) | grande | alta |
| PS-02 | Avvisi «CQ» a colori nel browser, nominativo ricavato dal segnale, clic: mette il nominativo nel QSO | DigiPan 2.0 | Trovare subito chi chiama | assente (dipende da PS-01) | piccolo | alta |
| PS-03 | QPSK31 e QPSK63 con correzione d'errore | fldigi, MultiPSK | Compatibilità con chi usa QPSK; più robusto in rumore | assente (voce «presto» nel menu) | medio | media |
| PS-04 | PSK125, 250, 500, 1000 e PSKR (con FEC) | fldigi, MultiPSK | Veloce con segnali buoni, robusto in fading | assente | medio | media |
| PS-05 | 8PSK con correzione d'errore | fldigi | Trasferire testo più veloce | assente | grande | bassa |
| PS-06 | FSK31 e QFSK | — | Ricevere chi usa PSK a frequenza | assente (voce «presto») | medio | bassa |
| PS-07 | Bit soft e Viterbi sulla varicode, con squelch sulla probabilità e caratteri incerti | progetto PSK-CW | 1-2 dB e zero caratteri spuri sul rumore | progetto PSK-CW (P1, P3, P5) | grande | media |
| PS-08 | Riferimento di fase su più simboli, simboli colpiti da QRN cancellati | progetto PSK-CW | 0,5 dB in rumore, molto di più con QRN e fading | progetto PSK-CW (P2, P4) | medio | bassa |
| PS-09 | Aggancio rapido dal preambolo (riga a 2f) e Snap debole | progetto PSK-CW | Non perdere i primi caratteri | progetto PSK-CW (P6) | medio | media |
| PS-10 | Cancellazione del vicino forte (decodifica, rimodula, sottrae) + LMS | MMVARI, progetto PSK-CW | Leggere un segnale debole accanto a uno forte | progetto PSK-CW (P7) | grande | bassa |
| PS-11 | Diversità a due ingressi | progetto PSK-CW | 3 dB in AWGN, 5-8 dB con Rayleigh | progetto PSK-CW (P8) | grande | bassa |
| PS-12 | Correzione dell'orologio della scheda audio in ppm | MMVARI, MMTTY (ClockAdj) | 0,2-0,5 dB con schede imprecise | progetto PSK-CW (P9) | piccolo | bassa |
| PS-13 | Fase vettoriale (phase scope), doppio canale, misura IMD in diretta | DigiPan | Capire se il proprio segnale è pulito e come è il segnale ricevuto | parziale (IMD letta; scope assente) | piccolo | media |
| PS-14 | PSK10, PSKAM10/31/50, PSKFEC31 | MultiPSK | Casi rari di compatibilità | assente | piccolo | bassa |

## Comune ai modi a tastiera

| ID | Funzione | Chi la ha | Cosa serve all'utente | Stato in Aurora | Impegno | Priorità |
|---|---|---|---|---|---|---|
| CM-01 | Registratore e riproduttore dell'audio: WAV con tag (nominativo, ora UTC, frequenza), riproduzione in ciclo, scelta del punto, rinomina | MMTTY, fldigi | Riascoltare un QSO, provare i decodificatori, mandare una registrazione a chi aiuta nelle segnalazioni | assente nei modi a tastiera | medio | alta |
| CM-02 | Pacchetto di registrazioni di prova e confronto con altri programmi | progetto PSK-CW, progetto RTTY | Misurare se Aurora decodifica meglio, non indovinare | progetto PSK-CW e RTTY | medio | alta |
| CM-03 | Olivia e Contestia (varie larghezze e toni) | fldigi, MultiPSK | Segnali molto deboli e disturbati sulle bande basse | assente (voce «presto») | grande | alta |
| CM-04 | MFSK (16, 32, 64...) e immagini | fldigi | Collegamenti con rumore e fading | assente | medio | media |
| CM-05 | MT63 | fldigi, MultiPSK | Compatibilità con la comunità MT63 e ARES | assente | grande | media |
| CM-06 | DominoEX e Thor | fldigi | Alternativa all'MFSK con lenta variazione di tono | assente | medio | bassa |
| CM-07 | Hellschreiber, Feld-Hell e varianti | fldigi | Modo storico e curioso, ricevibile quasi a occhio | assente | piccolo | media |
| CM-08 | Throb e ThrobX | fldigi | Curiosità, compatibilità | assente | piccolo | bassa |
| CM-09 | FSQ, IFKP e Scamp | fldigi | Chat a bassa potenza con richiamo | assente | medio | bassa |
| CM-10 | Packet AX.25 (300 baud), Pactor-I, AMTOR/SITOR | DigiPan (Pactor-I, solo ricezione), MultiPSK | Ricevere servizi esistenti | assente | medio | bassa |
| CM-11 | NAVTEX (SITOR-B) con filtro per stazione e tipo di messaggio | fldigi, MultiPSK | Avvisi marittimi ricevuti con la radio | assente | medio | media |
| CM-12 | SYNOP: bollettini meteo decodificati e mostrati su mappa | fldigi (flsynop), MultiPSK | Dati meteo dalle stazioni radio | assente | medio | bassa |
| CM-13 | WEFAX (cartine meteo) | fldigi | Cartine meteo ricevute con la stessa pagina dell'SSTV | assente | medio | media |
| CM-14 | RSID: riconoscimento automatico del modo da un breve segnale di servizio | fldigi | Aprirsi sul modo giusto senza indovinare | assente | medio | media |
| CM-15 | Log integrato con ricerca, ADIF, controllo di doppio, QRZ | DigiPan, fldigi | Tutto in un programma | parziale (log interno, ADIF, riquadro QRZ già presenti) | piccolo | bassa |
| CM-16 | Caratteri con confidenza, correzioni degli ultimi caratteri e modello del testo (I1, I2, I3) | progetto PSK-CW | Base per le righe CW-07, CW-19, RT-02, PS-07 | progetto PSK-CW (I1-I3) | medio | media |
| CM-17 | Orologio scheda audio e calibrazione della frequenza con un segnale noto | MMTTY (ClockAdj) | Frequenza e tempo corretti nelle misure | parziale (sincronizzazione dell'ora del PC fatta) | piccolo | bassa |
| CM-18 | Cascata: luminosità, contrasto e gamma regolabili, velocità di scorrimento | DigiPan | Cascata leggibile sul proprio monitor | da verificare (il contrasto automatico esiste per l'FT8) | piccolo | bassa |
| CM-19 | Trasmettere un file di testo (bollettini, esercizi) | MMTTY (SendFile), fldigi | Inviare un bollettino senza ribatterlo | da verificare | piccolo | bassa |
| CM-20 | Pagina PSK·RTTY·CW da telefono: leggere il testo, macro, trasmettere | (nessuno degli altri, solo Aurora) | Usare la stazione dal divano | da verificare | medio | media |
| CM-21 | Registro automatico del testo ricevuto su file con ora e frequenza | fldigi, MMTTY | Rileggere ciò che è passato | parziale (la scansione scrive ora e frequenza nel testo) | piccolo | bassa |
| CM-22 | DTMF, orologio WWV e misura di frequenza (FMT) | fldigi | Strumenti di servizio | assente | piccolo | bassa |

## Contest e cluster

| ID | Funzione | Chi la ha | Cosa serve all'utente | Stato in Aurora | Impegno | Priorità |
|---|---|---|---|---|---|---|
| CC-01 | Contest in CW, RTTY e PSK nella pagina Contest: scambio, doppi per banda e modo, moltiplicatori, punteggio, rate | N1MM | Un solo programma per CQ WW, WPX, ARRL, RTTY Roundup... | parziale (motore delle regole e liste RTTY Roundup già presenti; modi a tastiera assenti) | grande | alta |
| CC-02 | Editor dei contest a moduli con file TOML leggibile | — | Aggiungere o correggere un contest senza toccare il codice | parziale (regole in file; editor assente) | medio | media |
| CC-03 | Controllo parziale del nominativo (Master.dta/SCP) mentre lo si scrive | N1MM | Ripulire nominativi sbagliati ricevuti a metà | assente | medio | alta |
| CC-04 | Risposta chiara sul nominativo: doppio (con banda e ora), nuovo, nuovo moltiplicatore, non ammesso | N1MM | Non perdere punti con doppi | parziale (per i modi a periodo) | medio | alta |
| CC-05 | «Invio manda il messaggio giusto» (ESM) | N1MM | Gara veloce con una sola mano | assente | medio | alta |
| CC-06 | Cabrillo per CW, RTTY e PSK (CATEGORY-MODE, operatore, bande) | N1MM | Consegnare il log senza convertirlo | parziale (Cabrillo già per FT8) | piccolo | alta |
| CC-07 | Numeri progressivi, cut number (5NN, T per 0) e scambio per modo | N1MM | Scambi come si usano in CW | parziale | piccolo | media |
| CC-08 | Macro di contest che vincono su quelle del modo e su quelle comuni | — | Messaggi giusti per ogni gara | parziale (macro di modo; macro per contest assenti) | medio | media |
| CC-09 | Bandmap: spot sulla cascata della pagina CW/RTTY/PSK, clic sullo spot porta la radio e mette il nominativo nel QSO | N1MM | Cercare moltiplicatori senza guardare un'altra finestra | parziale (spot del cluster nella pagina; mappa di banda con etichette assente) | medio | media |
| CC-10 | Sorgente RBN (e HamAlert) con filtri per modo, S/N, WPM e banda | RBN, HamAlert | Vedere dove ci si sente in CW e RTTY | parziale (client telnet già nel crate `dx-cluster`; formato RBN da verificare) | piccolo | media |
| CC-11 | Server telnet locale in sola lettura (porta 7300, password, solo CQ, comando «SH/DX», annuncio del clic per il log) | — | Mandare gli spot decodificati a N1MM, Log4OM o a un altro PC | assente | medio | media |
| CC-12 | Spot automatici dai decodificatori verso un cluster o la rete RBN (suffisso «-#» per gli spot automatici) | — | Dare qualcosa alla comunità; per esempio se IU3ABC ascolta K1ABC | progetto PSK-CW (spot locali; invio in rete solo su richiesta) | medio | bassa |
| CC-13 | Invio a PSK Reporter di ciò che si decodifica in PSK, CW e RTTY | fldigi, MultiPSK | Gli altri vedono dove si ascolta | da verificare (c'è per FT8) | medio | media |
| CC-14 | Integrazione col log esterno via UDP (N1MM, Log4OM) anche per i modi a tastiera | N1MM, Log4OM | Un solo log vero | parziale (già per FT8) | piccolo | media |
| CC-15 | SO2R: due radio, una in ascolto mentre l'altra trasmette | N1MM | Gare di alto livello | assente | grande | bassa |

---

## Prima le cose che servono di più
Ordine pensato per un'utente che fa soprattutto contest e DX. Le righe tra parentesi sono quelle della tabella.

1. **Manipolazione CW vera (CW-14)**: senza, il CW di Aurora è un tono in SSB e molte radio lo tagliano; è la prima cosa che chiede chi usa il CW.
2. **FSK vera per l'RTTY (RT-10)**: lo standard dei contest RTTY; senza si resta in AFSK.
3. **Contest CW, RTTY e PSK (CC-01, con CC-04, CC-06)**: il motore dei contest c'è già, manca solo portarlo sui modi a tastiera.
4. **Decodifica di tutti i segnali CW con mappa di banda (CW-01 e CW-03)**: è la funzione più richiesta per il CW; il progetto è già scritto.
5. **Browser PSK con avvisi CQ (PS-01 e PS-02)**: il DigiPan di oggi; anche questo è già nel progetto.
6. **Registratore e riproduttore dell'audio (CM-01)**: costa poco, serve per le segnalazioni e per provare i decodificatori.
7. **AFC del CW e «vai al massimo» (CW-12)**: risolve la segnalazione #46 di un utente.
8. **Convalida dei nominativi e controllo parziale (CW-05 e CC-03)**: toglie i nominativi inventati, utile alla decodifica di tutta la banda, ai contest e agli spot.
9. **Decodifica probabilistica con squelch e caratteri incerti (RT-01 e RT-02)**: risolve «caratteri col solo rumore» (#27) e porta il vantaggio vero sugli altri programmi.
10. **Olivia e Contestia (CM-03)**: i modi per segnali molto deboli che gli altri programmi hanno e che Aurora ha solo come voce grigia.

Altre candidate subito dopo: scope XY (RT-12), ESM e macro di contest CW (CC-05, CW-17), decodificatore bayesiano (CW-02), server telnet locale (CC-11), QPSK31 e PSK125 (PS-03, PS-04), pagina da telefono (CM-20).

## Numero di proposte per sezione
CW 25 · RTTY 15 · PSK 14 · Comune ai modi a tastiera 22 · Contest e cluster 15 · totale 91.

## Fonti
File del progetto:
- `docs/progetto-rtty-oltre.md`, `docs/progetto-psk-cw-oltre.md`, `docs/analisi-psk-cw.md`, `docs/report-cloudflare-2026-11-12.md` (segnalazioni #27 e #46), `docs/mancante.md`, `docs/piano-contest.md`, `TODO.md`, `CHANGELOG.md`.
- Per cosa Aurora ha già: `app/src/pages/DigiPage.tsx`, `app/src/components/digi/*`, `core/crates/mode-digi/src/*`, `core/crates/contest/src/*`, `core/crates/dx-cluster/src/*`.
- fldigi 4.2.13: `terze-parti/fldigi/src/` (cartelle `cw` con winkeyer e nanoIO, `rtty/fsk.cxx`, `psk/viewpsk.cxx`, `olivia`, `contestia`, `mfsk`, `mt63`, `dominoex`, `thor`, `throb`, `feld`, `navtex`, `wefax`, `fsq`, `ifkp`, `rsid`, `spot`, `dxcluster`, `logbook`).
- MMTTY: `terze-parti/mmtty/` (finestre `Option.dfm`, `Main.dfm`, `TxdDlg.dfm`; `HISTORY.TXT` è in giapponese e non è stato letto in dettaglio).

Pagine pubbliche:
- 2Tone: <https://www.sp2pby.pl/pdf/2Tone%202102a.pdf>, <http://ok2pad.webz.cz/logger32/g3yyd_2tone.pdf>
- MRP40: <https://www.polar-electric.com/Morse/MRP40-EN/features.html>
- DigiPan: <https://www.digipan.net/>
- MultiPSK (F6CTE): <https://f1ult.pages-perso.free.fr/DIGIMODES/MULTIPSK/digimodesF6CTE_en.htm>, <https://wiki.radioreference.com/index.php/HF_and_LF_Modes_used_by_MultiPSK>
- N1MM Logger+: confronti già raccolti in `docs/piano-contest.md`; funzioni ESM, Cabrillo, bandmap e SO2R dalla conoscenza generale del programma, da ricontrollare sul manuale prima di decidere.
- Cosa non ho potuto verificare: righe segnate «da verificare» (stato in Aurora non letto fino in fondo) e le funzioni di MMTTY e N1MM non presenti nei file locali.
