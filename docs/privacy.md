# Informativa sulla privacy di Aurora

Aurora è un programma che gira sul tuo PC. I dati della stazione (nominativo, locatore, log dei QSO, impostazioni, password dei
servizi) restano nella cartella dati sul PC (di norma `%APPDATA%\Aurora`). Escono solo verso i servizi elencati qui sotto,
con le regole di quei servizi.

## Servizi accesi di fabbrica

Dalla versione 0.11.102 questi servizi sono **accesi di fabbrica** e si spengono in qualunque momento:

- **PSK Reporter**, lettura (Impostazioni → Servizi online, pagina Propagazione): ogni tanto Aurora chiede chi ti ha ricevuto; nella richiesta c'è il tuo nominativo.
- **PSK Reporter**, spotting (Impostazioni → Programmi collegati): manda i nominativi che ricevi, con frequenza, rapporto, il tuo nominativo e il locatore, come fanno gli altri programmi FT8.
- **Propagazione** (hamqsl.com) e **calendario delle DXpedition** (NG3K): sola lettura di pagine pubbliche, senza dati tuoi.
- **Messaggi dell'autore e versioni nuove** (dalla 0.11.168): Aurora e l'agente nel tray leggono ogni ora i messaggi pubblicati dall'autore
  (servizio `aurora-rapporti` su Cloudflare) e l'ultima versione su GitHub. Nella richiesta dei messaggi c'è solo il numero di versione di
  Aurora, per i messaggi rivolti alle versioni vecchie: nessun nominativo, nessun dato della stazione.
- **Agente nel tray** (dalla 0.11.169): lo stesso programma, avviato con Windows, che fa le letture qui sopra (versioni, messaggi,
  propagazione, DXpedition) anche con Aurora chiusa; legge il riepilogo del giorno dal log di Aurora sul PC, senza mandarlo da nessuna parte.
  Si spegne in Impostazioni → Agente nel tray.
- **DX Cluster**: Aurora entra nei server telnet scelti con il nominativo dei dati di stazione (nessuna password) e riceve gli spot.
- **Controllo dell'ora** (NTP) e **controllo degli aggiornamenti** su GitHub: sola lettura.

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

## Conteggio delle installazioni attive

Dalla versione 0.11.116 Aurora, una volta al giorno, dice al servizio dei rapporti «questa installazione è aperta oggi». Serve
solo a sapere quante persone usano Aurora e con quale versione. È **acceso di fabbrica** e si spegne in Impostazioni → Assistenza
→ «Conta questa installazione tra quelle attive».

**Cosa contiene**: l'identificativo casuale dell'installazione (lo stesso dei rapporti, rigenerabile), la versione di Aurora e
quella di Windows, il giorno. **Niente altro**: nessun nominativo, locatore, radio, log o impostazione. Il servizio non registra
l'indirizzo IP. Le righe restano 90 giorni, poi la pulizia notturna le cancella.

**Storico delle installazioni**: per sapere quante installazioni hanno usato Aurora dall'inizio, il servizio tiene per sempre un
elenco con il solo identificativo casuale, il primo e l'ultimo giorno in cui l'installazione si è fatta viva e la **nazione**
(la sigla di due lettere, per esempio IT, che Cloudflare ricava dalla connessione; niente città, regione o indirizzo IP), e i conteggi di ogni giorno per versione e sistema (solo numeri, senza identificativi). Rigenerando
l'identificativo in Impostazioni → Assistenza si conta come un'installazione nuova.

## Pacchetto per l'assistenza

«Prepara il pacchetto per l'assistenza» crea uno zip sul Desktop e non lo manda a nessuno: lo mandi tu, a chi vuoi. Quel
pacchetto contiene nominativo, locatore e messaggi ricevuti (ma nessuna password).
