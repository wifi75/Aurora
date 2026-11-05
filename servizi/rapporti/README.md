# aurora-rapporti

Il Worker Cloudflare che riceve i rapporti degli errori di Aurora. Nel programma i rapporti sono **facoltativi e spenti di
fabbrica** (Impostazioni → Assistenza → «Rapporti degli errori»); cosa contengono e cosa no è scritto in
[`docs/privacy.md`](../../docs/privacy.md).

| Rotta | Chi la usa | Cosa fa |
| --- | --- | --- |
| `POST /v1/report` | Aurora (livello 2) | rapporto breve JSON, max 32 KB → R2 `report/AAAA-MM-GG/<uuid>.json` + indice D1 |
| `POST /v1/bundle` | Aurora (dopo un crash, scheda Assistenza) | zip senza dati personali, max 10 MB → R2 `bundle/AAAA-MM-GG/<uuid>.zip` + codice breve in D1 |
| `GET /v1/summary` | tu | errori più frequenti per versione (segreto `ADMIN_TOKEN`) |
| `GET /v1/bundle/<codice>` | tu | scarica il pacchetto con il codice che l'utente ti ha detto (segreto `ADMIN_TOKEN`) |

Le richieste di Aurora devono avere `X-Aurora-Key` uguale a `CHIAVE_INGRESSO` di `wrangler.toml`: è una chiave **pubblica**
(sta anche nel codice di Aurora), serve solo a scartare il rumore. C'è un limite di 30 richieste al minuto per mittente;
l'indirizzo IP si usa solo come chiave di quel contatore e non si registra né si salva (i log del Worker sono spenti).
Ogni notte alle 03:17 UTC un Cron Trigger cancella file e righe più vecchi di 90 giorni (`GIORNI_CONSERVAZIONE`).

## Mettere online (una volta)

Dalla cartella `servizi/rapporti`:

```powershell
npm install
npx wrangler login
npx wrangler r2 bucket create aurora-rapporti
npx wrangler d1 create aurora-rapporti
```

`d1 create` stampa un `database_id`: incollalo in `wrangler.toml` al posto di `INCOLLA-QUI-L-ID-DEL-DATABASE`. Poi:

```powershell
npx wrangler d1 execute aurora-rapporti --remote --file=schema.sql
npx wrangler secret put ADMIN_TOKEN
npx wrangler deploy
```

`secret put` chiede il valore: scegli una stringa lunga e casuale (almeno 16 caratteri, per esempio l'uscita di
`[guid]::NewGuid().ToString('N')` in PowerShell) e tienila solo per te; non va mai nel repository.

`deploy` stampa l'indirizzo, del tipo `https://aurora-rapporti.<sottodominio>.workers.dev`. Mettilo in
`core/crates/aurora/src/rapporti.rs`, costante `INDIRIZZO` (finché c'è il segnaposto `SOTTODOMINIO` Aurora non manda niente e
nasconde il pulsante «Manda il pacchetto a chi sviluppa Aurora»), e fai una versione nuova.

## Leggere il riepilogo

```powershell
$t = 'IL-TUO-ADMIN_TOKEN'
$u = 'https://aurora-rapporti.<sottodominio>.workers.dev'
Invoke-RestMethod "$u/v1/summary?giorni=7" -Headers @{ Authorization = "Bearer $t" } | ConvertTo-Json -Depth 5
Invoke-RestMethod "$u/v1/summary?giorni=30&versione=0.11.96" -Headers @{ Authorization = "Bearer $t" }
# un pacchetto dal codice che ti ha detto l'utente
Invoke-WebRequest "$u/v1/bundle/1A2B3C4D" -Headers @{ Authorization = "Bearer $t" } -OutFile aurora-1A2B3C4D.zip
```

I rapporti singoli si vedono anche nella dashboard di Cloudflare (R2 → `aurora-rapporti`) o con
`npx wrangler r2 object get aurora-rapporti/report/AAAA-MM-GG/<uuid>.json --remote`.

## Controllo del codice

```powershell
npx tsc --noEmit
```

## Totale delle installazioni (dal 2026-11-05)

`GET /v1/attivi` restituisce anche `totale` (tutte le installazioni che hanno usato Aurora dall'inizio) e `nuove_per_giorno`.
Le righe di `attivi` si cancellano dopo 90 giorni, quindi il totale vive in una tabella a parte, `installazioni`
(identificativo, primo e ultimo giorno); quelle ferme da un anno passano al contatore `archiviate` della tabella `contatori`.

Per metterlo online, una volta, dalla cartella `servizi/rapporti`:

```powershell
npx wrangler d1 execute aurora-rapporti --remote --file=schema.sql
npx wrangler d1 execute aurora-rapporti --remote --command "INSERT INTO installazioni (installazione, prima, ultima) SELECT installazione, MIN(giorno), MAX(giorno) FROM attivi GROUP BY installazione ON CONFLICT (installazione) DO NOTHING"
npx wrangler deploy
```

Il secondo comando riempie l'elenco con le installazioni gia' viste negli ultimi 90 giorni (il conteggio e' partito il 2026-11-04).
