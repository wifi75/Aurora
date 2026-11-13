# Copertura della Guida

Tabella di controllo: per ogni funzione e ogni impostazione di Aurora, dove sta e quale voce della Guida (`id` in `app/src/i18n/guide*.ts`) la spiega. «manca» indica una voce che la Guida non spiega, con il motivo. Le sezioni delle Impostazioni sono organizzate come l'albero vero: famiglia › pagina › riquadro; il capitolo «Domande frequenti» rimanda alle voci complete.

Come si aggiorna: quando si aggiunge o si cambia una voce delle Impostazioni o una funzione, si aggiunge o si corregge la riga qui e la voce nel file della Guida; i testi usano `[[chiave]]` per i nomi, così seguono la traduzione.

Mappa dei file: `guide.ts` (primi passi, radio, operare, log, telefono, problemi), `guide-faq.ts` (domande frequenti), `guide-pagine.ts` e `guide-testuali.ts` (pagine e pannelli), `guide-station.ts`, `guide-modi1.ts`, `guide-modi2.ts`, `guide-testo.ts`, `guide-log.ts`, `guide-aspetto.ts`, `guide-accesso.ts` (una per famiglia delle Impostazioni).

## Impostazioni › Stazione


Famiglia `sw.sec.station` («Stazione»): Dati personali, Radio (CAT e PTT), Profili radio, Audio, Rotore, SDR · TCI · QMAP, Orologio.
Le sezioni sono in `app/src/i18n/guide-station.ts` (`GUIDE_STATION`). Percorso di ogni voce: Impostazioni › Stazione › pagina › riquadro.
Gli id `s-...` sono delle sezioni nuove; gli altri sono voci già presenti in `guide.ts`.

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Dati personali | Stazione › Dati personali | s-personal |
| Dati della stazione · Mio nominativo | Dati personali › Dati della stazione | s-personal |
| Dati della stazione · Mio locatore | Dati personali › Dati della stazione | s-personal |
| Dati della stazione · Regione IARU (tutte, 1, 2, 3) | Dati personali › Dati della stazione | s-personal |
| Dati della stazione · Generazione dei messaggi per nominativi composti di tipo 2 | Dati personali › Dati della stazione | s-personal |
| Operatore · Nome, QTH | Dati personali › Operatore | s-personal |
| Operatore · Cognome, Email (facoltativa), nota sui dati | Dati personali › Operatore | s-personal |
| Pagina Radio (CAT e PTT) | Stazione › Radio (CAT e PTT) | s-radio, s-radio-state, s-radio-display |
| Avvisi OmniRig non installato / rigctld mancante, pulsante Ripristina Hamlib (solo Windows) | Radio › Collegamento | s-radio |
| Sistema di controllo (Hamlib, OmniRig, TCI, FT-991(A) Link, Cat4OM, Nessuno) | Radio › Collegamento | s-radio (Nessuno: anche `nocat`) |
| Indirizzo TCI, Ricevitore (RX1/RX2) | Radio › Collegamento (solo TCI) | s-radio, `tci` |
| Radio di OmniRig (Rig 1/Rig 2) | Radio › Collegamento (solo OmniRig) | s-radio |
| Indirizzo FT-991(A) Link | Radio › Collegamento (solo FT-991(A) Link) | s-radio, `ft991link` |
| Gestione Cat4OM, Gruppo, Controllo, Chiave del gruppo, Radio | Radio › Collegamento (solo Cat4OM) | s-radio, `cat4om` |
| Pulsante Cerca di Cat4OM, elenco «Gruppi e radio», Usa/Scelta, problema audio in rete | Radio › Collegamento (solo Cat4OM) | s-radio |
| Cerca (marca o modello) e menu Radio | Radio › Collegamento (solo Hamlib) | s-radio |
| Porta (COM, /dev/ttyUSB*, /dev/ttyACM*, indirizzo:porta) | Radio › Collegamento (solo Hamlib) | s-radio, s-serial-linux |
| Pulsante Aggiorna le porte | Radio › Collegamento (solo Hamlib) | s-radio, s-serial-linux |
| Permesso Linux «Permission denied», gruppo dialout (uucp su Arch) | Radio › Collegamento (Linux) | s-serial-linux |
| Velocità (baud) | Radio › Collegamento (Hamlib, porta seriale) | s-radio |
| Bit di dati, Bit di stop, Controllo di flusso, Linea DTR, Linea RTS | Radio › Collegamento (Hamlib, porta seriale) | s-radio |
| Indirizzo CI-V (Icom), nota esadecimale e pulsante «Usa» | Radio › Collegamento (Hamlib, porta seriale, Icom) | s-civ, s-radio |
| Pulsante Applica, messaggi «Modifiche non ancora applicate» e «Un valore non va» | Radio › Collegamento | s-radio |
| Impostazioni consigliate per questa radio (In Aurora, Nel menu della radio, In Audio, Apri Audio, Applica le impostazioni consigliate) | Radio › riquadro dopo il collegamento (solo Hamlib e modello noto) | s-radio-state, `radiosetup` |
| Stato e prova · stato, frequenza, modo, PTT, avviso Icom a 115200 | Radio › Stato e prova | s-radio-state |
| Pulsanti Collega, Scollega | Radio › Stato e prova | s-radio-state |
| Pulsante Rileva la mia radio, elenco «Radio rilevate», Usa (solo Hamlib) | Radio › Stato e prova | s-radio-state |
| PTT · Metodo del PTT (CAT, DTR, RTS, VOX) | Radio › PTT (solo Hamlib) | s-radio-state |
| PTT · Porta del PTT | Radio › PTT (solo Hamlib con DTR o RTS) | s-radio-state |
| Split, modo e lettura · Split | Radio › Split, modo e lettura | s-radio-state |
| Split, modo e lettura · Modo scritto nella radio | Radio › Split, modo e lettura | s-radio-state |
| Split, modo e lettura · Lettura della radio ogni (s) | Radio › Split, modo e lettura (solo Hamlib) | s-radio-state |
| Collega la radio all'avvio di Aurora | Radio › Split, modo e lettura | s-radio-state |
| Leggi anche gli altri dati della radio | Radio › Split, modo e lettura | s-radio-state, `display` |
| Accendi la radio all'avvio, Spegni la radio alla chiusura | Radio › Split, modo e lettura (Hamlib, TCI, FT-991(A) Link; con Cat4OM solo Spegni) | s-radio-state |
| Condividi la radio con altri programmi, Porta di condivisione, indirizzo mostrato | Radio › Condividi la radio con altri programmi (solo Hamlib) | s-radio-display, `sharing` |
| Stile del display della radio (13 stili con anteprima) | Radio › Stile del display della radio | s-radio-display, `display` |
| Disposizione in testata (Blocco unico a doppio VFO, Due blocchi uguali affiancati) | Radio › Stile del display della radio | s-radio-display, `display` |
| Nascondi il display | Radio › Stile del display della radio | s-radio-display |
| Display esteso: cosa mostrare (10 caselle: Secondo VFO, Strumenti, Tensione/corrente/compressione, Preamplificatore e attenuatore, AGC, Riduzione del rumore, Guadagni, RIT e XIT, Sintonizzatore, VFO in uso) | Radio › Display esteso: cosa mostrare | s-radio-display, `display` |
| Condivisione della radio · Comando dalla pagina (OmniRig) | Radio › Condivisione della radio | s-radio-display |
| Riquadro Note (comprimibile) | Radio › Note | s-radio-display |
| Pulsanti dei display esteso (PRE, ATT, AGC, NB, NR, NOTCH, ATU, VFO, RIT, XIT) | Stazione (pagina principale), non nelle Impostazioni | `radiobuttons` |
| Pulsante Prova CAT | Impostazioni › in basso, in ogni pagina | s-test-buttons, `radiocheck` |
| Pulsante Prova PTT | Impostazioni › in basso, in ogni pagina | s-test-buttons, `radiocheck` |
| Pulsanti Chiudi e Salva, riga di stato «n modifiche da applicare» | Impostazioni › in basso, in ogni pagina | s-test-buttons |
| Pagina Profili radio | Stazione › Profili radio | s-profiles |
| Profili salvati (tendina) | Profili radio › Profilo | s-profiles |
| Nome del profilo | Profili radio › Profilo | s-profiles |
| Pulsanti Carica, Salva, Nuovo, Elimina; come creare, cambiare, cancellare un profilo | Profili radio › Profilo | s-profiles |
| Pagina Audio | Stazione › Audio | s-audio |
| Ingresso audio (tendina, «in uso», «non trovato», predefinito) | Audio › Ingresso audio | s-audio |
| Volume di registrazione (Windows): cursore Rx e indicatore AUTO | Audio › Ingresso audio | s-audio-levels, s-audio |
| Canale d'ingresso (Mono, Sinistro, Destro) | Audio › Ingresso audio | s-audio |
| Riga «In uso adesso» e messaggi di ingresso non aperto | Audio › Ingresso audio | s-audio |
| Uscita audio (trasmissione) (tendina) | Audio › Uscita audio (trasmissione) | s-audio |
| Volume di trasmissione (Windows, Pwr): cursore Pwr | Audio › Uscita audio (trasmissione) | s-audio-levels, `pwr` |
| Canale d'uscita (Mono, Sinistro, Destro, Entrambi) | Audio › Uscita audio (trasmissione) | s-audio |
| Salvataggio dei file audio (Nessuno, Decodificati, Tutti) | Audio › Salvataggio dei file audio | s-audio |
| Cartella dei file audio | Audio › Salvataggio dei file audio | s-audio |
| Tieni i file salvati (giorni): 0 = per sempre, di base 7; pulizia dei .wav all'avvio e ogni giorno, rotazione di ALL.TXT | Audio › Salvataggio dei file audio | s-audio |
| Applica, Annulla, Aggiorna elenco, «Modifiche da applicare», esito dell'applicazione | Audio › Applica | s-audio |
| Audio TCI e Cat4OM nelle tendine | Audio › Ingresso/Uscita | s-audio, `tci` |
| Riquadro Note | Audio › Note | s-audio |
| Indicatore Rx, cursore «Volume di registrazione», pulsante AUTO (pagina principale) | Stazione (pagina principale) | s-audio-levels, `audio` |
| Pagina Rotore | Stazione › Rotore | s-rotator |
| Rotore acceso | Rotore › Collegamento del rotore | s-rotator |
| Collegamento (Aurora avvia rotctld / rotctld già acceso), avviso rotctld non trovato | Rotore › Collegamento del rotore | s-rotator |
| Cerca il rotore, Modello | Rotore › Collegamento del rotore (Aurora avvia rotctld) | s-rotator |
| Porta seriale, Velocità | Rotore › Collegamento del rotore (Aurora avvia rotctld) | s-rotator, s-serial-linux |
| Indirizzo di rotctld, Porta TCP | Rotore › Collegamento del rotore (rotctld già acceso) | s-rotator |
| Campo di rotazione, Da, A (Altro…) | Rotore › Collegamento del rotore | s-rotator |
| Stato | Rotore › Collegamento del rotore | s-rotator |
| Pulsante Applica, messaggi di esito | Rotore › Collegamento del rotore | s-rotator |
| Uso del rotore: pagina Rotore, Gira, Via lunga, Ferma | Rotore (pagina principale) | `rotator` |
| Pagina SDR · TCI · QMAP | Stazione › SDR · TCI · QMAP | s-sdr |
| Indirizzo del server TCI | SDR › Collegamento TCI | s-sdr |
| Righe di sola lettura Sistema di controllo e Audio | SDR › Collegamento TCI | s-sdr |
| Pulsante Usa il TCI per tutto, conferma ed esito | SDR › Collegamento TCI | s-sdr, `tci` |
| Pagina QMAP nel menu (serve un SDR) | SDR › QMAP | s-sdr, `qmap` |
| Pulsante Apri QMAP, riga della sorgente I/Q | SDR › QMAP | s-sdr, `qmap` |
| Radio adatte (testo), Guida: SDR con TCI, Guida: QMAP | SDR › Radio adatte | s-sdr |
| Pagina Orologio | Stazione › Orologio | s-clock |
| Stato dell'orologio (scarto, ritardo, server, risposte, ultimo controllo, corretto) | Orologio › Stato dell'orologio | s-clock, `clock` |
| Pulsante Controlla ora | Orologio › Stato dell'orologio | s-clock |
| Pulsante Correggi l'orologio ora (solo Windows, da amministratore) | Orologio › Stato dell'orologio | s-clock |
| Spia e pulsante Prova (NTP) | Orologio › Stato dell'orologio | s-clock |
| Controllo periodico acceso | Orologio › Impostazioni | s-clock |
| Correggi da solo l'orologio di Windows | Orologio › Impostazioni | s-clock |
| Aggiorna l'ora all'avvio di Aurora | Orologio › Impostazioni | s-clock, `clock` |
| Controlla ogni (minuti) | Orologio › Impostazioni | s-clock |
| Soglia di allarme (millisecondi) | Orologio › Impostazioni | s-clock |
| Server NTP (uno per riga) | Orologio › Impostazioni | s-clock |
| Riquadro Note, indicatore dell'orologio in alto e avviso giallo | Orologio › Note; barra in alto | s-clock, `clock` |

#### Cosa resta scoperto

- Con il motore JTDX (non più il caso normale) le pagine Audio e Radio mostrano solo righe di sola lettura (`audio.input`, `radio.control`): la sezione `s-audio` spiega la pagina del motore proprio. Nessuna voce modificabile resta senza spiegazione.
- I messaggi di errore e di esito del CAT, dell'audio, del rotore e dell'orologio (testo rosso accanto ai pulsanti) sono descritti in generale (motivo, cosa controllare), non uno per uno: sono frasi complete già tradotte nell'interfaccia.


## Impostazioni › Modi: comune a tutti i modi e FT8 · FT4 (frequenze, registrazione, Auto CQ, sequenza, macro, automazioni)


Parte `modi1`. Sezioni nel file `app/src/i18n/guide-modi1.ts` (export `GUIDE_MODI1`). Percorso base: Impostazioni › Modi.
Le voci del motore compaiono solo con il motore proprio di Aurora (quello di base) e solo se «vive»; dove una voce non si vede sempre è scritto.

#### Pagina Frequenze (Comune a tutti i modi › Frequenze) · id `s-freq`

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
| --- | --- | --- |
| Pagina Frequenze (tabella delle frequenze di dial) | Modi › Comune a tutti i modi › Frequenze | s-freq |
| Aggiungi riga | Frequenze › riquadro Frequenze | s-freq |
| Frequenze iniziali | Frequenze › riquadro Frequenze | s-freq |
| Salva (la tabella si salva anche da sola) | Frequenze › riquadro Frequenze | s-freq |
| Messaggio «Tabella non valida…» | Frequenze › riquadro Frequenze | s-freq |
| Colonna Banda | Frequenze › tabella | s-freq |
| Colonna Modo | Frequenze › tabella | s-freq |
| Colonna Frequenza (MHz) | Frequenze › tabella | s-freq |
| Colonna Regione IARU | Frequenze › tabella | s-freq |
| Colonna Predefinita | Frequenze › tabella | s-freq |
| Pulsante ✕ Elimina (fine riga) | Frequenze › tabella | s-freq |
| Righe grigie «nascosta dalla regione IARU» | Frequenze › tabella | s-freq |
| Scelta della regione IARU (sta in Dati personali) | Stazione › Dati personali › Dati della stazione | s-freq (citata) |
| Nota in fondo alla pagina (testo di aiuto) | Frequenze › Note | manca: è solo un testo di aiuto e dice cose superate (vedi rapporto), non serve una riga |

#### Pagina Registrazione e comportamento (Comune a tutti i modi) · id `s-qso`, `s-qso-log`

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
| --- | --- | --- |
| Pagina Registrazione e comportamento | Modi › Comune a tutti i modi › Registrazione e comportamento | s-qso |
| Come si registrano i QSO (procedura) | stessa pagina | s-qso-log |
| Comportamento · Non monitorare all’avvio | riquadro Comportamento | s-qso |
| Filtro del log · Usa questo nominativo | riquadro Filtro del log | s-qso |
| Filtro del log · Usa questa griglia | riquadro Filtro del log | s-qso |
| Filtro del log · Filtra il log a partire dalla data/ora | riquadro Filtro del log | s-qso |
| Registrazione log · Richiedimi di loggare il QSO (finestra di conferma a fine QSO, con campi e pulsanti Registra/Scarta) | riquadro Registrazione log | s-qso, s-qso-log |
| Registrazione log · Messa a log automatica del QSO | riquadro Registrazione log (non compare: sempre attiva) | s-qso (spiegato che non c’è e cosa fa), s-qso-log |
| Registrazione log · Converti il modo in RTTY (vale solo per JT9) | riquadro Registrazione log | s-qso |
| Registrazione log · Rapporto dB nel commento | riquadro Registrazione log | s-qso, s-qso-log |
| Registrazione log · Distanza nei commenti | riquadro Registrazione log | s-qso, s-qso-log |
| Registrazione log · Cancella i dati dopo aver loggato il QSO | riquadro Registrazione log | s-qso |
| Registrazione log · Cancella il Nom DX e la Griglia all’uscita | riquadro Registrazione log (non compare: già sempre rispettata) | s-qso (spiegato che non c’è) |
| Registrazione su ALL.TXT · Messaggi decodificati | riquadro Registrazione su ALL.TXT | s-qso |
| Registrazione su ALL.TXT · Messaggi decodificati e di debug | riquadro Registrazione su ALL.TXT | s-qso |
| «Rimanda i QSO persi quando il log si riapre» | non è in questa pagina: sta in Log e servizi › Segnalazioni | s-qso-log (citata con il percorso) |
| Pulsante «Log QSO» per registrare a mano | Modi › FT8 · FT4 › Messaggi e filtri (pagina dell’altro collega) | s-qso (citata nel consiglio) |

#### Pagina Auto CQ (Comune a tutti i modi › Auto CQ) · id `s-autocq`, `s-autocq-cq`

La pagina si chiama «Auto CQ» ma contiene la Programmazione delle bande per orario UTC; il CQ automatico vero sta nelle Automazioni.

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
| --- | --- | --- |
| Pagina Auto CQ (Programmazione) | Modi › Comune a tutti i modi › Auto CQ | s-autocq |
| Tabella · Ora (5 righe) | riquadro Programmazione | s-autocq |
| Tabella · Minuto (5 righe) | riquadro Programmazione | s-autocq |
| Tabella · Banda (tendina, voce «—» toglie la riga) | riquadro Programmazione | s-autocq |
| Impostazioni · Usa la banda programmata | riquadro Impostazioni | s-autocq |
| Impostazioni · Salva (si salva anche da solo) e messaggio «Programmazione non valida» | riquadro Impostazioni | s-autocq |
| Nota in fondo alla pagina (testo di aiuto) | Auto CQ › Note | s-autocq (il contenuto è ripreso nella voce «Usa la banda programmata») |
| Come far chiamare CQ da sola (procedura) | Automazioni › Quando non c’è nessuno | s-autocq-cq |

#### Pagina Sequenza (FT8 · FT4 › Sequenza) · id `s-sequence`

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
| --- | --- | --- |
| Pagina Sequenza | Modi › FT8 · FT4 › Sequenza | s-sequence |
| Sequenza e scelta di chi chiamare · Sequenza automatica (AutoSeq) | riquadro Sequenza e scelta di chi chiamare | s-sequence |
| … · Un solo QSO | stesso riquadro | s-sequence |
| … · Rispondi a tutte le stazioni che mi chiamano (coda) | stesso riquadro | s-sequence |
| … · Salta Tx1 | stesso riquadro | s-sequence |
| … · Rispondi ai già lavorati (Risp B4) | stesso riquadro | s-sequence |
| … · Priorità ai CQ | stesso riquadro | s-sequence |
| … · Priorità alla distanza maggiore | stesso riquadro | s-sequence |
| Tra i messaggi decodificati… · quattro scenari (risposta al CQ, chiamata CQ e risposta, rapporto di risposta, RR73), ognuno con casella e numero di volte | riquadro «Tra i messaggi decodificati, chiama CQ o ferma il Tx se:» | s-sequence |
| Chiamata e arresto · Ferma il Tx se l’operatore ha risposto a un altro | riquadro Chiamata e arresto | s-sequence |
| Chiamata e arresto · Operazione CQ rigorosa | riquadro Chiamata e arresto | s-sequence |
| Funzionamento · Consenti cambi di frequenza Tx durante la trasmissione | riquadro Funzionamento | s-sequence |
| Funzionamento · Periodo di monitoraggio Tx (watchdog) | riquadro Funzionamento | s-sequence |
| Funzionamento · Ferma la Tx con ROS oltre | riquadro Funzionamento | s-sequence |
| Funzionamento · Ritardo Tx (s) | riquadro Funzionamento | s-sequence |
| Decodifica e attesa · Full duplex (satelliti) | riquadro Decodifica e attesa | s-sequence |
| Decodifica e attesa · Wait and Reply / Wait and Call | riquadro Decodifica e attesa | s-sequence |
| Decodifica e attesa · Wait and Pounce: a quale CQ (tendina con cinque scelte) | riquadro Decodifica e attesa | s-sequence |
| Band Hopping · Band Hopping acceso | riquadro Band Hopping | s-sequence |
| Band Hopping · Bande e frequenze (elenco) | riquadro Band Hopping | s-sequence |
| Note in fondo (testi di aiuto della pagina) | Sequenza › Note | s-sequence (contenuto ripreso nelle singole voci) |
| Problemi tipici (si ferma da sola, inizio tagliato, avviso ROS) | Sequenza | s-sequence (blocco «Se non va») |

La verifica del Fox (Hound) non è su questa pagina: sta in Contest · Fox/Hound (altro collega).

#### Pagina Macro Tx (FT8 · FT4 › Macro Tx) · id `s-macros`

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
| --- | --- | --- |
| Pagina Macro Tx | Modi › FT8 · FT4 › Macro Tx | s-macros |
| Elenco delle macro con pulsante Elimina e messaggio «Nessuna macro» | riquadro Macro TX | s-macros |
| Aggiungi · campo Nuova macro (fino a 30 caratteri) | riquadro Aggiungi | s-macros |
| Aggiungi · pulsante Aggiungi | riquadro Aggiungi | s-macros |
| Aggiungi · pulsante Ripristina | riquadro Aggiungi | s-macros |
| Legenda dei simboli @ & # (testo di aiuto) | Macro Tx › Note | s-macros |

#### Pagina Automazioni (FT8 · FT4 › Automazioni) · id `s-automation`

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
| --- | --- | --- |
| Pagina Automazioni | Modi › FT8 · FT4 › Automazioni | s-automation |
| Testata · stato Accesa/Spenta | in alto | s-automation |
| Testata · Modo delle Automazioni (Prova a vuoto / Modo reale, avviso di conferma) | in alto | s-automation |
| Testata · etichetta Fuori fascia | in alto | s-automation |
| Testata · Risposte oggi / Risposte (prova), QSO fatti, Ultima azione | in alto | s-automation |
| Testata · Accendi / Spegni | in alto | s-automation |
| A chi rispondere · sette caselle delle novità (nuovo DXCC, DXCC su nuova banda o modo, nuova griglia, griglia su nuova banda o modo, nuova zona CQ, nuovo nominativo, nominativo su nuova banda o modo) | blocco A chi rispondere | s-automation |
| A chi rispondere · Nuovo modo, Nuovo stato (USA), Stato sulla nuova banda (USA), Desiderati | blocco A chi rispondere | s-automation |
| A chi rispondere · Segnale minimo (dB) | blocco A chi rispondere | s-automation |
| A chi rispondere · Rispetta i CQ direzionali | blocco A chi rispondere | s-automation |
| Chi scegliere prima · La novità più rara / Il segnale più forte / Il migliore a punteggio | blocco Chi scegliere prima | s-automation |
| Limiti di sicurezza · Tentativi per stazione | blocco Limiti di sicurezza | s-automation |
| Limiti di sicurezza · Pausa per stazione senza risposta (minuti) | blocco Limiti di sicurezza | s-automation |
| Limiti di sicurezza · QSO all’ora | blocco Limiti di sicurezza | s-automation |
| Limiti di sicurezza · Attiva solo nella fascia oraria, Dalle, Alle | blocco Limiti di sicurezza | s-automation |
| Limiti di sicurezza · riga dei tetti fissati sul PC (solo da leggere) | blocco Limiti di sicurezza | s-automation |
| Esclusioni · Filtra chi risponde al mio CQ anche senza Auto | blocco Esclusioni | s-automation |
| Esclusioni · Stazioni escluse | blocco Esclusioni | s-automation |
| Esclusioni · Paesi esclusi (tendina e etichette ×, messaggio «elenco non disponibile») | blocco Esclusioni | s-automation |
| Esclusioni · Continenti (casella Escludi e sette continenti) | blocco Esclusioni | s-automation |
| Quando non c’è nessuno · CQ da solo e Chiama CQ da solo dopo (minuti) | blocco Quando non c’è nessuno | s-automation, s-autocq-cq |
| Quando non c’è nessuno · Band Hopping (solo con il motore proprio) e Elenco delle bande… | blocco Quando non c’è nessuno | s-automation |
| Prossime stazioni | riquadro in fondo | s-automation |
| Ultime stazioni chiamate in automatico | riquadro in fondo | s-automation |
| Registro delle decisioni (colori e tipi) | riquadro in fondo | s-automation |
| Problemi tipici (non risponde, si è spenta) | Automazioni | s-automation (blocco «Se non va») |

#### Voci non coperte

- «Tieni i file salvati per N giorni»: non è tra le pagine di questa area, quindi non la spiego qui.
- «Rimanda i QSO persi quando il log si riapre»: sta in Log e servizi, non in questa area; la cito con il percorso in `s-qso-log`.
- Testi di aiuto delle pagine (Note in fondo): ripresi nelle voci, non hanno una riga propria.


## Impostazioni › Modi: decodifica, messaggi e filtri, contest, JT, VHF, WSPR


Area `modi2`: pagine `ft8decode`, `messages`, `contest`, `jt`, `vhf`, `contestvhf`, `wspr` della famiglia «Modi» (`sw.fam.modes`).
Testi della Guida in `app/src/i18n/guide-modi2.ts` (`GUIDE_MODI2`). Percorso di ogni pagina: Impostazioni › Modi › sottofamiglia › pagina.
Le pagine `freq`, `qso`, `autocq`, `sequence`, `macros`, `automation` sono di un altro incarico.

Le voci del motore (decodifica, Fox, spot WSPRnet) compaiono solo con il motore proprio di Aurora e valgono dopo «Salva»; i filtri, le preferenze di schermo e i comandi del radiofaro agiscono subito.

#### FT8 · FT4 › Decodifica (`ft8decode`)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Decodifica | Modi › FT8 · FT4 › Decodifica | s-modi2-ft8dec |
| Avvio della decodifica FT8 (Presto / Normale / Tardi) | Decodifica › Decodifica | s-modi2-ft8dec |
| Profondità di decodifica (Veloce / Normale / Profonda) | Decodifica › Decodifica | s-modi2-ft8dec |
| Decodifica singola (solo attorno a Rx) | Decodifica › Decodifica | s-modi2-ft8dec |
| Pagina vuota senza motore proprio | Decodifica | s-modi2-ft8dec (rimedio «Se non va») |

#### FT8 · FT4 › Messaggi e filtri (`messages`)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Messaggi e filtri (filtri, CQ e novità, Schermo) | Modi › FT8 · FT4 › Messaggi e filtri | s-modi2-messages |
| Novità mostrate: pillole Tutte e le 14 categorie (CQ, nuovo DXCC, DXCC su nuova banda o modo, zona CQ, zona ITU, griglia, prefisso, nominativo, già lavorata) | Attività di banda: cosa mostrare | s-modi2-messages |
| SNR minimo (dB) | Attività di banda: cosa mostrare | s-modi2-messages |
| Distanza minima e massima (km) | Attività di banda: cosa mostrare | s-modi2-messages |
| Nascondi i messaggi a me | Attività di banda: cosa mostrare | s-modi2-messages |
| Messaggi nuovi in fondo (Attività di banda) | Attività di banda: cosa mostrare | s-modi2-messages |
| Solo su queste bande (una casella per banda della barra) | Attività di banda: cosa mostrare | s-modi2-messages |
| Nota esplicativa del riquadro | Attività di banda: cosa mostrare | s-modi2-messages (testo introduttivo) |
| Nascondi le mie trasmissioni | Finestra RX | s-modi2-messages |
| Messaggi nuovi in fondo (Finestra RX) | Finestra RX | s-modi2-messages |
| Messaggi del QSO solo nella Finestra RX | Finestra RX | s-modi2-messages |
| Età dei messaggi al posto dell'ora | Attività di banda e Finestra RX: come si vedono | s-modi2-messages |
| Nascondi i messaggi dai continenti (7 caselle) | Nascondi i messaggi dai continenti | s-modi2-messages |
| Nascondi i messaggi non standard | Opzioni | s-modi2-messages |
| Nascondi i messaggi del proprio continente | Opzioni | s-modi2-messages |
| Nascondi i CQ di stazioni già collegate su questa banda | Opzioni | s-modi2-messages |
| Con il filtro precedente, nascondi anche i loro scambi | Opzioni | s-modi2-messages |
| Sospendi tutti i filtri | Opzioni | s-modi2-messages (e `filters`) |
| Pulsante Ripristina (solo filtri) | Opzioni | s-modi2-messages |
| Nascondi i messaggi dai paesi | Elenchi | s-modi2-messages |
| Nascondi i messaggi dai nominativi | Elenchi | s-modi2-messages (e `filters`) |
| Mostra solo questi nominativi | Elenchi | s-modi2-messages (e `filters`) |
| Mostra sempre questi nominativi | Elenchi | s-modi2-messages (e `filters`) |
| Mostra solo (tendina: tutti, solo CQ, CQ/RRR/RR73/73, CQ/RR73/73) | Elenchi | s-modi2-messages |
| Nominativi o prefissi da evidenziare | Evidenzia nell'elenco | s-modi2-messages (e `filters`) |
| Paesi da evidenziare | Evidenzia nell'elenco | s-modi2-messages |
| Nota sui filtri (restano decodificati) | fondo dei filtri | s-modi2-messages (testo introduttivo) |
| CQ e novità: caselle delle 14 categorie | CQ e novità: cosa elencare | s-modi2-messages |
| CQ e novità: elenca SOLO queste stazioni / non elencare mai queste stazioni | CQ e novità: cosa elencare | s-modi2-messages |
| CQ e novità: righe al massimo | CQ e novità: cosa elencare | s-modi2-messages |
| CQ e novità: Ripristina | CQ e novità: cosa elencare | s-modi2-messages |
| Riga vuota tra i periodi di decodifica | Schermo | s-modi2-messages |
| Distanza in miglia invece che in km | Schermo | s-modi2-messages |
| Visualizza i tuoi messaggi Tx nella finestra Rx | Schermo | s-modi2-messages |
| Mostra il nome del paese / Mostra la bandiera | Schermo | s-modi2-messages |
| Mostra la distanza / Mostra l'azimut | Schermo | s-modi2-messages |
| Mostra la colonna DT / Età dei messaggi (stessa casella dei filtri) | Schermo | s-modi2-messages |
| Esc: ferma la trasmissione | Schermo | s-modi2-messages |
| Cambiando banda o modo svuota l'attività di banda | Schermo | s-modi2-messages |
| Doppio clic: rispondi alla stazione | Schermo | s-modi2-messages |
| Pulsante Tune / pulsante Log QSO nel pannello Comandi | Schermo | s-modi2-messages |
| QSO mostrati nella finestra Log (Log4OM2) | Schermo | s-modi2-messages |
| Pulsante Ripristina (tutte le preferenze di schermo) | Schermo | s-modi2-messages |
| Pagina dei messaggi del proprietario (`OwnerMessagesTab`) | non è nell'albero delle Impostazioni: indirizzo riservato `#/messaggi-autore` (route `ownerMessages`) | manca: strumento interno del proprietario per mandare messaggi a tutte le installazioni, non una funzione per gli utenti |

#### FT8 · FT4 › Contest · Fox/Hound (`contest`)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Contest · Fox/Hound | Modi › FT8 · FT4 › Contest · Fox/Hound | s-modi2-contest (e `contest`) |
| Hound automatico | Verifica del Fox (Hound) | s-modi2-contest |
| Verifica in linea il codice OTP | Verifica del Fox (Hound) | s-modi2-contest |
| Mostra i messaggi OTP | Verifica del Fox (Hound) | s-modi2-contest |
| Indirizzo del servizio di verifica | Verifica del Fox (Hound) | s-modi2-contest |
| Tendina Contest (con «attivo» e «Scegli un contest…») e nota/errore | Impostazioni dei contest | s-modi2-contest |
| Messaggi (attività) e Parola del CQ | Impostazioni dei contest | s-modi2-contest |
| Scambio fisso e Primo numero progressivo | Impostazioni dei contest | s-modi2-contest |
| Categoria, Modo, Potenza (W), Punti bonus | Impostazioni dei contest | s-modi2-contest |
| Hold Tx Freq e Chiusura RRR | Impostazioni dei contest | s-modi2-contest |
| Nominativo in gara, Locatore a 6 caratteri, Antenna, Operatori, Club | Impostazioni dei contest | s-modi2-contest |
| Nome, Email, Indirizzo (ADDRESS), LOCATION, Assistenza | Impostazioni dei contest | s-modi2-contest |
| Nome del contest nel Cabrillo, Categoria di tempo, Categoria aggiuntiva, Soapbox | Impostazioni dei contest | s-modi2-contest |
| Inizio (UTC) e Fine (UTC) | Impostazioni dei contest | s-modi2-contest |
| Frequenze del contest (pulsanti banda · modo · kHz) | Impostazioni dei contest | s-modi2-contest |
| Pulsante Salva del modulo | Impostazioni dei contest | s-modi2-contest |
| Pulsante Apri la pagina Contest | Impostazioni dei contest | s-modi2-contest |

#### JT65 · JT9 · FST4 (`jt`)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina JT65 · JT9 · FST4 | Modi › JT65 · JT9 · FST4 | s-modi2-jt (e `jt`, `jt65sub`) |
| JT65: analisi a priori (AP) | Decodifica | s-modi2-jt |
| JT65: ricerca profonda | Decodifica | s-modi2-jt |
| JT65: livello aggressivo (0-10) | Decodifica | s-modi2-jt |
| F Tol JT65 e F Tol JT9 | Decodifica | s-modi2-jt |
| Voci proprie di FST4 e FST4W | Decodifica | manca: nella pagina non ci sono voci per FST4/FST4W (il radiofaro FST4W si regola in WSPR · FST4W, spiegato in s-modi2-wspr e nella voce `fst4`) |

#### VHF · EME › Opzioni dei modi (`vhf`)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Opzioni dei modi | Modi › VHF · EME › Opzioni dei modi | s-modi2-vhf (e `q65`, `msk144`) |
| Q65: media dei periodi | Decodifica | s-modi2-vhf |
| Q65: svuota la media dopo il decode | Decodifica | s-modi2-vhf |
| F Tol Q65 | Decodifica | s-modi2-vhf |
| F Tol MSK144 | Decodifica | s-modi2-vhf |

#### VHF · EME › Contest (`contestvhf`)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Contest (VHF · EME) | Modi › VHF · EME › Contest | s-modi2-contestvhf |
| Tendina Contest (solo contest con MSK144 o Q65) | Impostazioni dei contest | s-modi2-contestvhf |
| Tutti i campi del modulo del contest (stessi della pagina FT8 · FT4) | Impostazioni dei contest | s-modi2-contestvhf (campo per campo in s-modi2-contest) |
| Frequenze del contest e pulsante Salva | Impostazioni dei contest | s-modi2-contestvhf |
| Pulsante Apri la pagina Contest | Impostazioni dei contest | s-modi2-contestvhf |
| Riquadro Verifica del Fox | non c'è in questo ramo | non applicabile: Fox/Hound è solo FT8 (si trova in Contest · Fox/Hound) |

#### WSPR · FST4W (`wspr`)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina WSPR · FST4W | Modi › WSPR · FST4W | s-modi2-wspr (e `wspr`, `fst4`) |
| Potenza del radiofaro (tendina dBm/W) | Radiofaro | s-modi2-wspr |
| Tx Pct | Radiofaro | s-modi2-wspr |
| Tipo 1 (pulsante acceso/spento) | Radiofaro | s-modi2-wspr |
| Invia gli spot a WSPRnet (casella «Attivo») | Spot su WSPRnet | s-modi2-wspr |
| Spia, «?» e «Ultimo invio» con la coda | Spot su WSPRnet | s-modi2-wspr |
| Pulsante Prova del servizio | Spot su WSPRnet | non applicabile: per WSPRnet la prova non esiste (uno spot di prova sarebbe pubblico); detto in s-modi2-wspr |

#### Voci esistenti di `guide.ts` trovate sbagliate nella stessa area

| Id | Cosa è sbagliato |
|---|---|
| `decstart` | Dice che «Avvio della decodifica FT8» sta in Sequenza, riquadro «Decodifica e attesa»: oggi sta in Modi › FT8 · FT4 › Decodifica (la Sequenza ha solo Full duplex, Wait features e Pounce in quel riquadro). Passi 1 e 2. |
| `wspr` | Dice di accendere «Invia gli spot a WSPRnet» in Log › Servizi online: la casella sta in Modi › WSPR · FST4W (la pagina Servizi online ha lasciato fuori WSPRnet di proposito). Primo passo. |


## Impostazioni › Modi: PSK · RTTY · CW, SSTV, Satelliti


Sezioni nel file `app/src/i18n/guide-testo.ts` (export `GUIDE_TESTO`). Percorso base: Impostazioni › Modi.

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Famiglia Modi: panoramica dei rami PSK · RTTY · CW, SSTV, Satelliti e di come si salvano | Modi | s-modes-family |
| Pagina Ricezione e trasmissione | Modi › PSK · RTTY · CW › Ricezione e trasmissione | s-digi-rxtx |
| Ricezione · Frequenza di ricezione (Hz) | Modi › PSK · RTTY · CW › Ricezione e trasmissione › Ricezione | s-digi-rxtx |
| Ricezione · AFC | idem | s-digi-rxtx |
| Ricezione · Snap | idem | s-digi-rxtx |
| Ricezione · Squelch (casella) | idem | s-digi-rxtx |
| Trasmissione · Lock (trasmetti su un'altra frequenza) | Modi › PSK · RTTY · CW › Ricezione e trasmissione › Trasmissione | s-digi-rxtx |
| Trasmissione · Frequenza di trasmissione (Hz) | idem | s-digi-rxtx |
| Trasmissione · Ascolto acceso all'avvio di Aurora | idem | s-digi-rxtx |
| Per modo · PSK31/PSK63: cursore soglia dello squelch (qualità) | Modi › PSK · RTTY · CW › Ricezione e trasmissione › Per modo | s-digi-rxtx |
| Per modo · PSK: Tutti i segnali della banda (PSK) | idem | s-digi-rxtx |
| Per modo · RTTY: Spostamento (Hz) e Velocità (baud) | idem | s-digi-rxtx |
| Per modo · RTTY: Inverti mark/space (Rev) e UOS | idem | s-digi-rxtx |
| Per modo · RTTY: Ricevitore (Automatico, Due filtri, Discriminatore) | idem | s-digi-rxtx |
| Per modo · RTTY: cursore soglia dello squelch (S/N) | idem | s-digi-rxtx |
| Per modo · CW: cursore velocità CW (WPM) | idem | s-digi-rxtx |
| Per modo · CW: Banda del filtro CW | idem | s-digi-rxtx |
| Per modo · CW: Decodifica SOM | idem | s-digi-rxtx |
| Per modo · CW: Tutti i segnali della banda (CW) | idem | s-digi-rxtx |
| Per modo · CW: Spot locali nel DX Cluster di Aurora e Spot locali solo di chi chiama CQ | idem | s-digi-rxtx |
| Per modo · CW: Convalida (Minima, Normale, Severa, Massima) | idem | s-digi-rxtx |
| Per modo · CW: cursore soglia dello squelch (S/N) | idem | s-digi-rxtx |
| Pagina Macro (F1…) | Modi › PSK · RTTY · CW › Macro (F1…) | s-digi-macros |
| Macro · elenco (tasto, etichetta, ambito), «Nessuna macro per questo modo» | idem | s-digi-macros |
| Macro · Modifica / Nuova macro | idem | s-digi-macros |
| Editor · Tasto (F1…F12, Ctrl/Shift e macro evento) | finestra Modifica macro, aperta dalla pagina Macro | s-digi-macros |
| Editor · Etichetta e Colore | idem | s-digi-macros |
| Editor · Vale per (Tutti i modi / Solo il modo) | idem | s-digi-macros |
| Editor · Testo (script) | idem | s-digi-macros |
| Editor · Comandi (tendina dei gruppi e elenco, «non ancora») | idem | s-digi-macros |
| Editor · Inserisci nel testo | idem | s-digi-macros |
| Editor · Pulisci, Annulla, OK | idem | s-digi-macros |
| Unisci le macro del modo / Ripristina le macro di Aurora (stanno nel menu File della pagina di lavoro, non qui) | pagina PSK · RTTY · CW, menu File | s-digi-macros (consiglio) |
| Pagina Scansione e memorie | Modi › PSK · RTTY · CW › Scansione e memorie | s-digi-scan |
| Proprietà della scansione · Sosta su ogni stazione (secondi) | idem › Proprietà della scansione | s-digi-scan |
| Proprietà della scansione · Fermati alla prima stazione trovata | idem | s-digi-scan |
| Proprietà della scansione · Scrivi l'ora / la frequenza di ogni stazione | idem | s-digi-scan |
| Proprietà della scansione · nota su come si avvia la scansione | idem | s-digi-scan |
| Memorie · elenco delle 20 memorie e testo «vuota» | idem › Memorie | s-digi-scan |
| Memorie · Richiama | idem | s-digi-scan |
| Memorie · Svuota | idem | s-digi-scan |
| Pagina Opzioni del DX Cluster | Modi › PSK · RTTY · CW › Opzioni del DX Cluster | s-digi-cluster |
| Quando scelgo uno spot · Doppio clic / Clic singolo | idem | s-digi-cluster |
| Quando scelgo uno spot · Il rotore gira verso la stazione | idem | s-digi-cluster |
| Spot · Gli spot spariscono dopo | idem | s-digi-cluster |
| Colonne da mostrare (DE, Freq, DX, Paese, Modo, Commenti, UTC) | idem | s-digi-cluster |
| Pagina SSTV Ricezione | Modi › SSTV › Ricezione | s-sstv-rx |
| Ricezione · Solo il modo (tendina) | idem | s-sstv-rx |
| Ricezione · Correzione sintonia in ricezione (Hz) | idem | s-sstv-rx |
| Ricezione · Ascolto acceso all'avvio di Aurora | idem | s-sstv-rx |
| Ricezione · Riconosci il modo anche senza VIS | idem | s-sstv-rx |
| Orologio della scheda audio · Correzione inclinazione (ppm) | idem | s-sstv-rx |
| Orologio della scheda audio · Calibra la scheda audio (pulsante e righe di stato) | idem | s-sstv-rx |
| Visore · Ingrandimento morbido | idem | s-sstv-rx |
| Riconoscimenti in fondo alla pagina | idem | s-sstv-rx |
| Pagina SSTV Trasmissione | Modi › SSTV › Trasmissione | s-sstv-tx |
| Trasmissione · Modo proposto in trasmissione | idem | s-sstv-tx |
| Trasmissione · Spostamento toni in trasmissione (Hz) | idem | s-sstv-tx |
| Trasmissione · FSK ID | idem | s-sstv-tx |
| Pagina SSTV Immagini | Modi › SSTV › Immagini | s-sstv-images |
| Salvataggio · Salva ogni immagine ricevuta | idem | s-sstv-images |
| Salvataggio · Tieni le immagini trasmesse | idem | s-sstv-images |
| Cartelle · Cartella delle immagini | idem | s-sstv-images |
| Cartelle · Cartella delle foto da trasmettere | idem | s-sstv-images |
| Avviso «un numero non va» e «Salva» in basso (bozza unica delle tre pagine SSTV) | Modi › SSTV | s-modes-family, s-sstv-rx, s-sstv-tx |
| Pagina Satelliti › Orbite | Modi › Satelliti › Orbite | s-sat-orbits |
| Server dei dati delle orbite (caselle dei server proposti) | idem | s-sat-orbits |
| Server aggiunti a mano, campo «Altro server» e pulsante Aggiungi | idem | s-sat-orbits |
| Avviso «Nessun server scelto» | idem | s-sat-orbits |
| Aggiornamento · Aggiorna da solo (tendina) | idem | s-sat-orbits |
| Aggiornamento · riga di stato e pulsante Aggiorna i dati delle orbite | idem | s-sat-orbits |
| Pagina Satelliti › Mappa | Modi › Satelliti › Mappa | s-sat-map |
| Mappa del mondo (tendina: Automatica, Mappa di Aurora, immagini della cartella maps) | idem | s-sat-map |
| Come aggiungere una mappa (nota sul formato) | idem | s-sat-map |
| Pagina Satelliti › Doppler e rotore | Modi › Satelliti › Doppler e rotore | s-sat-radio |
| Correggi il Doppler sulla radio | idem | s-sat-radio |
| Il rotore segue il satellite | idem | s-sat-radio |
| Prima del passaggio vai dove sorgerà | idem | s-sat-radio |
| Elevazione minima per Doppler e rotore | idem | s-sat-radio |
| Anteprima della cascata (WaterfallPreview) | non è in queste pagine: sta in Aspetto › Cascata | fuori area (pagina Cascata, di un altro collega) |
| Transponder, frequenza di discesa e correzione della salita (comandi sat.ctl.*) | non sono nelle Impostazioni: stanno nella pagina di lavoro Satelliti | fuori area (pagina di lavoro, di un altro collega) |

Niente resta senza spiegazione nell'area assegnata. Le due ultime righe sono volutamente fuori area.


## Impostazioni › Log, cluster e servizi


Le voci della Guida sono in `app/src/i18n/guide-log.ts` (`GUIDE_LOG`). Percorso base: Impostazioni › Log, cluster e servizi (famiglia `sw.fam.log`). Le voci con `ens.k.*` compaiono nella pagina solo se sono «vive» nel motore (vedi la nota in fondo).

#### Pagine

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Log e servizi (`sw.sec.log`) | Log, cluster e servizi › Log e servizi | s-log-logbooks |
| Pagina File ADIF (`sw.pg.files`) | Log, cluster e servizi › File ADIF | s-log-files |
| Pagina Programmi collegati (`sw.pg.reporting`) | Log, cluster e servizi › Programmi collegati | s-log-reporting |
| Pagina Servizi online (`tab.online`) | Log, cluster e servizi › Servizi online | s-log-online |
| Pagina DX Cluster (`tab.cluster`) | Log, cluster e servizi › DX Cluster | s-log-cluster |
| Pagina Desiderati (`tab.wishlist`) | Log, cluster e servizi › Desiderati | s-log-wishlist |
| Pagina DXCC (`tab.dxcc`) | Log, cluster e servizi › DXCC | s-log-dxcc |

#### Log e servizi

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Riquadro Database di Log4OM (`l4.title`): percorso, origine (`l4.srcAuto`, `l4.srcChosen`, `l4.srcEnv`), file mancante | Log e servizi, in cima | s-log-logbooks |
| Pulsante Scegli… (`l4.choose`) | Log e servizi › Database di Log4OM | s-log-logbooks |
| Casella del percorso, Usa (`l4.use`), Togli (`l4.remove`) | Log e servizi › Database di Log4OM | s-log-logbooks |
| Percorso scritto in aurora.env (casella e pulsanti assenti) | Log e servizi › Database di Log4OM | s-log-logbooks |
| Spia di ogni servizio (`ens.led.*`) e «?» del titolo | Log e servizi, riga di ogni servizio | s-log-logbooks |
| Interruttore del servizio (`ens.svc.on`) | Log e servizi, riga di ogni servizio | s-log-logbooks |
| Pulsante Prova (`ens.test`), esito e Ultimo invio (`ens.st.title`), coda dei QSO | Log e servizi, riga di ogni servizio | s-log-logbooks |
| Campi segreti: scrivi e premi Invio, Chiave salvata, Cambia, Togli (`ens.pw.*`) | Log e servizi, password e chiavi | s-log-logbooks |
| eQSL: Abilita l'invio (`EQSLSend`), Nome utente (`EQSLUser`), Password (`EQSLPasswd`), Nickname su QTH (`EQSLNick`) | Log e servizi › eQSL | s-log-logbooks |
| Wavelog/Cloudlog: Invia ogni QSO (`WavelogSend`), Indirizzo del sito (`WavelogUrl`), Chiave API (`WavelogKey`) | Log e servizi › Wavelog / Cloudlog | s-log-logbooks |
| Wavelog: Numero della stazione (`WavelogStation`): tendina, pulsante rilettura (`ens.wl.read`), casella di ripiego | Log e servizi › Wavelog / Cloudlog | s-log-logbooks |
| ClubLog: Invia ogni QSO (`ClublogSend`), Email (`ClublogEmail`), Password (`ClublogPasswd`), Nominativo del log (`ClublogCall`), Chiave API (`ClublogApiKey`) | Log e servizi › ClubLog | s-log-logbooks |
| QRZ.com logbook: Invia ogni QSO (`QRZLogSend`), Chiave del logbook (`QRZLogKey`), Nominativo registrato (`QRZLogCall`) | Log e servizi › QRZ.com | s-log-logbooks |
| HRDLOG.NET: Invia ogni QSO (`HrdlogSend`), Nominativo del log (`HrdlogCall`), Codice di caricamento (`HrdlogCode`) | Log e servizi › HRDLOG.NET | s-log-logbooks |
| LoTW: Firma e invia (`LoTWSend`), Percorso di tqsl.exe (`LoTWTqsl`), Posizione di stazione (`LoTWLocation`) | Log e servizi › LoTW | s-log-logbooks |
| Servizi con segreti assenti se Windows non protegge i segreti | Log e servizi | s-log-logbooks (Se non va) |
| Salvataggio delle voci con Salva (`sw.save`) | Finestra Impostazioni, in basso | s-log-logbooks |

#### File ADIF

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Esporta ADIF (`logio.export`) | File ADIF (e in cima a Programmi collegati) | s-log-files |
| Importa un file ADIF (`logio.import`), esito, limite di 256 MB, file vuoto, solo da rete di casa | File ADIF (e in cima a Programmi collegati) | s-log-files |
| Riquadro «Il tuo log: importa lo storico ADIF» (`cta.log.section`) | Programmi collegati, in cima | s-log-reporting, s-log-files |

#### Programmi collegati

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Server UDP primario: Server UDP (`UDPServer`), Numero porta (`UDPServerPort`) | Programmi collegati › Server UDP primario | s-log-reporting (esistente: logudp) |
| Accetta richieste UDP (`AcceptUDPRequests`), Risposte UDP accettate (`AcceptUDPReplyMessages`) | Programmi collegati › Server UDP primario | s-log-reporting |
| Abilita l'invio del file ADIF per il log (`EnableUDP1adifSending`), Prevenire messaggi con nominativi non confermati (`preventFalseUDPspots`) | Programmi collegati › Server UDP primario | s-log-reporting |
| Prova, spia e esito del server primario | Programmi collegati › Server UDP primario | s-log-reporting |
| Notifica dell'accettazione (`udpWindowToFront`), Finestra di ripristino (`udpWindowRestore`), Filtri di testo (`ApplyFiltersToUDPmessages`) | Non compaiono nella pagina (voci non ancora collegate nel motore) | s-log-reporting (nota: non compaiono) |
| Server UDP secondario: Abilita l'invio (`EnableUDP2adifBroadcast`), Server UDP2 (`UDP2Server`), Porta UDP (`UDP2ServerPort`), Prova | Programmi collegati › Invia file ADIF del QSO per il log | s-log-reporting (esistente: logudp, log4om-udp) |
| Rimanda i QSO persi quando il log si riapre (`UDP2Resend`) | Programmi collegati › Invia file ADIF del QSO per il log | s-log-reporting, s-log-resend-udp2 (esistente: log-resend) |
| Server TCP: Abilita l'invio (`EnableTCPConnection`), Server TCP (`TCPServer`), Porta TCP (`TCPServerPort`), Prova | Programmi collegati › Server TCP (N1MM) | s-log-reporting |
| Riquadro Informazioni: Paesi DXCC (`rep.cty`), Log dei QSO letto (`rep.log`), File ADIF (`rep.file`) | Programmi collegati, in fondo | s-log-reporting |

#### Integrazioni (Programmi collegati)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Titolo Integrazioni e «?» (`int.title`, `int.help`, `int.doubles`) | Programmi collegati › Integrazioni | s-log-integr |
| Pulsanti + Flusso WSJT-X, + BBLogger, + LogHX, + Gruppo multicast, + ADIF su UDP, + XML stile N1MM; limite di 16 | Programmi collegati › Integrazioni, in alto | s-log-integr, s-log-integr-dest |
| Togli (`int.remove`) e legenda del destinatario | Programmi collegati › Integrazioni › destinatario | s-log-integr |
| Nome (`int.name`), Indirizzo (`int.host`), Porta (`int.port`) | Programmi collegati › Integrazioni › destinatario | s-log-integr |
| Acceso (`int.enabled`) | Programmi collegati › Integrazioni › destinatario | s-log-integr |
| Flusso WSJT-X (`int.wsjtx`), Logged ADIF (12) (`int.loggedAdif`), ADIF su UDP (`int.adif`), XML N1MM (`int.n1mm`) | Programmi collegati › Integrazioni › destinatario | s-log-integr |
| Rimanda i QSO persi (`int.resend`) | Programmi collegati › Integrazioni › destinatario | s-log-integr, s-log-resend-udp2 (esistente: log-resend) |
| Multicast (`int.multicast`), TTL (`int.ttl`), Interfaccia (`int.iface`) | Programmi collegati › Integrazioni › destinatario | s-log-integr |
| Messaggi di errore dei valori e blocco del salvataggio (`int.err.*`) | Programmi collegati › Integrazioni | s-log-integr |
| Destinatario flusso WSJT-X (GridTracker, JTAlert, QLog, CQRLOG, MacLoggerDX): valori e cosa riceve | Integrazioni › + Flusso WSJT-X | s-log-integr-dest |
| Destinatario BBLogger | Integrazioni › + BBLogger | s-log-integr-dest (esistente: bblogger, bblogger-adif) |
| Destinatario LogHX | Integrazioni › + LogHX | s-log-integr-dest (esistente: loghx) |
| Destinatario Gruppo multicast | Integrazioni › + Gruppo multicast | s-log-integr-dest |
| Destinatario ADIF su UDP (Log4OM, WaveLogGate) | Integrazioni › + ADIF su UDP | s-log-integr-dest (esistente: log4om-udp) |
| Destinatario XML stile N1MM (HRD Logbook) | Integrazioni › + XML stile N1MM | s-log-integr-dest |
| Comandi che tornano ad Aurora (Reply, Halt Tx, testo libero, colori, Clear, Location, Configure) | Integrazioni, con Accetta richieste UDP | s-log-integr-dest, s-log-reporting |

#### Servizi online

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Chi mi ha ricevuto (PSK Reporter) (`onl.psk`) | Servizi online › PSK Reporter | s-log-online |
| Abilita lo spotting su PSK Reporter (`PSKReporter`), Prova | Servizi online › Spot su PSK Reporter | s-log-online |
| Abilita lo spotting su DXSummit (`AllowSpotsDXSummit`), Prova | Servizi online › Spot su DXSummit | s-log-online |
| Cerca nome e città su QRZ.com (`QRZLookup`), Utente (`QRZUser`), Password (`QRZPasswd`), Aggiungi nome e QTH ai QSO (`LookupToQso`), Prova | Servizi online › Ricerca QRZ.com | s-log-online (esistente: logsvc) |
| Cerca su HamQTH (`HamQTHLookup`), Utente (`HamQTHUser`), Password (`HamQTHPasswd`), Prova | Servizi online › Ricerca HamQTH | s-log-online |
| Propagazione (hamqsl.com) (`onl.solar`), Prova | Servizi online › Propagazione | s-log-online |
| Calendario delle DXpedition (`onl.dxped`), Storico delle finite (giorni) (`onl.dxpedDays`), Prova | Servizi online › DXpedition (NG3K) | s-log-online |
| Spot su WSPRnet (`WSPRnetUpload`) | Non è qui: Modi › WSPR · FST4W | s-log-online (rimando); esistente: wspr (percorso sbagliato) |
| Salvataggio: caselle Online subito, riquadri a righe con Salva | Servizi online | s-log-online |

#### DX Cluster

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Elenco Server (conteggio n/8), luce di ogni server, Usa (`cluster.server.on`), Togli (`cluster.server.remove`) | DX Cluster › Server | s-log-cluster |
| Verifica i server (`cluster.test`) | DX Cluster › Server | s-log-cluster |
| DX Cluster acceso (`cluster.enabled`) | DX Cluster › Impostazioni | s-log-cluster |
| Split DX (`cluster.splitDx.set`) | DX Cluster › Impostazioni | s-log-cluster (esistente: splitdx) |
| Nominativo per l'accesso (`cluster.callsign`, sola lettura) | DX Cluster › Impostazioni | s-log-cluster |
| Spot da tenere in memoria (`cluster.max`), Scarta gli spot dopo (`cluster.age`) | DX Cluster › Impostazioni | s-log-cluster |
| Comandi dopo l'accesso (`cluster.login`) | DX Cluster › Impostazioni | s-log-cluster |
| Server pubblici (`cluster.add.catalog`) e Aggiungi | DX Cluster › Aggiungi un server | s-log-cluster |
| Un tuo server telnet: Nome, Indirizzo, Porta, Aggiungi | DX Cluster › Aggiungi un server | s-log-cluster |
| Colore delle DXpedition: tavolozza, Colore a scelta, Ripristina | DX Cluster › Colore delle DXpedition | s-log-cluster |
| Stato dei collegamenti | DX Cluster › Stato dei collegamenti | s-log-cluster |
| Invia un comando al cluster: server, testo, Invia | DX Cluster › Invia un comando al cluster | s-log-cluster |
| Pannello DX Cluster vuoto | DX Cluster | s-log-cluster (Se non va); esistente: cluster |

#### Desiderati

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Nominativi (`wish.calls`), Prefissi (`wish.prefixes`) | Desiderati › Elenchi | s-log-wishlist |
| Paesi (`wish.countries`), Griglie (`wish.grids`) | Desiderati › Elenchi | s-log-wishlist |
| Avvisi accesi (`wish.enabled`), Dai messaggi decodificati (`wish.fromDecodes`), Dagli spot del cluster (`wish.fromCluster`) | Desiderati › Opzioni | s-log-wishlist |
| Solo su queste bande (`wish.bands`), Minuti prima di avvisare ancora (`wish.cooldown`) | Desiderati › Opzioni | s-log-wishlist |
| Avviso sul telefono (indirizzo ntfy) (`wish.notifyUrl`), Prova l'avviso (`wish.test`) | Desiderati › Avviso sul telefono | s-log-wishlist |
| Avvisi recenti | Desiderati › Avvisi recenti | s-log-wishlist |
| Barra di avviso in alto con «Vai alla frequenza» (`wish.tune`) | Pagina Stazione | s-log-wishlist (consiglio) |

#### DXCC

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Riquadro Come funziona (`dxs.guide.title`) | DXCC, in cima | s-log-dxcc |
| Anteprima (`dxs.preview`) | DXCC › Anteprima | s-log-dxcc |
| Mostra le didascalie, Mostra il totale, Mostra il campo «Prova un nominativo», Dimensione compatta | DXCC › Aspetto | s-log-dxcc |
| Righe (modi) da mostrare: FT8, FT4, DIG | DXCC › Righe (modi) da mostrare | s-log-dxcc |
| Bande da mostrare: 160m … 4m, U | DXCC › Bande da mostrare | s-log-dxcc |
| Ripristina i colori di fabbrica (`dxs.resetColors`) | DXCC › Colori | s-log-dxcc |
| Colori delle caselle per modo (FT8, FT4, DIG): lavorato, non confermato, mai lavorato, tasto ↺ | DXCC › Colori | s-log-dxcc |
| Colori delle sei etichette (NEW ONE, NEW BAND, NEW MODE, NEW GRID, GRID BAND, GRID MODE): sfondo e testo, accesa e spenta, tasto ↺ | DXCC › Colori | s-log-dxcc |

#### Componenti collegati fuori dalle sette pagine

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Rapporti degli errori: Chiedimi di mandare il rapporto, Manda in automatico, Invia statistiche, Vedi cosa mando, Vedi un rapporto automatico, Identificativo e Rigenera, Manda il pacchetto | Accesso e assistenza › Assistenza (`ReportsBox`) | s-log-rapporti (esistente: reports) |
| Finestra «Aurora si è chiusa per un errore» (`CrashPrompt`) | Si apre al riavvio dopo un crash | s-log-rapporti |
| Controllo della rete per il telefono (`NetworkChecklist`): Wi-Fi o cavo, firewall e Consenti Aurora, indirizzo del QR, consigli | Accesso e assistenza › Telefono, sotto il QR | s-log-netcheck |

#### Note sul «live» delle voci

- Le righe del motore (`ens.k.*`) si disegnano solo se la voce è «viva» (`support()` in `core/crates/remote-api/src/engine_settings.rs`). Le voci eQSL, Wavelog, ClubLog, QRZ, HRDLOG, le ricerche QRZ.com e HamQTH richiedono la protezione dei segreti di Windows; LoTW no.
- `udpWindowToFront`, `udpWindowRestore` e `ApplyFiltersToUDPmessages` non compaiono mai nella pagina nel programma attuale: la Guida lo dice in una riga.
- Con un motore diverso da quello proprio le pagine Log e servizi mostrano «Impostazioni non disponibili» e Programmi collegati un elenco informativo: non coperto dalla Guida perché non è l'uso normale.
- «Tieni i file salvati per N giorni»: non esiste nelle pagine di questa area (nessuna stringa né componente lo contiene); se è una voce dell'audio o dei backup spetta a chi ha quell'area.


## Impostazioni › Aspetto e avvisi


Famiglia `look` dell'albero delle Impostazioni (`app/src/lib/settingsTree.ts`). Testi in `app/src/i18n/guide-aspetto.ts` (`GUIDE_ASPETTO`). Percorso base di ogni riga: Impostazioni › Aspetto e avvisi › pagina.

Il pulsante «Spegni questi avvisi» (chiave `newdx.off`) e la regola «al massimo due schede» stanno nel commit `bfb626da`, non ancora in questo ramo: la Guida li descrive già; la chiave `newdx.off` esisterà dopo la fusione.

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
| --- | --- | --- |
| Pagina Aspetto | Aspetto e avvisi › Aspetto | s-aspetto |
| Lingua (menu con la bandiera) | Aspetto › riquadro Tema | s-aspetto |
| Tema: scuro, chiaro, automatico | Aspetto › riquadro Tema (e icona in basso a destra) | s-aspetto |
| Alto contrasto | Aspetto › riquadro Tema | s-aspetto |
| Carattere dell'interfaccia | Aspetto › riquadro Testo | s-aspetto |
| Carattere di dati e numeri | Aspetto › riquadro Testo | s-aspetto |
| Carattere del log | Aspetto › riquadro Testo | s-aspetto |
| Dimensione del log | Aspetto › riquadro Testo | s-aspetto |
| Dimensione del testo (menu e Ctrl + / Ctrl − / Ctrl 0) | Aspetto › riquadro Testo | s-aspetto |
| Colore del testo (del tema, campioni, colore libero, avviso di contrasto) | Aspetto › riquadro Testo | s-aspetto |
| Anteprima e Ripristina | Aspetto › riquadro Anteprima | s-aspetto |
| Come spegnere o cambiare finestre in evidenza, suoni, colori e messaggi | Aspetto e avvisi (varie pagine) | s-avvisi-spegni |
| Pagina Colori, tabella Evidenziazioni e suoni per tipo: nome del tipo | Colori › primo riquadro | s-colori |
| Casella Evidenzia (zona CQ, ITU, paese, griglia, prefisso, nominativo) | Colori › tabella | s-colori |
| per banda / per modo | Colori › tabella | s-colori |
| anche un suono | Colori › tabella | s-colori |
| Suono di avviso (Riproduci un suono di avviso) e per banda | Colori › tabella | s-colori |
| Già lavorati (colore) | Colori › tabella, ultima riga | s-colori |
| Barra / sottolinea (già lavorati) | Colori › riquadro Già lavorati | s-colori |
| Non mostrare | Colori › riquadro Già lavorati | s-colori |
| Inverti colore sfondo/testo | Colori › riquadro Già lavorati | s-colori |
| Evidenzia anche i messaggi tra altre stazioni | Colori › riquadro Già lavorati | s-colori |
| Paese / griglia lavorato ma non confermato = nuovo | Colori › riquadro Già lavorati | s-colori |
| Anteprima nella lista (nuova / non confermata) | Colori › riquadro Già lavorati | s-colori |
| Colori e anteprima: 17 tipi di messaggio (campione e selettore di colore) | Colori › riquadro Colori e anteprima | s-colori-tinte |
| Menu del suono accanto a ogni tipo | Colori › riquadro Colori e anteprima | s-colori-tinte |
| Ripristina i colori di Aurora | Colori › riquadro Colori e anteprima | s-colori-tinte |
| Mappa DX: quadrato lavorato e confermato | Colori › riquadro Mappa DX | s-colori-tinte |
| Chi usa LoTW: segna, colore, ultimo invio entro | Colori › riquadro Chi usa LoTW | s-colori-tinte |
| Sfondo diverso per il mio nominativo (ciano / giallo) | Colori › riquadro Aspetto dei messaggi | s-colori-aspetto |
| Evidenzia nominativo e locatore DX | Colori › riquadro Aspetto dei messaggi | s-colori-aspetto |
| Marcatori: altri messaggi, rosso, blu | Colori › riquadro Marcatori | s-colori-aspetto |
| Marcatore 73 / RR73 fra altre stazioni | Colori › riquadro Marcatori | s-colori-aspetto |
| Colori con un motore esterno (JTDX): tabella unica | Colori / Avvisi e suoni | s-colori (nel testo) |
| Pagina Avvisi e suoni, riquadro Avvisi e suoni: Avvisi sonori (interruttore e icona altoparlante) | Avvisi e suoni › primo riquadro | s-avvisi |
| Non suonare mentre trasmetti | Avvisi e suoni › primo riquadro | s-avvisi |
| Avviso sul mio nominativo / sul nominativo DX | Avvisi e suoni › primo riquadro | s-avvisi |
| Suono per il mio nominativo | Avvisi e suoni › primo riquadro | s-avvisi |
| Avviso in evidenza per un DXCC nuovo (e sulla banda) | Avvisi e suoni › primo riquadro | s-avvisi, s-avvisi-spegni |
| Pulsante «Spegni questi avvisi» sotto le schede del DXCC nuovo | Pagina Stazione, in alto a destra | s-avvisi-spegni, s-avvisi |
| Chiudi questo avviso (×) sulla scheda | Pagina Stazione, in alto a destra | s-avvisi-spegni |
| Dove compare l'avviso in evidenza (6 posizioni) | Avvisi e suoni › primo riquadro | s-avvisi |
| Suono dell'avviso in evidenza e suono QSO a log | Avvisi e suoni › primo riquadro | s-avvisi |
| Suono al primo messaggio decodificato | Avvisi e suoni › primo riquadro | s-avvisi |
| Suono che non parte / scheda che non compare | Avvisi e suoni | s-avvisi (se non va) |
| Riquadro Suoni: volume generale | Avvisi e suoni › riquadro Suoni | s-avvisi-suoni |
| Lingua della voce | Avvisi e suoni › riquadro Suoni | s-avvisi-suoni |
| Assegna suoni diversi automaticamente / Togli tutti i suoni / Ferma | Avvisi e suoni › riquadro Suoni | s-avvisi-suoni |
| Messaggio «Avvisi sonori è spento» | Avvisi e suoni › riquadro Suoni | s-avvisi-suoni |
| Tabella Altri avvisi (6 eventi, menu del suono, stato) | Avvisi e suoni › riquadro Suoni | s-avvisi-suoni |
| Menu del suono (Nessuno, voce, I miei file, aggiungi file, prova ▶, togli ×) | Avvisi e suoni, Colori | s-avvisi-suoni |
| Banco dei suoni da provare | Avvisi e suoni › riquadro Suoni | s-avvisi-suoni |
| Riquadro Messaggi toast: mostra riusciti / informazioni / errori | Avvisi e suoni › ultimo riquadro | s-avvisi-toast |
| Toast: secondi per tipo | Avvisi e suoni › ultimo riquadro | s-avvisi-toast |
| Toast: posizione e dimensione | Avvisi e suoni › ultimo riquadro | s-avvisi-toast |
| Toast: Prova i messaggi e Ripristina | Avvisi e suoni › ultimo riquadro | s-avvisi-toast |
| Pagina Cascata, Anteprima dal vivo (mostra / nascondi) | Cascata › primo riquadro | s-cascata |
| Canali per pixel, Inizio (Hz), Medie, Velocità della cascata | Cascata › riquadro Risoluzione | s-cascata |
| Tavolozza e barra dei colori | Cascata › riquadro Colori e livelli | s-cascata |
| Guadagno cascata, Zero cascata | Cascata › riquadro Colori e livelli | s-cascata |
| Scala per la mediana, Spianare il fondo | Cascata › riquadro Colori e livelli | s-cascata |
| Spettro: altezza, curva (corrente / cumulativa) | Cascata › riquadro Spettro | s-cascata-spettro |
| Spettro: guadagno e zero | Cascata › riquadro Spettro | s-cascata-spettro |
| Barre Rx e Tx, Riga di inizio periodo | Cascata › riquadro Barre e frequenza | s-cascata-spettro |
| Ora (timbro sulla cascata), Frequenza sotto il puntatore | Cascata › riquadro Barre e frequenza | s-cascata-spettro |
| Stile compatto / Stile veloce / Ripristina | Cascata › riquadro Barre e frequenza | s-cascata-spettro |
| Tavolozza personalizzata: punti, Togli, Aggiungi, Parti da quella scelta | Cascata › riquadro Tavolozza personalizzata | s-cascata-spettro |
| Pagina Finestre, riquadro Dove sta ogni elemento (12 elementi, menu del posto) | Finestre › primo riquadro | s-finestre |
| Posto «Nascosto» | Finestre › primo riquadro | s-finestre |
| Finestre a parte aperte: Mostra la finestra, Rimetti nella finestra principale | Finestre › secondo riquadro | s-finestre |
| Rimetti tutte le finestre a parte | Finestre › secondo riquadro | s-finestre |
| Ripristina la disposizione | Finestre › riquadro Finestre | s-finestre |
| Pagina Bande nella barra: una casella per banda (25 bande) | Bande nella barra | s-bande-barra |
| Tutte le bande (in realtà torna alle bande da 160 m a 2 m) | Bande nella barra | s-bande-barra |
| Testi di aiuto in fondo alle pagine (Colori/Avvisi: not2.help, not2.intro, not2.previewHelp, not2.alertHelp, snd.help; Cascata: wfs.note; Finestre: win.help; Bande: bands.help) | note in fondo alle pagine | manca: sono già testo a schermo tradotto; la Guida ne ripete il contenuto nelle sezioni di pagina, non le cita voce per voce |


## Impostazioni › Accesso e assistenza e Informazioni


Famiglia `sw.sec.access` (Accesso e assistenza) con le sue quattro pagine e la pagina Informazioni. Le sezioni stanno in `app/src/i18n/guide-accesso.ts` (`GUIDE_ACCESSO`). Percorso abbreviato: Impostazioni › Accesso e assistenza = «IAA».

Modalità «PC lento»: verificata nel codice (cartelle `app/src`, `core`, `desktop`) e nelle stringhe: non esiste nessuna casella, tendina o interruttore con quel nome o simili. Esiste solo l'avviso automatico `slow_pc` (barra di diagnosi in basso a destra, `DiagnosisBar`) con i pulsanti che cambiano la profondità di decodifica; è documentato in `s-acc-slowpc`.

#### Smartphone (IAA › Smartphone)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Smartphone | IAA › Smartphone | s-acc-phone |
| Indirizzo fisso in casa (nome aurora.local, sola lettura; messaggio «Nome fisso non attivo») | IAA › Smartphone › riquadro Indirizzo fisso in casa | s-acc-phone |
| Casella «Accesso senza codice dalla rete di casa» | IAA › Smartphone › riquadro Indirizzo fisso in casa | s-acc-phone |
| Pulsante «Mostra un codice di associazione» | IAA › Smartphone › riquadro Associa un telefono | s-acc-phone |
| Campo «Durata del codice (minuti)» (1-60, salvataggio automatico) | IAA › Smartphone › riquadro Associa un telefono | s-acc-phone |
| QR, codice, link, indirizzi in rete, minuti di validità | IAA › Smartphone › riquadro Associa un telefono (con il QR aperto) | s-acc-phone |
| QR degli altri indirizzi del PC (Wi-Fi, cavo, nome di rete, rete virtuale, altra rete) | IAA › Smartphone › riquadro Associa un telefono | s-acc-phone |
| Controllo della rete (riga Wi-Fi/cavo, riga firewall, riga indirizzo, attesa, consigli) | IAA › Smartphone › sotto il QR | s-acc-netcheck |
| Pulsante «Consenti Aurora» (regola del firewall di Windows) | IAA › Smartphone › controllo della rete | s-acc-netcheck |
| Elenco «Dispositivi associati» e pulsante «Revoca» | IAA › Smartphone › riquadro Dispositivi associati | s-acc-phone |
| Anteprima dal vivo telefono (390×844) e tablet (1180×820) | IAA › Smartphone › riquadro Anteprima dal vivo | s-acc-phone |
| Spiegazione già esistente dell'associazione (QR, link, accesso senza codice, dispositivi) | Guida › Telefono e tablet | phone (esistente in guide.ts) |
| Testo `acc.openWarn` e striscia `acc.banner`/`acc.bannerLink` («Accesso senza codice attivo…»/«Spegnilo») | nessuna: le stringhe esistono ma nel codice non sono usate da nessun componente | manca: non c'è nulla sullo schermo da spiegare |

#### Agente nel tray (IAA › Agente nel tray)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Agente nel tray | IAA › Agente nel tray | s-acc-agent |
| Casella «Agente attivo all'avvio di Windows» | IAA › Agente nel tray › riquadro Avvisi dell'agente nel tray | s-acc-agent |
| Tabella degli avvisi: colonne Avviso / Popup / Solo icona / Spento (una scelta per riga) | IAA › Agente nel tray › riquadro Avvisi dell'agente nel tray | s-acc-agent |
| Riga «Novità dal sito» | stessa tabella | s-acc-agent |
| Riga «Propagazione» | stessa tabella | s-acc-agent |
| Riga «DXpedition» | stessa tabella | s-acc-agent |
| Riga «Riepilogo del giorno» | stessa tabella | s-acc-agent |
| Campo «Resta aperto (secondi)» (3-60, di fabbrica 10) | IAA › Agente nel tray › riquadro Popup | s-acc-agent |
| Casella «Non disturbare» con orari «Dalle» e «Alle» (22:00 e 07:00 di fabbrica) | IAA › Agente nel tray › riquadro Popup | s-acc-agent |
| Pulsanti «Prova: aggiornamento» e «Prova: DXpedition» (Anteprima della cartolina) | IAA › Agente nel tray › riquadro Popup | s-acc-agent |
| Lingua dell'agente (cartoline e menu del tray) | segue la lingua di Aurora | updates (esistente in guide.ts, tip) |
| Nota in fondo alla pagina (`agent.intro`) | nessuna: il componente `Help` non disegna nulla, la nota non si vede | manca: non c'è nulla sullo schermo da spiegare (il contenuto è nel testo di s-acc-agent) |

#### Aggiornamenti (IAA › Aggiornamenti)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Aggiornamenti | IAA › Aggiornamenti | s-acc-updates |
| «Versione in uso» | IAA › Aggiornamenti › riquadro Stato | s-acc-updates |
| «Ultima versione trovata» (con stella e NEW) | IAA › Aggiornamenti › riquadro Stato | s-acc-updates |
| «Ultimo controllo» | IAA › Aggiornamenti › riquadro Stato | s-acc-updates |
| Riga di stato («Aurora è aggiornata», versione nuova con «Note e download», errore del controllo) | IAA › Aggiornamenti › riquadro Stato | s-acc-updates |
| Pulsante «Installa ora» (solo con versione nuova installabile) e barra di avanzamento | IAA › Aggiornamenti › riquadro Stato | s-acc-updates |
| Pulsante «Controlla ora» (apre sempre la locandina) | IAA › Aggiornamenti › riquadro Stato | s-acc-updates |
| Casella «Controlla le nuove versioni» | IAA › Aggiornamenti › riquadro Impostazioni | s-acc-updates |
| Campo «Ogni quante ore controllare» (1-168) | IAA › Aggiornamenti › riquadro Impostazioni | s-acc-updates |
| Repository, «Anche le versioni di prova», token di lettura | nessun campo visibile (restano di fabbrica; le stringhe esistono ma la scheda non le disegna) | s-acc-updates (ultima riga, detto che non si cambiano) |
| Nota «Gli aggiornamenti si salvano da soli» e note `upd.intro`/`tip.upd.token` | note in fondo: `Help` non disegna nulla, solo la prima si vede | s-acc-updates (testo introduttivo) |
| Icona/pannellino in testata («Installa ora», «Note e download», «Più tardi») | testata dell'app | s-acc-updbar |
| Barretta arancio in testata (scaricamento, «Controllo…», «Nuova»/«Installa», «Download non riuscito»/«Riprova») | testata dell'app | s-acc-updbar |
| Locandina della versione nuova («Installa e riavvia», «Più tardi», «Annulla», conto alla rovescia, primo avvio) | finestra principale; nelle Impostazioni solo dopo «Controlla ora» | s-acc-updbar |
| Locandina «Aurora è aggiornata» («Tutte le novità», «Chiudi») | dopo «Controlla ora» con versione aggiornata | s-acc-updbar |
| Spiegazione già esistente degli aggiornamenti | Guida › Primi passi › Aggiornamenti | updates (esistente in guide.ts) |

#### Assistenza (IAA › Assistenza)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Assistenza (ordine dei cinque riquadri) | IAA › Assistenza | s-acc-support |
| Pulsante «Salva una copia» (zip delle impostazioni e del log) | IAA › Assistenza › riquadro Copia delle impostazioni | s-acc-support (e backup, esistente) |
| Pulsante «Ripristina da una copia…» con conferma e riavvio | IAA › Assistenza › riquadro Copia delle impostazioni | s-acc-support (e backup, esistente) |
| Nota sulle password che non entrano nella copia | IAA › Assistenza › riquadro Copia delle impostazioni | s-acc-support |
| Pulsante «Prepara il pacchetto per l'assistenza» | IAA › Assistenza › riquadro Assistenza | s-acc-support (e support, esistente) |
| Pulsante «Apri la cartella» (dopo aver preparato il pacchetto) | IAA › Assistenza › riquadro Assistenza | s-acc-support |
| Campo «Email per l'assistenza» | IAA › Assistenza › riquadro Assistenza | s-acc-support |
| Pulsante «Manda per email» (apre Gmail nel browser) | IAA › Assistenza › riquadro Assistenza | s-acc-support |
| Avviso «Il pacchetto contiene… nessuna password» e messaggi d'errore | IAA › Assistenza › riquadro Assistenza | s-acc-support |
| Pulsante «Apri la cartella dei crash» (solo se il PC ha la cartella) | IAA › Assistenza › riquadro Rapporti dei crash | s-acc-support |
| Pulsante «Verifica adesso» | IAA › Assistenza › riquadro Verifica tutto | s-acc-support |
| Scheda Verifica tutto: righe Stazione, Radio, Audio, Orologio, Telefono, link «Sistema», pulsante «Chiudi» | si apre da «Verifica adesso» | s-acc-checkall |
| Casella «Chiedimi di mandare il rapporto dopo un crash» | IAA › Assistenza › riquadro Rapporti degli errori | s-acc-support (e reports, esistente) |
| Casella «Manda in automatico i rapporti degli errori (anonimi)» e riga della coda | IAA › Assistenza › riquadro Rapporti degli errori | s-acc-support (e reports, esistente) |
| Casella «Invia statistiche» | IAA › Assistenza › riquadro Rapporti degli errori | s-acc-support (e reports, esistente) |
| Pulsanti «Vedi cosa mando» e «Vedi un rapporto automatico» (finestra «Cosa parte, esattamente») | IAA › Assistenza › riquadro Rapporti degli errori | s-acc-support, s-acc-reports |
| «Identificativo anonimo di questa installazione» e pulsante «Rigenera» | IAA › Assistenza › riquadro Rapporti degli errori | s-acc-support |
| Pulsante «Manda il pacchetto a chi sviluppa Aurora» (solo se il servizio è configurato) e codice | IAA › Assistenza › riquadro Rapporti degli errori | s-acc-support |
| Striscia del primo avvio («Ho capito», «Impostazioni › Accesso e assistenza › Assistenza») | in cima all'app al primo avvio | s-acc-reports |
| Finestra dopo un crash («Manda il rapporto», «Vedi cosa mando», «No», «Chiudi») | alla riapertura dopo un crash | s-acc-reports |
| Privacy: cosa parte, cosa non parte, dove va, come spegnere tutto | riquadro Rapporti degli errori | s-acc-reports (e reports, esistente) |

#### Informazioni (Impostazioni › Informazioni, in fondo all'albero)

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagina Informazioni (logo, nome, descrizione, versione in uso) | Impostazioni › Informazioni | s-acc-about |
| Dedica dell'autore e riquadri Progettazione e sviluppo, Licenza, Codice derivato, Grazie a (sola lettura) | Impostazioni › Informazioni | s-acc-about |
| Ultime novità (tre) e link «Tutte le novità →» | Impostazioni › Informazioni | s-acc-about |
| Riquadro «Sostieni Aurora»: «Dona con PayPal» e «Lascia una stella su GitHub» | Impostazioni › Informazioni | s-acc-about |
| Collegamenti: sito, sorgenti, versioni pubblicate, segnala un problema, nominativo su QRZ.com | Impostazioni › Informazioni (in fondo) | s-acc-about |

#### Se il PC è lento

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Modalità «PC lento» o «leggera» | non esiste (nessuna casella o tendina) | s-acc-slowpc (spiega che non c'è e cosa c'è al suo posto) |
| Scheda gialla «Il PC fatica a decodificare in tempo» con «Passa a Normale»/«Passa a Veloce», «Impostazione» e chiusura | barra di diagnosi in basso a destra | s-acc-slowpc |
| Profondità di decodifica (Veloce/Normale/Profonda) | Impostazioni › Modi › FT8 · FT4 › Decodifica | s-acc-slowpc (e requirements, esistente) |
| «Modo SWL» come alleggerimento | pagina delle opzioni del motore | s-acc-slowpc |
| Requisiti del computer (4 core, 8 GB) | Guida › Primi passi | requirements (esistente in guide.ts) |


## Pagine e pannelli fuori dalle Impostazioni


Le voci `p-…` sono in `app/src/i18n/guide-pagine.ts` (export `GUIDE_PAGINE`). Gli altri id sono voci già presenti in `app/src/i18n/guide.ts` che spiegano davvero la funzione.

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Marchio AURORA (torna alla Stazione) | Barra in alto, a sinistra | p-nav |
| Pillola «FT8» (gruppo HF: FT8, FT4, JT9, JT65, FST4, FST4W, WSPR) | Barra in alto › pillole dei modi | p-nav |
| Pillola «VHF» (JT65, JT4, Q65, MSK144) | Barra in alto › pillole dei modi | p-nav |
| Pillole bloccate durante la trasmissione | Barra in alto › pillole dei modi | p-nav |
| Pillola PSK · RTTY · SSTV (con CW) e spostamento della radio sul piano IARU | Barra in alto › pillole dei modi | p-nav |
| Pillola Satelliti | Barra in alto › pillole dei modi | p-nav, p-sat |
| Pagina Fox (solo gruppo FT8) | Barra in alto › pagine del modo | p-navpages, p-fox |
| Pagina Contest | Barra in alto › pagine del modo | p-navpages, p-contest |
| Pagina QMAP (solo gruppo VHF e solo se accesa nelle Impostazioni) | Barra in alto › pagine del modo | p-navpages, p-qmap |
| Pagina Statistiche | Barra in alto › pagine del modo | p-navpages, p-stats |
| Pagine Log, Mappa DX, Propagazione, Rotore | Barra in alto › pagine di tutti i modi | p-navpages |
| Ingranaggio con il menu Impostazioni, Guida, Novità | Barra in alto, a destra | p-navpages |
| Impostazioni aperte sulla pagina del modo in uso; assenti dal telefono | Ingranaggio › Impostazioni | p-navpages |
| Ultima pagina ricordata all'avvio | Barra in alto | p-navpages |
| Icona della versione nuova e barretta di scaricamento | Barra in alto, a destra | p-updbar, updates |
| Pulsante ⤓ «Rimetti tutte le finestre a parte» | Barra in alto, a destra (solo con finestre staccate) | p-detach, p-navpages |
| Pulsante grande del modo con la tendina dei modi del gruppo | Stazione › sopra la cascata, a sinistra | p-modefreq |
| Sottomodi JT4 A-G, JT65 A-C, periodo e sottomodo di Q65, periodo di FST4 e FST4W | Stazione › accanto al modo | p-modefreq, jt65sub, jt4, q65, fst4 |
| Comandi Q65 (Lor/Gau, Deriva, EME, Cont, Media, Auto ⟲, ⟲ Cancella media) | Stazione › accanto al modo (con Q65) | p-modefreq, q65 |
| Comandi JT65 (AP, Prof., ⟲ Cancella media) | Stazione › accanto al modo (con JT65) | p-modefreq, jt65sub |
| Comandi MSK144 (periodo, Sh) | Stazione › accanto al modo (con MSK144) | p-modefreq, msk144 |
| Comandi WSPR e FST4W (potenza, Tx Pct, Tipo 1) | Stazione › accanto al modo (con WSPR o FST4W) | p-modefreq, wspr, fst4 |
| Pulsante delle frequenze: banda, MHz e tendina con frequenza principale, alternative e regioni | Stazione › sopra la cascata | p-modefreq, freqlist |
| Tasto destro sulla frequenza: salto del Band Hopping | Stazione › pulsante delle frequenze | p-modefreq, bandhop |
| Bande ammesse dal contest nella tendina | Stazione e Contest › pulsante delle frequenze | p-modefreq, p-contest |
| Interruttori di ricezione: Monitora, Hound, SWL, JT65+JT9, AGCc, Filtro, a7/a8, Multithread | Stazione › barra a segmenti sotto il modo | p-rxrow, agc, ft8plus, jt |
| Pulsanti Ferma e Decodifica | Stazione › barra a segmenti | p-rxrow |
| Pulsante Auto (Automazioni: reale, prova a vuoto, spente) | Stazione › barra a segmenti | p-rxrow, automation |
| Pillola di collegamento (Connessione… / Scollegato) | Stazione › striscia di stato | p-status |
| Frequenza in MHz (solo con display radio nascosto) | Stazione › striscia di stato | p-status |
| Avviso giallo dell'orologio (clic: corregge o apre le impostazioni) | Stazione › striscia di stato | p-status, clock |
| Display della radio, pulsanti PRE/ATT/AGC/NB/NR/NOTCH/ATU/VFO/RIT/XIT, S-meter | Stazione › in alto a destra | display, radiobuttons, meters (voci esistenti) |
| Decodifiche, barra e contatore del periodo | Barra in basso, a sinistra | p-footer |
| Profilo radio e watchdog (WD) | Barra in basso, a sinistra | p-footer |
| Icona dell'altoparlante e cursore del volume (avvisi sonori) | Barra in basso, a destra | p-footer, p-langtheme |
| Icona del tema (scuro, chiaro, automatico) | Barra in basso, a destra; telecomando in alto | p-footer, p-langtheme |
| Ora UTC, nome e versione di Aurora | Barra in basso, a destra | p-footer |
| Lingua (menu con bandiere, tastiera) | Impostazioni › Aspetto e avvisi › Aspetto › Tema | p-langtheme |
| Suoni per evento, voce, file propri | Impostazioni › Aspetto e avvisi › Avvisi e suoni | p-langtheme, sounds, voice |
| Messaggi a comparsa (toast) | In alto o in basso secondo le Impostazioni | toasts |
| Schede di diagnosi in basso a destra (orologio, nominativo, audio, PTT, PC lento, OmniRig, porta occupata, contest) | Finestra principale, in basso a destra | p-diag, pttfail, clock |
| Pulsanti delle schede: Sistema, Come risolvere, Passa a Normale o Veloce, Impostazione, × | Schede di diagnosi | p-diag |
| Scheda lampeggiante «DXCC nuovo» e «DXCC nuovo sulla banda» (doppio clic, ×, come spegnerla, dove compare) | In alto a destra (angolo scelto) | p-newdx |
| Barra della stazione desiderata (Vai alla frequenza, Chiudi) | In cima alla pagina | p-newdx |
| Striscia sugli errori anonimi (Ho capito) | Finestra principale, al primo avvio | p-notices, reports |
| Finestra «Manda il rapporto» dopo un crash | Finestra principale, al riavvio | p-notices, reports |
| Cartoline dell'autore con suono | In basso a destra | p-notices |
| Locandina dell'aggiornamento (Installa e riavvia, Più tardi, Novità) | Finestra principale | p-updbar, install |
| Blocco Bande aperte (condizioni, le più lontane che ti ascoltano) | Stazione › blocco in alto a sinistra | p-bandsopen, board |
| Pannello Attività di banda: pulsante Filtro, pillole delle novità, eccezioni | Stazione › Attività di banda › barra del titolo | p-actband, listview |
| Pulsanti Pulisci e Nuovi ↑ / ↓ | Stazione › Attività di banda e Finestra RX › barra del titolo | p-actband, p-rxwin, listview |
| Pulsante ⧉ di ogni pannello | Stazione › barra del titolo | p-detach |
| Colonne, etichette colorate e segni delle righe dei messaggi | Stazione › Attività di banda e Finestra RX | p-rows, list |
| Doppio clic, clic, tasto destro sulla riga e sul nominativo; menu QRZ.com e coda | Stazione › elenchi dei messaggi | p-rows, answer, queue |
| Riquadro del nominativo al passaggio del mouse | Stazione › messaggi, cluster | p-rows, logsvc |
| Filtri per nominativo, prefisso, già collegati, evidenziazione, bypass | Impostazioni › Modi › FT8 · FT4 › Messaggi e filtri | filters, p-actband |
| Età dei messaggi, nuovi in fondo | Elenchi dei messaggi | listview |
| Finestra RX (titolo, ±50 Hz, colonne spostabili, pulsante RX) | Stazione › sopra il pannello Log | p-rxwin, listview |
| Barra sopra la cascata: Bins/Pixel, Start, Tavolozza, Gain, Zero, N Avg, Spec, Flatten, Ora, tratto di banda | Stazione › Cascata | p-wfbar, wfset |
| Barre Rx e Tx, frecce, linee bianche, frequenza sotto il puntatore, zone del Filtro | Stazione › Cascata | p-wfmarks, spectrum |
| Etichette degli spot del cluster sulla cascata e «+N» | Stazione › Cascata | p-wfmarks |
| Righello con tacche e parentesi Tx e Rx | Stazione › sotto la cascata | p-wfmarks |
| Altezza dello spettro (maniglia) | Stazione › Cascata | p-wfbar, wfset |
| Etichetta DX, coda con numeri, prossimo della coda | Stazione › Trasmissione | p-txpanel, queue, queuejump |
| Riquadro RX / STOP TX (clic e doppio clic) | Stazione › Trasmissione | p-txpanel, stop |
| Etichetta Hound / Super Hound e verifica OTP del Fox | Stazione › Trasmissione | p-txpanel, hound-otp |
| Avvisi di sicurezza della trasmissione | Stazione › Trasmissione | p-txpanel, txsafety |
| I sei messaggi Tx1-Tx6: scelta con un clic, testo modificabile | Stazione › Trasmissione | p-txpanel |
| AutoSeq, 1 QSO, Risp. a tutti, Risp B4, Bypassa, Salta Tx1, RRR, Periodo Tx | Stazione › Trasmissione › colonna Sequenza del QSO | p-qsosw, queue |
| Abilita Tx, AutoTX, Wait and Call, Wait and Pounce | Stazione › Trasmissione › colonna Trasmissione | p-qsosw, wait |
| Livello Rx (indicatore, cursore, AUTO) | Stazione › Trasmissione, in fondo | p-levels, p-diag |
| Livello Pwr | Stazione › Trasmissione, in fondo | p-levels, pwr |
| Campi Tx, Rx, Rapporto e frecce ▲ ▼ | Stazione › Comandi | p-cmds, commands |
| Hold Tx Freq, Tune, Tx/Rx Split, Log QSO, Canc DX | Stazione › Comandi | p-cmds, commands |
| Chiamare CQ (sesto messaggio, Abilita Tx) | Stazione › Trasmissione | p-cqhow, cq |
| Caselle dei CQ (doppio clic, tasto destro, scorrimento, 15 minuti) | Stazione › CQ e novità | p-cqdxped, cqpanel |
| Elenco delle DXpedition, colori, scheda di dettaglio, Vai su kHz | Stazione › CQ e novità | p-cqdxped, cqpanel |
| DX Cluster: filtri Banda, Modo, Spotter, Solo nuovi, ricerca | Stazione › DX Cluster | p-cluster, cluster |
| DX Cluster: pulsante Spot DX | Stazione › DX Cluster | p-cluster, commands |
| DX Cluster: colonne spostabili, colori delle righe, doppio clic, tasto destro | Stazione › DX Cluster | p-cluster, splitdx, clustermode |
| Split DX: etichetta «UP 5» e pulsante di spegnimento | Stazione › DX Cluster | splitdx |
| Pannello Log: linguette Log, Mappa, RX; totale QSO; colonne; QSO A LOG | Stazione › Log | p-logpanel, logpanel |
| Pannello Log: doppio clic sul QSO | Stazione › Log | p-logpanel, logedit |
| Messaggio «Il log è vuoto» con pulsante | Stazione › Log | p-logpanel |
| Scheda DXCC: etichette NEW ONE, NEW BAND, NEW MODE, NEW GRID, GRID BAND, GRID MODE | Stazione › Log, sotto il log; Statistiche | p-dxcc |
| Scheda DXCC: tabella righe FT8, FT4, DIG per bande, colori, totale | Stazione › Log; Statistiche | p-dxcc |
| Scheda DXCC: campo Prova, pulsanti Paese e Totali | Stazione › Log; Statistiche | p-dxcc |
| Mappa nel pannello Log | Stazione › Log › linguetta Mappa | logmap, logmappanel, p-dxmapbar |
| Disposizione a blocchi, divisori, trascinamento | Stazione | board |
| Finestre a parte: ⧉, finestra nuova, dashboard, accanto a, rimetti | Stazione e altre pagine | p-detach |
| Scheda del rotore sopra la Stazione | Stazione › in alto | p-rotor |
| Finestra «Registrare il QSO con…» (campi, Registra, Scarta, Invio) | Finestra principale a fine QSO | p-logprompt |
| Esc, doppio Esc e altri tasti rapidi | Pagine in generale | p-keys, stop |
| Pagina Log: filtri Nominativo, Dal, Al, Banda, Modo, Togli i filtri | Barra in alto › Log | p-logpage, log |
| Pagina Log: selezione (casella, Seleziona tutti, Nessuno), conteggio | Log | p-logpage, logpanel |
| Pagina Log: colonne, Modifica, Mostra altri | Log | p-logpage |
| Pagina Log: Salva ADIF, Scarica ADIF, Salva Cabrillo | Log | p-logpage, logpanel |
| Pagina Log: Elimina selezionati, Svuota il log | Log | p-logpage |
| Pagina Log: Aggiungi QSO (modulo) | Log | p-logpage |
| Pagina Log: ← Stazione | Log | p-logpage |
| Finestra Correggi o cancella il QSO (ogni campo, Salva, Annulla, Elimina) | Log, pannello Log | p-logedit, logedit |
| Rimandare i QSO persi ai log esterni | Impostazioni › Log, cluster e servizi › Programmi collegati; Log | p-extlog, log-resend |
| Storico dei messaggi (filtri, Svuota lo storico, colonne, Carica altri) | Solo da indirizzo #/storico (non ha voce nei menu) | p-history |
| Mappa DX: stile, luminosità, linea grigia, linee, propagazione, Chi chiama chi, Solo chi mi sente, Nominativi, quadrati, banda, modo | Barra in alto › Mappa DX | p-dxmapbar, dxmap, dxmapoptions |
| Mappa DX: ingranaggio ⚙ (durata spot, colore, notte, crepuscoli, Luna, linee animate, puntini, colori, spessore, velocità, sbiadisce in) | Mappa DX | p-dxmapbar |
| Mappa DX: zoom, trascinamento, clic su punti e stazioni, legenda | Mappa DX | p-dxmapbar, dxmap |
| Propagazione: Aggiorna ora, Sole e bande, indici, condizioni per banda | Barra in alto › Propagazione | p-prop, propagation |
| Propagazione: Chi mi ha ricevuto (mappa, tabella, stazioni per banda) | Propagazione | p-prop, propagation |
| Statistiche: riquadri dei numeri, Per banda | Barra in alto › Statistiche | p-stats, stats |
| Statistiche: scheda DXCC, elenco dei paesi con filtro e tendina | Statistiche | p-stats, p-dxcc |
| Statistiche: Paesi che ti mancano | Statistiche | p-stats |
| Statistiche: Log interno (Esporta ADIF, Importa un file ADIF) | Statistiche | p-stats, p-logpage |
| Novità: pulsanti Nuovo, Modificato, Corretto; righe apribili | Ingranaggio › Novità | p-news, news |
| Guida: campo di ricerca, indice, indirizzo #/guida/voce | Ingranaggio › Guida | p-guide, settingssearch |
| Fox: stato, contatori, Posti, CQ verso, UltraFOX, Più CQ, Coda automatica, Azzera | Barra in alto › Fox | p-fox, fox-page, ultrafox |
| Fox: Fox acceso, Abilita Tx, STOP, comandi di ricezione, Split | Fox › barra dei comandi | p-fox |
| Fox: riquadro OTP (seme, ogni, Applica, Togli) | Fox | p-fox, hound-otp |
| Fox: Chi chiama (filtro, ordina, SNR, Solo nuovi, Prime 5 in coda, In coda, In cima) | Fox | p-fox |
| Fox: QSO in corso (Chiudi), In onda adesso, In coda (Togli), Fox log (ADIF, Cabrillo, Club Log) | Fox | p-fox |
| Fox: cascata e Messaggi ricevuti | Fox | p-fox |
| Contest: interruttore, scelta, Come funziona, barra del punteggio, tessera categoria | Barra in alto › Contest | p-contest, contest |
| Contest: Calendario con Preparami | Contest | p-contest, p-contest3 |
| Contest: schede In gara, Allenamento, Analisi | Contest | p-contest, p-contest3 |
| Contest: RX/TX e tendina delle frequenze | Contest › riga delle schede | p-contest, contestkeys |
| Contest: Decodifiche (ordine, Solo CQ, doppio clic, stella) | Contest › In gara | p-contest2, contest |
| Contest: QSO in corso, AutoSeq contest, Abilita Tx, Auto CQ, RR73 di nuovo, Dopo | Contest › In gara | p-contest2, contestkeys, contesttx |
| Contest: Consigli e ritmo, Regole, Cabrillo / EDI, Impostazioni del contest, Fine gara | Contest › In gara | p-contest2, p-contest3 |
| Contest: Consiglio di banda e modo | Contest › In gara | p-contest3 |
| Contest: Mappa dei moltiplicatori | Contest › In gara | p-contest3 |
| Contest: Log del contest (correzione, menu del tasto destro, Nuovo log) | Contest › In gara | p-contest3, contest |
| Contest: Allenamento (simulatore) e Analisi dopo la gara | Contest | p-contest3 |
| Contest: tasti F1–F4, Esc | Contest | contestkeys, p-keys |
| QMAP: Sorgente I/Q, indirizzo TCI, campionamento, file, Applica, Decodifica il file | Barra in alto › QMAP | p-qmap, qmap |
| QMAP: Centro, Periodo, Sottomodi, Evanescenza, +30 s, Media, Cancella media | QMAP | p-qmap, qmap |
| QMAP: spettro, Luna, tabella dei messaggi, pulsante QSO, Sked | QMAP | p-qmap, qmap |
| Rotore: rosa, ago, stato, Gradi, Vai, Ferma, Corto, Lungo, passi, Park | Barra in alto › Rotore | p-rotor, rotator |
| Rotore: Mostra / Togli dalla Stazione, ripiegare, nascondere, spostare, ⧉ | Rotore e scheda sulla Stazione | p-rotor |
| Satelliti: mappa, Aggiorna i dati delle orbite, impostazioni, Mappa del mondo, Aggiungi una mappa | Pillola Satelliti | p-sat, satellites |
| Satelliti: Passaggi (Ore, Elevazione minima), lista con ricerca e spunta, dati e orbita, informazioni e frequenze | Satelliti | p-sat, satellites |
| Satelliti: Doppler e rotore (transponder, discesa, correzione della salita, elevazione minima, interruttori) | Satelliti | p-sat, satellites |
| Pannello Segnali: elenco, colonne, clic, doppio clic con storico, Sintonizza, Nel QSO | PSK · RTTY · CW, sopra la cascata | p-signals, digicw |
| Pannello Segnali: Convalida, Pulisci, maniglia di altezza, spot locali | PSK · RTTY · CW | p-signals, digicw |
| Telecomando: barra (RX/TX, banda, modo, watt con − +, secondi, ⟳, tema) | Telefono e finestra stretta | p-phone, remote |
| Telecomando: Attività di banda, DX Cluster, Log, messaggi Tx, cascata | Telefono | p-phone, remote |
| Telecomando: CQ, Abilita Tx, STOP (due tocchi) | Telefono, barra in fondo | p-phone, stop, remote |
| Pagina Associa questo dispositivo (codice, nome, messaggi di errore, QR) | Telefono | p-pair, phone |
| Il telefono mostra dati vecchi | Telefono | phonestale |


## Pagine PSK · RTTY · CW e SSTV, telefono e agente nel tray


Le sezioni nuove stanno in `app/src/i18n/guide-testuali.ts` (`GUIDE_TESTUALI`, id con prefisso `t-`). Quando una voce già presente in `guide.ts` o scritta da un altro collega spiega davvero la cosa, la tabella cita quella (id senza prefisso `t-`). Le pagine delle Impostazioni (PSK · RTTY · CW, SSTV, Smartphone, Agente nel tray) sono di altri colleghi: qui compaiono solo dove la pagina di lavoro le apre. Percorso abbreviato: PRC = pagina «PSK · RTTY · CW» (barra in alto «PSK · RTTY · SSTV», percorso `#/testo`); SSTV = pagina SSTV (`#/sstv`).

#### Pagina PSK · RTTY · CW: struttura

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Ingresso nella pagina (la radio va sulla banda del piano IARU, l'ascolto parte da solo e si spegne uscendo) | Barra in alto › PSK · RTTY · SSTV | t-digi-page |
| Barra dei menu (apertura con clic, passaggio fra menu, chiusura con Esc o clic fuori) | PRC › in alto | t-digi-page |
| Selettore di banda e frequenze IARU | PRC › a destra della barra delle macro | t-digi-page |
| Scritta TX rossa (lampeggiante) | PRC › accanto alla barra delle macro | t-digi-page |
| Finestra di ricezione e finestra di trasmissione, divisore trascinabile | PRC › centro | t-digi-page, t-digi-rxtx |
| Pannello Segnali | PRC › sopra la cascata | t-digi-signals |
| Cascata (altezza trascinabile) | PRC › in basso a sinistra | t-digi-wf |
| Colonna di destra: linguette DX Cluster e Log, bordo sinistro trascinabile | PRC › a destra | t-digi-page, t-digi-cluster, t-digi-log |
| Scheda del rotore | PRC › colonna di destra | t-digi-page (voce di menu in t-digi-menu-view); la scheda in sé: `rotator` |
| Barra di stato | PRC › in fondo | t-digi-status |
| Avviso «Non riesco a leggere lo stato della stazione» | PRC › in cima | t-digi-page |
| Dove restano le scelte (Vista, larghezze, altezze, memorie, modelli) | PRC | t-digi-page |
| Panoramica generale della pagina (voce di prima) | PRC | digi, digipage (parziali: vedi rapporto) |

#### Pagina PSK · RTTY · CW: menu

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| File › Unisci le macro del modo a quelle di base | PRC › File | t-digi-menu-fe |
| File › Ripristina le macro di Aurora (e avviso «Macro di Aurora ripristinate») | PRC › File | t-digi-menu-fe |
| Modifica › Pulisci la finestra di ricezione | PRC › Modifica | t-digi-menu-fe |
| Modifica › Pulisci la finestra di trasmissione | PRC › Modifica | t-digi-menu-fe |
| Modifica › Copia il testo ricevuto | PRC › Modifica | t-digi-menu-fe |
| Modo › PSK31, PSK63, RTTY, CW (scelta del modo) | PRC › Modo | t-digi-menu-mode |
| Modo › SSTV | PRC › Modo | t-digi-menu-mode |
| Modo › Spostamento (Hz) e Velocità (baud) (solo RTTY) | PRC › Modo | t-digi-menu-mode |
| Modo › Inverti mark/space (Rev) e UOS (solo RTTY) | PRC › Modo | t-digi-menu-mode |
| Modo › Banda del filtro CW (solo CW) | PRC › Modo | t-digi-menu-mode, t-digi-cw |
| Modo › TX alla velocità ricevuta (solo CW) | PRC › Modo | t-digi-menu-mode, digicw |
| Modo › modi «in arrivo» (QPSK31, FSK31, MFSK, Olivia, Contestia, MT63, Throb, Hellschreiber, Packet, Pactor, Amtor) | PRC › Modo | t-digi-menu-mode |
| Modo › Impostazioni del modo… (apre le Impostazioni) | PRC › Modo | t-digi-menu-mode |
| Modo › Soglia dello squelch… (apre la finestra Impostazioni del modo) | PRC › Modo | t-digi-menu-mode, t-digi-modeset |
| Opzioni › Ascolto | PRC › Opzioni | t-digi-menu-opt |
| Opzioni › AFC, Lock, Snap, Squelch | PRC › Opzioni | t-digi-menu-opt |
| Opzioni › Soglia dello squelch… | PRC › Opzioni | t-digi-menu-opt, t-digi-modeset |
| Opzioni › Tutti i segnali della banda (CW / PSK) | PRC › Opzioni (solo CW e PSK) | t-digi-menu-opt, t-digi-signals |
| Opzioni › Spot locali nel DX Cluster di Aurora e «solo di chi chiama CQ» | PRC › Opzioni (solo CW e PSK) | t-digi-menu-opt, digicw |
| Opzioni › RX e TX (scorciatoia Pausa) | PRC › Opzioni | t-digi-menu-opt, t-digi-keys |
| Opzioni › Cerca a sinistra / Cerca a destra | PRC › Opzioni | t-digi-menu-opt |
| Opzioni › Scansione › Avvia, Ferma, Proprietà della scansione… | PRC › Opzioni | t-digi-menu-opt, digiscan |
| Opzioni › Memoria › Salva in / Richiama da › Memoria 1-20 | PRC › Opzioni | t-digi-menu-opt, digiscan |
| Vista › Barra dei comandi, Barra estesa (4 righe) | PRC › Vista | t-digi-menu-view |
| Vista › Log, Scheda del rotore, Cascata, Pannello Segnali, Nominativi sulla cascata, Barra di stato | PRC › Vista | t-digi-menu-view |
| Vista › Testo trasmesso anche nella ricezione, Sottolinea il testo trasmesso | PRC › Vista | t-digi-menu-view |
| Vista › Solo maiuscole, Zero barrato, TX lampeggiante | PRC › Vista | t-digi-menu-view |
| Vista › Linee sottili sulla cascata, Cascata larga (0-5 kHz), Anti-click sulla cascata | PRC › Vista | t-digi-menu-view, digicw |
| Vista › Rapporti di base (599) | PRC › Vista | t-digi-menu-view |
| Configura › Macro per questo modo… / Macro di base (F1…) | PRC › Configura | t-digi-menu-fe |
| Configura › Impostazioni del modo… | PRC › Configura | t-digi-menu-fe |
| Configura › Dati personali… | PRC › Configura | t-digi-menu-fe |
| Aiuto › Guida di questa pagina (apre tutta la Guida) | PRC › Aiuto | t-digi-menu-fe |

#### Pagina PSK · RTTY · CW: macro, finestre e tasti

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Barra delle macro: 4 righe da 12 tasti, righe Ctrl, Shift, Ctrl+Shift | PRC › in alto a sinistra | t-digi-macros |
| Esecuzione con clic o con il tasto della tastiera; tasto «+» del tastierino (Gray+) | PRC › barra delle macro | t-digi-macros |
| Ingranaggio (barra estesa o compatta) | PRC › barra delle macro | t-digi-macros, t-digi-menu-view |
| Bordo destro trascinabile (doppio clic: misura di partenza) | PRC › barra delle macro | t-digi-macros |
| Macro di fabbrica (F1-F12, Ctrl-F1…, F10-F12 per modo) | PRC › barra delle macro | t-digi-macros |
| Aspetto del tasto del CQ automatico acceso | PRC › barra delle macro | t-digi-macros |
| Clic destro su un tasto: finestra Modifica macro | PRC › barra delle macro | t-digi-macros, t-digi-macroedit |
| Modifica macro › Tasto (48 tasti e macro speciali) | Finestra Modifica macro | t-digi-macroedit |
| Modifica macro › Etichetta, Colore | Finestra Modifica macro | t-digi-macroedit |
| Modifica macro › Vale per (tutti i modi / solo il modo in uso) | Finestra Modifica macro | t-digi-macroedit |
| Modifica macro › Testo (script) e segnaposto più usati | Finestra Modifica macro | t-digi-macroedit |
| Modifica macro › Comandi (gruppi), elenco, «non ancora» | Finestra Modifica macro | t-digi-macroedit |
| Modifica macro › Inserisci nel testo, Pulisci, Annulla, OK | Finestra Modifica macro | t-digi-macroedit |
| Comandi delle macro che non fanno ancora niente (TUNE, BOOKMARK, MACROS) | Finestra Modifica macro | t-digi-macroedit |
| Finestra di ricezione: testo, caratteri incerti, scorrimento | PRC › Ricezione | t-digi-rxtx |
| Finestra di ricezione: doppio clic su un nominativo | PRC › Ricezione | t-digi-rxtx, digi |
| Finestra di ricezione: clic destro (Nominativo, Nome, QTH, Locatore, Rapporto ricevuto, Copia, Pulisci) | PRC › Ricezione | t-digi-rxtx |
| Finestra di trasmissione: scrittura dal vivo, caratteri rossi e in coda | PRC › Trasmissione | t-digi-rxtx, digilive |
| Finestra di trasmissione: Ctrl+Invio | PRC › Trasmissione | t-digi-rxtx, t-digi-keys |
| Elenco del testo già trasmesso (sopra la casella) | PRC › Trasmissione | t-digi-rxtx |
| Avvisi sotto la casella (trasmissione non pronta, occupata, disabilitata) | PRC › Trasmissione | t-digi-rxtx |
| RX/TX, Pausa | PRC › barra di stato, menu Opzioni | t-digi-keys, digilive |
| Esc (una volta) | PRC | t-digi-keys |
| Doppio Esc (spegne il CQ automatico) | PRC | t-digi-keys |
| Effetto generale di Esc in Aurora («Esc: ferma la trasmissione») | tutta Aurora | t-digi-keys |
| CQ automatico (pausa di 8 secondi, macro AutoCqText o F2) | PRC | t-digi-keys |
| Blocco dei tasti con una finestra aperta | PRC | t-digi-keys |
| Frequenza audio in Hz (casella nella barra di stato) | PRC › barra di stato | t-digi-status, t-digi-wf |
| Scansione: sosta, avvio, arresto, passaggio al segnale successivo | PRC › menu Opzioni | t-digi-menu-opt, digiscan |
| Cerca a sinistra/destra (Seek) | PRC › menu Opzioni, macro | t-digi-menu-opt |
| Memorie 1-20 | PRC › menu Opzioni | t-digi-menu-opt, digiscan |

#### Pagina PSK · RTTY · CW: cascata, barra di stato, finestre di impostazione

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Scala della cascata (frequenze vere o note) | PRC › cascata | t-digi-wf |
| Clic e trascinamento sulla cascata, Snap, limite 200-3500 Hz | PRC › cascata | t-digi-wf |
| Bandierina, rombo e coppia di triangoli (ricezione e trasmissione) | PRC › cascata | t-digi-wf |
| Lock, clic che sposta la trasmissione, Ctrl+clic | PRC › cascata | t-digi-wf, t-digi-menu-opt |
| Linee sottili, cascata larga o stretta | PRC › cascata | t-digi-wf, t-digi-menu-view |
| Nominativi di tutta la banda sulla cascata | PRC › cascata | t-digi-wf, digicw |
| Bordo alto della cascata (altezza 50-400) | PRC › cascata | t-digi-wf |
| Barra di stato › RX/TX | PRC › barra di stato | t-digi-status |
| Barra di stato › Sq, AFC, Lock, Snap | PRC › barra di stato | t-digi-status |
| Barra di stato › spostamento, velocità e Rev (solo RTTY) | PRC › barra di stato | t-digi-status, rttyrx |
| Barra di stato › banda del filtro e TX = RX (solo CW) | PRC › barra di stato | t-digi-status, digicw |
| Barra di stato › dati del modo (clic) | PRC › barra di stato | t-digi-status |
| Barra di stato › misure del segnale (S/N, IMD, WPM) | PRC › barra di stato | t-digi-status, digi |
| Barra di stato › nome del modo (in RTTY apre Impostazioni RTTY) | PRC › barra di stato | t-digi-status, t-digi-modeset, rttyrx |
| Barra di stato › clic destro (menu: squelch, AFC, Lock, Snap, soglia, UTC, TX lampeggiante) | PRC › barra di stato | t-digi-status, rttyrx |
| Finestra Impostazioni del modo › Ricezione (frequenza, AFC, Snap, Squelch, voci per modo) | PRC › Impostazioni del modo | t-digi-modeset |
| Finestra Impostazioni del modo › soglie dello squelch (PSK, CW, RTTY) | PRC › Impostazioni del modo | t-digi-modeset, t-digi-cw |
| Finestra Impostazioni del modo › Trasmissione (Lock, frequenza Tx, WPM, spostamento, velocità, Ascolto acceso all'avvio) | PRC › Impostazioni del modo | t-digi-modeset |
| Finestra Impostazioni del modo › Dati personali (nome, QTH) | PRC › Impostazioni del modo | t-digi-modeset |
| Finestra Impostazioni RTTY › profili Set 1-4, caselle, soglia, frequenze, ricevitore, spostamento, velocità, set di caratteri | PRC › Impostazioni RTTY | t-digi-modeset, rttyrx |
| Finestra Impostazioni RTTY › Valori di fabbrica, OK, Annulla | PRC › Impostazioni RTTY | t-digi-modeset |

#### Pagina PSK · RTTY · CW: segnali di banda, CW, cluster, log

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pannello Segnali: intestazione, conteggi, colonne, nominativo con bandiera | PRC › Segnali | t-digi-signals, digicw |
| Pannello Segnali: clic e doppio clic su una riga | PRC › Segnali | t-digi-signals |
| Pannello Segnali › Convalida (4 livelli) | PRC › Segnali | t-digi-signals, digicw |
| Pannello Segnali › Pulisci | PRC › Segnali | t-digi-signals |
| Pannello Segnali › bordo alto trascinabile | PRC › Segnali | t-digi-signals |
| Storico del segnale (Sintonizza, Nel QSO, Chiudi) | PRC › Segnali › doppio clic | t-digi-signals |
| CW: velocità in WPM (trasmissione e ricezione) | PRC › barra di stato, Impostazioni del modo, macro | t-digi-cw |
| CW: tono (frequenza di ricezione) | PRC › cascata, barra di stato | t-digi-cw |
| CW: banda del filtro | PRC › menu Modo, barra di stato | t-digi-cw, digicw |
| CW: decodifica SOM | PRC › Impostazioni del modo | t-digi-cw, digicw |
| CW: tasto telegrafico (non esiste un ingresso: solo tastiera) | PRC | t-digi-cw |
| CW: segni di procedura | PRC › testo e macro | t-digi-cw, digicw |
| DX Cluster › elenco e colonne | PRC › colonna di destra › DX Cluster | t-digi-cluster |
| DX Cluster › clic su uno spot, «UP N» (split) | PRC › colonna di destra › DX Cluster | t-digi-cluster |
| DX Cluster › Spot DX | PRC › colonna di destra › DX Cluster | t-digi-cluster, t-digi-spot |
| Finestra Invia spot (nominativo, DX de, frequenza, info, Invia, Annulla) | PRC › DX Cluster › Spot DX | t-digi-spot |
| DX Cluster › rotella (Opzioni del DX Cluster) | PRC › colonna di destra › DX Cluster | t-digi-cluster, t-digi-clustopt |
| Opzioni del DX Cluster › clic o doppio clic, rotore | PRC › Opzioni del DX Cluster | t-digi-clustopt |
| Opzioni del DX Cluster › scadenza degli spot, colonne | PRC › Opzioni del DX Cluster | t-digi-clustopt |
| Log › riga del QSO in corso (numero, modo, frequenza, data, UTC, nominativo, nome, QTH, paese, locatore, RST) | PRC › colonna di destra › Log | t-digi-log |
| Log › Salva il QSO a log, Azzera i dati del QSO | PRC › Log › in basso | t-digi-log |
| Log › Cerca nel log | PRC › Log › in alto | t-digi-log |
| Log › Colonne del log (matita), Adatta le colonne, Larghezze di partenza | PRC › Log › in alto | t-digi-log |
| Log › colonne spostabili e allargabili, clic destro sui titoli, bordo sinistro | PRC › Log | t-digi-log |
| Log › elenco dei QSO (ultimi 500, 300 mostrati) e «Nessun QSO nel log» | PRC › Log | t-digi-log |
| Pagine delle Impostazioni PSK · RTTY · CW (Ricezione e trasmissione, Macro, Scansione e memorie, Opzioni del DX Cluster) | Impostazioni › Modi › PSK · RTTY · CW | altro collega: s-digi-rxtx, s-digi-macros, s-digi-scan, s-digi-cluster |

#### Pagina SSTV

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Panoramica e ingresso nella pagina | Barra in alto › PSK · RTTY · SSTV | t-sstv-page, sstv |
| Tendina del modo in alto (PSK31, PSK63, RTTY, CW, SSTV) | SSTV › intestazione | t-sstv-page |
| Sigla di stato (In trasmissione, Ricezione in corso, Ascolto spento) | SSTV › intestazione | t-sstv-page |
| Selettore della banda e delle frequenze SSTV | SSTV › intestazione | t-sstv-page |
| Schede Ricevi e Componi e trasmetti | SSTV › intestazione | t-sstv-page |
| Rotella (apre le Impostazioni SSTV) e pulsante della finestra a parte | SSTV › intestazione | t-sstv-page |
| Schermo dell'immagine in arrivo, riga luminosa, messaggi di attesa | SSTV › Ricevi | t-sstv-rx |
| Originale / Pulita e scorrevoli Ordine e Levigatezza | SSTV › Ricevi | t-sstv-rx, sstv |
| Avvisi «Immagine salvata nella cronologia» e «Segnale perso» | SSTV › Ricevi | t-sstv-rx |
| Spettro con le righe 1200, 1500 e 2300 Hz e la riga rossa | SSTV › Ricevi › sotto lo schermo | t-sstv-rx, sstv |
| Modo riconosciuto e come (VIS, ritmo delle righe, a mano) | SSTV › Ricevi › a destra | t-sstv-rx |
| Nominativo (FSK ID), Riga n di N, Sincronismo, Livello, Sintonia | SSTV › Ricevi › a destra | t-sstv-rx |
| Pulsante TX | SSTV › Ricevi | t-sstv-rx, sstv |
| Avvia ascolto / Ferma ascolto | SSTV › Ricevi | t-sstv-rx, sstv |
| Scarta immagine | SSTV › Ricevi | t-sstv-rx |
| Tendina del modo e Ricevi ora | SSTV › Ricevi | t-sstv-rx, sstv |
| Riquadro Log QSO (nominativo, rapporti, nome, QTH, locatore, note, Metti a log, Pulisci) | SSTV › Ricevi › a destra | t-sstv-log |
| Ricerca del nominativo (paese, Dati da…, Non trovato) | SSTV › Log QSO | t-sstv-log, sstv |
| Cronologie Ricevute e Trasmesse (miniature, conteggio, frecce, rotella) | SSTV › colonna delle cronologie | t-sstv-hist |
| Seleziona, Tutte, Cancella (n), Fine | SSTV › cronologie | t-sstv-hist |
| Immagine grande: precedente e successiva, Originale/Pulita, Ingrandimento morbido | SSTV › cronologie › clic su una miniatura | t-sstv-hist |
| Immagine grande: Rispondi, Usa come sfondo, Salva, Cancella, Chiudi | SSTV › cronologie › clic su una miniatura | t-sstv-hist |
| Compositore › Trasmetti (F1) e Ferma (Esc), stato della trasmissione | SSTV › Componi e trasmetti › in alto | t-sstv-compose, sstv |
| Compositore › Modo di trasmissione e Corrispondente | SSTV › Componi e trasmetti › in alto | t-sstv-compose |
| Compositore › Annulla, Ripeti, Nuova | SSTV › Componi e trasmetti › in alto | t-sstv-compose |
| Compositore › Livelli (testo, rettangolo, ellisse, cornice, barre; su, giù, nascondi, duplica, elimina) | SSTV › Componi e trasmetti › a sinistra | t-sstv-compose |
| Compositore › Modelli (CQ, Rapporto, 73 / QSL, Salva come modello, I miei modelli) | SSTV › Componi e trasmetti › a sinistra | t-sstv-compose |
| Compositore › anteprima: sposta, ridimensiona, ruota, inquadra, zoom, trascina e incolla una foto | SSTV › Componi e trasmetti › al centro | t-sstv-compose, sstv |
| Compositore › barra e linea di avanzamento della trasmissione | SSTV › Componi e trasmetti | t-sstv-compose |
| Compositore › Foto di sfondo (scelta, cartella delle foto, adatta, zoom, sposta, ritocchi, colore) | SSTV › Componi e trasmetti › a destra | t-sstv-compose |
| Compositore › proprietà di un testo (testo e segnaposto, carattere, dimensione, stile, riempimento, contorno, ombra, riquadro, rotazione) | SSTV › Componi e trasmetti › a destra | t-sstv-compose |
| Compositore › proprietà di una forma | SSTV › Componi e trasmetti › a destra | t-sstv-compose |
| Tasti rapidi F1, Esc, Ctrl+Z, Ctrl+Maiusc+Z, Ctrl+Y, Canc, Ctrl+V, frecce | SSTV | t-sstv-keys |
| Doppio Esc e Pausa in SSTV (non esistono) | SSTV | t-sstv-keys |
| Pagine delle Impostazioni SSTV (Ricezione, Trasmissione, Immagini) | Impostazioni › Modi › SSTV | altro collega: s-sstv-rx, s-sstv-tx, s-sstv-images |

#### Telefono

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Associazione in generale | Telefono › pagina di associazione | phone, t-ph-pair |
| Pagina «Associa questo dispositivo»: apertura da QR o link, associazione automatica | Telefono | t-ph-pair |
| Campo Codice di associazione (8 cifre) | Telefono › Associa | t-ph-pair |
| Campo Nome del dispositivo | Telefono › Associa | t-ph-pair |
| Pulsante Associa questo dispositivo e messaggi di errore (cifre, scaduto, nome, troppi tentativi) | Telefono › Associa | t-ph-pair |
| Rientro da solo con l'accesso senza codice; messaggio «Non riesco a contattare Aurora», Riprova, cambio di indirizzo | Telefono | t-ph-pair |
| Il telefono non si aggiorna | Telefono | phonestale, t-ph-remote |
| Generare il codice, QR, indirizzi alternativi, durata, dispositivi associati, revoca, anteprime | Impostazioni › Accesso e assistenza › Smartphone | altro collega: s-acc-phone (controllo rete: s-acc-netcheck) |
| Telecomando: quando compare (meno di 768 punti; tablet in orizzontale) e cosa non si può fare | Telefono › Stazione | t-ph-remote, remote |
| Telecomando › frequenza, RX/TX, banda, modo (con conferma durante la trasmissione) | Telefono › barra in alto | t-ph-remote |
| Telecomando › potenza in watt, − e + | Telefono › barra in alto | t-ph-remote |
| Telecomando › secondi del periodo e barra di avanzamento | Telefono › barra in alto | t-ph-remote |
| Telecomando › pulsante di aggiornamento della pagina | Telefono › barra in alto | t-ph-remote, phonestale |
| Telecomando › tema chiaro/scuro | Telefono › barra in alto | t-ph-remote |
| Telecomando › Attività di banda (tocco per rispondere) | Telefono › Stazione | t-ph-lists, remote |
| Telecomando › DX Cluster (5 spot, tocco per sintonizzare) | Telefono › Stazione | t-ph-lists, remote |
| Telecomando › Log · ultimi 3 | Telefono › Stazione | t-ph-lists, remote |
| Telecomando › riquadro dei messaggi Tx (titolo, riga del DX, sei messaggi) | Telefono › Stazione | t-ph-tx, remote |
| Telecomando › Tx/Rx Split | Telefono › riquadro Tx | t-ph-tx |
| Telecomando › cascata | Telefono › Stazione | t-ph-tx |
| Telecomando › CQ, Abilita Tx / Tx acceso | Telefono › barra in basso | t-ph-tx, remote |
| Telecomando › STOP (doppio tocco libera anche il DX) | Telefono › barra in basso | t-ph-tx, remote |
| Telecomando su tablet in orizzontale (due colonne) | Tablet | t-ph-remote |

#### Agente nel tray

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Che cos'è, quando parte, un solo agente per utente | Windows › accanto all'orologio | t-ag-what |
| Icona e suggerimento sull'icona (anche i titoli degli avvisi «solo icona») | Windows › accanto all'orologio | t-ag-what |
| Clic sinistro sull'icona (apre Aurora) | Icona | t-ag-what |
| Menu del tasto destro › Apri Aurora | Icona › menu | t-ag-what |
| Menu del tasto destro › Cerca aggiornamenti (cartoline: versione nuova, aggiornata, controllo non riuscito) | Icona › menu | t-ag-what, t-ag-kinds |
| Menu del tasto destro › Esci dall'agente | Icona › menu | t-ag-what |
| Lingua dei testi dell'agente | Icona e cartoline | t-ag-what, updates |
| Cartolina: aspetto, posizione, suono, temi, durata, ×, Chiudi, pulsante principale | Angolo in basso a destra dello schermo | t-ag-card |
| Cartolina dell'aggiornamento: Aggiorna ora, avanzamento (Scarico › Controllo › Installo › Riavvio), Riprova e motivi del rifiuto | Angolo in basso a destra dello schermo | t-ag-card |
| Avviso Aggiornamento (sempre acceso) | Cartolina | t-ag-kinds |
| Messaggi dell'autore (auguri, avvisi) | Cartolina | t-ag-kinds |
| Avviso Propagazione (tempesta geomagnetica, 12 e 10 metri) | Cartolina o icona | t-ag-kinds |
| Avviso DXpedition (partono domani) | Cartolina o icona | t-ag-kinds |
| Avviso Riepilogo del giorno | Cartolina o icona | t-ag-kinds |
| Quando gli avvisi non compaiono (Aurora aperta, Non disturbare) | Agente | t-ag-kinds, t-ag-what |
| Dove si regola (Agente attivo, tabella degli avvisi, durata, Non disturbare, Anteprima) | Impostazioni › Accesso e assistenza › Agente nel tray | altro collega: s-acc-agent |
| Registro dell'agente `%TEMP%\aurora-agente.log` | File | t-ag-what |
| Menu del tasto destro nelle caselle di testo (Annulla, Ripeti, Taglia, Copia, Incolla, Elimina, Seleziona tutto, Cancella tutto) | Programma Windows › caselle di testo | t-ag-ctx |

#### Non coperto, con il motivo

| Funzione/voce | Dove sta (percorso) | Voce della Guida (id) |
|---|---|---|
| Pagine Satelliti e Rotore | Barra in alto › Satelliti, Rotore | manca: sono assegnate al collega «pagine», non a quest'area |
| Variabili d'ambiente `AURORA_AGENTE` e `AURORA_AGENTE_PROVA` | Avvio del programma | manca: sono per chi sviluppa o prova, non voci dell'interfaccia |
| Finestra principale di Windows: posizione e misure ricordate, controllo di WebView2, registri dei guasti, finestra «Salva con nome» del log, apertura dei link nel browser, chiusura di un Aurora vecchio rimasto acceso | Programma Windows (`desktop/`) | manca: non sono pagine né controlli della cartella assegnata (agente e cartoline); da assegnare a chi scrive «Programma Windows» e «Problemi comuni» |
| Pulsanti «Avvia ascolto», «Trasmetti» e «STOP» della pagina PSK · RTTY · CW e campi «Corrispondente», «Il mio nome», «Il mio QTH» sopra la casella | PRC | non esistono più nella pagina (l'ascolto parte da solo; Ctrl+Invio e RX/TX trasmettono; i dati del QSO stanno nella riga del Log): la voce `digi` di `guide.ts` li cita ancora |


## Riepilogo

Righe della tabella: 957; righe «manca»: 9. La voce «Tieni i file salvati per N giorni» non esiste in questo ramo: quando arriva si aggiunge qui e in Audio/File.
