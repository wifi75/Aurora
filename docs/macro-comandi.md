# I comandi delle macro in Aurora

Tabella di tutti i comandi delle macro dei modi testuali: famiglia, che cosa fanno in Aurora e stato.
Stato: **fatto** (fa la sua funzione), **al più vicino** (manca il pezzo di Aurora che servirebbe: si fa la cosa più utile, il motivo è scritto), **escluso** (per sicurezza).
L'elenco per l'interfaccia (editor e Guida delle macro, sette lingue) è `app/src/lib/macroCommands.ts`; l'interprete è `app/src/lib/macroLang.ts` con `macroExtra.ts`, l'esecuzione nella pagina `macroRuntime.ts`.

| Famiglia | Comando | Che cosa fa in Aurora | Stato |
|---|---|---|---|
| Di base (text) | `<MYCALL>` | il tuo nominativo | fatto |
| Di base (text) | `<MYMAINCALL>` | il tuo nominativo senza /P, /M... | fatto |
| Di base (text) | `<MYNAME>` | il tuo nome | fatto |
| Di base (text) | `<MYQTH>` | il tuo QTH | fatto |
| Di base (text) | `<MYWWLOC>` | il tuo locatore | fatto |
| Di base (text) | `<CALL>` | nominativo del corrispondente | fatto |
| Di base (text) | `<NAME>` | nome del corrispondente | fatto |
| Di base (text) | `<QTH>` | QTH del corrispondente | fatto |
| Di base (text) | `<WWLOC>` | locatore del corrispondente | fatto |
| Di base (text) | `<RSTS>` | rapporto da mandare | fatto |
| Di base (text) | `<RSTR>` | rapporto ricevuto | fatto |
| Di base (text) | `<NOTES>` | campo note | fatto |
| Di base (text) | `<NRS>` | scambio da mandare (contest; vuoto = numero progressivo) | fatto |
| Di base (text) | `<NRR>` | scambio ricevuto (contest) | fatto |
| Di base (text) | `<DATE>` | data di oggi (UTC) | fatto |
| Di base (text) | `<TIME>` | ora UTC | fatto |
| Di base (text) | `<LOCALTIME>` | ora locale | fatto |
| Di base (text) | `<STARTTIME>` | ora di inizio del QSO | fatto |
| Di base (text) | `<CTIME>` | ora UTC in formato HHMM | fatto |
| Di base (text) | `<GA>` | GM, GA o GE secondo l'ora | fatto |
| Di base (text) | `<GAL>` | come <GA>, frase lunga | fatto |
| Di base (text) | `<KHZ>` | frequenza della radio in kHz (<KHZ:n> con n decimali) | fatto |
| Di base (text) | `<MHZ>` | frequenza della radio in MHz (<MHZ:n>) | fatto |
| Di base (text) | `<BAND>` | banda in uso | fatto |
| Di base (text) | `<MODE>` | modo in uso | fatto |
| Di base (text) | `<QSONR>` | numero progressivo del contest (001, avanza a ogni QSO salvato) | fatto |
| Di base (text) | `<CR>` | ritorno a capo (CR) | fatto |
| Di base (text) | `<LF>` | nuova riga (LF) | fatto |
| Di base (text) | `<CRLF>` | a capo completo (CR + LF) | fatto |
| Di base (text) | `<TAB>` | tabulazione | fatto |
| Di base (text) | `<ANYQSOBEFOREDATE>` | data dell'ultimo QSO con questo nominativo nel log (vuoto se non c'è) | fatto |
| Di base (text) | `<ANYQSOBEFOREBAND>` | banda dell'ultimo QSO con questo nominativo | fatto |
| Di base (text) | `<ANYQSOBEFOREMODE>` | modo dell'ultimo QSO con questo nominativo | fatto |
| Di base (text) | `<PREVNRS>` | contest: lo scambio che hai mandato nell'ultimo QSO del log (per ripeterlo a chi lo chiede di nuovo) | fatto |
| Di base (text) | `<PREVCALL>` | contest: il nominativo dell'ultimo QSO del log | fatto |
| Di base (text) | `<PREVRSTS>` | contest: il rapporto mandato nell'ultimo QSO del log | fatto |
| Di base (text) | `<QSOBEFORETIME>` | contest: ora UTC del QSO già fatto con questo nominativo (vuoto se non c'è) | fatto |
| Di base (text) | `<QSOBEFOREDATE>` | contest: data del QSO già fatto con questo nominativo | fatto |
| Di base (text) | `<TEXTFILE:nome>` | il testo di un file della cartella dei testi delle macro | fatto |
| Di base (text) | `<[> <]>` | i segni < e > nel testo | fatto |
| Di base (text) | `<IMD>` | IMD del segnale PSK ricevuto: Aurora la mostra nella barra ma il comando non scrive niente | fatto |
| Di base (text) | `<VER>` | nome del programma | fatto |
| Di base (program) | `<EXEC:file>` | avvierebbe un programma esterno: Aurora non avvia programmi e toglie il comando dalle macro; la copia di sicurezza che facevano i vecchi .bat la fa Aurora da sola all'avvio | escluso |
| Di base (program) | `<TX>` | passa in trasmissione e manda il testo della macro (da solo: quello della casella) | fatto |
| Di base (program) | `<TXE>` | trasmette quando cade il segnale sulla frequenza di ricezione (al massimo dopo 30 s; Esc annulla) | fatto |
| Di base (program) | `<TXTOGGLE>` | alterna fra trasmissione e ricezione (in trasmissione: torna in ricezione a fine testo) | fatto |
| Di base (program) | `<RX>` | torna in ricezione quando la trasmissione arriva a questo punto; il testo dopo resta nella finestra senza partire | fatto |
| Di base (program) | `<RXANDCLEAR>` | come <RX> e, tornati in ricezione, pulisce la finestra di trasmissione | fatto |
| Di base (program) | `<RXC>` | uguale a <RXANDCLEAR> | fatto |
| Di base (program) | `<RXE>` | torna in ricezione a fine testo | fatto |
| Di base (program) | `<CLEARRXWINDOW>` | pulisce la finestra di ricezione | fatto |
| Di base (program) | `<CLEARTXWINDOW>` | pulisce la finestra di trasmissione (casella e testo già partito) fino a questo punto: il testo dopo resta | fatto |
| Di base (program) | `<CLEARWINDOW>` | pulisce le finestre di ricezione e di trasmissione | fatto |
| Di base (program) | `<CLEARQSO>` | azzera i dati del QSO | fatto |
| Di base (program) | `<GRABCALL>` | prende l'ultimo nominativo ricevuto nella finestra RX (non il tuo) e lo mette nel QSO: il <CALL> che segue lo usa subito | fatto |
| Di base (program) | `<FIXSTARTTIME>` | fissa adesso come ora di inizio del QSO | fatto |
| Di base (program) | `<SAVEQSO>` | salva il QSO a log (va prima dei comandi di pulizia) | fatto |
| Di base (program) | `<QTCSEND>` | apre la finestra per mandare i QTC (contest WAE) alla stazione del QSO | fatto |
| Di base (program) | `<QTCREAD>` | apre la finestra per ricevere una serie di QTC (contest WAE) dalla stazione del QSO | fatto |
| Di base (autocq) | `<AUTOCQ>` | avvia o ferma la chiamata CQ automatica; seguito da un testo, quel testo diventa la chiamata | fatto |
| Di base (program) | `<TUNE>` | portante per accordare sulla frequenza di trasmissione (<TUNE:s> per s secondi, di base 10) | fatto |
| Di base (program) | `<BEEP:ON>` | suona un avviso | fatto |
| Di base (program) | `<CURSOR:TX>` | porta il cursore nella casella di trasmissione | fatto |
| Di base (macros) | `<MACROS:file>` | Aurora tiene le macro nelle sue impostazioni: per caricarne un file usa Importa macro; il comando lo ricorda | al più vicino: Aurora tiene le macro nelle impostazioni: avviso con la voce del menu per importarle |
| Di base (frequency) | `<AUDIOFQ:Hz>` | imposta la frequenza audio (con + o - è relativa) | fatto |
| Di base (frequency) | `<SEEKLEFT>` | cerca un segnale verso sinistra | fatto |
| Di base (frequency) | `<SEEKRIGHT>` | cerca un segnale verso destra | fatto |
| Di base (frequency) | `<BOOKMARK>` | segna ora e frequenza nella finestra RX (come <MARK>) | fatto |
| Di base (mode) | `<MODE:PSK31>` | cambia modo (PSK31, PSK63, RTTY) | fatto |
| Di base (rtty) | `<BAUDRATE:n>` | velocità: 31.25 o 62.5 per PSK, 45.45, 50, 75... per RTTY | fatto |
| Di base (rtty) | `<SHIFT:Hz>` | spostamento RTTY in Hz | fatto |
| Di base (autocq) | `<ACQ>` | come <AUTOCQ> | fatto |
| Di base (autocq) | `<ALARM:testo>` | mostra un avviso con il testo | fatto |
| Di base (autocq) | `<ACQDELAY:s>` | pausa fra due CQ automatici (secondi) | fatto |
| Di base (autocq) | `<ASAUTOCQ>` | accende l'Auto CQ; il testo che segue diventa la chiamata | fatto |
| Di base (autocq) | `<AUTOCQRESUME>` | riprende l'Auto CQ fermato | fatto |
| Di base (macros) | `<EDITMACRO:nome>` | apre la finestra di modifica di una macro (F5, Ctrl-F5...) | fatto |
| Di base (macros) | `<MODEMACROS:file>` | usa le macro di un modo (PSK31, PSK63, RTTY, CW) senza cambiare modo; vuoto: quelle del modo in uso | fatto |
| Di base (macros) | `<HIDETEXT>` | il testo che segue non si trasmette fino a <SHOWTEXT> (erano comandi per un TNC esterno) | al più vicino: il testo nascosto (comandi per un TNC) non si trasmette |
| Di base (macros) | `<SHOWTEXT>` | il testo torna a essere trasmesso dopo <HIDETEXT> | al più vicino: vedi HIDETEXT |
| Di base (frequency) | `<FQ:kHz>` | porta la radio in modo che il segnale cada su questa frequenza in kHz | fatto |
| Di base (frequency) | `<SPLIT:1>` | split acceso (1) o spento (0): la trasmissione resta dov'è mentre sposti l'ascolto | fatto |
| Di base (frequency) | `<JUMPTOTXFQ>` | porta l'ascolto sulla frequenza di trasmissione | fatto |
| Di base (frequency) | `<ALIGN:Hz>` | allinea la frequenza di ricezione a un multiplo di questi Hz | fatto |
| Di base (frequency) | `<CATCMD:comando>` | manda un comando CAT alla radio | fatto |
| Di base (mode) | `<AFCON>` | accende l'AFC (<AFCOFF> lo spegne, <AFC> lo alterna) | fatto |
| Di base (mode) | `<SQUELCHON>` | accende lo squelch (<SQUELCHOFF>, <SQUELCH>) | fatto |
| Di base (mode) | `<SNAPON>` | accende Snap (<SNAPOFF>, <SNAP>) | fatto |
| Di base (mode) | `<LOCKTX>` | blocca la trasmissione dov'è (<UNLOCKTX> la libera) | fatto |
| Di base (program) | `<IGNOREBEFORE:aaaa-mm-gg hh:mm>` | i segni del log (BIGScan, fuori dal contest) non contano i QSO prima di questa data e ora UTC; <IGNOREBEFORE:ERASE> toglie il limite | fatto |
| Di base (program) | `<PERIOD:n>` | i segni del log (BIGScan, fuori dal contest) contano solo i QSO degli ultimi n minuti; 0 = spento | fatto |
| Di base (frequency) | `<STARTSCAN>` | avvia la scansione della banda (<STOPSCAN> la ferma) | fatto |
| Di base (mode) | `<MODESETTINGS>` | apre le impostazioni del modo | fatto |
| Di base (mode) | `<THRESHOLD>` | apre la soglia dello squelch | fatto |
| Di base (mode) | `<MEMW:n>` | salva le impostazioni nella memoria n (<MEMR:n> la rilegge) | fatto |
| Di base (rotor) | `<ROTOR:gradi>` | gira il rotore su quella direzione (0-359) | fatto |
| Di base (rotor) | `<ROTOR:SP>` | gira il rotore verso la stazione del QSO per la via corta | fatto |
| Di base (rotor) | `<ROTOR:LP>` | come <ROTOR:SP> ma per la via lunga | fatto |
| Di base (rtty) | `<LETTERS>` | RTTY: il passaggio alle lettere lo manda già il motore dove serve, il comando non aggiunge niente | al più vicino: il motore RTTY manda gia' LTRS/FIGS dove servono: non aggiunge niente |
| Di base (rtty) | `<NUMBERS>` | RTTY: il passaggio alle cifre lo manda già il motore dove serve, il comando non aggiunge niente | al più vicino: come LETTERS |
| Di base (sstv) | `<LOADPICTURE>` | apre la pagina SSTV per scegliere l'immagine da trasmettere | al più vicino: apre la pagina SSTV |
| Di base (sstv) | `<FROMCLIP>` | trasmette in SSTV l'immagine che c'è negli appunti | fatto |
| Di base (sstv) | `<FONT:nome>` | i testi sull'immagine si scrivono nella pagina SSTV: il comando la apre | al più vicino: apre la pagina SSTV |
| Di base (sstv) | `<LIMITSCANLINES:n>` | le righe dell'immagine si scelgono nella pagina SSTV: il comando la apre | al più vicino: apre la pagina SSTV |
| Di base (cw) | `<AR> <BT> <KN> <SK>` | segni di procedura: in CW un segno solo, negli altri modi le lettere | fatto |
| Di base (cw) | `<WPM:n>` | velocità in parole al minuto (con + o - è relativa) | fatto |
| Di base (cw) | `<LANGUAGE:xx>` | lingua delle frasi INT (come <FORCELANGUAGE:xx>) | fatto |
| Di base (cw) | `<WK:comando>` | manda il testo in CW con il WinKeyer (come <WKTEXT>) | fatto |
| Di base (hell) | `<SET FILTER:n>` | Hell non c'è in Aurora: parametro saltato | al più vicino: Hell non c'e' in Aurora: saltato |
| Di base (hell) | `<SET FONT:nome>` | Hell non c'è in Aurora: parametro saltato | al più vicino: Hell non c'e' in Aurora: saltato |
| Di base (hell) | `<SET SUBMODE:n>` | sottomodo: BPSK31, PSK63, RTTY, CW cambiano modo; quelli che Aurora non ha (Hell, MFSK) si saltano | fatto |
| Corrispondente e QSO precedente | `<ACALL>` | nominativo del corrispondente senza /P, /M e prefissi | al più vicino: il significato originale non e' documentato: in Aurora e' il nominativo senza prefissi e suffissi |
| Corrispondente e QSO precedente | `<CALLPOST:testo>` | il nominativo seguito dal testo, solo se il nominativo c'è | fatto |
| Corrispondente e QSO precedente | `<POST:sì,no>` | il primo testo se c'è un nominativo, il secondo se manca | fatto |
| Corrispondente e QSO precedente | `<OM>` | OM o YL secondo il nome del corrispondente | al più vicino: OM/YL ricavato dal nome con una regola (nomi in -A salvo i maschili noti) |
| Corrispondente e QSO precedente | `<DIST>` | distanza in km dal tuo locatore a quello del corrispondente (? se manca) | fatto |
| Corrispondente e QSO precedente | `<MILES>` | come <DIST>, in miglia | fatto |
| Corrispondente e QSO precedente | `<AZ>` | direzione in gradi verso il corrispondente (? se manca il locatore) | fatto |
| Corrispondente e QSO precedente | `<PREVNRR>` | lo scambio ricevuto nell'ultimo QSO del log | fatto |
| Corrispondente e QSO precedente | `<PREVCCALL>` | il nominativo dell'ultimo QSO senza /P e prefissi | fatto |
| Corrispondente e QSO precedente | `<PREVNAME>` | il nome dell'ultimo QSO del log | fatto |
| Corrispondente e QSO precedente | `<PREVQTH>` | il QTH dell'ultimo QSO del log | fatto |
| Corrispondente e QSO precedente | `<PREVRSTR>` | il rapporto ricevuto nell'ultimo QSO del log | fatto |
| Corrispondente e QSO precedente | `<PREVCTIME>` | ora UTC (HHMM) dell'ultimo QSO del log | fatto |
| Corrispondente e QSO precedente | `<PREVSUF2>` | le ultime due lettere del nominativo dell'ultimo QSO | fatto |
| Corrispondente e QSO precedente | `<LASTCALL>` | rimette nel QSO il nominativo dell'ultimo QSO del log | fatto |
| Corrispondente e QSO precedente | `<NRRPOST:testo>` | lo scambio ricevuto con i numeri abbreviati della CW tradotti (T e O = 0, A = 1, N = 9), poi il testo | fatto |
| Corrispondente e QSO precedente | `<QSOBEFORE>` | data e ora del QSO già fatto con questo nominativo sulla stessa banda e nello stesso modo (vuoto se non c'è) | fatto |
| Corrispondente e QSO precedente | `<QSOBEFORETIME1>` `<TIME1>` | ora UTC (HHMM) del QSO già fatto con questo nominativo | fatto |
| Corrispondente e QSO precedente | `<QSOBEFOREBAND>` | banda del QSO già fatto con questo nominativo | fatto |
| Corrispondente e QSO precedente | `<QSOBEFOREMODE>` | modo del QSO già fatto con questo nominativo | fatto |
| Corrispondente e QSO precedente | `<ONQSOBEFORE:macro>` | se il nominativo è già stato lavorato esegue la macro indicata | fatto |
| Testo selezionato | `<SELCALL>` | la parola selezionata nella finestra RX diventa il nominativo del QSO | fatto |
| Testo selezionato | `<SELNAME>` | il testo selezionato nella finestra RX diventa il nome | fatto |
| Testo selezionato | `<SELQTH>` | il testo selezionato nella finestra RX diventa il QTH | fatto |
| Testo selezionato | `<SELRSTS>` | la parola selezionata diventa il rapporto da mandare | fatto |
| Testo selezionato | `<SELRSTR>` | la parola selezionata diventa il rapporto ricevuto | fatto |
| Testo selezionato | `<SELNRS>` | la parola selezionata diventa lo scambio da mandare | fatto |
| Testo selezionato | `<SELNRR>` | la parola selezionata diventa lo scambio ricevuto | fatto |
| Testo selezionato | `<SELRXTEXT>` | scrive il testo selezionato nella finestra RX | fatto |
| Testo selezionato | `<SAVESELTEXT>` | copia negli appunti il testo selezionato nella finestra RX | al più vicino: il testo va negli appunti (non in un file) |
| Frasi automatiche | `<INTDE>` | «de» nella lingua del corrispondente (scelta dal prefisso) | fatto |
| Frasi automatiche | `<INTQSL>` | «QSL, grazie» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTAGNNR>` | «ripeti il numero» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTAGNCALL>` | «ripeti il nominativo» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTQRZ>` | «chi chiama?» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTGA>` | buongiorno, buon pomeriggio o buonasera nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTQB4>` | «ci siamo già collegati» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTQRL>` | «la frequenza è libera?» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTCQ>` | «chiamata generale» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<INTE>` | «e» nella lingua del corrispondente | fatto |
| Frasi automatiche | `<GM>` `<GE>` `<GN>` | saluto secondo l'ora locale (GM, GA, GE o GN); se hai le macro gm, ga, ge o gn usa quelle | fatto |
| Frasi automatiche | `<LANG_xx>` | le frasi INT di questa macro nella lingua xx (IT, EN, DE, FR, ES, PT, NL) | fatto |
| Frasi automatiche | `<FORCELANGUAGE:xx>` | le frasi INT restano nella lingua xx finché non la cambi (vuoto: di nuovo dal nominativo) | fatto |
| Ora e data | `<TXTIME>` | ora UTC dell'inizio dell'ultima trasmissione | fatto |
| Ora e data | `<LOCALDATE>` | data locale | fatto |
| Ora e data | `<TIME:formato>` `<DATE:formato>` `<LOCALTIME:formato>` `<CTIME:formato>` | ora UTC con un formato: %H %M %S ore minuti secondi, %d %m %Y %y giorno mese anno, %j giorno dell'anno, %b mese (vale anche per DATE, LOCALTIME, CTIME) | fatto |
| Ora e data | `<STARTTIME:hhmm>` | fissa l'ora di inizio del QSO (con % scrive l'ora di inizio con quel formato) | fatto |
| Ora e data | `<TIMER:s,macro>` | dopo s secondi esegue la macro (senza macro suona un avviso); <TIMER:0> lo ferma | fatto |
| QSO e log | `<CLEARNRR>` | svuota lo scambio ricevuto | fatto |
| QSO e log | `<INCNRR>` | lo scambio ricevuto più uno (stessi zeri davanti) | fatto |
| QSO e log | `<CORRECTCALLTX>` | ritrasmette il nominativo, dopo che l'hai corretto | al più vicino: ritrasmette il nominativo del campo |
| QSO e log | `<SAVE>` `<SAVESH>` | salva il QSO nel log (come <SAVEQSO>) | fatto |
| QSO e log | `<SAVESHAUTO>` | salva il QSO e svuota i campi | fatto |
| QSO e log | `<READLOG>` `<REFRESH>` | rilegge il log e i suoi segni | fatto |
| QSO e log | `<WKD>` | dice a schermo se e quando hai già lavorato il nominativo | fatto |
| QSO e log | `<WKDIGNOREBEFORE:data>` | come <IGNOREBEFORE:data> | fatto |
| QSO e log | `<ERASE>` | toglie il limite di <IGNOREBEFORE>: i segni del log contano di nuovo tutti i QSO | fatto |
| QSO e log | `<PRINTQSL>` `<QSLPRINT>` | stampa l'etichetta QSL del QSO in corso | al più vicino: stampa un'etichetta semplice dal programma di navigazione |
| QSO e log | `<SENDEQSL>` `<SENDEQSLNP>` | conferme in rete: Aurora manda ogni QSO salvato ai servizi scelti nelle Impostazioni (eQSL, LoTW, Club Log...), il comando lo ricorda | al più vicino: Aurora manda gia' ogni QSO salvato ai servizi scelti: il comando lo ricorda |
| QSO e log | `<SENDSPOT:commento>` | manda al DX cluster uno spot del corrispondente sulla frequenza del segnale | fatto |
| QSO e log | `<QSOCMDDDE:comando>` | Aurora non usa il DDE: i QSO salvati arrivano già ai programmi di log collegati | al più vicino: Aurora non ha il DDE: avviso (i QSO salvati arrivano gia' ai log collegati) |
| Trasmissione, modo e demodulatore | `<LOCKMODE>` `<UNLOCKMODE>` | blocca il modo: <MODE:...> non lo cambia più (<UNLOCKMODE> lo libera) | fatto |
| Trasmissione, modo e demodulatore | `<LOCKRX>` `<UNLOCKRX>` `<LOCKRXTOGGLE>` | blocca la frequenza di ricezione: clic e macro non la spostano (<UNLOCKRX>, <LOCKRXTOGGLE>) | fatto |
| Trasmissione, modo e demodulatore | `<MODEADD:modo>` `<MODETNC:modo>` | come <MODE:modo> | fatto |
| Trasmissione, modo e demodulatore | `<WORDMODE:ON>` | mentre scrivi la trasmissione parte a parole intere (OFF: lettera per lettera) | fatto |
| Trasmissione, modo e demodulatore | `<CAPITALSONLY:ON>` | solo maiuscole nella finestra di trasmissione (OFF le toglie) | fatto |
| Trasmissione, modo e demodulatore | `<CWID>` | in RTTY e PSK aggiunge DE e il tuo nominativo in fondo (<CWID:testo> un altro testo) | al più vicino: Aurora non mescola la CW negli altri modi: «DE nominativo» in coda al testo |
| Trasmissione, modo e demodulatore | `<REVERSEPADDLE>` | scambia punto e linea del paddle del WinKeyer | fatto |
| Trasmissione, modo e demodulatore | `<SNAPNOW>` | porta subito la ricezione sul segnale più vicino | fatto |
| Trasmissione, modo e demodulatore | `<INV>` `<INVON>` `<INVOFF>` | RTTY: inverte mark e space (<INVON>, <INVOFF>) | fatto |
| Trasmissione, modo e demodulatore | `<FILTER:PASS>` | filtro di ricezione: PASS passabanda stretto, NONE spento (anche i notch); NOTCH e NOISE accendono il passabanda | al più vicino: NOTCH e NOISE automatici non ci sono: accendono il passabanda |
| Trasmissione, modo e demodulatore | `<BROADCASTMODE:ON>` | ON: si resta in trasmissione anche se la macro ha <RX>, finché non premi RX; OFF torna normale | al più vicino: interpretato come «resta in trasmissione anche con <RX>» |
| Trasmissione, modo e demodulatore | `<TOGGLECHANNEL>` | scambia la ricezione principale con la prima finestra RX in più | fatto |
| Trasmissione, modo e demodulatore | `<ESCAPE>` | ferma subito la trasmissione | fatto |
| Trasmissione, modo e demodulatore | `<PTT:ON>` | apre la trasmissione (OFF la chiude) | fatto |
| Trasmissione, modo e demodulatore | `<SET BW:Hz>` `<SET TONES:n>` `<SET SUBMODE:n>` | un parametro del modo: BW (filtro della CW o passabanda), SHIFT, BAUD, WPM, SQUELCH, SUBMODE (BPSK31, PSK63, RTTY, CW); TONES e i sottomodi che Aurora non ha si saltano | al più vicino: TONES e i sottomodi MFSK/Hell non esistono in Aurora: saltati (SET SUBMODE:MFSK32 da' un avviso una volta) |
| Trasmissione, modo e demodulatore | `<GET SN>` `<GET OFFSET>` `<GET RATE>` | scrive un valore del segnale: SN (S/N), OFFSET (frequenza audio), RATE (baud o WPM), MODE, IMD | fatto |
| Trasmissione, modo e demodulatore | `<SPZOOM:n>` | cascata: 1 larghezza normale, 2 o più cascata larga | al più vicino: la cascata ha solo larghezza normale o larga |
| Trasmissione, modo e demodulatore | `<SPSHIFT:Hz>` | porta la ricezione su questa frequenza audio della cascata | fatto |
| Trasmissione, modo e demodulatore | `<AMPLITUDE:dB>` | livello di uscita: attenuazione in dB (0 = massimo) | fatto |
| Trasmissione, modo e demodulatore | `<ZFQ>` | sposta la radio perché il segnale in ricezione cada a 1500 Hz | fatto |
| Trasmissione, modo e demodulatore | `<FQB:kHz>` | frequenza di trasmissione (VFO B, split) in kHz | fatto |
| Trasmissione, modo e demodulatore | `<TEST>` | scrive TEST se c'è un contest attivo, se no niente | fatto |
| CAT e radio | `<CATCMDHEX:byte>` | manda alla radio un comando CAT scritto in esadecimale (radio Icom e altre a byte) | fatto |
| CAT e radio | `<CATCMDX:comando>` `<CATCMDDDE:comando>` | come <CATCMD>: accetta anche i byte scritti \0xNN | fatto |
| CAT e radio | `<CATWAIT:ms>` | aspetta questi millisecondi prima del comando CAT seguente | fatto |
| CAT e radio | `<CATTRANSVERTER:MHz>` | spostamento del transverter: lo si regola nelle Impostazioni della radio, il comando apre l'avviso | al più vicino: si regola nelle impostazioni della radio: il comando le apre |
| CAT e radio | `<CATVFOSYNCONCE>` `<CATVFOSYNCSHIFT:Hz>` | sposta una volta la radio perché il segnale cada sul centro (1500 Hz o quello di <CATVFOSYNCSHIFT:Hz>) | fatto |
| CAT e radio | `<CATENABLEVFOSYNC>` `<CATDISABLEVFOSYNC>` | sincronia della radio: a ogni macro il segnale torna al centro (<CATDISABLEVFOSYNC> la spegne) | al più vicino: la sincronia si rifa' a ogni macro, non a ogni clic |
| CAT e radio | `<TOGGLECATPORT>` `<TOGGLEPTTPORT>` | collega o scollega la radio (CAT); anche <TOGGLEPTTPORT>, perché in Aurora il PTT passa dal CAT | al più vicino: TOGGLEPTTPORT fa lo stesso: in Aurora il PTT passa dal CAT |
| CAT e radio | `<TOGGLESOUNDDEV>` | riapre l'ascolto della scheda audio (spegne e riaccende la ricezione) | fatto |
| CAT e radio | `<INPUTVOLUME:%>` | volume dell'ingresso audio in percento (senza numero apre le impostazioni audio) | fatto |
| CAT e radio | `<OUTPUTVOLUME:dB>` | attenuazione dell'uscita audio in dB (senza numero apre le impostazioni audio) | fatto |
| WinKeyer | `<WKTEXT:testo>` `<WK:testo>` | manda il testo in CW con il WinKeyer | fatto |
| WinKeyer | `<WKHEX:byte>` | manda al WinKeyer byte di comando in esadecimale | fatto |
| WinKeyer | `<WKPOT>` `<WKNOPOT>` | la velocità la decide il potenziometro del WinKeyer (<WKNOPOT>: la decide Aurora) | fatto |
| Rotore | `<ROTORCMD:comando>` | comando del rotore alla GS-232: M o W con i gradi gira, S o A ferma; si fa con il rotore di Aurora | fatto |
| Rotore | `<ROTORCMDHEX:byte>` | come <ROTORCMD> con il comando scritto in esadecimale | fatto |
| Finestre e testo | `<NEWRXWINDOW>` `<NEWRXWINDOW1>` `<NEWRXWINDOW2>` | apre una finestra RX in più sulla frequenza di ricezione | fatto |
| Finestre e testo | `<CLOSERXWINDOWS>` | chiude tutte le finestre RX in più | fatto |
| Finestre e testo | `<CLEAR>` | pulisce la finestra RX e quella di trasmissione | fatto |
| Finestre e testo | `<SHOW:nome>` `<HIDE:nome>` `<TOGGLE:nome>` | mostra (HIDE nasconde, TOGGLE alterna) una parte di Aurora: CONTROLBAR, LOGBAR, STATUSBAR, TUNINGBAR (cascata), CONTESTSTATS, QSOSTATISTIC, ROTOR, DXCLUSTER, WORLDMAP (mappa), BEACONS, CALLBOOK, KIWISDR (ricevitore in rete: avviso) | al più vicino: KIWISDR, CALLBOOK, BEACONS, TNC, TELNET, RIGEXPERTS non ci sono in Aurora: avviso una volta |
| Finestre e testo | `<SCROLLLOCK:ON>` | ferma lo scorrimento della finestra RX (OFF lo riprende) | fatto |
| Finestre e testo | `<CTRL-C>` `<CTRL-T>` | tasti Ctrl per un TNC esterno: in Aurora non scrivono niente | al più vicino: erano tasti per un TNC esterno: non scrivono niente |
| Finestre e testo | `<FONTSIZE:n>` | grandezza del testo della finestra RX in pixel (vuoto: quella di base) | fatto |
| Finestre e testo | `<FACE:nome>` | carattere della finestra RX | fatto |
| Finestre e testo | `<COLOR:RRGGBB>` | colore del testo della finestra RX | fatto |
| Finestre e testo | `<BOLD:ON>` | testo della finestra RX in grassetto (OFF normale) | fatto |
| Finestre e testo | `<ITALIC:ON>` | testo della finestra RX in corsivo (OFF normale) | fatto |
| Finestre e testo | `<MARK>` | segna ora e frequenza nella finestra RX | fatto |
| Finestre e testo | `<MARKERS:OFF>` | OFF: <MARK> e <BOOKMARK> non scrivono più segni; ON li riaccende | fatto |
| Finestre e testo | `<BMERASE>` | toglie dalla finestra RX le righe dei segni | fatto |
| Finestre e testo | `<ALERT:testo>` | avviso a schermo con un suono | fatto |
| Finestre e testo | `<PRINT:>` | stampa il testo della finestra RX | fatto |
| Finestre e testo | `<ACTIVATE>` | porta la finestra di Aurora davanti | fatto |
| Audio | `<RXLOG START>` `<RXLOG STOP>` | comincia a tenere il testo ricevuto; <RXLOG STOP> lo salva in un file .txt | fatto |
| Audio | `<STARTRECORD:nome>` `<STARTRECORD2:nome>` | registra l'audio ricevuto in un file WAV della cartella recordings (senza nome: data e ora) | fatto |
| Audio | `<STOPRECORD>` | ferma e salva la registrazione | fatto |
| Audio | `<STARTPLAY:nome>` `<STOPPLAY>` | trasmette una registrazione (cartella recordings o dei testi delle macro); <STOPPLAY> la ferma | fatto |
| Audio | `<REPLAY:n>` | fa ripassare nel ricevitore gli ultimi n secondi ricevuti (di base 10, al massimo 60) | fatto |
| Audio | `<WAVE:nome>` | trasmette il file .wav della cartella dei testi delle macro | fatto |
| Audio | `<DTMF:cifre>` | trasmette i toni DTMF (0-9, *, #, A-D; la virgola fa una pausa) | fatto |
| File e script | `<FILE:nome>` | il testo di un file della cartella dei testi delle macro; <FILE> senza nome fa scegliere un file da mettere nella casella di trasmissione | fatto |
| File e script | `<INIFILE:file,sezione,chiave>` | il valore di una chiave di un file INI della cartella dei testi delle macro (solo lettura) | fatto |
| File e script | `<RANDOM:file>` | una riga a caso del file | fatto |
| File e script | `<SCRIPT:file>` `<TNCSCRIPT:file>` `<TELNETSCRIPT:file>` | esegue come macro il testo del file (mai programmi esterni) | fatto |
| File e script | `<OnRX:testo,macro>` | quando nella ricezione arriva quel testo (* = qualsiasi) esegue la macro; <OnRX:> toglie tutte le attese | fatto |
| SSTV | `<PIC:file>` | trasmette in SSTV un'immagine della cartella delle immagini da trasmettere (%C: con il nominativo del corrispondente) | fatto |
| SSTV | `<SSTVMODE:modo>` | modo SSTV per <PIC> e <FROMCLIP> (senza nome: quello delle impostazioni SSTV) | fatto |
| SSTV | `<PUTRXPICT:n>` | ritrasmette in SSTV l'ultima immagine ricevuta (n: la n-esima più recente) | fatto |
| SSTV | `<PLACETEXT:testo>` `<TEXTSTYLE:...>` `<TRANSPARENTCOLOR:...>` `<ONLY:...>` `<PUTHEADER>` | testi, stili, trasparenza, intestazione e righe dell'immagine si preparano nella pagina SSTV: il comando la apre | al più vicino: testi e stili dell'immagine si fanno nella pagina SSTV: il comando la apre |
| Packet e rete | `<TALK:nominativo,testo>` | manda un messaggio TALK alla stazione tramite il DX cluster collegato | al più vicino: senza packet il messaggio passa dal DX cluster (comando TALK) |
| Packet e rete | `<YAPPSEND:file>` `<YAPPRECV>` `<PACKET>` | Aurora non ha il packet: avviso (vale anche per YAPPRECV e PACKET) | al più vicino: Aurora non ha il packet: avviso una volta |
| Macro dentro una macro | `<F1>` | esegue un'altra macro (tasto o nome, come <Hello>) dentro questa, fino a 10 livelli | fatto |
| Macro dentro una macro | `<CALL?>` `<RSTS?>` `<RSTR?>` `<NRS?>` `<NRR?>` | chiede il valore se è vuoto (anche <RSTS?>, <RSTR?>, <NRS?>, <NRR?>) | fatto |
| Macro dentro una macro | `<F1?>` | apre quella macro per modificarla | fatto |
| Finestre e testo | `<EXT:programma>` | non supportato per sicurezza: Aurora non avvia programmi esterni | escluso |

`<EXEC:...>` ed `<EXT:...>` restano esclusi: Aurora non avvia programmi esterni (scelta del proprietario) e li salta in silenzio.
I comandi che parlano col core usano le rotte `/api/v1/digi/cat`, `/digi/winkeyer/send`, `/digi/play`, `/digi/record`, `/rotator/*`, `/dx-cluster/command`, `/sstv/tx`, `/log/qsos`, `/output-level`, `/input-level`.
