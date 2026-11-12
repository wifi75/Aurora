# PSK e CW oltre tutti gli altri programmi (progetto, 2026-11-12)

Parte da «L1», cioè da Aurora dopo il lavoro di `docs/analisi-psk-cw.md` (filtro adattato, sincronismo, AFC con DCD, squelch su metrica,
FIR e AGC del CW, velocità a coppie, filtro degli stati brevi, segni di procedura, SOM di fldigi). Le cifre in dB sono **stime da misurare**.
Il vantaggio su fldigi, MMVARI, MRP40 e DigiPan verrà da tre cose:
1. **decisioni probabilistiche** al posto delle soglie: per il CW un decodificatore bayesiano a traliccio (Bell 1977); per il PSK
   la varicode decodificata con Viterbi su bit soft;
2. **modello del testo e dei nominativi** costruito su dati che Aurora ha già (DXCC, registro, spot del cluster);
3. **decodifica di tutti i segnali della banda audio** con lo storico di ciascuno (tutti i segnali CW, browser PSK).

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
- **I4** thread «tutta la banda» separato in `service.rs` con buffer di 60 s, perché il ricevitore principale non perda blocchi.

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

## Più segnali insieme (tutti i segnali CW, browser PSK)
Modulo `mode-digi/src/skim/`: canalizzatore FFT (rustfft), rivelatore senza griglia fissa (CW: inviluppo bimodale; PSK: righe a ±baud/2,
riga del quadrato, ciclostazionarietà), un decodificatore completo L1+L2 per segnale (64 canali CW, 30 PSK, CPU stimata < 10% di un core
con 40 canali CW), storico di 60 s per canale (il clic sintonizza e mostra tutto il passaggio), estrazione dei nominativi con DXCC e «già
lavorato», spot locali verso il cluster (in rete solo su richiesta e con alta confidenza). Con I/Q largo via TCI lo stesso canalizzatore
coprirebbe 48-192 kHz.

Interfaccia: pannello Tutta la banda/Browser (frequenza, WPM o modo, S/N, testo, nominativo con bandiera, CQ), nominativi sulla cascata,
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
- **Pacchetto WAV** (12 e 48 kHz, testo di riferimento, `elenco.csv`) ed esempio `punteggio` per confrontare MRP40, DigiPan,
  MultiPSK sul PC del proprietario; registrazioni vere con testo noto (esercitazioni CW e bollettini BPSK31 di W1AW) e di contest.

## Piano (dopo L1), un commit per passo
0. banchi di prova, fldigi arbitro per PSK e CW, soglia S/N e fattore di tempo reale, pacchetto WAV e `punteggio`;
1. confidenza ed eventi, modello del testo, grammatica dei nominativi;
2. PSK: P1, P3, P5, P2, P4, P6, P9;
3. CW: C2, C1, C3, C4, C5, C6+C7;
4. più segnali: canalizzatore e rivelatore, insieme di decodificatori con storico, nominativi e spot, P7, P8;
5. interfaccia (sette lingue);
6. risultati contro fldigi e contro gli altri programmi, Guida; facoltativo C8.

## Risultati (2026-11-12, ramo `lavoro/cw-psk-oltre`)
Misure con `tests/cw_confronto.rs` e `tests/psk_confronto.rs` (feature `fldigi-oracle`, fldigi 4.2.13 come arbitro, 2 prove per casella;
caratteri sbagliati in %, Levenshtein sul testo mandato, rumore e testo inventato prima e dopo compresi).

### Fatto
| Passo | Dove | Note |
|---|---|---|
| C1 traliccio con velocità nell'ipotesi | `cw_bayes.rs` | 30 velocità 5-60 WPM, fascio 320 (96 nei canali di banda), somme coerenti a pezzi di un'unità, Rice contro Rayleigh, rumore correlato dal filtro, miscela di ampiezze più quella adattata al tono, decisione con due caratteri di ritardo, probabilità per carattere (I1 nel ricevitore, non ancora negli eventi) |
| C2 pugno | `cw_bayes.rs` | cinque classi log-normali, medie e larghezze che si adattano piano sul percorso migliore (dopo 20 elementi, entro limiti) |
| C3 modello del testo | `cw_bayes.rs` | coppie di caratteri da un testo di QSO scritto per Aurora, peso 0,5; ogni carattere costa la sua probabilità (spezzare non conviene) |
| C5 filtro | `cw.rs` | invece del banco di filtri: somma coerente sull'elemento intero (filtro adattato a ogni durata) e filtro di almeno 150 Hz per il decodificatore |
| C6 noise blanker | `blanker.rs` | quantile 95% di |x|, buco di ±2 ms, segnale nuovo riconosciuto dopo 6 ms |
| C7 in parte | `cw.rs` | scarto fine del tono dallo spettro e AFC solo con il picco fermo per due spettri; scarto compensato campione per campione con l'AFC |
| Tutta la banda | `skim/` | spettro 2048 punti, canali aperti e chiusi da soli, storico 60 s, nominativi (ruoli, verifica, convalida, locatori esclusi), spot locali; libreria, non collegata al servizio |
| P1 bit morbidi | `psk.rs` | prodotto col simbolo prima diviso per la potenza di 4 simboli, scala dai momenti secondo e quarto |
| P3 varicode con fascio | `psk_viterbi.rs` | 96 ipotesi sull'albero dei codici, modello del testo leggero, due caratteri di ritardo |
| P5 squelch a probabilità | `psk_viterbi.rs` | carattere stampato solo sopra 140/255 |

### Non fatto
C4 (seconda passata all'indietro), C8, P2, P4, P6, P7, P8, P9; I2 come modulo `lm.rs` comune e grammatica dei nominativi dentro il traliccio; I3 eventi con
`channel`/`conf`/`revise_from`; I4 thread di tutta la banda nel servizio e pannello nell'interfaccia; pacchetto WAV ed esempio `punteggio`.

### CW contro fldigi (media di 9 righe)
| | L1 | ora | fldigi |
|---|---|---|---|
| media di tutte le righe | 45,7 | **24,2** | 73,6 |
| rumore -8 / -10 / -12 / -15 dB | 30,9 / 68,3 / 92,6 / 77,9 | **1,6 / 8,3 / 26,2** / 82,0 | 98,4 / 100 / 99,2 / 100 |
| tasto a mano (media) | 38,7 | **23,6** | 70,9 |
| 14 -> 28 WPM | 45,7 | **20,6** | 75,6 |
| QSB 15 dB | 65,2 | **54,1** | 89,5 |
| QRN | 63,8 | **21,2** | 77,4 |
| QRM +150 Hz | 37,9 | **19,9** | 68,9 |

Soglia del 5% di errori nel rumore bianco (interpolata fra le colonne): L1 circa -5 dB, ora circa -9 dB, fldigi circa -1 dB. Sul solo rumore (un minuto, squelch spento) nessun
carattere. Tutta la banda: 10 stazioni a -5 dB, 10 canali, 10/10 nominativi verificati, 10,5 volte più veloce del tempo vero (circa 1% di un core per canale).

### PSK contro fldigi (media di 9 righe)
| | L1 | ora | fldigi |
|---|---|---|---|
| PSK31 | 17,2 | **14,0** | 20,5 |
| PSK63 | 35,2 | **29,6** | 37,8 |
| PSK31 rumore -10 / -12 / -15 dB | 10,3 / 14,9 / 47,3 | **1,5 / 12,9** / 59,0 | 9,3 / 16,3 / 73,7 |
| PSK31 fading CCIR (media) | 29,0 | **29,8** | 33,1 |
| caratteri falsi in un minuto di rumore, squelch spento | 262 | **35** | — |

A +5 dB gli errori scendono dal 7-15% (caratteri inventati sul rumore prima e dopo il testo) a 0,5-4%. Nessuna riga peggiore di fldigi.

## Tutta la banda nella pagina e migliorie PSK (2026-11-12, ramo `lavoro/banda-cw-psk`)

### Fatto
| Cosa | Dove | Note |
|---|---|---|
| I4 thread di tutta la banda | `band.rs` | copia dell'audio a 12 kHz dal ricevitore principale con coda sua (se resta indietro perde blocchi suoi); limite di CPU: oltre un quarto di un core chiude i canali più deboli, sotto la metà ne riapre (fino a 40); evento `band` al massimo due volte al secondo e solo se cambia; rotta `GET /api/v1/digi/band` con lo storico di 60 s; impostazioni `band_cw`, `band_psk`, `band_spots` (accese) |
| I3 in parte | `aurora-core/src/event.rs` | `DigiSignal` con frequenza, WPM, S/N, coda del testo, posizioni dei caratteri incerti (probabilità sotto 150/255), nominativi con ruolo, verifica e paese |
| Pannello «Segnali» | `app/src/components/digi/BandSignals.tsx` | una riga per segnale, clic per sintonizzare, doppio clic per lo storico, incerti in grigio, CQ evidenziati e contati; nominativi sopra la traccia nella cascata; tema scuro e chiaro |
| Spot locali | `aurora/src/lib.rs` (`wire_band_decoder`) | nominativi verificati nel pannello del DX Cluster (`add_own_spot`, fonte «Aurora»), frequenza = radio + tono; mai in rete |
| Canali CW | `skim/mod.rs`, `cw.rs` | AFC dei canali limitato a ±20 Hz (con ±100 Hz nelle pause saltava sulla stazione vicina: con 30 stazioni a 113 Hz si trovavano 22 nominativi, ora 30); finestra e rotazioni dello spettro dell'AFC calcolate una volta (stessi numeri, CPU da 31% a 23% con 30 canali) |
| Browser PSK | `skim/psk.rs` | energia larga quanto un PSK sopra il fondo, portanti pure scartate, canale in prova finché lo squelch sulla qualità della fase non si apre (se no la frequenza si scarta per un minuto); frequenza dal baricentro, poi l'AFC |
| P4 | `psk.rs`, `blanker.rs` | noise blanker senza ritardo davanti al mixer: senza scariche l'uscita è identica bit per bit, con il QRN PSK31 dal 27,7% al 12,6% e PSK63 dal 41,6% al 30,3% |

### CPU di tutta la banda (un minuto di audio, `cpu_tutta_la_banda` e `tutti_i_segnali`)
| | 10 segnali | 30 segnali |
|---|---|---|
| CW a -5 dB | 10/10 nominativi, 8% di un core | 30/30 nominativi, 23% di un core |
| PSK31 a -8 dB | 10/10 nominativi, 0,3% di un core | 30/30 nominativi, 0,9% di un core |

### PSK contro fldigi dopo P4 (10 righe: aggiunta la riga QRN, scariche di 1-3 ms come nella prova CW)
| | prima | dopo | fldigi |
|---|---|---|---|
| PSK31 media | 15,4 | **13,9** | 21,5 |
| PSK63 media | 30,8 | **29,7** | 38,9 |
| PSK31 QRN | 27,7 | **12,6** | 30,5 |
| PSK63 QRN | 41,6 | **30,3** | 48,2 |

Le altre 18 righe sono identiche; caratteri falsi sul solo rumore invariati (35 PSK31, 76 PSK63 con lo squelch spento; 0 acceso).

### Provati e scartati (peggiorano almeno una riga)
- **P2** riferimento di fase su più simboli con decisioni (dure, poi morbide con tanh(LLR/2)), rotazione per simbolo stimata e ripartenza sul preambolo: a -12 dB in PSK31 dal 10,3% al 6,4% e fading dal 13,0% all'11,7%, ma a -8 dB dall'1,5% al 2,2%, 30 s di rumore dall'1,5% al 2,7%, PSK63 dal 6,4% all'8,1% (prova `studio`, peso 0,3; con 0,5 e 0,7 peggio). Anche usato solo con il segnale debole non migliora tutte le righe.
- **P6** aggancio rapido con il quadrato dei simboli del preambolo (rotazione media di 16 simboli, solo con rotazioni concordi e scarti oltre baud/16): PSK63 fuori -6 Hz da 29,7 a 28,8, ma PSK31 vicina +6 dB da 10,2 a 10,3 (a -15 dB); con soglie diverse altre righe peggiori di 0,2-1,0.
- **P9** orologio della scheda come seconda costante del sincronismo (solo con il DCD): con guadagno 0,001 otto righe peggiori, con 0,0001 cinque (PSK63 rumore +0,8, vicina +0,6).
- **P4 con cancellazioni** (LLR = 0 sui simboli con un buco del blanker): QRN PSK31 dal 12,6% al 39,0%; resta il solo blanker.

### Convalida dei nominativi e regole degli spot
| Cosa | Dove | Note |
|---|---|---|
| Forme dei nominativi | `skim/forms.rs`, `aurora/src/lib.rs` (`spawn_call_forms`) | forma = prefisso letterale di 2-3 caratteri (con un prefisso di una lettera entra la cifra: K1, A6) + cifre + lettere del suffisso, del nominativo di base (`X/CALL`, `CALL/P`); contate per paese DXCC dal log di Aurora, dagli spot del DX Cluster e da `MASTER.SCP` nella cartella dei dati, rifatte ogni mezz'ora; frequenza relativa = probabilita' a priori; tipica >= 1%, insolita 0,05-1%, sospetta sotto; paesi con meno di 200 nominativi non giudicati |
| Serie ITU | `skim/forms.rs` | scartate Q, quelle che cominciano con 0 o 1 e le coppie lettera-cifra non assegnate (per le lettere divise fra piu' paesi); un prefisso che `cty.dat` conosce vince sempre |
| Livelli | `skim/calls.rs` | Minima (forma), Normale (paese DXCC, sospetti solo verificati), Severa (niente sospetti, insoliti solo verificati), Massima (solo verificati di forma tipica); la probabilita' a priori entra nella verifica: punteggio 1,5 per una forma tipica, 2 per una insolita, 3 e tre ricorrenze per una sospetta. Impostazione `band_validation`, tendina nel pannello con il suggerimento |
| Regole degli spot | `skim/mod.rs`, `band.rs` (`SpotGate`, `BandSpot::comment`) | S/N in 500 Hz da spettri a tasto abbassato (riga del tono 6 dB sopra il fondo; dei 15 s si tengono quelli sopra il 70% del 90° percentile e se ne prende la mediana: con tono 0,2 e rumore 0,05 la prova da' 19,8 ± 3 dB come atteso); commento `CW 22 WPM 14 dB CQ` o `DE`; solo verificati; uno per nominativo e banda ogni 10 minuti, di nuovo oltre 1 kHz o al passaggio a CQ; `band_spots_cq_only` |

La probabilita' a priori non entra nel modello del testo del decodificatore CW (`cw_bayes.rs`): resta nella verifica dei nominativi.

### Non fatto
P7, P8, C4, C8; I2 come modulo comune; I3 nel testo del ricevitore principale (`channel`, `revise_from`); spot in rete dal browser (per scelta: solo locali); pacchetto WAV ed esempio `punteggio`.

## Fonti
E. L. Bell, tesi NPS 1977
<https://archive.org/details/optimalbayesiane00bell> · AG1LE <https://ag1le.blogspot.com/2013/09/new-morse-decoder-part-1.html> ·
MRP40 <https://www.polar-electric.com/Morse/MRP40-EN/features.html> · MMVARI <https://github.com/n5ac/mmvari> · DigiPan
<https://www.digipan.net/> · WinWarbler <https://dxlabsuite.com/winwarbler/Help/BroadbandDecode.htm> · specifica PSK31 ARRL
<https://www.arrl.org/psk31-spec>. Divsalar e Simon (IEEE Trans. Commun. 38(3), 1990) e Bauer e Hagenauer (2000) citati a memoria, da verificare.
