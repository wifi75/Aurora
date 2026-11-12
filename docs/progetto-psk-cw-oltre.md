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

## CW: rumore istante per istante, aggancio rapido, anti-click (2026-11-12, ramo `lavoro/cw-resto`)
Misure con `tests/cw_confronto.rs` (2 prove per casella, 11 righe: alle 9 di prima si aggiungono «AGC che pompa», una stazione forte
nella banda che fa scendere il guadagno della radio di 20 dB a ogni elemento, e «QRN largo», scrosci di 50-300 ms 8-14 dB sopra il
fondo) e con la prova nuova `avvio_a_freddo` (ricevitore nuovo come dopo un cambio di modo, stazione a velocità diversa da quella di
partenza per 15 s, 3 s di silenzio, un'altra stazione a un'altra velocità per 30 s; 4 prove per riga). fldigi non c'era come arbitro
(sottomodulo assente): i confronti sono fra versioni di Aurora.

### Tenuto
| Cosa | Dove | Prima | Dopo |
|---|---|---|---|
| Rumore istante per istante: due filtri ai lati del tono, larghi due volte la velocità di simbolo (30 Hz a 20 WPM, 60 Hz a 40), centrati a 2,5 larghezze (fascia 1,5-3,5), uno solo se dall'altra parte c'è una stazione (medie di 1 s); minimo scorrevole (medie di 40 ms, minimo su 3 s, diviso per il valore atteso del minimo, tarato nella prova `noise_estimates_match_the_decoder_filter`: le medie mobili valgono circa 600 medie indipendenti); nel traliccio la varianza di ogni tono è quella misurata in quei campioni, a metà strada (in dB) con la sua media lenta e solo se credibile rispetto al rumore delle pause | `cw_noise.rs`, `cw.rs`, `cw_bayes.rs` | media di tutte le righe 28,1 | **26,5** |
| — AGC che pompa / QSB / tono +25 Hz / QRN / QRN largo | | 39,4 / 54,1 / 24,7 / 21,2 / 51,5 | **26,7 / 52,1 / 20,5 / 21,1** / 51,9 |
| — rumore bianco / tasto a mano / 14 -> 28 WPM | | 16,9 / 23,6 / 20,6 | 16,9 / 25,3 / 22,0 |
| Scariche come «cancellati»: il blanker dice quali campioni ha azzerato, il decodificatore riceve per ogni campione il guadagno e la varianza rimasti nel filtro (un istante cancellato non è né tono né pausa) | `blanker.rs`, `cw.rs`, `cw_bayes.rs` | QRN 21,2 (riga senza cancellazioni, con il rumore istantaneo) | 19,3 (stessa prova, media 26,0 -> 25,8) |
| Stime del rumore che si agganciano subito (media lenta che all'inizio è la media di tutto, larghezza dei filtri laterali che segue solo la velocità misurata) | `cw_bayes.rs`, `cw_noise.rs` | avvio a freddo, primi 15 s a 0 e -6 dB: 1,4% (con il rumore istantaneo senza aggancio) | **0,0%** come il ricevitore di partenza |
| Velocità mostrata da tono + pausa che lo segue (il filtro allunga i toni e accorcia le pause della stessa quantità) | `cw_bayes.rs` | a gradini del 9% (le velocità del traliccio) | entro il 5% (prova `the_received_speed_is_measured_within_five_percent`) |
| Aggancio rapido del ricevitore a soglia (quello della 0.11.240, della decodifica SOM e del probabilistico spento): prime quattro coppie con peso 0,6, coppie anche fra caratteri chiusi troppo presto, AGC il doppio più veloce per 2 s, aggancio di nuovo dopo 2,5 s di silenzio | `cw.rs` | avvio a freddo, 0 dB: primi 15 s 7,1%, dopo 12,4% (30 -> 16 WPM: 78,8%); -6 dB: 16,1% e 28,5% | 0 dB: **4,1% e 4,4%** (3,6%); -6 dB: **13,5% e 13,9%** |
| Anti-click della cascata: rivelatore dei click (spettri di 5 ms ogni 2,7 ms, un tratto di 1 kHz «sale» se più del 30% delle righe sta 12 dB sopra il livello tipico, click se dura meno di 10 ms), i campioni del click si spengono con fronti morbidi prima della trasformata delle righe; istanti dei click in `SpectrumAnalyzer::recent_clicks` e `ClickDetector` per i canali di tutta la banda (non ancora collegati) | `audio-spectrum/src/click.rs`, `analyzer.rs` | CW manipolato duro: 17,4 dB di energia in più a 1,5-2,5 kHz | **1,4 dB**, tono -0,6 dB |
| «TX alla velocità ricevuta» (menu Modo, tasto «TX = RX») e interruttore anti-click (menu Vista, acceso di base, vale mentre la pagina ascolta) | `DigiPage.tsx`, `StatusBar.tsx`, `service.rs` | — | — |

Il ricevitore probabilistico non aveva il problema dell'avvio a freddo segnalato sulla 0.11.240 (che usa quello a soglia): a 0 e -6 dB
nessun errore nei primi 15 s, prima e dopo. A -10 dB i primi 15 s sbagliano in media un po' più di prima (5,6% contro 4,3% su tutte
le righe della prova, soprattutto le stazioni a 28 WPM), i 30 s dopo restano uguali (6,2% contro 5,9%). Sul solo rumore, un minuto,
nessun carattere falso con lo squelch spento e acceso.

### Scartato (misurato, non migliorava)
| Cosa | Risultato |
|---|---|
| Velocità del ricevitore a soglia come media delle ultime 4 soglie punto/linea (soglia = minima fra metà di punto + linea e punto + pausa interna), antidisturbo a 0,3 unità (minimo 6 ms), velocità a soglia come guida iniziale del traliccio | probabilistico 28,1 -> 28,1; a soglia 45,3 -> 48,4 (soglie dalle medie) e 49,4 (soglie di ogni coppia); con 0,3 unità di antidisturbo a +5 dB gli errori salgono da 1,6 a 4,0% |
| Rumore delle bande laterali senza restringimento verso la media lenta, medie di 40 ms | 30,4 (peggio a -8 e -10 dB e col tasto a mano); medie di 250 ms 27,3 |
| Rumore istantaneo moltiplicato per 2,5 (come la stima vecchia, che sovrastimava) | 43,3 |
| Pavimento del rumore al 70% del minimo scorrevole | 29,3 |
| Rumore istantaneo mai sotto la stima delle pause | 29,3 |
| Limite del rapporto segnale/rumore per campione che scende con il rumore misurato (per l'AGC che pompa) | rompe i segnali puliti fra silenzi digitali (prove `it_hears_what_it_sends_at_several_speeds`) |
| AGC a due inseguitori (bordo alto attacco 1 ms e rilascio 2 s, bordo basso sale in 2 s e scende in 50 ms) come normalizzazione all'ingresso del decodificatore: Aurora non ha un ascolto audio della ricezione digitale | 26,9 contro 25,9 (meglio AGC che pompa 22,4 e QRN largo 42,9, peggio QRM, segnale forte altrove e +5 dB) |
| Sostituzione delle celle della cascata con la mediana di tre righe vicine | 17,7 dB contro 17,4: la finestra delle righe (0,14 s) è larga cinque righe, un click ne copre quattro o cinque |

### Resta
Il tasto a mano e il salto 14 -> 28 WPM perdono 1,4-1,7 punti con il rumore istantaneo; la riga QRN largo resta sopra il 50% (a -8 dB il
traliccio perde la velocità); il ricevitore a soglia perde 2,3 punti nella tabella (colonne a -8 e -10 dB) in cambio dell'aggancio
rapido; filtro del tono legato alla velocità dentro il decodificatore (oggi solo per le bande laterali: il traliccio fa già la somma
coerente sull'elemento); collegare `ClickDetector` ai canali di tutta la banda.

## Fonti
E. L. Bell, tesi NPS 1977
<https://archive.org/details/optimalbayesiane00bell> · AG1LE <https://ag1le.blogspot.com/2013/09/new-morse-decoder-part-1.html> ·
MRP40 <https://www.polar-electric.com/Morse/MRP40-EN/features.html> · MMVARI <https://github.com/n5ac/mmvari> · DigiPan
<https://www.digipan.net/> · WinWarbler <https://dxlabsuite.com/winwarbler/Help/BroadbandDecode.htm> · specifica PSK31 ARRL
<https://www.arrl.org/psk31-spec>. Divsalar e Simon (IEEE Trans. Commun. 38(3), 1990) e Bauer e Hagenauer (2000) citati a memoria, da verificare.
