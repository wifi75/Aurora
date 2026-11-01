# Changelog

## Non rilasciato

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
- **Dedica ad Aura**, la figlia dell'autore da cui prende il nome Aurora: nella pagina Informazioni e nei README, in sei lingue.
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
