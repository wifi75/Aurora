# Firma gratuita dell'installatore con SignPath

Scopo: l'installatore per Windows firmato con il certificato della **SignPath Foundation** (gratuito per i progetti open source), cosi'
Windows e SmartScreen non lo trattano come sconosciuto.

## Come funziona

1. Si pubblica una release su GitHub come sempre.
2. Il lavoro `.github/workflows/windows.yml` compila l'installatore sulle macchine di GitHub (SignPath accetta solo quelle) e chiede la firma.
3. Il proprietario **approva a mano** la richiesta sul sito di SignPath (c'e' tempo fino a 12 ore).
4. Il lavoro controlla la firma di Windows, fa la firma degli aggiornamenti (minisign) e allega alla release `Aurora_x.y.z_x64-setup.exe`,
   la sua `.sig`, `Aurora-Setup.exe` e `SHA256SUMS.txt`, sostituendo quelli caricati dal PC.

Finche' i segreti non ci sono, il lavoro compila soltanto e non allega niente.

## Requisiti della SignPath Foundation (gia' a posto nel repository)

- Licenza OSI per tutto il progetto (GPL-3) e niente componenti proprietari.
- Compilazione dal codice pubblico su macchine di GitHub: `windows.yml`.
- Pagina «Code signing policy» con la frase richiesta, ruoli e privacy: sezione nei sei README.
- Elenco dei dati che escono dal PC mostrato durante l'installazione, con il modo di spegnerli: pagina «Dati che Aurora manda in rete»
  dell'installatore (`desktop/src-tauri/installer/installer.nsi`), dettagli in `docs/privacy.md`.
- Le DLL di Hamlib sono di terzi e non firmate: SignPath le accetta cosi'.

## Cosa deve fare il proprietario

1. **GitHub**: attivare l'autenticazione a due fattori (Settings > Password and authentication). SignPath la richiede.
2. **Domanda**: compilare il modulo su <https://signpath.org/apply> con
   - progetto: Aurora, <https://github.com/wifi75/Aurora>;
   - descrizione: «Free and open source FT8/FT4/JT65/JT9/Q65 station program for radio amateurs (GPL-3), Windows/Linux/macOS»;
   - pagina della policy: <https://github.com/wifi75/Aurora#code-signing-policy>.
3. Quando SignPath approva, sul loro sito: creare il progetto `Aurora` collegato al repository GitHub, la policy di firma
   `release-signing` con approvazione manuale, e un token API (utente CI).
4. **Salvare in GitHub** > Settings > Secrets and variables > Actions:

   | Tipo | Nome | Valore |
   |---|---|---|
   | Secret | `SIGNPATH_API_TOKEN` | il token API di SignPath |
   | Variable | `SIGNPATH_ORGANIZATION_ID` | l'ID dell'organizzazione SignPath (sul loro sito) |
   | Variable | `SIGNPATH_PROJECT_SLUG` | `Aurora` (solo se diverso) |
   | Variable | `SIGNPATH_POLICY_SLUG` | `release-signing` (solo se diverso) |
   | Secret | `AURORA_TIMBRO_AGGIORNAMENTI_AURORA_KEY` | la chiave che firma gli aggiornamenti automatici: il contenuto del file `%USERPROFILE%\.aurora-signing\aurora.key` |
   | Secret | `AURORA_PASSWORD_TIMBRO_AGGIORNAMENTI` | la password di quella chiave |

5. Alla prima release dopo l'attivazione: approvare la richiesta su SignPath e controllare che gli allegati della release siano stati sostituiti.

## Prova senza release

Actions > Windows > Run workflow, con il tag di una release esistente (es. `v0.11.196`).

## Esito della prima domanda (2026-11-08)

SignPath Foundation ha risposto di no per ora: il programma Foundation chiede segnali di visibilita' pubblica (stelle, fork, contributori su
GitHub, articoli, discussioni esterne, video). Si puo' ripresentare la domanda quando Aurora sara' piu' conosciuta. Nel frattempo
etichetta e sezione «Code signing policy» sono state tolte dai README; il workflow `windows.yml` resta (senza segreti compila soltanto).
