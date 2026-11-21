# Ripresa del lavoro (ultimo aggiornamento: 2026-11-21)

Questo file dice a chi riprende il lavoro, in una sessione nuova, **a che punto e' Aurora** e **cosa fare per primo**. Va aggiornato a ogni passaggio di sessione.

- Panic `tao` re-entrant VERIFICATO (2026-11-20) sul pacchetto F014CB6C (FTDX-101MP, installazione 0.11.297): contiene solo due panic vecchi, di 0.11.179 (2026-11-07) e 0.11.212 (2026-11-09), stesso backtrace (SendMessageW dentro un gestore di tao, `runner.rs:246`). Nessuno dalla 0.11.224 in poi: gia' superato; resta la mitigazione della 0.11.308. Il pacchetto 19044C72 (FT-891, 0.11.300) e' un altro caso: «WebView2 error 0x80070578, handle della finestra non valido» all'avvio (setup hook), non e' un re-entrant.

## Giro del 2026-11-20 sera (sola lettura, nessun difetto nuovo)
- Pacchetto `3FCA9234` (0.11.324, 2026-11-21 13:37 UTC, FT8, radio via Hamlib su COM4): «programma bloccato (runtime)» di 41 s con la finestra viva. Nello stesso minuto l'ingresso audio USB si e' fermato per 3 s due volte, le connessioni di rete sono cadute (errori 10053/10054 e poi 11001 «host sconosciuto») e la riproduzione e' durata 9,8 s invece di 7,7 s: e' un arresto di tutto il sistema (rete, USB, audio), non un difetto di Aurora. Nessuna correzione.
- Pacchetti letti: `7CF56D20`, `0FC727FE`, `5E66C6CC`, `BB6E5ECC` (tutti dell'installazione IC-820H di Vladimir, #83: i `split non spento` ripetuti sono di versioni <= 0.11.308, dalla 0.11.309 non ce ne sono; con 0.11.320 la lettura delle bande dei VFO funziona e dice Main = VHF, Sub = UHF) e `B93DEDB3` (0.11.304, TCI, ExpertSDR: quattro «blocchi del runtime» di 95-725 s tra le 08:26 e le 08:34 UTC con il PSK Reporter che si vede chiudere il collegamento subito dopo; la finestra batteva, nessuna ripetizione sulla 0.11.319 dello stesso PC; causa non dimostrata, forse sospensione del PC o rete: non toccato).
- Errori Cloudflare: solo rumore noto (rigctld che non risponde, avvisi dell'installatore, decodifica lenta su Pentium E5400, audio non trovato). Issue: nessun commento nuovo.

- Dopo la 0.11.322, da fare: statistiche dell'uso reale dell'app (segnale ogni 40 minuti con campo «uso», rotta `/v1/utilizzo` del Worker `aurora-rapporti` con segreto, pagina «Applicazione» nel portale Analytics: la parte del portale e' nella sessione «Aurora-portal» con il contratto JSON; serve aggiornare l'informativa privacy) e la cartina WEFAX storta del proprietario (un altro ricevitore la riceve dritta; ricampionatore escluso con prova, restano la misura della riga in fasatura e il `slant_ppm`); #83 aspetta le prove con `rigctl` di Vladimir.

- Dopo la 0.11.323, da fare (compiti in `dist-release/brief-wefax-1.md` e `brief-wefax-2.md`, fuori dal repository): IOC automatico dal tono d'inizio, inclinazione ricordata tra le cartine, contatore dei blocchi audio persi, AFC con due limiti, correzione dell'inclinazione sulla cartina in corso con misura a due clic, riga della soglia SQL, decodifica da file WAV. Dati d'uso: Worker `aurora-rapporti` con `/v1/battito` e `/v1/utilizzo` pubblicato il 2026-11-21 (migrazione 0005 eseguita), pagina «Applicazione» nel portale Analytics, segnale ogni 40 minuti dalla 0.11.323: i numeri compaiono man mano che si aggiorna.

## In preparazione (su main, NON pubblicata)
- IC-820H (#83), trasmissione con trame CI-V dirette (`rig-control/src/ic820h_civ.rs`, `HamlibRig::set_uplink_via_civ`, casella `uplink_civ_ic820h` dei satelliti, spenta): trame mandate con `+w` (risposta estesa di rigctld, finisce con `RPRT`; con `w` semplice non c'e' `RPRT`), eco `FE FE 42 E0` scartata, si leggono solo le trame `FE FE E0 42` (FB/FA/03). Prova senza scrivere, poi scrittura con controllo della banda prima di ogni `05`; fallita = mai scritture fino alla riconnessione. Esito in `radio.json` -> `ic820h_civ` (stato, frequenze, passi con risposte grezze) e nella pagina dei satelliti. Provato SOLO con la finta IC-820H del finto rigctld: non si sa se rigctld 4.7.2 rimanda anche la risposta della radio o solo l'eco (in quel caso la prova fallisce con «nessuna risposta» e il pacchetto lo mostra). Da chiedere a Vladimir: Aurora 0.11.327+, SAT acceso, casella accesa, un passaggio, poi il pacchetto.
- Elenco dei contest, linguette per modo: `app/src/lib/contestModes.ts` (modi dalle Note, poi dal nome; l'elenco del core non porta i modi strutturati, `TextContestRef` ha solo id, nome, note, date), linguette in `ContestListDialog.tsx`, stile `.ctest-tabs` in `digi-contest.css`; testi `ctest.tab*` in sei lingue piu' nl e ru, frase nella Guida. Provato solo con vitest, non a schermo.
- IC-820H (#83), CORREZIONE (2026-11-21): i file rigctl di Vladimir con `-m 3032` mostravano «model: Dummy» e i valori del finto rigctl (145000000 ecc.): il comando `rigctl -m 3032 ...` era stato digitato DENTRO il prompt di rigctl avviato senza argomenti, quindi tutti gli esperimenti con `07 D0`/`07 D1`/`07 B0` hanno parlato a una radio finta (lo conferma lui: i LED RX-TX dell'adattatore non si accendono). Le conclusioni delle tre righe precedenti sull'esito degli esperimenti NON valgono; restano vere solo la tabella CI-V del manuale (`07 D0`/`D1`/`B0` documentati) e il `-9` di `V Sub`, che viene da Hamlib. Gli ho chiesto di ripetere l'esperimento dal prompt dei comandi con la riga di avvio completa e di controllare i LED con `f`.
- IC-820H (#83), manuale CI-V dell'IC-821H letto dalla scansione (pagina «Remote jack (CI-V) information», tabella dei comandi, fonte primaria): `05` scrive la frequenza (solo scrittura, sulla banda selezionata), `06` modo, `07`: `00` VFO A, `01` VFO B, `A0` A=B, `B0` MAIN/SUB (scambio), `D0` «Main band access», `D1` «Sub band access»; `0F`: `00`/`01` split off/on, `10`/`11`/`12` simplex, duplex -, duplex +. Quindi `07 D0`/`07 D1` sono documentati per la famiglia e il `-9` di `V Sub` viene da Hamlib (il modello 3032 espone solo VFOA/VFOB/MEM). L'esperimento di Vladimir senza SAT ha mostrato che dopo `07 D1` la lettura `03` rilegge ancora la frequenza della banda principale: resta da provare `07 D1` + `03` + `05` + `07 D0` con SAT ACCESO, leggendo anche l'ACK (`FB`/`FA`). Se la selezione funziona, il ripiego per il 3032 e' la trama grezza (`w`) `07 D1`, `05 <frequenza BCD>`, rilettura `03` di conferma, `07 D0`, mai scrivendo senza la conferma. Non provato in onda.
- PSK, finestra di ricezione contro lista «Segnali» (prova `tests/psk_finestra_e_lista.rs`, tabelle con `--ignored`): tre cause. (1) squelch spento di base (`squelch_on: false`) = circa 30% di spazzatura nelle pause e nel rumore; ora in PSK spento vale `SQUELCH_DEFAULT` (15) in `Demod::new`, il `PskDemodulator::set_squelch(None)` resta aperto per le prove. (2) La differenziale insegue solo entro ~baud/4: con la portante a +10 Hz in PSK31 l'AFC finiva a -5 Hz (verso sbagliato) e `AfcGuard` lo riportava indietro finche' la metrica era sotto 25. Nuovo aggancio in `psk.rs` (`set_acquire`, `take_acquired`): banda base ±120 Hz a 500 c/s, segnale al quadrato, FFT 512, riga al doppio della portante (scartate quelle a mezzo baud da una piu' forte e a meno di baud/4), stazione confermata in 3 spettri, poi seguita (anche oltre il limite: `AfcGuard::retune` sposta il limite) e tenuta fino a una frequenza scelta nuova, anche nelle pause lunghe (prova con 25 s di silenzio e la vicina in onda); ascolto tenuto entro 1,5 Hz. Il segno dell'AFC e' giusto (±5 Hz converge gia' prima): oltre ±baud/4 il bit si decide a caso e l'AFC va dove capita. Portante oltre ±limite all'inizio (PSK31 ±25 Hz) non si aggancia, di proposito: rimedio lo Snap. L'etichetta rossa del nominativo e' la frequenza del canale della lista, non quella della ricezione. (3) Snap della cascata sulla riga piu' forte = riga laterale a ±baud/2: `snapToPsk` in `digiSpectrum.ts`. Numeri sintetici (S/N in 2,5 kHz, 48 casi): prima 50,3% giusti / 33,5% spazzatura, dopo 88,3% / 1,9%, lista 88,9% / 1,5%; con la bandierina fuori 8-15 Hz in PSK31 la finestra resta a 91-94% contro 99% della lista (i primi 2,5 s servono all'aggancio; idea: rileggere l'audio tenuto come `CATCH_UP_S` della lista). Limite noto: PSK63 con la vicina gia' in onda prima della stazione scelta ed entro ±30 Hz si aggancia la vicina. Provato solo con segnali sintetici, NON con la radio.
- #83 IC-820H, ricerca sulla trasmissione (fonti pubbliche, nessuna modifica al codice): il modello Hamlib 3032 non ha Main/Sub fra i VFO, quindi `V Sub` -> RPRT -9 e' un rifiuto della libreria (RIG_ENAVAIL), NON della radio; nella famiglia IC-821H la libreria imposta il satellite con `07 D0`/`07 D1` (selezione banda, con nota «in modo satellite Main e Sub sembrano invertiti»), `targetable_vfo = 0` (nessun comando scrive la banda non selezionata: 0x25/0x26 non esistono in questa famiglia) e il satellite non si legge da CI-V. Unica via: selezionare la banda (`07 D1`), scrivere con `05`, tornare con `07 D0`. Da verificare con Vladimir (rigctl, Aurora chiusa, nessuna trasmissione): per ogni trama `w` annotare la risposta (FB = accettata, FA = rifiutata), poi `07 D1` + lettura `03` (se torna l'altra banda la selezione funziona anche se il display cambia solo l'indicatore MAIN), `05` con una frequenza di prova + `03`, `07 D0` + `03`; ripetere con SAT acceso e spento. Il manuale della radio e' una scansione senza testo: la tabella dei comandi non e' stata letta.
- WEFAX, ricevitore: dopo la chiusura con il tono di fine (450 Hz) la correlazione scambiava lo strascico per immagine (parte la fasatura a vuoto, il tono d'inizio dell'altro IOC passava inosservato) e l'AFC seguiva il tono come se fosse nero/bianco (sottoportante a -125 Hz). `since_end` in `decoder.rs`: per 20 s dopo un'immagine chiusa da `ok`/`ok2`/`stop` niente scorciatoia sig-Image e niente AFC su tono fisso. Prova `con_il_tono_di_fine_e_subito_una_cartina_con_l_altro_ioc_esce_la_seconda_giusta` (righe al minuto 120/60 come nelle impostazioni vere). Provato solo con le prove automatiche.
- WEFAX, rotazione delle cartine salvate: `POST /api/v1/wefax/images/{id}/rotate` (`{dir: left|right}`), `WefaxService::rotate_image` e `image_io::rotate_quarter` in mode-wefax (stesso id e stesso file, larghezza e altezza scambiate); pulsanti a frecce curve nella finestra grande di `History.tsx` (stili in `styles/wefax-ruota.css`), `postWefaxRotate` nel client, testi in sei lingue + nl/ru, riga nella Guida.
- Contest UBA DX CW/SSB, REF Contest CW, EUDX Contest, WAE DX Contest CW/RTTY in `contest/` (commit 48219b0e, ba358a88, b5125d5a, aa43fcaf; motore 33096209). Regolamenti letti il 2026-11-21: uba.be/en/hf/contest-rules/uba-dx-contest (edizione 2027, senza data), concours.r-e-f.org reg_cdfhf_fr_20260513.pdf + regles_communes_HF_fr_20251209.pdf, eudx-contest.com/rules (6a edizione, 2027), darc.de WAE Rules 2025 (date 2026); nomi Cabrillo diversi da UBA-DX-CW presi dall'elenco WA7BNM (fonte secondaria). EU DX scelto: EUDX Contest dell'EUDXCC (l'EU HF Championship e' un'altra gara, gia' come `.contest`). Motore: chiavi dello script con la banda pesate da `mult_band`; `[contest] qtc` nei contest in cartella; nelle prove `qtc = N` fra i totali. Provato solo con le prove automatiche (stazione italiana + stazione del paese ospitante, WAE con QTC nel punteggio), NON a schermo ne' in gara. **Da verificare:** UBA: RU/BY trattati come non ammessi (fuori dal denominatore del bonus), nessuna regola dei doppi scritta; REF: /MM per gli stranieri (4b contro 6b), quali paesi sono oltremare (tutti i prefissi F* + TK), Corsica solo 2A/2B, date 2027-2028 calcolate, versione inglese .docx non letta; EUDX: «proprio paese» inteso come stato UE (I = IT9), SV/a e OJ0 messi in GR e FI, /MM non trattati, nessun elenco dei nomi delle regioni; WAE: regola del 000 non applicata, IG9 per gli europei come paese non europeo «I», proprio paese in RTTY contato, limite 36 ore e 10 minuti non controllati. Alias: UBA sostituisce i vecchi «UBA CW CONTEST» e «UBA SSB Contest»; REF e WAE non sostituiscono i vecchi file (REF vale ancora per la fonia; i due WAEDC hanno lo stesso nome). Il formato `[edizione]` non sa dire «ultimo sabato»: UBA e REF hanno le date scritte anno per anno. Nuovo tipo di scambio `zona_itu` (solo suggerito sotto il nominativo, niente cattura: il motore ignora `quando` e nell'EUDX le stazioni UE mandano la regione). Per chi gareggia: nel REF il progressivo e' acceso per tutti, i francesi lo spengono e scrivono il dipartimento in «Numero di partenza»; nell'EUDX la regione o la zona ITU da mandare va in «Numero di partenza». La finestra QTC decide «europeo» col continente che le passa il servizio (database DXCC?): TA1 potrebbe finire dalla parte sbagliata, da verificare. Le righe `QTC:` del Cabrillo dei contest in cartella non sono provate (la prova passa una lista vuota).
- IC-820H (#83), esperimento `07 B0` di Vladimir (2026-11-21, SAT acceso e spento, file letti): la radio rimanda solo l'eco e il display non cambia; `f` rilegge 145000000 (la cache di Hamlib) e dopo `F` la banda scritta e' la stessa. `07 B0`, `07 D0` e `07 D1` non selezionano il Sub: via CI-V non c'e' un comando noto per scrivere la trasmissione sul secondo VFO. Conclusione: per l'IC-820H il Doppler segue solo la ricezione e la frequenza di trasmissione va messa a mano; da vedere se Aurora puo' mostrarla piu' chiara. Non provato in onda.
- Contest CQ WW DX CW / SSB, CQ WW RTTY DX, ARRL RTTY Roundup in `contest/` (regolamenti letti il 2026-11-21: cqww.com/rules, cqwwrtty.com/rules.htm, RTTY-RU-Rules.pdf v3.0b; fonti e differenze nei `regolamento.md`). Script Rhai per zona dallo scambio e paesi WAE (`Lookup.wae` in `dxcc`, `InfoPaese.wae`, `wae_prefix` nel JSON del paese), `ContestTesto.cabrillo_contest`, tipo `zona_cq`. Provato solo con le prove automatiche (`tests/cartelle_repository.rs`, ora anche il Cabrillo), NON a schermo ne' in gara. **Da verificare:** W/VE come paese nel CQ WW RTTY (non scritto); `CONTEST: ARRL-RTTY` (non nel PDF); date 2027 di CQ WW e 2027-2028 del Roundup (calcolate); `LOCATION` del Cabrillo (obbligatorio per gli USA nel CQ WW) e la parola `DX` al posto del QTH nel CQ WW RTTY si scrivono a mano; i limiti multi-operatore e le ore del Classic/Roundup non sono controllati; nessun raddoppio sui 160 m nel CQ WW 2026 (l'esempio del compito non c'e' nel regolamento). Il vecchio «CQ WW DX Contest» intatto e' sostituito dalla cartella CW (alias); la fonia ha la sua cartella. Le schede `zona_cq`/`stato` dichiarate non sono ancora disegnate dall'interfaccia (punto 8 di `docs/contest-formato.md`): la vista SSB mostra la griglia dei moltiplicatori dello script (`cq`, `dxcc`, `dom`). ARRL DX CW/SSB erano gia' in cartella; Sweepstakes non fatto. Per chi gareggia: la zona da mandare va scritta in «Numero di partenza» (senza, le macro mandano vuoto e il Cabrillo avvisa `exch_sent`); nel CQ WW RTTY lo scambio mandato fuori da W/VE e' `15 DX`; il Cabrillo non scrive `CATEGORY-POWER`, `CATEGORY-ASSISTED` ne' `LOCATION` (a mano); nel Roundup il progressivo e' acceso per tutti (il motore ignora `quando`): i W/VE lo spengono e scrivono lo stato. /MM nel CQ WW: punti come continente sconosciuto (3), da verificare. Entita' WAE con il loro continente (`wae_cont`: IG9 in Africa, TA1 in Europa).
- WEFAX, decodifica da file WAV (punto 6): `mode-wefax/src/wav.rs` (parser senza dipendenze), `WefaxService::decode_file` / `stop_file` (thread `aurora-wefax-file` che manda blocchi da 0,25 s con pausa di 2 ms e al massimo 8 in coda; il ricevitore usa un `FileJob` con decoder e ricampionatore propri, `Msg::FileStart/FileAudio/FileEnd`), `RxStatus.decoding_file` / `file_percent`, rotte `POST /wefax/file` e `/wefax/file/stop`, pulsante in `Controls.tsx`. Fermandola l'immagine in corso si butta, a fine file si chiude con `finish()`. Prove: `tests/da_file.rs` (stesso segnale a 12000 Hz PCM16 mono e a 44100 Hz float stereo: stesse cartine) e il parser; non provata con una registrazione vera ne' a schermo.
- #83 IC-820H, trasmissione sulla banda secondaria: nessun codice cambiato. Documentato: Hamlib (`rigs/icom/ic821h.c`) non ha un `set_vfo` proprio (usa `icom_set_vfo`), `targetable_vfo=0`; le trame `07 D0`/`07 D1` (Main/Sub) vengono dalle tabelle di IC-910/IC-9700/IC-7600 e Vladimir ha visto che sull'IC-820H non fanno nulla. Nelle stesse tabelle `07 B0` = scambio Main/Sub; sulla lista di Hamlib (2020, IC-821H) lo scambio XCHG funziona solo mandato a mano come trama grezza (`V XCHG` rifiutato) e il modo satellite non si comanda da CI-V. Ipotesi (non verificata sull'IC-820H, il manuale in rete e' illeggibile): scambio `07 B0`, `F` sulla banda ora principale, di nuovo `07 B0`. Non implementato: se il secondo scambio fallisse la Doppler della ricezione finirebbe sulla banda di trasmissione. Esperimento da chiedere a Vladimir (Aurora chiusa, niente PTT, prima SAT spento poi acceso): `rigctl -m 3032 -r COM3 -s 9600`, poi `f`; `w \0xFE\0xFE\0x42\0xE0\0x07\0xB0\0xFD` (il display scambia le bande? la radio risponde FB `FE FE E0 42 FB FD` o NG `FA`?); `f` (legge l'altra banda?); `F` con quella frequenza +1000 Hz (cambia la banda giusta sul display?); di nuovo la riga `w` e `f` (tutto come prima?). Con esito positivo si fa il ripiego solo per il modello 3032.
- WEFAX, soglia SQL nello spettro (punto 5b): `Scope.tsx` disegna a destra una barra del livello (`rx.signal`, 0-100, aggiornato dallo stato ogni secondo) con la riga rossa tratteggiata a `squelch_level`, solo con SQL acceso. Niente riga sulla curva: la curva ha una scala adattiva sua, senza corrispondenza col livello. Non visto a schermo.
- IC-820H (#83), tre file rigctl di Vladimir letti (2026-11-21, versione 0.11.322, modello 3032): `V VFOA` manda `FE FE 42 E0 07 D0 FD` e `V VFOB` manda `07 D1`; la radio rimanda solo l'eco, il display non reagisce e, dopo `V VFOB`, `f` rilegge la stessa frequenza del VFOA appena scritta: con i comandi di Hamlib il Sub non si seleziona (come il `V Sub` -> -9 di Aurora). Con SAB acceso o spento il comportamento e' uguale. Serve il manuale CI-V dell'IC-820H per trovare il comando giusto (lettura/scrittura del Sub); finche' non si trova non c'e' correzione per la trasmissione. Non provato in onda.
- WEFAX, correzione dell'inclinazione sulla cartina in corso (secondo compito, punto 3): `Decoder::set_config` con `slant_ppm` cambiato a immagine in corso chiama `straighten_written` (estratta da `update_slant`) e il periodo di riga cambia senza salti; `mode-wefax/src/slant.rs` (`ppm_from_points`, `straighten` per le cartine salvate), `WefaxService::adjust_slant` / `straighten_image`, rotta `POST /wefax/slant`, `SlantOverlay.tsx` (visore e finestra grande della cronologia). La misura a due clic si somma solo a `slant_ppm` (non a `learned_ppm`: `fixed_ppm` li somma entrambi e si conterebbe due volte). Provato solo con il segnale sintetico (`tests/ricezione.rs`) e con prove del servizio: da verificare con una cartina vera e a schermo.
- WEFAX, ricezione piu' precisa (primo compito, `mode-wefax`): `auto_ioc` (tono d'inizio con quota di potenza su 300/675 Hz in tre finestre di fila, solo in attesa del tono; `Decoder::base_ioc` tiene l'IOC scelto, il riconosciuto resta fino alla scelta di un altro), `learned_ppm`/`learned_weight`/`learned_rate` in `wefax.json` (il `PUT` li ignora, si azzerano con `POST /wefax/rx/reset_learned`; sommati al periodo solo se la fasatura non l'ha misurato), `dropped_blocks` nello snapshot (coda da 512 blocchi), `afc_range_hz` e passo massimo 25 Hz (8 Hz in immagine). Tutto provato solo con il segnale sintetico (`tests/ricezione.rs`, prove del servizio): da verificare con cartine vere.

## Nella 0.11.323 (pubblicata 2026-11-21T09:30:55Z)
- WEFAX, inclinazione automatica (`auto_slant`, accesa di base): `mode-wefax/src/decoder.rs` (`update_slant`, `estimate_slant`, `row_pos` con base di posizione continua) stima ogni 40 righe (fino a 400) la deriva con la correlazione tra righe distanti 16, corregge `smpl_per_lin` (`auto_ppm`, max 5000 ppm, azzerata a ogni immagine) e raddrizza le righe gia' scritte con spostamento a sotto-pixel (anche lo stiramento). Interfaccia: spunta in `Controls.tsx`, testi nelle otto lingue, Guida. Provato solo con il segnale sintetico (`tests/ricezione.rs`, manuale senza fasatura): da verificare con una cartina vera (il proprietario aveva 120 righe al minuto, IOC 576).'
- Battito delle installazioni aperte (POST /v1/battito ogni ~40 min, GET /v1/utilizzo per il portale Analytics): Worker in `servizi/rapporti` (tabella `battiti`, `migrations/0005_battiti.sql`), client in `rapporti.rs` (`segnala_battito`, `nota_uso` chiamata da `history.rs` per le decodifiche, da `AutoPtt::set(true)` e da `QsoCompleted` in `lib.rs`). DA FARE A MANO prima di usarlo: `wrangler d1 execute <db> --file migrations/0005_battiti.sql --remote`, poi `wrangler deploy` (in quest'ordine). Il testo dell'interruttore (`rep.activeTip`) e' aggiornato in otto lingue. Non provato dal vivo: solo `tsc` e prove sul client.
- IC-820H (#83), pacchetto `00A5E732` (AO-7, 0.11.322, 2026-11-21 08:14-08:17): il ripiego a due VFO era rimasto spento dalle 07:34 (rifiuto con il Sub spento) e la spunta si era tolta dalle impostazioni salvate; quindi nessuna scrittura della trasmissione (`I 432143428` non supportato) e il display di trasmissione fermo. Ora `LoopState.vfo_retry_at`: dopo 60 s si riprova da solo e la spunta non si tocca (`mode-sat/src/service.rs`, prova `il_ripiego_riprova_da_solo_dopo_un_minuto_e_non_toglie_la_spunta`). Segnalazione di Vladimir: SAT e SAB accesi, RX 145,951 e TX 445,951 fermo (e' la lettura ripiegata di 300 MHz).

## Nella 0.11.322 (pubblicata 2026-11-21T08:08:07Z)

- WEFAX: spettro (`components/wefax/Scope.tsx`, righe rosse su nero/bianco, riga del centro, clic = centro) sopra la cascata `DigiWaterfall` (righello in Hz 1000-3000), al posto della piccola cascata della testata; interruttori AFC (acceso) e SQL con soglia (spento, 30) in testata (`AfcSql.tsx`). Core: `meter.rs` (livello in banda contro fasce di rumore, istogramma della frequenza istantanea), AFC max 150 Hz anche sul tono fisso del nero, SQL parte solo con segnale che cambia e chiude dopo 3 s. Provato solo con segnali sintetici (`tests/ricezione.rs`); pagina NON vista a schermo (anteprima senza motore).
- IC-820H (#83), pacchetto `256A9FEB` (0.11.321, Sub spento, 2026-11-21 07:23): `V Main` ok e `f` = 435.633.498 (il Main e' la banda di ricezione), `V Sub` -> RPRT -9; il predefinito del modello (Main = VHF) faceva scrivere `F 145962153` sul Main sopra la ricezione e `V Sub` falliva ancora: la radio restava su 145,96 (`f` = 445962153). Corretto in `set_uplink_via_vfo` (se `V Sub` da' -9 non si scrive niente), prova `with_the_sub_off_the_uplink_is_not_written_over_the_receive_frequency`, fake con `reject_vfo`. Con il Sub acceso resta da capire il display di trasmissione (chiesto a Vladimir il test con rigctl).
- IC-820H (#83): rilettura registrata della frequenza di trasmissione dopo la scrittura sul VFO di trasmissione (`satelliti: trasmissione scritta ... riletta ...`). Vladimir (2026-11-21 06:09 UTC) dice che con SUB acceso l'RX Doppler va ma la frequenza di trasmissione sul display non cambia, e con SUB spento Aurora non vede la radio; nel `cat-comandi.log` del pacchetto 7CF56D20 le scritture `V Main` / `F 1459625xx` / `V Sub` danno RPRT 0 ogni 10 s. Hamlib: `ic820h.c` registra solo VFO A/B (niente Main/Sub, split, satmode; FIXME «What about MAIN/SUB mode? And satellite mode?»), `ic821h.c` ha il commento «backwards in satmode». Chiesto a Vladimir un pacchetto dopo un passaggio con l'ora e i due display.

## Nella 0.11.321 (pubblicata 2026-11-20T20:13:32Z)
- Pulsante «Usa un rigctld gia' acceso» (#87), importazione macro fldigi nel menu File di PSK/RTTY (#80), casella «Aggiorna da solo quando Aurora e' ferma» (spenta di base). Da provare: la finestra di importazione e la casella non sono state viste a schermo. Risposte da dare a schildgnom (#80) e Luc (#87) con la versione pubblicata.

## Nella 0.11.320 (pubblicata 2026-11-20T18:03:31Z)
- IC-820H (#83): il pacchetto SO-50 di Vladimir (0.11.319, ripiego a due VFO acceso) mostra ancora `F rx` su Main e `V Sub` + `F tx` su Sub: `aurora.log` dice «bande dei VFO: uguali o non leggibili» sia alle 14:49 sia alle 16:52. Causa probabile: Hamlib risponde alla stessa frequenza a due `f` ravvicinate (cache). `hamlib_rig.rs`: la sonda attende 700 ms dopo ogni `V`, scrive le due frequenze lette (`satelliti: lettura delle bande dei VFO: Main = .. Sub = ..`) e per il modello 3032 ripiega su Main = 144, Sub = 430 (come nella foto di Vladimir: sinistra 137.790 USB = 144, destra 437.790 FM = 430). Prova `the_ic_820h_defaults_to_main_144_and_sub_430_when_the_bands_cannot_be_told_apart`. Non provato con la radio vera.

## Messaggio di aggiornamento (Cloudflare)
- `aggiorna-0.11.320` (inglese, 115 caratteri, `prima_di` 0.11.320, valido 7 giorni dal 2026-11-20T18:03Z) pubblicato come ultimo: il proprietario ha detto di mandarlo ancora una volta con la 0.11.320 e poi di NON mandarne piu' senza una sua richiesta. `aggiorna-0.11.319` e' stato fatto scadere subito.

## Servizio dei rapporti (Cloudflare), 2026-11-20 sera
- Messaggio dell'autore `aggiorna-0.11.319` (solo inglese, 115 caratteri: la cartolina del tray e' larga 390 px e mostra circa 3 righe, il resto e' tagliato; stile `aggiorna`, `prima_di` 0.11.319, valido fino al 2026-11-27) scritto direttamente in D1 (tabella `messaggi`): il segreto `ADMIN_TOKEN` non e' sul PC.
- Contatore dei clic: rotta `GET /v1/vai/<id>?a=<indirizzo>` del Worker (`servizi/rapporti/src/index.ts`, `destinazioneSicura`: solo https e solo `github.com/wifi75/Aurora` o `aurora.iu3wjq.com`), tabella `messaggi_clic` (migrazione `0004_clic.sql`, gia' applicata e Worker gia' pubblicato con `wrangler deploy`; il commit del repo e' locale, non pushato). Il pulsante del messaggio passa da li' e rimanda a `releases/latest`. Lettura: `SELECT * FROM messaggi_clic` e `messaggi_visti`. Il file locale del Worker non e' ancora su GitHub.
- Misura dell'effetto (alle 17:29 UTC): 0 cartoline mostrate, 20 installazioni su 158 sulla 0.11.319+, 73 scaricamenti del setup 0.11.319.

## Nella 0.11.319 (pubblicata 2026-11-20T15:05:18Z)

- Etichetta dei satelliti (`SatAlertLabel.tsx`, `sat-label.css`): il nome sta in `.sat-label-name` (max 11 em, puntini) e il testo `{sat} tra {time}` e' diviso con un segnaposto, cosi' il conto alla rovescia non si taglia mai (#82, ct1bxx). Prove passate, non vista a schermo.
- `desktop/src-tauri/src/webview_check.rs`: `window_failed` + `failed_texts` (sei lingue) e in `lib.rs` `start` esce con `std::process::exit(1)` dopo il messaggio se `open_main_with_retry` fallisce (prima il `?` faceva il panic di Tauri «Failed to setup app»). La lingua dei messaggi di Windows viene da `agent::ui_lang()` (0-5): olandese e russo ricadono su un'altra lingua; da controllare come si sceglie. Non provato con un WebView2 rotto.
- Aperti: #83 Vladimir (attende il suo test con la 0.11.318 e la risposta su quale banda e' sul display principale), #85 Luc (minuti del watchdog), #81 Luc, #80, #72; #82 da chiudere dopo la 0.11.319.

## Nella 0.11.318 (pubblicata 2026-11-20T14:29:42Z)

- #83 (Vladimir, IC-820H, pacchetto `1C4290E3`): nel registro `F 437790670` diventa `f` = `137790670` e `F 145993111` sul «Sub» mostra 445,99: su questa radio Main e Sub sono le due **bande** (144 e 430 MHz) e una frequenza scritta nella banda sbagliata viene «ripiegata» di 300 MHz. `rig-control/src/hamlib_rig.rs`: `set_uplink_via_vfo` legge una volta per connessione la banda di Main e Sub (`sat_band_map`, azzerata alla riconnessione) e con `uplink_vfos` scrive la trasmissione sul VFO della sua banda e torna a quello dell'altra per la ricezione; con bande uguali o non leggibili resta la coppia Sub/Main di sempre. Provato con la radio finta e con il registro, NON con la radio vera: attende il test di Vladimir (ripiego a due VFO acceso, ISS o RS-44).
- #86 chiusa (M8GVC: il CQ ora parte con AutoTX spento).

## Nella 0.11.317 (pubblicata 2026-11-20T14:14:59Z)

- #86 (M8GVC, FT-710): con AutoTX spento, scegliere il messaggio 6 (CQ) e accendere Abilita Tx spegneva Abilita Tx e non partiva niente. Causa: `TxDriver::plan` (`engine-native/src/tx.rs`) scambiava quel CQ per un CQ automatico a QSO finito (`cq_by_user` era vero solo con il comando `CallCq`). Correzione: `TxCommand::Enable(true)` con `ntx() == 6` e `SelectTx(6)` con Abilita Tx acceso impostano `cq_by_user`. Prova `enable_tx_with_the_cq_message_selected_transmits_even_with_auto_tx_off` (fallisce senza la correzione). Il CQ automatico a QSO finito resta solo con AutoTX.
- #85 (Luc, «auto stopping»): e' il watchdog di trasmissione (`TxWatchdogTimer`, 6 minuti di base, campo «Periodo di monitoraggio Tx» in Impostazioni › Modi › FT8 · FT4): non e' un difetto; spiegato e chiesto dopo quanti minuti si ferma.
- #83 (Vladimir, FO-29, pacchetto `0898B75C`): il ripiego a due VFO era spento (nessun `V Sub` nel log); in piu' con il SAT acceso la lettura `f` passa a turno da 145 a 435 MHz e Aurora conta 212 «cambi di banda» (azzerano Tx e messaggi). Sembra che la radio scambi da sola principale e secondario: non corretto, servono i suoi test.
- Guida aggiornata alla 0.11.316 (ingranaggio unico, rotore fuori dalla barra, etichetta dei satelliti opzionale, pallini e prova, conferma di chiamata, WEFAX, cascata).

## Nella 0.11.316 (pubblicata 2026-11-20T13:10:20Z)

- Diagnosi della trasmissione (piano in `docs/piano-diagnosi-trasmissione.md`): `tx-native` ha `TxReport` (picco audio, PTT letto dalla radio con `Ptt::read_state` su Hamlib, hook `set_report_hook`) e scrive «trasmissione: audio picco … PTT …» in `aurora.log`; `aurora-core` ha `StationStatus.tx_chain` (`TxChain`), pubblicato da `engine-native` (`NativeControls::set_tx_chain`) e riempito in `aurora/src/lib.rs` (modo radio dal CAT).
- Interfaccia: `TxChainDots.tsx` (tre pallini nella scheda Trasmissione, `lib/txChain.ts`) e `settings/TxCheck.tsx` («Prova senza trasmettere» in Impostazioni › Radio, `lib/txCheck.ts`): solo lettura, mai PTT ne' audio. Non visti a schermo ne' con una radio vera.
- Da fare: prove con il finto `rigctld` (radio che rifiuta il modo, PTT non leggibile), confronto del modo letto con quello richiesto (ora solo «modo che non porta il digitale»), eventuale riga in `diagnostica/` del pacchetto.

## Nella 0.11.315 (pubblicata 2026-11-20T12:54:27Z)

- Rotore fuori dalla barra in alto (`ModeNav.tsx`). `RotatorTab.tsx`: gruppo «Mostra in home» con due caselle (`useRotorCardVisible('station')` e `('digi')`) e il pulsante «Apri la pagina del rotore» (`#/rotore`). Non visto a schermo.
- Prossimo lavoro approvato: diagnosi della trasmissione (pallini nella scheda Trasmissione e «Prova senza trasmettere»), piano in `docs/piano-diagnosi-trasmissione.md`.
- #83 Vladimir: il test a mano con `rigctl` (Aurora chiusa, COM3 libero) non cambia niente sul display; risposto in russo con la richiesta della riga esatta di `rigctl` e delle risposte a `f` e `m`.

## Nella 0.11.314 (pubblicata 2026-11-20T12:43:43Z)

- Etichetta dei satelliti opzionale (`barLabel` in `SatAlertSettings`, spenta di fabbrica; `lib/satBarLabel.ts`, `useBarLabel`): casella in Impostazioni › Modi › Satelliti › Inseguimento e rotore › Avvisi e nella campanella; senza, `SatAlertEngine` non e' montato (niente suono in ogni pagina). Etichetta in mezzo alla barra (`margin-inline: auto`, `ModeNav.tsx`), clic = sceglie il satellite (`getSat`/`putSat`, `pickSatellite`) e apre `#/satelliti`. Non vista a schermo con un satellite vero.
- `UpdatePoster.tsx`: tolta `InstallProgress` (avanzamento nella schermata scura). Visto in anteprima con dati finti (pagina di prova poi tolta).
- Il «doppio clic che non funziona» segnalato dal proprietario era la casella «Doppio clic: rispondi alla stazione» tolta per errore: nessun difetto. Le caselle `gen.dblReply` e `gen.confirmUnseen` sono vicine e simili: valutare nomi piu' distinti.

## Nella 0.11.313 (pubblicata 2026-11-20T12:28:56Z)

- WEFAX (`WefaxPage.tsx`): l'ultima frequenza scelta nel menu si salva (`aurora.wefax.lastPreset`) e entrando nella pagina la radio ci torna. `modeRetune.ts`: la banda di prima si salva anche in `aurora.modenav.beforeReceiveOnly` e, se manca, si usa `lastDial('ft8')` (mai la banda piu' bassa, 1,810 MHz). Resta un caso da correggere: frequenze WEFAX dentro una banda radioamatoriale (es. 3 855 kHz) portano alla frequenza FT8 di quella banda e non a quella di prima.
- Menu frequenze WEFAX 19 rem, carattere 0,66 rem (testo piu' lungo misurato 252 px su 304). Locandina di aggiornamento senza la striscia «NUOVA VERSIONE», stella «NUOVA APP» a destra. Divisore cascata/spettro 14 px con maniglia 110x6 px (`bozze.css`). Non verificati a schermo.
- Proposta del proprietario, approvata (2026-11-20): togliere dalla locandina l'avanzamento del download (`InstallProgress` in `UpdatePoster.tsx`), restano i pulsanti; l'avanzamento sta nella schermata scura. Simulare prima in anteprima con risposta finta.
- Pacchetto `7A547090` (IC-820H, AO-7/RS-44): la lettura `f` del VFO principale salta tra 145,95 e 445,95 MHz e Aurora la scambia per cambi di banda; `F` sul VFO secondario accettato, una volta rifiutato (-9) e il ripiego a due VFO si spegne. Attende il test a mano di Vladimir.

## Nella 0.11.312 (pubblicata 2026-11-20T12:02:41Z)

- `Live.tsx`: la conferma di chiamata (`CallUnseenDialog`) non scatta piu' sul doppio clic delle liste dei decode (`onReply(id, true)`); resta per «CQ e novita'», DXpedition e avvisi di nuovi DX. L'interruttore sta in Impostazioni › Modi › FT8 · FT4 › Messaggi e filtri › Schermo (non in «Registrazione e comportamento»: F1DSZ era stato indirizzato male).
- `modeRetune.ts`: `rememberDialBeforeReceiveOnly` + `retuneTo`: uscendo dal WEFAX (frequenze fuori banda) la radio torna alla banda di prima; `lastPage.ts`/`ModeNav.tsx`: il gruppo PSK·RTTY·SSTV riapre l'ultima pagina del gruppo (`lastTextPage`).
- Russo nell'installatore (`installer.nsi`: 17 `LangString ... ${LANG_RUSSIAN}`, pagina dei dati con `$LANGUAGE = 1049`, `tauri.conf.json` «Russian») e nelle cartoline del tray (`agent.rs`: `tr_ru`, `tr_for`; il russo NON e' una settima voce fissa delle tabelle: `lang_index("ru")` resta 1). Non provato con l'installatore vero.
- Pulsanti dei temi scuri illeggibili al passaggio del mouse: controllo in corso (agente), file `pulsanti-tema-scuro.css`.
- Poi: 0.11.313 con l'etichetta dei satelliti nella barra in alto (#82).

## Nella 0.11.311 (pubblicata 2026-11-20T10:34:08Z)

- Russo come ottava lingua: dizionario `app/src/i18n/ru/`, foglio `docs/traduzione-russa.csv`, script `export-ru.mjs`/`importa-ru.mjs` (8.311 voci: interfaccia, Guida, ultime 10 Novita', macro). Traduzione AI da far rivedere (Vladimir). NON nell'installatore Windows ne' nelle schede dell'agente nel tray (restano in inglese). `lang` della pagina impostato all'avvio.
- Testi lunghi: `app/src/styles/lingue-lunghe.css` (9 punti corretti a 1280/1024/900 px); «Средне» al posto di «Удовлетворительно».
- Locandina: bollino a stella «NUOVA APP» tradotto. WEFAX: display di sola ricezione (frequenza, USB/LSB, led), cascata unica `bare`, larghezze fisse.
- Prossimo: 0.11.312 (conferma di chiamata solo per stazioni non nelle liste: segnalazione di F1DSZ), 0.11.313 (etichetta satelliti nella barra in alto, #82).

## Nella 0.11.310 (pubblicata 2026-11-20T09:50:38Z)

- Correzioni alla 0.11.309: all'avvio con una versione nuova si apre direttamente la locandina (tolta la finestrella «Vuoi aggiornare?»); sul PC un solo ingranaggio (`ModeNav.tsx`) che apre le Impostazioni; Guida e Novita' dentro le Impostazioni.
- WEFAX: tutto su una riga (i comandi dello zoom passano nello spazio `wefax-toolbar-slot` della testata con un portale), menu delle frequenze con carattere piu' piccolo; `commands/frequency` accetta `receive_only` (HF 1,6-30 MHz, fuori dalle bande radioamatoriali) e la pagina lo usa; niente TUNE nel WEFAX (`CompactRadioDisplay noTune`): solo ricezione.
- Russo: struttura preparata (`app/src/i18n/ru/`, `scripts/export-ru.mjs`, `importa-ru.mjs`, foglio `docs/traduzione-russa.csv`), tenuta nello stash `russo-infrastruttura`; traduzioni a lotti nello scratchpad; NON ancora nel repository.

## Nella 0.11.309 (in lavorazione, non ancora pubblicata)

Satelliti senza split da CAT (issue #83, IC-820H di Vladimir) e rotore.

- Evidenza su Hamlib (rigctl 4.7.2 incluso in `desktop/src-tauri/hamlib-bundle`, `--dump-caps`): IC-820H (3032) ha VFO `VFOA VFOB MEM` (niente Main/Sub), `Can set VFO: Y`, `Can get VFO: N`, nessuno split (ne' frequenza ne' modo), nessuna funzione e nessun livello leggibile (per questo `l STRENGTH` da -1). IC-910 e IC-9700 hanno invece split vero, `Main/Sub` e `SATMODE`.
- Spam tolto: `HamlibRig::set_tracking_frequency` (solo `F`) al posto di `set_frequency` per il Doppler (prima ogni secondo: controllo dell'arrivo, `M PKTUSB 0`, `S 0 VFOA`; con split vero lo spegneva a ogni passo). `Inner.split_unsupported`: dopo il primo -4/-11 a `S` non si riprova piu' (cambio di frequenza, `release_tx_split`, riconnessione) con un solo avviso; si azzera alla riconnessione (`open`). Il modo dei satelliti si scrive solo quando cambia (`LoopState.last_rx_mode/last_tx_mode`).
- Memoria dei rifiuti per collegamento radio: `SatRadio::link_id` (`HamlibRig::session_id`); resta tra un passaggio e l'altro, si azzera se la radio si ricollega o cambia.
- Modi: `SatSettings.rx_mode/tx_mode` (Auto, USB, LSB, FM, CW), `Transponder::auto_modes` (invertente: ricezione USB, trasmissione LSB; `Transponder.tx_mode` opzionale nel catalogo). Pagina Impostazioni › Modi › Satelliti › Doppler e rotore.
- Ripiego «due VFO» (`SatSettings.uplink_two_vfo`, SPENTO di fabbrica, sperimentale): `HamlibRig::set_uplink_via_vfo` fa in un blocco `V Sub`, `F`, `M` (solo se cambiato), `V Main`; se `V Sub` e' rifiutata prova `V VFOB`/`V VFOA` e ricorda la coppia; mai durante il PTT; il ritorno al VFO di ricezione si tenta sempre. Scrive la trasmissione al massimo ogni `uplink_interval_s` (10 s) o oltre `uplink_threshold_hz` (300 Hz, non piu' di una volta ogni 2 s). Al primo rifiuto si spegne da solo (`disable_two_vfo`, spunta tolta nel file salvato) e lo stato dice perche'. `SATMODE` si prova una volta, solo col ripiego acceso. `FakeRigctld` ha `fail_split_with` e `refuse_vfo`.
- Rotore (richiesta del proprietario): sparisce la costante di 150 s. `service.rs::drive_rotor`: PRIMA (`rotor_lead_s` 60, 0 = solo all'AOS, serve `rotor_preposition`), DURANTE (`rotor_start_el_deg` 0, `rotor_interval_s` 2, tolleranza `rotor_min_move_deg` 2, `min_el_deg`), DOPO il LOS (`rotor_after` stay di fabbrica / park / next, `rotor_after_delay_s` 10; un solo comando, annullato se riparte un passaggio; `next` cerca fra i satelliti spuntati nelle prossime 12 h). Pagina `SatTrackingPane.tsx` (Impostazioni › Modi › Satelliti › Inseguimento e rotore, `sattrack`; pulsante anche dalla scheda «Doppler e rotore» della pagina Satelliti): cursore e casella collegati, riepilogo in una riga, anteprima del prossimo passaggio, «Ripristina i valori di fabbrica». L'avviso sonoro (`rotorLeadMin`) e il movimento restano separati.
- Provato: `cargo test -p mode-sat` (43), prove di `rig-control` con rigctld finto (split rifiutato una volta, solo `F`, due VFO, ripiego VFOB, niente cambio di VFO in trasmissione), vitest sat/i18n/settingsTree, `tsc`.
- NON provato: nessuna radio vera. Non si sa se su un'IC-820H `V VFOB` selezioni la banda secondaria (potrebbe cambiare solo il VFO B della banda corrente: inutile per i 2 m) ne' se `V Sub` sia accettata; il rotore (prima, durante, parcheggio, prossimo passaggio) non e' provato con un rotore vero. Valori scelti da me senza dato: intervallo 10 s e soglia 300 Hz del ripiego, ritardo di 10 s, finestra di 12 h per `next`, tolleranza massima 10 gradi nell'interfaccia (il core accetta fino a 45 per i vecchi file), laterali dei non invertenti uguali (USB).
- Da decidere: tenere il ripiego a due VFO se la prova di Vladimir dice che `V VFOB` non cambia banda; accendere `SATMODE` anche con lo split vero (oggi no).

## Nella 0.11.309 (pubblicata 2026-11-20T09:35:42Z)

- Pagina «Inseguimento e rotore» dei satelliti (`SatTrackingPane.tsx`): anticipo di fabbrica 60 s (prima 150), tolleranza, parcheggio opzionale; spam di split e modo tolto sull'IC-820H; ripiego sperimentale a due VFO (spento); modi di ricezione e trasmissione. NON provato con radio e rotore veri.
- ADIF automatico con interruttore (spento) e cartella; conferma prima di chiamare una stazione non vista (`seenStation.ts`, `CallUnseenDialog.tsx`); ROS 2,0 (1,0-4,0) nella scheda Radio; ora automatica accesa di fabbrica (migrazione una tantum con `clock.v2`).
- Bandierine di Bande aperte: il paese si ricava dal nominativo in `GET /online/psk`; il messaggio in onda (Tx1..Tx6) si scrive nel registro; finestra principale riprovata 4 volte se WebView2 fallisce (0x80070578).
- Caso XX9BB (pacchetto 977AED7F): 10 trasmissioni ogni 30 s dalle 09:00:13, fermate a mano alle 09:05; il registro non diceva come era partita la chiamata (ora si scrive il messaggio in onda). Origine NON accertata.
- Prossimo: russo come ottava lingua (dizionario `app/src/i18n/ru/`, foglio `docs/traduzione-russa.csv`), controllo dei testi che escono dallo schermo.

## Nella 0.11.308 (pubblicata 2026-11-20T07:44:01Z)

- Satelliti: preavviso del rotore (`lib/satAlerts.ts`, evento `rotor`, minuti impostabili 0-30, suono suo), ascolto del suono alla scelta, CSS delle schede «Doppler e rotore» e «Informazioni» senza spazi vuoti (non verificato a schermo). Il rotore non parcheggia (decisione da prendere).
- Contest: Cabrillo con nomi ufficiali (JIDX-CW, CQ-WW-RTTY, ARRL-DX-CW...) e `avvisi` dopo il salvataggio; `da` (attivazione) accettato da punteggio, Cabrillo, controllo e QTC quando la finestra e' scaduta; regole di date per CQ WW RTTY, CQ WPX RTTY, ARRL DX CW/SSB, ARRL 10 m (da ricontrollare sui regolamenti). `.dom` JIDX NON fatto: servono i dati veri.
- Stazione del PC adattabile (`styles/stazione-adattabile.css`, solo sopra la soglia del telefono); telefono invariato. Provata solo con un server finto.
- `desktop/src-tauri/src/lib.rs`: la posizione non si legge piu' dentro CloseRequested (mitigazione del panic tao re-entrant, senza backtrace).

## Nella 0.11.307 (pubblicata 2026-11-20T07:21:50Z)

- WEFAX: crate `core/crates/mode-wefax` (ricezione ripresa da fldigi 4.2.13 di W1HKJ e Remi Chateauneu, GPL-3.0; 14 prove con segnale sintetico), rotte `/api/v1/wefax`, evento `wefax`, pagina `#/wefax` (visore adattabile, 1:1, zoom, cronologia), menu delle frequenze per banda e frequenza (`app/src/lib/wefaxStations.ts`, fonte NOAA/NWS marzo 2025 e DWD; la radio va sulla portante, assegnata - 1,9 kHz), finestra staccata. NON provato con segnali veri; 240 LPM solo in modo manuale.
- SSTV: entrando nella pagina la radio va sui 20 m (14,230 MHz). TUNE: un clic avvia sempre l'accordatura (prima, con l'accordatore gia' inserito, la spegneva) e resta acceso finche' non lo spegni.
- Aggiornamenti: primo controllo del core subito, finestrella «Vuoi aggiornare?» (Si/X) all'avvio, icona in testata ogni giorno, cartolina del tray a ogni avvio e ogni 6 ore. Non provato dal vivo.
- Olandese completo (202 + 17 voci). Elenco breve delle cose da fare: `docs/da-fare.md`.
- Indagine panic `tao` re-entrant (senza il backtrace): sospetti in `desktop/src-tauri/src/lib.rs` (CloseRequested sincrono, creazione/chiusura finestre dentro il gestore) e `agent.rs` (cartolina costruita in `run_on_main_thread`); correzione ancora da fare.

## Nella 0.11.306 (pubblicata 2026-11-20T06:09:57Z)

- Satelliti (`mode-sat/src/service.rs`, `LoopState.split_failed`/`tx_failed`): se la radio rifiuta lo split (IC-820H con Hamlib, `S 1 VFOB` -> -11) non si riprova ogni secondo e il Doppler della ricezione va avanti; la frequenza di trasmissione si prova una volta. Non provato con un'IC-820H; non si sa se Hamlib sa impostare la frequenza di trasmissione di quella radio.
- Issue: chiuse #77 (Pere: il clic era la lettura del ROS, ROS spento e potenza accesa) e #78; aperte #72 (Vladimir RN8M: provare la 0.11.306 con il Doppler e mandare un nuovo pacchetto), #81 (Luc), #80, #82 (avviso dei satelliti su tutte le pagine: oggi `useSatAlerts` esiste solo in `SatPage`).

## Nella 0.11.305 (pubblicata 2026-11-20T05:40:47Z)

- Display esteso FT-2000 (`radio-display/RigFaceDisplays.tsx`, `Ft2000Display`): disegno 504 x 66 (`FT_FACE_W`, `FT_FACE_H`), VFO-A sopra la frequenza, VFO-B e modo a sinistra della seconda, in TX il misuratore diventa potenza (ambra) con la barra ROS sotto, tilde se l'S-meter e' stimato; CSS in coda a `radiodisplay.css` (`.rd-face-slot--ft` con `aspect-ratio`). `FaceHeader` usa `useTuneButton` (TUNE a scatto). Visto in anteprima, non con una radio vera.
- DX Cluster: server di serie IU3WJQ-7, HamQTH, Hamcloud, VE7CC (N2KI solo nel catalogo); il primo dell'elenco si riprova ogni 5-10 s (`PRIMARY_RETRY_MAX_SECS`).
- Aperte: #77 (Pere), #81, #80, #78, #72; da provare dal vivo display, TUNE, spot dal cluster e passaggio al successivo.

## Nella 0.11.304 (pubblicata 2026-11-19T20:00:19Z)

- DX Cluster (`dx-cluster/src/lib.rs`): di base primo cluster «IU3WJQ-7» (`cluster.iu3wjq.com:8000`) e `spot_via` telnet con `spot_server` su di lui (`AURORA_CLUSTER_*`); `aurora_cluster_first` lo porta in cima anche nei file gia' salvati e rinomina «IU3WJQ» in «IU3WJQ-7»; `send_spot` prova in ordine il cluster scelto e poi gli altri collegati (8 s ciascuno, spot doppio possibile se un cluster accetta senza ripetere); `Config::repaired` ripara i valori non validi tenendo utente e password, con copia `dx-cluster.json.bak` prima di rimpiazzare il file. 38 prove verdi.
- TUNE (`radio-display/RadioExtras.tsx`, `useTuneButton`): un clic avvia `tune` e il tasto resta acceso (giallo) finche' un altro clic lo spegne (prova `atu`, poi si spegne comunque); acceso anche se la radio dice `tuner = true`. Usato dal display compatto, dalla scheda e dal display stretto. Non provato con una radio vera.
- Testata: display spostato di 5 px in basso (`.live-top > :last-child`), stimato da uno screenshot: da controllare a occhio.
- Ambiente: il disco C: era pieno (core\target\debug pesava circa 133 GB, ora cancellata); `cargo test`/`check` rigenerano la cache. La compilazione dell'installatore fallisce se il disco e' pieno.
- Aperte: #77 (pacchetto di assistenza di Pere), #81, #80, #78, #72.

## Nella 0.11.303 (pubblicata 2026-11-19T19:22:50Z)

- Radio spenta: la scritta grande (`body.rd-radio-off .rd-header::after`) ha la misura `min(clamp(14px, 1.4vw, 28px), --rd-off-w * 0,94 / (--rd-off-chars * 1,1))`; `--rd-off-chars` lo scrive `RadioDisplayHeader.tsx`, `--rd-off-w` e' la larghezza nota di ogni disposizione (`.rd-duo--single` 19,125 rem, `.rd-duo--pair` 38 rem, `.rd-face-slot` 40 rem). Niente `container-type` sul display (issue #55 e #60). Tolta dal disegno compatto la scritta piccola `noRadio`. Provata in anteprima con il tedesco (25 caratteri).
- Aperte: #77 (Pere: il clic resta con la potenza spenta; con il ROS spento «non collegato» e niente trasmissione: aspetto il pacchetto di assistenza; causa non trovata nel codice), #81 (Luc: prova di Wait and Pounce), #80, #78, #72.

## Nella 0.11.302 (pubblicata 2026-11-19T18:00:19Z)

- Display compatto ridisegnato: `radio-display/MiniDisplay.tsx` (viewBox 306 x 52, barre di 10 tacche uguali, in TX barra potenza e barra ROS con `PWR`/`SWR` alla stessa altezza), `RadioExtras.tsx` (`TuneChip`), `RadioDisplayHeader.tsx` (disposizione `single`: TUNE sopra il disegno, nessuna scheda a destra; `pair` conserva la scheda), CSS in coda a `radiodisplay.css` (`.rd-tune`, `.rd-duo--single` alto 3,25 rem, slot 19,125 rem, TUNE a `left: 41.8%`, `top: 63.5%`). Visto in anteprima, non con una radio vera. Le 12 prove di `RadioDisplay.test.tsx` che falliscono erano gia' rotte prima e non sono state rifatte.
- Da fare: seguire #77 (Pere), #72, #80, #78; pacchetto di crash con panic di tao 0.37.2; prove del display e del filtro dei falsi messaggi con radio vera.

## Nella 0.11.301 (pubblicata 2026-11-19T17:27:22Z)

- Display compatto (`radio-display/MiniDisplay.tsx`): frequenza principale piu' bassa con il VFO B in piccolo sotto (altra sessione), RX/TX piu' piccoli, misuratore ridotto (`scale(0.78 0.55)`), «PWR» e «SWR» sotto (valori in trasmissione, trattino in ricezione), niente scritta del modo. `RadioExtras.tsx`: la scheda di destra non ripete le frequenze, resta TUNE. Visto in anteprima, non con una radio vera.
- `RigConfig.read_tx_power` e `read_tx_swr` (`rig-control/src/cat_config.rs`, true di base): in trasmissione `hamlib_rig.rs` legge solo cio' che e' acceso (clic dei misuratori del TS-850, #77); caselle in Impostazioni › Radio (CAT), Guida in `guide-station.ts`. Il ROS spento disattiva la protezione. Non provato con un TS-850.
- DX Cluster: la password si manda senza spazi ai bordi (altra sessione).
- Da fare: scrivere a Pere (#77) quale casella provare; #72 (IC-820H e IC-707), #80 (macro FLDigi), #78 (chiudere dopo qualche giorno); il pacchetto di crash con «event handler is re-entrant» (tao 0.37.2) di un FTDX-101MP.

## Nella 0.11.300 (pubblicata 2026-11-19T17:12:15Z)

- DX Cluster: `dx-cluster/src/lib.rs` (`send_spot`: se `spot_server` non e' nell'elenco si usa il primo collegato; la sessione tiene `last_said` e `refused` e riporta cosa ha scritto il cluster alla chiusura); `settings/ClusterTab.tsx` (`LedLight` segue lo stato vero del collegamento, non solo la prova); caselle utente e password piu' larghe (`settings-layout.css`); `SpotDialog.tsx` mostra `ApiError.detail`. Causa del caso del proprietario: il server era stato riaggiunto con un altro indirizzo (`cluster.iu3wjq.com:8000` al posto di `dxregia.iu3wjq.com:8000`) e `spot_server` puntava ancora al vecchio.
- `engine-native/src/tx.rs` (`wait_and_pounce`): i CQ delle stazioni escluse dal filtro delle Automazioni non si scelgono; prova `wait_and_pounce_skips_cqs_of_stations_excluded_by_the_automations`. Segnalazione #81 (Luc): da avvisarlo che e' in versione; non provato in aria.
- Da fare: #77 (Pere, TS-850S: clic in trasmissione, decidere se leggere solo potenza o solo ROS), #72 (IC-820H in modo satellite, in attesa del pacchetto), #80 (macro FLDigi, richiesta), #78 (IC-7300, aperta qualche giorno).

## Nella 0.11.299 (pubblicata 2026-11-19T12:05:16Z)

- `settings/useAutosave.ts`: il salvataggio automatico non parte finche' il cursore e' in una casella `input[type=password]` (riprova a ogni pausa e salva appena si esce). Causa: la pausa di 600 ms tra due tasti salvava la password a meta', `setDraft(null)` svuotava la casella e compariva il segnaposto «Password salvata». Vale per tutte le schede con `useAutosave` (cluster, CAT, ecc.). Prova: `settings/useAutosave.test.tsx`. Non provato a mano.

## Nella 0.11.298 (pubblicata 2026-11-19T10:38:46Z)

- `lib/resetLayouts.ts` (`isLayoutKey`, `clearLayouts`): il pulsante di Impostazioni › Aspetto (`ap.boardResetBtn`) toglie dal browser tutte le disposizioni salvate (voci `aurora-board`, `aurora-bottom`, `aurora-finestre`, `aurora-layout`..., `aurora.*.cols`, larghezze e altezze, `aurora.sat.layout`, `aurora.xrx.*`) e ricarica; il `removeItem` si propaga al server delle preferenze (`syncSettings`). Il vecchio «Ripristina» azzerava solo `useBoard` e `useBottom`. Non provato dal vivo. Se compare una nuova voce di disposizione va aggiunta a `resetLayouts.ts`.
- `audio-spectrum/src/capture.rs` (`prepare_input`): scelta del dispositivo e lettura dei formati in un thread nuovo (errore COM 0x80010106 «modo thread» su Windows 11, Realtek «Mixage stereo»). Ipotesi dal messaggio di errore, non provata su quel PC.
- Dalla segnalazione #77 (Pere, TS-850S): il clic c'e' solo in trasmissione, quando si leggono due cose (potenza e ROS); da decidere quale tenere. Dalla #81 (Luc): il modo AUTO sceglie anche stazioni che non chiamano, e il filtro dei chiamanti non vale li'; da verificare e correggere. Da fare: invio ai moduli degli antivirus dei falsi positivi (Defender ha tolto file di Aurora a 5 installazioni della 0.11.292), importazione delle macro FLDigi (#80, richiesta).

## Nella 0.11.297 (pubblicata 2026-11-18T18:30:25Z)

- Solo la Guida (`guide-station.ts`): tolti i riferimenti agli stili `icdual`, `icblue`, `ledline`; dizionario olandese senza le loro voci. Display piu' basso con VFOB sotto la frequenza: ancora da fare, in attesa del via del proprietario (vedi «Da fare (olandese)» qui sotto).

## Da fare (olandese)

- `npx vitest run src/i18n` ha 2 prove che falliscono da prima di oggi: una voce olandese di Guida senza testo base (`t.12gffgz315`, elenco passaggi dei satelliti) e circa 13 voci dell'interfaccia senza traduzione olandese (si usa l'inglese). Si risolve con `node scripts/export-nl.mjs` e `node scripts/importa-nl.mjs` (foglio `docs/traduzione-olandese.csv`).
- Prossimo passo (display): il proprietario vuole il display piu' basso (circa 72 px), frequenza principale piu' piccola e in alto, sotto «VFOB» piccolo con la seconda frequenza, niente scheda a destra; mostrato come disegno, in attesa del suo via.

## Nella 0.11.296 (pubblicata 2026-11-18T17:17:56Z)

- `RadioExtras.tsx`: scheda dei VFO con VFOA (frequenza dal display, `displayModel`) e VFOB uno sotto l'altro e TUNE pulsantino; CSS in coda a `radiodisplay.css` (`.rx-vfos`, `.rd-duo .rd-slot--extras` largo quanto serve, a destra). FT-2000 e IC-7610 non la usano. Visto solo in anteprima con dati di prova.
- Prossimo passo: risposte di Luc (#78), Pere (#77), IZ0008SWL (#79), Hernan (#72); prova del filtro dei falsi messaggi a radio spenta.

## Nella 0.11.295 (pubblicata 2026-11-18T15:44:00Z)

- `ft8/src/filters.rs` (`reject_message`, dentro `chkfalse8`): messaggi con due `<...>` scartati; messaggi i3 = 3 (RTTY Roundup) arrivati al controllo dei dubbi scartati se nessuno dei due nominativi e' nell'elenco (con elenco vuoto JTDX lascia passare tutto). Filtro NOSTRO, non di JTDX: da confrontare con REA; in un contest vero un messaggio debole di una stazione non ancora conosciuta puo' essere perso. Non provato con radio spenta dal vivo.
- Tolti i display `icdual`, `icblue`, `ledline` (migrazione in `radioDisplay.ts`: `RETIRED_STYLES` -> ic7610, ic7610, led; `icblueq` -> ic7610).
- Segnalazioni aperte: #78 (Luc, IC-7300: S-meter vuoto dopo l'ultimo aggiornamento; chiesto pacchetto e stile del display; causa non nota, i display ridisegnati in 0.11.293/294 possibili), #73 (Toly), #77 (Pere). Spot: con «Invia gli spot da: DXSummit» lo spot non entra nella rete telnet (spiegato al proprietario).

## Nella 0.11.294 (pubblicata 2026-11-18T12:56:40Z)

- Pannelli e display a blocco unico (FT-2000, IC-7610, Icom doppio, Icom blu, LED essenziale) senza indicatori finti: restano frequenze dei VFO, modo, RX/TX, S-meter in RX, potenza e ROS in TX, TUNE; disegni piu' corti; stile «Icom blu con banda passante» ora «Icom blu» (migrazione in `radioDisplay.ts`). Non provato con radio vera.
- Prossimo passo: risposte di Pere (#77) e Toly (#73); schede del contest mancanti.

## Nella 0.11.293 (pubblicata 2026-11-18T12:13:38Z)

- Display della radio: `RadioExtras.tsx` mostra solo VFO B e TUNE (TUNE se la radio dice di avere l'accordatore o se i dati extra non si leggono); tolti `PwrSplitBox`, `SplitButton`, `RadioExtrasGroup` (le caselle `rx.*`). `CompactRadioDisplay.tsx` usa la stessa regola per TUNE. I pannelli FT-2000 e IC-7610 (`RigFaceDisplays.tsx`) hanno ancora gli indicatori disegnati: restano spenti senza i dati extra; se danno fastidio vanno svuotati.
- `core/crates/rig-control/src/meters.rs`: `TX_LEVELS = [Swr]`; in trasmissione si leggono solo potenza e ROS (il ROS ferma la trasmissione se troppo alto). Su Kenwood HF come il TS-850S ogni lettura di ALC/COMP fa cambiare il selettore del misuratore della radio (clic del relais), segnalazione #77 di Pere (Hamlib, 0.11.292). Da provare con quella radio.
- Prossimo passo: risposta di Pere (#77); risposta di Toly (#73: casella di avvio, versione 0.11.292); schede del contest mancanti.

## Nella 0.11.292 (pubblicata 2026-11-18T10:24:05Z)

- `RigConfig.restore_on_start` (`rig-control/src/cat_config.rs`, true di base): se falso `spawn_start_on_main_frequency` (`aurora/src/lib.rs`) non sposta la radio all'avvio. `restore_on_exit` (true di base): `CatService::remember_original` ricorda frequenza e modo che la radio aveva quando Aurora l'ha trovata e `restore_original_if_chosen` (chiamato da `CatService::shutdown`) li rimette alla chiusura, solo con Hamlib e solo se la radio non trasmette. Caselle in `CatTab.tsx` (`cat.restoreStart`, `cat.restoreExit` in `radioextras.ts`).
- Perche': Toly (#73, TS-790) metteva la radio sui 1296 MHz, avviava Aurora e la frequenza tornava ai 2 metri (avvio voluto dal proprietario il 2026-11-14); richiesta del proprietario di avere anche il ritorno alla chiusura. Da provare con una radio vera: ordine di chiusura (il ripristino avviene prima di liberare la porta) e radio spenta.

## Nella 0.11.291 (pubblicata 2026-11-18T09:48:48Z)

- Livelli audio di Windows: nella 0.11.290 le chiamate di `set_attenuation` che il motore fa prima di ogni trasmissione (`engine-native/src/tx.rs`) passavano da `WindowsVolumePort::set_attenuation` e la scambiavano per una mossa dell'utente; dopo la prima trasmissione Aurora riscriveva il volume di uscita. Ora il gancio del cursore (`set_attenuation_hook` in `engine-native/src/lib.rs`) chiama `TxPort::level_changed` e solo quello imposta `user_set`; `set_attenuation` di routine non scrive finche' l'utente non ha mosso il cursore. All'avvio il cursore prende il volume letto da Windows. Da provare con una scheda audio vera (un altro programma audio che trovava i livelli a 100 dopo Aurora). Se persiste, chiedere all'amico versione, se il livello e' di registrazione o di riproduzione, e se ha toccato AUTO.

## Nella 0.11.290 (pubblicata 2026-11-18T08:48:40Z)

- Schermi stretti (1280 x 800): `lib/uiScale.ts` sceglie la dimensione dell'interfaccia dalla larghezza (`autoScale`: 65-100%, pulsante «Adatta»), `GridBoard.tsx` rimisura l'altezza (osserva anche il contenitore e rimisura dopo l'avvio), `RadioDisplayHeader.tsx` usa `CompactRadioDisplay.tsx` sotto i 1500 punti (`useNarrowScreen` in `lib/radioDisplay.ts`: frequenza, S/PO, potenza, ROS, TUNE; stili scelti solo su schermo largo); tolti dal selettore i tre display quasi quadrati (migrati a `icblue`, `ic7610`, `ft2000`); Comandi a griglia (`bozze.css`); `FreqPicker` senza «MHz».
- Livelli audio di Windows (`core/crates/aurora/src/lib.rs`, `WindowsVolumePort`): il volume di uscita si scrive solo dopo che l'utente ha mosso il cursore Pwr (`user_set`); all'avvio il cursore prende il livello letto da Windows. Il livello di ingresso cambia solo con il cursore o con AUTO (spento di fabbrica). Da provare con una scheda vera: segnalazione di un amico del proprietario (un altro programma audio trovava i livelli cambiati).
- **Lvl TX (2026-11-20, non rilasciato)**: il cursore Pwr si chiama Lvl TX e di fabbrica attenua il segnale (Aurora non scrive piu' il volume di Windows); casella «Lvl TX = volume di Windows» in Impostazioni › Audio per il comportamento di prima (`tx_level_windows`, `WindowsVolumePort::route`). Fatto: core, interfaccia, Guida in sette lingue, olandese ricalcolato (cinque frasi riscritte senza traduzione nl). Da provare con la radio di Pascal F4LHL (Icom): ALC a zero con Lvl TX giusto e volume di Windows fisso. Da fare al prossimo rilascio: voce nelle Novita' e versione.
- Finestre della pagina FT8 spostabili (`useEditLayout`, Impostazioni › Aspetto); `?vista=desktop` per le prove con il browser che simula il tocco.
- #75 (Luc): con BIGScan spento la finestra RX/TX diventava una striscia per un `alignSelf: 'flex-start'` dentro la riga `digi-winrow`; tolto in `DigiPage.tsx`.
- Prossimo passo: far provare la 0.11.290 sul PC a 1280 x 800; schede del contest mancanti (QSO per ora a colori, ritmo 10/50/100, Check Mult, Spot Alert).

## Nella 0.11.289 (pubblicata 2026-11-17T22:24:46Z)
**#71 (PTT acceso e spento a fine QSO)**: dal registro dell'autore (zip allegato) la fine QSO per decodifica tardiva (1,6 s dopo l'inizio del periodo) fermava la trasmissione appena partita (`QEvent::Stopped(QsoFinished|SingleQso)` -> `halt_tx` in `engine-native/src/tx.rs`). Correzione: `let_started_tx_finish` lascia finire la trasmissione gia' in onda (`decode_now_s` >= inizio del periodo). Verificato su **WSJT-X** (`C:\WSJT\wsjtx\bin\wsjtx.exe`, Ghidra 12.1.4, REA): `MainWindow::cease_auto_Tx_after_QSO` `0x14025d290` chiama solo `auto_tx_mode(false)`; solo «Halt Tx» (`on_stopTxButton_clicked` `0x140262290` -> `requestTxStop` `0x1401f4ae0`) interrompe con `stopTx`. In Ghidra servono `ghidra.startup_timeout_ms` alto (1500000) per `wsjtx.exe` (21 MB). Due prove di `engine-native` (`a_band_change_halts_tx...`, `switching_between_ft8_and_ft4...`) falliscono nella corsa completa sotto carico, anche senza questa modifica: da solo passano. Comandi: CSS in fondo a `bozze.css` (filetto tolto, meno margine); non misurato sullo schermo dell'utente. Diagnostica: `layout_salvato`, `riquadri_della_pagina`, `ultima_pagina_di_lavoro` in `uiDiagnostics.ts`.

## Nella 0.11.288 (pubblicata 2026-11-17T20:27:45Z)
VFO B (IC-756PRO sul VFO B dopo il riavvio): dal pacchetto E3099A60 dell'amico (versione 0.11.287, `diagnostica/cat-comandi.log`) risulta che `v` risponde «funzione non disponibile (-11)» e che l'unico comando di Aurora che tocca il VFO e' `get_vfo_info VFOB` (prima voce di `ITEMS` in `extras.rs`, 183 ms di traffico): Hamlib passa al VFO B e non sa dove tornare. Ora `Item::VfoB` non usa `get_vfo_info` se `v` fallisce (resta la frequenza dello split). **Verificato dall'amico con la 0.11.288 (2026-11-17): il problema del VFO B e' risolto.** Pacchetti sul servizio: tenuti solo quelli dell'amico (CA93C2A7, 95BC62CD, 2E27F421, E3099A60), gli altri cancellati il 2026-11-17.

## Nella 0.11.287 (pubblicata 2026-11-17T19:45:26Z)
Pacchetto di assistenza completo (`core/crates/aurora/src/diagnostica.rs`, `rig-control/src/trace.rs`, `app/src/lib/uiDiagnostics.ts`, rotta `POST /api/v1/diagnostics/ui-snapshot`, `docs/pacchetto-assistenza.md`): **dal prossimo pacchetto del PC dell'amico con IC-756PRO (VFO B e USB-D dopo il riavvio) si legge `diagnostica/cat-comandi.log`**: cercare i comandi `V`, `S`, `F`, `M`, `X` attorno a «# collegato a rigctld» e «# chiusura». Hamlib 4.7.2 non ha i modi dati per la famiglia 756 (`rigctl -m 3027 -u`): Aurora non vede USB-D (lo mostra «USB»); il ripiego su USB non si scrive piu' se la radio e' gia' in USB (`set_cat_mode`). VFO: `RigctldClient::vfo` accetta solo nomi di sole lettere (`is_vfo_name`). Da fare ancora: pacchetti vecchi sul servizio (R2 `aurora-rapporti` e D1 `pacchetti`) da cancellare tenendo quelli del PC dell'amico (installazione 60cef8a8) e di Luc (#75, 3B07DEE5); riscrivere i 12 commit pubblicati con autore `unknown <email personale>` e trailer (serve la regola del ramo main sospesa; vedi la memoria `aurora-trailer-da-togliere-dopo-release-0-11-286`).

## Nella 0.11.286 (pubblicata 2026-11-17T18:32:12Z)
Log del contest con le colonne Scambio inv./ric. (`exch_sent`/`exch_rcvd` nel log interno, migrazione 6 di `logbook`; `DigiLog.tsx`, colonne `exS`/`exR`), finestre della vista SSB con bordo viola e barra sfumata (`digi-ssb-contest.css`), ricerca delle radio senza il limite di 12 e in ordine di numero (`CatTab.tsx`), protezione del VFO con Hamlib (`extras.rs` per la lettura del VFO B, `spawn_start_on_main_frequency` in `aurora/src/lib.rs` all'avvio: NON provata su una IC-756PRO vera, segnalazione di un amico del proprietario; se si ripete servono pacchetto di assistenza e impostazioni), avviso del nominativo (il motore lo legge solo all'avvio: `diag.callsign.restart`), niente avvisi mentre la guida del primo avvio e' aperta (`lib/firstRunOpen.ts`), km centrati in Attivita' di banda. Segnalazioni aperte: #71 (PTT acceso e spento a fine QSO FT8, attende i dati dell'autore), #72, #75 (RX/TX in CW, attende finestra e pacchetto 3B07DEE5). Dizionario olandese riallineato con `scripts/export-nl.mjs` e `scripts/importa-nl.mjs` (prove verdi); il CAT con radio spenta scrive l'avviso al primo tentativo e poi ogni 30.

## Nella 0.11.285 (pubblicata 2026-11-17T15:07:36Z, stesso contenuto della 0.11.284 con numero nuovo) e nella 0.11.284 (pubblicata 2026-11-17T14:49:25Z)

- Mappa dei Satelliti (`components/sat/SatMap.tsx`, `lib/satView.ts`): un solo disegno per fotogramma da `requestAnimationFrame` (la vista sta in `viewRef`, lo stato React si aggiorna al massimo 8 volte al secondo), qualita' bassa in movimento e rifinitura a riposo, rotella ancorata al cursore (`wheelZoomFactor`), zoom interpolato in scala logaritmica, contatore dei tempi con `localStorage['aurora.sat.perf']='1'`. Non misurato: serve la prova del proprietario con il contatore.
- Hamlib (`rig-control/src/hamlib_rig.rs`): `split_mode_unsupported`, un solo info se la radio non sa scrivere il modo nel VFO di trasmissione ne' copiare A=B (`-1`, `-4`, `-11`); gli errori veri restano warn.
- Installatore (`desktop/src-tauri/installer/hooks.nsh`, `installer.nsi`): `AuroraPulisciVecchie`, rinomino in `hamlib.vecchia` se `hamlib` non si toglie, `AuroraFile` con riprova nella copia. Provato solo in un installatore isolato (sei scenari, un difetto trovato e corretto); non provati l'installatore vero, la disinstallazione, il modo con le finestre e un antivirus reale.
- Rapporti Cloudflare letti il 2026-11-17: `docs/report-cloudflare-2026-11-17.md`. Issue: #75 (Luc, UBA: serve la casella «club / sezione» nella riga di inserimento), #71 e #72 in attesa dei dati degli utenti.
- Anche nella 0.11.284: disposizione standard dei contest in SSB e in CW/RTTY/PSK (`defaultContestLayout` in `lib/contestLayout.ts`, spazio 6 px, `docs/contest-disposizione.md`), scheda Aurora BIGScan, sovrascritture dei contest inclusi (`contest/sovrascritture/<id>.json`), cluster con un solo pannello, log su una riga.
- Prossimo passo (0.11.285): schede del contest mancanti (QSO per ora a colori, ritmo 10/50/100 QSO con punti all'ora, Check Mult, Spot Alert: da chiedere al proprietario cosa fa il clic), risposta a Luc (#75: attendere log e modello CAT), PC lenti, seconda fonte spot WSPR, portale web.

## Nella 0.11.283 (pubblicata 2026-11-17T10:23:29Z)

- Satelliti: mappa Terra NASA (`app/public/maps/terra-giorno.jpg` e `terra-notte.jpg`, fonti in `docs/mappe.md`), scelte `:terra` e `:aurora` (`lib/satMapChoice.ts`), notte con le luci mascherata dal terminatore; zoom (`lib/satView.ts`: `MapView`, `fitView`, `fitBox`) automatico all'AOS con 4 modi (`zoom` in `lib/satAlerts.ts`), manuale (rotella, pizzico, pulsanti) e per paese (`lib/satCountries.ts`, `lib/countryCodes.ts`). Non provato: pizzico su touch, memoria sul telefono (due bitmap 8192), suoni AOS.
- DX Cluster: filtro «Digi» (`isDigitalMode` in `lib/cluster.ts`) e modo `DIGI` per gli spot che dicono solo digitale (`guess_mode` in `dx-cluster/src/spot.rs`). Gli spot WSPR del cluster web del proprietario non passano dai telnet collegati: servirebbe una seconda fonte web (da valutare).
- Macro: colori per tema (`lib/macroThemeColors.ts`, `aurora.digi.macrocolors`), tasti in tema chiaro con colore pieno e bordo (`digi-macro-style.css`).
- Prossimo passo: portale web, segnalazione a Microsoft dei file Hamlib (account bloccato), seconda fonte spot, campi di scambio dei contest (#75), VirusTotal della 0.11.283.

## Nella 0.11.282 (pubblicata 2026-11-17T08:50:15Z)

- PTT non spento: `core/crates/tx-native/src/ptt_release.rs` (scala di tentativi), `TxError::PttStuck`, `ptt_stuck` e `TxAlertKind::PttAbandoned` nello stato, `PttStuckBanner.tsx`; `CatPtt`/`AutoPtt` memorizzano il target e rifiutano il comando se la radio e' cambiata. Aperti in `docs/mancante.md`: thread di spegnimento separato.
- Status UDP: `NativeControls::decoding_guard()` (`engine-native/src/rig.rs`), Status immediato a ogni cambio; ipotesi per la #76 (mappa di Logger32 vuota), da far provare a Pere (la risposta la pubblica il proprietario).
- Antivirus: `remote-api/src/files_check.rs`, `GET /api/v1/health/files`, `FilesMissingBanner.tsx`, sezione `faq-antivirus` in `guide-faq.ts`. Barra non vista a schermo, non provata con un antivirus vero.
- Anche: schede Satelliti con minimi piu' bassi (`SAT_MINS` in `lib/satLayout.ts`), etichette `.cluster-split` leggibili in tema chiaro. Prossimo passo: risposte a Pere (#76, bozza pronta: va detto che la correzione e' nella 0.11.282) e a Luc (#75), VirusTotal, zoom automatico AOS. Restano: zoom automatico AOS, risposte a Pere (#76) e Luc (#75), VirusTotal.

## Nella 0.11.281 (pubblicata 2026-11-16T22:33:26Z)

Pagina Satelliti: avvisi all'AOS (`lib/satAlerts.ts`, `lib/useSatAlerts.ts`, suoni sintetizzati in `lib/satAlertSound.ts`, riquadro «Avvisi» in `components/sat/SatAlertsPanel.tsx`; anello e lampo sulla mappa), schede spostabili e ridimensionabili su griglia a 12 colonne (`lib/dashGrid.ts`, `lib/satLayout.ts`, `components/sat/DashGrid.tsx`; angolo per larghezza e altezza, maniglione sul bordo destro per la larghezza, `aurora.sat.layout` versione 5), cartina ridimensionabile con rapporto 2/1 fisso (`aurora.sat.mapw`), stella con il nominativo, scelta della mappa nelle Impostazioni, la casella decide (`lib/satSelection.ts`), tabelle dei passaggi come griglie spostabili (`aurora.sat.grid.passes` e `aurora.sat.grid.all`). **Provato a schermo** con eventi puntatore sintetici e un AOS simulato; **non provato**: i suoni (non si sentono dal codice), tema chiaro, tocco e tastiera, il tratteggio durante il trascinamento. **Da fare:** zoom automatico all'AOS (richiesto dal proprietario, non ancora scritto). **Aperti:** risposta a Pere (#76) e a Luc (#75), VirusTotal della 0.11.278-281, sito e portale con le novita' delle ultime versioni, e lo split che si riaccende da solo (indagine in corso: la radio e' condivisa sulla porta 4532; con lo split su «Nessuno» lo accende un altro programma collegato o la radio).

## Nella 0.11.280 (pubblicata 2026-11-16T21:03:53Z)

PSK/RTTY/CW: proporzione RX/TX limitata fra 0,25 e 0,8 (`aurora.digi.rxfrac.<modo>`), doppio clic sulla linea `digi-split` per ripristinare, finestra TX con etichetta (`data-label` e `::before` in `digi-dark.css`). SSB: riga di inserimento compatta (`digi-ssb-contest.css`, classi `ssbc-field--name` e `--qth`), etichette corte (`ssbc.rsts`, `ssbc.rstr`, `ssbc.serialSent`, `ssbc.serialRcvd`). Il proprietario ha detto «no» alla pubblicazione immediata: la 0.11.280 si fa quando lo chiede. **Verificato a schermo** il ripristino della proporzione e la TX con etichetta; **non rivisto a schermo** l'ultima modifica delle etichette (solo tsc e test).

## Nella 0.11.279 (pubblicata 2026-11-16T20:27:17Z)

Nome, QTH e locatore del nominativo si compilano da soli da QRZ.com o HamQTH (se la ricerca e' configurata) nella riga SSB e nel pannello QSO di CW, RTTY e PSK (`lib/useCallsignAutofill.ts`, collegato una volta in `DigiPage.tsx`); il testo scritto a mano non si tocca. FT8/FT4 e SSTV lo facevano gia'; il log manuale (`LogEditDialog`) no. Guida Logger32: basta il server UDP primario (2237), niente seconda voce in Integrazioni sulla stessa porta (#76, Pere: i decode arrivano a Logger32 ma la mappa resta vuota; in attesa della sua risposta dopo aver tolto la voce doppia «BBLogger»). **Da provare dal vivo:** la ricerca con QRZ o HamQTH veri. **Aperti:** risposta a Pere (#76) e a Luc (#75, il log del contest e' ora da zero), VirusTotal della 0.11.278 e 0.11.279, sito e portale con le novita' delle ultime versioni.

## Nella 0.11.278 (pubblicata 2026-11-16T18:32:24Z)

Contest in SSB: il log del contest riparte da zero (solo QSO della gara, `inSet` in `DigiPage`, attivazione in `aurora.ssb.contest.since.<id>`, «Mostra tutto il log» / «Solo il contest»), doppi e moltiplicatori dentro la gara, scheda Statistiche per contest (`ContestStatsCard`), QSO precedenti per banda, suggerimento della zona CQ dal nominativo (`lib/contestEntry.ts`), date di inizio e fine validate con proposte per CQ WW DX, CQ WPX e JIDX (`windowProposals` in `lib/textContest.ts`), JIDX CW e SSB corretti (lo scambio inviato resta vuoto: lo imposta l'utente). Il log si aggiorna subito dopo ogni QSO (`invalidateQueries(['log4om'])`). DX Cluster: riga gialla delle DXpedition con testi scuri. **Provato a schermo** con il JIDX SSB: log da zero, numero QSO, suggerimento CQ, QSO precedenti. **Non provato:** segno DUPE dal vivo (il core scarta un QSO identico nello stesso minuto), statistiche con moltiplicatori, tema chiaro, 1366 px, Cabrillo. **Incidente:** per errore e' stato fatto `git checkout` di `DigiPage.tsx` con modifiche non committate; sono state rifatte dagli aiutanti (su file con modifiche non committate non si usano `git checkout`, `restore` ne' `stash`).

## Nella 0.11.277 (pubblicata 2026-11-16T17:27:01Z)

Vista SSB della pagina PSK/RTTY/SSTV: tre colonne (Log, «Inserisci QSO» su un rigo, DX Cluster) con colonne ridimensionabili, senza cascata, testo RX/TX e barra in basso (`SsbContestView.tsx`, `digi-ssb-contest.css`); con un contest scelto dal menu Contest (sempre presente) la riga diventa N. QSO, Nominativo, Seriale inviato (progressivo), Seriale ricevuto, con punteggio, moltiplicatori e DUPE. Pulsanti di banda diretti (bande scelte nelle Impostazioni, prima frequenza di fonia IARU R1 in `lib/bandplan.ts`; i 6 m a 50,150 MHz sono una scelta da confermare). Barra delle macro con ingranaggio (Barra singola / multipla a 4 righe, carattere dei pulsanti in tempo reale), righe Ctrl/Shift/Ctrl+Shift, Vista › Righe delle macro; voci dei menu inibite per modo (`lib/menuModi.ts`). Rotore compatto, mobile, pallina di stato, clic sulla rosa. Display radio FT-2000 senza vuoto a destra. Guida Logger32 (#76, mappa UDP: si apre a mano con Ctrl+J, porta 2237). **Da provare dal vivo**: riga con contest attivo, clic sulla rosa del rotore, frequenza dei 6 m, tema chiaro, 1366 px. **Decisione** (#75): QSO sempre nel log generale con tag; il log del contest riparte da zero come vista filtrata per tag e data; Cabrillo con verifica da li' (vedi `docs/mancante.md`).

## Nella 0.11.276 (pubblicata 2026-11-16T12:00:14Z)

Passaggi di tutti i satelliti, rotore (spostamento minimo, correzione azimut, PSTRotator solo invio), rilevamento dell'antivirus (nome nei rapporti, area `antivirus` per i file tolti; servizio su Cloudflare gia' aggiornato con migrazione 0003). **A ogni controllo dei rapporti** guardare anche gli antivirus: query in `servizi/rapporti/README.md` › Antivirus. Domanda di firma gratuita a SignPath Foundation inviata dal proprietario: quando accettano, adattare `windows.yml` (compilazione su GitHub e firma) e controllare che l'installatore mostri la privacy.

## Nella 0.11.275

Finestrina «Aggiornamento in corso» della copia di aiuto e avviso «Aurora aggiornata» alla riapertura (`1d503ffa`, `update_window.rs`). Provata a vista con il test ignorato; non provata con un aggiornamento vero. Da mettere nelle Novita' alla prossima versione.

## Stato al 2026-11-16 (0.11.275 pubblicata 2026-11-16T11:19:35Z; 0.11.274 2026-11-16T10:38:07Z)

Modo SSB nei modi testuali per log e contest in fonia (#74, `9348e234`, Guida `25e14d1c`). Issue aperte: #71 (PTT acceso e spento a fine QSO FT8: servono versione, impostazioni e registro dell'utente; meccanismo probabile: trasmissione pianificata e fermata da una decodifica tardiva, `engine-native/src/tx.rs` `collect_events`), #72 (IC-707 su macOS), #74 (contest, risposta da dare ora che la 0.11.274 e' uscita). Cartella locale pulita: solo `main`.

## Stato al 2026-11-15 (0.11.273 pubblicata 2026-11-15T21:53:39Z)

Installatore: se la cartella scelta non si scrive (Program Files senza diritti di amministratore) propone `%LOCALAPPDATA%\Aurora`; se Aurora resta aperta durante l'aggiornamento chiede di chiuderla e Riprova. Nati dai rapporti Cloudflare del 2026-11-15 (`docs/report-cloudflare-2026-11-15.md`). Da provare dal vivo: PC senza diritti di amministratore e Aurora che non si chiude.

## Stato al 2026-11-15 (0.11.272 pubblicata 2026-11-15T20:32:28Z)

- Menu Opzioni › Auto CQ nei modi testuali (Avvia, Ferma, Testo… = macro AutoCqText, Pausa… salvata in localStorage `aurora-digi-acq-delay`). RBN: spenta di base (Impostazioni › DX Cluster), elenco «Sentiti dalla rete RBN» sotto BIGScan con etichetta RBN; il proprietario non la vuole nel pannello DX Cluster.

## Stato al 2026-11-15 (0.11.271 pubblicata 2026-11-15T20:14:27Z)

- Tutti i comandi delle macro integrati in 15 famiglie (`macroLang.ts`, `DigiPage.tsx`, rotte `/digi/winkeyer/send`, `/digi/play`, `/digi/record`), tabella in `docs/macro-comandi.md`, Guida delle macro per famiglia; GRABCALL, PREVCALL, PREVRSTS. EXEC/EXT esclusi. Comandi «al piu' vicino» (packet, DDE, MFSK/Hell, KiwiSDR) e significati dedotti (ACALL, OM, BROADCASTMODE, CORRECTCALLTX, TEST, frasi INT) da verificare dal vivo.

## Stato al 2026-11-15 (0.11.270 pubblicata 2026-11-15T19:02:36Z)

- 0.11.269: copia automatica giornaliera (`backup::automatic`, `copie-automatiche`, ultime 28) al posto dei Backup.bat; EXEC tolto dai file delle macro all'avvio.
- 0.11.270: macro `<PREVNRS>`, `<QSOBEFORETIME>`, `<QSOBEFOREDATE>` dal log del contest (rotta `ammesso` con `precedente` e `ultimo_mandato`); approssimazioni dei contest completate (turni, potenza, `band_score`, `hour:`, `skip:`), restano solo quelle senza dati.

## Stato al 2026-11-15 (0.11.268 pubblicata 2026-11-15T18:12:22Z)

- 90 contest dei modi testuali, 89 con `[Score]`/`[Script]` dai regolamenti (gruppi 3A/3B/3C, prove in `tests/regole_contest_3*.rs`); tolti 59 contest non piu' organizzati (elenco e motivi in `docs/contest-regole.md`). Le macro saltano in silenzio i comandi non eseguiti (EXEC, MACROS, sconosciuti).
- Prossimi: comandi macro mancanti (`PREVNRS`, `QSOBEFORETIME`, `GRABCALL`, `WAVE`); approssimazioni elencate in `docs/contest-regole.md`; `dom/KDA.dom` e `dom/s50p13dc.dom` non piu' usati.

## Stato al 2026-11-15 (0.11.267 pubblicata 2026-11-15T17:33:24Z)

- Contest dei modi testuali: `[Score]` vero per 30 contest (fonti in `docs/contest-regole.md`), sezione `[Script]` in Rhai (`contest/src/script.rs`, RAEM come esempio), importa/esporta `.zip` (`pacchetto.rs`), 14 contest nuovi con macro (`docs/contest-macro-scelte.md`), `<EXEC:...>` ignorato con avviso, barre Rx/Tx della cascata con una richiesta per volta.
- Prossimi: punteggio dei contest rimanenti (gruppo 3); comandi macro non eseguiti (`PREVNRS`, `QSOBEFORETIME`, `GRABCALL`, `WAVE`); moltiplicatori `call` non mostrati nella griglia; fonti di 9 contest da ricontrollare.

## Stato al 2026-11-15 (0.11.266 pubblicata 2026-11-15T15:51:57Z)

- Impostazioni solo dal PC: rotte delle Impostazioni che sul client remoto ricadono sulla pagina principale (`isSettingsRoute`, `remoteFallback` in `lib/route.ts`), scritture delle impostazioni limitate con `settings_only_from_this_pc` (elenco in `docs/api.md`). Scelta del proprietario: il telefono resta per trasmettere e ricevere, niente altro (anche «PC lento», JT65/Q65 e profondita' di decodifica non vanno dal telefono).
- SSTV pannello a schede largo al massimo 440 px; token Club Log sempre validato; test `ui_settings` allineato al limite di 1 MB; Gen.Msgs tolto dai README.

## Stato al 2026-11-15 (0.11.265 pubblicata 2026-11-15T14:00:59Z)

- Aggiornamenti solo con «Installa» (tolto il conto alla rovescia di `UpdatePoster.tsx`; la locandina si apre solo a richiesta). Client remoti (`REMOTE_HOST` in `lib/phone.ts`): niente scritture delle impostazioni del PC, ascolto SSTV/testo solo con «Ascolta» (`ListenGate.tsx`). `POST /commands/band` con `mode` cambia il modo (`apply_mode_change`). «TX = RX» interruttore (`cw_wpm_follow_rx`). SSTV a schede. Intestazione «by IU3WJQ». README e Guida allineati al codice.
- Aperti, da decidere col proprietario: Gen.Msgs (citato solo nei README, pulsante assente); varchi delle Impostazioni da remoto (`#/finestra/impostazioni` non bloccata, `PUT /engine-settings` accettato dalla rete di casa); consiglio del README inglese di aprire Aurora dall'IP.

## Stato al 2026-11-15 sera (0.11.264 pubblicata 2026-11-15T13:23:16Z)

- CW ibrido «preciso / veloce» (`cw_hybrid.rs`, evento `rx_prov`, veloce di base: testo provvisorio dalla catena in 0,57 s, definitivo dell'attuale); BIGScan RTTY con stazioni nascoste confermate per testo proprio e trattenuto, primo carattere incerto tolto davanti a parole fisse; riquadro unico `PwrSplitBox` in tutti gli stili; istogramma SSTV veloce (`rx_hist` ogni 100 ms) senza spettro; +/- sui WPM; maniglia BigScan indipendente; Informazioni in tre colonne da 44rem.
- Restano fuori: testo provvisorio CW in BigScan, decodifica di sequenza nei canali BigScan RTTY (perde una stazione su `tmp30515`).

## Stato al 2026-11-15 pomeriggio (0.11.263 pubblicata 2026-11-15T12:20:27Z)

- WinKeyer (`rig-control/src/winkeyer.rs`, `mode-digi/src/service/keyer.rs`, Impostazioni › Stazione › WinKeyer, `docs/winkeyer.md`); sintonia SSTV a cursore ±200 Hz e istogramma dei livelli come nei programmi classici (`mode-sstv/src/decoder.rs`, `SstvTuning.tsx`, rotta `POST /sstv/rx/tune`); RTTY con decodifica di sequenza nel ricevitore singolo (spenta nei canali BigScan: perdeva UC0A); #67 #68 #69 #70, PR #65.
- Spenti nel codice e da riprendere: catena CW completa `cw_chain.rs` da sola (piu' lettere inserite; ora usata solo come testo provvisorio del ricevitore CW veloce, `cw_hybrid.rs`, predefinito; BigScan ancora senza), catena RTTY `rtty_chain.rs`.
- Disco C: pieno durante il lavoro: tolte le cartelle target e i worktree finiti (ora ~108 GB liberi). Tenere pulite le copie di lavoro degli aiutanti e `Aurora-fix-remoto/core/target-*`.

## Stato al 2026-11-15 (0.11.262 pubblicata 2026-11-15T10:16:01Z)

- QTC del WAE (`contest/src/qtc.rs`, `QtcDialog.tsx`, macro `<QTCSEND>`/`<QTCREAD>`, `docs/contest-qtc.md`); RTTY squelch con presenza della portante, BIGScan velocita' dal passo dei cambi (`lattice()`) e reverse con LSB; CW pezzi coerenti di 2 unita' e Kalman sulle pause (759/938 nominativi); cascata con finestra di Nuttall come WSJT-X (`audio-spectrum/analyzer.rs`); display della radio largo zero corretto (#55 #60, `components.css`), tasto SPLIT, pulsante CAT (`CatSwitch.tsx`); `dial_hz` iniziale solo senza CAT; bande accese nel selettore, misure per modo (`modeSizes.ts`), SSTV ridisposta.
- In corso: ricevitori completi CW (`cw_chain.rs`) e RTTY (decodifica di sequenza + catena completa) costruiti dal materiale di analisi, confrontati sul banco con gli attuali.
- Manuali del portale con versione propria (`data/manual-version.json`, `npm run manual:bump`), ora 1.1.

## Stato al 2026-11-15 mattina (0.11.261 pubblicata 2026-11-15T02:23:53Z: decodifica RTTY e CW)

- **RTTY** (banco a caratteri `core/crates/mode-digi/tests/rtty_caratteri.rs`, `docs/banco-rtty.md`): squelch che si apre dal primo carattere affidabile (`ProbSquelch::push`): CER nei silenzi 10,6 -> 3,0 %, trasmissioni separate -5 dB 9,4 -> 2,9 %, AWGN -5 dB 10,3 -> 7,6 %. BIGScan: finestra mobile ogni 0,5 s e spostamento/velocita' per canale (`tmp30515`: stazioni 16 -> 17/26, ritardo 6 -> 5 s), limite di carico anche sui ricevitori (`RttyMulti::set_max_lanes`).
- Provati e tolti: modello per tono Rice (meglio con stazione forte vicina e fading piatto, peggio con fading selettivo e a -8/-10 dB), «3 buoni su 4» per aprire, chiusura dopo 2 deboli, prova piu' lunga in BIGScan, tono solo (falsi canali), nominativi noti (nessun guadagno).
- **CW** (banco `tests/banco_cw.rs`, `docs/banco-cw.md`): in `cw_bayes.rs` confine fra classi di pausa dalle densita' pesate, pugno 0,8, priori pause 67/27/6, miscela ampiezze: CER medio 13,8 -> 11,5 %, tasto a mano 17 -> 9 %, nominativi 693 -> 716/938. Neo: 1 carattere falso in 40 min di rumore (prima 0), entro il rumore della misura.
- Aperti: RTTY con forte vicina, -10 dB e fading selettivo; crepitii (8-10 falsi/min nei silenzi); CW a -3/-6 dB e QSB veloce. Prossimo passo possibile: Viterbi su start/bit/stop per l'RTTY.

## Stato al 2026-11-15 (0.11.260 pubblicata, sito aggiornato)

- **0.11.260** (2026-11-14T23:01:32Z): frequenza in tempo reale (Hamlib: sola frequenza ogni 160 ms in sintonia, 300 ms da ferma; `hamlib_rig.rs`), barra delle macro di nuovo a 4 righe (F1-F12 solo sulla prima, niente Ctrl/Shift), macro spostabili col trascinamento a pressione lunga (`swapMacros`, Annulla), Guida alle macro a due colonne a colori, pagina macro delle Impostazioni a gruppi, ricerca nell'elenco contest, tendina del modo dalla barra in basso, campi contest solo con contest attivo, icone in BIGScan.
- Aperti: issue #55 (display assente, forse finestra staccata chiusa: `app/src/lib/detached.ts`); HRDLOG.NET «server non raggiungibile» da un utente (catena TLS o firewall suo); nominativi noti in BIGScan (aiutante al lavoro, prossima versione); riallineare `Aurora-fix-remoto` a `origin/main`.

## Stato al 2026-11-14 notte fonda (0.11.259 pubblicata, sito aggiornato)

- **0.11.259** (2026-11-14T22:12:47Z): barra delle macro dei modi testuali con una sola riga F1-F12 (`ControlBar.tsx`; tolte righe Ctrl/Shift, ingranaggio e voce `digi.bar.extended`), Guida aggiornata.
- Stazioni deboli accanto alle forti in BIGScan RTTY: provati il residuo spettrale con conferma nel tempo e la cancellazione della forte (SIC) - nessun guadagno sul banco, non tenuti (patch in scratchpad della sessione: `residuo-rtty.patch`, `sic-rtty.patch`). Ipotesi: fronti reali arrotondati, servirebbe stimare la forma dei fronti.

## Stato al 2026-11-14 tarda sera (0.11.258 pubblicata, sito aggiornato)

- **0.11.258** (2026-11-14T19:20:08Z): punteggio, statistiche e Cabrillo del contest dei modi testuali (`contest/src/punti_testo.rs`, `GET /api/v1/contest-testo/punteggio` e `/cabrillo`, `ContestStatsDialog.tsx`; codici moltiplicatori 0-5 scelti in Aurora, i 135 predefiniti hanno punti a zero); cattura dello scambio senza l'eco; BIGScan RTTY: canali storti spostati sul segnale vero (`rtty_multi.rs`), fine parola dopo 1 s (`skim/rtty.rs`), verifica alla prima lettura certa dopo CQ (`calls.rs`) -> ritardo 9 -> 6 s, presenza 46 -> 52%; split: registro di ogni comando e avviso «split acceso fuori da Aurora» (`hamlib_rig.rs`, `rd.splitExternal`), Doppler solo con la pagina Satelliti aperta; niente falsi blocchi con la finestra in secondo piano (`crash.rs` `BACKGROUND`).
- Split da solo del proprietario (FT-2000): nessuna strada di Aurora trovata; un altro programma aveva il CAT acceso sulla stessa radio (spento il 2026-11-14). Se torna, leggere il registro (righe `split:` e avviso esterno).
- Rapporti su Cloudflare (`wrangler d1`/`r2` da `servizi/rapporti`): letti fino al 2026-11-14, nessun altro difetto di Aurora.

## Stato al 2026-11-14 notte (0.11.257 pubblicata, sito aggiornato)

- **0.11.257** (2026-11-14T18:09:51Z): menu Contest dei modi testuali (`contest/elenco.rs`, `contest_testo_api.rs`, `ContestListDialog.tsx`, `textContest.ts`; 135 contest in `desktop/src-tauri/contest/` copiati al primo avvio in `<dati>/contest/` con `macros/` e `dom/`; formato in `docs/contest-file.md`, convertitore `contest/examples/converti_contest.rs`); macro ripremuta che riparte da capo (`transmit_restart`, `macroRestart.ts`), tasto F sui pulsanti, menu del tasto destro nella finestra TX (`TxMenu.tsx`), finestra TX unica, BPF visibile, suggerimenti in BIGScan, testo breve sul nominativo della cascata, tolti gli spot «nel mio Cluster» dall'interfaccia (`local_spots` resta spento nel core); Hamlib macOS universale dai sorgenti (`scripts/hamlib-macos-universale.sh`, Hamlib 4.7.2 + libusb 1.0.30, cache in `macos.yml`) perche' la 0.11.256 e' uscita senza pacchetto Mac.
- Rilascio dalla copia pulita `Aurora-rel256` (worktree): `Aurora-fix-remoto` resta la cartella di lavoro degli aiutanti; dopo il rilascio va riallineata a `origin/main`.
- Da verificare: compilazione macOS della 0.11.257 (prima prova di Hamlib dai sorgenti); tutto con la radio.

## Stato al 2026-11-14 sera tardi (0.11.256 pubblicata, sito aggiornato)

- **0.11.256** (2026-11-14T17:17:31Z): macro verificate una per una (`macroPlan.ts`, `docs/macro-verifica.md`), Guida alle macro nel menu Aiuto (`MacroGuide.tsx`, dal catalogo `macroCommands.ts`), Pausa/Cerca nella finestra RX, `<TXE>` col campo `carrier`; BIGScan: modo DXpedition (`dxPileup.ts`, `PileupMarks.tsx`), testo sulla cascata (`wfText.ts`), spot dal tasto destro (`spotPrefill.ts`), «Nel mio Cluster» (`local_spots`, prima `band_spots`, spento di base), falsi CQ con pausa di 2,5 s, banco preciso e sintonia/rilettura 2 s (ritardo 12 -> 9 s); RBN (`dx-cluster/src/rbn.rs`, server spot locale 7300, niente invio alla rete); CAT: modo del VFO di trasmissione in split (`write_split_mode`, `X`), split DX dimenticato alla riconnessione se la radio si e' spostata, ripiego USB senza modo dati (PR #58 IU0PWE); riapertura su pagina/frequenza/modo (`last_dial.rs`, `lastPage.ts`); issue #56 #59 #60 #61; HRDLOG.NET; maniglie e separatore RX/TX.
- Rilascio fatto da una copia pulita (`Aurora-rel256`, worktree staccato) perche' nella cartella di lavoro c'erano modifiche in corso: in una copia nuova servono `desktop/src-tauri/hamlib-bundle/` e `cty/cty.dat` (ignorati da git) copiati a mano.
- In corso: macro che ripartono da capo a ogni pressione e tasto F sui pulsanti (aiutante); maniglia della cascata sul bordo (`d67a8ed9`, gia' fatta, non pubblicata).
- Da fare: risposte alle issue date al proprietario; prova con la radio di tutto; registrazione CW vera per il banco.

## Stato al 2026-11-14 notte (0.11.255 pubblicata, sito aggiornato)

- **0.11.255** (2026-11-14T14:08:57Z): contest nei modi testuali: segni di BIGScan dal contest attivo (`logStatus.ts`, `useContestLog.ts`, `contest_valuta`: DUPE solo nella finestra della gara, NEW/B/M/BM/MULT/DXCC; B4 fuori gara), `<QSONR>` in gara = ultimo numero del contest + 1 (`contestNextSerial`), `POST /log/qsos` con `X-Aurora-Origine` e `exch_sent`/`exch_rcvd` (prima i QSO dei modi testuali entravano nel contest per caso), macro `<IGNOREBEFORE>`/`<PERIOD>` (`logWindow.ts`); BIGScan RTTY sintonia fine ±15 Hz nei canali (`rtty_multi.rs`, ritardo 12 -> 9 s; banco allineato a `band.rs`: interruzioni 5 -> 7); display della radio acceso se la radio risponde (`RadioDisplayHeader.tsx`, #55); HRDLOG.NET nominativo maiuscolo e codice senza spazi (`upload.rs`); riga fra RX e TX (`maniglie.css`); Guida aggiornata.
- Prove fallite su BIGScan (fondo locale ±300 Hz, picchi ±1, storia 2,5 s, soglia -0,35): nessun miglioramento, tolte. Il riferimento a canale singolo del banco mescola le stazioni vicine: va reso piu' preciso prima di riprovare le deboli (RZ3Z, UC0A).
- Da fare: risposta a Luc (#55) e a F1DSZ (HRDLOG.NET) date al proprietario da mandare; icone Pausa e Cerca nella barra RX; registrazione CW per il banco.

## Stato al 2026-11-14 sera (0.11.254 pubblicata, sito aggiornato)

- **0.11.254** (2026-11-14T12:39:20Z): Aurora BIGScan piu' affidabile: CQ solo per il nominativo accanto a CQ/TEST/QRZ (`calls.rs`, `cq_count`), sostituisce un CQ solo un altro CQ letto 2 volte, rinfresco con pezzi del nominativo (`partial_sighting`, 180 s per i CQ), canali RTTY aperti piu' a lungo e memoria dei nominativi per frequenza (`CallCache`, 240 s), CQ mai sfrattato, radio spostata di poco = frequenze ricalcolate (`Msg::Shift`, `band_follow`); la stazione CQ letta nella finestra RX o nelle finestre RX in piu' compare sempre (`cqCallOf`); doppio clic su una stazione, segni NEW/B4 dal log (`logStatus.ts`). Banco di prova `tests/bigscan_misura.rs` (`AURORA_BIGSCAN_WAV`, `AURORA_BIGSCAN_MODE=cw`): su `tmp30515.wav` interruzioni 37 -> 13, solo banda 57 -> 17, ritardo mediano 7 -> 12 s. Macro: `<TX>` senza `<RX>` resta aperta, `<RXANDCLEAR>` pulisce a fine trasmissione, `<TXE>`/`<RXE>`, `<QSONR>` progressivo a 3 cifre (`contestSerial.ts`), `<NRS>`/`<NRR>`. Barra sopra la finestra RX (Copia, Salva, Pulisci); maniglie tutte uguali (`maniglie.css`).
- Prossimo: stazioni deboli accanto alle forti (RZ3Z, UC0A mai trovate: fondo locale ±300 Hz in `rtty_multi.rs`, correlazione su 2-3 s, toni vicini, AFC dei canali), ritardo 12 s da riportare sotto 7; icone Pausa e Cerca nella barra RX; registrazione CW per il banco.

## Stato al 2026-11-14 pomeriggio (0.11.253 pubblicata, sito aggiornato)

- **0.11.253** (2026-11-14T11:28:30Z): finestre RX in piu' (fino a 4, `PUT /api/v1/digi/extra-rx`, `rx_text` con `rx_id`, `ExtraRxWindow.tsx`, indicatori trascinabili) e menu del tasto destro sulla cascata; Aurora BIGScan di base solo chi chiama CQ (`radarAnswering`), ruoli dei contest (dopo TU, prima di TEST/QRZ), stazioni non attive tolte (`stale`/`gone`, `SUPERSEDED_S` 10 s, 60 s, 3 min), ripulitura al cambio banda o di 50 Hz (`BAND_CLEAR_HZ`), interruttore ON/OFF e Pulisci, etichette col colore della stazione; RTTY: caratteri uno alla volta, a capo dopo 1,5 s di pausa (`RX_GAP_MS`), a capo persi recuperati, AGC confermato dal rumore, banda 200-4000 Hz; clic su spot RTTY/PSK porta alla pagina dei modi testuali (`lib/spotMode.ts`); display «RADIO NON COLLEGATA» in testo (#55); Informazioni in tre colonne.
- In attesa: seconda registrazione RTTY del proprietario (`Desktop	mp30515.wav`, ~110 MB, era ancora aperta da un altro programma); testi dell'altro programma per il confronto; registrazioni CW vere.

## Stato al 2026-11-14 (0.11.252 pubblicata, sito aggiornato)

- **0.11.252** (2026-11-14T09:57:44Z): **Aurora BIGScan** (CW e RTTY: elenco delle stazioni sicure a destra delle finestre RX/TX, una riga per nominativo, colore per stazione, colonne spostabili, maniglia; `RadarList.tsx`, `lib/digiBand.ts` `radarStations`, `skim/rtty.rs`, `band_rtty`), in primo piano nelle Novita' (`featured`). CW di tutta la banda: rumore non ingannato da due vicine (`SIDE_OCCUPIED` 2,5; 30 stazioni 23 -> 30/30), recupero degli ultimi 2,5 s, frammenti e letture di un carattere nascosti (`shadows`, `shadow_across`), 64 ipotesi sui canali forti. RTTY: cancello interno a squelch spento (rumore 182 -> 18 car/min), AFC dalla fase (aggancio < 1 s fino a 50 Hz), riscalatura dopo un salto di guadagno. Niente piu' PowerShell (SetupAPI, IP Helper, WLAN/NLM/Firewall COM, registro, copia di aiuto `--applica-aggiornamento` con `aggiornamento.log`).
- Da provare con la radio: tutto, in particolare aggiornamento senza PowerShell, «Consenti Aurora», RTTY accanto all'altro programma sul file del proprietario (`tmp12856.wav`, 2 stazioni a 1976 e 2806 Hz di centro).
- Prossimo: confronto RTTY con l'altro programma sui suoi testi (i due file di testo forte e debole), fading veloce RTTY (confermare il calo sui canali del rumore), registrazioni CW vere.

## Stato al 2026-11-14 (0.11.251 pubblicata, sito aggiornato)

- **0.11.251** (2026-11-13T23:07:29Z): Club Log col token personale dell'API REST v1 (`ClublogToken`, `POST /api/v1/qso` con Bearer, prova con `GET /me/usage`; vecchio `realtime.php` se manca il token); messaggi di HRDLOG.NET tradotti; frasi del registro delle decisioni tradotte (continente escluso, Abilita Tx riacceso, stato nuovo); VFO B delle Yaesu senza `get_vfo_info` letto da `get_split_freq` (FT-2000, non provato con la radio); impostazioni allineate a sinistra sotto le etichette; doppione ADIF tolto da Programmi collegati. Compilazione: `tsc --singleThreaded` e `CARGO_BUILD_JOBS=4`, il PC finiva la memoria. Non provato il token con Club Log vero.
- Da fare: pagina Informazioni (proposte A/B/C in attesa); PowerShell nascosto sostituibile con API di Windows in 4 punti (proposta, non chiesta); risposte a #47, #52, #53 date all'utente.

- **0.11.250** (2026-11-13T19:09:28Z): spot dal cluster telnet a scelta al posto di DXSummit (`dx_cluster::SpotVia`, `send_spot` aspetta la ripetuta `DX de <noi>`), nome di accesso e password per server (password mai rimandata all'interfaccia, `has_password`); azimut e distanza nel riquadro del nominativo; `auto_connect` acceso al primo collegamento (`auto_connect_offered`); stato di Club Log con `code`/`params` tradotti; verifica DXSummit filtrata per DX. Non provato con un cluster vero con password. Bozza di risposta all'issue #47 data all'utente.

- **0.11.249** (2026-11-13T16:56:56Z): spot verificati sull'elenco pubblico di DXSummit dopo l'invio (`reporting/src/dxsummit.rs` `check`/`listed`; controlli a 10, 30, 60, 90 s; stato «spot pubblicato» o «accettato ma non pubblicato»). Non ancora provato con uno spot vero.

- **0.11.248** (15:24 UTC): titolo della finestra con nome e autore, radio spenta a 14 segmenti, Tune e Log QSO sempre visibili, log vuoto con Aggiungi QSO e Importa ADIF.

- **0.11.247** (2026-11-13T14:37:32Z): notch e passabanda in frequenza davanti ai ricevitori PSK/RTTY/CW (`mode-digi/src/notch.rs`, oltre 60 dB, ritardo circa 0,3 s solo con filtri accesi), AFC vincolato, display spento grigio, Spot DX nella barra del cluster. FT-2000 del proprietario: l'accensione via CAT non funziona nemmeno a mano su COM10 senza VSPE (porta CAT della radio non alimentata da spenta): serve una presa comandata.

- **0.11.246** (14:01 UTC): Nuovo log vuoto protetto (archivio controllato o cancellazione con doppia conferma e `confirm: DELETE`), Log QSO apre la finestra compilata senza QSO in corso (SWL), Accendi con Cat4OM (`powerOnRadio`) e RTS alto per le Yaesu (proprietario: FT-2000 con microHAM e VSPE, da provare), display spento con radio che non risponde, Attivita' di banda a colonne spostabili, opzioni del cluster in FT8, ricerca della Guida con risultati. In studio: notch e AFC dei modi testuali.

- **0.11.245** (12:38 UTC): spia di salute della stazione (`GET /api/v1/health/resources`, `HealthLight.tsx`), avvisi e tooltip su fondo bianco, avviso di ingresso debole solo se dura 90 s e mai in TX/TUNE, niente falsi crash da `stderr.log` (`rumore_stderr`), tabella delle frequenze con righe sbagliate tolte una per una (`keep_valid`), menu delle Impostazioni a tasselli, scheda DXpedition «scuro nitido», TX durante TUNE, «Spegni split» sempre visibile, messaggio vero sul database di Log4OM (#51, causa ancora da capire).


- **0.11.244** (09:23 UTC): «Spegni split» accanto al display (`POST /cat/split-off`, `force_split_off`); aggiornamento in un solo
  giro (niente `/R`, riapre solo lo script, l'agente nel tray aggiorna da se' con Aurora chiusa: si vede dall'aggiornamento SUCCESSIVO);
  ascolto SSTV e modi testuali con battito e conteggio delle pagine, satelliti e forme dei nominativi solo quando servono; modalita'
  «PC lento» (`dec.slowPc`, porta `aurora_core::idle`); CW di tutta la banda -40% di CPU e ricampionamento del motore 20 volte piu'
  leggero (risultati identici); file salvati tenuti `aud.keep` giorni (7) e `ALL.TXT` che ruota; guardiano del thread CAT; avvisi
  `LagWatch` sul bus; coda audio limitata a 30 s; frequenze CW/SSTV/RTTY/PSK dai piani ARI (`app/src/lib/bandplan.ts`); manuale
  completo (`guide-*.ts`, copertura in `docs/copertura-guida.md`); separatore dei periodi e Tx nella finestra RX durante il QSO
  (#38, #42); schede DXCC nuovi al massimo due con «Spegni questi avvisi»; avviso di aggiornamento nella lingua di Aurora/Windows.
- **Attivita' pianificata** `aurora-rapporti-cloudflare` (ogni 8 ore, a PC acceso): scarica e svuota i rapporti, corregge i
  difetti piccoli con commit senza push.
- **CPU**: il PC e' quello di lavoro del proprietario: al massimo 12 thread in tutto, agenti compresi (con tre agenti `-j 2` ciascuno).
- **Non provato**: tutto quanto sopra con la radio vera e a schermo; Linux solo compilato da GitHub.
- **Prossimo passo**: prove del proprietario; idee rimaste in `docs/prestazioni-motore.md` e `docs/prestazioni-interfaccia.md`
  (cascata e vetro da misurare in WebView2); QPSK; tabella ITU ufficiale per la convalida dei nominativi.

## Stato al 2026-11-12 tarda notte (0.11.243 pubblicata, sito aggiornato)

- **0.11.243** (22:42 UTC): lo split di FT8 non si riaccende alla riconnessione del CAT ne' al cambio di frequenza, lo Split DX si spegne
  lasciando i modi testuali, PSK/RTTY/CW/SSTV trasmettono in simplex (Hamlib, OmniRig, TCI, Cat4OM); «Accendi» con 75 s di attesa e la
  sequenza `PS1;` a due coppie (0 e 1,2 s, 3,7 e 5,2 s); cambi di frequenza ravvicinati che non si contendono la radio; «Rimanda i QSO
  persi» anche per il server UDP secondario (`UDP2Resend`, acceso di base); CW con AGC dal fondo del rumore (banco 26,5% → 23,4%);
  Linux: uscita audio con buffer di 40 ms e formato a 1-2 canali, registro dei cambi di scheda, testi del volume senza «Windows», pagina
  Radio a 1024 px; avviso sull'indirizzo CI-V in esadecimale; ritocchi dell'interfaccia dei modi testuali, del DX Cluster e dei menu.
- **Dal registro del proprietario** (FT-2000 su COM11, Hamlib, «Accendi» e «Spegni»): dopo l'aggiornamento il CAT e' rimasto 3,5 minuti
  senza collegarsi perche' rigctld veniva chiuso dopo 15 s a meta' accensione; il proprietario dice che «Accendi» non accende la radio.
  La sequenza nuova e' da provare dal vivo: se non accende ancora, servono le righe `rig_control::wake` del registro.
- **Non provato**: tutto quanto sopra con la radio vera e, per Linux, su Linux (compilato solo su Windows; aspettare la conferma
  dell'utente con la IC-7100).
- **Prossimo passo**: prove del proprietario; collegare il rilevatore dei click ai canali di tutta la banda; QPSK.

## Stato al 2026-11-12 notte (0.11.242 pubblicata, sito aggiornato)

- **0.11.242** (21:02 UTC): QSO del log da correggere e cancellare (#47); pannello Segnali con «Controllo» che si salva, Pulisci, altezza e
  nominativi sulla cascata che spariscono (#50); SSTV con cronologie a strisce e freccette, «Trasmetti» in cima con F1 ed Esc; testo
  trasmesso solo nella finestra TX, doppio Esc che spegne l'Auto CQ, tasto RX/TX che ferma; avvio sui Windows 10 prima della 2004
  (`WerRegisterAppLocalDump` cercata quando serve); avvisi della stazione non nei primi 20 s; commento intero nel DX Cluster.
- **In pausa**: ramo `lavoro/cw-riprova` (commit `wip`, non verificato): AGC, velocita' e QRN largo del CW; l'ultima media del banco letta
  era 23,6% contro 26,5%, ma senza la tabella riga per riga. Da riprendere: finire le varianti, controllare che nessuna riga peggiori.
- **Da provare con la radio** e a schermo: tutto quanto sopra e' passato solo dalle prove automatiche.

## Stato al 2026-11-12 sera (0.11.241 pubblicata, sito aggiornato)

- **0.11.241** (19:34 UTC): CW con decodificatore probabilistico, noise blanker, rumore dalle bande laterali, aggancio rapido, «TX alla
  velocita' ricevuta» e anti-click della cascata; tutta la banda CW e PSK nella pagina (pannello Segnali, nominativi sulla cascata, convalida
  dei nominativi a livelli, spot solo locali); PSK con probabilita' e blanker; RTTY oltre (tre catene, squelch statistico, finestra
  Impostazioni RTTY); trasmissione aperta; SSTV (FSK ID, modo senza VIS, log con ricerca); «Rimanda i QSO persi» per i log chiusi
  (`qso-export`, modulo `waiting`); correzioni dai rapporti (aggiornamento fermato dal Restart Manager, WebView2 installato da Aurora, rigctld
  che chiude il collegamento, PulseAudio); schermata chiara dell'installatore quando non riesce; dimensione del testo (#49).
- **Rami di lavoro**: tutto e' su `main`. In corso sul ramo `lavoro/cw-riprova` (da `a549a2fd`): nuovo tentativo su AGC a due inseguitori,
  velocita' robusta con antidisturbo e QRN largo, scartati nella 0.11.241 perche' peggioravano il banco. Entra solo se migliora.
- **Decisioni del proprietario**: gli spot dei decodificatori restano locali (niente invio in rete); verso Log4OM solo UDP con la coda,
  niente scrittura nel suo archivio.
- **Portale**: il ramo `dist` di `aurora-portal` vuole il messaggio di commit `sito: <sha di main>`, altrimenti `aurora-sito aggiorna` lo
  rifiuta; da Git Bash l'alias ssh `aurora-portale` si usa con `/c/Windows/System32/OpenSSH/ssh.exe`.
- **Da provare con la radio** (proprietario): CW a tutta banda, PSK, RTTY, SSTV, coda dei QSO con Log4OM; la schermata d'errore
  dell'installatore non e' stata vista girare.
- **Risposte da dare** (le scrive il proprietario dalle bozze): #46, #27, #40, #48, #49 e F1DSZ.
- **Prossimo passo**: unire `lavoro/cw-riprova` se i numeri migliorano; collegare il rilevatore dei click ai canali di tutta la banda; QPSK.

## Stato al 2026-11-12 pomeriggio (lavoro dopo la 0.11.239, solo commit locali)

- **Commit locali, non pubblicati** (il proprietario li prova prima con l'installatore di prova `Aurora-PROVA-impostazioni.exe` sul Desktop):
  Impostazioni ad albero; pagine Scansione e memorie (PSK·RTTY·CW) e Contest; voci doppie tolte; issue #40 (righe non confermate piu'
  chiare e a righe); avviso sulla frequenza Tx fuori da 200-2700 Hz; Guida aggiornata ai percorsi nuovi. **Niente push finche' il proprietario
  non dice che la prova e' andata bene.**
- `dist-release\` contiene l'installatore di prova, non quello della 0.11.239: alla prossima release si ricompila.
- Le Novita' delle voci nuove si scrivono al rilascio (la prova vuole la voce piu' recente della versione in uso).
- Risposta per la #40 (avllucken) da scrivere quando la versione esce: la dara' il proprietario.

## Stato al 2026-11-12 (0.11.239)

- **0.11.239**: revisione completa con le sue correzioni (sicurezza dell'accesso locale, Split DX, XIT, SQLite) e i punti lasciati
  (Tune subito, Clear Avg di JT65, testi del cluster, PTT con la porta occupata, coda di WSPRnet, pulizie); Linux per la #41
  (audio PulseAudio/PipeWire, CAT con l'Hamlib 4.5 di Debian 12, PTT del Dummy, porte /dev); mappa del mondo dei satelliti (#27).
- **Installatore Windows compilato e firmato sul PC** con `scripts/rilascio-windows.ps1`: la chiave (`%USERPROFILE%\.aurora-signing\aurora.key`,
  id `92a42c12993d4c27`) l'ha ritrovata il proprietario ed e' verificata contro `desktop/updater.pub`. Il lavoro Windows di GitHub salta la
  compilazione se la release ha gia' installatore e `.sig` (lavoro `pc`); Linux e macOS restano su GitHub (gratuiti, repository pubblico).
  Procedura in `docs/rilascio.md`.
- **Da provare con la radio** (proprietario e utenti): Split DX, WSPR/FST4W con split, Cat4OM, OmniRig; su Linux KE4OZD (IC-7100, FT-857D,
  audio) — la #41 resta aperta finche' non conferma.
- **Lavori di una volta**: la cartella di compilazione del Rust su `D:\build\aurora-target` (C: era pieno); `cargo` con `-j 12` e prove con
  `--test-threads=2`, una compilazione alla volta.

## Stato al 2026-11-11 mezzanotte (0.11.238 pubblicata, sito aggiornato)

- **Pubblicata**: 0.11.238 (tre lavori verdi, installatore firmato da GitHub; sito compilato sul PC e caricato, mostra la 0.11.238).
- **Su `main` dopo la 0.11.238**: riquadro del nominativo con LoTW solo se la stazione lo usa (verde, senza data), anche dal DX Cluster con l'elenco dell'ARRL (`lotw_last_upload` in `/api/v1/callsign`); voci per importare le macro da altri programmi tolte dal menu di PSK·RTTY·CW; barra di stato con un solo carattere; rotore meno della meta' nella sua pagina e nella finestra a parte; donazioni con PayPal (badge, sezione nei README, `SOSTENITORI.md`, `.github/FUNDING.yml`).
- **Chiave di firma degli aggiornamenti** (superato il 2026-11-12, vedi sopra): non e' sul PC (`%USERPROFILE%\.aurora-signing\aurora.key` manca) ed e' solo nei segreti di GitHub, che non si rileggono: l'installatore firmato lo fa il lavoro Windows di GitHub (gratis, repository pubblico). Il proprietario non vuole compilare su GitHub: serve la sua copia della chiave per compilare sul PC.
- **Attenzione, cartella principale**: in `Desktop\Progetti\Aurora` il ramo `main` e' fermo alla storia vecchia (`1bce05e4`, 692 commit diversi da GitHub); si lavora in `Aurora-fix-remoto` da `origin/main`. Allinearla solo con il via del proprietario (copia in `backup-pre-rewrite`).
- **Un'altra sessione pusha su `main`** (es. `d7847b06`): prima di ogni push `git fetch` e `git rebase origin/main`.

## Stato al 2026-11-11 notte tardi (0.11.237 pubblicata, 0.11.238 pronta su `main`)

- **Su `main`, per la 0.11.238**: Split DX dal cluster («UP 5», «5UP», «10up», «DN», «QSX»); Cat4OM con la scoperta automatica (porta di gestione 17300, come Decodium); olandese come settima lingua (interfaccia, Guida, Novita', installatore; al primo avvio Aurora parte nella lingua di Windows); radiofaro FST4W e invio facoltativo a WSPRnet (spento di fabbrica); porta COM occupata con il nome del programma; OmniRig lento come avviso; PC lento con «Passa a Normale»; modo degli spot dal band plan IARU Regione 1; via QSL nell'elenco DXpedition; barra di stato PSK/RTTY/CW senza data e ora; testi senza confronti con altri programmi; macro in tema chiaro.
- **In corso**: griglie evidenti e colonne spostabili nella Finestra RX e nella scheda Log (ramo `lav-griglie`, worktree nello scratchpad).
- **Fatto fuori dal codice**: i 22 tag v0.11.134-155 spostati sui commit della storia pulita (nessun ramo o tag su GitHub cita piu' l'assistente); su Cloudflare cancellate le righe d'errore fino alla 0.11.235.
- **Repository privato `wifi75/Aurora-sviluppo` cancellato** (2026-11-11). **Da fare**: pubblicare la 0.11.238 e il sito (compilato sul PC); prove dal vivo con la radio (WSPRnet, radiofaro FST4W, Cat4OM con la scoperta, porta occupata).

## Stato al 2026-11-11 notte (0.11.237 pubblicata, altro gia' su `main`)

Aurora si chiama ora «Aurora — Multimode Ham Radio Station» (it: «stazione radioamatoriale multimodo»): il README e la descrizione dell'installatore lo dicono.

- **Pubblicata**: 0.11.237.
- **Su `main` dopo la 0.11.237** (non ancora in una release): FST4 e FST4W (periodi 15 s-30 min, verificati con `jt9.exe`), WSPR-2 (beacon, percentuale di Tx, verificato con `wsprd.exe`; invio a WSPRnet facoltativo, spento di fabbrica), JT65 sottomodi B/C con media dei periodi, MSK144 a 30 s e «Sh», caselle di decodifica di WSJT-X, rotore unificato, colonne del Contest, icone nuove, schermata di avvio, tempi di accensione dell'FT-2000, campo del rapporto, clic destro su QRZ, testo dell'SSTV centrato, LoTW nel riquadro del nominativo, installatore in sette lingue (con l'olandese).
- **In lavorazione**: Split DX dal DX Cluster (UP/DN/QSX nel commento dello spot), interfaccia in olandese (settima lingua, dizionario a parte; resta da fare) e scoperta delle radio di Cat4OM dalla pagina.
- **Un solo ramo, `main`**: `sviluppo` e' stato fuso e non serve piu'; il repository privato `Aurora-sviluppo` si cancella quando il proprietario da' il via. Il merge di `main` in `sviluppo` non e' piu' un passo da fare.
- **Il sito** (aurora-portal) si compila sul PC e si carica a mano: i minuti di GitHub Actions dei repository privati sono finiti.
- **Prossimi passi**: chiudere Split DX, olandese e scoperta Cat4OM; beacon FST4W in trasmissione; prove dal vivo con la radio (FST4/FST4W, WSPR, JT65 B/C, MSK144, OmniRig via COM, Cat4OM, TCI, SSTV); `wip-fox` (Fox in Attivita' di banda) se ancora aperto.

## Stato al 2026-11-11 sera (0.11.236 pubblicata)

- **Release del 2026-11-11**: 0.11.233, 0.11.234, **0.11.235** (decodificatori allineati a WSJT-X 3.3.0-beta6, OmniRig via COM, Cat4OM nativo, split con XIT, Linux #41, ALSA, TCI) e **0.11.236** (DX Cluster con griglia e colonne spostabili, orologio, avvisi chiudibili con le Impostazioni aperte, riquadro QRZ leggibile, doppioni tolti). Per la 0.11.236 controllare che i lavori Windows, Linux e macOS siano verdi e che la release abbia gli installatori.
- **Lavoro interrotto**: ramo `wip-fox` nel repository privato (`sviluppo/wip-fox`, commit `0edd6517`): righe Fox (DXpedition) leggibili in Attivita' di banda e finestra Rx, e filtro `chkspecial8var` di WSJT-X sui messaggi Fox dubbi. Da finire, provare (`cargo test -p ft8`, vitest su DecodeList) e portare su `main`.
- **Ramo `sviluppo`** (privato): contiene tutto `main` (merge `8093bcaa`) piu' SSTV con la pagina nuova (spettro sottile, log compatto, impostazioni nella categoria SSTV con l'ingranaggio). L'installatore di sviluppo il proprietario lo compila a casa.
- **Da provare dal vivo** (proprietario): OmniRig via COM (FT-710, FTdx3000), Cat4OM con Log4OM (CT1BXX), split Fake It, TCI, MSK144 vero, pagina SSTV. Lo split dal cluster e' gia' provato e funziona.
- **Prossimi lavori**: finire `wip-fox`; poi JT65 sottomodi B/C e media `avg65`; FST4/FST4W (circa 3000 righe); WSPR (circa 2200 righe); MSK144 periodo 30 s e «Sh» in trasmissione; caselle del menu Decode di WSJT-X che mancano (Auto Clear Avg, Include averaging, Aggressive, Single decode, F Tol per JT65/JT9/MSK144); guida «Collegare la radio» per modello (FTdx101, FT-991A, IC-7300...); traduzione olandese (#43, file da preparare per ON7KEC).
- **Segnalazioni aperte con risposta da dare**: #38 (OmniRig, F1DSZ), #41 (Linux), #43 (olandese). Ogni risposta agli utenti chiude con il P.S. della stella su GitHub.

## Stato al 2026-11-10 notte (0.11.232 in pubblicazione, ramo sviluppo con la CW)

- **Release del 2026-11-10**: 0.11.229 (rotore, filtri «sulla banda»), 0.11.230 (ricerca nelle Impostazioni, schede in gruppi, griglie non confermate), **0.11.231** (Contest: TX e Auto CQ sempre accesi, «Salta Tx1» da solo con i messaggi standard), **0.11.232** (PTT: una lettura mancata della radio non scarta piu' il PTT e, se non parte, un avviso a schermo dice il motivo; menu dei contest in ordine di data; il «×N» non sposta la bandiera). La 0.11.232 e' stata creata alle 22:29 UTC: **controllare che i tre lavori (Windows, Linux, macOS) siano verdi e che la release abbia `Aurora-Setup.exe` e il `.sig`**, poi correggere l'ora vera nelle Novita' (`gh release view v0.11.232 --json publishedAt`), commit e push.
- **Origine del fix PTT**: commenti su Facebook di Jeff Cadwalader (IC non specificato, «cambia frequenza ma niente PTT») e Michał Siarkiewicz (IC-7300). Nel codice: `HamlibRig::set_ptt` scartava il PTT con `failures > 0` e l'errore di trasmissione finiva solo nel log. Resta da vedere il pacchetto di assistenza per sapere se era proprio quella la causa; risposta pronta ai due utenti (Impostazioni › Radio, «Test PTT», 0.11.232, pacchetto di assistenza).
- **Ramo `sviluppo`** (repository privato `wifi75/Aurora-sviluppo`, remote `sviluppo`; mai su `origin`): contiene tutto `main` piu' i modi nuovi. Va fuso con `git merge main` dopo ogni versione di `main` (non farlo mentre una compilazione e' in corso: un nuovo push la annulla; l'installatore si rifa a ogni push, circa 11 minuti, release `sviluppo-ultima`). Nome ambiguo: usare `refs/heads/sviluppo`.
- **Contenuto di sviluppo**: SSTV (`mode-sstv`), PSK31/63, RTTY e CW (`mode-digi`, CW in `cw.rs`), Satelliti (`mode-sat`), menu nuovo con i modi in alto (un solo pulsante «PSK · RTTY · SSTV»), pagina PSK/RTTY/CW (macro a blocchetti con i sottoinsiemi, selettore di banda IARU in `app/src/lib/bandplan.ts`, log dello stesso log del pannello Log, DX Cluster con «Spot DX», cascata con rombi RX/TX e scala in kHz veri, opzioni della Vista, tema chiaro e scuro). Il dettaglio e' in `docs/ripresa.md` sul ramo `sviluppo`.
- **Da provare dal vivo** (le prove con la radio le fa il proprietario): CW (trasmissione e decodifica), spot a DXSummit, log di Log4OM con frequenza/nome/QTH, split con «up N», rotori.
- **Da fare**: linguetta «Mappa» nella colonna di destra; opzioni della Vista rimaste fuori (Word mode, stick cursor, IMD); comandi di macro dell'SSTV; split vero della radio per «up» oltre 3,5 kHz; fondere `sviluppo` in `main` solo quando il proprietario lo decide.
- **Attenzione alla pagina PSK/RTTY**: la finestra e' `position: fixed; inset: 0` (`styles/digi-log-corta.css`) perche' `100dvh` non segue lo zoom di `lib/uiScale.ts`; non tornare a `100dvh`.

## Ramo sviluppo - stato al 2026-11-10 notte (PSK, RTTY, CW, SSTV, Satelliti)

- **Pagina «PSK · RTTY · SSTV»** (un solo pulsante in alto; il modo si cambia dal menu «Modo»): macro a blocchetti (4 righe x 12), selettore di banda con le frequenze IARU (`app/src/lib/bandplan.ts`), RX/TX con divisore regolabile, colonna di destra con linguette DX Cluster e Log (bordo trascinabile, colonne a scelta), cascata sotto RX/TX (rombi RX blu e TX rosso, scala in kHz veri, 0-5 kHz o 0-3,5 kHz dal menu Vista, altezza regolabile), opzioni della Vista, tema scuro e chiaro, tutta in una schermata (`styles/digi-log-corta.css`, finestra fissa).
- **Log della pagina** = lo stesso del pannello Log (`getLog4om`, fino a 500 QSO, numero = totale - posizione); freq/nome/QTH letti dalle colonne di Log4OM se ci sono; il log interno ha le colonne `name` e `qth` (schema v4).
- **CW**: `core/crates/mode-digi/src/cw.rs` (modulatore Morse a tono, decodificatore a soglia adattiva, 7 prove), `DigiMode::Cw`, `cw_wpm`, comando `<WPM:n>`. Da provare con la radio.
- **Macro**: editor con i sottoinsiemi (testo, programma, frequenza, Auto CQ, macro, modo, rotore, RTTY, SSTV, CW, HELL); eseguiti: `<FQ>`, `<SPLIT>`, `<ROTOR>`, `<EDITMACRO>`, `<ALARM>`, `<ACQ>`, `<WPM>`; gli altri sono in elenco come «non ancora».
- **Cluster**: PSK nei modi, «Solo nuovi», clic = sintonia sul tono e «up N» in split, pulsante «Spot DX» con la finestra «Invia spot» (rotta `/api/v1/spot/dxsummit` con `freq_khz` e `info`).
- **Da fare**: provare tutto dal vivo (CW, spot, log di Log4OM, split); linguetta «Mappa»; opzioni della Vista che mancano (Word mode, stick cursor, IMD); SSTV: comandi di macro; auto split con la radio per «up» oltre 3,5 kHz; fondere `sviluppo` in `main` solo quando il proprietario lo decide (main resta alla 0.11.231, con il fix PTT e il menu dei contest ordinato non ancora in una release).

## Stato al 2026-11-10 sera (0.11.228 pubblicata, rotori su main)

- **0.11.228** (15:23 UTC): cascata ferma in trasmissione e spettro spegnibile (UN7MBV), filtro «Spotter» del DX Cluster (Luc ON7KEC), MSK144 centrato, chiusura di tutte le finestre con la principale, consigli ARS/split per FT-991 e IC-9700.
- **Rotori (P3) su main, non pubblicati**: Hamlib `rotctld` (`rig-control/src/rotator.rs`), API `/api/v1/rotator`, pagina «Rotore» con rosa dei venti, scheda «Rotore» nelle Impostazioni, Guida. `rotctld.exe` aggiunto all'installatore. Da provare dal vivo prima della prossima versione.
- **Regola sugli altri modi (P2)**: da programmi chiusi solo funzioni e parametri, mai codice; il codice da QSSTV, fldigi, slowrx (GPL) o scritto da zero.
- **In attesa del proprietario**: avviso nel pannello Log sulla fonte (Log4OM) per Luc; blocchi delle finestre al cambio di primo piano.

## Stato al 2026-11-10 pomeriggio (0.11.227 pubblicata)

- **Menu nuovo deciso** (P4 in docs/tasks.md): modi in alto (FT8, VHF, CW, RTTY, PSK, SSTV, Satelliti), a destra le pagine del modo e Log · Mappa DX · Propagazione · menu ⚙, niente seconda riga, volume e tema nella barra in fondo. Prototipo approvato in docs/prototipo-menu-modi/index.html: e' il prossimo lavoro grosso, da fare a passi (prima FT8/VHF con le pagine di oggi spostate).
- **0.11.227**: eta' dei messaggi da quando compaiono (come GridTracker), righe Tx all'inizio del periodo, nominativo di chi trasmette in grassetto, Finestra RX con colonne fisse. Contest di prova: DARC FT4, 2026-11-10 19:00 UTC (Luc ON7KEC). Progetti grossi in docs/tasks.md: satelliti, altri modi, rotori; MSK144 a 30 s (issue #27).

- **0.11.226** (12:19 UTC): al primo avvio con un installatore vecchio prima la locandina dell'aggiornamento e poi la guida iniziale; Finestra RX con colonne larghe quanto il contenuto (messaggio mai tagliato); cartolina del tray senza i passi.

- **Versioni del giorno**: 0.11.220-0.11.225. Ultime: 0.11.223 eta' dei messaggi e messaggi nuovi in fondo, Filtri divisi per elenco, Automazioni che salvano «nuovo nominativo» e «griglia su nuova banda», agente del tray nella lingua di Aurora; 0.11.224 PTT Hamlib con risposte in ritardo, chiusura ordinata del TCI; 0.11.225 «CQ e novita'» (bandierina, nominativo, paese, due colori chiari, via dopo 15 minuti, scorrimento), riquadro RX che in trasmissione diventa «STOP TX (Esc)», Auto con conferma del modo reale sulla Stazione, suono del QSO a log a scelta, mappe che seguono la banda della radio, OmniRig mancante detto con il link.
- **Release**: si creano con --latest=false e diventano «ultime» da sole dopo installatore e firma (passo di windows.yml): provato con 0.11.223-0.11.225.
- **cty.dat**: copia di riserva in desktop/cty-riserva usata quando country-files.com non risponde.
- **Rapporti su Cloudflare**: letti e cancellati (errori e pacchetti) il 2026-11-10 pomeriggio. Da guardare: blocco della finestra sulla 0.11.224 (i3-1005G1, 36 s dopo «finestra in primo piano»).
- **Richieste degli utenti** (Luc ON7KEC e altri): le decide il proprietario, non vanno fatte tutte; prima si propongono.
- **In attesa**: Aurora in giapponese e cinese, da decidere solo se arrivano utenti dagli annunci (gli annunci si scrivono in italiano, inglese, giapponese e cinese).
## Stato al 2026-11-10 mattina (0.11.222 pubblicata)

- **0.11.222** (08:39 UTC): l'aggiornamento non resta piu' bloccato su «la release non ha il pacchetto di installazione con la sua firma».
- **Release mai vuota per le copie installate** (dopo l'errore «la release non ha il pacchetto di installazione con la sua firma» della 0.11.221 e 0.11.222): la release si crea SEMPRE con `gh release create vX.Y.Z --latest=false` e senza file; il lavoro `Windows` allega installatore e `.sig` e solo allora la segna come ultima (passo «Release segnata come ultima solo con installatore e firma» di `windows.yml`). Mai creare una release come ultima senza installatore e firma. Se il lavoro Windows fallisce, la release resta non ultima e le copie installate restano sulla versione precedente, completa. In piu', dalla 0.11.222, Aurora richiede la release a GitHub se al momento di scaricare manca il pacchetto (`updater::prepare_locked`).

- **0.11.221** (08:03 UTC): solo dipendenze aggiornate (`cargo update` nei due workspace; interfaccia con react-query 5.107, zustand 5.1, vite 8.3.7, vitest 5.1; servizio dei rapporti con wrangler 4.159 e workers-types 5). jsdom resta alla 30 e typescript del servizio alla 5.9.

- **0.11.220** (07:52 UTC): «Ripristina Hamlib» su Windows, trasmissione che riparte da sola se la scheda audio della radio manca all'avvio, finestre a parte aperte fuori dal thread della finestra, decodifica con un processore libero, primi messaggi dopo il cambio di banda, voce della Guida «Che computer serve», tipo di computer nei rapporti.
- **Installatore da GitHub**: la release si crea senza file; il lavoro `Windows` (`windows.yml`) compila l'installatore, lo firma con il segreto `AURORA_TIMBRO_AGGIORNAMENTI_AURORA_KEY` e allega `Aurora_x.y.z_x64-setup.exe`, `.sig`, `Aurora-Setup.exe` e `SHA256SUMS.txt`. Sul PC dell'ufficio la chiave di firma non c'e': `scripts/rilascio-windows.ps1` li' non si puo' usare.
- **Identita' dei commit**: nel repository `user.email` deve essere l'indirizzo noreply di GitHub; il 2026-11-10 autore e committer dei commit dopo `ef9510e9` sono stati riscritti (push forzato di main).
- **Prossimo passo**: prove del proprietario al DARC FT4 (2026-11-10 19:00 UTC); controllare i rapporti dopo la 0.11.220 (blocchi delle finestre a parte, PC lenti con il tipo di computer); poi i lavori di `docs/tasks.md`.

## Stato al 2026-11-09 notte (0.11.219 pubblicata)

- **Versioni del giorno**: 0.11.205-0.11.218. Ultime: 0.11.215 nuovo stato in Auto e pagina Contest ordinata, voce degli avvisi rifatta con Kokoro (licenza Apache 2.0, `scripts/genera-voce-kokoro.py`; le frasi di Edge/Ava sono state tolte); 0.11.216 correzioni dai rapporti di Cloudflare (PowerShell con percorso intero per l'installatore, niente falsi blocchi/panic in chiusura `crash::exiting`, rapporti senza «radio non collegata»); 0.11.217 paesi non confermati come DXCC nuovo (`newDXCCUnconfirmed`), Mappa DX con banda in uso, «solo chi mi sente», linea rossa in TX; 0.11.218 Contest da tastiera (F1 Auto CQ, F2-F4, Esc), «Come funziona», avvisi sopra le Impostazioni, ricarica che resta sulla pagina.
- **0.11.219** (23:00): mappa del pannello Log semplificata (sempre banda in uso, riga di comandi corta) e passaggio automatico al Log quando un QSO va a log; selettore banda/frequenza piu' leggibile; Contest spento pulito; correzione della pagina vuota ricaricando (mappa di calore su tela larga 0) e `PageGuard` contro gli errori di pagina; alleggerimenti (righe visibili prima, `content-visibility`, misure ferme con la finestra nascosta). Tentativo di tenere viva la Stazione nascosta **tolto**: dava la pagina nera (non riprovarlo senza prove serie).
- **Contest**: nel log del contest entrano solo i QSO avviati dalla pagina Contest (intestazione `x-aurora-origine: contest` su reply e CQ) e fatti durante la gara; chi non fa il contest entra a 0 punti (errore, fuori dal Cabrillo). Da provare dal vivo al DARC FT4 del 2026-11-10, 19:00 UTC.
- **Conteggio delle persone**: `rapporti::codice_persona` (SHA-256 del nominativo sul PC) nel segnale `/v1/attivo`; sul Worker HMAC con `PERSONA_SEGRETO` (impostato) e colonne `persona` su D1 (aggiunte). Rapporti degli errori svuotati il 2026-11-09 dopo la lettura.
- **Riordino**: in `D:\progetti` restano `Aurora` e `Aurora-archivio` (LEGGIMI dentro); copie di lavoro git chiuse (i rami restano), compilazioni vecchie di `D:uild` cancellate.
- **Test**: i test del motore toccati passano; nell'interfaccia 39 test falliscono gia' da prima del 2026-11-09 (elenco confrontato con la 0.11.212), da sistemare a parte.
- **In attesa**: issue #9 (TCI «Audio silent», avllucken: chieste versione e pacchetto); risposte di Luc sulle sue richieste; frase «Virus free» dei README da rivedere (VirusTotal non si fa piu').
- **Blocchi della finestra**: su Cloudflare arrivano «programma bloccato (finestra)» da altre installazioni subito dopo ogni release (uno «finito dopo 29 s»): non spiegati, servono un pacchetto di assistenza o il minidump.
- **Prossimo passo**: prove del proprietario al DARC FT4 (2026-11-10 19:00 UTC); controllare i rapporti dopo la 0.11.219; poi i lavori di `docs/tasks.md`.

## Stato al 2026-11-08 mattina (passaggio ad altro account)

- **Lavoro solo in locale**: il proprietario ha chiesto di NON pubblicare (niente push, tag, release, sito) finche' non dice «pubblica». Sul ramo `main` locale ci sono circa 17 commit non pubblicati dopo la 0.11.194 (versione nei file: 0.11.195).
- Installatore locale pronto: `D:\build\Aurora-0.11.1\dist-release\Aurora_0.11.195_x64-setup.exe` (09:20). Il proprietario lo installa e prova; la grafica la vede dal vivo copiando `app/dist` in `%LOCALAPPDATA%\Aurora\dist`.
- Fatto oggi: correzione del cambio banda che rimbalzava (0.11.194 pubblicata), JT4 A-G, Q65-30A, MSK144 + MSK40 + tempo reale, bande da 2200 m a 24 GHz, selettore delle frequenze (pulsanti delle bande tolti), accesso senza codice in casa di base, frequenza principale all'avvio, «R + locatore» come conferma, tasti del display che non diventano grigi, zoom del display opaco sopra il display fisso, spot a DXSummit nel registro. Dettagli in `memory.md` (sezione «Modi e bande nuovi») e `TODO.md`.
- Compilare con 9 processi (`jobs = 9` in `~/.cargo/config.toml`): con 18 la memoria finiva e Aurora si chiudeva.
- **Prossimo passo**: aspettare le prove del proprietario; poi Q65 (periodi e sottomodi selezionabili, media, AP, EME), FST4/FST4W e WSPR, sistemare le prove che falliscono gia' (elenco in `TODO.md`).
- File non tracciati da lasciare: `app/tsconfig.tsbuildinfo` (generato) e `docs/report-cloudflare-2026-11-06.md` (il proprietario non ha ancora deciso se committarlo).

## Prompt da incollare nella sessione nuova

```text
Riprendi il lavoro su Aurora nella cartella D:\progetti\Aurora.
1. Leggi AGENTS.md, docs/ripresa.md (la sezione «Stato al 2026-11-10 notte» in cima), memory.md e docs/tasks.md.
2. Allineati: git fetch su origin e sul remote sviluppo; main e sviluppo erano allineati ai remoti (main 57890cee alla 0.11.232, sviluppo 6e1c1499).
3. Dimmi in breve a che punto siamo; poi, per prima cosa: controlla che l'installatore di sviluppo abbia la CW nel menu Modo e rispondi a chi segnalava il PTT (la 0.11.232 e' pubblicata); poi riprendi dai punti «Da fare» di docs/ripresa.md.
Regole: niente riferimenti all'assistente nei file e nei commit (niente trailer); mai il locatore vero ne' email personali; push e release autorizzati (AGENTS.md); un push su sviluppo annulla la compilazione in corso, quindi raggruppali; dopo ogni versione di main, git merge main in sviluppo; le prove dal vivo con la radio le faccio io; i testi sono in sei lingue.
```

## Stato

- **Ultima versione**: **0.11.191** (2026-11-07). Dal 2026-11-07 sera (risposte alle issue e agli utenti): 0.11.189 dimensione del testo (Impostazioni > Generali, Ctrl +/-/0, `lib/uiScale.ts`) e Impostazioni che non si chiudono con Salva (issue #3 e #4, chiuse con #1 e #2); 0.11.190 pagina Log con «Aggiungi QSO», «Elimina selezionati», «Svuota il log» (`POST`/`DELETE /api/v1/log/qsos`) e Guida di BBLogger rifatta dai PDF ufficiali; 0.11.191 bande senza CAT (`ManualDialHook`, `frequenza-a-mano.txt`). Da provare dal vivo: tutte e tre. Fino alla 0.11.188 il lavoro e' nel CHANGELOG.
- Prima, 0.11.151 (2026-11-05): pubblicata dal PC (Mappa DX come GridTracker2, finestre a parte senza cornice, display su un altro schermo, pagina Log, orologio con correzione a un clic). Su `main` dopo la 0.11.151, non ancora rilasciati: ingresso e uscita audio che si riprendono da soli se la radio sparisce durante l'uso, guardiano dei blocchi piu' preciso, correzioni Linux (Pwr, aggiornamenti, OmniRig), storico permanente delle installazioni nel servizio dei rapporti.
- Versioni precedenti, del 2026-11-04:
  - 0.11.102: Aurora indipendente (non legge piu' nulla da JTDX o WSJT-X; `cty.dat` e Hamlib dentro l'installatore), guida al primo avvio (`app/src/components/FirstRun.tsx`, compare finche' manca il nominativo), servizi pubblici accesi di fabbrica, BBLogger, prima versione Linux.
  - 0.11.103: modo, periodo e frequenza si pubblicano anche senza ingresso audio (`engine_native::publish_status`, avviato dal core prima del motore).
  - 0.11.104: copia delle impostazioni (Impostazioni > Assistenza, `core/crates/aurora/src/backup.rs`: zip sul Desktop, ripristino in `<dati>\ripristino` applicato al prossimo avvio da `backup::apply_pending`); suono dello splash (`--autoplay-policy=no-user-gesture-required` in `desktop/src-tauri/src/lib.rs`).
  - 0.11.105: schede audio ritrovate anche con nomi vecchi (`match_device_name` / `is_default_name` in `audio-spectrum` e `tx-native`: `\xNN`, «2- », scheda tra parentesi, «Default Input Device»); OmniRig assente detto in chiaro.
  - 0.11.108: con due schede USB uguali vince il nome esatto (`match_device_name`); confermato dal vivo.
  - 0.11.109: «Clicca per configurare» al posto di display e log vuoti (`SetupCta`).
  - 0.11.110: niente decodifiche false nel rumore (picco del periodo < 0,02: a7/a8/a priori spenti; < 0,0005: periodo vuoto).
  - 0.11.111: avvisi della stazione in schede galleggianti in basso a destra (`DiagnosisBar`), i pannelli non si spostano.
  - 0.11.112: log vuoto -> Impostazioni > Programmi collegati con import ADIF e log esterni UDP nella stessa pagina.
  - 0.11.113: sei display nuovi (`ModernDisplays.tsx`): estesi 640x70 `icdual`/`icblue`/`ledline`, quasi quadrati 330x140 `icblueq`/`icblackq`/`yaesuq` (fissi nell'angolo, icone della testata piccole con `body.rd-square`); tutti i display estesi alti 70; misure accanto alle scelte.
  - 0.11.121: OmniRig che non risponde spiegato in chiaro (`explain_omnirig` in `tx-native/src/ptt.rs`); DX Cluster nascosto sul telefono se spento.
  - 0.11.120: telefono (QR senza adattatori virtuali, `lan::lan_ipv4s_named`; `GET /api/v1/lan/check` e `POST /api/v1/lan/allow` per rete «Pubblica» e firewall, `NetworkChecklist.tsx`), finestra che si ricollega se il flusso resta muto 8 s (`api/stream.ts`), Novita' compatte, quasi quadrato a 120, splash una volta per avvio (sessionStorage), controllo della versione nuova 8 s dopo l'avvio.
  - 0.11.119: porta del CAT scrivibile (indirizzi di rete per Hamlib NET rigctl, avviso se la condivisione usa la stessa porta), paese leggibile nel pannello CQ, quasi quadrato sopra la testata (createPortal).
  - 0.11.118: pannelli da togliere in Impostazioni > Finestre, import ADIF fino a 256 MB.
  - 0.11.117: «gia' lavorato» solo per nominativi nel log, tilde dello S-meter stimato.
  - 0.11.116: conteggio anonimo delle installazioni attive (`Rapporti::segnala_attivo`, una volta al giorno, `count_active` acceso di fabbrica; Worker `POST /v1/attivo`, `GET /v1/attivi` con `ADMIN_TOKEN`, tabella D1 `attivi`).
  - 0.11.115: anteprime dei display estesi a 640x70 nel menu Radio.
  - 0.11.114: locandina della versione nuova all'apertura (`UpdatePoster.tsx`), «Piu' tardi» fino alla versione dopo.
  - 0.11.106: decodifica FT8 con tempo massimo (`ft8::decode::Options::deadline`, 11 s dal motore).
- **Rapporti degli errori**: letti e cancellati il 2026-11-04 (R2 e D1 vuoti): da li' sono nate la 0.11.105 e la 0.11.106; i casi ancora aperti sono in `TODO.md` («Dai rapporti degli errori»).
- **Storico pulito**: il 2026-11-02 lo storico di GitHub e' stato riscritto (git filter-repo, push forzato di main, gh-pages e tag) per togliere ogni riferimento all'autore delle tabelle del codice polare dell'UltraFOX. Regola: non citarlo mai (vedi memoria privata). Copia dello storico vecchio solo in locale: `D:\progetti\Aurora-backup-prima-pulizia-qpc.bundle`.

## Lavoro in corso

- **2026-11-05, non ancora rilasciato** (su `main`, voci in CHANGELOG «Non rilasciato»; ultima versione pubblicata 0.11.131):
  - audio: cattura con `cpal` feature `realtime` (MMCSS), buffer riusati nel thread audio, rapporto dei blocchi persi con causa (`Xrun`), minuti, blocchi al minuto, tipo di ingresso, Hz, canali, formato (`audio-spectrum/src/capture.rs`);
  - avvisi sullo schermo dal motore: `aurora_core::notice::notice(id, detail)` → `Event::Notice` → `DiagnosisBar` (`audio_loss`, `no_radio`, `omnirig`, `slow_pc`; lo stesso avviso non si ripete prima di 2 minuti);
  - «Verifica tutto» in Impostazioni › Assistenza (`app/src/components/CheckAll.tsx`): stazione, radio, audio, orologio, telefono;
  - OmniRig: terzo tentativo del cambio frequenza; porta del server occupata: il messaggio dice il programma (Windows);
  - nominativo o locatore da compilare: avviso, non piu' errore nei rapporti;
  - AGCc anche in FT4 (`ft8::sync::agc_compensate_len`, `ft4::decode::Options::agc`);
  - decodifica lenta: `engine-native/src/slow.rs` (4 periodi su 10 oltre il 70% → rapporto `engine_native::lento` e avviso `slow_pc`);
  - riquadro «Radio non collegata» dentro la riga in alto, con spazio sopra e sotto.

- Prototipi grafici in `docs/prototipo-*.html` e `docs/mockup-display-*.html` (anche sul Desktop del PC di casa, cartella `Aurora-prototipi`): il proprietario vuole sempre un prototipo prima del codice grafico.
- Rapporti su Cloudflare letti alle 18:00 del 2026-11-04: 16 installazioni attive; un pacchetto di assistenza (OmniRig non in linea). Le prove con il core di prova vanno fatte su un'altra porta (8799) se Aurora e' aperta: sulla 8787 il core di prova fallisce e manda un falso rapporto «porta in uso».
- Nessun ramo aperto. Il proprietario deve rimettere il PTT della sua stazione su RTS / COM9 in Impostazioni > Radio: dalla 0.11.102 non viene piu' da JTDX.ini.
- **Servizio dei rapporti** online: `https://aurora-rapporti.tizianocassone.workers.dev` (cartella `servizi/rapporti/`, R2 `aurora-rapporti`, D1 `aurora-rapporti`, segreto `ADMIN_TOKEN` impostato dal proprietario). Per leggerli: `npx wrangler d1 execute aurora-rapporti --remote --command "SELECT ..."` dalla cartella del servizio; i pacchetti si scaricano con `wrangler r2 object get aurora-rapporti/<chiave> --remote`.
- **Linux**: versione dallo stesso codice, non provata dall'autore e senza assistenza; `scripts/compila-linux.sh`, AppImage e `.deb` allegati da GitHub Actions a ogni release.
- Restano da fare: «Prova» per eQSL, PSK Reporter, DXSummit, NG3K, propagazione, NTP, UDP/TCP; NAME e QTH per i QSO registrati a mano; CAT4OM; pagina dei rapporti nell'installatore; tema del telefono separato da quello del PC (decisione del proprietario).
- I test li lancia il proprietario: prima di pubblicare solo `tsc` e `cargo check`.

## Prossimo passo

0. **2026-11-06, su `main` non rilasciato**: AP in FT4 e FT8 collegata al motore, elenco dei nominativi conosciuti, controllo del locatore (dettagli in `memory.md`, «Motori: AP…»). Dal PC di casa con REA: confronto sui WAV Aurora/JTDX e prova dal vivo di queste tre cose (voci in `TODO.md`).

0. Pubblicare la versione con il lavoro del 2026-11-05 (dal PC), poi guardare su Cloudflare se i blocchi audio persi calano e da che tipo di ingresso arrivano.
0. JT65/JT9 sul segnale debole e SWL (`NFILT2`, `ldeepsync`): da fare dal PC con JTDX e REA, non a memoria.
1. Opzioni del display lette da Hamlib per ogni modello (`TODO.md`, «Display grandi in testata»).
2. Provare dal vivo i sei display, soprattutto i quasi quadrati in testata, e la locandina alla prossima versione.
3. Guardare i rapporti e le installazioni attive su Cloudflare (`SELECT ... FROM attivi` o `GET /v1/attivi`). Download al 2026-11-04: 266 `Aurora-Setup.exe` (prime installazioni), 22 Linux, 374 exe con versione = aggiornamenti automatici.
4. Le voci di `TODO.md` «Dai rapporti degli errori»: avvio fallito con WebView2 su Windows 11 Dev, porta 8787 occupata, campioni audio persi.
5. Poi le voci aperte di `docs/tasks.md` (TCI, Q65, MSK144, pulsanti dei modi, QMAP).

## Da provare dal vivo (proprietario)

Del 2026-11-04: guida al primo avvio su un PC pulito, copia e ripristino delle impostazioni, suono dello splash, decodifica sul PC lento con l'IC-7300 (audio con due schede USB uguali confermato dal vivo con la 0.11.108).
Scheda DXpedition con mappa (corretta nella 0.11.65 e 0.11.67), lente sul display (2x), pannello QSO con colori (viola acceso, rosso per Abilita Tx/AutoTX), doppio clic su spot FT4/FT8 che cambia modo, pagina Log. 
Pannello Comandi (DX scritto a mano, rapporto, Hold Tx Freq, tipo di CQ, Tune e Log QSO), pannello QSO in due colonne, Hound automatico e FOX, Wait and Pounce, chiusura con RR73 dopo un R+rapporto al Tx1, nuovo menu Impostazioni e pagina Log, barre Rx/Tx, display Yaesu/Icom, colore DXpedition nel cluster, a7/a8, stop per ROS alto, invii a Wavelog/ClubLog/QRZ/LoTW, integrazioni UDP.

## Dove stanno le cose

- Codice: `D:\progetti\pc\Aurora` (repository pubblico github.com/wifi75/Aurora). Compilazione dell'installatore: `D:\build\Aurora-0.11.1` (cartella di rilascio, si porta sul commit con `git checkout --detach`).
- Analisi di WSJT-X 3: `D:\build\analisi-wsjtx` (sorgenti e binari), `docs/analisi-wsjtx-3.md`.
- Chiave di firma degli aggiornamenti: nei segreti di GitHub (installatori compilati da GitHub); copia sul PC di casa in `%USERPROFILE%\.aurora-signing\aurora.key`, solo per prove in locale.
- Le copie di lavoro `D:\build\wt-*` non elencate sopra sono gia' unite a `main` e si possono cancellare.
