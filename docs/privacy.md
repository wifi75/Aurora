# Informativa sulla privacy di Aurora

Aurora è un programma che gira sul tuo PC. I dati della stazione (nominativo, locatore, log dei QSO, impostazioni, password dei
servizi) restano nella cartella dati sul PC (di norma `%APPDATA%\Aurora`). Escono solo verso i servizi elencati qui sotto,
con le regole di quei servizi.

## Servizi accesi di fabbrica

Dalla versione 0.11.102 questi servizi sono **accesi di fabbrica** e si spengono in qualunque momento:

- **PSK Reporter**, lettura (Impostazioni → Servizi online, pagina Propagazione): ogni tanto Aurora chiede chi ti ha ricevuto; nella richiesta c'è il tuo nominativo.
- **PSK Reporter**, spotting (Impostazioni → Programmi collegati): manda i nominativi che ricevi, con frequenza, rapporto, il tuo nominativo e il locatore, come fanno gli altri programmi FT8.
- **Propagazione** (hamqsl.com) e **calendario delle DXpedition** (NG3K): sola lettura di pagine pubbliche, senza dati tuoi.
- **DX Cluster**: Aurora entra nei server telnet scelti con il nominativo dei dati di stazione (nessuna password) e riceve gli spot.
- **Controllo dell'ora** (NTP) e **controllo degli aggiornamenti** su GitHub: sola lettura.

Restano spenti finche' non li accendi tu, perche' chiedono un tuo account: eQSL, LoTW, ClubLog, QRZ.com, HRDLog, Wavelog,
ricerca dei nominativi su QRZ.com e HamQTH.

## Rapporti degli errori

I rapporti degli errori servono a chi sviluppa Aurora per trovare e correggere i problemi. Sono **accesi di fabbrica**: l'installatore e il primo avvio lo dicono, e si spengono in qualunque momento in
Impostazioni → Assistenza → «Rapporti degli errori».

- **Dopo un crash** («Chiedimi di mandare il rapporto dopo un crash»): al riavvio Aurora chiede se mandare il rapporto. Parte
  solo se premi «Manda il rapporto». Lo stesso pacchetto parte con «Manda il pacchetto a chi sviluppa Aurora».
- **Errori automatici** («Manda in automatico i rapporti degli errori (anonimi)»): per ogni errore grave parte un rapporto breve.
  Errori uguali si raggruppano: al massimo un invio per errore ogni ora e 20 al giorno; senza rete aspettano in una coda sul PC.

**Cosa contengono**: versione di Aurora e di Windows, motore, modello della radio, metodo di PTT, backend del CAT, nome della
scheda audio, area del problema (CAT, PTT, audio, decodifica, UDP, servizi…), il messaggio d'errore ripulito, quante volte è
successo, il giorno e un identificativo casuale dell'installazione. Il pacchetto dopo un crash aggiunge il log del programma
ripulito (senza le righe di decodifica e di QSO) e i rapporti di testo del crash.

**Cosa non contengono mai**: nominativo, locatore, messaggi decodificati, log dei QSO, `ALL.TXT`, impostazioni, password, chiavi
e token, indirizzi IP, email, il nome utente di Windows e il nome del PC, i minidump (contengono la memoria del programma). Prima
dell'invio una pulizia sostituisce nominativi e locatori riconosciuti, percorsi `C:\Users\<nome>`, indirizzi IP, email e codici
lunghi con segnaposto come `<nominativo>`. «Vedi cosa mando» mostra esattamente il testo che partirebbe.

**L'identificativo** è un numero casuale (UUID) creato sul PC, non legato alla persona; «Rigenera» ne crea uno nuovo.

**Dove vanno**: a un servizio di chi sviluppa Aurora su Cloudflare (Workers, R2 e D1). Il servizio non registra e non salva
l'indirizzo IP del mittente: lo usa solo per limitare le richieste.

**Quanto restano**: 90 giorni, poi una pulizia automatica notturna li cancella.

**Come spegnerli**: togli il segno alle due opzioni. Spegnendo gli errori automatici si svuota anche la coda sul PC.

## Pacchetto per l'assistenza

«Prepara il pacchetto per l'assistenza» crea uno zip sul Desktop e non lo manda a nessuno: lo mandi tu, a chi vuoi. Quel
pacchetto contiene nominativo, locatore e messaggi ricevuti (ma nessuna password).
