# RTTY oltre tutti gli altri programmi (progetto, 2026-11-12)

Punto di partenza: il ricevitore rifatto il 2026-11-12 (`core/crates/mode-digi/src/rtty.rs`): centro fra i toni e mark alto in USB, filtro a
coseno rialzato K=1,35 (fldigi), ATC con livelli acceso/spento per tono, start come `is_mark_space` (fldigi), PLL di bit che si corregge sui fronti, UOS, squelch su
S/N, discriminatore sotto 40 Hz. Prove: `tests/rtty_confronto.rs`, `tests/rtty_fldigi.rs` (fldigi arbitro, feature `fldigi-oracle`).
Le cifre in dB sono **stime da misurare**.

## Il margine che resta
Eb/N0 = S/N(3 kHz) + 18,2 dB a 45,45 baud. Per l'FSK non coerente ideale il CER e' circa 0,1 % a −6 dB, 5 % a −9 dB, 36 % a −12 dB; Aurora fa
9,2 / 37,4 / 73,1 %: circa **3 dB** dal limite. Le perdite: filtro non adattato al bit, decisione dura (il valore morbido si butta via), start
cercato su decisioni dure, un solo campione a meta' bit. A S/N alto l'1-3 % residuo sono caratteri inventati nel rumore prima e dopo il testo:
va misurato a parte.

## Cosa fanno gli altri (fonti pubbliche e codice)
- **MMTTY 1.70L** (`terze-parti/mmtty`, LGPL): passa-banda e notch/LMS facoltativi, filtro AA6YQ a due picchi, **limitatore sempre acceso**,
  quattro demodulatori (IIR 60 Hz di base, FIR, PLL, FFT a 0,8 bit), «Smooth», ATC min/max (spenta di base), squelch su |m−s|, **voto di
  maggioranza** per bit, stop 1/1,42/1,5/2, profili (Standard, Fluttered, Multi-path, AA6YQ FIR512, FFT, 23 Hz). Niente decisione morbida,
  niente diversity, niente piu' decodificatori insieme (si aprono piu' istanze).
- **2Tone** (G3YYD, chiuso): modi Flat/Flutter/Selective/Spread; Selective tratta i due toni separati («diversity in banda»); squelch da 1,5 dB con
  memoria di 4 caratteri; Contest Mode (alfabeto ridotto); **Repeat Match** (ripetizioni combinate per autocorrelazione).
- **Decodifica bayesiana** (idea generale): probabilita' d'errore per bit e per carattere, caratteri incerti in chiaro, **squelch sulla
  probabilita'**, fusione bayesiana dei nominativi simili, correzioni di registro, numeri, CQ e DE, conoscenza dei QSO e dei contest.
- fldigi: gia' portato (vedi `docs/analisi-psk-cw.md` e il CHANGELOG).

## Il progetto (catena per ogni segnale)
1. **B1 soppressore d'impulsi** e variante a limitatore (come MMTTY) contro QRN, AGC che pompa, cattura.
2. **B2 front-end decimato**: oscillatore a rotazione, decimazione complessa a ~16 campioni per bit, somme scorrevoli (CPU 20-50 volte meno:
   rende possibili varianti parallele e multicanale).
3. **B3 metriche morbide (LLR)** con ampiezza e rumore stimati in banda (Kahn a S/N basso, inviluppo ad alto; un tono che svanisce diventa da
   solo «solo mark/solo space»). 0,5-1 dB, di piu' con un tono attenuato; squelch e AFC non sporcati dalla stazione vicina.
4. **B4 decodifica del carattere intero a massima verosimiglianza**: programmazione dinamica su start + 5 bit + stop variabile + pausa, con
   ipotesi «niente segnale», registro LTRS/FIGS come stato, orologio a lungo termine (45 contro 45,45 baud, ppm), ritardo di 2 caratteri o
   correzione successiva; da qui la probabilita' del carattere. **1-2 dB** alla soglia del 5-10 %.
5. **B5 varianti in parallelo e fusione per carattere**: boxcar 1 bit, finestra 0,5 bit (flutter), coseno rialzato 2×baud (spread), discriminatore,
   limitatore; si sceglie il carattere piu' probabile con calibrazione. 2-4 dB stimati con flutter, multipath e QRM.
6. **B6 squelch statistico**: stampa se P(giusto) ≥ 0,5, in chiaro sotto 0,9, isteresi, memoria di 4 caratteri. Apre 3-6 dB piu' in basso a parita'
   di caratteri inventati.
7. **B7 ripetizioni combinate**: somma delle LLR delle parole ripetute (+3 dB con due copie, +4,8 con tre), pila dei nominativi.
8. **B8 registro FIGS/LTRS come stato nascosto** e conoscenze del contest (alfabeto ridotto, scambio da `contest::Regole`, nominativi da `dxcc`)
   attraverso un'interfaccia `TextPrior`.
9. **B9 multicanale**: coppie di picchi alla distanza dello shift con energia alternata; una catena completa per segnale (30 segnali sotto il 10 %
   di un core).
10. **B10 diversity** con due ingressi (somma delle LLR a rapporto massimo): +3 dB in AWGN, molto di piu' con fading indipendenti.
Robustezza: avviso di audio tagliato, livelli ad attacco veloce, filtri stretti e squilibrio dei toni assorbiti da B3/B4, AFC a decisione guidata.

## Prove
- **MMTTY come secondo arbitro** (feature `mmtty-oracle`): `oracle/mmtty_rtty_rx.cpp` con le classi di `Rtty.cpp`/`fir.cpp`/`CLX`/`ComLib`
  copiate alla lettera (LGPL, autori in testa) e uno stub della VCL; ingresso ×32768, mark sotto con `m_inv`; i 7 profili da `UserPara.ini`;
  verifica di fedelta' con `mmtty.exe` su file .mmv.
- **Scenari**: rumore da −15 a +6 dB a passi di 1 dB, fading ITU-R F.1487 (Good/Moderate/Poor/Flutter/alte latitudini), fading selettivo, QRM RTTY
  a 100-400 Hz e co-canale, portanti CW, PSK31, splatter SSB, audio tagliato, filtri della radio 250/300/500 Hz e twin-peak, AGC, deriva, shift
  e baud diversi, ±500 ppm, stop 1-2, jitter, impulsi, testo di contest con 2-3 stazioni sovrapposte.
- **Misure**: CER sul solo segnale, caratteri inventati nel rumore, nominativi e scambi giusti, soglia S/N al 5 e 10 %, curva ideale, velocita'
  rispetto al tempo reale; almeno 3000 caratteri per cella, confronto appaiato con intervalli bootstrap.
- **Criterio «battere tutti»**: in ogni scenario Aurora (impostazioni di base) almeno pari al migliore fra fldigi e il miglior profilo MMTTY, con
  almeno 0,5 dB di vantaggio sulla soglia al 10 %.
- **Registrazioni vere** in `rtty-corpus/<id>/` (WAV 12 kHz, .mmv, `meta.toml`, testo di riferimento, nominativi dal log, uscite di ogni
  decodificatore) per confrontare anche 2Tone sul PC del proprietario; bollettini RTTY di W1AW come testo noto.

## Piano (un commit per passo)
1 scenari comuni e misure · 2 MMTTY arbitro · 3 tabella Aurora/fldigi/MMTTY · 4 raccolta WAV e valutazione · 5 front-end decimato · 6 LLR e rumore in
banda · 7 decodifica del carattere intero · 8 caratteri con affidabilita' (eventi e interfaccia) · 9 squelch statistico · 10 soppressore e
limitatore · 11 varianti e fusione · 12 registro e prior del contest · 13 ripetizioni e nominativi · 14 AFC e tolleranze · 15 multicanale ·
16 diversity. Ogni passo dal 5 in poi si chiude solo se la tabella non peggiora in nessuno scenario.

Fonti: [W0YK CTU 2019](https://www.contestuniversity.com/wp-content/uploads/2019/05/13-W0YK-Taking-Digital-Contesting-to-the-Limit_2019.pdf),
[2Tone Operational Information](https://www.sp2pby.pl/pdf/2Tone%202102a.pdf), [manuale 2Tone](http://ok2pad.webz.cz/logger32/g3yyd_2tone.pdf),
[W8TN su Repeat Match](https://w8tn.blogspot.com/2016/11/new-feature-of-2tone-rtty-decoder.html).

## Risultati dei passi

Misure con `tests/rtty_tabella.rs` (`--features fldigi-oracle,mmtty-oracle`, 480 caratteri per cella, griglia da −15 a +6 dB a passi di 1
dB, AFC spento in tutti, squelch spento): soglia S/N (dB in 3 kHz) a cui il CER scende sotto il 10 % e CER medio sulla griglia (%). Per
MMTTY il migliore dei sette profili in quello scenario. «--» = non scende mai sotto il 10 %.

**Passo 1** (scenari e misure): `tests/common/rtty_scenari.rs` con 25 scenari (rumore a 85/170/425/850 Hz e 45,45/50/75 baud, errore di
sintonia, space attenuato, CCIR buono/medio/cattivo, flutter 10 Hz, alte latitudini 30 Hz/7 ms, QRM RTTY a 250 Hz +6 dB e a 120 Hz −6 dB,
portante 30 Hz sopra il mark, PSK31 a 150 Hz, scheda a ±500 ppm, trasmettitore a 45 baud, stop di 1 e 2 bit, scariche, filtro della radio a
250 Hz, audio tagliato, AGC che pompa), soglia al 5 e al 10 % interpolata, CER medio, caratteri inventati su un minuto di rumore.

**Passo 2** (MMTTY arbitro): `oracle/mmtty_rtty_rx.cpp` e `oracle/mmtty/mmtty_dsp.inc` (generato da `oracle/mmtty/estrai.py` dai sorgenti
1.70L), sette profili di `UserPara.ini` (Standard, Fluttered, Fluttered FIR, Multi-path, AA6YQ FIR512, FFT, 23 Hz PLL). Tutti decodificano
il nostro segnale pulito (`tests/rtty_mmtty.rs`). Da fare sul PC del proprietario: confronto di fedelta' con `mmtty.exe` sugli stessi file.

**Passo 3** (tabella di partenza, ricevitore del 12 novembre):

| scenario | Aurora | fldigi | miglior MMTTY |
|---|---|---|---|
| rumore 170/45 | −8,2 / 20,6 | −8,1 / 21,7 | AA6YQ512 −7,9 / 20,8 |
| rumore 170/75 | −5,5 / 30,5 | −5,2 / 32,0 | Fluttered −5,1 / 33,9 |
| rumore 425/50 | −7,7 / 22,8 | −7,6 / 24,0 | AA6YQ512 −7,4 / 23,0 |
| rumore 850/45 | −8,0 / 20,7 | −7,7 / 21,6 | AA6YQ512 −7,3 / 22,5 |
| rumore 85/45 | −7,7 / 20,0 | −7,9 / 20,9 | Fluttered −7,5 / 24,3 |
| fuori +10 Hz | −7,4 / 22,6 | −7,4 / 22,8 | Fluttered −7,0 / 26,4 |
| space −12 dB | −5,4 / 29,8 | 2,5 / 49,9 | Flutt.FIR 5,1 / 71,6 |
| CCIR buono | −0,8 / 30,3 | −0,5 / 31,9 | AA6YQ512 3,2 / 32,4 |
| CCIR medio | 3,1 / 35,3 | 1,1 / 35,2 | Flutt.FIR 5,2 / 42,1 |
| CCIR cattivo | -- / 41,6 | 1,8 / 36,7 | Flutt.FIR 5,9 / 47,0 |
| flutter 10 Hz | -- / 67,1 | -- / 50,5 | Fluttered 4,0 / 48,2 |
| alte latitudini | -- / 73,9 | -- / 63,9 | Flutt.FIR 3,8 / 50,9 |
| QRM a 250 Hz +6 | −8,2 / 21,7 | −7,7 / 22,7 | AA6YQ512 −8,0 / 21,2 |
| QRM a 120 Hz −6 | −8,2 / 21,1 | −6,8 / 23,0 | FFT −8,0 / 22,5 |
| portante +30 Hz | 4,0 / 39,5 | -- / 50,7 | AA6YQ512 −6,2 / 22,4 |
| PSK31 a 150 Hz | −8,0 / 20,7 | −8,0 / 21,6 | AA6YQ512 −7,8 / 20,4 |
| scheda +500 ppm | −7,5 / 22,1 | −7,5 / 22,9 | AA6YQ512 −7,7 / 21,8 |
| scheda −500 ppm | −7,4 / 22,3 | −7,2 / 22,7 | AA6YQ512 −8,0 / 22,1 |
| 45 baud | −6,8 / 25,9 | −6,0 / 27,5 | Flutt.FIR −6,5 / 27,8 |
| stop 1 bit | −7,6 / 22,1 | −7,4 / 22,1 | AA6YQ512 −7,7 / 21,1 |
| stop 2 bit | −8,4 / 19,8 | −8,0 / 20,9 | AA6YQ512 −8,3 / 19,9 |
| scariche | −7,3 / 22,0 | −7,7 / 22,6 | AA6YQ512 −8,0 / 21,0 |
| filtro 250 Hz | −8,3 / 20,7 | −8,1 / 21,6 | FFT −8,1 / 21,1 |
| audio tagliato | −6,5 / 26,1 | −6,2 / 27,3 | AA6YQ512 −6,5 / 25,2 |
| AGC che pompa | -- / 50,6 | −6,9 / 23,5 | AA6YQ512 −7,8 / 20,7 |

Caratteri inventati su un minuto di solo rumore, squelch spento: Aurora 113, fldigi 123, MMTTY 132-147. Con lo squelch: Aurora (S/N 6 dB)
0, MMTTY Standard/Fluttered/Multi-path 0, AA6YQ512 13, Flutt.FIR 101, FFT 154, PLL 122 (la soglia 600 di MMTTY non e' tarata per quei
demodulatori).

Il ricevitore di partenza e' gia' il migliore sul rumore bianco, ma perde con il fading (CCIR medio e cattivo, flutter, alte latitudini:
il limitatore di MMTTY e l'ATC veloce di fldigi reggono meglio), con la portante dentro la banda, con l'AGC che pompa e, di poco, con le
scariche e con la scheda a −500 ppm: sono questi i bersagli dei passi successivi.

**Passo 4** (registrazioni vere): `tests/rtty_corpus.rs` legge ogni cartella di `rtty-corpus/` (fuori dal repository, in `.gitignore`; o
quella di `AURORA_RTTY_CORPUS`): `audio.wav` (PCM 16 bit o float 32, qualunque frequenza, si porta a 12 kHz; stereo: sinistro, il destro
servira' alla diversity), `meta.toml` (`centre_hz`, `shift_hz`, `baud`, `reverse`), `riferimento.txt` e `nominativi.txt` facoltativi e le
uscite degli altri programmi come `uscita-2tone.txt`, `uscita-mmtty.txt`. Scrive `uscita-aurora.txt` (e quelle degli
arbitri compilati) e stampa CER e nominativi trovati per ogni programma. `AURORA_WAV=<cartella>` con la prova `tabella` scrive i WAV a
0 dB di ogni scenario, da far leggere a 2Tone. Da fare sul PC del proprietario: registrare bollettini W1AW (testo noto) e un
paio d'ore di contest, passarli a 2Tone e MMTTY e mettere le uscite nelle cartelle.

**Passo 5** (front-end decimato): oscillatore a rotazione sul centro, passa-basso FIR a finestra di Blackman che tiene toni e canali del
rumore, decimazione a circa 32 campioni per bit; filtri dei toni, filtri dell'AFC, canali del rumore e discriminatore lavorano sul segnale
complesso decimato e l'AFC sposta solo l'oscillatore. Tabella invariata entro il rumore della misura (scarti fra −0,1 e +0,25 dB; portante
−1,0 dB), velocita' da 225 a 1490 volte il tempo reale (prova `velocita`): tenuto.

**Passo 6** (LLR e rumore in banda): ogni campione da' un valore morbido, la decisione dell'ATC divisa per la varianza del rumore misurata
sulle ampiezze dei due toni (decisione guidata, quattro stati), e il bit si decide sulla somma dei valori morbidi intorno al centro invece
che su un campione solo (la «maggioranza» di MMTTY, ma morbida). Da solo non cambia niente: con gli stessi semi la somma degli scarti delle
soglie e' −0,13 dB su 25 scenari (finestra di 0,25 bit; con 0,5 e 0,75 bit le differenze restano nel rumore della misura). Non tenuto come
passo a se': le LLR entrano nel passo 7, che ne ha bisogno.

**Passo 7** (carattere intero): per ogni possibile centro dello start il punteggio del carattere (bit precedente a mark, start a space, i
cinque dati comunque, stop a mark) sulle somme delle LLR intorno ai centri dei bit (0,8 bit; 0,5 e 1,0 bit fanno peggio), il migliore in
una finestra di un bit, poi il prossimo start non prima di 6,5 bit. Rispetto al passo 5: soglie da −0,8 a +0,15 dB (somma −10,4 dB, quasi
tutta dalla portante: −5,6 dB), CER medio giu' in tutti gli scenari (−0,1 … −7,6 punti; fading e AGC −2,4 … −4,0). Il prezzo: sul solo
rumore, a squelch spento, 191 caratteri al minuto invece di 113 (li toglie lo squelch statistico del passo 9). Tenuto, di base; la macchina
dei bit di fldigi resta con `set_frame_decoder(false)`.

**Passo 8** (caratteri con affidabilita'): ogni carattere della decodifica intera porta la probabilita' di essere giusto, il prodotto
delle probabilita' dei sette bit (start a space, cinque dati, stop a mark) dalle somme delle LLR scalate di 0,03, e una lettera o una
cifra moltiplica anche quella del FIGS o LTRS da cui dipende. Taratura (prova `taratura`, rumore, CCIR medio, space −12 dB, 75 baud da
−12 a 0 dB): detti al 90-100 % giusti al 98 %, al 80-90 % all'83 %, al 50-60 % al 58 %, al 10-20 % al 14 %. `process_chars` da' i
caratteri con la probabilita'; l'evento `rx_text` porta in `weak` le posizioni dei caratteri sotto 0,9 e la finestra di ricezione li
mostra in chiaro. Tabella identica al passo 7 (la decodifica non cambia).

**Passo 9** (squelch statistico): guarda la probabilita' media degli ultimi quattro caratteri; chiuso li trattiene e si apre sopra 0,7
(escono anche quelli trattenuti: l'inizio del testo non si perde), aperto si chiude sotto 0,5 e un carattere sotto 0,5 aspetta il
seguente prima di uscire (il rumore che segue il segnale non passa). Prova `squelch` (480 caratteri per cella, soglia al 10 %):

| scenario | spento | S/N 6 dB | statistico |
|---|---|---|---|
| rumore 170/45 | −7,6 | −7,4 | −8,0 |
| CCIR medio | 2,2 | 2,0 | 1,0 |
| space −12 dB | −4,8 | −3,2 | −5,0 |
| rumore 170/75 | −5,3 | −5,3 | −6,6 |
| QRM a 250 Hz +6 | −7,4 | -- | −7,7 |

Sul minuto di solo rumore: 211 caratteri a squelch spento, 0 con quello su S/N, 0 con quello statistico (anche aprendo a 0,4). Lo
squelch statistico apre quindi piu' in basso di quello su S/N (fino a 1,6 dB con un tono attenuato, e regge la stazione vicina che
inganna il rapporto segnale/rumore) e toglie anche i caratteri inventati prima e dopo il testo, per cui la soglia migliora persino
rispetto allo squelch spento. Con lo squelch acceso il servizio usa lo statistico; la soglia S/N (ora 0 dB di base) si aggiunge solo se
alzata. La tabella a squelch spento resta identica.

**Passo 10** (soppressore d'impulsi e limitatore): alla frequenza piena un campione oltre 8 volte il livello medio (media di |x| su 20 ms
fatta sui campioni ritagliati alla soglia, cosi' un impulso non la sposta) si azzera insieme ai 0,5 ms seguenti. Rispetto al passo 7:
soglie da −0,20 a +0,10 dB, CER medio da −0,38 (scariche) a +0,06 punti; tenuto, acceso di base (`set_blanker`). Con 5 volte invece di 8
tocca anche il rumore e peggiora tutto di 0,1-0,2 dB. Il limitatore come MMTTY (filtro largo quanto la coppia di toni, poi ampiezza
costante, poi i filtri dei toni) da solo perde molto nel rumore bianco (+2 … +11 dB di soglia) e guadagna solo con il flutter (CER −4,4
punti), le alte latitudini (−3,0) e l'AGC che pompa (−10,8): non si usa da solo ma come variante del passo 11. Provati e scartati: le
ampiezze normalizzate sul rumore accanto ai toni (AGC −16 punti ma portante +3,4 dB e space attenuato +1,6 dB) e un compensatore dei
salti di guadagno che riscala i livelli dell'ATC (scatta anche sul rumore: +17,7 dB in somma).

**Passo 11** (varianti e fusione): accanto alla catena principale ne girano altre due sullo stesso audio, una col limitatore (LLR scalate
di 0,3, che altrimenti sarebbero troppo sicure) e una con le ampiezze normalizzate sul rumore accanto ai toni (media in salita su 32 bit,
in discesa su 2: l'AGC della radio che scende di colpo). I caratteri delle tre catene che cadono a meno di mezzo carattere (3,5 bit)
sono lo stesso carattere e vince il piu' probabile; uno visto solo da una variante passa solo sopra 0,6 (senza questa regola il rumore
dava caratteri diversi in ogni variante e la loro unione peggiorava tutto di 0,8 dB). Si decide 11 bit dopo lo start (circa 3 bit oltre
la catena sola). Rispetto al passo 10: soglie da −1,57 (CCIR medio) a +0,16 dB, CER medio giu' quasi ovunque: AGC che pompa −17,0
punti (e ora una soglia: −2,8 dB), flutter −4,3, alte latitudini −1,7, portante −1,9 punti e −0,66 dB, CCIR cattivo −1,1. La taratura
delle probabilita' resta giusta (detti al 90-100 % giusti al 97,9 %), la velocita' scende a 448 volte il tempo reale. Una variante con il
filtro piu' largo (2 baud) non aggiunge niente oltre queste due. Tenuto, di base (`set_variants`); spento con il discriminatore.

Tabella dopo il passo 11 (Aurora, soglia al 10 % / CER medio):

| scenario | passo 11 | passo 3 |
|---|---|---|
| rumore 170/45 | −8,1 / 19,4 | −8,2 / 20,6 |
| rumore 170/75 | −5,5 / 30,3 | −5,5 / 30,5 |
| rumore 425/50 | −8,1 / 21,6 | −7,7 / 22,8 |
| rumore 850/45 | −8,2 / 19,5 | −8,0 / 20,7 |
| rumore 85/45 | −8,3 / 19,2 | −7,7 / 20,0 |
| fuori +10 Hz | −7,8 / 21,3 | −7,4 / 22,6 |
| space −12 dB | −5,3 / 28,4 | −5,4 / 29,8 |
| CCIR buono | −0,8 / 28,8 | −0,8 / 30,3 |
| CCIR medio | 1,5 / 32,6 | 3,1 / 35,3 |
| CCIR cattivo | -- / 37,6 | -- / 41,6 |
| flutter 10 Hz | -- / 60,1 | -- / 67,1 |
| alte latitudini | -- / 68,0 | -- / 73,9 |
| QRM a 250 Hz +6 | −8,3 / 20,2 | −8,2 / 21,7 |
| QRM a 120 Hz −6 | −8,2 / 20,1 | −8,2 / 21,1 |
| portante +30 Hz | −3,3 / 30,3 | 4,0 / 39,5 |
| PSK31 a 150 Hz | −7,8 / 19,7 | −8,0 / 20,7 |
| scheda +500 ppm | −7,8 / 20,6 | −7,5 / 22,1 |
| scheda −500 ppm | −7,7 / 20,7 | −7,4 / 22,3 |
| 45 baud | −7,1 / 24,2 | −6,8 / 25,9 |
| stop 1 bit | −8,4 / 19,0 | −7,6 / 22,1 |
| stop 2 bit | −8,4 / 19,5 | −8,4 / 19,8 |
| scariche | −7,8 / 20,0 | −7,3 / 22,0 |
| filtro 250 Hz | −8,2 / 19,1 | −8,3 / 20,7 |
| audio tagliato | −6,9 / 24,4 | −6,5 / 26,1 |
| AGC che pompa | −2,8 / 29,6 | -- / 50,6 |

**Passo 12** (registro FIGS/LTRS come stato nascosto e modello del testo): ogni parola si trattiene fino allo spazio (o 20 bit dopo
l'ultimo carattere) e si prova, per un carattere alla volta, l'ipotesi che fosse un FIGS o un LTRS letto male (o il contrario): vince se
la probabilita' dei cinque bit con quel codice piu' il modello del testo supera la parola letta di almeno 1 nat. Il modello di base
(`HamPrior`, interfaccia `TextPrior` per quelli di un contest) premia numeri, nominativi (prefisso, cifra, suffisso, /P) e parole di sole
lettere, e penalizza i miscugli di lettere e cifre che non sono nominativi, l'interpunzione dentro le parole e le parole fatte solo
delle lettere della fila delle cifre (Q W E R T Y U I O P: «TOO» invece di «599» quando si perde il FIGS). Rispetto al passo 11: soglie
da −0,11 a +0,09 dB, CER medio giu' in tutti gli scenari (−0,12 … −0,78 punti). Tenuto, di base (`set_prior`). Il prezzo: il testo esce
a parole intere invece che carattere per carattere. Il testo delle prove e' di tipo QSO (numeri e nominativi frequenti): su un testo
senza numeri il guadagno sara' minore. Il modello dello scambio di un contest (da `contest::Regole`) resta da collegare.

**Passo 13** (ripetizioni combinate): si ricordano le ultime 8 parole di almeno 4 caratteri con i valori morbidi dei bit; una parola della
stessa lunghezza con almeno il 70 % dei caratteri uguali e' una ripetizione («IU3ABC IU3ABC», «599 599»), e per ogni posizione si sommano
i valori delle due copie (con due copie il rumore pesa la meta'); un carattere incerto (sotto 0,9) si cambia se la somma e' piu' sicura
di lui, e la copia combinata resta in memoria per la ripetizione seguente. La prima versione (60 % uguali, 3 caratteri, ogni carattere
cambiabile) peggiorava il CER medio di 0,1-0,3 punti: il rumore fa parole corte che si somigliano. Rispetto al passo 12: soglie da −0,13 a
+0,02 dB, CER medio da −0,12 a −0,01 punti; tenuto, di base (`set_repeats`). La pila dei nominativi (fusione dei nominativi simili anche
non ripetuti di seguito) non e' fatta: la memoria delle ripetizioni ne copre la parte piu' utile.

**Passo 14** (sintonia fine a decisione guidata e tolleranze): con i caratteri decodificati bene (probabilita' media sopra 0,8) si misura
la rotazione di fase da un campione all'altro sul tono che la decisione dice acceso, separatamente per mark e space; ogni 16 bit, se i
due toni danno lo stesso errore entro 1,5 Hz, l'oscillatore del front-end si sposta di meta' dell'errore, entro un limite (25 Hz nelle
prove), anche con l'AFC spento. Rispetto al passo 13: 45 baud (scheda all'1 %, quindi toni spostati di 15 Hz) −0,44 dB e −0,92 punti,
fuori +10 Hz −0,28 dB, ma portante +30 Hz +0,26 dB e +0,44 punti (la portante sposta la misura sul mark; con un solo errore medio dai due
toni era +6,2 dB, con la concordanza entro 3 Hz +0,33). Peggiora uno scenario: resta un'opzione spenta (`set_fine_tuning`), la tabella
di base non cambia. Velocita' (45 contro 45,45 baud) e ±500 ppm: la decodifica del carattere intero ricentra ogni start, l'errore di
orologio lungo un carattere resta sotto 0,08 bit e non serve un orologio a lungo termine (scheda ±500 ppm entro 0,1 dB dal rumore).

**Passo 15** (multicanale): `rtty_multi::RttyMulti` trova da solo i segnali RTTY della banda audio (300-3000 Hz) e apre un ricevitore
completo per ognuno (con varianti, modello del testo, ripetizioni e squelch statistico). Un banco di filtri di Goertzel a passi di 5 Hz
misura la potenza su finestre di un bit; ogni secondo un centro e' un segnale se i due toni alla distanza dello spostamento sono picchi
(entro 10 Hz), stanno 6 dB sopra la mediana della banda e le loro potenze bit per bit sono anticorrelate (sotto −0,5). Un canale nuovo
non nasce a meno di mezzo spostamento da uno aperto; un canale senza caratteri sicuri per 30 s si chiude; al massimo 30. Prove: tre
stazioni insieme a 805, 1500 e 2210 Hz con rumore si decodificano tutte (e non si aprono canali sui lobi laterali); una portante nel
rumore non apre canali. Prima versione: i lobi a ±65 Hz dei toni forti aprivano canali falsi, tolti chiedendo i picchi. Il motore e'
pronto; l'interfaccia (una riga per canale nella finestra di ricezione) non e' ancora fatta.

**Passo 16** (diversity): `RttyDiversity` decodifica due ingressi (i due canali di una scheda stereo con due ricevitori o due antenne):
ogni ingresso ha la sua catena fino ai valori morbidi e campione per campione le due LLR si sommano (combinazione a rapporto massimo:
ognuna e' gia' divisa per il rumore del suo ingresso), poi decodifica del carattere intero, modello del testo, ripetizioni e squelch
lavorano sulla somma. Prova: lo stesso testo su due ingressi che svaniscono a turno (fading lento in controfase, 0,4 Hz, fra −15 dB e 0
dB) con rumori indipendenti: 81 errori su 198 caratteri con un ingresso, 0 con due. La tabella non cambia (un ingresso). Il motore e'
pronto; il servizio riceve ancora un ingresso solo (l'audio stereo e la scelta del secondo canale nelle Impostazioni sono da fare).

Fine dei passi 8-16, con le impostazioni di base: sul minuto di solo rumore 185 caratteri a squelch spento, 0 con lo squelch statistico;
439 volte il tempo reale.

**Tabella dopo il passo 16** (Aurora con le impostazioni di base; fldigi e il miglior profilo MMTTY dalla tabella del passo 3, stessi
scenari e stessi semi: gli arbitri non cambiano). Soglia al 10 % / CER medio; in grassetto dove Aurora e' dietro.

| scenario | Aurora | fldigi | miglior MMTTY |
|---|---|---|---|
| rumore 170/45 | −8,1 / 19,0 | −8,1 / 21,7 | −7,9 / 20,8 |
| rumore 170/75 | −5,7 / 29,4 | −5,2 / 32,0 | −5,1 / 33,9 |
| rumore 425/50 | −8,2 / 21,4 | −7,6 / 24,0 | −7,4 / 23,0 |
| rumore 850/45 | −8,3 / 19,1 | −7,7 / 21,6 | −7,3 / 22,5 |
| rumore 85/45 | −8,4 / 18,9 | −7,9 / 20,9 | −7,5 / 24,3 |
| fuori +10 Hz | −7,9 / 20,7 | −7,4 / 22,8 | −7,0 / 26,4 |
| space −12 dB | −5,4 / 28,2 | 2,5 / 49,9 | 5,1 / 71,6 |
| CCIR buono | −0,8 / 28,5 | −0,5 / 31,9 | 3,2 / 32,4 |
| CCIR medio | **1,4** / 32,2 | 1,1 / 35,2 | 5,2 / 42,1 |
| CCIR cattivo | **--** / 37,3 | 1,8 / 36,7 | 5,9 / 47,0 |
| flutter 10 Hz | **-- / 59,7** | -- / 50,5 | 4,0 / 48,2 |
| alte latitudini | **-- / 67,8** | -- / 63,9 | 3,8 / 50,9 |
| QRM a 250 Hz +6 | −8,4 / 19,8 | −7,7 / 22,7 | −8,0 / 21,2 |
| QRM a 120 Hz −6 | −8,3 / 19,6 | −6,8 / 23,0 | −8,0 / 22,5 |
| portante +30 Hz | **−3,3 / 30,1** | -- / 50,7 | −6,2 / 22,4 |
| PSK31 a 150 Hz | −8,1 / 19,3 | −8,0 / 21,6 | −7,8 / 20,4 |
| scheda +500 ppm | −7,8 / 20,4 | −7,5 / 22,9 | −7,7 / 21,8 |
| scheda −500 ppm | **−7,8** / 20,4 | −7,2 / 22,7 | −8,0 / 22,1 |
| 45 baud | −7,2 / 23,8 | −6,0 / 27,5 | −6,5 / 27,8 |
| stop 1 bit | −8,4 / 18,8 | −7,4 / 22,1 | −7,7 / 21,1 |
| stop 2 bit | −8,5 / 19,2 | −8,0 / 20,9 | −8,3 / 19,9 |
| scariche | **−7,9** / 19,6 | −7,7 / 22,6 | −8,0 / 21,0 |
| filtro 250 Hz | −8,2 / 18,8 | −8,1 / 21,6 | −8,1 / 21,1 |
| audio tagliato | −6,9 / 24,2 | −6,2 / 27,3 | −6,5 / 25,2 |
| AGC che pompa | **−2,8 / 29,3** | −6,9 / 23,5 | −7,8 / 20,7 |

Aurora e' davanti a tutti in 18 scenari su 25 (CER medio il piu' basso in 20). Restano dietro: fading veloce (flutter, alte latitudini:
MMTTY coi profili per il flutter), CCIR cattivo (fldigi), portante dentro la banda e AGC che pompa (il filtro AA6YQ di MMTTY), e di
0,1-0,3 dB scheda −500 ppm, scariche e CCIR medio. Il criterio «battere tutti di 0,5 dB in ogni scenario» non e' ancora raggiunto: i
prossimi bersagli sono un demodulatore per il fading veloce (finestra corta, decisione per tono come nel modo «Selective») come terza
variante, un notch adattivo per le portanti, e il collegamento all'interfaccia di multicanale e diversity.


## Salti di guadagno (2026-11-14)

Tabella di partenza: `rtty_tabella` (solo Aurora, 4 prove da 120 caratteri per cella, griglia -15..6 dB), uguale a quella del passo 16;
CER medio sui 25 scenari 26,61 %, 185 caratteri al minuto sul solo rumore a squelch spento (0 con gli squelch).

**Causa trovata** (scenario peggiore rispetto agli arbitri: «AGC che pompa», soglia -2,8 dB contro -7,8 del miglior MMTTY). Prova di
studio: lo stesso scenario a 30 dB di S/N perde 2-4 caratteri dopo ogni discesa del guadagno, uguale a 6 dB: non e' rumore. La catena
principale da sola sbaglia il 46 % dei caratteri. Stampando ampiezze e livelli campione per campione: dopo la discesa di 15 dB il livello
medio del mark acceso resta quello di prima (0,134 contro un mark che ora vale 0,03), il mark sta sotto la sua soglia, la decisione dice
sempre space e aggiorna solo i livelli dello spento; il livello stantio si sblocca solo dopo 16 bit (limite dell'eta' dei livelli
all'inviluppo lento di fldigi). Varianti provate una alla volta: varianza delle LLR piu' corta (nessun effetto), eta' dei livelli piu'
corta con un inviluppo veloce (AGC 46 -> 30 % ma rumore 170/45 +2,5 e space -12 dB +7,6 punti: scartata).

**Correzione tenuta**: la somma delle ampiezze dei due toni (che non cala ai cambi di bit) si confronta con quella attesa dai livelli,
la minore fra «mark acceso + space spento» e il contrario (cosi' un tono attenuato non scatta); se resta per piu' di 3 bit sotto 0,85
volte l'attesa, livelli e varianza si riportano alla scala misurata. Test `un_guadagno_che_scende_di_colpo_non_perde_caratteri`
(catena principale, AGC senza rumore): 18 caratteri persi su 88 senza la correzione, 4 con.

| scenario | prima s10 / CER | dopo s10 / CER |
|---|---|---|
| rumore 170/45 | -8,1 / 19,0 | -8,0 / 19,1 |
| rumore 170/75 | -5,7 / 29,4 | -5,6 / 29,6 |
| rumore 425/50 | -8,2 / 21,4 | -8,2 / 21,6 |
| rumore 850/45 | -8,3 / 19,1 | -8,2 / 19,3 |
| rumore 85/45 | -8,4 / 18,9 | -8,3 / 19,0 |
| fuori +10 Hz | -7,9 / 20,7 | -7,8 / 20,9 |
| space -12 dB | -5,4 / 28,2 | -5,4 / 28,2 |
| CCIR buono | -0,8 / 28,5 | -0,8 / 28,6 |
| CCIR medio | 1,5 / 32,2 | 1,3 / 31,9 |
| CCIR cattivo | -- / 37,3 | -- / 36,5 |
| flutter 10 Hz | -- / 59,7 | -- / 56,8 |
| alte latitudini | -- / 67,8 | -- / 67,0 |
| QRM a 250 Hz +6 | -8,4 / 19,8 | -8,4 / 19,9 |
| QRM a 120 Hz -6 | -8,3 / 19,6 | -8,3 / 19,8 |
| portante +30 Hz | -3,3 / 30,1 | -3,3 / 30,1 |
| PSK31 a 150 Hz | -8,1 / 19,3 | -8,0 / 19,5 |
| scheda +500 ppm | -7,8 / 20,4 | -7,9 / 20,6 |
| scheda -500 ppm | -7,8 / 20,4 | -7,7 / 20,6 |
| 45 baud | -7,2 / 23,8 | -7,2 / 24,0 |
| stop 1 bit | -8,4 / 18,8 | -8,4 / 18,9 |
| stop 2 bit | -8,5 / 19,2 | -8,4 / 19,3 |
| scariche | -7,9 / 19,6 | -7,9 / 19,9 |
| filtro 250 Hz | -8,2 / 18,8 | -8,1 / 19,0 |
| audio tagliato | -6,9 / 24,2 | -7,0 / 24,3 |
| AGC che pompa | -2,8 / 29,3 | -3,6 / 24,6 |

CER medio 26,61 -> 26,35 %; nessuna riga peggiora di oltre 0,3 punti (gli scenari a rumore bianco salgono tutti di 0,1-0,3: piccolo
costo sistematico, non rumore della misura); 182 caratteri al minuto sul solo rumore (prima 185), 0 con gli squelch.

**Scartate (numeri)**:
- stessa correzione senza attesa di 3 bit (soglia 0,85, media 0,5 bit): CER medio 25,50, AGC -10,0, flutter -9,9, alte latitudini
  -8,0, CCIR cattivo -4,6 punti, ma rumore 170/75 +1,7 e 9 righe oltre +0,5, 206 caratteri inventati: fuori regola. Con l'attesa di 1
  bit rumore 170/75 +1,2, di 2 bit +0,8 (sempre fuori regola). Il margine sul fading veloce c'e': serve un rilevatore del salto che non
  scatti sulle fluttuazioni del rumore (prossimo passo).
- solo nella catena principale (non nelle varianti): CER medio 25,63 ma rumore 170/75 +1,4.
- variante normalizzata con media del rumore piu' veloce (salita 1 bit, media dei due canali invece del minimo): AGC -4,4 ma CER medio
  26,68 e space -12 dB +0,9; salita 4 bit: AGC -2,4, CER medio 26,63. Il rumore piu' veloce fa vincere alla fusione caratteri sbagliati
  della variante in tutti gli scenari.

Le soglie s10 di CCIR medio e degli scenari col fading saltano di 1-2 dB per variazioni di CER di pochi decimi (curva piatta intorno al
10 %): per questi si guarda il CER medio.

## Caratteri inventati a squelch spento (2026-11-14)

Problema dalla radio vera: a squelch spento (impostazione di base del servizio, `squelch_on: false`) il ricevitore stampa righe di lettere
casuali sul rumore e fra una trasmissione e l'altra; 182 caratteri al minuto sul solo rumore della tabella. Lo squelch statistico di
base (0,7 / 0,5) li toglie tutti ma toglie anche testo vero: CER medio da 26,35 a 29,51 %, tutte le 25 righe peggiorano (alte
latitudini +11,9, portante +30 Hz +7,4): non si puo' accendere di base.

**Studio**: probabilita' dei caratteri sul solo rumore (3 minuti, 631 caratteri): 62 % sotto 0,1, 83 % sotto 0,2, 93 % sotto 0,3. Sul
segnale (prova `taratura`) i caratteri sotto 0,2 sono giusti il 15-21 % delle volte, quelli sopra 0,9 il 98 %. Un cancello con le regole
dello squelch statistico (media degli ultimi quattro caratteri, i primi trattenuti mentre decide) e soglie basse separa le due cose.

**Tenuto**: cancello interno sempre attivo quando lo squelch statistico dell'utente e' spento (`GATE_OPEN` 0,4, `GATE_CLOSE` 0,2); vale
anche per le impostazioni gia' salvate con `squelch_on: false`, perche' non dipende da un valore delle impostazioni. Test
`a_squelch_spento_il_rumore_fra_due_trasmissioni_non_stampa_spazzatura` (rumore, trasmissione, rumore, trasmissione, rumore): senza il
cancello 80 caratteri inventati, con al massimo 10; il testo vero c'e' tutto.

| scenario | prima s10 / CER | dopo s10 / CER |
|---|---|---|
| rumore 170/45 | -8,0 / 19,1 | -8,2 / 18,7 |
| rumore 170/75 | -5,6 / 29,6 | -5,9 / 27,9 |
| rumore 425/50 | -8,2 / 21,6 | -8,2 / 21,1 |
| rumore 850/45 | -8,2 / 19,3 | -8,3 / 19,1 |
| rumore 85/45 | -8,3 / 19,0 | -8,4 / 18,5 |
| fuori +10 Hz | -7,8 / 20,9 | -7,9 / 20,5 |
| space -12 dB | -5,4 / 28,2 | -5,4 / 28,0 |
| CCIR buono | -0,8 / 28,6 | -0,8 / 28,6 |
| CCIR medio | 1,3 / 31,9 | 1,2 / 31,6 |
| CCIR cattivo | -- / 36,5 | -- / 36,0 |
| flutter 10 Hz | -- / 56,8 | -- / 56,8 |
| alte latitudini | -- / 67,0 | -- / 67,2 |
| QRM a 250 Hz +6 | -8,4 / 19,9 | -8,4 / 19,5 |
| QRM a 120 Hz -6 | -8,3 / 19,8 | -8,4 / 19,5 |
| portante +30 Hz | -3,2 / 30,1 | -3,3 / 29,9 |
| PSK31 a 150 Hz | -8,0 / 19,5 | -8,3 / 19,1 |
| scheda +500 ppm | -7,8 / 20,6 | -7,9 / 20,2 |
| scheda -500 ppm | -7,7 / 20,6 | -7,8 / 20,4 |
| 45 baud | -7,2 / 24,0 | -7,3 / 23,9 |
| stop 1 bit | -8,4 / 18,9 | -8,5 / 18,6 |
| stop 2 bit | -8,4 / 19,3 | -8,5 / 18,8 |
| scariche | -7,9 / 19,9 | -8,0 / 19,4 |
| filtro 250 Hz | -8,1 / 19,0 | -8,3 / 18,7 |
| audio tagliato | -7,0 / 24,3 | -7,0 / 24,1 |
| AGC che pompa | -3,6 / 24,6 | -3,6 / 24,4 |

CER medio 26,35 -> 26,02 %, nessuna riga peggiora di oltre 0,2 punti; solo rumore 182 -> 18 caratteri al minuto. Registrazione vera di un
contest (121 s, squelch spento): stazione forte 283 -> 261 caratteri stampati, tutto il testo vero resta («CQ TEST PR1T», «LY6A LY6A»,
«OH9VD 599 276», «PR1T QRZ?») e si accorciano le righe casuali; stazione debole 368 -> 102 caratteri, resta «SP5H 599 150» e la
spazzatura scende a poche righe. Restano righe casuali con probabilita' alta: servira' altro.

**Scartate (CER medio / caratteri al minuto sul rumore)**: 0,7/0,5 29,51 / 0; 0,5/0,3 26,58 / 0 (alte latitudini +1,7, portante +1,2,
CCIR buono +0,8); 0,4/0,25 26,12 / 5 (alte latitudini +0,6); 0,35/0,22 26,09 / 18; 0,3/0,2 26,14 / 33; 0,2/0,12 26,28 / 161.
