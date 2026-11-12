# Informativa sulla privacy di Aurora

Aurora è un programma che gira sul tuo PC. I dati della stazione (nominativo, locatore, log dei QSO, impostazioni, password dei
servizi) restano nella cartella dati sul PC (di norma `%APPDATA%\Aurora`). Escono solo verso i servizi elencati qui sotto,
con le regole di quei servizi.

## Servizi accesi di fabbrica

Dalla versione 0.11.102 questi servizi sono **accesi di fabbrica** e si spengono in qualunque momento:

- **PSK Reporter**, lettura (Impostazioni → Servizi online, pagina Propagazione): Aurora resta collegata al flusso in tempo reale `mqtt.pskreporter.info` (solo i rapporti sul tuo nominativo) e ogni 5 minuti chiede a pskreporter.info chi ti ha ricevuto nelle ultime 3 ore. I due servizi vedono il tuo nominativo e il tuo indirizzo IP.
- **PSK Reporter**, spotting (Impostazioni → Programmi collegati): manda i nominativi che ricevi, con frequenza, rapporto, il tuo nominativo e il locatore, come fanno gli altri programmi per i modi digitali.
- **Propagazione** (hamqsl.com) e **calendario delle DXpedition** (NG3K): sola lettura di pagine pubbliche, senza dati tuoi.
- **Messaggi dell'autore e versioni nuove** (dalla 0.11.168): Aurora e l'agente nel tray leggono ogni ora i messaggi pubblicati dall'autore
  (servizio `aurora-rapporti` su Cloudflare) e l'ultima versione su GitHub. Nella richiesta dei messaggi c'è solo il numero di versione di
  Aurora, per i messaggi rivolti alle versioni vecchie: nessun nominativo, nessun dato della stazione.
  Dalla 0.11.182, quando una cartolina con un messaggio compare, Aurora (o l'agente) lo dice al servizio con il codice casuale
  dell'installazione, lo stesso del conteggio delle installazioni attive e solo se quel conteggio è acceso: serve solo a sapere quante
  installazioni hanno visto il messaggio. Nessun nominativo, nessun indirizzo salvato.
- **Agente nel tray** (dalla 0.11.169): lo stesso programma, avviato con Windows, che fa le letture qui sopra (versioni, messaggi,
  propagazione, DXpedition) anche con Aurora chiusa; legge il riepilogo del giorno dal log di Aurora sul PC, senza mandarlo da nessuna parte.
  Si spegne in Impostazioni → Agente nel tray.
- **DX Cluster**: Aurora entra nei server telnet scelti con il nominativo dei dati di stazione (nessuna password) e riceve gli spot.
- **Comando della radio** (Hamlib `rigctld`, OmniRig via COM, Cat4OM, TCI): tutto in locale sul PC (di norma `127.0.0.1`, per Cat4OM la porta di gestione `127.0.0.1:17300` e la porta Control del gruppo che indica); niente esce dal computer.
- **WSPR e FST4W**: Aurora decodifica e trasmette il radiofaro sul tuo PC e, di fabbrica, **non manda niente a WSPRnet** (come WSJT-X, dove
  «Upload spots» è spento). FST4 non manda mai niente.
- **Controllo dell'ora** (NTP) e **controllo degli aggiornamenti** su GitHub: sola lettura. Quando c'e' una versione nuova, Aurora scarica in anticipo da GitHub il pacchetto e la sua firma nella cartella temporanea di Windows (`%TEMP%\Aurora-update`), senza mandare dati; si installa solo quando lo chiedi.

**Spot a WSPRnet** (spento di fabbrica, Impostazioni → Servizi online → «Invia gli spot a WSPRnet»). Acceso, dopo ogni periodo WSPR o FST4W
ricevuto Aurora manda a `http://wsprnet.org/post/` (http, non cifrato, come WSJT-X) una richiesta per ogni segnale decodificato con:
nominativo, locatore e potenza di chi trasmette, data e ora del periodo, rapporto S/N, scarto di tempo, deriva, frequenza; il tuo
nominativo, il tuo locatore, la tua frequenza di ricezione, il modo e la versione di Aurora. Se nel periodo non c'è niente parte una sola
richiesta di stato con il tuo nominativo, il locatore, le frequenze di ricezione e di trasmissione, la percentuale «Tx Pct» (0 con
«Abilita Tx» spento), la potenza del radiofaro, il modo e la versione. WSPRnet vede anche il tuo indirizzo IP e pubblica gli spot sul suo
sito. Le richieste partono fra 0 e 20 s dopo la decodifica; se WSPRnet non risponde si riprova per un'ora al massimo e poi si lasciano
perdere; spegnendo l'opzione la coda si svuota e non parte più niente. A differenza di WSJT-X 3.3, Aurora non manda gli spot anche al secondo
indirizzo `wsprnet.eu`.

Restano spenti finche' non li accendi tu, perche' chiedono un tuo account: eQSL, LoTW, ClubLog, QRZ.com, HRDLog, Wavelog,
ricerca dei nominativi su QRZ.com e HamQTH. Con la ricerca accesa Aurora manda a quei servizi solo il nominativo da cercare (anche in
anticipo, per i nominativi visibili nel DX Cluster e in Attivita' di banda); se sono accese tutte e due e QRZ.com non da' le informazioni
QSL, lo stesso nominativo si chiede anche a HamQTH.

## Rapporti degli errori

I rapporti degli errori servono a chi sviluppa Aurora per trovare e correggere i problemi. Sono **accesi di fabbrica**: l'installatore e il primo avvio lo dicono, e si spengono in qualunque momento in
Impostazioni → Assistenza → «Rapporti degli errori».

- **Dopo un crash** («Chiedimi di mandare il rapporto dopo un crash»): al riavvio Aurora chiede se mandare il rapporto. Parte
  solo se premi «Manda il rapporto». Lo stesso pacchetto parte con «Manda il pacchetto a chi sviluppa Aurora».
- **Errori automatici** («Manda in automatico i rapporti degli errori (anonimi)»): per ogni errore grave parte un rapporto breve.
  Errori uguali si raggruppano: al massimo un invio per errore ogni ora e 20 al giorno; senza rete aspettano in una coda sul PC.

**Cosa contengono**: versione di Aurora e di Windows, motore, modello della radio, metodo di PTT, backend del CAT, nome della
scheda audio, area del problema (CAT, PTT, audio, decodifica, UDP, servizi, associazione del telefono…), il messaggio d'errore ripulito, quante volte è
successo, il giorno e un identificativo casuale dell'installazione. Il pacchetto dopo un crash aggiunge il log del programma
ripulito (senza le righe di decodifica e di QSO) e i rapporti di testo del crash. L'inizio del rapporto di testo del crash, già ripulito allo stesso modo, si salva anche nell'indice del servizio, per leggerlo senza aprire il pacchetto.

**Associazione del telefono non riuscita** (dalla 0.11.123): se un codice scade senza che il telefono si associ, il rapporto dice solo il
*tipo* di indirizzo messo nel QR (per esempio «192.168, rete di casa» o «adattatore virtuale»), il tipo di adattatore (Wi-Fi, cavo,
virtuale), quanti indirizzi ha il PC, se la rete di Windows è Pubblica o Privata, se il firewall ha la regola per Aurora, se il
telefono è arrivato ad Aurora e come è finita. Mai l'indirizzo IP, il nome della rete Wi-Fi o il nome degli adattatori.

**Audio e decodifica** (dalla versione dopo la 0.11.131): il rapporto dei blocchi audio persi dice la causa indicata da Windows, da
quanti minuti, quanti al minuto e il *tipo* di ingresso (USB, integrata, virtuale), con frequenza, canali e formato, mai il nome della
scheda. Se la decodifica è spesso lenta, il rapporto dice i tempi, il numero di processori e quali opzioni del decoder sono accese.

**Cosa non contengono mai**: nominativo, locatore, messaggi decodificati, log dei QSO, `ALL.TXT`, impostazioni, password, chiavi
e token, indirizzi IP, email, il nome utente di Windows e il nome del PC, i minidump (contengono la memoria del programma). Prima
dell'invio una pulizia sostituisce nominativi e locatori riconosciuti, percorsi `C:\Users\<nome>`, indirizzi IP, email e codici
lunghi con segnaposto come `<nominativo>`. «Vedi cosa mando» mostra esattamente il testo che partirebbe.

**L'identificativo** è un numero casuale (UUID) creato sul PC, non legato alla persona; «Rigenera» ne crea uno nuovo.

**Dove vanno**: a un servizio di chi sviluppa Aurora su Cloudflare (Workers, R2 e D1). Il servizio non registra e non salva
l'indirizzo IP del mittente: lo usa solo per limitare le richieste.

**Quanto restano**: 90 giorni, poi una pulizia automatica notturna li cancella.

**Come spegnerli**: togli il segno alle due opzioni. Spegnendo gli errori automatici si svuota anche la coda sul PC.

## Rapporti dell'installatore

Ogni installazione o aggiornamento su Windows scrive un **registro sul PC**, `installazione.log` nella cartella dei dati di Aurora
(`%APPDATA%\Aurora`, o quella di `AURORA_DATA_DIR`); ogni installazione sovrascrive il registro della precedente. Contiene: versione
nuova e versione installata prima, modo (normale, passivo, aggiornamento silenzioso da Aurora), versione e build di Windows,
architettura, tipo di account (amministratore o utente), lingua dell'installatore, cartella di destinazione e spazio libero sul disco,
WebView2 (versione presente, scaricamento e installazione), se Aurora o il suo core erano aperti e sono stati chiusi, la cartella
di Hamlib vecchia non tolta (file bloccati), i file non copiati o spariti subito dopo la copia (per esempio `rigctld.exe` tolto da un
antivirus), l'esito e, quando l'installatore mostra la sua finestra, l'elenco dei dettagli. Solo se qualcosa non va si aggiungono il
nome e lo stato degli antivirus registrati in Sicurezza di Windows e i programmi ancora aperti dalla cartella di installazione. Il
registro resta sul PC finché non parte uno dei rapporti qui sotto.

**Quando l'installazione non riesce** (WebView2 non scaricato o non installato, Aurora che non si chiude, file che non si possono
scrivere, versione più vecchia rifiutata in silenzio, installazione fermata da un errore) l'installatore manda:

- un **rapporto breve** come quelli degli errori automatici, area `installatore`: identificativo casuale dell'installazione (lo
  stesso di Aurora se c'è già, `rapporti-installazione.txt`, altrimenti uno nuovo che non si salva), giorno, versione, Windows,
  processore, numero di core e memoria, e il messaggio con il motivo, gli avvisi e gli errori del registro, il modo e la versione di prima;
- il **registro ripulito** in un pacchetto zip (motivo `installazione`), con un `info.json` con gli stessi dati.

Prima dell'invio la stessa pulizia dei rapporti di Aurora toglie nome utente, nome del PC e dominio di Windows (i nomi generici
come «User», «Admin» o «Radio» restano: non dicono chi sei), i percorsi `C:\Users\<nome>` (diventano `C:\Users\<utente>`), email,
indirizzi IP, nominativi, locatori, valori dopo `password=` o `token:` e codici lunghi. L'invio passa da PowerShell (già presente in
Windows 10 e 11; se manca non si manda nulla), nascosto e con un tempo massimo di 60 secondi: senza rete l'installatore non resta appeso
e il registro resta comunque sul PC.

**Quando l'installazione riesce ma con degli avvisi**, o si è fermata a metà, l'installatore non manda nulla: al primo avvio
Aurora legge il registro e, se ci sono avvisi o errori, manda **un solo rapporto breve** area `installatore` con gli avvisi
ripuliti, una volta per installazione (si ricorda l'impronta del registro in `rapporti-installazione-vista.txt`). Se l'installatore
ha già mandato il suo rapporto, Aurora non lo rimanda. Il registro ripulito entra anche nel pacchetto dopo un crash, in quello
«Manda il pacchetto a chi sviluppa Aurora» e nel pacchetto per l'assistenza sul Desktop.

**La tua scelta vale anche qui**: se in Aurora hai spento «Manda in automatico i rapporti degli errori» (`"auto_errors": false` in
`rapporti.json`), né l'installatore né Aurora mandano rapporti dell'installazione; il registro resta solo sul PC. Alla **prima
installazione** la scelta non c'è ancora: vale quella di fabbrica di Aurora, cioè rapporti accesi, come dice la pagina «Dati che
Aurora manda in rete» dell'installatore. Un'installazione annullata prima di cominciare non scrive il registro e non manda nulla.

## Conteggio delle installazioni attive

Dalla versione 0.11.116 Aurora, una volta al giorno, dice al servizio dei rapporti «questa installazione è aperta oggi». Serve
solo a sapere quante persone usano Aurora e con quale versione. È **acceso di fabbrica** e si spegne in Impostazioni → Assistenza
→ «Invia statistiche».

**Cosa contiene**: l'identificativo casuale dell'installazione (lo stesso dei rapporti, rigenerabile), la versione di Aurora e
quella di Windows, il giorno, dalla 0.11.214, se hai impostato il nominativo, un **codice ricavato dal nominativo** e, dalla
0.11.220, il **tipo di computer**: modello del processore, numero di core e memoria in GB (niente numeri di serie, nome del computer o
indirizzi). Gli stessi dati del computer accompagnano i rapporti degli errori, per capire su quali macchine Aurora fatica. **Niente
altro**: il nominativo, il locatore, la radio, il log e le impostazioni non si mandano. Il servizio non registra l'indirizzo IP.
Le righe restano 90 giorni, poi la pulizia notturna le cancella.

**Il codice ricavato dal nominativo** serve a contare le persone e non i PC (chi usa Aurora su due computer conta una volta).
Aurora lo calcola sul tuo PC: prende il nominativo di base (maiuscolo, senza /P, /M, /MM, /QRP, /A ne' prefissi come EA/) e ne fa
l'impronta SHA-256 insieme a un testo fisso del progetto. **Il nominativo non lascia mai il PC.** Il servizio, prima di salvarlo,
trasforma di nuovo il codice con una chiave segreta (HMAC-SHA256) e ne tiene 32 caratteri: nel database non c'e' nemmeno il codice
mandato. Senza nominativo impostato si manda solo il codice casuale, come prima.

**Storico delle installazioni**: per sapere quante installazioni hanno usato Aurora dall'inizio, il servizio tiene per sempre un
elenco con il solo identificativo casuale, il primo e l'ultimo giorno in cui l'installazione si è fatta viva e la **nazione**
(la sigla di due lettere, per esempio IT, che Cloudflare ricava dalla connessione; niente città, regione o indirizzo IP), il codice della persona trasformato come sopra (se c'e'), e i conteggi di ogni giorno per versione e sistema (solo numeri, senza identificativi). Rigenerando
l'identificativo in Impostazioni → Assistenza si conta come un'installazione nuova.

## Pacchetto per l'assistenza

«Prepara il pacchetto per l'assistenza» crea uno zip sul Desktop e non lo manda a nessuno: lo mandi tu, a chi vuoi. Quel
pacchetto contiene nominativo, locatore e messaggi ricevuti (ma nessuna password).
