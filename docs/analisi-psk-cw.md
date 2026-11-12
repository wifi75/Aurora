# PSK e CW di Aurora a confronto con fldigi (2026-11-12)

Analisi di sola lettura del codice di Aurora (`core/crates/mode-digi/src/psk.rs`, `cw.rs`, `varicode.rs`, `service.rs`), di fldigi 4.2.13
(`terze-parti/fldigi/src/psk/psk.cxx`, `pskcoeff.cxx`, `pskvaricode.cxx`, `cw/cw.cxx`, `cw/morse.cxx`, `filters/filters.cxx`, `include/`). Le perdite in dB sono stime da misurare con le prove della sezione 5; gli errori della sezione 4
sono letti nel codice. La tabella varicode di Aurora e quella di fldigi coincidono su tutte le 256 voci.

## PSK31 / PSK63

### Come lavora Aurora
1. Campionamento a 12 kHz; 384 campioni per simbolo in PSK31, 192 in PSK63 (`psk.rs:14`, `:24-28`).
2. Mixer in f64 con e^(−jωt) (`:164-169`).
3. Un solo filtro: somma mobile lunga 2·decim (4 ms in PSK31), decimazione a 16 campioni per simbolo (`:104-105`, `:171-181`).
   Banda di rumore di circa 250 Hz (500 Hz in PSK63). **Nessun filtro adattato.**
4. Sincronismo: 16 «cassetti» di ampiezza mediati 0,97/0,03; si decide 8 cassetti dopo il minimo (`:183-191`). Risoluzione 1/16 di simbolo.
5. Decisione: bit 1 se Re(z·conj(prev)) > 0 (`:192-198`), solo se `mag > 1e-4` e squelch aperto.
6. AFC: guadagno 0,08 per simbolo, ±30 Hz, su ogni simbolo anche col solo rumore (`:200-206`).
7. Squelch sul livello assoluto in dB (`:195-196`); di base AFC acceso, squelch spento, −45 dB (`service.rs:255-257`).
8. Varicode: buffer `String`, fine carattere su `"00"`, svuotato oltre 12 caratteri (`:213-227`). Nessun DCD.
9. Nessuna misura di S/N né di IMD (`service.rs:704-706`).

### fldigi negli stessi passi
| Passo | fldigi 4.2.13 |
|---|---|
| Campionamento | 8 kHz, 256/128 campioni per simbolo (`psk.cxx:372`, `:384-392`) |
| Filtro 1 + decimazione | `fir1` Hann 65 coefficienti, /16 o /8 (`psk.cxx:942-947`, `pskcoeff.cxx:274-281`) |
| Filtro adattato | `fir2` = `pskcore_filter`, 65 coefficienti = 4 simboli (`pskcoeff.cxx:203-269`) |
| Sincronismo | anticipo/ritardo su `syncbuf`, correzione frazionaria (`psk.cxx:2061-2149`) |
| AFC | solo con DCD o acquisizione e `afcmetric ≥ 0,05`, guadagno 1/dcdbits (`:1442-1478`) |
| Squelch / DCD | metrica di fase 0-100 indipendente dal livello (`:1561-1609`); DCD sul preambolo `0xAAAAAAAA` (`:1611-1701`) |
| Varicode | registro a 32 bit, nessun limite (`:1116-1144`) |
| S/N e IMD | Goertzel (`:966-974`, `:1835-1892`) |

### Difetti di Aurora, dal più grave
| # | Difetto | Effetto | Soluzione | Da chi |
|---|---|---|---|---|
| 1 | Nessun filtro adattato | stima 7-9 dB persi sui segnali deboli; il QRM vicino entra quasi intero | `fir1` Hann (96 coefficienti a 12 kHz) e `fir2` `pskcore_filter` | fldigi (GPL compatibile) |
| 2 | AFC sul rumore, acceso di base con squelch spento | la frequenza vaga fino a ±30 Hz, fuori dal campo di aggancio (±7,8 Hz in PSK31) | AFC solo con DCD e metrica, guadagno 1/32 o 1/64 | fldigi, tenendo l'errore avvolto di Aurora |
| 3 | Squelch e soglia `1e-4` assoluti | dipende dal volume e dall'AGC | metrica di fase di fldigi + tenuta di 20 simboli | combinazione |
| 4 | Sincronismo a minimo quantizzato | simboli saltati o doppi con segnale debole | anticipo/ritardo frazionario sull'uscita del filtro adattato | fldigi |
| 5 | Codici varicode da 12 bit persi (`psk.rs:224`) | non si ricevono è é ì ò ù e gli altri da 232 a 255 | limite `> 13` | correzione di una riga |
| 6 | Nessun DCD, buffer non azzerato | caratteri sporchi all'avvio | DCD BPSK e azzeramento | fldigi |
| 7 | Nessuna misura S/N e IMD | manca all'operatore | Goertzel di fldigi, IMD valida dopo 21 inversioni di fila | combinazione |
| 8 | Trasmissione a piena ampiezza subito, coda di 8 simboli | click all'avvio | salita dal silenzio a mezzo coseno, coda di 32 simboli (fldigi) | combinazione |

## CW

### Come lavora Aurora
1. Mixer al tono e boxcar complesso di 6 ms a 12 kHz (`cw.rs:133`, `:253-264`): banda di circa 167 Hz, lobi a −13 dB.
2. Nessuna media dopo il rivelatore legata alla durata del punto.
3. Picco con discesa di 6 s, fondo uguale al minimo istantaneo (`:267-268`); controllo sulla potenza di tutta la banda (`:271-273`).
4. Decisione con isteresi a 0,55 / 0,40 fra fondo e picco (`:274-280`); squelch in dB assoluti (`:276-278`).
5. Ignorati i toni sotto 0,3 unità (`:231`), nessun filtro sulle pause; punto se dura meno di 2 unità (`:234`).
6. Velocità: media esponenziale al 30% su ogni elemento, senza convalida (`:236-241`).
7. Carattere dopo 2 unità di pausa, parola dopo 5 (`:294`, `:298`); manca SK.

### fldigi
| Passo | fldigi |
|---|---|
| Filtro | FIR Hamming 512 coefficienti, 150 Hz di base, decimazione a 800 Hz (`cw.cxx:80`, `:376-377`); filtro adattato facoltativo |
| Inviluppo | media mobile di 1/3 di punto (`cw.cxx:380-383`, `:804`), legata però alla velocità di trasmissione |
| AGC | medie salita 0,25 s / discesa 1,25 s, valori normalizzati (`cw.cxx:693-720`) |
| Squelch | metrica S/N 0-100 (`:725-732`) |
| Antidisturbo | toni sotto 1/2 punto (`:595`, `:921-927`) |
| Velocità | coppie punto-linea con rapporto 2-4, media su 16 (`:604-616`, `:936-947`) |
| Carattere e parola | carattere fra 2 e 4 punti, spazio oltre 4 (`:990-1022`) |
| Extra | `<AR> <SK> <KN>` (`morse.cxx:49-51`), decodifica SOM facoltativa |

### Difetti di Aurora, dal più grave
| # | Difetto | Effetto | Soluzione | Da chi |
|---|---|---|---|---|
| 1 | Nessuna media dopo il rivelatore, boxcar fisso di 6 ms | elementi spezzati con S/N moderato, QRM poco attenuato: stima 3-6 dB persi | decimazione a 500 Hz, FIR con banda selezionabile (150 Hz di base), media di 1/3 di punto legata alla velocità ricevuta | fldigi, con decimazione a 500 Hz |
| 2 | Picco a 6 s, fondo = minimo istantaneo | dopo un QRN la soglia resta alta; in QSB il segnale si perde | medie salita/discesa di fldigi, fondo sui tratti a tasto alzato | fldigi |
| 3 | Controllo sulla potenza di tutta la banda, soglie assolute | un CW debole zittito da un segnale forte altrove; con volume basso niente | via il controllo, squelch su S/N | fldigi |
| 4 | Velocità in media al 30% senza convalida | un disturbo sposta la velocità e la stima scappa | medie separate punto/linea, soglia a metà, solo coppie plausibili | fldigi, con soglia a metà |
| 5 | Nessun filtro sulle pause | una linea con un buco diventa due punti | ignorare stati brevi (toni e pause) a circa 0,35 unità, minimo 8 ms | nostro |
| 6 | Spazio fra parole a 5 unità | parole attaccate col tasto a mano | 4-4,5 unità | fldigi |
| 7 | Mancano SK e i segni di procedura | `*` al posto di SK | `<SK>`, facoltativi `<AR> <KN> <BT>` | fldigi |
| 8 | CW a mano irregolare | molti `*` | decodifica SOM facoltativa | fldigi |

## Errori evidenti
- **Varicode** (`psk.rs:224`): con `> 12` i codici da 12 bit (indici 232-255, `varicode.rs:64-69`) si perdono; basta `> 13`.
- **AFC del PSK sul rumore**: con squelch spento `open` è sempre vero (`psk.rs:196`) e l'AFC si muove fino a ±30 Hz.
- **Soglie che dipendono dal volume**: `1e-4` e dBFS in PSK e CW; lo stesso `squelch_db` ha due scale diverse nei due modi.
- **Polarità**: nel BPSK nessun errore (bit 1 = nessuna inversione, come fldigi; il segno dell'AFC è giusto). Per un futuro QPSK
  la regola dell'inversione va ricavata per il mixer di Aurora, verificata con un file di fldigi e collegata alla LSB dal CAT.
- **Difetto di fldigi da non copiare**: fase in [0, 2π) (`psk.cxx:1496`), l'AFC sui bit 1 usa solo gli errori positivi; l'errore avvolto
  di Aurora (`psk.rs:202-203`) è giusto.
- **CW**: il fondo è un rivelatore di minimo, non un livello di rumore; `pattern.len() > 8` (`cw.rs:243`) incoerente col commento.
- **Prove**: quelle attuali usano circa +15 dB di S/N; il CW non ha prove con rumore.

## Piano di lavoro
**Fase 0 — banchi di prova** sul modello di `tests/rtty_confronto.rs`: `tests/psk_confronto.rs` (rumore da +5 a −15 dB, fuori frequenza,
30 s di rumore e poi il segnale, deriva, fading CCIR, stazione vicina, errore d'orologio della scheda, livelli diversi, i 256 caratteri)
e `tests/cw_confronto.rs` (rumore, tasto a mano, salti di velocità, QSB, QRN, QRM, tono spostato, livelli, segnale forte altrove, segni di
procedura); misure di CER, velocità stimata e caratteri falsi. File WAV per confrontare con fldigi.
**PSK**: varicode → metrica di qualità e squelch 0-100 (con conversione delle impostazioni salvate, testi in sette lingue, S/N) → AFC con DCD
→ filtri `fir1`/`fir2` → sincronismo anticipo/ritardo → DCD e azzeramento → facoltativi S/N, IMD, salita e coda in trasmissione → QPSK.
**CW**: ingresso con decimazione e FIR → AGC di fldigi e squelch S/N → filtro degli stati brevi → velocità a coppie con soglia adattiva →
`<SK>` e segni di procedura → facoltativa la decodifica SOM.
Dopo ogni passo si rilanciano le prove di confronto e si annotano i numeri prima e dopo.
