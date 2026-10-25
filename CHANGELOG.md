# Changelog

## Non rilasciato

## 0.11.11 - 2026-10-25
- **Avviso di versione nuova** come piccola icona in testata, prima della lingua (un clic apre Installa ora, Note e download, Piu' tardi), al posto della barra larga.
- Base della scheda Audio come in JTDX (canali d'ingresso e d'uscita, cambio a caldo dell'uscita, salvataggio dei file audio nel formato di JTDX): pezzi del core pronti e provati, collegamento alla scheda nella versione successiva.
- **Importazione delle impostazioni da JTDX chiave per chiave**: le voci mancanti nel file di Aurora prendono il valore di `JTDX.ini` (altrimenti quello di fabbrica di JTDX); le scelte gia' salvate non si toccano. Corregge la chiamata fermata da «Ferma la Tx se il DX risponde a un altro», che Aurora aveva acceso mentre in JTDX era spento.
- **Schede Notifiche, Sequenza e Generali complete come in JTDX**: evidenziazioni «Nuovo» (zona CQ, zona ITU, paese, griglia, prefisso, nominativo, per banda e per modo), «gia' lavorato» (barrato, sottolineato, colorato o nascosto), avvisi e suoni per tipo (4 suoni, volume, prova, non suonare mentre trasmetti), scenari brevi con il numero di volte, priorita' ai CQ e alla distanza maggiore, interruttori uguali a quelli del pannello QSO.
- **Gia' collegati barrati**: nell'elenco dei messaggi (Attivita' di banda e CQ e novita') la stazione gia' nel log sulla stessa banda e nello stesso modo ha il messaggio barrato, come in JTDX; scheda Notifiche con colore, barra, sottolinea e «non mostrare». «Gia' lavorato» e' cio' che resta quando niente e' nuovo secondo le evidenziazioni accese (zona CQ, ITU, paese, griglia, prefisso, nominativo, «per banda» e «per modo»).
- **Schede Sequenza, Generali e Notifiche come in JTDX** con il motore proprio: contatori degli scenari brevi, ferma il Tx se il DX risponde a un altro, CQ rigorosa, priorita' ai CQ e alla distanza, interruttori del pannello QSO anche in Impostazioni (stesso stato); filtro del log, periodo di monitoraggio Tx, cambi di frequenza Tx durante la trasmissione, non monitorare all'avvio, riga vuota tra i periodi, azimut; evidenziazioni di «nuovo» a caldo, inverti colori, marcatori, sfondo ciano/giallo, avvisi sonori con suono a scelta, prova e silenzio mentre si trasmette. Voci non realizzabili spente con il motivo (`docs/impostazioni-motore-proprio.md`).
- Le etichette di novita' seguono l'ordine di JTDX (zona CQ prima del paese) e i valori di fabbrica di JTDX (paese e nominativo accesi, per banda e per modo).

## 0.11.10 - 2026-10-25
- **QSO gia' collegati barrati** (stessa banda e stesso modo, dal log interno, `wsjtx_log.adi` e Log4OM2) nelle liste Attivita' di banda e CQ e novita', con le scelte di JTDX: evidenzia, sottolinea o nascondi (Impostazioni -> Notifiche).
- **Schede Notifiche, Sequenza e Generali come in JTDX** (in corso di completamento: le voci non ancora disponibili sono elencate con il motivo).
- **Scheda DXCC compatta**: senza DIG, senza i numeri per banda e modo e senza i pulsanti Paese/Totali; si popola solo con un nominativo scelto e prende meno spazio, cosi' il log ne ha di piu'.

## 0.11.9 - 2026-10-25
- **Scheda DXCC sotto il log**: nello stesso pannello, il log di Log4OM2 in alto e la scheda DXCC sotto; il pannello del log e' piu' largo e la scheda si adatta (etichette e matrice in piccolo).
- **Logiche di QSO verificate nel programma di riferimento**: registrazione al primo 73/RR73, contatori, pausa per stazione, passaggio a Tx6/CQ, banda di guardia, watchdog, scelta di chi chiamare, avvio tardivo (finestra 13,64 s in FT8 e 6,04 s in FT4, si trasmette la coda) e ritrasmissione. Esc spegne solo Abilita Tx; «Invia» del testo libero non accende Abilita Tx; AutoSeq si spegne con un testo libero diverso da 73; `split` e' l'inverso di `pbTxLock`. Tx in zona libera (spenta di base: `AURORA_FREE_TX=1`).

## 0.11.8 - 2026-10-25
- **Scheda DXCC nella pagina Stazione**: pannello con le sei etichette (NEW ONE, NEW BAND, NEW MODE, NEW GRID, GRID BAND, GRID MODE) e la matrice dei paesi lavorati per banda (160...4 m e U) con righe FT8, FT4 e DIG; dati dal log interno, da `wsjtx_log.adi` e da Log4OM2. Logica verificata nel programma di riferimento (`docs/scheda-dxcc.md`). Esportazione e importazione ADIF dalla pagina Statistiche.
- **Tutto cio' che imposti resta impostato al riavvio**: interruttori (AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, RRR, periodo Tx, AutoTX, Split), frequenze Tx e Rx, modo FT8/FT4, livelli Pwr per banda e **Abilita Tx** (scelta del proprietario) si salvano in `controls.json`. Monitora riparte sempre acceso.
- Logiche di banda satura e di QSO riverificate con il programma di riferimento (in corso).
- **Scheda DXCC per banda e modo**: nella pagina Statistiche (e in piccolo nel blocco Stazione DX) le sei etichette NEW ONE, NEW BAND, NEW MODE, NEW GRID, GRID BAND, GRID MODE per la stazione DX e la matrice dei paesi lavorati (righe FT8, FT4, DIG; colonne 160...4 m e U) con il totale, i colori verde/arancio/grigio, il tema chiaro e scuro e i suggerimenti in sei lingue. Dati dal log interno, dai file ADIF e da Log4OM2 (sola lettura), uniti senza doppioni; si aggiorna a ogni QSO. Logica in `docs/scheda-dxcc.md` (verificata nell'eseguibile di JTDX e in Decodium). Nuova rotta `GET /api/v1/dxcc-matrix`.
- **Log interno: esportazione e importazione ADIF**: `GET /api/v1/log/export.adi` (formato di JTDX, intervallo di date) e `POST /api/v1/log/import` (senza doppioni, per ripartire dal proprio storico, per esempio `wsjtx_log.adi`), con i pulsanti nella pagina Statistiche. Il log interno (`logbook.db`) passa alla versione 2: ora di inizio e di fine e stato delle conferme.

## 0.11.7 - 2026-10-25
- **Cursore Pwr come in JTDX**: potenza massima in alto (0 dB), minima in basso; verificato nel programma e nel sorgente (`attenuazione = (450 - valore) / 10`). Prima era invertito.
- **Il livello Pwr agisce anche durante la trasmissione**: il guadagno lo applica il dispositivo audio mentre suona, con una rampa senza click, e il cursore lo aggiorna subito (anche al cambio banda con la memoria per banda).
- **Tx/Rx Split acceso si vede acceso** nel pannello Comandi (prima restava senza evidenza). Con Split acceso le barre Tx e Rx della cascata sono indipendenti; spento, Tx segue Rx.

## 0.11.6 - 2026-10-25
- **Flusso WSJT-X verso GridTracker e Log4OM2 con il motore proprio**: Aurora manda gli stessi messaggi UDP di JTDX (Heartbeat, Stato con il nominativo in chiamata, Decodifiche, Clear, QSO registrato, ADIF, Close), copiati dal programma con il reverse; GridTracker li gira a Log4OM2. I comandi di ritorno (Reply, Halt, testo libero, frequenza) si accettano solo se acceso `AURORA_UDP_ACCEPT=1`.
- **Impostazioni di Aurora** (scheda Segnalazioni come in JTDX, salvate in `settings-motore.json`): server UDP primario e secondario, invio ADIF, richieste UDP; le voci non ancora disponibili sono visibili e spente, con il motivo.
- **ALL.TXT** scritto dal motore proprio nel formato di JTDX (mensile, decodifiche, righe `Transmitting`, `QSO logged`); `AURORA_ALL_TXT=0` lo spegne. Studio con REA di JTDX e Decodium in `docs/analisi-all-txt-e-aggiornamenti.md`.
- **Hamlib incluso nell'installatore** (`rigctld` e DLL nella cartella `hamlib`), condivisione della radio con altri programmi (porta 4532), porte virtuali VSPE riconosciute e guida passo per passo sulle porte COM nella Guida in sei lingue.
- **STOP TX e tasto Esc immediati**: l'arresto di audio e PTT non aspetta piu' il ciclo del motore, che puo' essere occupato a decodificare.
- **Mappa**: i pulsanti +, - e casa funzionano (zoom senza animazione).
- **S-meter stimato dall'audio** per le radio senza misuratori (per esempio con OmniRig).
- **S-meter stimato dall'audio** (radio senza misuratori, per esempio con OmniRig): il motore proprio misura il livello RMS dell'audio a 12 kHz a finestre di 150 ms e lo converte in punti S (6 dB per punto, oltre S9 si contano i dB). Il riferimento e' relativo: il pavimento di rumore e' il 10° percentile degli ultimi 30 s (inseguito lentamente), S9 sta a 40 dB sopra (il rumore cade a circa S2); attacco rapido e rilascio di circa 400 ms. Si ferma in trasmissione e con Monitora spento. Taratura: `AURORA_SMETER_S9_DB` (dB tra pavimento e S9, da 20 a 70, di base 40). Nello stato la stima esce in `meters` con `estimated: true` e solo `s_points`/`s_over_db`; con i misuratori veri della radio si usano quelli. Il display mostra `~S6` e il testo alternativo dice «S-meter stimato dall'audio» (6 lingue).

## 0.11.5 - 2026-10-25
- **QSO verso Log4OM2 con il motore proprio**: Aurora manda il record ADIF di ogni QSO registrato all'indirizzo del secondo server UDP di JTDX (`UDP2Server`, `UDP2ServerPort`, letto da `JTDX.ini`); prima, senza JTDX, non arrivava piu' niente.
- **FT4 nel motore proprio**: il pulsante FT4 porta davvero il motore in FT4 (periodi da 7,5 s, decodifica e trasmissione).
- **Salta Tx1 e periodo di risposta**: la risposta va nel periodo opposto a quello del DX e Salta Tx1 agisce subito; il pulsante del periodo Tx vale.
- **Comandi della pagina**: Gen.Msgs, Split, copie di frequenza Rx/Tx, Prepara e Invia del messaggio libero (13 caratteri come JTDX).
- **Cambio di banda come JTDX** (verificato con il programma): ferma la trasmissione, azzera la Stazione DX e svuota Attivita' di banda e CQ e novita'; vale anche quando la banda la cambia la radio.
- **Misuratori della radio** letti da Hamlib (S-meter, potenza, ROS, ALC) e passati al display.
- **Tabella dell'attivita' di banda** compatta e tema chiaro leggibile sulle righe che mi chiamano.

## 0.11.4 - 2026-10-25
- **Nuovo logo**: la cascata con le tracce dei segnali FT8, in tutte le icone di Windows, nella favicon e nell'app per telefono.
- **Logo animato in alto a destra**: la cascata scorre (si ferma con il movimento ridotto del sistema).
- **Schermata di avvio**: logo grande, nome, motto nelle sei lingue e versione, con un breve saluto a otto toni (se il sistema permette l'audio); si chiude da sola o con un clic.

## 0.11.3 - 2026-10-25
- **Trasmissione del motore proprio**: doppio clic su un messaggio, CQ, messaggio libero, frequenza Tx e STOP TX arrivano al motore di Aurora (sequenza dei sei messaggi con AutoSeq, audio in uscita, PTT sulla linea RTS o DTR di una porta COM, via OmniRig oppure VOX; uscita audio e PTT si leggono da `JTDX.ini` se c'e'). Il livello segue il cursore Pwr. Il QSO a fine sequenza finisce nel log.
- **Trasmissione sempre abilitata, nessuna opzione**: tolto `AURORA_ALLOW_TX` e il messaggio «Trasmissione chiusa». Restano STOP TX, i tetti e le automazioni spente di base.
- **Decodifica piu' sensibile**: sottrazione dei segnali decodificati, filtri e aiuto a priori (AP); sensibilita' sintetica dall'83% al 93%.
- **CAT per tutte le radio**: scheda Impostazioni -> Radio CAT con Hamlib (`rigctld`), elenco dei modelli, rilevamento della radio, profili con nome, riconnessione automatica e PTT che si spegne alla chiusura (`docs/cat-radio.md`).
- **Display della radio**: cinque stili (VFD, strumento analogico, TFT, LCD ambra, LED) a scelta nelle Impostazioni, piccolo in testata, alimentati dal CAT; STOP TX e gli altri controlli spostati accanto.
- **Lingua con le bandiere** e **tabella dell'attivita' di banda** con bandiere piu' grandi, distanza stretta a sinistra e etichette piu' larghe.
- **Niente vassoio di sistema**: la X chiude tutto come una normale applicazione.
- **Il cambio modo porta la radio sulla frequenza di quel modo** nella banda in uso, come nelle Impostazioni -> Frequenze.
- Analisi con REA di JTDX e Decodium in `docs/analisi-*.md`.

## 0.11.2 - 2026-10-25
- **Pulsanti del motore proprio**: con Aurora da sola, Monitora (parte **sempre acceso** all'avvio), Ferma, Hound, Modo SWL, Sinc, AGCc, Filtro e gli altri non cercano piu' JTDX: lo stato sta in Aurora e Monitora agisce sul decoder (spento = non si decodifica). La frequenza e il cambio banda passano dalla radio (OmniRig). Abilita Tx, AutoTX e FT4 restano chiusi finche' non sono collegati.

## 0.11.1 - 2026-10-25
- **Aurora funziona da sola, senza JTDX**: nel programma Windows il motore e' quello proprio (`AURORA_ENGINE=native`): ascolta l'ingresso audio (lo stesso della cascata), decodifica FT8 e pubblica messaggi, paese e novita' e lo stato della stazione (frequenza letta da OmniRig). Per ora **solo ricezione**: la trasmissione del motore proprio non c'e' ancora e i comandi di trasmissione restano chiusi. `AURORA_ENGINE=jtdx` rimette JTDX.
- **FT4 nel motore proprio** (crate `ft4`: modulazione, sincronismo, decoder con sottrazione; non ancora collegato all'ingresso audio).
- **Registratore dei periodi**: il motore proprio salva gli ultimi 400 periodi (circa 144 MB) in WAV a 12 kHz in `recordings` nella cartella dei dati, per misurarlo sulle stesse registrazioni (`AURORA_NATIVE_RECORD=0` lo spegne).
- **CAT con Hamlib per tutte le radio (fondamenta)**: modulo `rig-control::hamlib` che lancia `rigctld` come processo nascosto e ci parla via TCP (frequenza, modo, PTT, split), con elenco dei modelli (`rigctl -l`, FT-2000 compresa); provato con il modello dummy. Manca ancora la schermata Impostazioni → Radio e il collegamento alla scelta del backend.
- **Confronto dei modi con Decodium**: `docs/confronto-modi-decodium.md` (FT4, JT65, JT9, JT4, FST4, Q65, MSK144, WSPR, CW, RTTY, SSB, SSTV) con l'ordine di integrazione nel motore proprio.

## 0.11.0 - 2026-10-25
- **Automazioni in banda affollata**: pausa per stazione che scade (i tentativi falliti si dimenticano dopo 15 minuti, regolabile), priorita' a punteggio (novita' e segnale insieme) e rispetto dei CQ direzionali («CQ NA» solo dal Nord America, «CQ DX» solo da fuori dal proprio continente, dal nominativo letto da JTDX). Dalle analisi di Decodium (`docs/analisi-decodium-banda-satura.md`, `docs/analisi-decodium-decoder.md`).
- **Cascata: nero, guadagno, contrasto e velocita'**: quattro regolazioni sul bordo alto della cascata (si vedono passando il mouse, sempre sul telefono) come in Decodium; agiscono sul disegno e quindi su tutta la storia, e restano salvate.
- **Cascata piu' reattiva**: niente piu' media a 4 righe (mostrava 3,8 righe al secondo); ora scorre a 15 righe al secondo come Decodium, con il doppio di storia. La media si puo' rimettere con la chiave `aurora-wf-rows` (1-4).
- **Motore proprio, ricezione FT8 (in ombra)**: crate `ft8` (messaggi a 77 bit, CRC, LDPC con BP e OSD, sincronismo, decoder del periodo con sottrazione, modulazione GFSK) e crate `engine-native` (audio a 12 kHz, periodi UTC di 15 s, decodifica, confronto con JTDX). Decodifica in ombra accanto a JTDX, scrive `native-decodes.log` e `native-parity.log` nella cartella dei dati e racconta il confronto in `GET /api/v1/engine/native`. Acceso nel programma Windows (`AURORA_NATIVE_DECODE=0` lo spegne). Algoritmi di JTDX 2.2.159 e WSJT-X (GPL 3).
- **Analisi di Decodium** (sorgente pubblico e REA): CAT per tutte le radio (`docs/analisi-decodium-cat.md`), cascata (`docs/analisi-decodium-cascata.md`), banda satura (`docs/analisi-decodium-banda-satura.md`).

## 0.10.6 - 2026-10-24
- **Niente finestra di PowerShell**: cambiando banda (e nelle altre letture della radio) si apriva per un attimo una finestra nera di PowerShell; ora parte nascosta, come gia' faceva la sessione fissa. Vale anche per l'avvio dell'installatore dell'aggiornamento e per la correzione dell'ora.

## 0.10.5 - 2026-10-24
- **Aggiornamento automatico riparato**: l'installatore partiva e si chiudeva subito perche' la riga che chiude Aurora chiudeva anche i suoi figli, installatore compreso. Ora chiude solo il programma e Aurora lancia l'installatore staccato da se' (`cmd /C start`). Dalla 0.10.5 in poi l'aggiornamento da Aurora funziona; chi ha la 0.10.4 installa la 0.10.5 a mano una volta.
- **Licenza GPL-3.0 (solo)**: Aurora passa da «3 o successiva» a «3», perche' il motore FT8/FT4 proprio porta in Rust algoritmi di JTDX e WSJT-X (GPL 3). Manifest, README, installatore e script delle licenze allineati.

## 0.10.4 - 2026-10-24
- **Trasmissione aperta nel programma Windows**: `AURORA_ALLOW_TX=1` e' il predefinito (scelta del proprietario); `AURORA_ALLOW_TX=0` in `aurora.env` la chiude. STOP, tetti e automazioni spente di default restano.
- **Monitora acceso all'avvio**: appena il motore risponde (anche se parte dopo Aurora, fino a dieci minuti) l'ascolto si accende; `AURORA_MONITOR_ON_START=0` lo lascia com'e'.
- **Aurora riparte da dove l'hai lasciata**: ultima pagina aperta, ultima scheda delle impostazioni e dimensione/posizione della finestra (con controllo che non finisca fuori da un monitor staccato).
- **Barra degli aggiornamenti**: «Installa ora» compare anche quando la versione nuova e' arrivata a pagina gia' aperta; «Note e download» apre il browser di Windows invece di non fare nulla.

## 0.10.3 - 2026-10-24
- **Installazione forzata sopra una versione aperta**: l'installatore (e il disinstallatore) chiude Aurora da solo, anche se e' nascosta nel vassoio, e prosegue senza fermarsi con «Aurora e' in esecuzione!». Il core riparte al prossimo avvio. Vale dalla 0.10.3: chi ha una versione piu' vecchia deve chiudere Aurora a mano una volta (vassoio, Esci).

## 0.10.2 - 2026-10-24
- **Programma Windows senza associazione**: la finestra di Aurora sul PC entra da sola, senza «Associa questo dispositivo». Vale solo per chi arriva da questo stesso PC (mai da un proxy o da un tunnel); i telefoni e gli altri PC chiedono sempre il codice. Si spegne con `AURORA_TRUST_LOCAL=0` in `aurora.env`.

## 0.10.1 - 2026-10-24
- **Licenze**: `THIRD-PARTY-NOTICES.md` (avvisi di terze parti) e `docs/licenze-terze-parti.md` (tutte le dipendenze con la licenza, generato da `scripts/licenze-terze-parti.py`, controllo che siano compatibili con la GPL-3.0-or-later). Autore con nominativo (IU3WJQ) in README, pacchetti, installatore e piede dell'interfaccia.
- **Programma Windows con installatore** (`desktop/`, Tauri 2): Aurora si installa con un installatore personalizzato (logo, autore, licenza GPL, barra laterale e intestazione con il marchio, italiano e inglese, senza permessi di amministratore). Il core gira dentro il programma e la finestra mostra la sua pagina; chiudendola Aurora resta nel **vassoio** (apri, installa aggiornamento, cerca aggiornamenti, avvia con Windows, cartella dei dati, esci), parte con Windows e una seconda copia mostra la prima. Trova da sola JTDX, il suo log, `cty.dat`, Log4OM2 e l'ingresso audio; le impostazioni si danno in `%APPDATA%\Aurora\aurora.env`. Trasmissione sempre chiusa finche' non la si apre.
- **Aggiornamento automatico completo**: notifica di Windows quando esce una versione nuova, voce «Installa Aurora X» nel vassoio e «Installa ora» nella pagina; scarica il pacchetto e la firma, **installa solo con firma minisign valida** (chiave pubblica incorporata, privata fuori dal repository), mai in trasmissione; l'installatore riapre Aurora. `scripts/rilascio-windows.ps1` prepara installatore, firma e somme di controllo; guida in `docs/installer-windows.md`.
- **Mappa come GridTracker2**: rifatta con Leaflet. Terreno e confini veri (tessere Esri National Geographic), zona di notte calcolata dal Sole e aggiornata ogni minuto, equatore, tropici e circoli polari con il nome, quadrati lavorati in giallo, stazioni ricevute, linea gialla fino alla stazione scelta, stella sulla propria stazione e, passando il cursore, coordinate, distanza, direzione e locatore. «Sfondo reale» e «Giorno e notte» si accendono e spengono (e si ricordano); senza lo sfondo resta la mappa disegnata dal programma, che funziona anche senza internet. Le tessere vengono da internet: il sito vede quali zone guardi, non il tuo nominativo.
- **Pagina Propagazione da cruscotto**: tre quadranti luminosi (flusso solare, indice A, indice K) con il giudizio a colori, i dati del Sole in tessere, la matrice delle bande per giorno e notte con il giorno o la notte reali sul tuo QTH e la banda in uso evidenziati, e per chi ti ha ricevuto: statistiche, mappa con una linea sul percorso reale colorata per banda, barre delle stazioni per banda e tabella.
- **Comandi a JTDX molto piu' rapidi**: Abilita Tx, la scelta del messaggio Tx, il cambio modo, la frequenza Rx e il livello Pwr passano da una sessione PowerShell sempre aperta che ricorda per dieci minuti i controlli della finestra di JTDX. Misurato dal vivo: un comando scende da circa 1 secondo a circa 60 ms, la lettura periodica da circa 1 secondo a circa 460 ms. Due sessioni (comandi e letture) cosi' una lettura lenta non fa aspettare mai un comando; si riaprono da sole se si bloccano; si riscaldano all'avvio e ogni cinque minuti.
- **Priorita' tra piu' CQ nello stesso ciclo**: i messaggi di un ciclo si raccolgono per 350 ms e l'automazione risponde a **uno solo**, il migliore. Regola «Con piu' CQ nello stesso ciclo rispondi a»: la novita' piu' rara (prima la lista dei desiderati, poi paese nuovo, nuovo su banda, zona, griglia; a parita' il segnale piu' forte) oppure il segnale piu' forte. Il registro dice quale e' stato scelto e quali sono stati lasciati. Tetti e salvaguardie valgono per la risposta scelta.
- **Cambio banda: il modo della radio si rimette**: se la radio, cambiando banda, richiama il modo memorizzato per quella banda (per esempio USB al posto di USB dati), Aurora riscrive il modo di prima, solo se e' cambiato e se OmniRig lo permette per quella radio. Da provare con la radio.

## 0.10.0 - 2026-10-24
- **Telefono** (schermi fino a 640 px): la pagina Stazione diventa a schede (Attivita' di banda, CQ, Trasmissione, Comandi, Cascata, DX Cluster, Log), una alla volta a tutta larghezza e con la scelta ricordata; menu e testata compatti con scorrimento di lato; modo, banda, comandi e interruttori del QSO in griglie a colonne uguali con bersagli da 44 px; campi a 16 px (niente zoom); messaggi su due righe con il testo leggibile; STOP TX sopra la riga in fondo. Corretti anche i pannelli con elenco che si schiacciavano a zero su schermi stretti. Sopra i 640 px non cambia nulla.
- **Aggiornamento automatico, base**: il core scarica il pacchetto di installazione dalla release (anche da un repository privato col token), ne verifica la firma minisign e lo installa solo se valida; `POST /api/v1/update/install` (solo dal PC, mai in trasmissione) e pulsante «Installa ora» nella barra degli aggiornamenti, visibile solo quando il programma sa installare (`can_install`). Il guscio Windows che lo usa non c'e' ancora.
- **Automazione, filtro dei falsi decode**: prima di rispondere a un CQ si scartano i messaggi segnati «?» dal motore, i nominativi senza la forma di un nominativo e i segnali sotto -23 dB di stazioni mai sentite in sessione; il motivo compare nel registro delle decisioni. Non cambia l'elenco dei messaggi.
- **Automazione, risposta ai desiderati**: nuova regola «Rispondi anche ai CQ della lista dei desiderati» (spenta di default) che usa la stessa lista degli avvisi, con filtro di banda sulla frequenza della radio e tutte le salvaguardie (segnale minimo, tentativi, tetti, modo reale solo dal PC, trasmissione chiusa di default).
- **Servizi online a richiesta** (scheda Impostazioni > Servizi online, **spenti di default**): *PSK Reporter* (chi ha ricevuto il tuo nominativo nelle ultime 3 ore, con distanza dal tuo locatore e il piu' lontano) e *propagazione* da hamqsl.com (flusso solare, indici A e K, condizioni per gruppo di bande di giorno e di notte, con la banda in uso evidenziata). Nuova pagina Propagazione. Sono richieste di sola lettura e rare; ogni servizio dice cosa vede il sito; non si invia mai nulla di cio' che si riceve o trasmette.
- **Diagnosi automatica** nella pagina Stazione: avvisa quando l'orologio e' fuori oltre il 4 % del periodo (8 % = problema), l'ingresso audio e' saturo, muto o debole, manca il nominativo nel motore o non si decodifica nulla da 3 minuti; ognuno con il modo di sistemarlo. Sparisce quando e' tutto a posto.
- **Spot del cluster sulla cascata**: gli spot della banda in uso che cadono dentro la cascata (frequenza dello spot meno quella della radio) compaiono in basso con tacca e nominativo, su piu' righe se vicini, per dieci minuti; un clic porta la frequenza di ricezione (Rx) su quella dello spot. Solo ricezione, non trasmette.
- **Storico dei messaggi**: ogni messaggio ricevuto si ricorda (`data/history.sqlite`, 30 giorni, `AURORA_HISTORY_DAYS`) e si cerca nella nuova pagina Storico per nominativo o testo, banda, modo e periodo, con pagine da 100 e svuotamento su conferma (`GET/DELETE /api/v1/history`, `GET /api/v1/history/stats`). Non e' il log dei QSO e non vi finisce nulla.
- **Filtro «Nascondi i CQ di stazioni gia' collegate su questa banda»** nella scheda Filtri: gli altri messaggi di quelle stazioni restano visibili.
- **Livello RX**: indicatore del livello dell'audio in ingresso accanto al cursore Pwr (valore medio, picco, stato Muto/Basso/Giusto/Alto/Saturo) con avviso di saturazione dopo 3 secondi e di silenzio dopo 8; evento `audio_level` sul flusso. Idea dal confronto con Decodium, scritta da zero.
- **Volume di registrazione di Windows**: cursore e pulsante AUTO accanto all'indicatore Rx (`GET/PUT /api/v1/input-level`), sul dispositivo da cui viene la cascata, lo stesso di JTDX. AUTO scende subito se il segnale satura, sale piano se resta debole, non agisce in trasmissione e si spegne se muovi il cursore; la scelta si ricorda in `data/input-level.json`.
- **Licenza**: Aurora e' rilasciata con la GNU GPL versione 3 o successiva (`LICENSE`, `license` nei manifest, badge del README).
- Studio di Decodium 4.0 (GPL-3.0) e confronto delle funzioni che Aurora non ha: `docs/confronto-decodium.md`.
- **Aggiornamenti** (Impostazioni → Aggiornamenti): Aurora controlla ogni qualche ora l'ultima release sul repository GitHub (nuovo crate `updater`; repository e intervallo a scelta, anche le versioni di prova, token di sola lettura per un repository privato che non esce mai dal programma) e, se e' piu' nuova di quella in uso, mostra una barra in alto con note e link, la manda con un evento a tutte le pagine aperte e, con l'indirizzo ntfy, anche sul telefono; avvisa una volta sola per versione, anche dopo un riavvio. Rotte `GET/PUT /update` e `POST /update/check`. Non scarica ne' installa nulla: l'installazione automatica spetta al programma Windows (da fare).
- **Lista dei desiderati** (Impostazioni → Desiderati): nominativi, prefissi, paesi e griglie da collegare, con filtro di banda e attesa tra due avvisi. Il core guarda ogni messaggio decodificato e ogni spot del cluster (anche a pagina chiusa); alla prima apparizione una barra gialla in alto (con suono se gli avvisi sonori sono accesi, e notifica del browser se consentita), la riga evidenziata negli elenchi e, con un indirizzo **ntfy**, la notifica sul telefono. Rotte `GET/PUT /wishlist` e `POST /wishlist/test`; nuovo crate `wishlist`.
- **Mappa** (`#/mappa`): planisfero con i quadrati locatore gia' lavorati nel log di Log4OM2 (piu' QSO, piu' colore; filtro per banda), le stazioni che si ricevono ora dal loro locatore (CQ e risposte, colori delle etichette del motore) e la tua posizione. Rotella o + e − per ingrandire, si trascina. Nuova rotta `GET /map/worked` (aggregata nel database: 17.000 QSO diventano 1.500 righe). Il flusso dei dati ora sta nell'applicazione e non nella pagina della stazione: cambiando pagina i messaggi continuano ad arrivare.
- Interruttori del QSO allineati alla finestra di JTDX: pulsante AutoSeq con il livello scritto dal motore («AutoSeq 3») e casella RRR (senza spunta il Tx4 e' RR73, con la spunta RRR).
- Impostazioni → Generali: casella «Cambiando banda o modo svuota l'attivita' di banda» (accesa di base). Vale per i cambi fatti da Aurora, subito, e per quelli fatti nel motore; spenta l'elenco resta, e «Cancella» lo svuota a mano.
- Riavvio di Aurora: i dati arrivano subito. Prima lo stato di JTDX (frequenza, modo, DX) compariva solo a fine periodo, fino a 13-15 secondi dopo, e nel frattempo anche STOP e i comandi non partivano. Ora Aurora legge lo stato dalla finestra di JTDX in meno di un secondo e ricorda su disco indirizzo e identificativo di JTDX (`data/jtdx-peer.txt`), cosi' i comandi, compreso STOP, raggiungono JTDX subito; la pagina si riconnette in 2 secondi al massimo (prima 8). I messaggi decodificati restano quelli del periodo in corso: nessuna vecchia stazione ricompare.
- Le letture da JTDX con l'accessibilita' di Windows escono ora in UTF-8 (prima la console li rovinava: lo spazio non divisibile della frequenza diventava un carattere illeggibile).
- Barre Rx (verde) e Tx (rossa) sulla cascata larghe quanto il segnale del modo, dalla frequenza verso destra: FT8 50 Hz (piu' stretta), FT4 83 Hz; per gli altri modi resta la barra sottile.
- Messaggi Tx1-Tx6 come nel motore: Aurora legge dalla sua finestra i testi veri (rapporto e CQ come li ha scritti) e il pallino selezionato, cioe' da dove trasmette; con «Salta Tx1» si parte da Tx2 e la prima riga si vede barrata. I messaggi si scelgono a mano con un clic (`POST /commands/tx-message`): il motore trasmette da quello e poi prosegue da solo. La riga e' accesa forte mentre si trasmette e col solo bordo mentre aspetta il turno.
- Orologio UTC (HH:MM:SS) accanto alla barra del periodo, con la stessa grafica e la stessa misura della barra.
- Cambio banda piu' veloce: da circa 2,7 a circa 0,9 secondi (attese a scatti brevi, split toccato solo se serve). Il pulsante di banda o di modo scelto si accende subito e resta cosi' finche' il motore non conferma, senza tornare indietro e poi riavanzare; se il comando fallisce torna com'era. Vale anche per il doppio clic sul cluster.
- Tasto del periodo di trasmissione come nel motore: «Tx 00/30» o «Tx 15/45» (FT4: «Tx 00/15» o «Tx 7,5/22,5»), di due colori, un clic passa dall'uno all'altro.
- Cascata: una riga per pixel con scorrimento continuo verso il basso, righe mediate (circa 4 al secondo) e filtro orizzontale: niente piu' effetto serpente.
- Attivita' di banda: l'etichetta colorata resta dentro la sua colonna (testo lungo con i puntini); Log: carattere piu' grande.
- Stazione DX: la riga Tx in corso si riconosce dal tipo di messaggio (locatore, rapporto, R+rapporto, RR73, 73, CQ) anche se il testo non coincide col modello, e mostra il testo davvero trasmesso.
- Attivita' di banda: la tacca rossa delle chiamate verso di te non compare piu' sui messaggi di chiusura (RR73 e 73), a QSO concluso.
- Menu in alto e schede delle Impostazioni: la voce in cui ci si trova e' piena e in grassetto (anche `aria-current`).
- Tema scuro: pannelli con fondo pieno, senza vetro sfocato dietro al testo, e tabelle del cluster e del log a corpo intero: i nominativi non risultano piu' sfuocati.

## 0.9.0 - 2026-10-24
- **DX Cluster** (crate `dx-cluster`): client telnet verso server pubblici (catalogo verificato) o propri, con accesso col nominativo, riconnessione, paese dal `cty.dat`, filtri della vista e comandi solo di consultazione (`sh/`, `set/`...): mai spot o messaggi pubblici. Finestra a sinistra di Comandi (DE, Freq, DX, bandiera, Paese, Modo, Commenti, UTC), doppio clic = la radio va sulla frequenza esatta dello spot (`POST /commands/frequency`, solo dentro le bande dei radioamatori, mai in trasmissione). Scheda DX Cluster nelle impostazioni con LED di raggiungibilita' dei server (`POST /dx-cluster/test`), salvataggio automatico e spento di default.
- **Client NTP** (crate `clock-sync`): scarto dell'orologio dai server NTP, controllo periodico, correzione dell'ora di Windows (amministratore, solo rete di casa), indicatore «Orologio» e DT mediano (il «Med» di JTDX) nella barra di stato, scheda Orologio.
- **Impostazioni di JTDX** lette dal suo `JTDX.ini` (`GET /engine-config`, `AURORA_JTDX_INI`, senza password): UDP, radio, audio, sequenza, avvisi, avanzate. Si modificano dall'interfaccia (`POST /engine-config`: chiude JTDX, scrive il file con copia di sicurezza, lo riavvia; mai in trasmissione, solo rete di casa) con una barra «Applica e riavvia» / «Annulla» e le voci cambiate evidenziate.
- **Log di Log4OM2** in sola lettura (`GET /log4om`, `AURORA_LOG4OM_DB`): finestra Log accanto all'attivita' di banda, numero di QSO scelto in Impostazioni.
- Preferenze dell'interfaccia salvate sul server (`data/ui-settings.json`, separate per PC e telefono). Le schede Cluster e Orologio si salvano da sole.
- **Radio**: cambiando banda lo split della FT-2000 si spegne (OmniRig `SetSimplexMode` lo lasciava acceso); il cursore Pwr si regola anche a trasmissione chiusa, in basso la potenza massima come in JTDX; modo JT accanto a FT8 e FT4; comandi di ricezione di JTDX (Hound, Modo SWL, Monitora, Sinc, AGCc, Filtro, Ferma, Decodifica, Cancella) e barra del periodo.
- Doppio clic: la stazione scelta compare subito e resta finche' JTDX non la conferma; avviso se JTDX accetta i Reply solo per CQ e 73. Doppio Esc: libera la stazione da chiamare (Canc DX). Aprendo Aurora non si rivedono piu' le stazioni di prima (`AURORA_REPLAY_ON_CONNECT` ora spento).
- Interfaccia: Attivita' di banda a colonne con griglia su un rigo solo, barra di stato essenziale, modi e bande sulla prima riga, contrasti dei temi chiaro e scuro, frequenze scritte `21.074,0`, piede di pagina sottile.
- L'interfaccia non nomina piu' JTDX: nei testi (sei lingue) e nel codice dell'app si parla di Aurora e di «motore»; la rotta delle impostazioni del motore e' ora `/engine-config` (era `/jtdx-config`).
- DX Cluster: i filtri Banda e Modo sono a scelta multipla (menu a caselle leggibile in entrambi i temi, anche «La mia banda»); le scelte si ricordano.
- Tema scuro, Stazione DX: righe Tx1-Tx6 neutre con una sola tinta per la riga in corso, testo piu' nitido.
- Cambio banda: OmniRig non rende scrivibile `Freq` sulla FT-2000 (il comando pareva riuscito ma la radio restava ferma): ora si scrive `FreqA` e si attende la frequenza nuova.
- Lo script `avvia-sviluppo.ps1` apre la trasmissione (con `-Chiusa` resta chiusa) e collega JTDX.ini, il log di Log4OM2 e OmniRig. Avvio pulito: chiude le copie dell'avvio precedente, apre una sola finestra di Windows Terminal e tiene una sola copia di JTDX; il server di sviluppo (5173) parte solo con `-Sviluppo`.
- Cambio banda: aspetta fino a circa 3 secondi che OmniRig sia in linea e ritenta una volta. Cluster: frequenza senza decimale. Attivita' di banda: distanza senza parentesi quadre. Valori di `AcceptUDPReplyMessages` corretti (1 = solo CQ, 2 = CQ e 73, 3 = qualsiasi). L'avviso «buffer audio perso» compare una volta a ogni 10, 100, 1000 occorrenze.

## 0.8.1 - 2026-10-24
- Tasto Esc: ferma la trasmissione e spegne AutoTX e Abilita Tx, in tutte le pagine; si spegne in Impostazioni → Generali.
- Il doppio clic su un messaggio accende anche Abilita Tx (con la trasmissione aperta): con «1 QSO» JTDX lo spegne a ogni QSO finito e non richiamava piu' la stazione scelta.
- Blocco DX della scheda Trasmissione piu' stretto.
- Cursore Pwr verticale reso uguale in ogni browser (un cursore orizzontale ruotato): su alcuni browser il verso era invertito e portava il livello al minimo invece che al massimo. In alto e' sempre il 100%.

## 0.8.0 - 2026-10-24
- Pannello Comandi come JTDX: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX e Abilita Tx; frequenze Tx/Rx con le frecce di copia e Tx/Rx Split; Canc DX e Gen.Msgs; CQ e messaggio libero. Nuove rotte `GET /controls` e `POST /commands/control` (elenco chiuso: niente accordo ne' log; Abilita Tx e AutoTX si accendono solo con la trasmissione aperta, spegnerli e' sempre permesso).
- Scheda Trasmissione in due blocchi (stazione DX con Tx1-Tx6, e interruttori del QSO) con il livello Pwr verticale a destra.
- Doppio clic su un messaggio: la stazione compare subito nel blocco DX e JTDX la chiama al momento giusto; l'opzione si spegne in Impostazioni → Generali.
- Corretto il cambio modo FT8/FT4: prima accendeva solo la casella senza far cambiare modo a JTDX; ora preme il pulsante e resta nella banda di prima, sulla frequenza che la tabella indica per il nuovo modo.
- Cambio banda e modo si vedono subito nella pagina, senza aspettare il prossimo stato di JTDX.
- STOP TX piu' piccolo e discreto a riposo, pieno in trasmissione; l'esito dei comandi e' una nota che sparisce da sola e non occupa spazio.
- Pulsanti di modo e di banda a misura fissa; FT8 viola e FT4 ambra, anche da spenti.

## 0.7.0 - 2026-10-24
- Guida in-app (`#/guida`): 11 argomenti brevi nelle sei lingue, raggiungibile dalla testata e dalla scheda Avanzate.
- Pagina Statistiche (`#/statistiche`): premi DXCC, zone CQ e ITU, quadranti, prefissi, QSO per banda e paesi mancanti, dal log ADIF.
- Elenco messaggi: i propri messaggi trasmessi compaiono come righe `Tx` (una per trasmissione), in ordine d'orario, senza risposta al doppio clic.

## 0.6.0 - 2026-10-24
- Sei lingue (italiano, inglese, tedesco, francese, spagnolo, portoghese) con selettore nella testata e un tooltip su ogni controllo; i testi stanno in tabelle per area (`app/src/i18n/strings`) e i test impediscono chiavi mancanti o segnaposto diversi.
- Impostazioni a 13 schede come JTDX: generali, radio, audio, sequenza, macro TX, segnalazioni, frequenze, notifiche, filtri, programmazione, avanzate, accesso, finestre, bande.
- Filtri dei messaggi (continenti, paesi, nominativi, solo CQ/RRR/RR73/73, non standard, proprio continente), preferenze di visualizzazione (miglia, paese, bandiera, distanza, DT), flag evidenzia/suono per ogni tipo di messaggio, macro TX (@ DX, & io, # rapporto).
- Frequenze di lavoro per banda e modo (FT8/FT4) modificabili e salvate; il cambio banda le usa. Programmazione delle bande per orario UTC (fino a 5 righe).
- Accesso: durata del codice di associazione decisa dall'operatore (1-60 minuti, predefinita 10, solo dal PC), accesso senza codice per le prove (solo rete di casa, mai da proxy, mai al riavvio, `AURORA_OPEN_ACCESS`), elenco e revoca dei dispositivi, codice con QR dalle impostazioni.
- Stazione: bandiere dei paesi, colonne a larghezza fissa che si adattano al pannello, etichette corte con il nome intero nel tooltip, stazione chiamata in evidenza, chiamate verso di te in rilievo con tacca rossa e avviso sonoro dedicato, doppio clic per rispondere (un clic seleziona soltanto), fase del QSO nel pannello Tx, DX e messaggio in trasmissione.
- Cascata ridimensionabile a mano anche da telefono; il trascinamento dei pannelli non li fa piu' sparire.
- Livello audio di uscita (cursore Pwr di JTDX) con il suggerimento sull'ALC: `GET/PUT /api/v1/output-level`, la modifica richiede la trasmissione aperta.
- Continente in ogni messaggio e nello stato; il simulatore e i test non contengono nominativi reali.
- Corretto: con l'accesso senza codice il flusso `/stream` veniva chiuso ogni secondo (il dispositivo virtuale non e' nell'archivio), cosi' sul telefono comparivano "Scollegato", lampeggio e nessun dato. Ora il flusso resta aperto finche' l'accesso senza codice e' acceso.

## 0.5.0 - 2026-10-24

## 0.5.0 - 2026-10-24
- `qso-engine`: automazione dei QSO (risponde ai CQ con novita'). Parte spenta e in prova a vuoto; un solo QSO attivo, tentativi per stazione, tetto di risposte di fila (si spegne), tetto giornaliero, dead-man senza dispositivi collegati, STOP TX la spegne. Registro delle decisioni con il motivo. API `GET/PUT /api/v1/automation`: dal telefono solo prova a vuoto o spegnimento, il modo reale solo dal PC con l'avviso confermato e `AURORA_ALLOW_TX=1`.
- Il motore JTDX annuncia `TxHalted` a ogni STOP TX (anche prima di conoscere JTDX).
- `qso-export`: ogni QSO registrato va nel log di Aurora e, se configurato, in ADIF su UDP (`AURORA_EXPORT_UDP`) o su file (`AURORA_EXPORT_FILE`).
- `GET /api/v1/station-info`: come e' avviata la stazione (sola lettura).
- Interfaccia: finestre (cascata, elenco, stazione, CQ) trascinabili in qualunque colonna o in basso e ridimensionabili, divisore tra colonne; disposizione ricordata. Elenco delle stazioni a sinistra come in JTDX (UTC, dB, DT, frequenza, messaggio, paese e distanza in km, righe colorate per tipo). Barra delle bande (solo visualizzazione). Aspetto di vetro traslucido con sfondo luminoso.
- Impostazioni a schede: Colori (le 17 categorie di JTDX piu' la risposta automatica, valori di partenza del tema scuro di JTDX), Finestre, Avvisi sonori, Trasmissione, Log esterni, Collegamenti. Pagina Automazioni.
- Comandi di trasmissione: CQ (TriggerCQ di JTDX, provato dal vivo: il bool a `true` abilita Tx), messaggio libero (13 caratteri; con invio solo a trasmissione aperta) e frequenza audio di trasmissione. API `commands/cq`, `commands/free-text`, `commands/tx-frequency`; cio' che trasmette richiede `AURORA_ALLOW_TX=1` e una via consentita. Pannello Trasmissione nell'interfaccia.
- Stato: stazione DX, rapporto, frequenze Tx/Rx e messaggio in trasmissione da JTDX; pannello con Tx1-Tx6 previsti (si accende quello in uso).
- Etichette complete come i colori di JTDX: zona CQ e ITU (anche per banda), griglia per banda, prefisso (WPX), nuova stazione, gia' lavorata.
- Finestre comprimibili con doppio clic sul titolo (la cascata si riduce alla sola barra).
- L'interfaccia compilata si serve da Aurora (`AURORA_UI_DIR`) anche sulla rete locale, per l'uso da smartphone.
- Cambio banda: `rig-control` comanda la radio via OmniRig (condiviso con JTDX, che lo usa come CAT), `POST /api/v1/commands/band` con le frequenze FT8 predefinite, mai in trasmissione; barra delle bande cliccabile. Si attiva con `AURORA_RIG=omnirig`. Provato dal vivo (15 m -> 20 m -> 15 m).
- Cambio modo FT8/FT4 (si preme il pulsante di JTDX con l'accessibilita' di Windows, `POST commands/mode`), frequenza Rx impostabile (`commands/rx-frequency`).
- Spettro con i colori di JTDX (blu, giallo, bianco) e le due barre verde (Rx) e rossa (Tx) trascinabili; clic sposta Rx, Maiusc+clic sposta Tx.
- Linguetta piccola a destra: rosso lampeggiante = trasmissione in onda, verde = ascolto. Barra delle bande filtrabile dalle impostazioni, banda in uso ben evidente.
- Statistiche del log (premi DXCC, zone CQ e ITU, griglie, prefissi, paesi mancanti), regole dell'automazione salvate.
- Simulatore e test senza nominativi reali.
- Test dell'interfaccia: cartelle temporanee uniche nei test e2e.

## 0.4.0 - 2026-10-24
- Cascata: contrasto automatico che segue il rumore di fondo.
- `dxcc`: paese e zone dei nominativi da `cty.dat` (prefissi piu' lunghi, eccezioni `=CALL`, `/P` e simili, `/MM` senza paese).
- Novita' sui decode: paese di chi trasmette e etichette NUOVO DXCC, DXCC NUOVA BANDA, NUOVA ZONA, NUOVA GRIGLIA, calcolate dai QSO gia' fatti (log ADIF di JTDX) e aggiornate a ogni QSO chiuso. Senza log o senza banda nota restano solo le etichette CQ.
- Nuove variabili: `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`; file mancanti non fermano Aurora.
- Interfaccia: pannello "CQ e novita'", paese sotto ogni messaggio, avvisi sonori (spenti all'inizio, suono diverso per nuovo paese e altre novita', volume ricordato).

## 0.3.0 - 2026-10-24
- Collegamento reale a JTDX 2.2.159: `udp-hub` (ripetitore del flusso UDP, inoltro identico a GridTracker2 e comandi di ritorno), `engine-jtdx` (Status, Decode e QSO negli eventi di Aurora; Reply e HaltTx verso JTDX).
- **Trasmissione chiusa per impostazione predefinita** (`AURORA_ALLOW_TX=0`): un Reply fa trasmettere JTDX davvero. HaltTx e STOP TX funzionano sempre.
- `audio-spectrum`: cascata dall'ingresso audio (`AURORA_AUDIO_DEVICE`).
- `udp-bridge`: Heartbeat di JTDX senza revisione, comando Replay; test con pacchetti reali anonimizzati.
- Documentazione: mappa dei flussi UDP (JTDX, GridTracker2, Log4OM2), guida al collegamento, esiti del reverse con REA e Ghidra.

## 0.2.0 - 2026-10-24
- Core: `aurora-core` (tipi, bus di eventi, trait `Engine`), `engine-sim` (stazione simulata), `logbook` (SQLite, duplicati, ADIF), `auth` (associazione dispositivi con token a hash e blocco anti-forza-bruta), `remote-api` (API v1, WebSocket, OpenAPI), programma `aurora`.
- Interfaccia: React 19 + TypeScript 7 + Vite 8, tema Notte radio, cascata WebGPU con ripiego WebGL2, pagina Live, associazione del dispositivo, installazione come app (PWA).
- Script `avvia-sviluppo.ps1` e `associa-telefono.ps1`.
- Reverse di `jtdx.exe` con REA + Ghidra: conferme su server UDP e Reply (vedi `docs/fase0-rapporto.md`).

## 0.0.2 - 2026-10-24
- `udp-bridge`: decodifica e codifica del protocollo UDP di JTDX.
- Strumenti di prova `udp_capture`, `udp_send`, `audio_tone`.
- Bozza dell'inventario delle funzioni di JTDX e guida alle prove manuali.

## 0.0.1 - 2026-10-24
- Struttura iniziale del repository e specifica di design.
