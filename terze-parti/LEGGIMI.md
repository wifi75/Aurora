# Codice di terze parti

Sorgenti liberi da cui Aurora riprende i modi nuovi (P2 in `docs/tasks.md`). Sono sottomoduli git: non entrano nell'installatore, servono come base per i motori scritti in Rust nei crate `core/crates/mode-*`. Chi riprende algoritmi o tabelle da qui lo scrive nel commento del file e cita gli autori.

| Cartella | Progetto | Versione presa | Licenza | Fonte |
|---|---|---|---|---|
| `qsstv` | QSSTV di Johan Maes ON4QZ: SSTV e DRM | commit `31c323c` del 2026-11-02 | GPL-3.0 | <https://github.com/ON4QZ/QSSTV> |
| `fldigi` | fldigi di Dave Freese W1HKJ e altri: RTTY, PSK, CW, MFSK, Olivia, MT63, Hell... | tag `v4.2.13` del 2026-07-29 | GPL-3.0 | <https://git.code.sf.net/p/fldigi/fldigi> (la copia su GitHub e' ferma al 2022) | si prendono solo funzioni, parametri e comportamento (macro, pulsanti, pagine), mai codice: vedi `memory.md`.

## Scaricarli

```
git submodule update --init terze-parti/qsstv
git submodule update --init terze-parti/fldigi
```

Su Windows fldigi contiene il percorso `flarq_doxygen/user_src_doc/aux/` («aux» e' un nome riservato di Windows): dopo il primo errore si esclude quella cartella e si rifa' il checkout:

```
git -C terze-parti/fldigi config core.protectNTFS false
git -C terze-parti/fldigi sparse-checkout set --no-cone "/*" "!/flarq_doxygen/"
git -C terze-parti/fldigi reset --hard HEAD
```

## Controllo degli aggiornamenti: ogni 6 mesi

Prossimo controllo: **maggio 2027**. Si guarda l'ultima versione di ciascuno (`git ls-remote --tags`, pagina SourceForge di fldigi), si leggono le novita' e, se toccano modi gia' portati in Aurora, si riportano le correzioni nei crate `mode-*`. Poi si aggiorna questa tabella e la data del prossimo controllo.
