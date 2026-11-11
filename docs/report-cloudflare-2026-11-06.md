# Report Cloudflare — 6 novembre 2026

Dati letti dal database D1 `aurora-rapporti` (installazioni anonime e rapporti degli errori). Nessun IP, nessun nominativo.

## Installazioni

| | |
|---|---|
| Installazioni di sempre | **49** (dal 4 al 6 novembre 2026) |
| Nuove | 23 il 4/11 · 16 il 5/11 · 10 il 6/11 |
| Attive oggi | 20 |
| Attive negli ultimi 7 giorni | 49 |

### Nazioni (salvate dal 5/11 sera)

| Nazione | Installazioni |
|---|---|
| Italia (IT) | 14 |
| Finlandia (FI) | 2 |
| Svezia, Portogallo, Polonia, Regno Unito, Francia, Brasile | 1 ciascuna |
| Non ancora note | 27 (non si sono più fatte sentire dopo il 5/11 sera) |

### Versioni in uso (ultimi 7 giorni)

| Versione | Installazioni |
|---|---|
| 0.11.155 (ultima) | 18 |
| 0.11.116 | 15 |
| 0.11.131 | 9 |
| 0.11.139 | 5 |
| 0.11.119 · 0.11.122 · 0.11.124 | 4 ciascuna |
| altre (0.11.123, .125, .133, .142, .149, .151, .154) | 1–3 ciascuna |

Una stessa installazione conta in più righe se si è aggiornata nella settimana.

### Sistemi

Quasi tutto Windows 11 (Pro e Home, 22H2–26H2) e Windows 10 22H2; **1 installazione Linux**; 1 Windows 11 Insider (build 29698).

## Errori

| Area | Volte | Tipi diversi |
|---|---|---|
| PTT / trasmissione | 166 | 50 |
| Audio | 137 | 42 |
| Decoder | 16 | 8 |
| CAT | 14 | 10 |
| Altro, telefono, blocchi, panic | 12 | 10 |

### I più frequenti

1. **Linux, 0.11.131 — uscita audio ALSA in errore** (`snd_pcm_avail_delay: I/O error`): 105 volte, una sola installazione. Dalla 0.11.152 l'audio si riprende da solo quando la scheda sparisce: da verificare se torna con le versioni nuove.
2. **«L'audio ha perso un blocco di campioni»**: circa 90 volte in versioni dalla 0.11.101 alla 0.11.131. Avviso senza conseguenze (la cascata resta attiva); nessuno nelle versioni recenti.
3. **Trasmissione non attiva: nominativo o locatore non validi**: 8 volte (0.11.122, 0.11.131) — utenti che premono Tx senza aver scritto nominativo e locatore.
4. **0.11.155 — uscita audio Windows «Failed to get current padding»**: 5 volte il 6/11. Errore WASAPI quando la scheda audio della radio sparisce (radio spenta o USB staccato).
5. **0.11.155 — ingresso audio «Linea FTDX… USB AUDIO CODEC» non trovato**: 3 volte il 6/11, stessa causa (radio spenta all'avvio).
6. **PTT senza radio collegata**: 7 volte in versioni vecchie (0.11.101, 0.11.116).

Pacchetti di assistenza: 2 (4/11, versioni 0.11.116 e 0.11.128).
