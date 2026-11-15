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
- Eliminati il 2026-11-15 perche' non piu' organizzati o senza regolamento: DRCG WW RTTY, DL-DX RTTY, Aegean RTTY, SCC RTTY Championship, Poland WW BPSK63, ARR PSK63, TARA Mêlée (si aggiungeranno se tornano).
## Gruppo 3C

Regolamenti letti il 2026-11-15. Prove in `core/crates/contest/tests/regole_contest_3c.rs`.

| Contest | Regola in breve | Fonte (edizione) |
|---|---|---|
| RUSSIAN DX Contest | Russia 10, stesso paese 2, stesso continente 3, altro 5 (russi fra loro 2/5); paesi e oblast per banda (`dom/russian.dom`), CW e SSB con la stessa stazione | <https://www.rdxc.org/rules> (2027) |
| Russian WW MultiMode Contest | stesso paese 1, altro paese 3, altro continente 5, il doppio su 160/80/40; paesi e oblast per banda e modo | <http://www.rdrclub.ru/news-radio/russian-ww-mm-contest/159-rus-ww-multimode-contest> (2026) |
| Scandinavian Activity Contest | solo QSO fra Scandinavia e resto del mondo; non scandinavi: 1 punto (3 su 80/40 m fuori Europa), aree numeriche scandinave per banda; scandinavi: 2 Europa, 3 fuori, paesi per banda (`[Script]` per l'ammissione) | <https://www.sactest.net/blog/rules/> (2023, contest in calendario 2027) |
| Stew Perry Topband Distance Challenge | 160 m; 1 punto + 1 ogni 500 km interi fra i centri dei quadrati; nessun moltiplicatore (`[Script]`) | <https://www.kkn.net/stew/stew_rules.html> (rev. 2025) |
| TRC DX Contest | 160-10 m CW/SSB; stesso continente 1, altro 2, stazione TRC 10, fra soci 1; paesi DXCC e paesi TRC per banda e modo (`[Script]`) | <https://trcdx.org/rules-trc-dx/> (2026) |
| TRC DIGI Contest | come il TRC DX, solo RTTY, moltiplicatori per banda (`[Script]`) | <https://trcdx.org/rules-trc-digi/> (2025) |
| UA2 QSO Party | 80/40/20 m; 2 punti CW, 3 SSB, il triplo con le stazioni R2F; 15 punti per distretto RDA di Kaliningrad nuovo per banda, sommati (`[Script]`) | <https://www.qrz.ru/contest/detail/382.html> (2027) |
| UBA CW contest, UBA SSB Contest | 80-10 m; Belgio 10, paesi UE 3, altri 1, Russia e Bielorussia esclusi; sezioni UBA, prefissi belgi e paesi UE per banda | <https://www.uba.be/en/hf/contest-rules/uba-dx-contest> (2026-2027) |
| UBA Spring 80M CW, 80M SSB, 6M SSB-CW, 2M SSB | per gli stranieri solo QSO con ON, 3 punti; moltiplicatori le sezioni UBA (anche XXX e UBA) | <https://www.uba.be/en/hf/contest-rules/spring-contest-dst> (2026) |
| UK_EI DX Contest | tabella europei/UK-EI/DX con il doppio su 80/40 m; paesi (tranne UK/EI) e distretti UK/EI per banda | <https://www.ukeicc.com/dx-contest-rules.php> (2026, v19) |
| Ukrainian DX Contest | Ucraina 10, stesso paese 1, continente 2, altro 3; paesi e oblast per banda (`dom/udxc.dom`) | <https://www.qsl.net/ut1ia/rules.html> (pagina senza data; urdxc.org non raggiungibile) |
| Venezuelan Independence Contest | stesso paese 1, continente 3, altro 5; paesi per banda e circuiti YV una volta | regole 2017 riportate da <https://dxnews.com/forum/forum/contesting/1223-venezuelan-independence-contest>; contest in calendario 2027 |
| WAEDC CW-SSB Contest | solo QSO fra Europa e resto del mondo, 1 punto + QTC; europei: paesi non europei e aree di W, VE, VK, ZL, ZS, JA, BY, PY; gli altri: paesi europei; pesi 4/3/2/2/2 (`[Script]` per l'ammissione) | <https://www.darc.de/der-club/referate/conteste/wae-dx-contest/en/wae-rules/> (2025, date 2026) |
| YB DX Contest, YB DX RTTY Contest | 80-10 m; YB 10, stesso paese 1, continente 2, altro 3; prefissi YB e paesi per banda | <https://ybdxcontest.com/>, <https://rtty.ybdxcontest.com/> (2026-2027) |
| YO DX HF Contest | 80-10 m; YO 8, altro continente 4, continente 2, paese 1 (YO: DX 8, EU 4, YO 0); paesi e distretti YO per banda | <https://www.yodx.ro/rules.php> (2025-2026) |

Approssimati:
- RDXC e Ukrainian DX: i paesi WAE non sono distinti dal DXCC; le stazioni /MM e RI1FJ/RI1AN non sono riconosciute.
- SAC: i nominativi senza cifra (area 0) non portano moltiplicatore.
- Stew Perry: i fattori di potenza (x2/x4 per la stazione lavorata, x1,5/x3 per la propria) non sono applicati.
- UA2 QSO Party: i due turni non sono distinti (la stessa stazione nel secondo turno conta come doppio).
- UBA DX: il bonus percentuale per i QSO con il Belgio non e' calcolato; la sezione XXX conta come moltiplicatore; Monte Athos
  (SV/A) manca dall'elenco dei paesi UE.
- UK/EI: il doppio dei punti fra le 01 e le 05 per le stazioni UK/EI non e' applicato.
- WAEDC CW-SSB: l'elenco WAE non e' distinto dal DXCC.

Tolti perche' non piu' organizzati (ultima edizione o regolamento prima del 2024, o contest sospeso): RememberSK, SA_Sprint, SSTV,
Sumy, TARA Skirmish, TARA-Grid-Dip, TARA-Rumble, Taras 200, Triathlon DX, UBA Low Band Winter, UBA-Millcontest (BMA), UCC Party,
UKRAINIAN DX CLASSIC RTTY, UN DIGI, UN DX, URDA Winter, Ukrainian RTTY Championship, VLADIMIR, YO International PSK31, ZHAS.


## Gruppo 3B (letti il 2026-11-15)

Prove in `core/crates/contest/tests/regole_contest_3b.rs`. Tolti perche' non piu' organizzati (ultima edizione o regolamento prima
del 2024, oppure regolamento introvabile e sito chiuso): French BPSK, Georgia, HPC World Wide DX, Happy Birthday Rudolf, IAFA,
International Bucuresti, Jakarta RTTY, Mongolian DX CW-SSB e RTTY, Nauryz DX, NDG, ON 10 Meter, PGA-DIGI, PSK-63 Marathon, PSKFEST,
RTTYOPS WW DX.

### French HF Championship (Coupe du REF)
Fonte: <https://concours.r-e-f.org/contest/a-propos/> (REF, riassunto dei regolamenti 2025; i PDF non erano leggibili).
- CW ultimo fine settimana pieno di gennaio, SSB di febbraio, sabato 06:00 - domenica 18:00 UTC; bande 80-10 m. Francesi RST +
  dipartimento (F6REF 00, oltremare il prefisso), gli altri RST + numero.
- Stranieri: solo i QSO con la Francia (anche oltremare), 1 punto stesso continente, 3 altro continente.
- Moltiplicatori per banda: dipartimenti e oltremare; i francesi contano i paesi DXCC.
- `[Score]`: `points=dxcc:<Francia e oltremare>&same_cont=1; dxcc:<...>=3`, `mult=exch/band/if:dxcc:<...>; dxcc/band/if:my_dxcc:F&not_dxcc:<...>`.

### GACW WWSA CW DX Contest
Fonte: <https://gacw.ar/wwsa/reglas-rules/> (GACW, revisione del 6 febbraio 2024).
- Secondo fine settimana di giugno, sabato 15:00 - domenica 15:00 UTC; CW, 80-10 m. Scambio RST + zona CQ.
- Punti: stesso paese 0, stesso continente 1, altro continente 3, Sud America da fuori 5.
- Moltiplicatori per banda: zone CQ e paesi.
- `[Score]`: `points=same_dxcc=0; other_cont&cont:SA=5; other_cont=3; same_cont=1`, `mult=cq/band; dxcc/band`.

### HA-DX Contest
Fonte: <https://ha-dx.com/en/contest-rules> (MRASZ, 2026).
- Terzo fine settimana pieno di gennaio, sabato 12:00 - domenica 12:00 UTC; CW e SSB, 160-10 m. HA RS(T) + contea, gli altri RS(T) + numero.
- Punti: stazione HA 10, stesso continente 2, altro continente 5.
- Moltiplicatori per banda (non per modo): paesi DXCC/WAE esclusa HA e le 20 contee.
- `[Score]`: `points=dxcc:HA=10; same_cont=2; other_cont=5`, `mult=dxcc/band/except:HA; exch/band/if:dxcc:HA`.

### Helvetia Contest
Fonte: <https://www.contestcalendar.com/contestdetails.php?ref=326> (WA7BNM); il PDF USKA 2026 non era leggibile.
- Fine aprile, sabato 13:00 - domenica 12:59 UTC; CW, SSB, digitale, 160-10 m. HB RS(T) + cantone, gli altri RS(T) + numero.
- Punti: stazione HB 10, stesso continente 1, altro continente 3.
- Moltiplicatori per banda: cantoni e paesi DXCC.
- `[Score]`: `points=dxcc:HB=10; same_cont=1; other_cont=3`, `mult=exch/band/if:dxcc:HB; dxcc/band/except:HB`.

### Holyland DX Contest
Fonte: <https://www.contestcalendar.com/contestdetails.php?ref=319> (WA7BNM); la pagina IARC <https://tools.iarc.org/wwhc/#/rules> non si legge senza browser.
- Meta' aprile, sabato 21:00 - domenica 21:00 UTC; CW e SSB, 80-10 m. 4X RS(T) + area, gli altri RS(T) + numero.
- Punti: stesso paese 1, stesso continente 2, altro continente 4; con una stazione 4X (o da 4X verso un altro continente) 8.
- Moltiplicatori per banda: aree 4X e paesi DXCC.
- `[Score]`: `points=dxcc:4X=8; my_dxcc:4X&other_cont=8; same_dxcc=1; same_cont=2; other_cont=4`, `mult=exch/band/if:dxcc:4X; dxcc/band`.

### IARU Region 1 Field Day
Fonte: <https://www.darc.de/der-club/referate/conteste/iaru-region-1-fieldday/rules> (DARC, 5 gennaio 2026).
- CW primo fine settimana pieno di giugno, SSB di settembre; 160-10 m. Scambio RST + numero.
- Punti (stazione portatile): fissa in Europa 2, fissa fuori 3, portatile (/P, /M, /MM, /AM) in Europa 4, fuori 6.
- Moltiplicatori per banda: paesi DXCC/WAE.
- `[Score]`: `points=call:P,M,MM,AM&cont:EU=4; call:P,M,MM,AM=6; cont:EU=2; any=3`, `mult=dxcc/band`.

### IGRY WW RTTY Contest
Fonte: <https://www.ig-ry.de/ig-ry-ww-contest> (IG-RY, edizione 2027).
- Secondo fine settimana pieno di aprile, sabato 12:00 - domenica 18:00 UTC; RTTY 45 baud, 80-10 m. Scambio RST + anno di prima licenza.
- Punti: 1 per QSO. Moltiplicatori: anni di licenza diversi, per banda.
- `[Score]`: `points=any=1`, `mult=exch/band`.

### Marconi Memorial Contest HF CW
Fonte: <https://www.arifano.it/contest_marconi.html> (ARI Fano, 30a edizione, 2026).
- Primo fine settimana di luglio, sabato 14:00 - domenica 13:59 UTC; CW, 160-10 m. Scambio RST + numero.
- Punti: 1 per QSO. Moltiplicatori: paesi della lista CQ WW, per banda.
- `[Score]`: `points=any=1`, `mult=dxcc/band`.
