# PSK e CW oltre tutti gli altri programmi (progetto, 2026-11-12)

Parte da «L1», cioè da Aurora dopo il lavoro di `docs/analisi-psk-cw.md` (filtro adattato, sincronismo, AFC con DCD, squelch su metrica,
FIR e AGC del CW, velocità a coppie, filtro degli stati brevi, segni di procedura, SOM di fldigi). Le cifre in dB sono **stime da misurare**.
Il vantaggio su fldigi, , MMVARI, decodificatore, MRP40 e DigiPan verrà da tre cose:
1. **decisioni probabilistiche** al posto delle soglie: per il CW un decodificatore bayesiano a traliccio (Bell 1977, decodificatore); per il PSK
   la varicode decodificata con Viterbi su bit soft;
2. **modello del testo e dei nominativi** costruito su dati che Aurora ha già (DXCC, registro, spot del cluster);
3. **decodifica di tutti i segnali della banda audio** con lo storico di ciascuno (decodificatori CW, browser PSK).

Da L1 servono due uscite: per il PSK il campione complesso di ogni simbolo dopo filtro adattato e sincronismo, con la stima del rumore;
per il CW la banda base complessa decimata (circa 500 Hz) dopo il FIR, prima dell'isteresi.

## Riferimenti teorici
- PSK31: S/N in 3 kHz = Eb/N0 − 19,8 dB (PSK63 −16,8 dB). DBPSK a decisione dura, CER 2% ≈ −12 dB in 3 kHz. La rivelazione coerente guadagna
  solo ~0,6 dB in rumore bianco: i guadagni veri sono nel QRN, nel QRM, nel fading, nella diversità e nella decodifica multipla.
- CW: a 20 WPM S/N in 3 kHz = Eb/N0 − 22,6 dB; ogni raddoppio di velocità costa 3 dB. Fra un rivelatore a soglia e un decodificatore a
  sequenza probabilistica ci sono plausibilmente 3-5 dB.

## Infrastruttura comune
- **I1** caratteri con confidenza 0-255, stato provvisorio/definitivo, correzione degli ultimi caratteri (Viterbi con ritardo).
- **I2** modello del testo `lm.rs`: trigrammi di caratteri (CW maiuscolo, PSK sensibile alle maiuscole), lessico dei QSO, grammatica dei
  nominativi convalidata con `dxcc::DxccDb::lookup`, probabilità dal registro e dagli spot, MASTER.SCP facoltativo.
- **I3** eventi: `channel`, `seq`, `conf`, `revise_from` in `DigiEvent` (`aurora-core/src/event.rs`); tipo `browser` per l'elenco dei canali.
- **I4** thread «decodificatori» separato in `service.rs` con buffer di 60 s, perché il ricevitore principale non perda blocchi.

## CW
| # | Tecnica | Guadagno atteso | CPU | Rischi |
|---|---|---|---|---|
| C1 | traliccio bayesiano: stato = nodo dell'albero Morse × tono/pausa × durata × velocità (~20 valori 5-60 WPM); verosimiglianza Rice/Rayleigh sull'inviluppo; ampiezza con Kalman; fascio di 256-1024 percorsi, decisione ritardata di un carattere | 3-5 dB | ~6 MFLOP/s per canale | pavimento d'errore se il modello delle durate è rigido; latenza 0,5-1 s |
| C2 | modello del «pugno»: cinque durate log-normali stimate in linea (EM), scale separate per punto, linea e pause (bug, peso, Farnsworth) | CER dimezzato col tasto a mano | trascurabile | adattarsi al rumore: fermo con squelch chiuso |
| C3 | modello del testo e dei nominativi dentro il traliccio, peso regolabile | CER −20/40% vicino alla soglia | trascurabile | nominativi inventati: testo grezzo sempre visibile, spot solo con alta confidenza |
| C4 | seconda passata all'indietro sui 10-30 s in memoria | prima parola da ~30-50% a ~5% di errori | una passata in più | testo che cambia: segnalarlo |
| C5 | banco di 2-3 filtri adattati alla velocità, scelta per verosimiglianza | 1-2 dB sopra 35 e sotto 12 WPM | ×3 sul FIR | — |
| C6 | noise blanker prima del filtro stretto, campioni «cancellati» | CER dimezzato col QRN | trascurabile | soglia relativa al rumore |
| C7 | AFC del tono ±50 Hz (interpolazione del picco a tasto giù) e tolleranza al chirp (anche segnalazione #46) | evita la perdita con deriva | trascurabile | aggancio a un vicino |
| C8 | due stazioni sullo stesso tono (sperimentale) | separa pile-up di due stazioni | ×4 stati | complessità |

## PSK
| # | Tecnica | Guadagno atteso | CPU | Rischi |
|---|---|---|---|---|
| P1 | bit soft (LLR) dal campione di simbolo e dalla stima del rumore | base per P2-P5 | trascurabile | σ² sbagliato |
| P2 | riferimento di fase su più simboli (Divsalar e Simon) o anello guidato dalle decisioni, adattato al Doppler | 0,5-0,7 dB in AWGN, nessuna perdita in fading veloce | trascurabile | richiede AFC fine |
| P3 | Viterbi sulla varicode (~600 stati dell'albero dei codici) con LLR e modello del testo, decisione ritardata di 2-3 caratteri | 1-2 dB; zero caratteri spuri sul rumore | ~0,1 MFLOP/s | il modello forza il carattere comune: peso limitato |
| P4 | cancellazioni dei simboli colpiti dal QRN (LLR = 0) | 3-6 dB equivalenti col QRN | trascurabile | — |
| P5 | squelch sulla probabilità del carattere; incerti in grigio | rumore stampato verso zero | trascurabile | — |
| P6 | aggancio col quadrato del segnale (riga a 2f) e riconoscimento del preambolo (righe a ±15,625 Hz) | primi caratteri non persi, Snap a −10 dB | 1 FFT 4096/s | — |
| P7 | cancellazione del vicino forte (decodifica, rimodulazione e sottrazione) + LMS di MMVARI | segnali 20-30 dB sotto un vicino a 40-60 Hz | ~0,3 MFLOP/s per vicino | sottrazione sbagliata |
| P8 | diversità: due ingressi (TCI/SDR) combinati a rapporto massimo; più rami interni; fusione delle ripetizioni dei nominativi | 3 dB AWGN, 5-8 dB in Rayleigh | ×2 | serve l'hardware |
| P9 | correzione dell'orologio della scheda in ppm (come MMVARI) | 0,2-0,5 dB con ±300 ppm | trascurabile | — |

## Più segnali insieme (decodificatori CW, browser PSK)
Modulo `mode-digi/src/skim/`: canalizzatore FFT (rustfft), rivelatore senza griglia fissa (CW: inviluppo bimodale; PSK: righe a ±baud/2,
riga del quadrato, ciclostazionarietà), un decodificatore completo L1+L2 per segnale (64 canali CW, 30 PSK, CPU stimata < 10% di un core
con 40 canali CW), storico di 60 s per canale (il clic sintonizza e mostra tutto il passaggio), estrazione dei nominativi con DXCC e «già
lavorato», spot locali verso il cluster (in rete solo su richiesta e con alta confidenza). Con I/Q largo via TCI lo stesso canalizzatore
coprirebbe 48-192 kHz.

Interfaccia: pannello decodificatori/Browser (frequenza, WPM o modo, S/N, testo, nominativo con bandiera, CQ), nominativi sulla cascata,
caratteri incerti in grigio con le correzioni, impostazioni classica/avanzata, peso del modello del testo, «tasto a mano», canali massimi,
spot automatici, secondo ingresso. Testi in sette lingue.

## Prove
- Nuovi generatori in `tests/common/mod.rs`: pugno (macchina, mano regolare e molto irregolare, bug, peso, Farnsworth), QRN a impulsi,
  AGC della radio, clipping, deriva e chirp, banda con più stazioni; profili Watterson/CCIR con `fading_ccir`.
- Scenari: AWGN, CCIR good/moderate/poor, QRM vicino, QRN, tasto a mano, velocità 5-60 WPM e salti, deriva, chirp, orologio ±300 ppm,
  clipping e AGC, 60 s di solo rumore.
- Misure: soglia S/N per CER 2% e 5% (bisezione), CER sui primi 10 caratteri, nominativi esatti e sbagliati ma plausibili, caratteri
  falsi al minuto, tempo d'aggancio, WPM stimati, fattore di tempo reale con 1/10/40/64 canali.
- **fldigi come arbitro** anche per PSK e CW (stessa feature `fldigi-oracle` dell'RTTY): mai peggiori di fldigi (tolleranza 0,5 dB), in media
  +1 dB PSK AWGN, +3 dB PSK con QRN e fading, +3 dB CW AWGN, CER dimezzato CW a mano.
- **Pacchetto WAV** (12 e 48 kHz, testo di riferimento, `elenco.csv`) ed esempio `punteggio` per confrontare decodificatore, MRP40, DigiPan,
  , MultiPSK sul PC del proprietario; registrazioni vere con testo noto (esercitazioni CW e bollettini BPSK31 di W1AW) e di contest.

## Piano (dopo L1), un commit per passo
0. banchi di prova, fldigi arbitro per PSK e CW, soglia S/N e fattore di tempo reale, pacchetto WAV e `punteggio`;
1. confidenza ed eventi, modello del testo, grammatica dei nominativi;
2. PSK: P1, P3, P5, P2, P4, P6, P9;
3. CW: C2, C1, C3, C4, C5, C6+C7;
4. più segnali: canalizzatore e rivelatore, insieme di decodificatori con storico, nominativi e spot, P7, P8;
5. interfaccia (sette lingue);
6. risultati contro fldigi e contro gli altri programmi, Guida; facoltativo C8.

## Fonti
decodificatore <https:///Cw/> ·  <https:///> · E. L. Bell, tesi NPS 1977
<https://archive.org/details/optimalbayesiane00bell> · AG1LE <https://ag1le.blogspot.com/2013/09/new-morse-decoder-part-1.html> ·
MRP40 <https://www.polar-electric.com/Morse/MRP40-EN/features.html> · MMVARI <https://github.com/n5ac/mmvari> · DigiPan
<https://www.digipan.net/> · WinWarbler <https://dxlabsuite.com/winwarbler/Help/BroadbandDecode.htm> · specifica PSK31 ARRL
<https://www.arrl.org/psk31-spec>. Divsalar e Simon (IEEE Trans. Commun. 38(3), 1990) e Bauer e Hagenauer (2000) citati a memoria, da verificare.
