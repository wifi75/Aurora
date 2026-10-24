# Memoria di progetto

Stato al 2026-10-24, versione 0.8.0. Per riprendere il lavoro leggi anche `docs/mancante.md` (cosa manca) e `CHANGELOG.md`.

## Scelte di fondo
- JTDX 2.2.159 (`C:\JTDX64\159`) resta il motore di decodifica, CAT e TX; Aurora è cervello e interfaccia. JTDX non accetta `Configure` via UDP: le sue impostazioni di decodifica restano in JTDX.
- Core Rust (workspace `core/`, Tokio/Axum), interfaccia React + TypeScript + Vite, cascata WebGPU con ripiego WebGL2, PWA per il telefono. Guscio desktop Tauri 2 ancora da fare. Database SQLite.
- Nessun login a password: dispositivi associati con un codice monouso (durata scelta dall'operatore, 1-60 minuti, predefinito 10). Per le prove esiste l'accesso senza codice, solo rete di casa, mai da proxy.
- Grafica: "Notte radio" con vetro traslucido; colori dei messaggi come le 17 categorie di JTDX, configurabili.

## Come funziona il collegamento con la stazione
- JTDX → server UDP primario `127.0.0.1:2334` = hub di Aurora → inoltro identico a GridTracker2 (2333). Il server secondario di JTDX verso Log4OM2 (2237) non si tocca: non aggiungere lo stesso destinatario ad `AURORA_EXPORT_UDP`, i QSO sarebbero doppi.
- Cambio banda e frequenza Rx: stesso OmniRig che usa JTDX come CAT. Cambio modo FT8/FT4 e frequenza Rx: si comandano i controlli di JTDX con l'accessibilità di Windows (UI Automation) via PowerShell.
- `TriggerCQ` (UDP tipo 51): il secondo parametro a `true` abilita Tx (provato dal vivo).

## Sicurezza (non negoziabile)
- Trasmissione chiusa di default (`AURORA_ALLOW_TX=0`), due cancelli indipendenti (API e motore). STOP TX mai bloccato e spegne anche l'automazione.
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
