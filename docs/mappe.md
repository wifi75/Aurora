# Mappe della Terra

La pagina Satelliti ha, di base, la mappa «Terra (satellite NASA)»: la foto della Terra di giorno e, dove e' notte, la foto delle luci delle citta'. Le due immagini stanno in `app/public/maps/` e sono servite insieme all'interfaccia (`/maps/terra-giorno.jpg`, `/maps/terra-notte.jpg`).

## Fonti

| File | Fonte | Originale |
|---|---|---|
| `terra-giorno.jpg` | NASA Earth Observatory, Blue Marble Next Generation con topografia e batimetria, agosto 2004: <https://visibleearth.nasa.gov/images/74092/august-blue-marble-next-generation-w-topography-and-bathymetry> | 21600 x 10800 px (500 m), equirettangolare |
| `terra-notte.jpg` | NASA Earth Observatory, Black Marble, Night Lights 2012 (3 km): <https://earthobservatory.nasa.gov/images/79765/night-lights-2012-map> (oggi <https://science.nasa.gov/earth/earth-observatory/night-lights-2012-map-79765>) | 13500 x 6750 px (3 km), equirettangolare; dati Suomi NPP VIIRS |

Le dimensioni di Black Marble sono quelle della pagina NASA (verificate); quelle di Blue Marble sono le dichiarate dalla NASA per la versione a 500 m e non sono state riverificate.

## Come sono state ridotte

Tutte e due a 8192 x 4096 px (rapporto 2:1, quello della mappa del programma) con filtro Lanczos e salvate in JPEG. L'interfaccia le decodifica una volta sola e le tiene anche ridotte a 4096, 2048 e 1024 px di larghezza (ottenute per dimezzamenti) per disegnare alla risoluzione che serve allo zoom.

## Licenza e credito

Le immagini della NASA Earth Observatory si possono usare liberamente, citando la fonte. Il credito e' nei README (sei lingue), in `THIRD-PARTY-NOTICES.md` e nella Guida del programma: «Mappe della Terra: NASA Earth Observatory (Blue Marble Next Generation, Black Marble)». Non indicano un'approvazione da parte della NASA.

## Come si usano nel programma

- Impostazioni › Modi › Satelliti › Mappa: «Terra (satellite NASA)» (predefinita se non si e' scelto nulla e non c'e' una mappa «Contour HiRes»), «Mappa semplice (vettoriale)» e le immagini aggiunte dall'utente. I nomi riservati salvati nell'impostazione `map` sono `:terra` e `:aurora` (la vecchia mappa vettoriale, quindi chi l'aveva scelta la tiene); il core li tratta come un nome qualsiasi, senza modifiche.
- La notte e' la foto delle luci mascherata dall'altezza del Sole (`nightAlpha`), con il crepuscolo come passaggio morbido; con la mappa semplice o una dell'utente resta il velo blu.
- «Confini degli Stati sulla foto» (spento di base, preferenza del browser) disegna i confini sottili di Natural Earth sopra la Terra.
