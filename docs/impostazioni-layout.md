# Impostazioni: sistema di layout delle schede

Ogni scheda delle Impostazioni e' una pagina larga a riquadri e colonne. I componenti stanno in `app/src/settings/layout/index.tsx`, lo stile in `app/src/styles/settings-layout.css`, i titoli nuovi in `app/src/i18n/strings/layout.ts`.

## Componenti

| Componente | A cosa serve |
|---|---|
| `Page` | griglia della scheda: 3 colonne da 1200 px, 2 da 760 a 1199 px, 1 sul telefono |
| `Group title span rows aside` | riquadro con titolo e cornice; `span` = colonne occupate (1, 2, 3), `rows` = righe occupate, `aside` = un tasto o un contatore a destra del titolo |
| `Cells min` | griglia di celle che si adatta alla larghezza del riquadro (`min` = larghezza minima di una cella in rem) |
| `Field label hint stack wide changed` | cella con etichetta a sinistra (due righe al massimo, il testo intero e' nel suggerimento) e controllo a destra; e' una `<label>` |
| `Cell label` | come `Field` ma senza `<label>` (un valore, dei pulsanti) |
| `Actions`, `Message` | pulsanti in fila e messaggi di stato dentro un riquadro |
| `Notes` | note lunghe in un riquadro comprimibile (in fondo a una colonna) |
| `Unavailable count` | voci che Aurora non ha, in un riquadro compatto chiuso |

Anche `Check` e `Row` di `settings/common.tsx` sono celle (stesso aspetto di `Field`). Le tabelle vere usano `sx-table` dentro `sx-tablewrap` (intestazioni di colonna, ogni cella col suo perimetro; `sx-tablewrap--scroll` fissa l'altezza e scorre dentro).

## Regole

- Niente liste verticali una sotto l'altra: gruppi per argomento, voci su piu' colonne.
- Nessuna voce e nessuna funzione si toglie: le voci che Aurora non ha restano in `Unavailable`, con il motivo e il controllo spento (resta nell'albero per l'accessibilita' e per i test).
- I testi di aiuto lunghi vanno in `Notes`; i messaggi che servono subito (avvisi, errori) restano visibili dentro il riquadro.
- Titoli di riquadro diversi dalle etichette delle voci: cosi' le etichette restano uniche per i test e per i lettori di schermo.
- Testi nuovi solo con le sei traduzioni (`layout.ts`).

## Barra delle schede

Due righe ordinate per tipo (dieci schede ciascuna), definite in `app/src/pages/Settings.tsx` (`ROWS`): «Stazione e trasmissione» (Generali, Radio, Audio, Frequenze, Bande, Sequenza, Macro TX, Programmazione, Filtri, Avanzate) e «Avvisi, dati e sistema» (Notifiche, Desiderati, DXCC, DX Cluster, Segnalazioni, Servizi online, Orologio, Aggiornamenti, Accesso, Finestre). Sul telefono ogni riga scorre di lato. La vecchia scheda «Radio CAT» e' dentro «Radio» (il vecchio indirizzo `cat` apre «Radio»).

## Da riordinare quando gli altri lavori sono finiti

Le schede **Audio** (`InfoTabs.tsx`, `AudioDevicesPanel.tsx`) e **Segnalazioni** (`InfoTabs.tsx`, `NativeSettings.tsx` con `NativeReports`) non sono ancora nel sistema: si vedono a tutta larghezza ma con le liste verticali di prima. Per riordinarle:

1. avvolgere il contenuto in `<Page>` e dividerlo in `Group` per argomento (Audio: ingresso, uscita, canali, file audio, livelli; Segnalazioni: registrazione del log, file ADIF, server UDP primario e secondario, server TCP, spot online, ALL.TXT);
2. sostituire `<div className="jx-list">` con `<Cells>` (le righe `SettingField` dentro un `Group` prendono da sole l'aspetto a cella, vedi `.sx-body .jx-row` nel CSS);
3. mettere gli aiuti lunghi (`ens.rep.help`, `ens.note`, `ens.audio.help`) in `<Notes>` e le voci spente in `<Unavailable count>`;
4. aggiornare `Settings.test.tsx` solo se cambiano i testi.
