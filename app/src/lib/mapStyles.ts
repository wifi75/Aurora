/** Uno sfondo della mappa: tessere pubbliche, con chi le fornisce. */
export interface MapStyle {
  id: string;
  name: string;
  url: string;
  attribution: string;
  /** Lo zoom piu' alto che il servizio disegna. */
  maxNativeZoom: number;
  subdomains?: string;
  /** Sfondo scuro: segni e scritte della mappa chiari. */
  dark?: boolean;
}


/** Gli sfondi: solo quelli della NASA (Blue Marble di giorno, City Lights di notte). */
export const MAP_STYLES: readonly MapStyle[] = [
  { id: 'bluemarble', name: 'Blue Marble (NASA)', url: 'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief_Bathymetry/default/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpeg', attribution: 'NASA EOSDIS GIBS', maxNativeZoom: 8, dark: true },
  { id: 'citylights', name: 'City Lights (NASA)', url: 'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/VIIRS_CityLights_2012/default/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpg', attribution: 'NASA EOSDIS GIBS', maxNativeZoom: 8, dark: true },
];

export const DEFAULT_STYLE = 'bluemarble';

export function mapStyle(id: string | undefined): MapStyle {
  return MAP_STYLES.find((s) => s.id === id) ?? MAP_STYLES[0]!;
}
