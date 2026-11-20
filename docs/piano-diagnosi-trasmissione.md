# Piano: diagnosi della trasmissione (PTT, modo radio, audio)

Scelto il 2026-11-20 dopo la lettura delle issue di WSJT-X (#173, #166, #167, #172, #144, #159: PTT che scatta senza RF o senza audio dopo la «preparazione del modo radio», senza nessun avviso).

## Obiettivo
Ogni trasmissione lascia un esito leggibile passo per passo; se un passo fallisce l'utente lo vede con una frase chiara e un suggerimento. Nessun pulsante che trasmette per provare.

## Decisioni del proprietario
- Gli indicatori (pallini) stanno nella **scheda Trasmissione**.
- Il pulsante **«Prova senza trasmettere»** si fa (Impostazioni › Radio): esegue tutta la catena senza mai dare il PTT.

## Passi
1. **Una riga di registro per trasmissione** («in onda Tx{n}», `engine-native/src/tx.rs`): modo radio chiesto e modo letto indietro, esito PTT, dispositivo audio, picco reale e durata del segnale mandato, potenza/ROS/ALC se la radio li legge. La riga va anche nel pacchetto di assistenza: `diagnostica/trasmissioni.log` (`docs/pacchetto-assistenza.md`).
2. **Controlli dopo ogni passo**, solo all'inizio di ogni trasmissione (non a ogni secondo):
   - dopo il cambio del modo radio, leggere `m` e confrontarlo con il richiesto (USB-D/Data invece di USB);
   - dopo il PTT, leggere lo stato di trasmissione via CAT: se la radio dice «ricezione» mentre Aurora trasmette, avviso «la radio non ha accettato il PTT»;
   - sul segnale audio: picco quasi zero o volume di Windows a zero = «audio muto verso la radio».
   Le radio che non rispondono a tutti i comandi (FT-991A, Xiegu G90) saltano il controllo senza errore.
3. **Pallini nella scheda Trasmissione**: Modo radio, Audio, PTT; verdi o rossi, la causa al passaggio del mouse. Nessuna finestra che si apre.
4. **«Prova senza trasmettere»**: modo, audio verso il dispositivo (a PTT spento) e stato letto dalla radio; esito con le stesse frasi dei pallini.
5. **Watchdog del PTT**: verificare con prove che il PTT sia sempre rilasciato (errore, chiusura di Aurora, periodo scaduto); una parte esiste dalla 0.11.282.
6. **Prove** con il finto `rigctld` (`rig-control/src/fake_rigctld.rs`): radio che rifiuta il modo, radio che non entra in trasmissione, audio muto, PTT che resta acceso. Le prove con la radio vera le fa il proprietario.

## Fuori dal piano
Nessuna trasmissione di prova (portante), nessuna modifica alle logiche di QSO (`docs/logiche-jtdx.md`). Il confronto con la «preparazione del modo radio» di WSJT-X 3.3.0-beta6 si fa con REA, non a intuito.

## Stato
- [x] 1 registro per trasmissione: riga «trasmissione: audio picco … PTT …» in `aurora.log` (finisce nel pacchetto di assistenza, niente file a parte) e rapporto `TxReport` (`tx-native/src/scheduler.rs`)
- [x] 2 controlli: PTT letto dalla radio all'inizio dell'audio (`Ptt::read_state`, Hamlib), picco audio, modo radio letto dal CAT; avvisi nel registro (parziale: il confronto del modo con quello richiesto e' solo «modo che non porta il digitale»)
- [x] 3 pallini nella scheda Trasmissione (`TxChainDots.tsx`, `lib/txChain.ts`, stato `tx_chain`)
- [x] 4 «Prova senza trasmettere» in Impostazioni › Radio (`settings/TxCheck.tsx`, `lib/txCheck.ts`): solo lettura, mai PTT ne' audio
- [ ] 5 watchdog del PTT verificato
- [x] 6 prove: rapporto in `scheduler.rs`, `txChain.test.ts`, `txCheck.test.ts`; prove con il finto `rigctld` e PTT che non si legge: ancora da scrivere
