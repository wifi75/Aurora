# Motore FT8/FT4 di Aurora

Data: 2026-10-24 · Stato: approvato dal proprietario («Aurora deve sostituirsi a JTDX», via libera alla fase 1) · Licenza: GPL-3.0

## Obiettivo

Aurora ascolta la scheda audio, decodifica FT8 (poi FT4), trasmette e comanda la radio **senza JTDX**. Successo: con il motore proprio acceso, su
una registrazione di aria vera Aurora ritrova almeno il 95% dei messaggi che JTDX decodifica sulla stessa registrazione, e in trasmissione un
ascoltatore (o JTDX in ricezione) decodifica i messaggi di Aurora. JTDX resta installabile come motore alternativo finche' serve.

## Come lo si costruisce

- **Sorgenti**: JTDX 2.2.159 (GitHub `jtdx-project/jtdx`, GPL 3, derivato di WSJT-X) e WSJT-X. Gli algoritmi si portano in Rust (non si collega il
  Fortran): crate `core/crates/ft8`. L'FFT e' `rustfft` (MIT/Apache).
- **REA + Ghidra** (gia' installati, vedi `fase0-rapporto.md`) su `jtdx.exe`: servono dove il sorgente non basta (soglie, tabelle, comportamento
  reale) e per controllare i numeri del porting. Non sostituiscono il sorgente, che e' pubblico e piu' leggibile.
- **Verifica**: (1) prove su vettori noti (messaggi, CRC, codice); (2) giro completo codifica → onda → decodifica; (3) confronto con JTDX sulle
  stesse registrazioni (`ALL.TXT` di JTDX contro le decodifiche di Aurora), con la percentuale di parita' come numero di riferimento.

## Parti del crate `ft8`

| Modulo | Cosa fa | Da dove |
|---|---|---|
| `message` | 77 bit ⇄ testo (nominativi standard, /P, non standard con hash, locatore, rapporto, testo libero) | `lib/ft8v2/packjt77.f90` |
| `crc` | CRC a 14 bit | `lib/ft8v2/chkcrc14a.f90`, `lib/crc14.cpp` |
| `ldpc` | LDPC(174,91): codifica, propagazione di credenze, OSD | `lib/ft8v2/{encode174_91,bpdecode174_91,osd174_91}.f90`, tabelle `ldpc_174_91_c_*` |
| `modem` | 174 bit → 79 toni → onda GFSK | `lib/genft8.f90`, `lib/gen_ft8wave.f90` |
| `sync` | ricerca candidati, discesa a 200 Hz, allineamento fine, LLR dei bit | `lib/sync8.f90`, `ft8_downsample.f90`, `sync8d.f90`, `ft8b.f90` |
| `decode` | periodo intero: piu' passate, sottrazione, doppioni, SNR | `lib/ft8_decode.f90`, `ft8b.f90`, `ft8v2/subtractft8.f90` |

Le firme pubbliche stanno gia' in `core/crates/ft8/src/*.rs` (le implementazioni sono segnaposto): ogni modulo si scrive e si prova da solo.

## Fasi

1. **Ricezione FT8** (questa): il crate `ft8` completo in ricezione, il confronto con JTDX e, in Aurora, un motore «nativo» che decodifica dall'ingresso
   audio ed emette gli stessi eventi `Decode` di oggi, in parallelo a JTDX per il confronto.
2. **FT4** in ricezione.
3. **Trasmissione**: onda GFSK sull'uscita audio scelta, PTT con OmniRig, tempi del periodo; prima su carico fittizio, con il proprietario alla radio.
4. **Motore predefinito**: il nativo diventa quello di partenza, JTDX facoltativo; i comandi (Monitora, Hound, Abilita Tx...) passano dallo stato interno.

## Rischi

- La sensibilita' di JTDX viene da decine di accorgimenti in `ft8b.f90` (97 KB: analisi a-priori, tre livelli di metrica, sottrazione, OSD, filtri
  sui falsi). Si portano a gruppi, misurando ogni volta la parita' con JTDX; non si promette la stessa sensibilita' al primo giro.
- Tempo e orologio: servono gli stessi 15 s del resto della rete (modulo `clock-sync` gia' presente).
- Licenza: il codice portato e' GPL 3, quindi Aurora e' GPL-3.0 (solo), non «o successiva».
