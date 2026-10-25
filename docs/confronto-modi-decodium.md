# Modi di Decodium e di Aurora

Verifica del 2026-10-25 sul sorgente di Decodium 1.0.705 (cartelle `lib/`, `Decoder/`, `Modulator/`, `WSPR/`, `src/cw`, `src/rtty`, `src/ssb`, `src/sstv`, `src/voicekeyer`)
e sulla copia installata 1.0.711. FT2 e' escluso per scelta del proprietario.

## Cosa fa Decodium e cosa fa Aurora oggi

| Modo | Decodium | Aurora oggi (con JTDX) | Motore proprio (`ft8`) | Sorgente da cui portarlo | Ordine |
|---|---|---|---|---|---|
| **FT8** | ricezione e trasmissione | si (JTDX) | ricezione fatta, trasmissione: onda pronta, manca l'uscita audio e il PTT | JTDX `lib/ft8*.f90` | in corso |
| **FT4** | ricezione e trasmissione | si (JTDX) | da fare | JTDX `lib/ft4/*.f90` | **1** |
| **JT65** | si | non esposto (JTDX lo decodifica) | da fare | JTDX/WSJT-X `lib/jt65*.f90`, `ftrsd`, `lib/afc65b.f90` | 2 |
| **JT9** | si | non esposto (JTDX lo decodifica) | da fare | `lib/jt9*.f90`, `afc9.f90` | 2 |
| **JT4** | si | no | da fare | `lib/jt4*.f90` | 4 |
| **FST4 / FST4W** | si | no (JTDX non lo ha) | da fare | WSJT-X `lib/fst4/*.f90` (nella copia di Decodium) | 3 |
| **Q65** | si | no (JTDX non lo ha) | da fare | `lib/q65*.f90`, `lib/qra/` | 3 |
| **MSK144** | si | no | da fare | `lib/msk144*.f90`, `lib/mskrtd.f90` | 5 |
| **WSPR** | si | no | da fare | `WSPR/`, `lib/wsprd/` | 5 |
| **Echo** | si | no | da fare (poco interesse) | `lib/avecho.f90` | 7 |
| **CW** | decodifica e tasto | no | altro sottosistema | `src/cw` di Decodium (da analizzare) | 6 |
| **RTTY** | si | no | altro sottosistema | `src/rtty`, `src/jtty` | 6 |
| **SSB** (voce, voice keyer) | si | no | altro sottosistema | `src/ssb`, `src/voicekeyer` | 6 |
| **SSTV** | si | no | altro sottosistema | `src/sstv` | 6 |
| Rotore, SDR (RTL) | si | no | fuori dai modi | `src/rotor`, `src/rtl` | altro |

## Come si integrano

I modi WSJT (FT4, JT65, JT9, JT4, FST4, Q65, MSK144, WSPR) hanno tutti la stessa forma: periodi a tempo fisso, decodifica sul periodo intero, messaggi a
77 bit (o 72 per JT65/JT9/JT4/WSPR) e modulazione propria. Il crate `ft8` ha gia' le parti condivise (messaggi a 77 bit, LDPC, CRC, FFT, periodi UTC,
confronto con JTDX in `engine-native`): per ogni modo nuovo si aggiunge un modulo (sincronismo, metriche, codice proprio, modulatore) e un modo
nell'ingresso e nell'uscita audio.

- **Prima FT4** perche' e' nell'uso quotidiano: JTDX lo ha, Aurora lo mostra gia' e il motore proprio deve poterlo sostituire. Condivide con FT8 il codice
  LDPC(174,91) e i messaggi; cambiano periodo (7,5 s), toni (4, 20,833 Hz) e sincronismo.
- **JT65 e JT9**: servono i decoder Reed-Solomon e a toni (JT9) di JTDX; i messaggi sono a 72 bit.
- **FST4, Q65, MSK144, WSPR**: si portano da WSJT-X (la cartella `lib/` di Decodium li ha).
- **CW, RTTY, SSB, SSTV** non sono modi a periodo: sono sottosistemi separati (decodifica continua, tasto, voce) e vanno in un piano a parte, dopo l'analisi
  dei loro sorgenti in `src/`.

Ogni modo si considera finito quando il confronto con l'originale (JTDX per FT4, JT65, JT9; WSJT-X per gli altri) sulle stesse registrazioni da' una quota
alta di messaggi ritrovati, con la trasmissione provata dal proprietario con la radio.

## Prima registrazione, poi misura

Serve un registratore di periodi in `engine-native` (WAV a 12 kHz di ogni periodo, a richiesta) per misurare i modi su aria vera senza dipendere da
JTDX acceso: lo ha chiesto l'analisi del decoder (`docs/analisi-decodium-decoder.md`).
