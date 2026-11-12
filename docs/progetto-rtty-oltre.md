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
