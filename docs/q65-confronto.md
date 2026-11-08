# Q65: confronto della decodifica con WSJT-X

Voce 21 di `docs/tasks.md`. Prima misura del 2026-11-08.

## Come si rifà

- Segnali: `q65sim.exe` di WSJT-X (`"K1ABC W9XYZ EN37" <sottomodo> 1500 <doppler> 0.0 0.0 1 <periodo> 1 <file> <snr>`), un segnale a 1500 Hz per file, rumore diverso in ogni file.
- WSJT-X: `jt9.exe` 3.2.0-rc1 con `-3 -p <periodo> -b <sottomodo> -d 3 -L 200 -H 3000 -f 1500 -F 50`, un processo per file (niente media fra i file), nessun nominativo (solo AP di CQ).
- Aurora: `cargo run --release -p jt --example misura_q65 -- <periodo> <sottomodo> [--gauss] <file.wav> ...` (`core/crates/jt/examples/misura_q65.rs`), un periodo per volta, evanescenza lorentziana (come WSJT-X) e, dove indicato, gaussiana. Con `--media` i file si sommano come periodi successivi.
- Si conta un successo quando il messaggio giusto compare fra quelli decodificati.

## Risultati (decodificati / file)

| Periodo | Sottomodo | Doppler (Hz) | SNR (dB) | jt9 | Aurora lorentz. | Aurora gauss. |
|---|---|---|---|---|---|---|
| 15 | A | 0 | -18 | 10/10 | 10/10 | 10/10 |
| 15 | A | 0 | -19 | 10/10 | 10/10 | 10/10 |
| 15 | A | 0 | -21 | 6/6 | 4/6 | |
| 15 | A | 0 | -23 | 6/6 | 0/6 | |
| 30 | A | 0 | -22 | 6/6 | 6/6 | |
| 30 | A | 0 | -24 | 6/6 | 5/6 | |
| 30 | A | 0 | -26 | 6/6 | 0/6 | |
| 60 | A | 0 | -24 | 6/6 | 6/6 | |
| 60 | A | 0 | -26 | 6/6 | 6/6 | |
| 60 | A | 0 | -28 | 6/6 | 2/6 | |
| 60 | A | 1 | -23 | 6/6 | 6/6 | 6/6 |
| 60 | A | 1 | -25 | 6/6 | 6/6 | 6/6 |
| 60 | B | 5 | -20 | 6/6 | 6/6 | 6/6 |
| 60 | B | 5 | -22 | 6/6 | 6/6 | 6/6 |
| 60 | B | 5 | -24 | 6/6 | 5/6 | 6/6 |
| 60 | D | 20 | -17 | 6/6 | 6/6 | 6/6 |
| 60 | D | 20 | -19 | 6/6 | 6/6 | 6/6 |
| 60 | D | 20 | -21 | 6/6 | 6/6 | 6/6 |
| 120 | A | 0 | -27 | 4/4 | 4/4 | |
| 120 | A | 0 | -29 | 4/4 | 4/4 | |
| 120 | A | 0 | -31 | 4/4 | 3/4 | |
| 300 | A | 0 | -30 | 3/3 | 3/3 | |
| 300 | A | 0 | -32 | 3/3 | 3/3 | |

## Cosa dice

- Con l'allargamento Doppler (60B a 5 Hz, 60D a 20 Hz) e nei periodi lunghi Aurora decodifica come `jt9` fino ai valori provati; il modello gaussiano su 60B -24 dB ha preso il file che il lorentziano perdeva.
- Senza Doppler, sui segnali piu' deboli Aurora resta indietro: circa 2 dB in 15A (-21/-23), 1-2 dB in 30A e 60A (-24/-26 e -28). Le soglie di WSJT-X in questi casi non sono state raggiunte: `jt9` ha decodificato tutto anche al valore piu' basso provato, quindi il suo limite e' piu' in basso.
- Velocita': un periodo da 60 s costa ad Aurora alcuni secondi (circa 9 s con il PC occupato dalla stazione), a `jt9` meno di mezzo secondo.

## Cosa resta

- Recuperare i 1-2 dB senza Doppler: confrontare con `q65_decode.f90` la ricerca del sincronismo (WSJT-X prova anche la frequenza di ricezione con tolleranza stretta e la correlazione su `ccf` a piu' passi), le ipotesi di larghezza B90 provate e la soglia di `q65_dec_q3`/`q012`.
- Prendere le soglie di `jt9` scendendo con l'SNR finche' non perde file, e ripetere con piu' file per punto (6 sono pochi).
- Media: confronto di `--media` con `jt9` che tiene la media fra i file nello stesso processo.
- Velocita' della decodifica (FFT ripetute per ogni candidato e larghezza).
