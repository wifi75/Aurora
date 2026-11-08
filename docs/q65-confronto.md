# Q65: confronto della decodifica con WSJT-X

Voce 21 di `docs/tasks.md`. Prima misura del 2026-11-08; rifatta lo stesso giorno dopo aver portato in Aurora la decodifica di un
periodo di `lib/q65_decode.f90` e `lib/qra/q65/q65.f90` (WSJT-X 3.0.1).

## Come si rifà

- Segnali: `q65sim.exe` di WSJT-X (`"K1ABC W9XYZ EN37" <sottomodo> 1500 <doppler> 0.0 0.0 1 <periodo> 1 <file> <snr>`), un segnale a 1500 Hz per file, rumore diverso in ogni file, 10 file per punto (6 a 120 s, 5 a 300 s e in qualche punto di 60B e 60D).
- WSJT-X: `jt9.exe` 3.2.0-rc1 con `-3 -p <periodo> -b <sottomodo> -d 3 -L 200 -H 3000 -f 1500 -F 50`, un processo per file (niente media fra i file).
  - **Attenzione**: senza `-c` e `-x`, `jt9` usa di suo `mycall=K1ABC`, `hiscall=W9XYZ`, `hisgrid=EN37` (`lib/jt9.f90`), cioe' proprio il messaggio di `q65sim`. Cosi' decodifica con la lista dei messaggi del QSO («q3», `q65_dec_q3`), che e' una decodifica con il messaggio quasi noto. La prima misura confrontava questo con Aurora senza nominativi: il ritardo di 1-2 dB che ne usciva era in gran parte questo.
  - Alla cieca: `-c b -x b` (nessun nominativo: solo l'AP del CQ, come Aurora senza nominativi).
- Aurora: `cargo run --release -p jt --example misura_q65 -- <periodo> <sottomodo> --rx 1500 <file.wav> ...` (`core/crates/jt/examples/misura_q65.rs`), evanescenza lorentziana come WSJT-X; con i nominativi anche `--mio K1ABC --dx W9XYZ --locatore EN37`. Il codice di prima: commit `b9ef1bed` (non aveva `--rx` ne' i nominativi).
- Si conta un successo quando il messaggio giusto compare fra quelli decodificati. Tutto a priorita' bassa sullo stesso PC con la stazione in onda.

## Risultati (decodificati / file)

| Periodo | Sottomodo | Doppler (Hz) | SNR (dB) | File | jt9 alla cieca | Aurora prima | Aurora ora | jt9 con nominativi (q3) | Aurora ora con nominativi |
|---|---|---|---|---|---|---|---|---|---|
| 15 | A | 0 | -21 | 10 | 8 | 7 | **10** | 10 | **10** |
| 15 | A | 0 | -22 | 10 | 4 | 1 | **4** | 10 | **10** |
| 15 | A | 0 | -23 | 10 | 0 | 0 | **0** | 10 | **10** |
| 15 | A | 0 | -24 | 10 | 0 | 0 | **0** | 8 | **9** |
| 15 | A | 0 | -25 | 10 | 0 | 0 | **0** | 5 | **7** |
| 30 | A | 0 | -24 | 10 | 8 | 8 | **9** | 10 | **10** |
| 30 | A | 0 | -25 | 10 | 2 | 3 | **4** | 10 | **10** |
| 30 | A | 0 | -26 | 10 | 1 | 0 | **2** | 10 | **10** |
| 30 | A | 0 | -27 | 10 | 0 | 0 | **0** | 9 | **10** |
| 30 | A | 0 | -28 | 10 | 0 | 0 | **0** | 3 | **6** |
| 30 | A | 0 | -29 | 6 | 0 | 0 | **0** | 1 | **1** |
| 60 | A | 0 | -26 | 10 | 10 | 10 | **10** | 10 | **10** |
| 60 | A | 0 | -27 | 10 | 8 | 8 | **10** | 10 | **10** |
| 60 | A | 0 | -28 | 10 | 1 | 2 | **5** | 10 | **10** |
| 60 | A | 0 | -29 | 10 | 0 | 0 | **1** | 10 | **10** |
| 60 | A | 0 | -30 | 10 | 0 | 0 | **0** | 9 | **9** |
| 60 | A | 0 | -31 | 10 | 0 | 0 | **0** | 4 | **4** |
| 60 | A | 0 | -32 | 10 | 0 | 0 | **0** | 2 | **2** |
| 60 | B | 5 | -24 | 10 | 10 | 9 | **10** | 10 | **10** |
| 60 | B | 5 | -25 | 5 | 3 | 3 | **3** | 5 | **5** |
| 60 | B | 5 | -26 | 10 | 1 | 1 | **1** | 10 | **10** |
| 60 | B | 5 | -28 | 10 | 0 | 0 | **0** | 7 | **7** |
| 60 | B | 5 | -30 | 10 | 0 | 0 | **0** | 0 | **0** |
| 60 | D | 20 | -21 | 10 | 10 | 10 | **10** | 10 | **10** |
| 60 | D | 20 | -23 | 10 | 10 | 9 | **10** | 10 | **10** |
| 60 | D | 20 | -24 | 5 | 0 | 0 | **0** | 5 | **5** |
| 60 | D | 20 | -25 | 10 | 0 | 0 | **0** | 10 | **10** |
| 60 | D | 20 | -27 | 10 | 0 | 0 | **0** | 3 | **4** |
| 120 | A | 0 | -31 | 6 | 5 | 5 | **6** | 6 | **6** |
| 120 | A | 0 | -32 | 6 | 2 | 2 | **3** | 6 | **6** |
| 120 | A | 0 | -33 | 6 | 0 | 0 | **2** | 6 | **6** |
| 120 | A | 0 | -35 | 6 | 0 | 0 | **0** | 4 | **4** |
| 120 | A | 0 | -37 | 6 | 0 | 0 | **0** | 1 | **1** |
| 300 | A | 0 | -34 | 5 | 4 | 5 | **5** | 5 | **5** |
| 300 | A | 0 | -35 | 5 | 1 | 4 | **4** | 5 | **5** |
| 300 | A | 0 | -36 | 5 | 0 | 0 | **1** | 4 | **5** |

## Tempo per file (s)

Misurato da fuori (per `jt9` comprende l'avvio del processo, circa 0,2 s), con il PC occupato dalla stazione.

| | jt9 alla cieca | Aurora prima | Aurora ora | jt9 con nominativi | Aurora ora con nominativi |
|---|---|---|---|---|---|
| segnale decodificato subito | 0,4-1,4 | 7,3-8,8 | 0,04-0,2 | 0,2-0,7 | 0,06-0,3 |
| segnale non decodificato | 2,1-5,9 | 7,3-8,8 | 0,7-0,9 | 1,2-2,3 | 1,0-2,8 |

## Cosa dice

- Alla cieca Aurora ora decodifica come `jt9` o meglio in ogni punto. Senza Doppler guadagna circa 1 dB su `jt9` in 60A (-28 dB: 5/10 contro 1/10), 120A e 300A, mezzo dB in 30A, ed e' pari in 15A; con l'allargamento Doppler (60B a 5 Hz, 60D a 20 Hz) e' pari. Rispetto ad Aurora di prima il guadagno e' di 1-1,5 dB senza Doppler (15A -22: 4/10 contro 1/10; 60A -28: 5/10 contro 2/10; 120A -33: 2/6 contro 0/6).
- Con i nominativi la decodifica da lista porta Aurora al livello di `jt9`: pari in 60A, 60B, 120A, un po' meglio in 15A (-25: 7/10 contro 5/10), 30A (-28: 6/10 contro 3/10), 60D e 300A.
- Da dove viene il guadagno, tutto da `q65_decode.f90`/`q65.f90`: sincronismo a 1/8 di simbolo invece di 1/2, soglia statistica dei candidati (`q65_ccf_22`), la frequenza di ricezione provata sempre anche sotto soglia, la ricerca fine di `q65_loops` (cinque mezzi bin di frequenza, cinque sedicesimi di simbolo di tempo, larghezze B90 entro la distanza 5), le energie di `spec64` con equalizzazione della banda, il tetto a 20 volte il 40° percentile e `q65_bzap`. In piu' rispetto a WSJT-X Aurora prova anche la metrica di Bessel prima di quella dell'evanescenza: sui segnali senza allargamento rende circa 1 dB in piu' della sola lorentziana (60A -28 dB: 5/10 contro 1/10), che e' il margine su `jt9` alla cieca.
- Velocita': da 7-9 s a 0,04-0,9 s per periodo alla cieca (10-100 volte meno), meno di `jt9` alla cieca. Le energie di ogni candidato si ricavano da una sola FFT del periodo ritagliando la banda del segnale (64·(2+spb)+64 bin), invece di 85 FFT lunghe un simbolo per ogni ritardo provato. Con molti segnali in banda i candidati si possono provare in parallelo (`Options::threads`, il motore usa i thread della decodifica FT8); con un segnale solo il tempo e' quello del tentativo sulla frequenza di ricezione.

## Cosa resta

- Media: confronto di `--media` con `jt9` che tiene la media fra i file nello stesso processo (la media di Aurora non e' cambiata).
- Non portati: la deriva di frequenza (`max_drift`, il ramo W3SZ di `q65_dec0`), il ritardo EME oltre 1 s (`emedelay`: la ricerca ora va da -1 a +1 s come WSJT-X senza EME, prima Aurora arrivava a ±2 s), la lista dei contest (`q65_set_list2`).
- Prova dal vivo in Q65-60A su 6 m o 2 m.
