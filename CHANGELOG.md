# Changelog

## Non rilasciato
- **Aggiornamento**: se Aurora controlla mentre GitHub sta ancora allegando l'installatore alla release, non resta piu' per un'ora su «la release non ha il pacchetto di installazione con la sua firma»: al momento di scaricare richiede subito la release.

## 0.11.221 - 2026-11-10
- **Dipendenze aggiornate**: librerie Rust alle ultime versioni compatibili (`cargo update` in `core` e `desktop/src-tauri`); interfaccia con @tanstack/react-query 5.107, zustand 5.1, d3-geo 3.1.2, vite 8.3.7, vitest 5.1, @vitejs/plugin-react 6.3; servizio dei rapporti con wrangler 4.159 e @cloudflare/workers-types 5. Nessun cambiamento nell'uso.

## 0.11.220 - 2026-11-10
- **Hamlib**: su Windows, se `rigctld.exe` sparisce (di solito l'antivirus), il pulsante «Ripristina Hamlib» nella scheda Radio lo riscarica dalla release ufficiale 4.7.2 con impronta SHA-256 controllata. Su Linux Hamlib e' dentro l'AppImage e obbligatorio nel .deb; la compilazione si ferma se manca.
- **Blocchi**: le finestre a parte si aprono senza fermare la finestra principale (blocchi «apertura di una finestra a parte» della 0.11.219); il rapporto di un blocco elenca le ultime attivita' della finestra.
- **Audio**: con la radio spenta o il cavo USB staccato all'avvio la trasmissione non resta piu' spenta: riparte da sola sulla stessa scheda appena la radio si accende.
- **Decodifica**: un processore resta libero per audio, CAT e finestra, i thread sono a priorita' piu' bassa e non si ricreano a ogni periodo.
- **Attivita' di banda**: dopo un cambio di banda non si perdono piu' i primi messaggi della banda nuova.
- **PC lento**: l'avviso suggerisce anche di spegnere a7/a8; nuova voce della Guida «Che computer serve».
- **Rapporti**: tipo di computer (processore, core, memoria) con il segnale giornaliero e con gli errori; il PTT fermo nei primi secondi dopo il collegamento non e' piu' un errore.

## 0.11.219 - 2026-11-09
- **Mappa del pannello Log**: sempre sulla banda in uso, una riga sola di comandi (Linee, Chi chiama chi, Solo chi mi sente, Nominativi), niente legenda ne' coordinate al passaggio; si apre gia' della misura giusta (niente mappa «fantasma»); tolto dalla Mappa DX il riquadro degli spot, la durata e' tra le opzioni.
- **QSO a log**: con la Mappa aperta il pannello passa da solo al Log (o «Log interno»), mostra la riga che lampeggia e torna alla Mappa; l'animazione riconosce la riga dal nominativo.
- **Selettori**: banda e frequenza piu' leggibili con sfondo pieno nei due temi; il modo diventa grigio in trasmissione.
- **Contest**: spento e' pulito (niente schede ne' frequenza); RX/TX quadrato a fondo pieno e centrato, pulsanti senza pallini, Abilita Tx e Auto CQ rossi da accesi, STOP TX con Esc, moltiplicatori con prefisso e nominativo in colonne fisse.
- **Ricarica**: niente pagina vuota ricaricando sul Contest; un errore in una pagina non svuota piu' tutta l'interfaccia.
- **Prestazioni**: Attivita' di banda disegna prima le righe visibili e non impagina quelle fuori schermo; con la finestra nascosta si fermano le misure periodiche.

## 0.11.218 - 2026-11-09
- **Contest, tastiera**: F1 Auto CQ, F2/F3/F4 scambio, R + scambio e chiusura; Esc ferma, doppio Esc cancella il QSO; i tasti sono scritti accanto ai pulsanti.
- **Contest, pagina**: frequenza e quadratino RX/TX sopra le decodifiche (clic su TX ferma, su RX cancella il QSO); «Come funziona» con il riassunto del regolamento e un QSO d'esempio; Abilita Tx e Auto CQ rossi quando accesi; tolta la scritta Fox/Hound (vietato da tutti i regolamenti, il blocco resta); moltiplicatori 4 per riga con legenda a quadratini nel titolo; tabella con dB, Hz, km, Novita' e Valore a sinistra.
- **Avvisi in primo piano**: messaggi a scomparsa, avviso del DXCC nuovo e schede della stazione sopra le Impostazioni.
- **Ricarica**: ricaricando la pagina si resta dove si era (l'apertura di Aurora parte sempre dalla Stazione).

## 0.11.217 - 2026-11-09
- **Paesi non confermati** (richiesta di Luc): a scelta, un paese lavorato ma non confermato (LoTW o QSL nel log) resta «DXCC nuovo», nei colori, nel filtro e nelle Automazioni.
- **Mappa DX**: banda in uso di base, «Solo chi mi sente» (solo le linee di PSK Reporter), nominativi visibili al passaggio del mouse, linea rossa verso la stazione chiamata in trasmissione; la linguetta Mappa del pannello Log e' la Mappa DX completa.
- **Finestra RX**: torna all'ultima riga anche dopo il doppio clic (cambio della frequenza RX) e con le righe Tx.

## 0.11.216 - 2026-11-09
- **Aggiornamento**: l'installatore parte anche sui PC dove PowerShell non e' nel PATH (dai rapporti: «installatore non avviato: program not found»).
- **Rapporti degli errori**: niente piu' falsi «programma bloccato» e panic nei secondi in cui Aurora si chiude per aggiornarsi; non si mandano piu' «radio non collegata» e l'uscita audio che Windows rifa' e Aurora riapre da sola.
- **Contest, moltiplicatori**: tessere colorate per stato (nuovo, nuovo qui ma preso su un'altra banda, gia' preso qui, stazione gia' lavorata) con la legenda; nelle decodifiche le stazioni gia' lavorate sulla banda sono depennate.

## 0.11.215 - 2026-11-09
- **Nuovo stato nelle Automazioni**: «Nuovo stato (USA)» e «Stato sulla nuova banda (USA)» anche come regole di Auto.
- **Stato nel log**: lo stato del corrispondente si salva nel log di Aurora (dalla ricerca QRZ/HamQTH e dagli ADIF importati) e va nel record ADIF; gli stati lavorati uniscono il log di Aurora e Log4OM.
- **Contest**: nel log del contest entrano solo i QSO avviati dalla pagina Contest e fatti durante la gara (quelli di prima si tolgono da soli); chi non fa il contest entra non valido a 0 punti e il Cabrillo lo scarta; riga per chiudere il QSO con i messaggi normali; tasto destro sul log (valido, incerto, non in contest).
- **Pagina Contest**: colonne fisse con griglia e titoli a sinistra, selettore delle frequenze accanto alle schede, bande del consiglio cliccabili, QSO e log a sinistra, moltiplicatori a tessere, «Consigli e ritmo» compatto.
- **Voce degli avvisi**: frasi rifatte con Kokoro (licenza libera Apache 2.0), voci femminili; il tedesco usa le frasi inglesi.
- **DX Cluster**: tolta la riga «Radio portata su…» sopra la tabella.

## 0.11.214 - 2026-11-09
- **Mappa nel pannello Log**: nominativi accanto alle stazioni nel colore della novita', clic per chiamare, zoom (rotella e +/-), trascinamento; si pulisce a ogni cambio di frequenza.
- **Coda TX**: i nominativi in coda si cliccano per passare subito a loro; il pulsantino » passa al primo della coda.
- **Suoni**: per ogni avviso la voce (Ava: frase dell'avviso e nominativo con l'alfabeto fonetico, nella lingua di Aurora o in inglese) o un proprio file WAV/MP3.
- **Avviso del DXCC nuovo**: compare una volta sola per stazione, anche cambiando pagina; si sceglie dove mostrarlo e con quale suono.
- **Nuovo stato (USA)**: pillole «Nuovo stato» e «Stato sulla nuova banda» in Attivita' di banda (stato da QRZ/HamQTH, stati lavorati dal log di Log4OM).
- **Automazioni**: «Filtra chi risponde al mio CQ anche senza Auto» applica i filtri di Attivita' di banda anche al CQ fatto a mano.
- **Finestra RX**: bandierina e paese spostati a destra, la riga Tx non si taglia piu'.
- **Statistiche**: l'interruttore si chiama «Invia statistiche» (acceso di fabbrica) e dice cosa si manda. Con il nominativo impostato, il segnale giornaliero aggiunge un codice ricavato dal nominativo sul PC (SHA-256, il nominativo non si manda mai) per contare le persone e non i PC; il servizio lo salva solo dopo un HMAC con una chiave segreta.
## 0.11.213 - 2026-11-09
- **Aggiornamento dal tray**: con «Aggiorna ora» del tray la finestra di Aurora non mostra la locandina (c'e' gia' la cartolina del tray), anche se Aurora viene aperta per l'occasione.
- **Auto**: colorato solo quando e' acceso (arancio in prova a vuoto, verde in modo reale); spento e' come gli altri pulsanti.
- **Automazioni, continenti**: nuova casella «Escludi»: spuntata, Aurora non risponde ai continenti scelti; spenta, risponde solo a quelli (prima era solo «solo questi», e chi spuntava EU per escluderla otteneva il contrario).
- **Attivita' di banda**: con la finestra RX aperta, i messaggi della stazione con cui si sta facendo il QSO restano solo nella finestra RX.
- **Finestra RX**: il suggerimento su «Hz ±50» spiega che mostra solo la propria frequenza, mentre Auto guarda tutta la banda.

## 0.11.212 - 2026-11-09
- **Automatico**: il pulsante accende le Automazioni nel modo scelto nelle Impostazioni (prova a vuoto o reale); prima chiedeva sempre il modo reale senza la conferma dell'avviso, il motore rifiutava e non succedeva nulla. Se non si accendono, un avviso apre le Impostazioni → Automazioni.
- **Auto**: il pulsante si chiama «Auto» ed e' sempre colorato: viola spento, arancio in prova a vuoto, verde in modo reale.

## 0.11.211 - 2026-11-09
- **Pannello RX**: la finestra RX e' un pannello separato sopra il Log, con la sua barra del titolo (come Bande aperte e Comandi); il pulsantino «RX» del Log lo accende e lo spegne.
- **Log**: scheda DXCC sotto il log piu' compatta.

## 0.11.210 - 2026-11-09
- **Aggiornamento**: se l'installatore silenzioso finisce senza riaprire Aurora (successo aggiornando dal tray), lo script che lo ha lanciato la riapre da solo.
- **Filtro e Automazioni**: scegliendo una novita' (nuova griglia, nuova zona…) si accende da sola la sua evidenziazione in Segnalazioni, se era spenta: prima il filtro mostrava solo le proprie trasmissioni (ON7KEC); messaggio chiaro quando nessuna stazione ha le novita' scelte.
- **Nuova griglia**: evidenziazione «nuova griglia» e «griglia su nuova banda» accese di base (prima spente: Automazioni e filtri «Nuova griglia» non trovavano niente); «Griglia su nuova banda» anche tra le risposte delle Automazioni (ON7KEC).
- **Automazioni**: tra «A chi rispondere» anche «Nuova stazione» e «Stazione su nuova banda o modo»; la pausa per stazione puo' essere 0 (nessuna pausa, la richiama subito) (richieste di ON7KEC).
- **Attivita' di banda**: nel «Filtro» anche le eccezioni come GridTracker: Solo CQ, RR73 come CQ, Ha locatore, Non il mio continente (richiesta di ON7KEC).
- **TCI**: niente avviso «audio muto» quando l'audio TCI arriva basso ma c'e' (issue #9); resta solo con zeri veri.
- Impostazioni, scheda del Log: nuovo riquadro «Database di Log4OM» per scegliere (finestra «Apri file» o percorso a mano) o togliere il file di Log4OM 2; si controlla, si ricorda e i QSO già fatti si ricaricano subito.

## 0.11.209 - 2026-11-09
- **Contest**: nuovi regolamenti YBDXPI FT8 Contest, VHF-UHF FT8 Activity Contest (Europa e Nord America, 144/432/1296 MHz a rotazione) e NCCC FT4 Sprint settimanale; il calendario accetta piu' ordinali («primo,secondo,terzo-mercoledi») e i contest settimanali («ogni-giovedi»); punti per paese e continente.
- **Pannello Log**: linguette «Log | Mappa | RX» a destra; il titolo segue la linguetta; con «RX» la finestra RX come in WSJT-X (chi trasmette dove sta la barretta Rx, ±50 Hz, i messaggi diretti a te e le tue trasmissioni); totale «N QSO» accanto al titolo.
- **Aspetto**: nuova scheda delle Impostazioni con tema (scuro, chiaro, automatico), «Alto contrasto» in chiaro e in scuro per una lettura piu' facile, carattere dell'interfaccia e dei dati, dimensione del testo dal 90% al 140%, colore del testo con controllo del contrasto, anteprima immediata e «Ripristina».
- Stazione: il pannello Log ha le linguette «Log | Mappa»; con «Mappa» la Mappa DX compatta compare al posto del log, nello stesso spazio (la scelta si ricorda).
- **Attivita' di banda**: pulsantino «Filtro» nella barra con le pillole delle novita' (nuovo DXCC, DXCC su banda, nuova griglia, gia' lavorata...; «Tutte» = nessun filtro), seguito anche dalle Automazioni; nella scheda Filtri una sezione completa con novita', SNR minimo, distanza minima e massima, bande, messaggi a me e mie trasmissioni.
- **Automazioni**: in modo reale «Abilita Tx» resta acceso a fine QSO (se il motore lo spegne lo riaccendono; STOP le ferma come prima) e il motore non risponde piu' a chi ci chiama ed e' escluso da continenti, paesi o nominativi delle regole o nascosto dai filtri di Attivita' di banda; lo scarto resta nel registro delle decisioni.
- **Automazioni**: pulsante «Automatico» nella barra in alto per accenderle o spegnerle con un clic (verde in modo reale, arancio in prova; richiesta di ON7KEC).

## 0.11.208 - 2026-11-09
- **TCI**: niente piu' avviso «segnale debole: alza il volume di registrazione» con l'audio dal TCI (non c'e' un volume di Windows da alzare e il segnale si decodifica); restano gli avvisi di ingresso muto e saturo.
- **Contest**: linguette «Allenamento» (pile-up finto leggero, medio o fitto per 15, 30 o 60 minuti con lo scambio del contest scelto, errori, doppi e chi non riceve RR73: niente trasmissione e niente log, punteggio a parte) e «Analisi» (dal log del contest: QSO, moltiplicatori, punteggio, QSO incerti, QSO ora per ora, per banda rate migliore e tempo perso, un consiglio).
- **Contest**: consiglio di banda e modo (per ogni banda del regolamento rate stimato e moltiplicatori nuovi in ascolto, FT4 quando il pile-up e' fitto, pulsante «Porta la radio su …» con le frequenze del regolamento, cambi di banda nell'ora se limitati) e mappa dei moltiplicatori della banda (gia' lavorati, nuovi in ascolto in arancio, sentiti non nuovi; per sezioni, stati e paesi l'elenco dei nuovi): un clic chiama la stazione.
- **Contest**: avviso quando un nominativo composto (tuo o del DX) non permette i messaggi di contest; PACCdigi con 3 punti per le stazioni olandesi (nuovo tipo di punti «per paese»).
- **Contest**: con il contest acceso, su una banda o in un modo non ammessi dal regolamento la trasmissione non parte (Abilita Tx si spegne, con un avviso); il menu delle frequenze mostra solo le bande del contest; lo scambio ricevuto si controlla con le liste di sezioni ARRL/RAC, stati, province canadesi e olandesi (fuori lista: segnato nel log con il motivo e niente moltiplicatore); moltiplicatori DXCC; EDI con righe ERROR per i seriali persi, PSect e antenna (SAnte).
- **Regione IARU**: al primo avvio, se e' ancora «Tutte», si imposta da sola dal nominativo (IU3WJQ -> Regione 1); nelle Impostazioni ogni regione dice a cosa corrisponde (1 Europa, Africa, Medio Oriente, Russia; 2 Americhe; 3 Asia e Oceania).
- **OmniRig**: scelta della radio Rig 1 o Rig 2 nella scheda Radio e con il blocchetto «RIG 1 / RIG 2» a sinistra del display, nei colori dello stile del display (frequenza e PTT; richiesta di IK1SOW).
- **Meno lavoro inutile**: le riletture del log che arrivano insieme si uniscono (dopo un QSO se ne facevano quattro, ognuna con decine di migliaia di QSO); niente confronto con JTDX quando JTDX non c'e' (una riga e una scrittura ogni periodo); registrazioni WAV di ogni periodo spente di base (circa 144 MB e una scrittura ogni 15 s, si riaccendono con AURORA_NATIVE_RECORD=1).
- **Contest e Fox**: interruttore verde «acceso / spento» ben visibile; a contest spento i QSO non vanno nel log del contest e il servizio non lavora sulle decodifiche.
- **Impostazioni**: scheda «CQ e novità» tutta sua (prima era dentro Filtri); scheda Frequenze con i comandi in alto, tabella alta quanto la finestra e righe fitte.
- **Novità**: le etichette «su nuova banda» si chiamano «su nuova banda o modo» (valgono anche per modo con «per modo» in Segnalazioni).
- **Esporta log**: «Salva Cabrillo…» accanto a «Salva ADIF…» (Cabrillo 3.0 con frequenza, data, ora, rapporto e locatore inviati e ricevuti).
- **Q65**: deriva di frequenza (max_drift di W3SZ), ritardo EME fino a 5,5 s nel sincronismo e lista dei messaggi dei contest (q65_set_list2), con i pulsanti «EME», «Deriva» e «Cont» accanto a «Lor/Gau».
- **Automazioni**: filtri di caccia a pulsanti a pillola riquadri «Prossime stazioni» (i CQ interessanti non scelti, dal migliore) e «Ultime stazioni chiamate in automatico» (idee di ON7KEC, issue #6).

## 0.11.207 - 2026-11-09
- **Distanze**: i km sempre interi, senza decimali (nella pagina Contest comparivano con dieci cifre dopo la virgola).

- **Testata**: con la versione nuova pronta la striscia diventa una pillola verde fluo «Nuova 0.11.x» con il puntino che pulsa e «Installa».

## 0.11.206 - 2026-11-09
- **Aggiornamento**: all'avvio, prima dell'installazione e nell'aggiornamento dall'app si chiude anche il core avviato da solo (`aurora.exe`) rimasto acceso, che teneva la porta e mostrava la versione vecchia; si chiude anche l'Aurora che non e' piu' in ascolto ma tiene connessioni aperte con la finestra.
- **Testata**: i pulsanti della striscia di aggiornamento restano leggibili al passaggio del mouse.

## 0.11.205 - 2026-11-09
- **Aggiornamento**: all'avvio Aurora chiude un Aurora di un'altra versione rimasto acceso sulla stessa porta (anche solo su 127.0.0.1), che si prendeva la finestra e mostrava la versione vecchia anche dopo l'aggiornamento; la finestra del programma non usa piu' il service worker e toglie quello vecchio.
- **Aggiornamento**: il controllo si ripete ogni ora (prima ogni 3 ore) e parte 1 secondo dopo l'avvio; la locandina compare subito anche mentre la versione nuova si scarica, con il nastrino «Nuova versione» in diagonale; «Piu' tardi» vale fino alla prossima apertura.
- **Aggiornamento silenzioso**: come le app che si aggiornano da sole, a pacchetto pronto e verificato la locandina installa da sola dopo 5 secondi (con «Annulla», mai mentre la radio trasmette), senza la finestra dell'installatore, e Aurora si riapre da sola.
- **Installatore**: grafica nuova con tutti i modi (FT8, FT4, Q65, MSK144, JT65, JT9, JT4), Contest e DXpedition; con una versione piu' vecchia installata la scelta in alto e gia' selezionata e' «Aggiorna (consigliato)».

## 0.11.204 - 2026-11-08
- **Contest**: nuovi regolamenti C.S.M.I. Marconi Speed Contest (25 aprile, FT4/FT8, stazioni IY, log non da inviare), FT8 DX Contest e FT4 DX Contest dell'European FT8 Club; PACCdigi corretto (orari 12:00-11:59, frequenze FT8 1,843-3,577-7,077-14,077-21,077-28,077 MHz e FT4 dal regolamento, scambio rapporto e numero progressivo).
- **Frequenze**: 70,100 MHz alternativa FT8 dei 4 m; le righe nuove (70 cm, 4 m) si aggiungono anche alle tabelle gia' salvate.

## 0.11.203 - 2026-11-08
- **Aggiornamento**: l'installatore parte solo con Aurora e l'agente chiusi (prima «Impossibile chiudere Aurora» a meta' installazione); la locandina non compare da sola durante l'aggiornamento dal tray.
- **FT-991(A) Link**: nessun errore durante il cambio di banda, DATA-USB rimandato a ogni cambio di banda (la radio riprendeva FM).
- **70 cm**: frequenze FT8 432,174 e FT4 432,170 MHz.

## 0.11.202 - 2026-11-08
- **Categoria del contest a pannello**: nella barra solo la sigla (SO-LP, MO, Rover, 2A); il clic apre un pannello come il menu dei contest con le categorie del regolamento e la loro spiegazione, gli operatori (riga OPERATORS; da soli il nominativo della stazione), e potenza, Assisted e classe del Field Day (trasmettitori + lettera A-F) solo dove il regolamento li prevede. Le scelte si salvano per contest e vanno nell'intestazione Cabrillo (CATEGORY-OPERATOR/POWER/ASSISTED/TRANSMITTER/STATION, OPERATORS) ed EDI, e nello scambio del Field Day (2A EMA). Nuovi campi: nominativo in gara, indirizzo ed email (ADDRESS/EMAIL); nome, indirizzo, email e club si scrivono una volta e valgono per tutti i contest. Pagina Contest segnata BETA con un nastro nell'angolo.
- **Cascata nella pagina Contest**: piccola e senza barre, sotto le Decodifiche e larga quanto loro, come nella Fox (filtro, spot del cluster, barre Rx/Tx trascinabili, clic che sposta la Rx, righello con i messaggi); parte da circa 60 px, con una maniglia sopra per regolarla (fino a 420 px), e l'altezza si ricorda. Le colonne di destra restano a tutta altezza e la pagina sta nella finestra senza scorrere.
- **Pagina Contest piu' grafica**: riquadri con lo stile delle schede della Stazione (bordo e barra del titolo colorata); Decodifiche come Attivita' di banda (fasce alterne, bandiere, nominativo a pastiglia, km, valore a pastiglia verde con ★ per la consigliata, novita' come etichette colorate, doppi attenuati, QSO in corso giallo con la barra rossa).
- **Date del contest nella barra**: accanto alla tendina, compatte («29–30 ago 2026»), calcolate ogni anno dalla regola del calendario; un clic apre gli orari UTC e locali, la durata, ore massime e pausa minima e il link al regolamento; «?» se vanno ricontrollate sul regolamento dell'anno. Al posto della tendina del sistema un menu a comparsa nello stile dell'app: contest raggruppati (HF, Field Day, VHF e oltre, Altro), nome senza anno, data della prossima edizione e pallino di stato (verde in corso, arancio entro 7 giorni, grigio lontano), voce scelta in viola, frecce, Invio ed Esc; il pulsante mostra nome e data, e «ⓘ» accanto apre orari, durata e regolamento.
- **Tx/Rx Split nel QSO in corso del Contest**: lo stesso pulsante della Stazione e della Fox.
- **Pagina Contest** (nuova voce «Contest» nel menu, `#/contest`), come il prototipo approvato: barra del punteggio (contest scelto, tempo alla fine, QSO, moltiplicatori, punti, rate 10 e 60 minuti, previsto a fine gara, scambio inviato, categoria, Fox/Hound bloccato), decodifiche ordinate per valore (punti e moltiplicatori nuovi, doppi barrati, QSO in corso, chi non fa il contest), QSO in corso con la sequenza e «chi chiamare dopo», moltiplicatori mancanti per banda, log del contest con le correzioni a mano e i QSO incerti, consigli e ritmo; finestre Regole del contest (regolamento in breve e link a quello ufficiale), Calendario con «Preparami», Cabrillo/EDI/ADIF con il controllo prima dell'invio, Impostazioni del contest. Testi nelle sei lingue, scheda «Contest» nella Guida.
- **Regolamenti di 17 contest** in file TOML (`core/crates/contest/regole`, sostituibili o aggiungibili nella cartella dei dati `contest/regole`): WW Digi, ARRL Digi, ARRL Field Day, International FT Challenge, ARRL VHF di gennaio, giugno e settembre, CQ WW VHF digitale, IARU R1 MGM 50 e 70 MHz, EU VHF, ARI FT8 Activity 144 e 432, DARC FT4, RSGB FT4, PACCdigi e «Personalizzato», con scambio, bande e frequenze, doppi, punti, moltiplicatori, orari, categorie e formato del log; i valori da ricontrollare sono segnati «da verificare sul regolamento dell'anno».
- **Impostazioni preparate per ogni contest**: scegliendo il contest Aurora imposta messaggi e parola del CQ, scambio (locatore, classe e sezione del Field Day, seriale con il numero di partenza), Fox e Hound spenti, AutoSeq, Hold Tx Freq, chiusura e modo; tutto modificabile e salvato per contest, e «Fine gara» rimette le impostazioni di prima.
- **Sequenza di contest nel motore** come la «Special operating activity» di WSJT-X 3.0.1 (NA VHF, ARRL Digi, WW Digi, EU VHF con i due nominativi tra parentesi, Field Day, RTTY Roundup): scambio ricevuto letto e messo a log, seriale che avanza a ogni QSO, RR73 rimandato se il corrispondente ripete lo scambio. Solo con un contest attivo: fuori dal contest i QSO restano quelli di sempre (e un QSO normale senza rapporto continua a non andare a log). Con un contest attivo Fox e Hound restano spenti e la pagina Fox lo spiega.
- **Log del contest** separato (oltre al log normale, dove ogni QSO va come sempre con gli scambi): doppi per banda o per banda e modo secondo il regolamento, punti (fissi, per banda, per distanza, km IARU), moltiplicatori (campi, quadrati, prefissi WPX, province), punteggio, QSO incerti; esportazione Cabrillo 3.0, EDI (REG1TEST) per banda e ADIF con `CONTEST_ID`.
- **Aggiornamento dall'app anche su macOS**: a ogni release il workflow macOS allega accanto al .dmg il pacchetto di aggiornamento `Aurora-macOS.app.tar.gz` (Aurora.app compressa) con la sua firma minisign `.sig`, fatta con la stessa chiave degli aggiornamenti di Windows. Sul Mac Aurora lo scarica in anticipo con le stesse ripetizioni, ne controlla la firma e con «Installa e riavvia» estrae l'app in una cartella temporanea, sostituisce l'Aurora.app in uso, toglie la quarantena e si riapre (`open -n`) appena la vecchia istanza e' chiusa; barretta in testata e locandina funzionano come su Windows. Se l'app gira dal .dmg, da una copia protetta di macOS (App Translocation) o in una cartella senza permesso di scrittura, l'errore lo dice con il link alla release. Al primo avvio dopo l'aggiornamento macOS potrebbe chiedere conferma (Aurora non e' firmata da Apple). Windows e Linux non cambiano.
- **Automazioni, «Abilita Tx» automatico**: quando le Automazioni si accendono in modo reale Aurora accende da sola «Abilita Tx» (senza, le risposte automatiche non partivano) e lo riaccende prima di ogni risposta o CQ automatico se il motore lo ha spento a fine QSO; quando le Automazioni si fermano (a mano, per un tetto, dal dead-man, con STOP o passando alla prova a vuoto) «Abilita Tx» torna com'era prima: se l'avevano acceso loro, si spegne. In prova a vuoto non si tocca. Il registro delle decisioni lo dice; tooltip e Guida aggiornati (richiesta di ON7KEC, issue #6).
- **Automazioni, interruttore «Prova a vuoto | Modo reale»**: nella testata della scheda Automazioni il modo si sceglie da un piccolo interruttore. Passare al modo reale (solo dal PC) chiede di confermare un avviso nelle sei lingue: Aurora trasmettera' da sola, l'operatore resta responsabile, STOP TX e i limiti restano attivi, ARRL e DXCC non accettano QSO completamente automatici; solo con la conferma parte la richiesta con `accept_license_notice`. Tornare alla prova a vuoto non chiede conferma. Guida e tooltip aggiornati.
- **Automazioni e filtri di Attivita' di banda**: una stazione nascosta dai filtri di Attivita' di banda (continenti, proprio continente, paesi, nominativi, «mostra solo», solo CQ, non standard, gia' collegati; «sempre visibili» e «Bypass» compresi, come nell'elenco) non riceve mai una risposta automatica, anche in prova a vuoto; il registro scrive «nascosta dai filtri di Attivita' di banda». La pagina manda i filtri ad Aurora quando cambiano e all'avvio (`PUT /api/v1/band-filters`, letti con `GET`); al riavvio Aurora li rilegge dalle preferenze dell'interfaccia salvate (richiesta di ON7KEC, issue #6).
- **Rilascio**: pagina Contest in beta (filigrana BETA), automazioni con Modo reale, Mac con Hamlib incluso e aggiornamento dall'app.

## 0.11.201 - 2026-11-08
- **TCI come WSJT-X**: scelta del ricevitore della SDR nella scheda Radio («Ricevitore» RX1 o RX2, come «TCI Client RX1/RX2» di WSJT-X): frequenza, modo, PTT, split, misuratori e audio usano quello scelto; RX2 si accende con `rx_enable` se e' spento (collegamento pronto solo dopo la conferma) e si rispegne alla chiusura. Ogni comando di scrittura (frequenza, modo, PTT, VFO B e split, potenza, accordo) aspetta l'eco del server con i tempi di WSJT-X (frequenza 2 s, modo/PTT/VFO B 1 s, split 0,5 s), viene ripetuto una volta e poi da' un errore chiaro; una frequenza confermata a oltre 1 MHz da quella chiesta e' un errore; la frequenza e il modo gia' giusti non si rimandano e il modo voluto si rimette se la SDR lo cambia. Alla chiusura lo split acceso da Aurora si toglie prima di «Spegni».
- **TCI, audio a frequenze diverse da 48 kHz**: se il server chiede l'audio da trasmettere a 24, 12 o 8 kHz (`audio_samplerate`) l'onda viene ricampionata (filtro sinc a finestra, continuo tra un chrono e l'altro) e il tono resta dov'e'; in ricezione la cascata e il decodificatore accettano gia' ogni frequenza da 8 kHz in su.
- **TCI, PWR − / + e TUNE**: i pulsanti PWR del display (a passi del 5%) cambiano la potenza della SDR (`drive`) e TUNE accende e spegne la portante di accordo (`tune`); potenza e accordo si leggono dalla SDR. Guida «SDR con TCI» aggiornata.
- **Automazioni riordinate** (Impostazioni → Automazioni): testata sempre visibile con stato Accesa/Spenta, Prova a vuoto/Modo reale, risposte e QSO fatti del giorno, ultima azione e il pulsante Accendi/Spegni; sotto cinque blocchi a griglia (A chi rispondere, Chi scegliere prima, Limiti di sicurezza, Esclusioni, Quando non c'e' nessuno), ognuno con il suo «?» e un suggerimento su ogni comando; registro delle decisioni compatto e colorato (verde risposte e CQ, arancio saltati con il motivo, rosso bloccati).
- **Regole nuove nel motore delle automazioni**, salvate con le altre in `automation-rules.json` e valide anche in prova a vuoto (il registro dice cosa avrebbe fatto): «nuovo modo» (paese mai collegato nel modo in uso, lo stesso calcolo della scheda DXCC); al massimo N QSO all'ora (0 = nessun limite; contano i QSO registrati con stazioni chiamate dall'automazione o arrivate al suo CQ); attiva solo in una fascia oraria in ora locale (anche oltre la mezzanotte; il registro dice quando esce e quando rientra); paesi esclusi scelti dall'elenco DXCC (`GET /api/v1/automation/countries`); solo certi continenti; CQ da solo dopo N minuti senza CQ a cui rispondere, solo in modo reale, a trasmissione ferma e dentro la fascia: ogni CQ conta come una risposta di fila senza QSO, e se l'automazione si spegne (tetto, dead-man, operatore) con un suo CQ in onda la trasmissione si ferma. Di base tutto spento e nessun limite orario; STOP, dead-man e tetti restano. Interruttore del Band Hopping e collegamento alla scheda Sequenza per l'elenco delle bande. Guida aggiornata.
- **Modo**: il pulsante del modo si blocca solo mentre la radio trasmette, non con Abilita Tx acceso in ricezione.
- **Barra del periodo**: scorre continua invece che a scatti.

## 0.11.200 - 2026-11-08
- **Aggiornamento scaricato in anticipo**: quando il controllo trova una versione nuova, sul PC Aurora la scarica subito in sottofondo (con le ripetizioni gia' esistenti) e ne controlla la firma, senza installarla; il pacchetto resta pronto e «Installa» non lo riscarica (ne ricontrolla solo la firma sul file). L'installatore parte solo con il file completo e la firma valida; se lo scaricamento non riesce lo stato lo dice e si puo' riprovare. `GET /api/v1/update` ha il nuovo `stage` (fase, percentuale, MB, velocita', secondi mancanti, errore) e c'e' `POST /api/v1/update/download` per riprovare.
- **Barretta dell'aggiornamento nella testata**: durante lo scaricamento, al posto dell'icona, una barretta arancio compatta con «⬇ versione», la barra e «45% · 6 s»; a fine scaricamento diventa «Pronto · Installa e riavvia», in errore «Download non riuscito · Riprova». Un clic sulla versione apre il pannello di sempre (note, Piu' tardi).
- **Cartolina del tray con l'avanzamento**: «Aggiorna ora» non chiude piu' la cartolina, che mostra la barra con percentuale, MB scaricati e totali, velocita' e tempo che manca, e i passi Scarico → Controllo → Installo → Riavvio (quello in corso evidenziato, quelli fatti con ✓); se il pacchetto era gia' pronto salta a Installo. In errore mostra il motivo e «Riprova».
- **Documentazione**: memory con le decisioni del 2026-11-08; informativa privacy con il download anticipato degli aggiornamenti.

## 0.11.199 - 2026-11-08
- **Nuovo sistema di controllo «FT-991(A) Link (UDP)»** (Impostazioni → Radio, accanto a Hamlib, OmniRig e TCI) per le Yaesu FT-991/FT-991A collegate al programma FT-991(A) Link, che tiene la porta COM: Aurora manda i comandi CAT Yaesu via UDP (indirizzo di base 127.0.0.1:50001, modificabile), un comando per datagramma con risposta attesa 500 ms e una ripetizione. Legge frequenza (`FA`), modo (`MD0`) e PTT (`TX`) a intervalli, cambia frequenza e modo (DATA-USB per FT8), trasmette con il PTT via CAT (`TX1;`/`TX0;`), legge lo S-meter (`SM0;`) se il programma lo da' e lo ignora se no, «Accendi» e «Spegni» con `PS1;`/`PS0;`, si ricollega da solo. Solo VFO A: lo split e' sempre simulato (la frequenza si sposta solo in trasmissione). Se il programma non risponde lo stato dice di aprire COM Port e premere Start UDP in FT991Link; nuova scheda nella Guida.

- **Spot del DX Cluster sulla cascata** rifatti: etichette piccole e compatte (nominativo in carattere monospazio, sfondo scuro semitrasparente, bordo sottile giallo) nella parte alta della cascata, sotto le etichette Rx/Tx, e non piu' sopra lo spettro o il righello delle frequenze. Gli spot vicini si dispongono su al massimo tre righe senza sovrapporsi; quelli che non trovano posto finiscono in un «+N» che li elenca nel suggerimento. Una tacca sottile segna la frequenza esatta, il clic porta la ricezione li' come prima e il suggerimento mostra anche il commento dello spot.

- **Pagina Fox compatta**: la testata sta in due righe basse (nome e stato, i numeri della sessione in tesserine piccole, Posti, CQ verso, UltraFOX, Piu' CQ, Coda automatica e Azzera; sotto Fox, Tx, STOP, i comandi di ricezione, lo split e la verifica OTP), con gli stessi comandi di prima. Intestazioni dei riquadri e righe piu' basse e dense; la pagina riempie la finestra senza scorrere e lo spazio liberato va a «Chi chiama», ora il riquadro piu' grande (su 1880x950 circa 24 righe visibili invece di 8-9), con sotto la cascata, larga solo quanto lui. In mezzo QSO in corso, In onda, In coda e Fox log scendono fino in fondo alla pagina e si dividono l'altezza in parti uguali; «Messaggi ricevuti» resta a destra a tutta altezza. Le due maniglie regolano l'altezza della cascata.
- **KG4**: Guantanamo solo con il suffisso di due lettere; KG4ABC e simili sono Stati Uniti.
- **Trasmissione**: il CQ (Tx6) scritto a mano si salva in `tx6-cq.txt` e torna al riavvio; barra del periodo arancio e rossa in Tx.
- **Avvio lento della finestra** (WebView2 dopo un aggiornamento) non piu' segnalato come blocco del programma.
- **README**: tolte etichetta e sezione SignPath (domanda non accolta per ora).

## 0.11.198 - 2026-11-08
- **Attivita' di banda** si svuota sempre al cambio di frequenza della radio (almeno 1 kHz).
- **Impostazioni**: scegliendo un display la finestra non scorre piu' fuori schermo (restava solo lo sfondo sfocato).
- **Display quasi quadrati nella Stazione** (Yaesu ambra, Icom blu, Icom nero): la riga con modo, menu delle frequenze e Monitora/Hound/SWL non sparisce piu', resta a sinistra del display, e i pannelli partono sotto il display invece di salire sotto la testata (anche con «Dimensione del testo» ingrandita). Nella scelta del display (Impostazioni → Radio) il clic su un quasi quadrato non fa piu' scorrere via la finestra delle Impostazioni lasciando solo lo sfondo sfocato.
- **Impostazioni → Radio, «Apri Audio»** nel riquadro delle impostazioni consigliate ora porta davvero alla scheda Audio della stessa finestra (nel riquadro e nella finestra a parte); anche un nuovo indirizzo `#/impostazioni/<scheda>` con le Impostazioni gia' aperte cambia scheda.
- **Bandiere nelle liste**: tolta la cornice chiara che disegnava un rettangolo attorno alla bandiera; in Attivita' di banda e nel DX Cluster le bandiere sono circa il 22% piu' grandi, con il riquadro nelle proporzioni della bandiera e le righe allineate come prima.
- **Pagina Fox: una sola barra di comandi in alto**: sotto titolo, stato e contatori, Accendi Fox, Abilita Tx e STOP TX stanno nella stessa barra dei comandi di ricezione (Monitora, Hound, SWL, AGC, Filtro, Ferma, Decodifica, Cancella...) e dello Split, che prima erano sotto la voce «Cascata»; tutti i pulsanti hanno le stesse misure e lo stesso stile. Sulla cascata resta solo la sua barra degli strumenti.
- **Schermata di avvio aggiornata**: sotto il motto le sigle dei modi oggi supportati (FT8, FT4, JT65, JT9, Q65, MSK144) in pastiglie uguali; versione presa dal core o, se non risponde ancora, da quella dell'interfaccia (mai scritta a mano); righe centrate con una sola spaziatura fra gli elementi.
- **Scheda SDR nelle Impostazioni** (accanto a Radio e Audio): in un posto solo l'indirizzo del server TCI (lo stesso del CAT), lo stato del collegamento con la frequenza, l'audio in uso e il pulsante «Usa il TCI per tutto» che, dopo una conferma, mette il CAT su TCI e l'audio su «TCI (dalla radio)»/«TCI (alla radio)»; l'interruttore «Pagina QMAP nel menu» (spostato qui da Generali, spento di base) con la sorgente I/Q di QMAP e il pulsante per aprirla; le radio adatte e i collegamenti alle sezioni TCI e QMAP della Guida (`#/guida/<sezione>`). Dalla finestra a parte delle Impostazioni le pagine si aprono nella finestra principale, e le preferenze cambiate li' valgono subito anche nella finestra principale.
- **QMAP come WSJT-X 3**: nel minuto di 60 s cerca anche i Q65-30 nelle due meta' («+30 s»), decodifica gia' a 19,5, 30 e 49,5 s oltre che a fine minuto, salta le meta' in cui si trasmette; sincronismo controllato per ogni candidato prima del decodificatore e ritardo dell'eco lunare fino a 5,5 s; media dei periodi per ogni segnale («Media», con il pulsantino per cancellarla, ×N sui messaggi dalla media); Luna (Az/El) e Doppler della propria eco dal locatore delle impostazioni, con la frequenza «sked» di ogni messaggio.

- **Hound con SuperFox: verifica OTP** come WSJT-X: la riga `$VERIFY$ CALL 123456` che accompagna ogni pacchetto SuperFox/UltraFOX si nasconde (o si mostra con «Mostra i messaggi OTP») e il codice va alla stessa verifica in linea del Fox classico, con l'esito «CALL verified/invalid» a 750 Hz; la riga non va ai programmi collegati via UDP.
- **Etichetta Hound**: accanto a RX/TX una piccola etichetta «Hound» (o «Super Hound» quando si riceve un SuperFox), rossa finche' il Fox non e' verificato e verde dopo la verifica OTP, come in WSJT-X.
- **Fox: seme OTP salvato nel programma** (`fox-otp.json` nella cartella dei dati, tolto dal pacchetto per l'assistenza): torna da solo a ogni avvio, non solo quando si apre la pagina Fox; la pagina non lo rilegge (casella vuota = resta quello salvato, «Togli» lo cancella). Un seme rimasto nel browser passa al programma una volta.
- **Fox log su Club Log**: pulsante «Club Log» nel Fox log che mette i QSO della sessione nella coda dell'invio diretto a Club Log (serve Club Log acceso nelle impostazioni; i doppioni Club Log li tiene buoni).
- **Fox: QSO per continente** nei numeri della pagina Fox.
- **Q65 piu' sensibile e piu' veloce**: la decodifica di un periodo segue ora `q65_decode.f90` di WSJT-X (sincronismo a 1/8 di simbolo, soglia statistica dei candidati, frequenza di ricezione provata sempre, ricerca fine su mezzi bin e sedicesimi di simbolo, equalizzazione della banda) e, con il DX noto, prova la lista dei messaggi del QSO come la decodifica «q3». Su segnali di prova decodifica come `jt9` o meglio (fino a 1 dB in piu' senza Doppler) e costa 0,04-0,9 s per periodo invece di alcuni secondi (`docs/q65-confronto.md`).
- **Correzione urgente**: decodifica anticipata FT8 spenta; nella 0.11.197 un R+rapporto letto a 11,8 s si perdeva nella sequenza e si ripeteva il rapporto invece di RR73.
- **QSO senza rapporto** dell'altra stazione: non va piu' a log; «R + locatore» cliccato fa ripetere il nostro rapporto.
- **Cambio di banda**: il menu delle frequenze si riattiva quando la radio si collega dopo l'apertura; il nome della banda porta sulla frequenza principale.

## 0.11.197 - 2026-11-08
- **Fox (DXpedition)**: verifica OTP come WSJT-X (codice a 6 cifre dal seme di 16 caratteri: `CALL.123456` nel Fox classico, nel pacchetto con UltraFOX); niente trasmissione Fox sulle frequenze FT8 standard e nella sottobanda WSPR; ordinamento dei Hound da un programma esterno via UDP (AnnotationInfo); pagina Fox con ritmo dei QSO, doppio ordinamento e filtri dei chiamanti, distanza, prossimo pacchetto, parametri e Fox log salvabile in ADIF e Cabrillo.
- **Avvio**: appena la radio risponde (controllo ogni 0,3 s) va sull'ultima banda scelta, sulla frequenza principale del modo in uso; la banda si ricorda in `ultima-banda.txt`.
- **Q65: modello di evanescenza e «Cancella media»**: accanto a periodo e sottomodo di Q65 una tendina Lor/Gau (modello lorentziano come WSJT-X, o gaussiano) e un pulsantino che svuota la media dei periodi; la scelta resta dopo il riavvio.
- **Q65: confronto con WSJT-X** su segnali di prova in `docs/q65-confronto.md`.
- **Decodifica anticipata FT8** (come `m_earlyDecode` di WSJT-X): a 11,81 s una decodifica veloce del periodo non ancora finito mostra subito le stazioni forti; a fine periodo la decodifica intera aggiunge solo i messaggi nuovi.
- **Accendi/Spegni la radio**: 15 s di attesa per l'accensione (Hamlib aspetta la radio) e alla chiusura Aurora aspetta il core, cosi' lo spegnimento fa in tempo.
- **DX Cluster e Log**: colonne compatte, titoli in linea con i dati, paese separato.
- **TCI** (SunSDR/ExpertSDR, ANAN con Thetis): nuovo sistema di controllo nella scheda Radio con l'indirizzo del server (di base `127.0.0.1:40001`); frequenza, modo USB/USB dati e PTT via rete, ricollegamento automatico. Anche l'audio passa dal TCI, senza cavi audio virtuali: nella scheda Audio l'ingresso «TCI (dalla radio)» porta l'audio ricevuto a cascata, livello e decodificatore, l'uscita «TCI (alla radio)» manda l'onda alla SDR a ogni richiesta del server. S-meter, potenza e ROS dalla SDR; split vero sul VFO 1 o simulato; «Accendi» e «Spegni» accendono e spengono la SDR (`start;`/`stop;`).
- **Q65 completo**: periodi da 15, 30, 60, 120 e 300 s e sottomodi da A a E, scelti con due tendine accanto al selettore dei modi («Q65-60B») e ricordati alla riapertura; durata dei periodi e trasmissione seguono la scelta. Ricezione come WSJT-X: ipotesi AP del QSO (CQ, mio nominativo, nominativo del DX, RRR/73/RR73), metrica dell'evanescenza rapida per EME e scatter (tabelle lorentziane e gaussiane), media dei periodi della stessa parita' per i segnali troppo deboli per un periodo solo.
- **QMAP** (ricezione panoramica Q65 come WSJT-X 3): nuova pagina che decodifica tutta una fetta fino a 90 kHz da un SDR (flusso I/Q via TCI o file WAV I/Q), mostra lo spettro con i segnali trovati e con «QSO» porta la radio sul segnale in Q65 con periodo e sottomodo giusti. Da provare dal vivo con un SDR.
- **Installatore**: pagina «Dati che Aurora manda in rete» con i servizi e dove si spengono; firma gratuita con SignPath preparata (`.github/workflows/windows.yml`, `docs/signpath.md`), da attivare.
- **UltraFOX**: il codice polare si chiama AQP-127 (`docs/aqp-127.md`); le posizioni del messaggio si ricalcolano con `examples/tabelle_qpc.rs`.
- **Accendi e Spegni la radio** (come JTDX, `RigPower`/`RigPower_off`): due caselle nella scheda Radio con Hamlib; `\set_powerstat 1` al primo collegamento, `\set_powerstat 0` alla chiusura di Aurora.
- **Band Hopping con MSK144** («MSK144 6m»).
- **Agente nel tray**: «Cerca aggiornamenti» risponde sempre, anche quando GitHub non risponde (prima nessuna finestra).
- **Misura della decodifica FT8** su periodi veri (`ft8/examples/misura_wav.rs`): banda piena, profondita' 3, 1,7-2,8 s.
- **Installa e riavvia**: barra di download come quella di un browser (percentuale, MB, velocita' e tempo che manca); il motore dice l'avanzamento in `GET /api/v1/update` (`download`).
- **Pannello Log stretto**: le colonne non escono piu' dal riquadro (sotto 380 px si toglie Paese, sotto 300 px anche Modo e Rapporti).
- **Elementi nascosti o staccati**: le finestre vicine si allargano da sole a riempire il posto lasciato libero (`fillGaps`), poi si possono ritoccare.
- **Impostazioni**: Dati della stazione su tre colonne allineate con la dimensione del testo a segmenti; Finestre con una tessera per elemento e il pallino del posto.

## 0.11.196 - 2026-11-08
- **Maniglie per allargare le finestre**: una sola per ogni confine, che allarga e stringe insieme le finestre delle due parti (per esempio DX Cluster con Bande aperte e Comandi); c'e' solo dove allargando nessuna finestra finisce sotto un'altra.
- **AutoSeq**: suggerimento che spiega cosa fa acceso e spento (anche la stazione che chiama preparata come prossimo DX, senza trasmettere).
- **README**: macOS nei badge e link diretto al `.dmg`; badge di Tauri.
- **Aggiornamento silenzioso**: dall'app l'aggiornamento si installa senza aprire la finestra dell'installatore e Aurora si riapre da sola.
- **Installatore**: grafica nuova con tutti i modi (FT8, FT4, Q65, MSK144, JT65, JT9, JT4), Contest e DXpedition; con una versione piu' vecchia gia' installata la scelta di base e' l'aggiornamento («Non disinstallare»); la disinstallazione non cancella mai i dati (tolta la casella «Elimina i dati»). Modello NSIS proprio (`desktop/src-tauri/installer/installer.nsi`, da quello di Tauri 2.12.4).
- **Finestre sempre davanti**: aperta dal tray (o da un'altra istanza) la finestra di Aurora, con l'avviso della versione nuova, viene sopra a tutte le altre: l'agente cede il primo piano (`AllowSetForegroundWindow`) e la finestra resta un attimo «sempre in primo piano».
- **Righe colorate leggibili al passaggio del mouse** (QSO in corso in giallo, mio nominativo in giallo o ciano).
- **CQ e novita'**: paese e ora dell'ultima ricezione nelle caselle, DXCC nuovo che lampeggia; avviso in evidenza del DXCC nuovo con suono proprio («Allarme DX raro»), regolabile in Notifiche.

## 0.11.195 - 2026-11-08
- **LogHX**: tipo pronto in Programmi collegati (flusso WSJT-X sulla 2237 con Logged ADIF) e voce della Guida rifatta dal registro delle versioni di LogHX 3.1: impostazione di JTDX in LogHX, «Active calls» con «Accetta richieste UDP», multicast o rebroadcast con GridTracker, CAT condiviso con OmniRig.
- **Band Hopping**: tasto destro sul selettore delle frequenze (era sui pulsanti delle bande, tolti).
- **Stazione**: interruttori di ricezione in una barra a segmenti (acceso = verde), modo, frequenza e interruttori su una riga.
- **Scaricamento dell'aggiornamento piu' robusto**: con un errore del server di GitHub (5xx, 429) o la rete che cade si riprova tre giri, alternando l'indirizzo pubblico e l'API di GitHub (un utente aveva solo «504» sull'indirizzo pubblico).
- **macOS**: `.dmg` universale compilato da GitHub a ogni release (non firmato da Apple).
- **Aggiornamento non riuscito**: la pagina dice il motivo (scaricamento, salvataggio, firma, avvio dell'installatore) invece del solo `install_failed`, e l'errore arriva anche nei rapporti.
- **Q65-30A** nel motore proprio: ricezione e trasmissione (`jt::q65`: codice QRA 15,65 su GF(64) con passaggio di messaggi, CRC-12, messaggi a 77 bit come FT8), selettore dei modi, frequenze di WSJT-X (6 m 50,275 e 2 m 144,180 MHz; 50,211 e 144,116 EME come alternative). Il decodificatore supporta anche i periodi 15-300 s e i sottomodi A-E, non ancora selezionabili.

## 0.11.194 - 2026-11-08
- **Corretto**: con la radio collegata, un cambio banda che la radio non confermava in tempo scriveva la frequenza «a mano» e la radio rimbalzava fra due bande (difetto della 0.11.193).
- **JT4** (sottomodo A) nel motore proprio: ricezione, trasmissione e selettore dei modi (`jt::jt4`, Fano con la metrica di `getmet4`).
- **Frequenze**: tendina con tutte le frequenze del modo in uso, banda per banda, come in WSJT-X (standard, alternative, DXpedition); aggiunte FT4 e JT9 a 60 m e 28,091 MHz FT8 dalla tabella di WSJT-X.
- **Display della radio**: i tasti non diventano grigi al passaggio del mouse.

## 0.11.193 - 2026-11-07
- **Radio senza CAT**: nuovo sistema di controllo «Nessuno» in Impostazioni > Radio. Corretto: con il motore proprio i pulsanti delle bande non cambiavano mai la frequenza senza radio (il `Rig` del motore nascondeva la frequenza a mano, `AppState::dial_rig`); se la radio scelta non risponde le bande cambiano comunque la frequenza di Aurora.
- **Radio**: «Applica» sempre attivo.
- **Log**: in «Aggiungi QSO» il modo si sceglie da una tendina completa (digitali, fonia, CW, altri); il pulsante «+ Aggiungi QSO» sta accanto al titolo.

## 0.11.192 - 2026-11-07
- **Impostazioni**: «Salva» sempre attivo; senza modifiche in sospeso dice «Tutto salvato» (molte voci si salvano da sole appena cambiate, e il pulsante spento sembrava guasto).

## 0.11.191 - 2026-11-07
- **Bande senza CAT**: senza radio comandabile i pulsanti delle bande (e il doppio clic sugli spot) impostano solo la frequenza di Aurora, ricordata al riavvio (`frequenza-a-mano.txt`, `ManualDialHook`, `manual_dial` in `/api/v1/station-info`); la radio si sposta a mano. Voce nuova della Guida «Senza CAT: la banda a mano».

## 0.11.190 - 2026-11-07
- **Log**: «Aggiungi QSO» (QSO scritto a mano: nominativo, data e ora UTC, banda o frequenza, modo, rapporti, locatore, nome, commento; va a log come gli altri, anche ai programmi collegati, con `Event::QsoLogged`), «Elimina selezionati» e «Svuota il log» con conferma. I QSO cancellati spariscono anche da `aurora_log.adi` (`logbook::adif_without`); svuotando, il file si mette da parte con la data. API `POST` e `DELETE /api/v1/log/qsos`, solo dalla rete di casa.
- **Guida di BBLogger** riscritta dai documenti ufficiali (aribusto.it): interfaccia Wsjtx con Msg ID 5 e 12, «UDP 12» e «Auto-Save», JTAlert sulla 2334; nuova voce per il modo file ADIF con `aurora_log.adi`.

## 0.11.189 - 2026-11-07
- **Dimensione del testo** (issue #3): Impostazioni > Generali, da 90% a 160% per tutta l'interfaccia (`zoom` sulla radice, `lib/uiScale.ts`), anche con Ctrl +, Ctrl - e Ctrl 0; vale per ogni dispositivo a parte (`aurora-ui-scale` tra le preferenze proprie del telefono).
- **Impostazioni** (issue #4): «Salva» salva e lascia aperta la finestra; il pulsante Applica non c'e' piu'; «Chiudi» e la X chiedono se scartare le modifiche non salvate (prima la X le scartava senza avvisare).

## 0.11.188 - 2026-11-07
- **Comandi**: messaggi come toast (niente `live-note` sotto i pulsanti), nessun avviso per frequenze e interruttori; pulsanti `flex: 0 1 auto` che si stringono invece di uscire dal bordo.
- **README**: badge VirusTotal e analisi antivirus degli installatori.

## 0.11.187 - 2026-11-07
- **Comandi**: Tune di nuovo disegnato (`prefs.cmdTune`, rosso mentre trasmette), Spot DX su DXSummit (`postSpotDxsummit`, solo con `AllowSpotsDXSummit`), Log QSO solo con `prefs.cmdLogQso` (prima sempre visibile); sei pulsanti in una riga.
- **Mappa DX**: linea da me al DX con Abilita Tx o in trasmissione; legenda in una riga.
- **Guida**: pannello Comandi descritto com'e' oggi; aiuto DXSummit con il posto giusto del pulsante.

## 0.11.186 - 2026-11-07
- **Schede staccate**: con almeno una scheda della Stazione staccata (o nascosta) la pagina si sblocca: spostamento e ridimensionamento delle altre, misure salvate a parte per insieme di schede staccate (`aurora-board-staccate`); rimesse tutte, torna la disposizione bloccata.
- **Menu per staccare** mai tagliato dalla scheda (`:has(.dock-detach-menu)`).
- **Bande aperte staccata**: nuovo elemento `bands` (prima staccava `dxmap` e apriva la Mappa DX; la Mappa DX staccata toglieva Bande aperte dalla Stazione).

## 0.11.185 - 2026-11-07
- **Coda**: con un DX impostato le chiamate vanno sempre in coda (`QueueReply` non risponde piu' da solo), massimo 4 (`QUEUE_MAX`), colonna accanto al DX.
- **Comandi** in un blocco della disposizione a se' (`cmds`); le disposizioni salvate dividono lo spazio di Bande aperte.
- **Trasmissione**: doppio clic su RX = Canc DX; riga del DX ad altezza fissa.
- **Bande aperte**: colonne Giorno/Notte fisse e allineate, icone sole e luna, fino a 5 stazioni lontane se entrano.
- **Interfaccia**: barre del titolo con l'aurora pastello, DXpedition in cima dopo 15 s, piccole correzioni.

## 0.11.184 - 2026-11-07
- **Chiusura RR73 come WSJT-X**: dopo il nostro RR73 a log, senza novita' dall'altro, Aurora salta il periodo invece di ripeterlo (`hold_slot`); risponde se ripete il R+rapporto, chiude al 73, si ferma dopo 3 periodi.
- **Ripresa del QSO**: doppio clic o Wait and Reply con lo stesso DX ripartono dal messaggio scelto (caso BG2EFX).
- **Wait and Reply / Wait and Call spenti di fabbrica**.
- **Interfaccia**: le 3 piu' lontane con colonne fisse e paese, pie' di pagina piu' leggibile, logo con l'aurora che scorre, scelta della lingua tolta dalla barra in alto.

## 0.11.183 - 2026-11-07
- **Chiusura con RRR come WSJT-X** (verificato sui sorgenti 2.7 e 3.0.1, `message_is_73`): mandare RRR non registra piu'; si registra al suo 73 (`Status::R73`) o, se non arriva, all'ultimo tentativo di chiusura o dopo 5 risposte RRR al suo R+rapporto ripetuto (`RRR_REPEATS`, ora contato anche prima della registrazione). RR73 e 73 invariati.

## 0.11.182 - 2026-11-07
- **Messaggi dell'autore, conteggio**: Aurora e l'agente nel tray dicono al servizio «cartolina mostrata» (`POST /v1/messaggi/<id>/visto`, codice anonimo dell'installazione, solo con il conteggio delle installazioni attive acceso); la pagina dei messaggi mostra «Visto da N · M attive potevano riceverlo» (M dalla tabella `attivi`). Informativa privacy aggiornata.

## 0.11.181 - 2026-11-07
- **QSO a log**: nuovo effetto sulla riga del Log (scelta del proprietario, proposta 2a): aurora viola-magenta-azzurro-verde che scorre in uno strato `.qso-fx` sotto la riga, due scie viola sul bordo, nominativo viola, «A LOG» in fondo; tolti il verde fluo e il giallo.
- **Statistiche**: alla prima apertura le schede (numeri, Per banda, scheda DXCC, paesi mancanti) sono subito al loro posto con «…»; prima la scheda DXCC compariva a sinistra e poi saltava nella sua colonna.

## 0.11.180 - 2026-11-07
- **Messaggi dell'autore, traduzione**: pulsante «Traduci in tutte le lingue» nella pagina dei messaggi; il servizio `aurora-rapporti` traduce dall'italiano con Workers AI (Llama 3.3, istruito sul gergo radioamatoriale) in `POST /v1/messaggi/traduci`, solo con ADMIN_TOKEN e con il freno ai tentativi; le traduzioni si ricontrollano nella cartolina prima di pubblicare.

## 0.11.179 - 2026-11-07
- **Messaggi dell'autore, temi**: cinque cartoline (Natale con il cappellino sull'angolo, Anno nuovo, Pasqua, Aggiornamento, Servizio) in Aurora e nell'agente nel tray (`AURORA_AGENTE_PROVA=natale` per provarle); il tema viaggia nel campo `stile` del servizio `aurora-rapporti` (colonna D1 nuova), le versioni vecchie mostrano la cartolina normale.
- **Pagina dei messaggi**: modulo a sinistra e cartolina d'anteprima sempre visibile a destra con scelta della lingua; i modelli sono gia' scritti in tutte e sei le lingue e si chiamano nella lingua dell'interfaccia; scelta del tema a mano.
- **Pannello Log**: scheda DXCC un po' piu' piccola (nominativo e paese, bandiera, campo Prova, caselle).

## 0.11.178 - 2026-11-07
- **Attivita' di banda**: le mie trasmissioni con lo stesso testo (il CQ ripetuto ogni periodo) restano una riga sola, la piu' recente, con «×N»; quelle diverse del QSO restano tutte (scelta del proprietario: JTDX le mostra tutte).
- **QSO in corso**: oltre alla riga ferma in cima resta in lista l'ultima risposta della stazione ricevuta da me (prima spariva quando l'ultimo messaggio era il mio).

## 0.11.177 - 2026-11-07
- **Audio che perde blocchi**: l'avviso ha «Come risolvere» (anche un clic sul testo) che apre una scheda con i sei controlli da fare sul PC, in 6 lingue, su due colonne sugli schermi larghi.

## 0.11.176 - 2026-11-07
- **Agente, «Aggiorna ora»**: se Aurora non conosce ancora la versione nuova (ultimo controllo di prima della pubblicazione) la chiede subito a GitHub invece di rispondere «nessuna versione nuova trovata».

## 0.11.175 - 2026-11-07
- **Messaggi agli utenti**: modelli pronti, anteprima della cartolina sul PC dell'autore, contatori dei caratteri, conferma prima di pubblicare con destinatari e scadenza, «Usa di nuovo» nell'elenco dei messaggi.
- **Icona dei messaggi**: all'avvio Aurora non riporta piu' alla Stazione chi apre `#/messaggi-autore`.

## 0.11.174 - 2026-11-07
- **Agente: «Aggiorna ora» installa davvero** la versione nuova (prima apriva solo Aurora): chiede ad Aurora di scaricare e installare; con Aurora chiusa la apre e aspetta che risponda.
- **DXpedition sentite**: riga «SENTITA DA TE» con frequenza esatta, modo, segnale e ora dell'ultimo messaggio ricevuto; doppio clic sulla spedizione la chiama come in Attivita' di banda (un clic apre la scheda).

## 0.11.173 - 2026-11-07
- **Filigrana PROVA LOCALE** solo nella prova nel browser, mai nella finestra del programma installato (nemmeno con la grafica di prova).
- **Prova nel browser** (`vite.prova.config.ts` fuori dal repository, icona «Aurora - Prova locale»): prende la configurazione del PC e non la riscrive.

## 0.11.172 - 2026-11-07
- **Finestra dell'aggiornamento**: con le Impostazioni aperte dentro la finestra principale restava nascosta e non cliccabile (la pagina sotto e' disattivata, `inert`, e la locandina stava li' dentro); ora e' fuori dalla pagina, sopra le Impostazioni.
- **Agente nel tray**: la cartolina non si apriva con Aurora aperta (WebView2 rifiutava la finestra: stessa cartella dati con impostazioni diverse, errore 0x8007139F); ora l'agente ha la sua cartella (`%LOCALAPPDATA%\Aurora\agente-webview`) e la pagina della cartolina e' un file temporaneo. Registro dell'agente in `%TEMP%\aurora-agente.log`; `AURORA_AGENTE_PROVA=1` mostra una cartolina di prova all'avvio.
- **Scheda DXCC elastica**: sempre larga quanto il pannello (prima una colonna fissa di 408 px usciva dal riquadro, campo «Prova» compreso).

## 0.11.171 - 2026-11-07
- **Riga Tx con etichetta**: la tua riga di trasmissione in Attivita' di banda porta l'etichetta della stazione chiamata (presa dal suo ultimo messaggio ricevuto).

## 0.11.170 - 2026-11-07
- **Errori della radio**: il messaggio dice il motivo vero mandato dal core (es. «la radio non ha raggiunto la frequenza»); tolto il vecchio «controlla OmniRig» che compariva anche con Hamlib.
- **Potenza**: sopra il 98,5% si mostra 100% (Hamlib legge l'FT-2000 a 100 W come 0,992).
- **Agente**: «Domani parte…» solo per le DXpedition di paesi non ancora lavorati (elenco salvato da Aurora; senza elenco o vecchio di 7 giorni, per tutte).
- **Messaggi agli utenti** (solo per l'autore): pagina di Aurora fuori da ogni menu (`#/messaggi-autore`), aperta da un'icona sul desktop dell'autore, per scrivere, vedere e togliere i messaggi push; il segreto resta solo su quel PC; tolta la pagina web del servizio; il servizio frena i tentativi ripetuti.

## 0.11.169 - 2026-11-07
- **Agente nel tray** (nuovo): Aurora avviata con `--agente` resta accanto all'orologio anche chiusa (icona sempre visibile), apre Aurora con un clic e mostra la cartolina con il suono per le versioni nuove (sempre) e i messaggi dell'autore; a scelta Propagazione (tempesta geomagnetica, 12-10 m aperti), DXpedition che partono domani e riepilogo di ieri. Controlla all'avvio e ogni ora; con Aurora aperta non ripete gli avvisi. Acceso di fabbrica e in avvio con Windows; si spegne dalla pagina «Agente nel tray» delle Impostazioni (o `AURORA_AGENTE=0`). Prima di un aggiornamento Aurora lo chiude.
- **DXpedition in onda**: salgono in cima all'elenco finche' sono spottate (15 minuti), con chi le ha spottate e il modo (dal cluster, dal commento o dalla frequenza); doppio clic sulla frequenza porta la radio li'.
- **Attivita' di banda e DX Cluster**: tolti l'aggancio alle righe e l'«ancora» dello scorrimento, che facevano scendere la lista a ogni riga nuova in cima; a ogni periodo (o spot) nuovo la lista torna in cima. Le decodifiche di un periodo si aggiungono tutte insieme (un solo ridisegno).
- **QSO a log al primo RRR** (come con RR73); se l'altro ripete il rapporto, al massimo 5 RRR poi stop; niente doppio log al suo 73.
- **Log immediato**: il QSO appena messo a log compare subito in cima al pannello Log, prima che Log4OM lo scriva. Riga, suono e lampeggio partono nello stesso istante: verde fluo che pulsa per 5 s con una luce che scorre, poi si dissolve.
- **Comandi dentro Bande aperte** (al posto di continenti e record): Tx, Rx e Rapporto su una riga; Hold Tx Freq, Tx/Rx Split, Log QSO e Canc DX in una riga. Tolti dal pannello Trasmissione i pulsanti e la scritta Hound.
- **Potenza RF ricordata**: quella scelta con + e − si rimette a ogni collegamento della radio; avviso se la radio la rifiuta.
- **DXpedition in onda**: spottata negli ultimi 15 minuti, riga arancione che lampeggia e «IN ONDA» con frequenza e modo.
- **Avviso senza CAT**: provando a trasmettere senza radio collegata compare un avviso (la ricezione resta libera).
- Trasmissione senza la riga sotto il DX; bandiera di Attivita' di banda piu' grande.
- **Coda nella Trasmissione**: accanto al blocchetto giallo del DX, tra il DX e RX/TX, su una riga (al massimo 3 nominativi e «+n»); i messaggi non scendono piu'.

## 0.11.168 - 2026-11-07
- **Coda**: le stazioni accodate hanno l'etichetta «IN CODA n» (fondo azzurro) e restano ferme sotto la stazione chiamata, nell'ordine della coda; dopo il log salgono di un posto.
- **Attivita' di banda**: il ritorno in cima dopo 15 s cerca il contenitore che scorre davvero (lista, pannello o blocco), quindi funziona anche nella pagina a blocchi.
- **Aggiornamenti**: «Controlla ora» dalle Impostazioni (finestra sempre davanti) apre la locandina li', non dietro nella finestra principale.
- **Messaggi dell'autore**: Aurora legge ogni ora i messaggi pubblicati dal servizio dei rapporti (`/v1/messaggi`) e li mostra una volta sola come cartolina con il suono (tema festa per gli auguri).

## 0.11.167 - 2026-11-07
- **Prestazioni**: undici componenti leggono solo i campi dello stato che usano (`useStatusPick`), la Stazione non passa piu' lo stato intero a Trasmissione, Comandi e barra di stato, DX Cluster/Log/Bande aperte sono `memo`: modifiche alla pagina da 749 a 405 ogni 15 s (DX Cluster da 407 a 31). Striscia CQ a 20 passi/s, S-meter ridisegnato al massimo 25 volte/s, barra in basso 2 volte/s; niente aggiornamenti a finestra nascosta.
- **Aggiornamenti**: «Controlla aggiornamenti» apre sempre la locandina (anche se rimandata); se si e' gia' aggiornati dice «Aurora e' aggiornata» con le novita' della versione installata.
- **Trasmissione senza CAT**: avviso «non puoi trasmettere, la radio non e' collegata» (la ricezione senza CAT non e' mai limitata).
- **Finestre staccate**: nascono grandi quanto il pannello sullo schermo, mai a tutto schermo; ridimensionabili.
- **Bande aperte**: record di distanza piu' visibili (nominativo in oro, km in verde).
- **Aggiornamenti**: controllo automatico ogni 3 ore di fabbrica (prima 6).

## 0.11.166 - 2026-11-07
- **Attivita' di banda**: il ritorno all'ultima stazione dopo 15 s funziona davvero. Il conto riparte solo quando scorre la persona (rotella, dito, tasti, barra); gli spostamenti della lista all'arrivo delle stazioni nuove (ogni 15 s) lo facevano ripartire all'infinito. Il ritorno e' immediato (lo scorrimento morbido veniva annullato).
- Prova locale: la cartolina dell'agente nel tray suona un accordo crescente (con una nota in piu' per gli aggiornamenti).

## 0.11.165 - 2026-11-07
- **Tema automatico**: il pulsante del tema passa da Scuro a Chiaro ad Automatico (segue Windows e cambia con lui).
- **Log**: dopo un QSO il pannello rilegge Log4OM subito e poi dopo 1,5, 4 e 8 s (prima fino a 10 s di ritardo); caratteri del pannello 0,7 px piu' piccoli.
- **Attivita' di banda**: etichette piu' strette (4,3rem, 8,5 px) e centrate; lo spazio va al messaggio e a bandiera e paese (paese 11 px, bandiera piu' grande).
- **Schede**: bordo viola di Aurora.
- **Tema chiaro**: la riga in trasmissione ha il testo scuro (prima bianco, illeggibile).
- **Rapporti dei crash leggibili subito**: insieme al pacchetto di un crash parte un riassunto (l'inizio del rapporto di testo, gia' ripulito) che il servizio salva nel suo indice: la causa si legge senza scaricare lo zip.
- **Impostazioni**: titolo «Funzioni di Aurora» e nome della scheda aperta piu' evidenti.
- Prova locale: filigrana «PROVA LOCALE» e pagina «Agente nel tray» (prototipo) solo nella grafica compilata con `VITE_AURORA_PROVA=1`.

## 0.11.164 - 2026-11-06
- **Pagine che scorrono**: su schermo largo solo la Stazione resta tutta nella finestra; Fox, Propagazione e le altre pagine scorrono di nuovo fino in fondo (la parte sotto restava nascosta).
- **Splash**: mostra l'autore completo.
- **Bande aperte**: record di «Oggi» solo dagli spot di oggi (UTC), «7 giorni» di calendario, archivio dei record rovinato non fa più cadere la scheda; continenti più precisi (Alaska e Aleutine in NA, Azzorre in EU, Panama in NA).
- **Conferme**: la finestra non si chiude due volte con Esc e clic insieme.
- Tolto codice non più usato (mini-mappa intera di WorldMap, calcolo `--foxc-top` della Fox).

## 0.11.163 - 2026-11-06
- **Pulsante del modo** (FT8, FT4...): il modo scelto e' ben in evidenza (bordo pieno, alone, testo marcato).

## 0.11.162 - 2026-11-06
- **Bande aperte**: al posto della mini-mappa, «Continenti che ti ascoltano» (stazioni PSK Reporter per continente, stimato dal locatore) e «Record di distanza» di oggi e degli ultimi 7 giorni.
- **Attivita' di banda**: se si scorre e poi la si lascia per 15 secondi torna da sola all'ultima stazione ricevuta (ogni scorrimento fa ripartire il conto).
- **Liste e tabelle che scorrono** (Attivita' di banda, Log, DX Cluster, DXpedition, Fox): si fermano all'inizio di una riga, senza righe tagliate.
- **Trasmissione**: pulsanti QSO tornati alla misura originale; i cursori Rx e Pwr restano sempre visibili in fondo al pannello.
- **Mappa piccola intera** (`WorldMap` con `whole`): tutte le terre dentro il riquadro e cartina ripetuta ai lati.

## 0.11.161 - 2026-11-06
### Corretto
- Trasmissione: i cursori Rx e Pwr restano sempre visibili in fondo al pannello anche quando è basso (0.11.160: potevano finire sotto il bordo); distanze fra i pulsanti a 6 px.

### Modificato
- Statistiche: riquadri dei numeri alti uguali, «Per banda» con numeri allineati a destra, scheda DXCC a tutta larghezza.

## 0.11.160 - 2026-11-06
### Aggiunto
- Bande aperte: mini-mappa NASA di chi ti ascolta (PSK Reporter) con le linee dalla propria stazione e il collegamento alla Mappa DX.
- Fox: Tx/Rx Split nella cascata.

### Modificato
- Bande aperte: Giorno e Notte con caselle uguali, tabella a tutta larghezza.
- Mappa DX: planisfero sempre intero in altezza (da +82° a -60°), ripetuto ai lati.
- Fox: cascata uguale a quella della Stazione (barra, filtro, spot, righello); «Messaggi ricevuti» alto quanto la colonna con la cascata.
- Trasmissione: pulsanti QSO più grandi (25 px, 10,5 px), 10 px fra i pulsanti, cursori più in basso.
- Stazione: display allineato al bordo destro di Bande aperte e centrato anche con la pagina a blocchi; nazione a 9,2 px, etichette degli stati a 9 px; la scritta del Filtro solo nel suggerimento.
- Impostazioni: Aggiornamenti e Dati della stazione riordinati; la cartella dei crash passa in Assistenza.
- Conferme (Prova PTT, storico, copia di sicurezza, cambio banda dal telefono) nella grafica di Aurora invece di quella del browser.

### Corretto
- Prova CAT: aspetta l'esito del collegamento (prima diceva «non risponde» mentre si collegava).
- Prova PTT: aspetta l'inizio del periodo di trasmissione (prima spegneva il Tune prima che partisse).

## 0.11.159 - 2026-11-06
- **Crash corretto**: accendendo la radio dopo Aurora, la ripresa automatica dell'audio faceva partire il decodificatore fuori dal suo motore asincrono e Aurora si chiudeva («there is no reactor running», 0.11.155-0.11.158).
- **Uscita audio**: un attimo di ritardo della scheda (underrun) non la dava piu' per persa; prima la trasmissione seguente falliva con «il dispositivo non risponde piu'».
- **Blocchi della finestra**: il salvataggio della posizione delle finestre non si fa piu' sul thread della finestra, e spostare o ridimensionare a lungo non e' scambiato per un blocco.
- **Guida**: il doppio clic durante la trasmissione cambia il messaggio al volo (dalla 0.11.157).

## 0.11.158 - 2026-11-06
### Modificato
- Attività di banda e CQ: colonne a 10,5 px (km a 9,5 px), niente linea fra bandiera e paese, paese accostato alla bandiera, colonna dei km a larghezza fissa e allineata, più spazio alle etichette.
- Display a frontalino (FT-2000, IC-7610) nella Stazione: 10 px più alto e centrato fra barra del menu e pannelli.

## 0.11.157 - 2026-11-06
### Modificato
- Doppio clic su un'altra stazione durante la trasmissione: nello stesso periodo il messaggio in onda si sostituisce al volo (niente arresto del Tx); nel periodo opposto la trasmissione si ferma come prima.
- Attività di banda e CQ: caratteri più piccoli, bandiera e colonna dei km strette, più spazio ai messaggi.

## 0.11.156 - 2026-11-06
### Modificato
- Pagina principale a blocchi con misure fisse: Attivita' di banda, Log, DX Cluster, Bande aperte, Trasmissione, CQ e novita', Cascata. Si scambia di posto a destra e a sinistra trascinando la barretta del titolo; tra due finestre vicine un divisore stringe una e allarga l'altra.
- Trasmissione contiene anche i comandi del QSO (compatti) con i cursori Rx e Pwr in una riga; Abilita Tx e AutoTX accesi sono rossi. La scheda QSO separata non c'e' piu'.
- CQ e novita': i CQ in una fascia orizzontale su due righe che scorre da sola, sotto le DXpedition su una riga (anche quelle in arrivo fino a 120 giorni).
- Cascata: barra dei comandi, righello con le parentesi Tx e Rx, spettro sempre visibile sotto la cascata e a sfondo nero.
- Mappa DX: solo gli sfondi della NASA (Blue Marble di giorno, City Lights di notte).
- DX Cluster: colonne di larghezza fissa e allineate, Commenti visibile.
- Barra di stato in fondo con l'ora UTC, le decodifiche e il periodo.
- Impostazioni in una finestra a parte (1040x760): 15 schede, Prova CAT e Prova PTT, Annulla, Applica e Salva; le voci del motore partono solo con Applica o Salva.

### Aggiunto
- Bande aperte: condizioni di propagazione per giorno e notte e le tre stazioni piu' lontane che ti hanno ascoltato (PSK Reporter).
- I sei messaggi Tx si possono scrivere a mano (Invio o uscita dal campo li conferma, Esc annulla); il motore li tiene finche' non vengono rigenerati.
- Analisi a priori (AP) anche in FT4 (nominativo, DX, CQ) e collegata al motore proprio anche in FT8.
- Elenco dei nominativi conosciuti (log + stazioni ascoltate, salvato in `known_calls.txt`): filtra i falsi decode deboli, attivo da 300 nominativi.
- Controllo del locatore: scarta i decode deboli o AP con un quadrato troppo lontano dal paese del nominativo (oltre 5500 km dal centro).

### Corretto
- Con Abilita Tx acceso, Wait and Pounce non si arma: ora il pulsante e' grigio e lo spiega.
- Il pulsante Ripristina non sposta piu' le finestre.

## 0.11.155 - 2026-11-05
### Corretto
- Mappa DX: stazioni in onda, quadrati QSX/CQ/CQDX/QRZ e linee «Chi chiama chi» non comparivano (l'ora dei messaggi è in ms dalla mezzanotte UTC, era confrontata con l'ora completa).
- Mappa DX: il menu per staccare la mappa non resta sotto la cartina.
- Mappa DX: la notte è più scura (predefinito 0,75; i valori salvati più chiari delle prime versioni si alzano una volta sola).

## 0.11.154 - 2026-11-05
### Modificato
- La vecchia «Mappa» esce dal menu (doppione della Mappa DX): `#/mappa`, le finestre staccate e l'ultima pagina ricordata aprono la Mappa DX; tolta anche da Guida e Impostazioni › Finestre.

## 0.11.153 - 2026-11-05
### Corretto
- Radio configurata ma spenta: il display resta (spento, «Radio non risponde») invece del riquadro «Radio non collegata – Clicca per configurarla», che ora compare solo se la radio non è mai stata configurata.

## 0.11.152 - 2026-11-05
- **Linux**: il cursore Pwr ora abbassa davvero il segnale (prima si trasmetteva sempre al massimo, perche' il volume di Windows li' non c'e'); niente pulsante «Installa» degli aggiornamenti (era l'installatore di Windows), resta il link alla release con AppImage e .deb; OmniRig non e' piu' proposto tra i CAT.
- **Radio spenta e riaccesa durante l'uso**: se la scheda audio della radio sparisce (radio spenta, cavo USB mosso) ricezione e trasmissione si riprendono da sole appena torna, senza riavviare Aurora; intanto un avviso spiega cosa succede.
- **Blocchi del programma**: la finestra «Salva con nome» aperta a lungo non e' piu' scambiata per un blocco; il rapporto di un blocco dice cosa stava facendo la finestra e quanto e' durato.

## 0.11.151 - 2026-11-05
### Aggiunto
- Mappa DX: linee dalla propria stazione a chi l'ha ricevuta (PSK Reporter), con interruttore e colore nel menu.

### Modificato
- Mappa DX: i quadrati delle stazioni sentite restano ricordati finché Aurora è aperta, così le linee «Chi chiama chi» compaiono anche senza quadrato nell'ultimo messaggio.

## 0.11.150 - 2026-11-05
### Aggiunto
- Mappa DX: propagazione (mappa di calore e quadratini PSK Reporter, Spot RX, durata massima), linee «Chi chiama chi» tratteggiate e animate personalizzabili, centro del paese se manca il quadrato, quadrati in onda QSX/CQ/CQDX/QRZ divisi in triangoli con il log, legenda; puntini delle stazioni facoltativi.
- Guida: voce «Mappa DX».

## 0.11.149 - 2026-11-05
### Aggiunto
- Mappa DX (menu): sfondi, luminosità, colore di fondo, notte automatica, linea grigia con crepuscoli, Sole e Luna, stazioni in onda colorate che sbiadiscono, linee ad arco animate, quadrati lavorati/confermati per banda e modo, clic con quadrato, distanza, paese e zone CQ/ITU; staccabile.
- `/api/v1/map/worked` legge gli stessi log della scheda DXCC e restituisce anche `cells` (quadrato, banda, modo, conferme).

### Corretto
- Display staccato: niente barre di scorrimento quando lo si ridimensiona.

## 0.11.148 - 2026-11-05
- **Propagazione in finestra a parte**: si stacca solo la mappa «Chi mi ha ricevuto», a tutta finestra (prima tutta la pagina).

## 0.11.147 - 2026-11-05
- **Finestra del display staccato**: si porta alla misura esatta del display (`plugin:window|set_size`, permesso `core:window:allow-set-size` per `finestra-*`), senza fondo nero in piu'; dopo un ridimensionamento dai bordi o con la maniglia la finestra segue il display in proporzione.

## 0.11.146 - 2026-11-05
- **Display della radio da solo**: la sua finestra a parte e' senza cornice (`decorations(false)`) e si sposta dal bordo o dalla presa ⠿ (`data-tauri-drag-region`, permesso `core:window:allow-start-dragging` per le finestre `finestra-*` in `capabilities/finestre.json`); ⤓ la rimette nella principale. Anche le altre finestre a parte (un pannello, o piu' pannelli in riga/colonna) sono senza cornice e si spostano dalla barretta del blocco; solo la dashboard a blocchi resta una finestra normale.

## 0.11.145 - 2026-11-05
- **Orologio**: l'avviso «Orologio sbagliato» e' un pulsante che corregge subito l'ora di Windows (se Windows non lo permette apre le impostazioni dell'orologio); soglia di fabbrica 500 ms invece di 200 (FT8 tollera bene qualche decimo, e l'orologio di Windows deriva di centinaia di ms), chi aveva 200 passa a 500.

## 0.11.144 - 2026-11-05
- **Display in finestra a parte**: maniglia nell'angolo in basso a destra per ridimensionare il solo display in proporzione (CSS `zoom`, da 0,3 a 8 volte), indipendente dalla finestra; doppio clic torna ad adattarsi alla finestra; misura ricordata (`aurora-display-scala`).

## 0.11.143 - 2026-11-05
- **Pulsantino per staccare il display**: passandoci sopra la lente (zoom 1,5) non si apre piu' e si chiude se era aperta; prima copriva il menu ⧉ e lampeggiava.

## 0.11.142 - 2026-11-05
- **Interruttori al primo clic**: lo stato chiesto resta per 2,5 s (`lib/pendingControls.ts`) cosi' la prima rilettura del motore, che poteva arrivare prima dell'applicazione, non riporta indietro Abilita Tx e gli altri (Comandi, QsoSwitches, telefono).
- **Associazione del telefono**: con il QR in vista l'elenco dei dispositivi si rilegge ogni 2 s (prima mai); appena il telefono compare, l'attesa finisce da sola e arriva l'avviso «Telefono associato».

## 0.11.141 - 2026-11-05
- **Display della radio in finestra a parte** (`display`, `DisplayFit.tsx`): pulsantino ⧉ nell'angolo in alto a destra del display (bianco su fondo scuro, compare al passaggio); nella finestra il display scala con CSS `zoom` mantenendo le proporzioni; nella principale resta solo un segnaposto. Anche in Impostazioni › Finestre.
- **Pagina Log con log grandi** (segnalato con 71 252 QSO): la tabella disegnava tutte le righe e la pagina si bloccava; ora 300 per volta con «Mostra altri»; importazione e lettura erano gia' veloci (71 000 QSO in pochi secondi).

## 0.11.140 - 2026-11-05
- **Errore «libhamlib-4.dll» durante l'aggiornamento**: il rigctld di Aurora sopravviveva alla chiusura del programma e teneva la DLL; l'installatore ora lo ferma con `taskkill /FI "MODULES eq libhamlib-4.dll"` (senza PowerShell), poi con PowerShell solo per quello della cartella di Aurora, fino a quattro tentativi.
- **Impostazioni › Audio**: la «Cartella dei file audio» non mostra piu' una cartella rimasta da JTDX; fuori dalla cartella di Aurora non vale (come gia' faceva il core) e compare quella di Aurora.

## 0.11.139 - 2026-11-05
- **Impostazioni › Audio**: cursore del volume di Windows sotto l'ingresso (registrazione, `/api/v1/input-level`) e sotto l'uscita (trasmissione Pwr, `/api/v1/output-level`), si invia al rilascio (`settings/DeviceVolume.tsx`).
- **Dashboard a blocchi**: 12 colonne e 24 righe che seguono la finestra in larghezza e in altezza; il primo blocco riempie la finestra (prima restava uno spazio grigio vuoto).

## 0.11.138 - 2026-11-05
- **Dashboard a blocchi** (finestra `#/finestra/dash`, `GridBoard.tsx`): griglia a 12 colonne, ogni elemento si trascina dalla barretta e si ridimensiona dall'angolo, gli altri blocchi si spostano e risalgono; posizione e misura ricordate. Si sceglie dal menu ⧉ («Nella dashboard a blocchi») e in Impostazioni › Finestre.
- **Finestre a parte che lampeggiavano e si chiudevano** (0.11.137): la finestra nuova non trovava ancora il suo gruppo e si chiudeva; ora il contenuto viaggia anche nell'indirizzo (`?items=`) e la finestra si chiude solo se il gruppo c'era ed e' stato svuotato.
- **Barra dei pannelli**: le scritte a destra (es. «QSO nel log») si spostano per non coprire il pulsantino ⧉.

## 0.11.137 - 2026-11-05
- **Impostazioni › Finestre**: «Dove sta ogni elemento» per i 7 pannelli, la Mappa e la Propagazione (finestra principale, finestra nuova, accanto a una finestra a parte, nascosto) al posto delle caselle «Pannelli mostrati»; «Finestre a parte aperte» con Mostra, Rimetti e Rimetti tutto; «Ripristina la disposizione» rimette anche tutto nella finestra principale (`settings/WhereTab.tsx`).
- **Installazione piu' veloce**: niente PowerShell per chiudere Aurora (solo `taskkill`, mezzo secondo); PowerShell solo se la cartella di Hamlib resta bloccata, al massimo due volte da 6 s. Nella 0.11.135-136, su PC dove PowerShell non risponde, l'installazione restava ferma fino a un minuto.

## 0.11.136 - 2026-11-05
- **Finestre a parte**: rimesso l'ultimo elemento (⤓) la finestra si chiude da sola; nel programma per Windows `window.close()` non bastava, ora la pagina chiede `/__chiudi-finestra` e il guscio chiude la finestra (`on_navigation`).
- **Mappa bianca nella finestra a parte**: la finestra nasce nascosta e Leaflet calcolava uno zoom infinito con larghezza zero; ora la misura si prende solo quando c'e' e la vista si rimette a posto.
- **Rimetti tutto**: con finestre a parte aperte, accanto a lingua e volume compare un pulsantino ⤓ che riporta tutto nella finestra principale e chiude le finestre a parte.

## 0.11.135 - 2026-11-05
- **Installatore fermo all'inizio** (segnalato su un PC aziendale): prima `taskkill` di Aurora, poi PowerShell con tempo massimo (`/TIMEOUT` 15 s e 8 s, `-NonInteractive -ExecutionPolicy Bypass`); se PowerShell non risponde si prosegue; `Get-Process` invece di `Get-CimInstance` (WMI lento) per il rigctld di Aurora. Immagini dell'installatore senza la versione scritta dentro (mostravano 0.11.46).
- **Finestre a parte**: posizione, misura e massimizzazione ricordate per ognuna (`<etichetta>.json` accanto a `finestra.json`), anche all'uscita di Aurora.

## 0.11.134 - 2026-11-05
- **Finestre sul secondo schermo** (prima prova): pulsantino ⧉ nella barra di ogni pannello della Stazione e accanto ai titoli di Mappa e Propagazione; l'elemento va in una finestra a parte (`#/finestra/<id>`, creata dal guscio sul secondo schermo se c'e', ridimensionabile) e sparisce dalla finestra principale. Più elementi nella stessa finestra, in riga o in colonna, con divisore; ⤓ li rimette; la X li riporta nella principale. Le finestre si riaprono all'avvio (`lib/detached.ts`, `SoloWindow.tsx`). Non su telefono, tablet e PC collegati da remoto. Da fare: posizione e misura ricordate per ogni finestra.

## 0.11.133 - 2026-11-05
- **Radio accesa dopo Aurora**: se la scheda audio della radio non c'e' all'avvio (radio spenta, cavo USB staccato), Aurora riprova ogni 5 s e la ricezione parte da sola appena compare; intanto un avviso spiega cosa succede. Prima serviva riavviare Aurora.

## 0.11.132 - 2026-11-05
- **Decodifica FT8 piu' sensibile sui QSO in corso**: se 30 s prima nello stesso punto c'era un messaggio, Aurora prova a ritrovarlo o a leggere il suo seguito (rapporto, RRR, RR73, 73), come JTDX; in Modo SWL con la risincronizzazione sui toni attesi. Nelle prove il seguito di un QSO a -23 dB si legge 18 volte su 20 invece di 0, senza messaggi falsi in piu'.
- **Modo SWL**: la sottrazione dei segnali decodificati usa il filtro corto di JTDX per l'ascolto (`NFILT2`), che segue meglio i segnali con evanescenza.
- **Messaggi della radio nella lingua scelta** (issue #1): stato ed errori di CAT, Hamlib, OmniRig, PTT, porta occupata e rilevamento della radio («il nome … contiene il modello»), registro delle automazioni, errori dell'audio ed esiti degli invii ai servizi (LoTW, eQSL, PSK Reporter, DX Summit, TCP), che il core scrive in italiano, ora si leggono in inglese, tedesco, francese, spagnolo e portoghese (`app/src/lib/coreText.ts`).
- **AGCc anche in FT4**: la compensazione dell'AGC della radio ora vale anche in FT4 (nelle prove, a -17 dB 15 periodi decodificati su 20 invece di 3).
- **Decodifica lenta**: se il PC impiega spesso piu' del 70% del periodo a decodificare compare un avviso (Multithread, Modo SWL, programmi pesanti) e parte un rapporto anonimo con tempi, processori e opzioni.

- **Audio, meno blocchi persi**: il thread che legge la scheda audio gira a priorita' alta (MMCSS su Windows) e non alloca piu' memoria a ogni blocco; nel rapporto dei blocchi persi c'e' anche la causa indicata da Windows.
- **Rapporti piu' chiari**: i blocchi audio persi riportano anche da quanti minuti, quanti al minuto e il tipo di ingresso (USB, integrata, virtuale; mai il nome), frequenza, canali e formato. Il nominativo o locatore da compilare non e' piu' un errore segnalato ma un avviso.
- **Avvisi sullo schermo** (stesse schede della diagnosi, sei lingue): blocchi audio persi (cavo USB, risparmio energetico), trasmissione senza radio, OmniRig che non risponde (ora tre tentativi). **Verifica tutto** (Impostazioni › Assistenza): controlla stazione, radio, audio, orologio e telefono e porta dove sistemare. **Porta occupata**: il messaggio dice quale programma la usa (Windows).
- **Riquadro «Radio non collegata»**: non si sovrappone piu' alla barra in alto ne' al pannello Comandi, ha spazio sopra e sotto.
## 0.11.131 - 2026-11-04
- **Splash di apertura**: il display della radio (livello 1001 dalla 0.11.129, quasi quadrati 1050) compariva sopra; ora display a livello 1 e splash a 2000.

## 0.11.130 - 2026-11-04
- **Display in testata**: fuori dal flusso della riga (`position: absolute`), centrato fra il bordo della barra del menu e i pannelli; non si sovrappone e non sposta nulla. Frontalini FT-2000/IC-7610 8 px sopra e sotto; con gli estesi da 90 la riga ha `min-height` quanto basta (4 px sopra e sotto). Tolta la salita sul bordo della barra della 0.11.129.

## 0.11.129 - 2026-11-04
- **Frontalini FT-2000 / IC-7610**: altezza del disegno (`height: auto`) invece del riquadro di 90, niente striscia vuota sotto; PWR alto quanto il frontalino.
- **Attività di banda / CQ**: colonne della distanza e dell'etichetta 7 px più strette in tutte le larghezze.
- **Display esteso alto 90 senza spostare niente**: la riga in alto resta alta quanto i comandi a sinistra, il display sale di 22 px sul bordo basso della barra del menu (in primo piano) e finisce dove cominciano i pannelli.
- **Lente del display** (passaggio del mouse) ingrandita 1,5 volte invece di 2: copre meno i pannelli.

## 0.11.128 - 2026-11-04
- **Display estesi** 640 × 90 punti (prima 80), anche anteprime, invito «Clicca per configurare» e blocco PWR.
- **Attività di banda / CQ** (solo PC): testo del messaggio 1 punto più piccolo (`calc(0.8rem - 1px)`).

## 0.11.127 - 2026-11-04
- **QSL da HamQTH**: se QRZ.com (senza abbonamento XML) non manda LoTW/eQSL/diretta/manager, `GET /api/v1/callsign/{call}` completa con HamQTH (`Person::fill_from`), nome e città restano di QRZ; nuovo campo `bureau` (`qsl` di HamQTH) e lettera **B**.
- **Precaricamento** dei nominativi di Attività di banda (i 60 più recenti), come nel DX Cluster.
- **Testata**: con il display esteso lingua, volume e tema in fondo a destra, senza pallino; con il quasi quadrato restano dopo «Novità» col pallino.
- **Riga del QSO in corso**: l'etichetta (NUOVA STAZ. ecc.) tiene i suoi colori e resta leggibile.

## 0.11.126 - 2026-11-04
- **Display quasi quadrati** 330 × 130 (prima 110), centrati fra il bordo della finestra e i pannelli; la barra del menu si ferma prima del display con l'angolo arrotondato.
- **Display estesi**: stesso spazio sopra e sotto per ogni tipo (la regola prima valeva solo per l'invito «Clicca per configurare»).
- **Documentazione**: `docs/cat4om-protocollo.md`, il protocollo nativo di CAT4OM (WebSocket JSON, canali Control e Management, comandi, eventi, audio in rete) per l'integrazione.

## 0.11.125 - 2026-11-04
- **QSO in corso**: in Attività di banda/CQ la stazione chiamata compare in una sola riga ferma (in cima, in fondo se l'elenco è rovesciato) con il messaggio più recente; colore per fase: giallo chiamata, arancio rapporto, verde R+rapporto, blu chiusura; a DX libero torna a scorrere (`DecodeList`).
- **QSL da QRZ.com e HamQTH**: `lotw`, `eqsl`, `mqsl`/`qsldirect`, `qslmgr`/`qsl_via` letti e passati in `GET /api/v1/callsign/{call}`; lettere L E D accanto alla Stazione DX nei Comandi, righe ✔/✖ nel riquadro del nominativo, che ora c'è anche in Attività di banda (non sul telefono).
- **Display**: estesi 640 × 80 centrati fra menu e pannelli; quasi quadrati 330 × 110.
- **Testata**: lingua, volume e tema piccoli con ogni display, subito dopo «Novità» e staccati da un pallino colorato; tolto il logo a destra.
- **Telefono**: tacca rossa di chi mi chiama sottile e dentro la riga; sotto il QR il consiglio per Chrome su Android (connessioni sicure, rete locale, Firefox).

## 0.11.124 - 2026-11-04
- **Log dopo RRR senza 73**: molte stazioni non rispondono al RRR; all'ultimo tentativo di chiusura (o dopo 3 senza limite scelto) il QSO va a log, o si apre la conferma, invece di essere abbandonato (`qso-native/src/sequencer.rs`).
- **Telefono**: tasto «Tx/Rx Split» a destra sulla riga della stazione chiamata nel riquadro Messaggi Tx.
- **Pannello Log**: colonna del paese visibile anche nella finestra stretta (i rapporti si tolgono per primi), con i puntini se non ci sta.
- **Attività di banda / CQ**: bandiera, paese e distanza 10 px più a destra, più spazio al messaggio.
- **Display estesi** a 640 × 75 punti (prima 70); **LED essenziale** con S, Po e SWR sulla riga della frequenza, sotto i pulsanti.

## 0.11.123 - 2026-11-04
- **Telefono bloccato dal firewall, corretto**: se alla domanda di Windows «Consenti l'accesso» si premeva Annulla, Windows creava regole di blocco che vincono su ogni permesso, e «Consenti Aurora» non bastava. Ora «Consenti Aurora» (Impostazioni › Smartphone/Tablet) toglie i blocchi su Aurora e aggiunge il permesso per il programma e per la porta TCP, su ogni rete (anche «Pubblica»); il controllo della rete riconosce anche la regola della porta e non segnala più la rete «Pubblica» quando il permesso c'è.
- **QR del telefono più sicuro**: con il PC su più reti (cavo e Wi-Fi) nel QR va per primo l'indirizzo della Wi-Fi, dove è quasi sempre il telefono; sotto il QR compaiono gli altri indirizzi del PC, ognuno con il suo QR piccolo («il telefono non apre la pagina? prova il successivo»), e il consiglio di inquadrarlo con la fotocamera e aprirlo in Chrome o Safari, non dentro Messenger o WhatsApp. Sei lingue.
- **Diagnosi dell'associazione dal telefono**: se un codice scade senza associazione, Aurora manda un rapporto anonimo con la causa probabile: telefono mai arrivato (rete diversa, dati mobili, isolamento della Wi-Fi), rete di Windows «Pubblica», firewall senza regola, indirizzo del QR di un adattatore virtuale o senza indirizzo dal router; oppure telefono arrivato ma codice sbagliato o scaduto. Solo tipi, mai indirizzi o nomi (informativa privacy aggiornata).
- **Finestra di conferma del log rifinita**: nominativo in evidenza, banda, modo, frequenza e ora a etichette, campi in due colonne anche sul telefono, «Scarta» e «Registra» affiancati; provata su PC (chiaro e scuro), iPhone 13 e iPhone SE.
- **Telefono**: nel DX Cluster e nel Log il nome del paese accanto al nominativo, dopo la bandiera (più piccolo; accorciato con i puntini se non c'è spazio).
- **Conferma prima di registrare il QSO** (Impostazioni › Sequenza › «Registrazione log» › «Richiedimi di loggare il QSO», spento di fabbrica): a QSO concluso si apre una finestra con nominativo, locatore, rapporti, nome, città e commento correggibili, «Registra» (anche con Invio) o «Scarta». Prima la voce esisteva ma non faceva nulla. Diversamente da JTDX non esclude «Messa a log automatica», che decide solo come finisce la trasmissione. Se la voce si spegne con QSO in attesa, vanno a log; oltre 20 in attesa il più vecchio va a log da solo. Sei lingue.
- **Prove dell'interfaccia** allineate alla pagina Novità compatta e al controllo della rete nella scheda Smartphone/Tablet (850 su 850).
- **Rapporti audio**: poche perdite di campioni sono informazione nel registro; avviso (e rapporto) solo da 100 in su.

## 0.11.122 - 2026-11-04
- **Locandina dell'aggiornamento**: finche' il primo controllo non e' fatto la finestra lo chiede ogni 5 s (`UpdatePoster`, `UpdateBar`); prima, con l'avviso gia' dato a un avvio precedente, aspettava la lettura ogni 10 minuti.

## 0.11.121 - 2026-11-04
- **OmniRig che non risponde**: il PTT spiega di controllare porta e modello di Rig1 e la radio accesa, o di passare a Hamlib (dai rapporti: mezz'ora di trasmissioni fallite su una stazione).
- **Telefono**: il riquadro del DX Cluster non compare se il cluster e' spento in Aurora.

## 0.11.120 - 2026-11-04
- **Associazione del telefono**: indirizzi del QR senza adattatori virtuali (WSL, Hyper-V, VirtualBox, VPN, tunnel) e con prima 192.168.x; nella scheda la rete Wi-Fi del PC, l'indirizzo scelto, l'avviso se la rete e' «Pubblica» o il firewall blocca Aurora con «Consenti Aurora» (`GET /api/v1/lan/check`, `POST /api/v1/lan/allow`), i consigli per il telefono.
- **Finestra**: si ricollega da sola se il flusso resta muto per 8 s (prima si fermava finche' non la si ricaricava).
- **Novita'**: una riga per versione, filtri per tipo, la versione in uso aperta.
- **Display quasi quadrato** alto 120 punti, banda passante visibile; **suono dello splash** una volta per avvio; controllo della versione nuova 8 s dopo l'apertura.

## 0.11.119 - 2026-11-04
- **Porta del CAT scrivibile**: oltre alle COM accetta un indirizzo di rete (`127.0.0.1:4532` per «Hamlib NET rigctl», flrig, rigctld di un altro programma); avviso se la condivisione della radio usa la stessa porta.
- **CQ e novita'**: colonna del paese con larghezza minima (niente piu' «S..»).
- **Display quasi quadrato** messo sulla pagina sopra la testata (prima finiva sotto, sopra il pannello Comandi).

## 0.11.118 - 2026-11-04
- **Pannelli da togliere**: Impostazioni › Finestre › «Pannelli mostrati» toglie o rimette ogni pannello della stazione (cascata, attività di banda, log, DX Cluster…); «Ripristina la disposizione» li rimostra tutti (richiesta di un utente). Sei lingue.
- **Import ADIF grandi**: limite del file portato da 64 a 256 MB (un log di 70 mila QSO esportato con tutti i campi superava i 64 MB); se il file è troppo grande il messaggio lo dice chiaramente invece di un errore generico.

## 0.11.117 - 2026-11-04
- **«Già lavorato»** solo per un nominativo davvero nel log (non per ogni stazione di un paese gia' fatto), spiegazione nelle sei lingue.
- **S-meter stimato** con la tilde anche nei display a lancetta nuovi.
- **Novita'** tenute alle ultime 40 voci; link diretti ad AppImage e .deb nei README; prove dell'interfaccia allineate (846 su 846).

## 0.11.116 - 2026-11-04
- **Installazioni attive**: `POST /v1/attivo` nel Worker (tabella D1 `attivi`, una riga per installazione e giorno, 90 giorni) e `GET /v1/attivi` per il proprietario; nel core `Rapporti::segnala_attivo` una volta al giorno se `count_active` (acceso di fabbrica, si spegne in Impostazioni > Assistenza). Informativa privacy aggiornata.

## 0.11.115 - 2026-11-04
- **Menu Radio**: le anteprime degli stili estesi (`.rd-card--wide`) hanno la misura vera 640 x 70 invece della larghezza della pagina.

## 0.11.114 - 2026-11-04
- **Locandina della versione nuova** (`UpdatePoster.tsx`): al centro all'apertura, con le note della release, la data, «Note e download», «Piu' tardi» (fino alla versione dopo) e «Installa e riavvia» (aggiornamento firmato). Sei lingue.

## 0.11.113 - 2026-11-04
- **Sei display nuovi** (`ModernDisplays.tsx`, `modern-displays.css`): estesi `icdual`, `icblue`, `ledline` (640 x 70) e quasi quadrati `icblueq`, `icblackq`, `yaesuq` (330 x 140, fissi nell'angolo; con la classe `rd-square` le icone della testata si rimpiccioliscono). Indicatori solo se la radio da' il dato e se spuntati in «Display esteso: cosa mostrare»; clic che comanda la radio come gli altri pannelli.
- **Altezza dei display estesi**: da 64 a 70 punti; misure accanto a ogni stile e disposizione in Impostazioni › Radio.

## 0.11.112 - 2026-11-04
- **Log in una pagina**: `LogTransfer` (import/export ADIF) in cima a Impostazioni › Programmi collegati, sopra server UDP e invio ADIF; il riquadro del log vuoto porta li'.

## 0.11.111 - 2026-11-04
- **Avvisi della stazione**: la barra della diagnosi (`DiagnosisBar`) diventa una pila di schede fisse in basso a destra con chiusura per avviso (riappare se cambia); i pannelli non si spostano piu'.

## 0.11.110 - 2026-11-04
- **Decodifiche false senza radio**: un «CQ SU3NRA» a -25 dB su un PC senza radio (solo microfono). Con il picco del periodo sotto 0,02 (lo stesso «debole» della diagnosi) si spengono a7, a8 e l'analisi a priori; sotto 0,0005 (muto) il periodo non si decodifica ma la sequenza del QSO avanza lo stesso.

## 0.11.109 - 2026-11-04
- **Radio e log da configurare**: componente `SetupCta`; il display in testata, con il CAT scollegato o in errore, mostra solo «Radio non configurata / non collegata · Clicca per configurarla» (verso Impostazioni › Radio); il pannello Log vuoto porta a Statistiche › importazione del log. Sei lingue.

## 0.11.108 - 2026-11-04
- **Due schede audio uguali**: `match_device_name` prova prima il nome esatto (con il «2- »), poi il nome ripulito solo se una scheda sola corrisponde; prima la 0.11.105 poteva aprire la scheda sbagliata e la ricezione si fermava.

## 0.11.107 - 2026-11-04
- **Guida**: la voce «Installare Aurora» racconta la guida al primo avvio; nuova voce «Copia delle impostazioni» nei Problemi comuni (6 lingue).
- **Documentazione**: `docs/ripresa.md` allineata alla 0.11.106; TODO dai rapporti degli errori.

## 0.11.106 - 2026-11-04
- **Decodifica FT8 a tempo**: dai rapporti degli errori, un PC impiegava fino a 66 s per periodo e restava indietro. Nuova `Options::deadline`: dopo 11 s dall'inizio della decodifica non si comincia un'altra passata; i messaggi gia' trovati restano.

## 0.11.105 - 2026-11-04
- **Schede audio, dai rapporti degli errori**: il nome salvato si confronta decodificando `\xNN`, senza il numero «2- » che Windows aggiunge, e come ultima prova con la sola scheda tra parentesi (Windows in altre lingue); «Default Input/Output Device» e «predefinito» vogliono dire il dispositivo predefinito di Windows. Vale per cascata, trasmissione e livelli.
- **OmniRig assente**: «classe COM non registrata» diventa «OmniRig non e' installato: scegli Hamlib in Impostazioni > Radio».

## 0.11.104 - 2026-11-04
- **Copia delle impostazioni** (Impostazioni › Assistenza): zip sul Desktop con i file di configurazione e una copia coerente del log dei QSO; il ripristino controlla lo zip, mette i file in `ripristino` e li applica al prossimo avvio. Fuori password e chiavi (`*.secret`), dispositivi associati, storico e file dei rapporti.
- **Suono della schermata di avvio**: WebView2 bloccava l'audio prima di un clic; ora si sente.

## 0.11.103 - 2026-11-04
- **Modo senza audio**: lo stato della stazione (modo, periodo, frequenza) si pubblica subito, anche se nessun ingresso audio si apre; prima il selettore del modo restava «—» e non si apriva.

## 0.11.102 - 2026-11-04
- **Guida al primo avvio**: passo passo lingua, nominativo e locatore, scheda audio e radio (CAT o VOX), ogni passo saltabile; compare solo finche' manca il nominativo.
- **Testi**: nessun testo dice piu' che i dati vengono da JTDX o `wsjtx_log.adi`; le fonti sono il log di Aurora e i software esterni collegati in Impostazioni.
- **Servizi accesi di fabbrica**: PSK Reporter (chi ti ha ricevuto), propagazione (hamqsl.com), calendario delle DXpedition (NG3K) e DX Cluster partono accesi; si spengono dalle impostazioni. Chi li aveva gia' scelti trova le sue scelte. Restano spenti solo i servizi che chiedono un account (QRZ, LoTW, eQSL, ClubLog, HRDLog, Wavelog). Informativa privacy aggiornata.
- **DX Cluster con il nominativo di stazione**: non si scrive piu' nella scheda del cluster; Aurora usa quello dei dati di stazione (Impostazioni › Generali) e lo segue se cambia.
- **BBLogger**: destinatario pronto in Impostazioni › Programmi collegati › Integrazioni («+ BBLogger»: 127.0.0.1:2237, flusso WSJT-X e Logged ADIF), voce della Guida aggiornata nelle sei lingue e passi in `docs/integrazioni-log.md`.
- **Link Linux senza 404**: nei README i pulsanti AppImage e .deb aprono la pagina della release e la guida ha l'indirizzo completo; nella pagina web, finche' la release non ha i pacchetti Linux, i pulsanti portano alla release con un avviso.
- **Paesi e bandiere senza altri programmi**: il file dei paesi `cty.dat` (AD1C, country-files.com) e' dentro Aurora, aggiornato a ogni versione; prima Aurora lo cercava solo nell'installazione di JTDX e, senza JTDX, non mostrava bandiere, paesi ne' NEW ONE/NEW BAND.
- **Tolto il vecchio modo JTDX**: Aurora non fa piu' da schermo a JTDX (motore `AURORA_ENGINE=jtdx`, ponte UDP, comando della finestra di JTDX, lettura e modifica di `JTDX.ini`, riavvio di JTDX): resta solo il suo motore. OmniRig resta come driver della radio (solo frequenza).
- **Aurora indipendente**: all'avvio non legge piu' JTDX (ne' `JTDX.ini` ne' il suo log) e non ne prende motore, radio OmniRig, PTT, audio o porte UDP; `rigctld` e' solo quello incluso in Aurora (o, su Linux, quello del sistema), mai quello di altri programmi. Chi aveva impostato audio e radio in Aurora li ritrova; chi li aveva solo in JTDX li sceglie una volta in Impostazioni.
- **FT8/FT4 senza radio collegata**: il modo si cambia anche senza CAT (radioascolto), la radio resta dov'e'.
- **Bande bloccate**: il suggerimento spiega che si sbloccano collegando la radio in Impostazioni › Radio (prima diceva «AURORA_RIG=omnirig»), nelle sei lingue.
- **Linux** (non provata dall'autore, senza assistenza): compila dallo stesso codice di Windows; `scripts/compila-linux.sh` prepara AppImage e `.deb` con un comando, guida in sei lingue (`docs/linux.md`), sezione Linux nella pagina web e nei README; GitHub Actions controlla Linux a ogni push e allega AppImage e `.deb` a ogni release.
- **Pagina web**: download con linguette Windows / Linux e passi per Ubuntu·Debian·Mint, Fedora, Arch·Manjaro e compilazione, con pulsante «Copia» per ogni comando; pulsanti grafici Linux nei README; guida Linux ampliata (passo passo per distribuzione, primo avvio, disinstallazione, problemi comuni). Lo script installa anche Node.js 22 se manca o è troppo vecchio.

## 0.11.101 - 2026-11-04
- **File audio**: sempre nella cartella di Aurora (una cartella fuori, per esempio copiata da JTDX, torna a quella di Aurora).
- **Attivita' di banda**: bandiera e paese piu' a sinistra, nome del paese mezzo punto piu' piccolo.
- **Attivita' di banda**: colonna dei km sempre presente e larga uguale, linee dritte anche nelle righe senza distanza.
- **Radio**: «Impostazioni consigliate» allineate in alto a sinistra.

## 0.11.100 - 2026-11-04
- **Rapporti degli errori accesi di fabbrica** (domanda dopo un crash ed errori anonimi), con avviso una volta al primo avvio e collegamento a Impostazioni > Assistenza; informativa e Guida aggiornate. Da fare: la stessa informativa con le caselle nell'installatore.

## 0.11.99 - 2026-11-04
- **Attivita' di banda**: colonna di bandiera, paese e km piu' larga verso sinistra, il paese non si accavalla con i km.

## 0.11.98 - 2026-11-04
- **Trasmissione con Hamlib e un vecchio JTDX.ini**: se il PTT preso da JTDX e' OmniRig ma la radio e' collegata con Hamlib, Aurora usa il PTT della radio. Era il motivo per cui l'IC-7300 di un utente non andava in TX («PTT: radio non in linea»).
- **Rapporti degli errori attivi**: servizio online (aurora-rapporti.tizianocassone.workers.dev, R2 + D1, riepilogo protetto da ADMIN_TOKEN); i rapporti partono per chi li accende in Impostazioni > Assistenza.

## 0.11.97 - 2026-11-04
- **Rapporti degli errori** (Impostazioni > Assistenza, spenti di fabbrica): dopo un crash e automatici anonimi, con anteprima e pulizia dei dati personali; Worker Cloudflare in servizi/rapporti (non ancora online: finche' l'indirizzo e' il segnaposto non parte niente); informativa privacy in docs/privacy.md.
- **Bandiere**: grafie di NG3K e del cluster (Rodrigues I, Tokelau, Swains I, Pitcairn e altre).

## 0.11.96 - 2026-11-04
- **Assistenza**: scheda a se' nelle Impostazioni; «Manda per email» apre Gmail in Chrome gia' compilato; il campo email e' vuoto di fabbrica.
- **Attivita' di banda**: km visibili anche con il pannello stretto, colonna larga quanto il numero, niente doppia linea quando manca la distanza.
- **Telefono**: bandiera e paese seguono le opzioni di Schermo.

## 0.11.95 - 2026-11-04
- **Pacchetto per l'assistenza** (Impostazioni > Informazioni): zip sul Desktop con log, crash, configurazioni senza segreti, ultime righe di ALL.TXT e info.txt (versione, PTT effettivo, audio, porte, stato Tx); «Manda per email» con posta gia' compilata (indirizzo scelto dall'utente); voce nella Guida.
- **PTT**: senza PTT da JTDX si decide a ogni trasmissione da Impostazioni > Radio, senza riavvio.
- **Radio**: riquadro «Impostazioni consigliate» con elenchi allineati a sinistra.

## 0.11.94 - 2026-11-04
- **Guida riorganizzata** in sei gruppi con passi brevi e «Se non va»; nuove voci per Log4OM, LogHX3 e BBLogger2 via UDP (lato Aurora e lato log); docs/integrazioni-log.md aggiornato.
- **Fox**: cascata larga solo fino a «Messaggi ricevuti», che scende fino in fondo.
- **Attivita' di banda**: piu' spazio al messaggio ricevuto, carattere mezzo punto piu' piccolo.
- **Messaggi**: percorso giusto per «qualsiasi messaggio» (Programmi collegati) e per lo spot DXSummit (Servizi online).

## 0.11.93 - 2026-11-04
- **Aggiornamenti**: la versione nuova trovata ha la stella e la targhetta NEW.
- **crash\stderr.log**: via le righe di avvio e l'avviso innocuo di WebView2 «Failed to unregister class Chrome_WidgetWin_0» (compare alla chiusura della finestra e non e' un guasto); restano solo gli errori veri.

## 0.11.92 - 2026-11-03
- **Trasmissione senza JTDX**: il motore usava solo il PTT di JTDX (RTS/DTR/OmniRig) o il VOX; ora, senza PTT da JTDX, usa quello di Impostazioni > Radio (CAT attraverso Hamlib con `T 1`/`T 0`, oppure OmniRig). Era il motivo per cui IC-7300 e IC-7610 non andavano in TX.
- **Configurazione guidata della radio**: riquadro «Impostazioni consigliate per questa radio» con pulsante Applica (Icom, Yaesu, Kenwood), voce «Configurare la radio» nella Guida, avviso Icom sotto 115200 baud.
- **Impostazioni > Radio**: nomi delle voci su una riga.
- **Telefono**: un tocco chiama la stazione, i tocchi ripetuti non fanno ripartire la sequenza.
- **Documenti**: sezione Icom IC-7300 e IC-7610 in docs/cat-radio.md.

## 0.11.91 - 2026-11-03
- **Revisione del codice**: giallo solo per la stazione chiamata e la riga toccata (non per quelle in coda); il cluster del telefono non usa il testo cercato sul PC; l'accesso senza codice si cambia in memoria solo dopo averlo salvato.

## 0.11.90 - 2026-11-03
- **Telefono**: cluster con modo e chi ha fatto lo spot; log con bandiera; colonne allineate; stazione chiamata in giallo (anche appena toccata) e nel riquadro Tx; blocchi con bordo e titolo del loro colore; barra con la frequenza 2 px piu' in basso.

## 0.11.89 - 2026-11-03
- **Telefono**: cluster e log aperti all'avvio; cambio tema chiaro/scuro in alto a destra; barra con la frequenza un po' piu' su.

## 0.11.88 - 2026-11-03
- **Telefono e tablet sempre aggiornati**: la pagina confronta la sua versione con quella di Aurora sul PC (all'avvio, quando torna in primo piano e ogni minuto); se Aurora e' stata aggiornata toglie service worker e cache e si ricarica da sola.
- **Telefono**: pulsante ⟳ per ricaricare dal PC; barra di stato dell'iPhone piena e pagina piu' in basso (niente sfocatura); bandiere piu' grandi e colonna del paese piu' larga; titoli piu' alti.

## 0.11.87 - 2026-11-03
- **Telefono**: la pagina principale arriva sempre dalla rete (cache nuova del service worker), niente piu' versione vecchia dopo un aggiornamento; testata e pie' di pagina non disegnati; cluster con gli ultimi 5 spot; barra dei pulsanti piu' bassa.

## 0.11.86 - 2026-11-03
- **Telefono e tablet**: solo la pagina Stazione, senza testata, pie' di pagina e toast; bande, modi, filtri, macro, lingua e tema presi da Aurora sul PC; titoli e colonne compatti; la pagina non scorre di lato; doppio tocco su STOP = Canc DX.
- **Smartphone/Tablet** (Impostazioni): pagina compatta, QR piccolo; accesso senza codice dalla rete di casa che funziona e resta al riavvio (accesso-libero.txt).
- **DXpedition**: confini e paesi preparati in anticipo, la scheda si apre subito.
- **Elenchi**: colonna dei km 10 px piu' larga.

## 0.11.85 - 2026-11-03
- **Telefono**: attivita' di banda su una riga con colonne allineate, al massimo 10 righe in vista; tolta la riga «In onda»; schermata fissa senza zoom; spazio per la barra dell'iPhone.

## 0.11.84 - 2026-11-03
- **QR e aurora.local**: associazione del telefono inquadrando un QR; Aurora si annuncia in rete come aurora.local; il telefono ricorda gli indirizzi; vista tablet in orizzontale; anteprime dal vivo in Impostazioni > Smartphone/Tablet.
- **Impostazioni**: tolte le spiegazioni fisse e i sottotitoli delle pagine; righe dei servizi basse.
- **Pagina Fox**: cascata 110 px, chi chiama 620 px.
- **DX Cluster**: precaricate le informazioni di tutti gli spot visibili, riquadro subito aperto.
- **Telefono come telecomando** (schermi sotto 768 px): barra con frequenza, RX/TX, banda, modo e potenza; cascata e spettro; chi e' in onda con bandiera e paese; i sei messaggi Tx sempre visibili e il riquadro «In trasmissione» con la stazione chiamata; elenco dei messaggi con un tocco per chiamare; 5 spot del cluster e ultimi 3 QSO richiudibili; CQ, Abilita Tx e STOP sempre in fondo.

- **Telefono, tablet e altri PC**: niente menu Impostazioni (si cambiano dalla finestra di Aurora o da un browser sullo stesso computer).

- **Impostazioni**: tolti tutti i riquadri «Note» dalle pagine.

- **Servizi di log e Servizi online**: colonna a destra pulita: solo «Prova» con «riuscita» o «non riuscita» (il messaggio completo passando il mouse), niente «Ultimo invio: mai», le note dentro il «?» del titolo.

- **DX Cluster**: riquadro con bandiera, paese, nome e citta' anche sul nominativo DX; le informazioni degli ultimi 30 spot (DX e chi li ha fatti) si caricano in anticipo, una alla volta.


## 0.11.83 - 2026-11-03
- **«Prova» e spia** anche per eQSL (accesso senza invio), spot su PSK Reporter e DXSummit (riquadri separati), NG3K, propagazione, orologio NTP (scarto), uscite UDP (Heartbeat) e TCP (connessione).
- **Scheda DXpedition**: si apre una volta sola gia' completa (cartina creata prima del primo disegno, riquadro del paese con segnaposto uguale a quello vero, scheda rimontata per ogni stazione).
- **Pagina Fox/DXpedition**: cascata piu' bassa (150 px) ed elenco dei messaggi piu' lungo (560 px).

## 0.11.82 - 2026-11-02
- **Ricerca QRZ.com e HamQTH**: la prima ricerca senza sessione aperta bloccava il servizio per sempre («Prova in corso…», nome e citta' mai arrivati). Il lucchetto dei dati ora si lascia prima dell'accesso.

## 0.11.81 - 2026-11-02
- **Spettro sotto la cascata**: di nuovo su «Attuale» (anche nello stile WSJT-X e una volta per chi aveva «Cumulativo»): segue il segnale in tempo reale invece di restare quasi fermo.
- **Finestra CQ**: colonna del paese larga, il nome si legge intero.
- **Elenchi dei messaggi**: bandiera e paese 15 px piu' a destra.
- **Riquadro del nominativo nel DX Cluster**: tolte le righe della fonte e dei dati ridotti.

## 0.11.80 - 2026-11-02
- **Pannello Comandi**: righe compatte (proposta 1): Call e locatore su una riga, rapporto con Lookup, Gen.Msgs e Canc DX, Tx con Tx=Rx e Split, Rx con Rx=Tx e Hold Tx.
- **Display FT-2000**: barre PO e SWR a segmenti con la scala graduata (0-50-100 W, 1-1.5-2-3-∞) e il valore a fianco.
- **Elenco dei messaggi piu' veloce**: in FT8 i messaggi entrano nell'elenco a fine di ogni passata del decodificatore, senza aspettare l'ultima; nella finestra CQ bandiera e paese hanno piu' posto.
- **CQ e novita'**: ogni stazione compare una sola volta, con la chiamata piu' recente.
- **DX Cluster**: DX, bandiera e paese non piu' tagliati a sinistra.
- **DX Cluster**: la frequenza non viene piu' tagliata a sinistra.
- **Scheda DXpedition**: niente cartina scura provvisoria prima di quella vera, e il riquadro del paese ha gia' il suo posto mentre si carica.

- **DX Cluster, chi ha fatto lo spot**: passando il mouse sul nominativo nella colonna DE compare un riquadro con bandiera, paese e continente e, con le ricerche accese, nome e citta'. Le ricerche partono dopo un breve ritardo e non si ripetono.
- **Ricerca QRZ.com e HamQTH** (Impostazioni > Servizi online): nome e citta' dei nominativi da QRZ.com (utente e password; dati completi con l'abbonamento XML, ridotti senza) e da HamQTH (gratuito), usato se QRZ e' spento o non trova. Spente di base, password protette da Windows, «Prova», risultati in memoria tre giorni. Nuova rotta `GET /api/v1/callsign/{call}`.
- **QRZ.com Logbook**: campo facoltativo «Nominativo registrato su QRZ», per i QSO fatti con /P, /QRP o prefissi esteri; rimandi fra le pagine Servizi di log e Servizi online.
- **HRDLOG.NET**: invio diretto dei QSO (nominativo e codice di caricamento), con la stessa coda, i nuovi tentativi e lo stato degli altri servizi. Formato non confermato dalla documentazione ufficiale (non raggiungibile): da provare dal vivo.
- **Password e chiavi nelle Impostazioni**: non spariscono piu' appena scritte. Si salvano uscendo dal campo o con Invio (non a ogni tasto), poi il campo mostra «Salvata ••••» con «Cambia» e «Togli», e la «Prova» del riquadro parte da sola con l'esito in verde o in rosso.
- **Cascata come WSJT-X**: di fabbrica una riga ogni 0,288 s (mezzo simbolo FT8, ~52 righe per periodo di 15 s invece di ~225), 4 canali per pixel, Flatten, spettro Cumulative, linea verde di periodo; pixel netti (filtro NEAREST, nessuna interpolazione) e segnali FT8 come blocchi compatti. In Impostazioni > Cascata la scelta «Velocità della cascata» (come WSJT-X / più veloce come JTDX) e i pulsanti «Stile WSJT-X» e «Stile JTDX» che impostano tutto in un clic (quello attivo si vede selezionato). Chi aggiorna passa allo stile WSJT-X una sola volta; le modifiche fatte dopo restano.
- **Impostazioni, revisione grafica di tutte le pagine**: le etichette vanno a capo per intero invece di troncarsi a due righe o con i puntini (Generali, Filtri, Sequenza, Servizi di log, Programmi collegati, colori di Notifiche); i campi di testo cedono spazio al nome fino a una larghezza leggibile e un indirizzo gia' scritto allarga il campo; le tendine mostrano per intero la scelta («Predefinito del modello» in Radio, «Nominativo completo in Tx3» in Generali); quando nome e valore non stanno su una riga il valore scende sotto invece di uscire dalla cella. Sequenza: un solo riquadro «Note» (erano due), regole di Aurora con righe normali e l'elenco delle stazioni escluse largo. Audio: la nota sciolta sotto la pagina e' nel riquadro «Note». Programmi collegati: le informazioni (cty.dat, log letto) in un riquadro come gli altri.

## 0.11.79 - 2026-11-02
- **DX Cluster**: niente piu' striscia vuota accanto a «Modo» (si sposta il contenuto, non la cella); colonna Freq altri 5 px a sinistra.
- **Display FT-2000**: tolta la scala di sintonia sotto la frequenza.
- **Potenza della radio (PWR −/+)**: dopo il comando Aurora rilegge la potenza; se la radio non l'ha cambiata (per esempio la FT-2000 con la manopola RF PWR) lo dice invece di far credere che sia cambiata. Il log registra potenza chiesta e potenza letta.
- **Display FT-2000**: RX e TX in alto subito dopo l'ultima cifra della frequenza principale (prima coprivano il modo).
- **Aggiornamenti**: tolte le voci per chi sviluppa (repository, versioni di prova, token di lettura); restano «Controlla le nuove versioni» e ogni quante ore.
- **Programma Windows**: WebView2 non scrive piu' in stderr.log il messaggio innocuo «Failed to unregister class Chrome_WidgetWin_0. Error = 1412» a ogni chiusura.

## 0.11.78 - 2026-11-02
- **Telefono**: le bande stanno in una riga, i messaggi vanno a capo invece di finire nei «…», niente riga vuota con l'icona in testata, sfumatura che indica il menu delle pagine da scorrere.
- **Programmi collegati, riquadro Integrazioni**: al posto del menu a tendina un pulsante per ogni tipo di programma (flusso WSJT-X, multicast, ADIF su UDP, XML di N1MM) con la sua spiegazione; i testi lunghi passano nel «?».
- **Scheda DXpedition, riquadro del paese**: ora e' una mappa della regione centrata sul paese, con il mare, il reticolo, le terre vicine in grigio, il paese in verde e un punto rosso sulla DXpedition; in basso a destra un mondo in miniatura con un riquadro rosso su dove si trova. Lo zoom segue la grandezza del paese (piu' largo per Russia o Brasile, stretto per le isole piccole) e funziona anche vicino all'antimeridiano (Vanuatu, Fiji).
- **DX Cluster**: colonne Freq, DX e Paese allineate meglio, titolo «Paese» sopra le bandierine, Modo spostato a destra, UTC allineato all'ora.
- **Suggerimenti ovunque**: nella pagina Fox ogni contatore, pulsante, interruttore, riquadro e colonna spiega cosa fa passandoci sopra; suggerimenti anche sulle spie e sugli strumenti dei display della radio e sui controlli di Storico, Esporta, Mappa, Propagazione, Statistiche, cluster, DXCC e Impostazioni, in tutte e sei le lingue.
- **Display FT-2000**: RX e TX accanto alla frequenza principale; spie e riquadri spenti ben visibili; al posto del riquadro FT8/AGC (gia' mostrati altrove) la potenza della radio con i pulsanti − e +.

## 0.11.77 - 2026-11-03
- **Display FT-2000 e IC-7610**: il TUNER/ATU tenuto premuto 1 secondo avvia l'accordatura, come il tasto della radio; la pressione breve lo accende o spegne.
- **Pagina Fox**: STOP TX nello stile neon; riquadri centrali allungabili con la maniglia sotto (l'altezza si ricorda).
- **Sito**: titolo e testi con FT8, FT4, JT65 e JT9, Fox e UltraFOX, scheda CAT con SWR, TUNE e potenza (gia' pubblicato).
- **README**: etichette con il totale dei download e con le visite (visitor-badge.laobi.icu, servizio esterno scelto dal proprietario).

## 0.11.76 - 2026-11-02
- **Accordatore della radio**: accanto ad ATU il pulsante **TUNE** avvia l'accordatura automatica (Hamlib `G TUNE`; provato con Hamlib per FT-2000 e IC-7610, da verificare in aria). Mai durante una trasmissione di Aurora.
- **Potenza dalla radio**: pulsanti **PWR −** e **+** a passi del 5% (Hamlib `L RFPOWER`), con la potenza impostata in percentuale.
- **SWR nel display piccolo**: in trasmissione al posto del modo, ambra da 2 e rosso da 3.

## 0.11.75 - 2026-11-02
- **Etichetta FOX e Hound automatico**: chi manda R+rapporto o RRR non e' mai considerato una Fox (niente piu' FOX su stazioni normali che usano il messaggio doppio).

## 0.11.74 - 2026-11-02
- **Pagina Fox con un aspetto proprio, «Neon UltraFOX»**: viola e ciano su fondo scuro in ogni tema, numeri e pulsanti luminosi, bordo rosa quando si e' in onda.
- **Cascata della pagina Fox**: si alza e si abbassa trascinando la maniglia sotto (l'altezza si ricorda).

## 0.11.73 - 2026-11-02
- **Pagina Informazioni** centrata e ridisegnata (testata con logo e versione, dedica, quattro riquadri, collegamenti), con il collegamento al sito web di Aurora.
- **Sito web**: scheda «Fox e UltraFOX per le DXpedition» nelle sei lingue (gia' pubblicata).

## 0.11.72 - 2026-11-02
- **Fox**: «Abilita Tx» resta acceso al primo clic (nel modo Fox la sequenza normale non parte piu' e non lo rispegne) e il pulsante cambia subito.
- **Pagina Fox in una schermata**: numeri nella testata, poi chi chiama, QSO in corso ben in vista, in onda e coda, messaggi ricevuti; la cascata sotto a tutta larghezza, ridimensionabile trascinando l'angolo (l'altezza si ricorda).
- **Etichetta FOX e Hound automatico**: solo per i messaggi DXpedition sotto i 1000 Hz; le risposte doppie di stazioni normali non sono piu' scambiate per una Fox.

## 0.11.71 - 2026-11-02
- **Pagina Fox ridisegnata e completa**: testata con lo stato (spento, trasmissione spenta, pronto, in onda) e i comandi Fox, Abilita Tx e STOP; opzioni a pulsanti e interruttori; numeri della sessione; cascata con le barre Rx/Tx e i comandi di ricezione; messaggi ricevuti; tabella di chi chiama con SNR colorato; in onda adesso, QSO in corso con i passi e coda. Non serve piu' passare dalla pagina Stazione.
- **Sequenza**: dopo il nostro RRR il 73 del DX chiude e registra il QSO senza mandare un altro 73.
- **Scheda DXpedition**: riquadro con la forma del paese (solo il contorno).
- **Pagina Informazioni**: copyright, licenza GPL-3, assenza di garanzia e codice derivato; crediti completi in `THIRD-PARTY-NOTICES.md`.

## 0.11.70 - 2026-11-02
- **UltraFOX** (compatibile con SuperFox di WSJT-X 3.3): nella pagina Fox la casella «UltraFOX» manda tutte le stazioni del periodo in un solo segnale largo (da 750 a 2260 Hz, circa 13 s); in Hound (FT8) Aurora decodifica anche le Fox UltraFOX/SuperFox e risponde come a quelle classiche. In fase di prova; la firma digitale non e' ancora gestita.
- **Zona di trasmissione**: a QSO finito e registrato DX e messaggi si cancellano da soli appena la sequenza smette di trasmettere verso quella stazione; le chiusure (RRR/RR73/73) verso una stazione gia' a log non si ripetono piu' all'infinito (al massimo 3, o il limite scelto).
- **CAT**: se la radio o il programma che la condivide (es. Log4OM) non risponde, Aurora riprova da sola ogni 5 s finche' non si preme Scollega.
- **Scheda DXpedition**: cartina chiara (Esri) senza linee e stato in quattro riquadri (stato, giorni mancanti, sentita, ultimo spot).
- Pagina Fox: corretta la tendina «CQ verso» (le opzioni venivano rifiutate).

## 0.11.69 - 2026-11-01
- **Fox**: la pagina si apre subito (prima restava su «Il motore non ha ancora mandato lo stato Fox»); il motore manda lo stato ogni 2 s con l'interruttore letto subito. Nel menu Fox sta dopo Stazione, con la spiegazione al passaggio del mouse.
- **Scheda DXpedition**: mappa disegnata dal programma, senza la cartina National Geographic e senza tropici, equatore e circoli polari.

## 0.11.68 - 2026-11-01
- **Modo Fox classico (WSJT-X 3.3)**: nuova pagina Fox nel menu in alto. Coda dei Hound, fino a 5 stazioni nella stessa trasmissione, messaggio combinato «RR73; ...», CQ nei posti liberi, QSO a log da soli. Evento `fox` e `POST /api/v1/fox`. In fase di prova; SuperFox escluso.
- **Splash**: elenca tutti i modi (FT8, FT4, JT65, JT9).
- **Comandi**: spot DX e commento solo con lo spot acceso e il DX Cluster collegato.
- **Cascata**: pulsante «Stile WSJT-X» (righe a blocchi, Flatten, tavolozza Default, riga di inizio periodo).
- Guida: nuova sezione «Fare la Fox».

## 0.11.67 - 2026-11-01
- **Scheda della DXpedition**: la mappa e' quella della pagina Propagazione (terreno e confini veri, zona di notte, linea dal proprio QTH con la DXpedition selezionata) al posto della mappa disegnata a mano, che aveva difetti grafici.

## 0.11.66 - 2026-11-01
- **Zoom del display come una lente**: passando il mouse si apre una copia identica del display ingrandita 2 volte in larghezza e in altezza (CSS `zoom`), invece di una versione solo allargata e stirata.

## 0.11.65 - 2026-11-01
- **Scheda della DXpedition**: corretto il blocco della 0.11.64 (la scheda apriva una pagina vuota sopra a tutto). Ora la mappa prende centro e zoom prima di linee e punti, la scheda sta sopra a tutto e una rete di sicurezza la chiude se qualcosa va storto.
- **DX Cluster**: il doppio clic su uno spot FT4 o FT8 cambia anche il modo, prima della frequenza (cosi' il cambio di modo non riporta la radio sulla frequenza standard).
- **Zoom del display** il doppio di prima; **pannello QSO** con i pulsanti di trasmissione sotto la sequenza, piu' larghi e leggibili.

## 0.11.64 - 2026-11-01
- **Scheda della DXpedition**: un clic su una DXpedition apre la scheda con la mappa (linea dal proprio QTH e riquadro ingrandito sul paese), paese, continente e zone, date, note e QSL di NG3K, distanza e direzione, ultima volta sentita e «Vai su ... kHz» dall'ultimo spot del cluster. Il core da' latitudine e longitudine del paese (`/api/v1/dxcc`, da cty.dat).
- **Display con zoom**: passando il mouse sul display della radio si apre una copia piu' grande (circa 44rem) con gli stessi indicatori cliccabili.
- **Menu in alto**: solo testo, la pagina scelta sottolineata con i colori dell'aurora.
- Pannello QSO con testo piu' piccolo (i nomi entrano interi); AUTO del livello d'ingresso chiaro acceso/spento; «by IU3CYV» piu' leggibile; tendina della lingua nelle Impostazioni non piu' tagliata; con i pannelli FT-2000 e IC-7610 la disposizione del display e' spenta con la spiegazione.

## 0.11.63 - 2026-11-01
- **Pagina Log**: bandiera accanto al paese, griglia con bordi, intestazione colorata e righe alterne, Locatore staccato da Ric., pagina larga al massimo 72rem e centrata.

## 0.11.62 - 2026-11-01
- **Pannello Comandi come WSJT-X 3.3**: DX Call e locatore scrivibili (Invio prepara i messaggi senza trasmettere), Lookup su QRZ.com, rapporto da inviare scelto a mano, Hold Tx Freq, tendina del tipo di CQ (CQ, DX, continenti, POTA, SOTA, TEST), Tune e Log QSO a scelta (Impostazioni > Generali, spenti di base); ogni voce con il «?» in sei lingue. Nuove API `/api/v1/commands/dx`, `report`, `tune`, `log-qso` e interruttore `hold_tx`.
- **Pannello QSO**: sequenza in due colonne a sinistra, pulsanti che fanno trasmettere in colonna a destra, misure fisse.
- **Hound**: il Tx1 non appare piu' barrato (WSJT-X 3.3 non permette di saltarlo, `elide_tx1_not_allowed`) e il Tx6 (CQ) e' spento.
- **Righe Tx**: paese, bandiera e distanza della stazione chiamata (prima la distanza dal proprio locatore e nessun paese).
- **Bandiere**: riconosciute anche le grafie di NG3K (Gambia, Solomon Is, eSwatini...); bandiera, nominativo e paese allineati nella scheda DXCC.
- Guida: nuova sezione «Il pannello Comandi».

## 0.11.61 - 2026-11-01
- **R+rapporto subito dopo il Tx1**: se la stazione chiamata risponde direttamente con R+rapporto (MSHV, molte DXpedition, caso V26JJ) Aurora chiude con RR73 e registra, come WSJT-X 3.3 (DecodedMessageReaction.cpp); prima restava sul Tx1 come JTDX.
- **Wait and Pounce** (WSJT-X 3.3, `updateRespondTarget`, `AutoRespondPolicy.hpp`, `AutoRespondScoring.hpp`): nuovo pulsante «Wait and Pounce» tra gli interruttori del QSO; armato con Abilita Tx spento, a fine periodo risponde da solo al CQ scelto con la voce «Wait and Pounce: a quale CQ» (scheda Generali: nessuno, il primo, il più lontano, il più forte, il più debole; di fabbrica il primo), saltando le stazioni già collegate nella sessione; senza risposta entro 6,2 periodi la trasmissione si spegne; scattato, il pulsante si disarma.
- **Hound automatico**: quando il DX scelto trasmette come Fox di WSJT-X (righe «A RR73; B <FOX> -08») Hound si accende da solo, la Tx sale sopra 1000 Hz e Split si accende, con l'avviso «… lavora in Fox/Hound: Hound acceso»; se il DX risponde con R+rapporto (una stazione normale, per esempio le risposte multiple di MSHV) o si sceglie un'altra stazione Hound si spegne da solo, prima di preparare il messaggio (così «Salta Tx1» vale subito). Il pulsante Hound forza a mano finché non cambia il DX. Voce «Hound automatico» nella scheda Generali, accesa di fabbrica.
- **Hound a mano come WSJT-X**: accendendo Hound una Tx sotto 1000 Hz sale fra 1000 e 3000 Hz (`QRandomGenerator::bounded (1000, 2999)` di WSJT-X), Split si accende se era spento e un avviso dice cosa è stato fatto; in Hound una chiamata non parte mai sotto 1000 Hz. Con Hound acceso «Salta Tx1» è disattivato con la spiegazione.
- **Segnale FOX**: nella lista dei messaggi le righe delle stazioni in Fox/Hound di WSJT-X hanno l'etichetta arancione FOX e uno sfondo proprio; nel pannello Trasmissione il badge «FOX · Hound acceso» (o «FOX · accendi Hound») se il DX scelto è una Fox, altrimenti «Hound» quando è acceso; nella scheda DXpedition «Fox/Hound» accanto alla spedizione sentita trasmettere così. Le risposte multiple normali non hanno l'etichetta.
- **ALL.TXT con la banda del periodo**: i messaggi di un periodo decodificati dopo un cambio di banda si scrivono sotto la frequenza di quel periodo (frequenza di metà periodo), poi torna la riga della banda nuova.
- **Scambi di contest nel log**: lo scambio ricevuto dal DX (`579 WI`, `559 0013`, `2A EMA`, `570123 JN65AB`) entra nel QSO (`exch_rcvd`) e nei record ADIF con i campi di WSJT-X 3.2 (`STX`/`SRX`, `STX_STRING`/`SRX_STRING`, Field Day, RTTY Roundup). Il motore non fa ancora la sequenza dei contest, quindi lo scambio inviato resta vuoto.
- **Misura della decodifica FT8 su banda piena vera**: prova `banda_vera_da_wav` (WAV fuori dal repository) e numeri in `docs/analisi-decodium-decoder.md`.
- **API**: nuovi interruttori `pounce`; voci `PouncePolicy` e `HoundAuto` di `/api/v1/engine-settings`; avvisi `hound_on`, `fox_detected`, `hound_off` in `tx_alerts`; campi `exch_sent`/`exch_rcvd` del QSO.
- **Guida**: Wait and Pounce; la sezione Hound spiega che di solito Aurora accende e spegne Hound da sola.
- Tolto il pulsante «Sinc» (non collegato al motore); DXpedition piu' compatte.


## 0.11.60 - 2026-11-01
- **«Log» nel menu in alto al posto di «Storico»**: apre tutti i QSO con filtri ed esportazione ADIF; il pulsante «Esporta ADIF» sparisce dalla pagina Stazione (resta il totale dei QSO). Lo storico dei messaggi ricevuti esce dal menu.
- **Impostazioni riordinate per argomento**: il menu ha sei gruppi con titolo, Stazione (Generali, Radio, Audio, Orologio), Ricezione (Cascata, Filtri, Bande, Frequenze), Trasmissione e QSO (Sequenza, Macro TX, Programmazione), Avvisi (Notifiche, Desiderati, DXCC), Rete e log (DX Cluster, Servizi online, Servizi di log, Programmi collegati) e Sistema (Accesso, Finestre, Aggiornamenti, Informazioni). La pagina Avanzate non c'e' piu' (il vecchio indirizzo apre Filtri). La ricerca trova anche le singole voci, comprese quelle spostate.
- **Nuova pagina «Servizi di log»**: eQSL, Wavelog/Cloudlog, ClubLog, QRZ.com e LoTW, un riquadro per servizio con interruttore, campi, «Prova» e stato dell'ultimo invio; le spiegazioni sono nel «?» del titolo.
- **«Segnalazioni» diventa «Programmi collegati»**: server UDP primario, ADIF via UDP, server TCP (N1MM) e Integrazioni. Gli spot su PSK Reporter e DXSummit sono in Servizi online; registrazione del QSO e ALL.TXT in Sequenza.
- **Voci spostate dove servono**: monitoraggio Tx, cambi di frequenza Tx, limite ROS, full duplex, Wait features, verifica del Fox e Band Hopping passano da Generali a Sequenza; l'avvio della decodifica FT8 passa in Filtri (riquadro «Decodifica»).
- **Niente piu' riquadri «Non disponibili in Aurora»**: le voci di JTDX che Aurora non usa non compaiono in nessuna pagina.
- **Errori del salvataggio chiari**: quando un valore e' rifiutato compare il motivo vero accanto alla voce, con il suo nome (per esempio «Server UDP2: serve un indirizzo IP (o localhost)»), non piu' un messaggio generico in fondo alla pagina. Un valore vecchio e incoerente di un servizio (per esempio importato da JTDX) non blocca piu' il salvataggio delle altre voci: i controlli fra piu' voci valgono solo per il servizio che si sta cambiando.

## 0.11.59 - 2026-11-01
- **DX Cluster**: le DXpedition sono solo evidenziate nel colore scelto, senza l'etichetta «DXped» che tagliava il nominativo.
- **Barre Rx e Tx**: mentre si trascinano, l'etichetta con gli Hz diventa piena del colore della barra e si legge bene; al rilascio torna trasparente.
- **Display della radio piu' reattivo**: con il CAT su USB, rete o seriale da 19200 baud in su si leggono 6 valori in piu' per giro invece di 3 (PRE, ATT, AGC, NB, NR, NOTCH, VFO B...): il display si aggiorna in circa la meta' del tempo. Le seriali lente restano a uno per giro per non disturbare il PTT.

## 0.11.58 - 2026-11-01
- **DX Cluster**: gli spot delle DXpedition in onda secondo il calendario NG3K si evidenziano con l'etichetta «DXped», nel colore scelto in Impostazioni > DX Cluster (tavolozza fluo: giallo di base, verde, arancio, rosa, azzurro, viola, oppure un colore qualsiasi).
- **Riquadro CQ**: con le DXpedition affiancate il messaggio CQ si legge intero, la colonna del paese si stringe.
- **Barre Rx e Tx fluide**: trascinandole sulla cascata la barra segue subito il mouse e la frequenza va al motore al massimo ogni 80 ms (prima si aspettava la risposta del motore a ogni movimento e la barra andava a scatti).
- **Due nuovi stili del display della radio a pannello unico**: «Pannello Yaesu FT-2000» (cifre a 7 segmenti azzurre, riquadri ANT/ATT/FLT/IPO/R.FLT/AGC con valori ambra, spie PROC/TUNER/BUSY/NB/NR/NOTCH, scala di sintonia con riquadro CLAR, VFO B con S-meter a barra, riquadro modo e AGC) e «Pannello Icom IC-7610» (MAIN e SUB affiancati con S-meter ad arco, modo in riquadro azzurro, colonna ANT/P.AMP/ATT/IP+/AGC/VOX/COMP, spia TX, DUAL-W e SPLIT). In testata prendono il posto dei due blocchi alla stessa misura (40rem x 4rem); gli indicatori restano cliccabili come nel display esteso; i dati che la radio non da' restano spenti. Scelta in Impostazioni > Radio con anteprima, testi in 6 lingue.

## 0.11.57 - 2026-11-01
- **Cascata**: la riga verde di inizio periodo si puo' spegnere (Impostazioni > Cascata > «Riga di inizio periodo», accesa di base come JTDX).
- **Anteprima dal vivo della cascata nelle Impostazioni**: in Impostazioni > Cascata il pulsante «Mostra anteprima dal vivo» apre la cascata e lo spettro veri (stessi componenti della pagina Stazione, righe del motore), fissi in cima mentre si cambiano le opzioni: ogni modifica si vede subito. La scelta aperta/chiusa resta ricordata; l'anteprima si spegne del tutto (renderer e abbonamento alle righe) uscendo dalla scheda o con la finestra nascosta, e senza righe dal motore dice di accendere radio e audio.
- **Flatten sul tratto visibile** (WSJT-X v3.3.0-beta5, `widgets/plotter.cpp`, `izVisible`/`savgBins`): scala e appiattimento della cascata e dello spettro si calcolano solo sulle colonne visibili con dati, cosi' un bordo senza canali non piega la linea di base.
- **Band Hopping con il tasto destro**: clic destro sulla barra delle bande per passare subito alla voce successiva dell'elenco (nuovo comando `POST /api/v1/commands/band-hop-next`), anche col giro automatico spento; non in trasmissione.
- **Stazione Wavelog dalla tendina**: in Impostazioni > Segnalazioni il numero della stazione Wavelog/Cloudlog si sceglie da un elenco letto dal server con la chiave salvata (nuovo `POST /api/v1/qso-upload/wavelog-stations`, solo dalla rete di casa); se il server non risponde resta la casella del numero.
## 0.11.56 - 2026-11-01
- **Momento di avvio della decodifica FT8** (WSJT-X 3.3, `m_ft8DecoderStart`/`fixStop`): nuova voce «Avvio della decodifica FT8» nella scheda Generali, Presto (13,82 s), Normale (14,11 s, di fabbrica come WSJT-X) o Tardi (14,40 s); il periodo si consegna al decoder in quel momento (prima sempre a 14,2 s). FT4 e JT invariati.
- **Full duplex per i satelliti** (WSJT-X, `actionFull_Duplex_Mode`): spento (di fabbrica) il periodo in cui si e' trasmesso non si decodifica piu', come WSJT-X che spegne il monitor in trasmissione; acceso si decodifica e si mostra, ma la sequenza non reagisce a quei messaggi (`fullDuplexBlocked`).
- **Wait and Reply e Wait and Call** (WSJT-X 3.3, `processWaitReplyCall`, `planWaitReplyCall`, `WaitFeaturePolicy.hpp`): con un DX impostato e Abilita Tx spento, se il DX ti chiama il QSO riprende da solo; nuovo pulsante «Wait and Call» tra gli interruttori del QSO: quando il DX chiama CQ o chiude il suo QSO (RR73, RRR, 73) lo si chiama da soli per 6,2 periodi, poi la trasmissione si spegne se non ha risposto. Voce «Wait and Reply / Wait and Call» nella scheda Generali, accesa di fabbrica come in WSJT-X.
- **Verifica del Fox con OTP (Hound classico)** (WSJT-X 3.3, `processSFoxVerification`, `handleVerifyMsg`, `Network/FoxVerifier.cpp`): in Hound il codice `CALL.123456` del Fox si nasconde (voce «Mostra i messaggi OTP», spenta di fabbrica) e, se non e' `000000` e «Verifica in linea» e' accesa, si chiede `GET {url}/check/{CALL}/{ora ISO}/{codice}.text` (5 s, servizio di fabbrica `https://www.9dx.cc`); l'esito compare nella pagina Stazione (verde verificato, rosso non valido) e come riga «CALL verified/invalid». Il formato SuperFox (`$VERIFY$`) resta da fare.
- **Band Hopping FT8/FT4** (WSJT-X 3.3, `widgets/mainwindow_bandhopping.cpp`): voce «Band Hopping acceso» con l'elenco di bande («FT8 20m», «FT4 40m») e fino a 8 frequenze proprie in kHz; ogni due minuti (minuti pari) la radio passa alla voce successiva, mai con Abilita Tx acceso o in trasmissione. Spento di fabbrica.
- **API**: lo `status` porta `fox_verification` (`call`, `verified`); nuove voci `FT8DecoderStart`, `FullDuplex`, `WaitFeaturesEnabled`, `ShowOTP`, `OTPVerify`, `OTPUrl`, `BandHopping`, `BandHopList` di `/api/v1/engine-settings`; nuovo interruttore `wait_call`.
- **Guida**: nuovi passi su avvio della decodifica e Full duplex, Band Hopping, Wait and Reply / Wait and Call, verifica OTP del Fox, in sei lingue.

## 0.11.55 - 2026-11-01
- **Cascata: la storia resta al ridimensionamento** (come `CPlotter::resizeEvent` di WSJT-X 3.2): le righe ricevute stanno in un anello condiviso fuori dal renderer, cosi' ridimensionando la finestra, passando tra vista a finestre e a schede o spostando il pannello la cascata nuova riparte con tutte le righe gia' disegnate, comprese le righe verdi di inizio periodo.
- **Filtri dei messaggi come WSJT-X 3** (Impostazioni > Filtri, salvati da soli, «?» in sei lingue): «Mostra solo questi nominativi» (Whitelist, con prefissi JA*), «Mostra sempre questi nominativi» (Always Pass, vince sugli altri filtri), «nascondi anche i loro scambi» per le stazioni gia' collegate (Hide B4 su tutti i messaggi, non solo i CQ), «Sospendi tutti i filtri» (Bypass) e il nuovo riquadro «Evidenzia nell'elenco» per nominativi, prefissi e paesi (righe viola, come Highlight Whitelist entries e Highlight Territory). Si affiancano ai filtri per continente, paese, nominativo e ai colori di DXCC, desiderati e gia' collegati gia' presenti. Nuova voce nella Guida.
- **CAT, rifiniture dalla 3.2 di WSJT-X**: con «modo dati» il profilo DATA2 o DATA3 scelto sulla radio (USB-D2/D3, radio con piu' profili come la TS-990S) resta dopo un cambio di frequenza invece di tornare a DATA1; con lo split della radio i ruoli dei VFO restano quelli scelti (chi riceve sul VFO B trasmette sul VFO A) e non si scambiano durante il PTT, mentre il VFO di trasmissione segue l'offset Tx anche in trasmissione; dopo una lettura fallita della radio un PTT in attesa si scarta; frequenze nulle o fuori scala e offset audio oltre 6000 Hz si rifiutano prima di arrivare alla radio.
- **Hamlib 4.7.2 ufficiale** nell'installatore di Windows (prima 4.6 da JTDX): `scripts/prepara-hamlib.ps1` scarica `hamlib-w64-4.7.2.zip` dalla release ufficiale di GitHub e ne verifica lo SHA-256; `-Hamlib <cartella>` e `-SoloLocale` per usare JTDX o WSJT-X installati. 313 modelli. Dettagli in `docs/cat-radio.md`.
- **PSK Reporter**: pacchetti al massimo di 952 byte (passano anche sulle linee con pacchetti piccoli); l'ora dello spot e' quella del periodo ricevuto, con la data giusta a cavallo della mezzanotte; i messaggi dei periodi prima di un cambio di banda o di modo si segnalano con la banda e il modo di allora.
- **Wavelog/Cloudlog**: la prova controlla anche che la chiave abbia il permesso di scrittura e, se il numero della stazione manca o e' sbagliato, elenca le stazioni lette dal server.
- **ADIF**: `ADIF_VER` 3.1.8 nell'esportazione e nell'invio a eQSL; regole degli scambi di contest di WSJT-X 3.2 pronte nel registro (non ancora usate nei record).
- **Diagnostica sempre accesa** (programma Windows): nella cartella `%APPDATA%\Aurora\crash` un guardiano scrive minidump e rapporto quando il runtime o la finestra restano fermi oltre 20 s (uno per blocco), un filtro scrive minidump e rapporto delle eccezioni non gestite, ogni panic lascia un `panic-….txt` con backtrace e l'errore standard va in `stderr.log` (li' Rust scrive il motivo degli `abort`, i fast-fail c0000409). Si tengono 10 dump e 50 testi. In Impostazioni > Aggiornamenti il pulsante «Apri la cartella dei crash» (con «?» in sei lingue); API `GET /api/v1/diagnostics` e `POST /api/v1/diagnostics/crash-folder` (solo dal PC). Prova dal vivo con `AURORA_PROVA_BLOCCO=1`. Dettagli in `docs/diagnostica.md`.
- **Blocco della trasmissione da un controller UDP** (WSJT-X 3.3, tipi 17 e 18; motore proprio, solo con «Accetta richieste UDP»): un TxInhibit (18) prenota il blocco per 100-30000 ms per controller (fino a 64 seguiti uno per uno, oltre una prenotazione comune), si rinnova, scade o si rilascia con durata 0; i blocchi si sommano (basta uno). Con un blocco attivo il PTT resta spento o si spegne subito e torna quando l'ultimo scade o si rilascia. Comandi malformati o fuori limite si ignorano e si contano. Aurora manda InhibitStatus (17) all'avvio, dopo ogni Heartbeat, a ogni cambio e un ultimo «non supportato» alla chiusura. Nel pannello Tx compare «Tx bloccata da …».
- **Messaggi non rappresentabili bloccati prima del PTT** (come WSJT-X 3.2): ogni messaggio FT8/FT4 si impacchetta e si rilegge prima di trasmettere; se non arriverebbe uguale (nominativo troppo lungo o composto non standard, prefisso o suffisso, rapporto fuori scala) non parte, la sequenza si spegne e un avviso nel pannello Tx dice quale messaggio, in sei lingue.
- **Stop per ROS alto** (come WSJT-X): con il CAT Hamlib che legge il ROS, in trasmissione un valore oltre la soglia ferma subito la Tx (audio, PTT e sequenza) e mostra l'avviso con ROS e soglia. Soglia in Impostazioni > Generali > «Ferma la Tx con ROS oltre» (di fabbrica 2,5, 0 = mai, con «?» in sei lingue, salvata da sola). Potenza e ROS restano visibili nel display della radio come prima.
- **API**: lo `status` porta `tx_alerts` (elenco di avvisi `unrepresentable`, `high_swr`, `inhibited` con `detail`, `value`, `limit`); nuova voce `SWRLimit` di `/api/v1/engine-settings`.
- **Guida**: nuovo capitolo su Fox e Hound (cosa sono, come collegare una DXpedition passo per passo), in sei lingue.
- **Decodifica FT8 "a8" del corrispondente** (porting di `ft8_a8d.f90` di WSJT-X 3.3, GPL 3): con DX Call e DX Grid compilati, a fine periodo si provano sul residuo, a Rx +-10 Hz, i circa 130 messaggi che il DX puo' mandarci (RRR, RR73, 73, CQ, locatore, rapporti da -30 a +30) per correlazione con l'onda attesa; si accetta solo se i simboli attesi spiegano lo spettro (`plog`, simboli sbagliati, `sigobig` come WSJT-X). Su audio sintetico il DX a -23 dB passa da 0 a 20 decode su 20, a -24 dB da 0 a 15, a -25 dB da 0 a 7, senza falsi (anche su 50 periodi di solo rumore). Non in Hound.
- **Decodifica FT8 "a7"** (porting di `ft8_a7.f90` di WSJT-X 3.3): i decode di ogni parita' si annotano e nel periodo successivo della stessa parita' si cercano i loro seguiti (circa 206 messaggi per coppia di nominativi) per distanza dalle metriche dei bit. Su audio sintetico il seguito a -23 dB passa da 0 a 13 decode su 20, a -24 dB da 0 a 9; nessun falso su 50 periodi di solo rumore dopo una banda affollata. I messaggi trovati con a7/a8 hanno il segno `*`.
- **Decodifica FT8 multithread**: i candidati di ogni passata si provano in parallelo (rayon) e si accettano nell'ordine del singolo thread, rifacendo quelli che hanno letto uno stato poi cambiato (spettro rifatto dopo una sottrazione vicina, hash, QSO decodificato): i decode sono identici a quelli del singolo thread (verificato in prova con 2, 4 e 8 thread). Su una banda sintetica di 30 segnali, profondita' 3: 1145 ms con un thread, 726 ms con 4, 637 ms con 8.
- **Interruttori «a7/a8» e «Multithread»** nella riga di ricezione (solo FT8, accesi di base come in WSJT-X), sei lingue.

## 0.11.54 - 2026-11-01
- **Dedica ad Aurora**, la figlia dell'autore da cui prende il nome il programma: nella pagina Informazioni e nei README, in sei lingue.
- **Pagina di esportazione del log ADIF** (`#/esporta`, dal pulsante «Esporta ADIF» nella barra del pannello Log e da Statistiche): nel programma Windows «Scarica ADIF» non salvava niente, perche' un link di download nella finestra del programma non apre la scelta del file. La pagina elenca i QSO del log interno (data, ora, nominativo, banda, modo, rapporti, locatore, paese) con una casella per riga, «Seleziona tutti» e «Nessuno», ricerca per nominativo, filtri per data (dal/al), banda e modo e il conteggio dei selezionati; «Salva ADIF…» apre la finestra «Salva con nome» di Windows per scegliere cartella e nome (proposto `aurora_log_AAAAMMGG.adi`) e salva solo i QSO selezionati, con un messaggio che dice il percorso. Dal telefono o da un browser resta il download dei QSO selezionati. Se il log e' vuoto la pagina lo dice. Sei lingue, «?» su date e rapporti. L'importazione resta in Statistiche.
- **API**: nuove rotte `GET /api/v1/log/qsos` e `POST /api/v1/log/export-save` (solo dal PC); `GET /api/v1/log/export.adi` accetta `ids` (anche a intervalli, `3,7,10-25`).

- **Invio diretto dei QSO** (scheda Impostazioni > Segnalazioni, motore proprio): ogni QSO messo a log parte subito verso **Wavelog/Cloudlog** (API `index.php/api/qso` con chiave e numero della stazione), **ClubLog** (invio in tempo reale, chiave API dell'applicazione configurabile), **QRZ.com Logbook** (`ACTION=INSERT`) e **LoTW** (firma e caricamento con TQSL installato: `tqsl -d -u -a compliant -x -l <posizione> <file.adi>`, percorso cercato da solo). Ogni servizio ha interruttore (spento di fabbrica), campi, pulsante «Prova» (senza scrivere QSO dove il servizio lo permette), stato dell'ultimo invio con i QSO in coda e spiegazione «?», in sei lingue. Chiavi e password protette con DPAPI come quella di eQSL. Se la rete manca i QSO restano in una coda salvata su file (`invii-in-coda.json`) e si riprovano con attese crescenti, anche dopo un riavvio; un errore di credenziali ferma solo quel servizio finche' non si correggono. Rotta nuova `POST /api/v1/qso-upload/test`. Dettagli e limiti in `docs/integrazioni-log.md`.
- **Integrazioni UDP** (Impostazioni > Segnalazioni, nuovo riquadro «Integrazioni», sei lingue, «?» per ogni voce, salvataggio automatico): elenco di destinatari UDP, ciascuno con le sue scelte: flusso WSJT-X, Logged ADIF (12), ADIF su UDP, XML stile N1MM (`contactinfo`, porta di base 12060), unicast o **multicast** con TTL e interfaccia di uscita (piu' programmi sulla stessa porta: GridTracker, QLog, MacLoggerDX, CQRLOG, N1MM+, HRD, JTAlert). I comandi di ritorno arrivano anche dai gruppi multicast. Le variabili `AURORA_UDP_FORWARD` (oltre il primo indirizzo) e `AURORA_EXPORT_UDP` sono ora solo i valori iniziali dell'elenco.
- **Nuovi comandi UDP in ingresso** (solo con «Accetta richieste UDP»): Highlight Callsign (13) colora le righe del nominativo nella lista dei messaggi (sfondo e testo, «solo l'ultima», tolto con colori non validi); Clear (3) svuota le liste come «Cancella»; Location (11) cambia il locatore per la sessione; Configure (15) imposta modo FT8/FT4, frequenza Rx, DX call e locatore e genera i messaggi; Switch Configuration (14) si ignora con una riga di log. Nuovi eventi WebSocket `highlight_callsign` e `clear_decodes`.
- **QSO Logged (5)** con i campi di WSJT-X 2.x in coda: Exchange sent, Exchange received, ADIF Propagation mode (vuoti).

## 0.11.53 - 2026-11-01
- **Display a doppio VFO**: con il tema chiaro il blocco unico mostrava ancora la cornice del display; ora e' un solo schermo continuo.

## 0.11.52 - 2026-11-01
- **Tasto destro sulla cascata = frequenza Tx**, come in JTDX: il sinistro sposta la barra Rx, il destro la Tx, tenendo premuto la barra segue il mouse. La Tx si sposta anche durante la trasmissione: il messaggio in corso si rimodula subito alla nuova frequenza (con «cambi di frequenza Tx durante la trasmissione» acceso, come di fabbrica).
- **Display della radio in due disposizioni** (Impostazioni > Radio > «Disposizione in testata»): blocco unico a doppio VFO come una FT-2000 (VFO A grande a sinistra, VFO B e indicatori a destra in un solo schermo continuo, senza separatori, scelta di fabbrica) oppure due blocchi uguali affiancati. Il display esteso torna in alto a destra accanto al display e non occupa piu' la riga dei controlli.
- **Guida aggiornata** con le funzioni dalla 0.11.41 alla 0.11.51, in sei lingue.

## 0.11.51 - 2026-11-01
- **Cascata di nuovo visibile**: con la 0.11.50 restava nera perche' l'interfaccia scartava le righe del nuovo spettro (6827 canali, il limite era 4096); ora le accetta fino a 8192 come il core.
- **Log interno nel pannello Log**: senza log esterno collegato (`AURORA_LOG4OM_DB`) il pannello mostra il log di Aurora, con paese, rapporti e totale; il titolo del pannello e' ora «Log».
- **Totale dei QSO ed esportazione ADIF nella barra del pannello Log**: non coprono piu' la tabella.
- **Sito**: pubblicato dal ramo `gh-pages` (le modifiche a `main` non fanno piu' partire distribuzioni che si accavallano). Ricerca delle integrazioni con altri programmi di log in `docs/integrazioni-log.md`.

## 0.11.50 - 2026-11-01
- **Cascata e spettro come il Wide Graph di JTDX** (letto con REA in `jtdx.exe` e nei sorgenti GPL): i segnali FT8 non sono piu' blob larghi e saturi ma righe sottili e leggibili. Causa: la finestra era di 0,68 s (Hann) contro i 0,16 s di un simbolo FT8, quindi ogni riga mescolava 4 toni, in piu' un massimo su piu' canali e una scala dei colori di 5,7 livelli per dB. Ora il core calcola lo spettro di `symspec.f90` (finestra gaussiana di 0,144 s, canali da 0,73 Hz, 6827 canali da 0 a 5000 Hz) e l'interfaccia lo disegna con le stesse formule: canali per pixel, N Avg, Scale (mediana), Flatten, indice di colore `10*Gain*y + 10*Zero + 40` sulla tavolozza da 256 colori. Nuova scheda **Impostazioni > Cascata** (6 lingue, «?» per ogni voce, valori di fabbrica e limiti di JTDX, ricordata al riavvio, applicata subito): Bins/Pixel (5), Start, N Avg, tavolozza (le 30 di JTDX + una tua), Gain, Zero, Scale, Flatten, grafico dello spettro sotto la cascata (Spec % 30, Current/Cumulative, Gain e Zero), Bars e Freq. Barre Rx/Tx larghe come in JTDX (FT8 43,75 Hz, FT4 62,5 Hz, JT65 177,65 Hz, JT9 15,6 Hz) e solo bordi, due linee bianche che seguono il mouse, finestra del Filtro asimmetrica (FT8 -60/+110 Hz, Hound -290/+340, FT4 -95/+179), riga verde a ogni inizio di periodo. Analisi in `docs/analisi-jtdx-reverse-rea.md`. Da provare dal vivo: aspetto dei segnali e velocita' (N Avg 4-5 = velocita' di JTDX); non fatto il timestamp nella cascata.
- **Display esteso cliccabile**: PRE, ATT, AGC, NB, NR, NOTCH, ATU, RIT, XIT e il VFO in uso sono ora pulsanti che cambiano l'impostazione sulla radio via CAT (Hamlib). NB, NR, NOTCH e ATU si invertono; PRE e ATT passano da spento ai valori che la radio accetta (10, 20 dB; per ATT anche 6, 12, 18) e tornano a spento; AGC cicla auto, fast, medium, slow, spento; VFO passa all'altro (A e B, Main e Sub); RIT e XIT si azzerano. I valori che la radio rifiuta vengono saltati; dopo il comando il dato si rilegge subito. Mai in trasmissione, solo dal PC; con OmniRig dice che non e' supportato. Nuova rotta `POST /api/v1/cat/extra`. Spiegazione di ogni clic nel suggerimento, in sei lingue; il display in alto resta senza popup.
- **Sito web e README**: pagina web promozionale in sei lingue (`docs/index.html`, GitHub Pages); il README su GitHub e' in inglese con le altre lingue (`README.it.md`, `.de`, `.fr`, `.es`, `.pt`) e un link diretto che scarica sempre l'ultima versione (`Aurora-Setup.exe` in ogni release).
- **Novita'**: ora le ultime 40 voci. **Librerie**: compilatore Rust 1.99.1 e librerie aggiornate.

## 0.11.49 - 2026-11-01
- **Tutte le impostazioni si salvano da sole**: niente piu' «Applica» o «Salva». Radio (CAT), Audio, Frequenze, Programmazione, Accesso e le impostazioni del motore si applicano dopo una breve pausa e uscendo dalla pagina; «Leggi anche gli altri dati della radio» e «Collega la radio all'avvio» si salvano subito (prima uscendo dalla pagina la spunta si perdeva).

## 0.11.48 - 2026-11-01
- **Pagina Novità** (pulsante dopo la Guida): le ultime 20 funzioni aggiunte, modificate o corrette, a linea del tempo per versione, con schede colorate (nuovo, modificato, corretto) e data e ora del rilascio nell'ora del dispositivo, in sei lingue. L'elenco sta in `app/src/lib/novita.ts` e una prova controlla che la voce piu' recente sia della versione in uso.
- **Display della radio senza popup**: passandoci sopra o cliccando non si apre piu' niente.
- **Niente menu del tasto destro del browser** (Indietro, Ricarica, Ispeziona...): resta solo nei campi di testo, per copia e incolla; il tasto destro sulle righe dei messaggi (coda) funziona come prima.
- **Esporta ADIF piu' facile da trovare**: il riquadro «Log interno di Aurora» della pagina Statistiche e' aperto di base (Scarica ADIF e Importa).
- **Messaggi toast personalizzabili** (Impostazioni > Notifiche, riquadro «Messaggi toast»): quali tipi mostrare (gli errori sempre), quanti secondi restano, posizione (in alto al centro, in alto a destra, in basso) e dimensione, con il tasto «Prova i messaggi».
- **Messaggi toast immediati**: l'avviso compare subito al clic, senza aspettare la risposta del motore; se il comando poi fallisce diventa un errore.
- **CQ e novita' diviso esattamente a meta'**: a sinistra i CQ, a destra le DXpedition; il titolo «DXpedition» sta nella barra del pannello, nella meta' destra, con lo stesso stile di «CQ e novita'». Le DXpedition hanno schede colorate per stato (in onda verde, in arrivo blu, finite grigie, sentita adesso arancio), bandiera e pastiglia con la data.
- **Doppio clic su una stazione**: la risposta parte per prima e subito dopo si accende Abilita Tx (prima le due richieste partivano insieme e Abilita Tx poteva arrivare a stazione ancora non scelta); se non si accende, l'errore si vede.
- **Pastiglia RX/TX piu' piccola**: TX sta nello stesso posto e della stessa misura di RX, senza la scritta STOP; un clic su TX ferma la trasmissione.
- **Impostazioni piu' compatte e ordinate**: la casella di spunta sta subito prima del suo testo (prima era all'altro capo della cella); lingua e tema in un solo riquadro «Aspetto»; tabelle corte (frequenze, programmazione, notifiche) a larghezza contenuta; macro in griglia; i riquadri bassi salgono accanto a quelli alti.
- **Stazione, testata**: modi, bande e controlli di ricezione stretti uno sotto l'altro a sinistra, display piu' basso a destra; il display esteso ha un'area fissa a due righe con tutti i pulsanti (PRE, ATT, AGC, NB, NR...) e senza il doppione dello S-meter e della potenza; se le voci sono accese ma i dati extra della radio sono spenti, lo dice.
- **Radio, riquadro Collegamento**: le caselle hanno la larghezza che serve; larghe per sistema di controllo, modello e porta (il testo si legge intero), strette per bit di dati, bit di stop, velocita' e indirizzo CI-V; il tutto su due righe.

## 0.11.47 - 2026-11-01
- **Tutta la radio nella scheda Radio**: dal CAT al display, anche «Display esteso: cosa mostrare» e la scelta dello stile del display sono ora in Impostazioni > Radio (non piu' in Generali).
- **Testata compatta e a misura fissa**: righe di modi, bande e pulsanti vicine; il display e' meno alto e il display esteso ha altezza fissa (quello che non entra scorre dentro il riquadro), cosi' la pagina non si allunga.
- **Spiegazioni dei «?» di nuovo visibili**: la nuvoletta restava tagliata dentro l'etichetta; ora compare sopra a tutto, centrata sul «?», passandoci sopra col mouse (un clic la tiene aperta).

## 0.11.46 - 2026-11-01
- **Display esteso della radio**: sotto il display in alto secondo VFO (piu' piccolo), strumenti, PRE/ATT, AGC, NB/NR/notch, guadagni, RIT/XIT, ATU e VFO in uso. Ogni voce si accende in Impostazioni > Generali > «Display esteso: cosa mostrare», con la spiegazione; i dati in piu' si leggono con «Leggi anche gli altri dati della radio» (scheda Radio, CAT).
- DXpedition: il pannello non resta piu' su «Scarico il calendario...».
- Trasmissione: la riga del DX con Tx/Rx resta dentro il riquadro.

## 0.11.45 - 2026-11-01
- **DXpedition**: calendario NG3K (Impostazioni > Servizi online, spento di base); il pannello «CQ e novita'» si divide: CQ a sinistra, DXpedition a destra (in onda, sentite adesso in cima, in arrivo, storico da scorrere).
- **STOP**: niente pulsante a parte; in onda la pastiglia rossa TX e' anche lo STOP.
- **Notifiche**: stile «pillola aurora»; nessuna notifica a ogni spostamento delle barre Rx/Tx; il doppio clic dice «Rispondo a NOMINATIVO» e l'avviso sui messaggi non CQ e' separato e chiaro.
- **DX Cluster**: frequenza come testo normale, senza pillola gialla.
- Motore: lettura dei dati in piu' della radio (preamp, ATT, AGC, NB/NR, guadagni, RIT/XIT, secondo VFO) pronta ma spenta: il display esteso arriva nella prossima versione.

## 0.11.44 - 2026-11-01
- **Km e azimut** anche sui messaggi senza locatore, col locatore gia' mandato da quella stazione.
- **STOP** nel pannello Trasmissione, accanto a RX/TX.
- **Notifiche** piu' evidenti: piu' in basso, colore pieno, quasi opache.
- **Display della testata** con la larghezza di prima ma piu' alto.
- **Installatore**: chiude anche i rigctld orfani di Aurora e riprova finche' la cartella hamlib si libera (niente piu' «libhamlib-4.dll» bloccata).

## 0.11.43 - 2026-11-01
- **Cascata fedele al filtro della radio**: Aurora legge col CAT (Hamlib) la larghezza del filtro e la cascata mostra quell'ampiezza, fino a 5000 Hz (0-3000 Hz se la radio non la dice).
- **Coda sempre visibile**: nel pannello Trasmissione, sotto il DX, «In coda» con pallini blu numerati e nominativi.
- **rigctld orfano**: se la porta di rigctld e' occupata da un rigctld rimasto dopo un crash, Aurora lo chiude da sola e riprova.
- **Impostazioni**: pagina Informazioni (logo, versione, autore, licenza, collegamenti); spiegazione «?» su tutte le voci di Notifiche, sugli interruttori QSO e sulle preferenze dello schermo.
- Display della radio piu' largo con lo STOP accanto; avvisi sonori e volume nella testata accanto alla lingua; Tx/Rx a destra uno sotto l'altro; meno spazio tra modo/bande e pulsanti.
- DX Cluster con frequenza come testo e intestazioni in linea; log con data senza anno e colonne strette.

## 0.11.42 - 2026-11-01
- **Trasmissione**: RX verde / TX rosso lampeggiante spostato accanto al blocco del DX, come pastiglia ben visibile.
- **DX Cluster**: frequenza piu' stretta, piu' spazio al paese; intestazioni «Paese» e «UTC» allineate.
- **Elenco messaggi**: colonna km fissa (4,3rem) e colonna paese fissa sul nome DXCC piu' lungo; lo spazio avanzato va ai nominativi.
- **Log Log4OM**: data e rapporti piu' compatti, paese leggibile.
- Impostazioni: tolto il doppione di avvisi sonori e volume (il volume resta in «Suoni»).

## 0.11.41 - 2026-11-01
- **Impostazioni riordinate**: barra laterale con ricerca, pagine compatte e un «?» con la spiegazione di ogni voce (anche quelle del motore), in 6 lingue.
- **Coda**: nell'elenco un pallino blu col numero del posto al posto dell'etichetta grande.
- **Notifiche toast** in alto: vetro traslucido colorato per tipo, entrata e uscita animate.
- **Barra in alto**: modo a tendina e bande a pillole piccole.
- **Stato RX/TX**: tolta la striscia «In ascolto / Tx abilitato»; accanto al periodo di trasmissione RX verde e TX rosso lampeggiante.
- **Icone**: avvisi sonori (altoparlante con il volume accanto), tema chiaro/scuro e lingua.
- Intestazione: «by IU3CYV» sotto AURORA.

## 0.11.40 - 2026-10-31
- **Guida riscritta**: 8 capitoli con indice, ricerca e passi pratici numerati, in 6 lingue; i nomi di schede e pulsanti si prendono dall'interfaccia (`app/src/i18n/guide.ts`).
- **Statistiche riorganizzate**: numeri in alto con le barre di avanzamento, bande e scheda DXCC affiancate, paesi mancanti e log ADIF a richiesta.
- **Orologio**: tolte le pillole «In linea» e orologio/DT; un avviso compare solo se l'ora e' fuori tolleranza o non verificata da piu' di un giorno (anche tra un avvio e l'altro), e porta alle impostazioni dell'orologio. Nuova impostazione «Aggiorna l'ora all'avvio di Aurora».

## 0.11.39 - 2026-10-31
- **Propagazione**: «la piu' lontana» anche con il nome del paese.

## 0.11.38 - 2026-10-31
- **Due VFO per i programmi che condividono la radio**: con la condivisione accesa rigctld parte con `--vfo`, come chiede Log4OM con «VFO MODE»; Aurora se ne accorge (`\chk_vfo`) e indica il VFO nei suoi comandi. Prima Log4OM perdeva il primo VFO.
- **Propagazione**: «la piu' lontana» con chi mi ha ricevuto, bandierina e orario; condizioni per banda con la parola intera sotto le barrette (non si taglia piu').

## 0.11.37 - 2026-10-31
- **Propagazione**: stazioni ricevute in al massimo 4 colonne di uguale lunghezza, senza scorrimenti interni che tagliavano la distanza; condizioni per banda leggibili nella barra stretta (la condizione di adesso con un velo di colore).

## 0.11.36 - 2026-10-31
- **Il volume di Windows segue sempre il cursore Pwr**: all'avvio si applicava il livello di fabbrica (50 %) prima di quello salvato, e cursore e scheda non coincidevano. Ora il livello salvato arriva subito alla scheda e si riapplica prima di ogni trasmissione.

## 0.11.35 - 2026-10-31
- **Cursore Pwr = volume di Windows dell'uscita verso la radio**: prima era un'attenuazione del segnale generato (come JTDX), che poteva solo abbassare; il segnale ora resta pieno e il livello lo da' il volume della scheda.
- **S-meter e potenza non spariscono piu'**: un errore momentaneo della radio (-1, frequente con la FT-2000 e con altri programmi sulla porta) toglieva il misuratore per tutta la sessione; ora si abbandona solo dopo 5 errori di fila.

## 0.11.34 - 2026-10-31
- **Radio via Hamlib che "spariva" al riavvio**: il programma Windows imponeva OmniRig trovando JTDX; ora vince la scelta fatta nella pagina della radio. L'elenco dei modelli si legge anche da rigctld se manca rigctl.
- **Installatore**: chiude Aurora e il suo rigctld finche' non sono spariti (prima la DLL di Hamlib restava bloccata: «Errore nell'apertura del file in scrittura: libhamlib-4.dll»).
- **Misuratori dalla radio**: in trasmissione la potenza compare sempre (il CAT sa subito che si trasmette); S-meter e potenza con l'inerzia della radio (salita immediata, discesa lenta).
- Pagina della radio: i modelli trovati compaiono sotto la ricerca, la porta si sceglie da un menu.

## 0.11.33 - 2026-10-31
- **AGCc funziona**: compensazione AGC portata da JTDX (`agccft8` e il ramo `lagcc` di `sync8`) e migliorata. Quando l'AGC della radio cambia il guadagno (una stazione forte che parte o finisce), quei tratti tornano al livello del resto del periodo. Misura tutto il periodo e non solo i bordi, non dipende dal volume d'ingresso e compensa il salto intero. Sul banco di prova, con l'AGC che pompa, a -21 dB da 9 a 18 decodifiche su 20; nessun peggioramento sull'audio senza sbalzi. Solo FT8.
- **Cancella** svuota anche l'elenco delle decodifiche e le righe Tx.

## 0.11.32 - 2026-10-31
- **Programma "sordo" con il Filtro acceso**: il Filtro decodifica solo ±60 Hz attorno a Rx (±290 in Hound). Ora la cascata si scurisce fuori dalla finestra e accanto al pulsante compare un avviso. **AGCc** con il motore proprio non fa niente (la compensazione AGC di JTDX non c'e' ancora): disabilitato.
- **Linguetta viola corretta davvero**: con la sequenza automatica lo stato non portava il testo in onda e la pagina non registrava i Tx: la risposta della stazione chiamata appariva rossa. Ora il motore pubblica il messaggio davvero trasmesso.
- **Suono**: la risposta alla mia chiamata suona sempre, anche da una stazione che aveva gia' fatto suonare altro; la stessa riga ripetuta tace.
- **Doppio clic su R+rapporto** di una stazione diversa dal DX in corso: si resta su RR73, non si torna al rapporto.
- **AutoTX** accende anche Abilita Tx; a QSO finito il CQ automatico riparte solo con AutoTX acceso (scelte diverse da JTDX).
- **Gia' lavorata** segnata in ogni messaggio della stazione, non solo nei CQ.
- Sigla «in coda» solo sull'ultima riga della stazione e piu' larga; pillola dell'orologio leggibile sul tema chiaro.

## 0.11.31 - 2026-10-31
- **Propagazione riorganizzata**: sole, indici, condizioni per banda e statistiche di ricezione ora in un'unica barra stretta a sinistra; mappa piu' corta a destra, con la lista delle stazioni ricevute subito sotto invece che in una sezione separata piu' in basso.

## 0.11.30 - 2026-10-31
- **Quadratini della mappa visibili di nuovo**: l'opacita' bassa (12%-50%) introdotta per togliere l'effetto scacchiera li aveva resi quasi invisibili sullo sfondo reale. Alzata a 35%-70%.
- **Tabella "Chi mi ha ricevuto" ordinata e piu' ricca**: dalla stazione piu' lontana alla piu' vicina, bandierina del paese accanto al nominativo, distanza in km o miglia secondo le preferenze, divisa in piu' colonne affiancate (una tabella ogni 14 righe) per vederne il piu' possibile senza scorrere.

## 0.11.29 - 2026-10-31
- **Scheda Propagazione compatta davvero**: i riquadri dei fatti (raggi X, macchie solari...) e le pillole delle condizioni per banda erano grandi come pulsanti (padding, raggi e font da elemento dominante), non solo i quadranti gia' sistemati nella 0.11.28. Ridotti insieme: matrice delle bande stretta a 28rem invece di 46rem, font e padding dimezzati.

## 0.11.28 - 2026-10-31
- **Quadranti del sole molto piu' piccoli**: erano larghi fino a 46rem (circa 245px l'uno), un elemento dominante della pagina Propagazione invece di un'icona. Ridotti a 20rem (circa 105px l'uno).

## 0.11.27 - 2026-10-31
- **Cronologia tra periodi FT8**: un CQ debole, una chiamata diretta a te o il segnale del canale QSO che non decodificano da soli ora si sommano (in potenza o in ampiezza, come JTDX) con il periodo precedente della stessa parita' se si ripetono li' entro 2 Hz e 0,05 s. E' il meccanismo con cui JTDX cattura i segnali deboli che si ripetono e che Aurora non aveva ancora: sorgente pubblico di JTDX letto alla lettera (`docs/analisi-jtdx-reverse-rea.md`), provato con un CQ che non decodifica da solo ma decodifica sommato al periodo successivo. Restano fuori la ridecodifica col messaggio del periodo prima e il decoder dedicato del canale QSO (`ft8s`): voci successive della stessa analisi.

## 0.11.26 - 2026-10-31
- **Motore FT8 a profondita' 3**: il motore dal vivo restava sulla profondita' 2 di JTDX (6 passate); la profondita' 3 (9 passate, soglia di sincronismo piu' bassa a rotazione, seconda lettura dei simboli a tempo invertito) era gia' scritta e provata ma mai usata. Ora e' quella in uso: decodifica i segnali piu' deboli, a costo di piu' CPU per periodo.
- **Linguetta viola/rossa corretta**: la stazione DX in corso diventava viola su ogni suo messaggio, compreso il primo con cui mi aveva chiamato lei per prima, appena ci rispondevo (es. doppio clic). Ora resta rossa finche' non le trasmetto davvero qualcosa.
- **Quadratini della mappa meno invasivi**: tolto il bordo scuro delle griglie lavorate (dava un effetto a scacchiera) e abbassata l'opacita' (12%-50% invece di 25%-85%).

## 0.11.25 - 2026-10-26
- **Pulsanti JT65 e JT9 che si accendono**: il pulsante del modo in uso non aveva uno sfondo (mancavano i loro colori: ora JT65 verde e JT9 rosa), quindi sembrava che il clic non facesse niente.
- **Barra del periodo a 60 secondi in JT65 e JT9** (prima contava fino a 15) e, con la pagina vuota, un testo che dice che in JT65 e JT9 si decodifica a fine minuto e che dopo il cambio di modo servono da uno a due minuti (serve un minuto intero dall'inizio: tornando a FT8 dopo 60 s non si e' ancora visto niente).
- **JT65+JT9 insieme**: nuovo interruttore «JT65+JT9» tra i pulsanti di ricezione; in uno dei due modi JT decodifica nello stesso minuto anche l'altro (in `ALL.TXT` ognuno con il suo segno), la trasmissione resta nel modo scelto.
- **JT9**: sottrazione dei segnali decodificati (`subtract9`) e piu' passate; **JT65**: segue anche la deriva di frequenza (da -3 a +3 Hz sul messaggio, `afc65b`), piu' passate dopo la sottrazione, e un messaggio di testo libero si accetta solo se esce dalla decisione dura (JTDX lo mostra con un marcatore; qui si evitano i messaggi inventati vicino ai segnali forti).
- **Hound con salti di frequenza** (`m_houndTXfreqJumps` di JTDX, letto in `mainwindow.cpp`): con Split acceso, una frequenza che non sia una comune dell'FT8 e la ricezione tra 200 e 999 Hz, il primo R+rapporto va sulla frequenza della spedizione e i successivi 300 Hz piu' su (piu' giu' sopra i 600 Hz); a QSO perso si torna alla frequenza da cui si chiamava.
- Letti con REA e con i sorgenti di JTDX (`jt65_decode.f90`, `decode65a.f90`, `afc65b.f90`, `jt9_decode.f90`): il decoder JT65 di JTDX e' molto piu' ricco di quello di WSJT-X (ipotesi dinamiche, controllo dei nominativi, criteri sul ritardo); portato solo quanto sopra, il resto in `docs/2026-10-26-jt65-jt9.md`.

## 0.11.24 - 2026-10-26
- **JT65 e JT9 nel motore proprio**: nel selettore dei modi FT8 / FT4 / **JT65** / **JT9**. Il motore decodifica il minuto intero (JT65 con sottrazione dei segnali gia' decodificati, JT9 con decodificatore di Fano), trasmette (la risposta a un CQ parte nel minuto opposto, il tono piu' basso e' la frequenza Tx), chiude con RRR (RR73 non esiste in quei modi), scrive `ALL.TXT` con `#` (JT65) e `@` (JT9) e registra il QSO con il modo giusto. Il cambio di modo porta la radio sulle frequenze di JT65 e JT9 di JTDX (14,076 e 14,078 MHz su 20 m, ...), lette con REA da `jtdx.exe`; una tabella delle frequenze salvata prima le riceve senza perdere le proprie righe. Prove: la risposta trasmessa si rilegge con il ricevitore, tre minuti di audio nel motore vivo escono sul bus. Cose ancora indietro rispetto a JTDX (afc, avg65, dynhint, JT65+JT9 insieme, sottomodi) in `docs/2026-10-26-jt65-jt9.md`.
- **SWL come JTDX**: oltre alla finestra e alle soglie, la quarta lettura (`llrd`), la prima lettura con `llra` anche alla prima passata e nessun controllo sui rapporti dei toni dei tipi a priori 1 e 2 (letto in `ft8b.f90` dei sorgenti di JTDX).
- **«Filtro» e Hound**: il pulsante Filtro restringe davvero la ricerca a +-60 Hz attorno alla frequenza di ricezione (+-290 Hz con Hound). Spento di fabbrica come in JTDX: prima il pulsante risultava acceso ma non faceva niente, e quello stato ricordato non si ripristina.
- **Cambio frequenza e S-meter piu' pronti**: lo stato (frequenza, S-meter, trasmissione) si pubblica quattro volte al secondo invece di una, e la frequenza della radio si rilegge ogni quarto di secondo con il CAT (prima ogni 3 s); con OmniRig l'intervallo si adegua al costo della lettura.
- **«IN CODA» non copre piu' il nominativo**: la sigla porta il posto in coda («IN CODA 1») e sta nella colonna delle etichette (accanto al paese se la riga ha gia' un'etichetta), mai sopra il testo del messaggio.
- **«Canc DX» ferma subito la trasmissione**: oltre a liberare la stazione, ferma audio e PTT e scarta il messaggio gia' consegnato alla catena (prima la risposta alla stazione cancellata partiva lo stesso); con una coda si passa alla prossima stazione al primo periodo utile.
- Clippy: tipo con nome per una prova dell'uscita audio.

## 0.11.23 - 2026-10-26
- **Coda delle stazioni che chiamano**: tasto destro su una riga della finestra Attivita' di banda mette la stazione in coda; a QSO registrato (e a fine della trasmissione di chiusura RR73/73) Aurora risponde alla prima della coda, con l'ultimo messaggio che ha mandato. Senza un QSO in corso il tasto destro risponde subito (come il doppio clic); di nuovo sulla stessa la toglie. Fino a 8 stazioni, una stazione sparita da 10 minuti si salta, STOP TX, cambio di banda e cambio di modo svuotano la coda. La riga in coda ha la sigla «IN CODA» e una barra blu. Solo motore proprio (con JTDX decide JTDX). `POST /api/v1/commands/queue-reply`.
- **«Risp. a tutti»** (nuovo interruttore nel riquadro QSO, accanto a «1 QSO», e nelle impostazioni della Sequenza): ogni stazione che mi chiama direttamente va in coda da sola (non i CQ, non le chiusure RR73/73, non il DX in corso, non i doppioni; con «Risp B4» spento nemmeno i gia' collegati in sessione). Spento di fabbrica.
- **Chi mi ha risposto**: l'etichetta del suo nominativo e la linguetta a destra diventano viola con il tooltip «Ti ha risposto» (prima era verde solo se avevo trasmesso a lui da poco); la linguetta rossa resta solo per chi mi chiama per primo. La stazione che sto lavorando e che mi scrive e' sempre una risposta.
- **Hound e modo SWL nel motore proprio**: Hound non trasmette CQ e usa la logica di chiusura della spedizione; SWL allarga la finestra di ricerca del ritardo (+-86 passi invece di +-62, picco da -74 a 101, vicini a meno di 3 Hz), usa le soglie del sincronismo della soglia bassa, sottrae anche alla terza passata di un ciclo da 3 e fa sempre la lettura a tempo invertito (valori letti con REA in `jtdxjt9.exe`). La quarta sottopassata di SWL non e' portata.
- **Crate `jt` (JT65 e JT9), non ancora nel programma**: messaggi a 72 bit, Reed-Solomon, trasmissione e ricezione di JT65, trasmissione e ricezione di JT9 con decodificatore di Fano; i vettori della guida di WSJT-X coincidono simbolo per simbolo e la tabella delle metriche di JT9 e' uguale a quella di `jtdxjt9.exe` (REA). Stato e cose mancanti in `docs/2026-10-26-jt65-jt9.md`.

## 0.11.22 - 2026-10-26
- **Novita' DXCC e zone con banda sconosciuta**: con «Banda» e «Banda+Modo» accese e la banda non nota, il modo si controlla ancora (prima un paese nuovo solo in quel modo non si illuminava). Prove per zone CQ/ITU, locatore, nominativo e per «solo modo».
- **Diagnostica del programma Windows**: i panic finiscono nel log con il thread e il punto; ogni 10 minuti (ogni minuto se troppo alte) una riga con memoria privata, maniglie, oggetti USER e GDI del processo, per capire arresti e consumi.
- **Notifiche, «Colori e anteprima»** va a capo sugli schermi stretti (sotto 640 px) invece di uscire dalla cella.
- Immagini dell'installatore rigenerate con la versione giusta.

## 0.11.21 - 2026-10-25
- **«DXCC BANDA» (e zone CQ/ITU su banda) corretta**: un paese gia' collegato su quella banda, in qualunque modo (anche RTTY, SSB, PSK31) e con qualunque nominativo, non e' piu' «nuovo su banda»; prima, con le caselle «Banda» e «Banda+Modo» accese, una stazione slovacca in FT8 su 20 m si illuminava anche avendo gia' la Slovacchia su 20 m in altri modi. Locatore, prefisso e nominativo restano come in JTDX.
- **Scheda Audio**: le tendine di ingresso e uscita sono a tutta larghezza, il nome del dispositivo si legge intero.
- **Notifiche, «Colori e anteprima» compatta**: ogni voce occupa un solo rigo (nome, suono, prova, anteprima e colore).

## 0.11.20 - 2026-10-25
- **Barra in alto fissa**: i pulsanti accanto al logo restano fermi quando si scorre la pagina (un vecchio stile li rimetteva in flusso).

## 0.11.19 - 2026-10-25
- **Scheda Audio sul layout comune** a riquadri (Ingresso, Uscita, File audio, Applica).
- **Regione IARU funzionante** (Impostazioni -> Generali): la tabella delle frequenze e' ora quella iniziale di JTDX per FT8 e FT4 (letta con REA da `jtdx.exe`), con la regione e la riga predefinita di ogni frequenza; la regione scelta decide quali righe valgono (in regione 1 i 160 m vanno su 1,810 MHz, in regione 3 gli 80 m in FT4 su 3,568 MHz; con «Tutte» e regione 2 resta tutto com'era). Vale subito per cambio di banda, cambio di modo (come JTDX: resta sulla frequenza se e' gia' una riga del nuovo modo) e programmazione. Scheda Frequenze con regione e riga predefinita, righe nascoste attenuate e tasto «Frequenze iniziali di JTDX»; il valore iniziale si legge dal `JTDX.ini`.
- **«Generazione dei messaggi per nominativi composti di tipo 2»** (Impostazioni -> Generali): ora si sceglie, si salva e si legge dal `JTDX.ini`. Come in JTDX (verificato nell'eseguibile) riguarda solo JT65, JT9 e T10: in FT8 e FT4 non cambia nessun messaggio, e la pagina lo dice sotto la voce.

## 0.11.18 - 2026-10-25
- **«CQ e novità» personalizzabile** (Impostazioni -> Filtri): scegli i tipi di messaggio elencati, le sole stazioni da elencare (JA*, EA8AA...), quelle da non elencare mai e il numero massimo di righe; le scelte restano al riavvio.
- **Suoni una volta sola per indicativo**: una stazione fa suonare una notifica la prima volta che entra nelle regole, poi mai piu' (anche se ritrasmette o cambia funzione); una stazione nuova suona di nuovo.
- **Pannello Comandi compatto**: tutto in un unico flusso, controlli piu' piccoli e spazi minimi.
- **Attivita' di banda**: la linea verticale prima del blocco etichette e' sottile e uguale anche nelle righe senza etichetta.

## 0.11.17 - 2026-10-25
- **Suoni una volta sola per stazione**: una stazione che ha gia' fatto suonare una funzione non la fa suonare di nuovo per 10 minuti, anche se ritrasmette.
- **QSO gia' lavorati** barrati con una linea unica da bordo a bordo, anche negli spazi vuoti; linee verticali delle colonne a colore pieno, uguali su tutte le righe.
- **Pulsante «Installa ora»** anche nella scheda Impostazioni -> Aggiornamenti.
- **All'avvio si apre sempre la pagina Stazione.**
- Al posto del titolo «Stazione DX» c'e' l'etichetta gialla con nominativo e paese; Tx e Rx sulla riga del nominativo.

## 0.11.16 - 2026-10-25
- **PSK Reporter e DXSummit accesi di base** (scelta del proprietario; si spengono dalla scheda Segnalazioni). **Pulsante «Spot DX»** nel riquadro DX della pagina Stazione: invia lo spot a DXSummit come JTDX (frequenza dial + Rx, commento con modo e testo), senza bloccare il motore e senza ripetere lo stesso DX per 5 minuti.
- **QSO messo a log**: tutta la riga lampeggia tre volte, lentamente, in cima al pannello Log, che si aggiorna subito.

## 0.11.15 - 2026-10-25
- **Scheda Segnalazioni funzionante** (come JTDX): commento ADIF con rapporto dB e distanza, conversione RTTY, cancellazione dei dati dopo il log, server TCP per il record ADIF, spotting su PSK Reporter (**spento di base**: lo accendi tu dalla scheda; nessun invio verso internet parte da solo), invio a eQSL con la password protetta da Windows, ALL.TXT con e senza debug, stato dell'ultimo invio; scheda riordinata a riquadri.
- **Suoni per ogni funzione**: banco di 53 suoni sintetizzati (nessun file), menu «Suono» con ▶ in ogni riga della tabella dei colori nelle Notifiche e riquadro «Suoni» in fondo; senza suono assegnato non suona.
- **Livello Pwr**: memoria per banda spenta di fabbrica (come JTDX) e cursore sempre allineato al motore: corregge la potenza al minimo dopo un cambio di banda.
- **Messaggi a me**: se la stazione ha risposto alla tua chiamata compare la linguetta **verde** («ti ha risposto»); se ti chiama per prima resta quella rossa. **Gia' lavorati**: barrata tutta la riga.
- Barra in alto fissa in primo piano, guida visibile nelle impostazioni DXCC, caselle DXCC sempre con contorno.

## 0.11.14 - 2026-10-25
- **Bug del livello Pwr al cambio di banda corretto**: il cursore mostrava il massimo ma si trasmetteva al minimo, perche' la memoria del livello per banda ripristinava un livello vecchio senza che il cursore lo seguisse. Ora la memoria per banda e' spenta di fabbrica come in JTDX (un solo livello su tutte le bande; `AURORA_PWR_BAND_MEMORY=1` la accende) e il cursore segue sempre il valore del motore (ogni 2 secondi).
- Linea verticale che chiude la colonna paese e distanza presente anche nelle righe con l'etichetta colorata; frequenza Tx e Rx sulla riga del nominativo DX, a destra.

## 0.11.13 - 2026-10-25
- **Impostazioni riordinate**: pagine larghe a riquadri con cornice e celle a colonne, barra delle schede in due righe per tipo, scheda Notifiche come tabella vera con intestazioni, **schede Radio e Radio CAT unite**; anteprime dei colori con il nominativo dell'utente.
- **Scheda Audio funzionante**: ingresso e uscita a menu, canali, cambio a caldo con ripristino del dispositivo precedente se non si apre, salvataggio dei file audio nel formato di JTDX (Nessuno/Decodificati/Tutti).
- **Scheda DXCC**: stazione su una riga con bandiera e paese, campo di prova piccolo, numero dei QSO del log a destra del titolo, linee verticali piu' marcate nelle liste.

## 0.11.12 - 2026-10-25
- **Scheda Impostazioni «DXCC»**: scegli le bande e i modi da mostrare, i colori delle caselle per modo (lavorato/confermato, non confermato, mai lavorato) e dei sei NEW ONE/BAND/MODE e GRID, e le opzioni di aspetto; anteprima viva e ripristino dei colori di fabbrica.
- **Scheda DXCC sempre presente**, anche senza stazione (etichette e caselle grigie), senza spazio vuoto sotto.
- **Stazione DX** (etichetta gialla con nominativo, locatore, bandiera e paese) nel riquadro Trasmissione al posto del nominativo; titolo tolto e pastiglia tolta dalla barra.
- **Distanza a sinistra** nella colonna Attivita' di banda.
- **Esc piu' reattivo**: i comandi arrivati mentre si decodifica (STOP TX, Esc, Abilita Tx) valgono prima di decidere cosa trasmettere, e la pagina mostra subito la trasmissione ferma.

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
