# Copia di riserva di cty.dat

`cty.dat` e' il file dei paesi di Jim Reisert AD1C (<https://www.country-files.com/>), che Aurora include nell'installatore.
A ogni compilazione si scarica la versione aggiornata dal sito; questa copia serve solo quando il sito non risponde
(il 2026-11-10 non ha risposto alle macchine di GitHub per 40 minuti e la versione Linux della 0.11.223 non e' stata compilata).

Va rinnovata ogni tanto, per esempio a ogni nuova edizione del file (escono piu' volte l'anno):

    Invoke-WebRequest https://www.country-files.com/cty/cty.dat -OutFile desktop\cty-riserva\cty.dat
