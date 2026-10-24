# Memoria di progetto

Stato al 2026-10-24, versione 0.10.0. Per riprendere il lavoro leggi anche `docs/mancante.md` (cosa manca) e `CHANGELOG.md`.

## Scelte di fondo
- JTDX 2.2.159 (`C:\JTDX64\159`) resta il motore di decodifica, CAT e TX; Aurora è cervello e interfaccia. JTDX non accetta `Configure` via UDP: le sue impostazioni di decodifica restano in JTDX.
- Core Rust (workspace `core/`, Tokio/Axum), interfaccia React + TypeScript + Vite, cascata WebGPU con ripiego WebGL2, PWA per il telefono. Guscio desktop Tauri 2 ancora da fare. Database SQLite.
- Nessun login a password: dispositivi associati con un codice monouso (durata scelta dall'operatore, 1-60 minuti, predefinito 10). Per le prove esiste l'accesso senza codice, solo rete di casa, mai da proxy.
- Grafica: "Notte radio" con vetro traslucido; colori dei messaggi come le 17 categorie di JTDX, configurabili.

## Logiche di JTDX
- `docs/logiche-jtdx.md` raccoglie le logiche del motore e come le segue Aurora: JTDX decide (messaggio da trasmettere, periodo, avanzamento), Aurora legge e comanda come un utente. Tabella di tutti i controlli della sua finestra e delle impostazioni che contano, errori passati e la regola che ne viene.
- Dopo un riavvio lo stato si legge dalla finestra di JTDX (meno di 1 s) e indirizzo e identificativo UDP di JTDX stanno in `data/jtdx-peer.txt`, cosi' STOP e i comandi partono subito.

- Lista dei desiderati (crate `wishlist`, osservatore in `aurora/src/wanted.rs`): i dati stanno in `data/wishlist.json`; il riconoscimento (nominativo, prefisso, paese, griglia) e' duplicato in `app/src/lib/wishlist.ts` per evidenziare le righe e va tenuto uguale. L'avviso al telefono usa ntfy (HTTP POST con `ureq`, solo se l'utente scrive un indirizzo): i dati passano dal server ntfy scelto, quindi argomento segreto o server proprio.
- Mappa (`#/mappa`): planisfero SVG con dati `world-atlas` (ISC), griglie lavorate da `GET /map/worked` (Log4OM2) e stazioni dai locatori nei messaggi.

## Come funziona il collegamento con la stazione
- JTDX → server UDP primario `127.0.0.1:2334` = hub di Aurora → inoltro identico a GridTracker2 (2333). Il server secondario di JTDX verso Log4OM2 (2237) non si tocca: non aggiungere lo stesso destinatario ad `AURORA_EXPORT_UDP`, i QSO sarebbero doppi.
- Cambio banda e frequenza Rx: stesso OmniRig che usa JTDX come CAT. Cambio modo FT8/FT4 e frequenza Rx: si comandano i controlli di JTDX con l'accessibilità di Windows (UI Automation) via PowerShell.
- `TriggerCQ` (UDP tipo 51): il secondo parametro a `true` abilita Tx (provato dal vivo).

- Comandi a JTDX con UI Automation: i pulsanti si premono con `Invoke` (il clic), mai con `Toggle()`, che accende solo la casella senza far scattare il segnale (era il bug di FT4). L'elenco dei controlli raggiungibili e' chiuso (`rig-control/src/controls.rs`): niente accordo ne' log. Cambiando modo si resta nella banda di prima, sulla frequenza della tabella.
- Log di Log4OM2: database SQLite in `Documents\LOG4OM2\<nominativo>.sqlite` (36484 QSO, 98 colonne, tabella `Log`), aperto solo in lettura nelle prove. Aurora oggi legge solo l'ADIF di JTDX (2060 QSO): novita' e statistiche ne escono sottostimate. Lettura da decidere con Tiziano.
- Avvio di sviluppo: serve `AURORA_AUDIO_DEVICE='USB Audio'` per la cascata; senza e' vuota (dimenticato una volta al riavvio).

- Cursore Pwr: un cursore orizzontale ruotato di 90 gradi (il verticale per CSS era invertito in alcuni browser e portava il livello al minimo, 45 dB, scambiato per «non funziona»). 100% = 0 dB di attenuazione, sempre in alto. Il valore parte al rilascio e richiede la trasmissione aperta.
- Esc ferma la trasmissione e spegne AutoTX e Abilita Tx (opzione in Generali, accesa). Il doppio clic su un messaggio accende anche Abilita Tx: con «1 QSO» JTDX lo spegne a ogni QSO finito.

- Preferenze dell'interfaccia sul server (`data/ui-settings.json`, separate per PC e telefono): non si perdono cambiando indirizzo o browser. Le impostazioni di JTDX si leggono (sola lettura) da `JTDX.ini` (`AURORA_JTDX_INI`), senza password.
- Client NTP (`clock-sync`, scheda Orologio): controlla lo scarto dell'ora di Windows; correggerla serve Aurora come amministratore e solo dalla rete di casa. Indicatore «Orologio» e DT mediano (il «Med» di JTDX) nella barra di stato.
- DX Cluster (`dx-cluster`, finestra a sinistra di Comandi, scheda DX Cluster): client telnet, spento di default; server pubblici verificati il 2026-10-24 nel catalogo; accetta solo comandi di consultazione (`sh/`, `set/`...), mai spot o messaggi pubblici. Log di Log4OM2 letto in sola lettura (`AURORA_LOG4OM_DB`).

- Radio FT-2000 via OmniRig: `SetSimplexMode` la mette in split e sul VFO B. Il cambio banda scrive `Split` spento, aspetta che la radio lo accetti e solo dopo scrive `Freq`; scrivere `Vfo` subito dopo lo split lo fa riaccendere (provato dal vivo).
- Il doppio clic puo' perdere il periodo perche' Abilita Tx passa da PowerShell e UI Automation (circa 1 s): idea per risolverlo, una sessione PowerShell fissa.
- Modifica delle impostazioni di JTDX: scrivendo `JTDX.ini` a JTDX chiuso (lo riscrive all'uscita). Core pronto (`ini_edit`, `jtdx_process`), interfaccia da fare.
- Nessun nominativo o locatore reali nei file del repository: usare dati finti (IU3ABC, IO91wm). Nella cronologia locale ci sono commit con quelli veri: prima di un push serve riscriverla.
- Giro periodico: ogni 30 minuti si fa un giro completo di git (commit) e documentazione (CHANGELOG, TODO, memory, docs/api, README) senza il push finche' la cronologia non e' ripulita.

## Sicurezza (non negoziabile)
- Il core parte con la trasmissione chiusa (`AURORA_ALLOW_TX=0`), due cancelli indipendenti (API e motore); lo script `avvia-sviluppo.ps1` su richiesta di Tiziano (2026-10-24) la apre, con `-Chiusa` resta chiusa. STOP TX mai bloccato e spegne anche l'automazione.
- Un Reply fa trasmettere JTDX anche con Enable Tx spento: provato per errore il 2026-10-24, non si ripete senza carico fittizio e conferma.
- Automazioni: partono spente e in prova a vuoto; il modo reale solo dal PC con l'avviso confermato; limiti impostati sul PC.
- Nei repository niente nominativi reali, niente riferimenti a strumenti di assistenza, niente segreti.

## Problemi noti
- Versione smartphone (segnalato 2026-10-24): causa trovata e corretta, il flusso WebSocket si chiudeva ogni secondo in accesso senza codice. Da riprovare sul telefono vero; restano da verificare layout e lampeggio grafico.
- Da fuori casa non ci si collega ancora (trasporti Tailscale/Cloudflare da fare); l'accesso senza codice vale solo per indirizzi della rete di casa.

## Convenzioni di lavoro
- Test prima del codice; per i cambi di interfaccia si controlla anche nel browser.
- I test del core con Aurora accesa: `cargo test --target-dir target/testrun` (Windows blocca l'eseguibile in uso). Mai lanciare cargo da `core/crates`.
- Aurora si guarda dall'IP del PC (192.168.1.225:8787), non da `localhost`; l'avvio di sviluppo imposta tutte le variabili in una sola chiamata (le variabili d'ambiente non passano tra chiamate PowerShell).
- Testi dell'interfaccia: una tabella per area in `app/src/i18n/strings/`, colonne `[it, en, de, fr, es, pt]`; i test impediscono chiavi mancanti o segnaposto diversi.

- Impostazioni del motore modificabili dall'interfaccia (`JtdxValues`, bozza in `lib/jtdxDraft.ts`): le modifiche non si salvano da sole, restano in bozza finche' non si preme «Applica e riavvia JTDX» (chiude JTDX, scrive `JTDX.ini` con copia `JTDX.ini.bak-aurora`, riavvia). Nel testo dell'interfaccia l'applicazione e' Aurora, JTDX e' «il motore».
- Avvio: `scripts/avvia-sviluppo.ps1` parte sempre da pulito (chiude core e pannelli rimasti, una sola finestra `wt -w aurora`, una sola copia di JTDX). Il cambio banda ritenta una volta: OmniRig a volte e' occupato per un istante.
- Regola di AGENTS.md: nessun riferimento all'assistente nei messaggi di commit; i commit locali fino a questo ciclo ne hanno ancora (trailer): da pulire prima del push insieme a nominativo e locatore reali.

- Livello dell'ingresso audio (2026-10-24): il volume che conta e' quello di registrazione di Windows del dispositivo della cascata (stesso di JTDX, che non ha un controllo dell'ingresso). `audio-spectrum` misura (`LevelMeter`, evento `audio_level` ogni 250 ms) e legge/scrive il volume con la libreria `windows` (`InputEndpoint`); `input-level` ha la regola AUTO (scende subito se satura, sale max 12 % dopo 3 s sotto 0,5, ferma in TX e con ingresso muto). Provato dal vivo solo in lettura: la scrittura e AUTO vanno provati con la radio.
- Storico (`decode-history`, `data/history.sqlite`, 30 giorni): non e' il log dei QSO e non vi finisce nulla. Servizi online (`online-info`, `data/online.json`): PSK Reporter e hamqsl.com, **spenti di default**; non si invia mai nulla di cio' che si riceve o trasmette (niente invio spot a PSK Reporter: lo fa gia' JTDX).
- Decodium (GPL-3.0) e' stato studiato solo come ispirazione (`docs/confronto-decodium.md`): nessun codice copiato. Licenza di Aurora: GPL-3.0-or-later.
- Automazione: filtro dei falsi decode prima di ogni risposta e regola `reply_to_wanted` (usa la lista dei desiderati); restano dietro tutte le salvaguardie. Da provare in prova a vuoto prima del modo reale.
- Comandi a JTDX (2026-10-24): due sessioni PowerShell fisse in `rig-control::session` (comandi e letture) con memoria dei controlli di 10 minuti; gli script scritti per un processo si adattano (`exit` diventa `throw`). La frequenza OmniRig e i comandi su processi di JTDX restano a un processo per volta. Il cambio banda rimette il modo della radio solo se OmniRig lo permette (`WriteableParams`).
- Automazione: un ciclo = un gruppo di messaggi (finestra di 350 ms nel runner, `Brain::on_batch`): si risponde a un CQ solo. Rango: lista dei desiderati 5, paese nuovo 4, nuovo su banda 3, zona 2, griglia 1; preferenza regolabile (`prefer`).
