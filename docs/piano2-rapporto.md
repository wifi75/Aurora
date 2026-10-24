# Piano 2 — Rapporto di esecuzione (core e interfaccia su stazione simulata)

Data: 2026-10-24 · Versione: 0.2.0 · Stato: **codice completo e verificato dai test automatici; verifiche manuali di Tiziano da fare**

## Esito automatico

| Verifica | Esito |
|---|---|
| `cargo test --workspace` | 85 passati, 0 falliti, 1 ignorato (il test sulle catture reali di JTDX, in attesa dei pacchetti) |
| `cargo clippy --workspace --all-targets` | nessun avviso |
| `npm test` (interfaccia) | 55 passati in 12 file |
| `npm run build` (`tsc` + Vite) | riuscita |
| Ricerca dei nomi degli strumenti di assistenza nei file tracciati | nessun risultato |

Copertura dei test sui punti di rischio del piano: client WebSocket lento (non blocca gli altri), codice di associazione (5 errori ⇒ blocco di 60 s),
righe di spettro non valide (scartate, mai inoltrate), `Reply` su decode sconosciuto (404) e `HaltTx` idempotente, cookie assente/sconosciuto/revocato (401, anche sul WebSocket).

## Decisioni prese durante l'esecuzione

- **`halt-tx` non è mai bloccato dalla policy di trasporto** (il piano lo lasciava implicito): fermare la trasmissione è sempre ammesso; la policy blocca solo `reply`.
- **Trasporto ricavato dalle intestazioni** (`cf-ray`, `tailscale-user-login`), non da un'intestazione scelta dal client: chi arriva da Cloudflare non può farsi passare per rete locale.
- **Spegnimento ordinato**: `AppState::begin_shutdown` chiude i WebSocket aperti (senza, lo spegnimento restava appeso ai flussi).
- **Righe di spettro validate anche nel WebSocket**, oltre che alla sorgente, e **cookie `Secure` configurabile** (`AURORA_SECURE_COOKIE`): emersi dalla revisione finale del branch, con test scritto prima.
- **Niente Tailwind e niente `react-router`**: l'interfaccia usa i token CSS e un routing a hash di due pagine (YAGNI).
- **TypeScript 7.0.2, Vite 8.3.5 e Vitest 5.0.5 funzionano insieme**: il ripiego a TypeScript 6 non è servito.
- **`run(config, shutdown)`** come da piano; `/api/{*rest}` risponde 404 invece di servire `index.html`.
- **Revisione finale fatta da me, non da un revisore indipendente.** Il giudizio di un secondo paio d'occhi resta consigliato prima di aprire Aurora fuori dalla rete locale.

## Limiti noti

- **Il codice di associazione è globale**: chi indovina male 5 volte blocca per 60 secondi anche l'associazione legittima. Accettato: il codice vive 2 minuti e lo genera il proprietario.
- **Nessun HTTPS locale né mDNS** (`aurora.local`): sono previsti nel Piano 3 (trasporti). Per ora l'accesso da telefono è `http://<IP-del-PC>:5173` sulla stessa rete.
- **L'interfaccia non è stata provata in un browser da me**, per tua regola: la cascata (WebGPU/WebGL2), il layout e l'installazione come app sono verificati solo dai test della logica e dalla build.
- Il PC deve restare acceso con Aurora in esecuzione; il telefono è solo uno schermo.

## Verifiche manuali per Tiziano

1. `.\scripts\avvia-sviluppo.ps1` — attendi «in ascolto»; apri `http://127.0.0.1:5173`.
2. `.\scripts\associa-telefono.ps1` — compare il QR; inquadralo col telefono (stessa rete Wi-Fi) e conferma. **Atteso:** pagina Live con cascata che scorre, decode che compaiono, STOP TX in basso.
3. **Cascata:** scorre fluida, tracce nitide, colori dal blu notte al ciano e all'ambra. Annota qui il renderer usato: `data-backend` del contenitore `.waterfall` (`webgpu` o `webgl2`, dagli strumenti per sviluppatori) e i fotogrammi al secondo percepiti.
4. **Telefono:** layout in verticale e in orizzontale, tocco su un decode (risposta simulata: lo stato diventa TX), STOP TX, installazione come app dal menu del browser.
5. **Revoca:** dall'API (`DELETE /api/v1/devices/{id}`) il telefono deve perdere l'accesso entro un secondo.

Esiti: _da compilare dopo le prove_.

## Prossimi passi

- **Piano 3:** collegamento reale a JTDX (`JtdxEngine` sopra `udp-bridge`), dopo le prove manuali della Fase 0; poi trasporti (HTTPS locale, Tailscale, Cloudflare), audio reale per la cascata e controllo radio.
- Il tag `v0.2.0` si mette dopo le tue verifiche manuali.
