# RTTY oltre tutti gli altri programmi (progetto, 2026-11-12)

Punto di partenza: il ricevitore rifatto il 2026-11-12 (`core/crates/mode-digi/src/rtty.rs`): centro fra i toni e mark alto in USB, filtro a
coseno rialzato K=1,35 (fldigi), ATC con livelli acceso/spento per tono, start come `is_mark_space` (fldigi), PLL di bit (), UOS, squelch su
S/N, discriminatore sotto 40 Hz (). Prove: `tests/rtty_confronto.rs`, `tests/rtty_fldigi.rs` (fldigi arbitro, feature `fldigi-oracle`).
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
- **** (, chiuso): **bayesiano**, probabilita' d'errore per bit e per carattere, caratteri incerti in chiaro, **squelch sulla
  probabilita'**, fusione bayesiana dei nominativi simili, correzioni di registro, numeri, CQ e DE, conoscenza dei QSO e dei contest.
- fldigi e : gia' portati (vedi `docs/analisi-psk-cw.md` e il CHANGELOG).

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
  decodificatore) per confrontare anche 2Tone e  sul PC del proprietario; bollettini RTTY di W1AW come testo noto.

## Piano (un commit per passo)
1 scenari comuni e misure · 2 MMTTY arbitro · 3 tabella Aurora/fldigi/MMTTY · 4 raccolta WAV e valutazione · 5 front-end decimato · 6 LLR e rumore in
banda · 7 decodifica del carattere intero · 8 caratteri con affidabilita' (eventi e interfaccia) · 9 squelch statistico · 10 soppressore e
limitatore · 11 varianti e fusione · 12 registro e prior del contest · 13 ripetizioni e nominativi · 14 AFC e tolleranze · 15 multicanale ·
16 diversity. Ogni passo dal 5 in poi si chiude solo se la tabella non peggiora in nessuno scenario.

Fonti: [W0YK CTU 2019](https://www.contestuniversity.com/wp-content/uploads/2019/05/13-W0YK-Taking-Digital-Contesting-to-the-Limit_2019.pdf),
[2Tone Operational Information](https://www.sp2pby.pl/pdf/2Tone%202102a.pdf), [manuale 2Tone](http://ok2pad.webz.cz/logger32/g3yyd_2tone.pdf),
[W8TN su Repeat Match](https://w8tn.blogspot.com/2016/11/new-feature-of-2tone-rtty-decoder.html), [](https:///),
[ demo](https:////Demo), [decodificatore Server](https://www.//Files/.pdf).
