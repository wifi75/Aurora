# Regole dei contest dei modi testuali

Riassunto dei regolamenti ufficiali usati per la sezione `[Score]` dei file `.contest` (`docs/contest-file.md`), con la fonte e la
data di lettura. Il punteggio di Aurora e' una stima: quello ufficiale lo calcola l'organizzatore dal Cabrillo.
Le prove con i log calcolati a mano stanno in `core/crates/contest/tests/regole_contest.rs`.

## Primo gruppo (letti il 2026-11-15)

### CQ WW RTTY DX Contest
Fonte: <https://www.cqwwrtty.com/rules.htm> (CQ).
- Bande 80, 40, 20, 15, 10 m. Scambio RST + zona CQ; W e VE aggiungono stato o area.
- Punti: stesso paese 1, stesso continente 2, altro continente 3.
- Moltiplicatori per banda: zone CQ, paesi (DXCC + WAE), stati USA (48 + DC) e aree canadesi (14). KL7 e KH6 contano solo come paese.
- Punteggio = punti x (zone + paesi + QTH W/VE).
- `[Score]`: `points=same_dxcc=1; same_cont=2; other_cont=3`, `mult=cq/band; dxcc/band; exch/band/if:dxcc:K,VE`.

### CQ WPX RTTY Contest
Fonte: <https://www.cqwpxrtty.com/rules.htm> (CQ).
- Bande 80-10 m. Scambio RST + numero.
- Punti su 28/21/14 MHz: altro continente 3, stesso continente 2, stesso paese 1; su 7 e 3,5 MHz il doppio.
- Moltiplicatori: prefissi WPX, una volta sola in tutto il contest.
- `[Score]`: `points_band=80:2,40:2`, `mult=wpx`.

### WAE DX Contest RTTY (DARC)
Fonte: <https://www.darc.de/der-club/referate/conteste/wae-dx-contest/en/wae-rules/> (DARC, §6, §7, §8, §12).
- Bande 80-10 m. In RTTY tutti lavorano tutti; i QTC solo fra continenti diversi, al massimo 10 fra due stazioni (`docs/contest-qtc.md`).
- Punti: 1 per QSO, 1 per QTC mandato o ricevuto.
- Moltiplicatori per banda (§6, che vale anche in RTTY): paesi DXCC ed elenco WAE; per W, VE, VK, ZL, ZS, JA, BY, PY, RA8, RA9 e RA0
  contano le aree numeriche. Peso: 80 m x4, 40 m x3, 20/15/10 m x2.
- Punteggio = (QSO + QTC) x somma dei moltiplicatori pesati.
- `[Score]`: `mult=dxcc/band/except:K,VE,VK,ZL,ZS,JA,BY,PY,UA9; area:K,VE,VK,ZL,ZS,JA,BY,PY,UA9/band`, `mult_band=80:4,40:3,20:2,15:2,10:2`;
  `m_iQtc=2`.

### BARTG HF RTTY Contest
Fonte: <https://bartg.org.uk/wp-content/uploads/2025/05/bartg-hf-rtty-rules-2025-v3.pdf> (BARTG, regole 2025 v3).
- Bande 80-10 m, con i segmenti di frequenza del regolamento (fuori segmento zero punti: non controllato).
- Punti: 1 per QSO, una volta per banda.
- Moltiplicatori per banda: paesi DXCC (anche JA, W, VE, VK) e aree numeriche JA, W, VE, VK. Continenti: una volta sola, al massimo 6.
- Punteggio = punti x moltiplicatori x continenti.
- `[Score]`: `mult=dxcc/band; area:JA,K,VE,VK/band; cont/product`.

### ARRL RTTY Roundup
Fonte: <https://www.arrl.org/rtty-roundup> e il regolamento completo
<https://arrl.org/files/file/Contest%20Rules%20PDFs/2022/RTTY%20Roundup%20Rules%20-%202_2%20-%202022.pdf> (ARRL, versione 2.2).
- Bande 80-10 m, una volta per banda in qualsiasi modo. Scambio: W/VE RST + stato o provincia, gli altri RST + numero.
- Punti: 1 per QSO.
- Moltiplicatori una volta sola: stati USA + DC, province e territori canadesi, paesi DXCC esclusi USA e Canada (KH6 e KL7 sono DXCC).
- `[Score]`: `mult=exch/if:dxcc:K,VE; dxcc/except:K,VE`; `m_bDupeNoMode=1`.

### RAEM (Coppa della Russia «Memorial E. Krenkel»)
Fonte: <https://raem.srr.ru/?p=47> (SRR), confrontato con l'elenco di <https://www.digitalrus.ru/>.
- Scambio: numero + latitudine e longitudine in gradi interi (`57N85E`).
- Punti: 50 per QSO; 1 punto per grado di differenza di latitudine e 1 per grado di longitudine; +100 con una stazione polare
  (latitudine 66 o piu'); +300 con la stazione commemorativa RAEM. Nessun moltiplicatore; chi sta dentro i circoli polari moltiplica il
  totale per 1,1.
- `[Score]`: `points=any=50`, `bonus=call:RAEM=300`; `[Script]`: `punti(qso)` con le coordinate e la stazione polare,
  `punteggio(totali)` con il fattore 1,1 (esempio in `docs/contest-file.md`).

### SARTG WW RTTY Contest
Fonte: <https://www.sartg.com/contest/wwrules.htm> (SARTG).
- Bande 80-10 m. Scambio RST + numero.
- Punti: stesso paese 5, stesso continente 10, altro continente 15.
- Moltiplicatori per banda: paesi DXCC e aree numeriche di VK, VE, JA e W.
- `[Score]`: `points=same_dxcc=5; same_cont=10; other_cont=15`, `mult=dxcc/band; area:K,VE,JA,VK/band`.

### Russian WW Digital Contest
Fonte: <http://www.rdrclub.ru/index.php/russian-ww-digital-contest/51-rus-ww-digi-rules> (Russian Digital Radio Club), trovato
tramite <https://www.digitalrus.ru/>.
- RTTY45 e BPSK63, bande 160-10 m; 3-4 ottobre 2026, 12:00-11:59 UTC. Scambio: russi RST + oblast, gli altri RST + numero.
- Punti su 20/15/10 m: stesso paese 1, altro paese 3, altro continente 5 (stazione QRP 5); su 160/80/40 m il doppio.
- Moltiplicatori per banda e per modo: paesi DXCC e oblast russi.
- `[Score]`: `points_band=160:2,80:2,40:2`, `mult=dxcc/band/mode; exch/band/mode/if:dxcc:UA,UA9,UA2` (oblast dall'elenco
  `dom/russian.dom`). Il file conteneva i dati del Russian WW PSK: nome, note e date sono stati corretti.

### Ukrainian DX DIGI Contest
Fonte: riassunto del regolamento su <https://www.qsl.net/contest_ari/reg_ukr_digi.html> (ARI), che rimanda a
<http://www.qsl.net/ur5fav/udrpc/>; sito del contest indicato da <https://www.digitalrus.ru/>: <http://www.izmail-dx.com/> (non
raggiungibile il giorno della lettura).
- RTTY e PSK31, bande 80-10 m; una stazione si lavora una volta per banda e per modo. Scambio: ucraini RST + oblast, gli altri RST + numero.
- Punti: stesso continente 1, altro continente 3, stazioni ucraine 5; su 80 m il doppio.
- Moltiplicatori per banda e per modo: paesi DXCC e oblast ucraini.
- `[Score]`: `points=dxcc:UR=5; same_cont=1; other_cont=3`, `points_band=80:2`, `mult=dxcc/band/mode; exch/band/mode/if:dxcc:UR`.

## Secondo gruppo (letti il 2026-11-15)

### SP DX RTTY Contest (PK RVG)
Fonte: il sito indicato da <https://www.digitalrus.ru/> (<http://www.pkrvg.org/zbior.html>) non e' raggiungibile; regole dal riassunto ARI
<https://www.qsl.net/contest_ari/reg_spdxrtty.html>.
- Bande 80-10 m. Scambio RST + numero; le stazioni polacche RST + provincia (una lettera).
- Punti: stesso paese 2, stesso continente 5, altro continente 10.
- Moltiplicatori per banda: paesi DXCC e province polacche (16); continenti una volta sola (al massimo 6), a parte.
- Punteggio = punti x (paesi + province) x continenti.
- `[Score]`: `points=same_dxcc=2; same_cont=5; other_cont=10`, `mult=dxcc/band; exch/band/if:dxcc:SP; cont/product`.

### OK DX RTTY Contest (CRK)
Fonte: <http://www.crk.cz/ENG/DXCONTE.HTM> (CRK, indicato da <https://www.digitalrus.ru/>; il server rifiuta la lettura automatica),
regole 2020 riportate da <https://cq.sk/en/ok-dx-rtty-contest-2020>.
- Bande 80-10 m. Scambio RST + zona CQ.
- Punti: su 10/15/20 m stesso continente 1, gli altri 2; su 40/80 m stesso continente 3, gli altri 6.
- Moltiplicatori per banda: paesi DXCC; per chi non e' OK/OL anche ogni stazione OK diversa.
- `[Score]`: `points=band:80,40&same_cont=3; band:80,40=6; same_cont=1; any=2`, `mult=dxcc/band; call/band/if:dxcc:OK&not_my_dxcc:OK`.

### EA RTTY Contest (URE)
Fonte: <https://concursos.ure.es/en/eartty/bases/> (URE).
- Bande 80-10 m. Scambio: EA RST + provincia, gli altri RST + numero; EA4URE manda HQ.
- Punti: EA con EA (anche EA6, EA8, EA9) 2, EA con DX 1, DX con DX 1, DX con EA 3.
- Moltiplicatori per banda: entita' EADX-100, province spagnole, EA4URE (HQ), aree numeriche di W, VE, JA, VK (il primo QSO con questi
  paesi e con EA, EA6, EA8, EA9 vale doppio: paese e area o provincia).
- `[Score]`: `points=my_dxcc:EA,EA6,EA8,EA9&dxcc:EA,EA6,EA8,EA9=2; my_dxcc:EA,EA6,EA8,EA9=1; dxcc:EA,EA6,EA8,EA9=3; any=1`,
  `mult=dxcc/band; exch/band/if:dxcc:EA,EA6,EA8,EA9; area:K,VE,JA,VK/band`.

### Mexican (FMRE) RTTY International Contest
Fonte: <https://rtty.fmre.mx/reglas/Rules_FMRE_RTTY_2024.pdf> (FMRE, 26a edizione, 2024).
- Bande 80-10 m. Scambio: XE RST + stato, gli altri RST + numero.
- Punti: con una stazione XE 4, stesso paese 2, altro paese 3.
- Moltiplicatori per banda: i 32 stati messicani (elenco `dom/Mexico32.dom`) e i paesi DXCC.
- `[Score]`: `points=dxcc:XE=4; same_dxcc=2; any=3`, `mult=exch/band/if:dxcc:XE; dxcc/band`; `m_sStateListFile=dom/Mexico32.dom`.

### Alessandro Volta RTTY DX Contest
Fonte: <https://ari.it/english-area/contest/6746-55th-alessandro-volta-rtty-dx-contest.html> (ARI, 55a edizione); la tabella dei punti
zona CQ -> zona CQ viene dal file `Points_AVOLTA.txt` dei dati di <https://www.digitalrus.ru/> (righe `cq1`...`cq40` del file).
- Bande 80-10 m. Scambio RST + numero + zona CQ.
- Punti dalla tabella delle zone; fuori dal proprio continente su 80 e 10 m il doppio. QSO nello stesso paese (o nella stessa area di
  VK, VE, JA, ZL, W) non validi.
- Moltiplicatori per banda: paesi DXCC; per VK, VE, JA, ZL e W contano solo le aree. Un moltiplicatore in piu' per ogni paese di un
  altro continente lavorato su almeno 4 bande (non calcolato).
- Punteggio = punti x moltiplicatori x numero dei QSO.
- `[Score]`: `valid=other_dxcc`, `points=other_cont&band:80,10=cqtab*2; any=cqtab`, `mult=dxcc/band/except:JA,VE,VK,K,ZL;
  area:JA,VE,VK,K,ZL/band`, `times_qso=1`.

### Russian «Radio» WW RTTY Contest
Fonte: <http://www.radio.ru/cq/contest/rule-results/index2.shtml> (rivista «Radio»).
- Bande 80-10 m, una volta per banda. Scambio: russi RST + oblast, gli altri RST + zona CQ.
- Punti: stesso continente 5, altro continente 10.
- Moltiplicatori per banda: oblast russi (elenco `dom/russian.dom`) e paesi DXCC.
- `[Score]`: `points=same_cont=5; other_cont=10`, `mult=dxcc/band; exch/band/if:dxcc:UA,UA9,UA2`; `m_sStateListFile=dom/russian.dom`.

### Open Ukraine RTTY Championship
Fonte: <http://uarl.com.ua/openrtty/2012/rtty2012e.txt> (UARL, indicato da <https://www.digitalrus.ru/>).
- Parte bassa 160 e 80 m (a turni), parte alta 40-10 m. Scambio: sigla della regione + numero (`PO001`).
- Punti: 2 per QSO, piu' 10 per ogni regione nuova su ogni banda (e su ogni turno della parte bassa). Nessun moltiplicatore: il
  risultato e' la somma delle bande.
- `[Score]`: `points=any=2`, `mult=exch/band/add:10` (i turni della parte bassa non sono distinti).

### JARTS WW RTTY Contest
Fonte: il sito <http://jarts.jp/> (indicato da <https://www.digitalrus.ru/>) non si legge in modo automatico; regole dal riassunto ARI
<https://www.qsl.net/contest_ari/reg_jarts.html>.
- Bande 80-10 m. Scambio RST + eta' dell'operatore (00 per le YL, 99 per le multi).
- Punti: stesso continente 2, altro continente 3.
- Moltiplicatori per banda: paesi DXCC tranne JA, W, VE, VK, e le aree di JA, W, VE, VK.
- `[Score]`: `points=same_cont=2; other_cont=3`, `mult=dxcc/band/except:JA,K,VE,VK; area:JA,K,VE,VK/band`.

### BARTG Sprint RTTY e BARTG Sprint 75
Fonti: <https://bartg.org.uk/wp-content/uploads/2025/05/bartg-sprint-rules-2025-v3.pdf> e
<https://bartg.org.uk/wp-content/uploads/2026/02/bartg-sprint75-rules-2026-v1.pdf> (BARTG).
- Bande 80-10 m con i segmenti del regolamento (non controllati); una stazione una volta per banda. Scambio: numero.
- Punti: 1 per QSO.
- Moltiplicatori una volta sola: paesi DXCC e aree di JA, W, VE, VK; continenti una volta sola (al massimo 6), a parte.
- Punteggio = punti x moltiplicatori x continenti.
- `[Score]`: `mult=dxcc; area:JA,K,VE,VK; cont/product`. Nello Sprint `m_bDupeNoBand` e' tornato a 0 (il doppio e' per banda).

### SARTG New Year RTTY Contest
Fonte: <https://www.sartg.com/contest/nyrules.htm> (SARTG).
- Bande 80 e 40 m. Scambio RST + numero + nome + auguri nella propria lingua.
- Punti: 1 per QSO.
- Moltiplicatori per banda: paesi DXCC tranne la Scandinavia, dove contano le aree numeriche (SM3 e SK3 sono la stessa).
- `[Score]`: `bands=80,40`, `mult=dxcc/band/except:SM,LA,OH,OZ,TF,OY,OX,JW,JX,OH0,OJ0; area:SM,LA,OH,OZ,TF,OY,OX,JW,JX,OH0,OJ0/band`.

### TARA RTTY Mêlée
Fonte: <http://www.n2ty.org/seasons/tara_melee_rules.html> (TARA).
- Bande 160-10 m, niente WARC. Scambio: W RST + stato, VE RST + provincia, gli altri RST + numero.
- Punti: 1 per QSO, una volta per banda.
- Moltiplicatori una volta sola: stati USA + DC, province e territori canadesi, paesi DXCC esclusi USA e Canada.
- `[Score]`: `mult=exch/if:dxcc:K,VE; dxcc/except:K,VE`.

### Makrothen RTTY Contest
Fonte: <https://www.pl259.org/makrothen/makrothen-rules/> (Makrothen).
- Scambio: il quadrato del locatore (4 caratteri). Una stazione una volta per banda.
- Punti: la distanza in km fra i centri dei due quadrati (raggio 6378,16 km, per difetto), per 2 su 80 m e 1,5 su 40 m; 100 punti nello
  stesso quadrato. Nessun moltiplicatore.
- `[Score]`: `points=any=0`, `special=makrothen` (distanza scritta a mano in `regole_punti.rs`).

### YU DX Contest
Fonte: <https://www.yudx.yu1srs.org.rs/2025/INTERNATIONAL_YUDX_HF_CONTEST_2025.pdf> (SRS, regole 2025, lette dal riassunto di
<https://cq.sk/en/yudx-contest-2025/>: il PDF del sito si apre solo nel browser).
- CW e SSB (non e' un contest RTTY), bande 160-10 m. Scambio: YU RST + distretto, gli altri RST + zona ITU.
- Punti: con YU/YT 10, altro continente 4, stesso continente 2, stesso paese 1, YU con YU 1.
- Moltiplicatori per banda: paesi DXCC e, per chi non e' YU, i distretti YU.
- `[Score]`: `points=my_dxcc:YU&dxcc:YU=1; dxcc:YU=10; same_dxcc=1; same_cont=2; other_cont=4`,
  `mult=dxcc/band; exch/band/if:dxcc:YU&not_my_dxcc:YU`.

### Russian WW PSK Contest
Fonte: <https://rdrclub.ru/russian-ww-psk-contest/49-rus-ww-psk-rules> (Russian Digital Radio Club, indicato da
<https://www.digitalrus.ru/>), confrontato con il riassunto di `INDICE.md` dei dati digitalrus.
- BPSK31, BPSK63, BPSK125, bande 160-10 m. Scambio: russi RST + oblast, gli altri RST + numero.
- Punti su 20/15/10 m: stesso paese 1, altro paese 3, altro continente 5; su 160/80/40 m il doppio.
- Moltiplicatori per banda: paesi DXCC e oblast russi (elenco `dom/russian.dom`).
- `[Score]`: `points=same_dxcc=1; same_cont=3; other_cont=5`, `points_band=160:2,80:2,40:2`, `mult=dxcc/band; exch/band/if:dxcc:UA,UA9,UA2`;
  `m_sStateListFile=dom/russian.dom`.

### EA PSK63 Contest (URE)
Fonte: <https://concursos.ure.es/en/eapsk63/bases/> (URE).
- BPSK63, bande 80-10 m. Punti, scambio e moltiplicatori come l'EA RTTY.
- `[Score]`: come l'EA RTTY.

### UBA PSK63 Prefix Contest
Fonte: <https://www.uba.be/en/hf/contest-rules/uba-psk63-prefix-contest> (UBA, edizione 2027).
- BPSK63, bande 80-10 m. Scambio: ON RST + sezione UBA (tre lettere, XXX per chi non e' socio), gli altri RST + numero.
- Punti: 1 per QSO. I QSO con Russia e Bielorussia valgono zero punti e nessun moltiplicatore.
- Moltiplicatori per banda: prefissi WPX e sezioni UBA.
- `[Score]`: `valid=not_dxcc:UA,UA9,UA2,EU`, `mult=wpx/band; exch/band/if:dxcc:ON`.

## Cosa resta approssimato

- Zone CQ e ITU vengono dal database DXCC di Aurora, non dallo scambio ricevuto (un W6 che passa zona 3 conta la zona del paese).
- Elenco WAE (IT9, GM/Shetland, JW/Bear, 4U1V...) non distinto dal DXCC; nel WAE RTTY contano tutti i paesi per tutti.
- Stazioni /MM, QRP (5 punti nel Russian WW Digital) e i segmenti di frequenza del BARTG non sono riconosciuti.
- Stati, province e oblast si prendono dallo scambio cosi' com'e' scritto: senza elenco dei domini non si controlla che esistano.
- Alessandro Volta: i QSO nella stessa area di VK, VE, JA, ZL, W non sono esclusi (lo e' solo lo stesso paese) e manca il moltiplicatore
  per i paesi di un altro continente lavorati su 4 bande.
- Open Ukraine RTTY: i turni della parte bassa (160 e 80 m) non sono distinti.
- UBA PSK63 Prefix: la sezione XXX dei non soci conta come moltiplicatore.
- I moltiplicatori `call` (stazioni OK, stazioni portoghesi) non compaiono nella griglia della finestra delle statistiche.
- Eliminati il 2026-11-15 perche' non piu' organizzati o senza regolamento: DRCG WW RTTY, DL-DX RTTY, Aegean RTTY, SCC RTTY Championship, Poland WW BPSK63, ARR PSK63 (si aggiungeranno se tornano).
