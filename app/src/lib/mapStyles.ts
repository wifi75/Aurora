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

const ESRI = 'https://server.arcgisonline.com/ArcGIS/rest/services';
const ESRI_ATTR = '© Esri';
const OSM_ATTR = '© OpenStreetMap contributors';

/** Gli sfondi tra cui scegliere (gli stessi servizi pubblici delle mappe dei programmi radioamatoriali). */
export const MAP_STYLES: readonly MapStyle[] = [
  { id: 'natgeo', name: 'National Geographic (Esri)', url: `${ESRI}/NatGeo_World_Map/MapServer/tile/{z}/{y}/{x}`, attribution: `${ESRI_ATTR} — National Geographic, Esri, DeLorme, NAVTEQ, UNEP-WCMC, USGS, NASA, ESA, METI, NRCAN, GEBCO, NOAA, iPC`, maxNativeZoom: 10 },
  { id: 'satellite', name: 'Satellite (Esri)', url: `${ESRI}/World_Imagery/MapServer/tile/{z}/{y}/{x}`, attribution: `${ESRI_ATTR} — Maxar, Earthstar Geographics`, maxNativeZoom: 10, dark: true },
  { id: 'physical', name: 'Physical (Esri)', url: `${ESRI}/World_Physical_Map/MapServer/tile/{z}/{y}/{x}`, attribution: `${ESRI_ATTR} — US National Park Service`, maxNativeZoom: 8 },
  { id: 'relief', name: 'Shaded Relief (Esri)', url: `${ESRI}/World_Shaded_Relief/MapServer/tile/{z}/{y}/{x}`, attribution: ESRI_ATTR, maxNativeZoom: 10 },
  { id: 'oceans', name: 'Oceans (Esri)', url: `${ESRI}/Ocean/World_Ocean_Base/MapServer/tile/{z}/{y}/{x}`, attribution: `${ESRI_ATTR} — GEBCO, NOAA, National Geographic`, maxNativeZoom: 10 },
  { id: 'topo', name: 'Topographic (Esri)', url: `${ESRI}/World_Topo_Map/MapServer/tile/{z}/{y}/{x}`, attribution: ESRI_ATTR, maxNativeZoom: 10 },
  { id: 'street', name: 'Street (Esri)', url: `${ESRI}/World_Street_Map/MapServer/tile/{z}/{y}/{x}`, attribution: ESRI_ATTR, maxNativeZoom: 10 },
  { id: 'lightgray', name: 'Light Gray (Esri)', url: `${ESRI}/Canvas/World_Light_Gray_Base/MapServer/tile/{z}/{y}/{x}`, attribution: ESRI_ATTR, maxNativeZoom: 10 },
  { id: 'darkgray', name: 'Dark Gray (Esri)', url: `${ESRI}/Canvas/World_Dark_Gray_Base/MapServer/tile/{z}/{y}/{x}`, attribution: ESRI_ATTR, maxNativeZoom: 10, dark: true },
  { id: 'osm', name: 'Mapnik (OpenStreetMap)', url: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png', attribution: OSM_ATTR, maxNativeZoom: 10 },
  { id: 'osm-de', name: 'Mapnik Deutsch (OpenStreetMap)', url: 'https://tile.openstreetmap.de/{z}/{x}/{y}.png', attribution: OSM_ATTR, maxNativeZoom: 10 },
  { id: 'osm-fr', name: 'Mapnik français (OpenStreetMap)', url: 'https://{s}.tile.openstreetmap.fr/osmfr/{z}/{x}/{y}.png', attribution: `${OSM_ATTR} — OSM France`, maxNativeZoom: 10, subdomains: 'abc' },
  { id: 'humanitarian', name: 'Humanitarian (OpenStreetMap)', url: 'https://{s}.tile.openstreetmap.fr/hot/{z}/{x}/{y}.png', attribution: `${OSM_ATTR} — HOT`, maxNativeZoom: 10, subdomains: 'abc' },
  { id: 'citylights', name: 'City Lights (NASA)', url: 'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/VIIRS_CityLights_2012/default/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpg', attribution: 'NASA EOSDIS GIBS', maxNativeZoom: 8, dark: true },
  { id: 'bluemarble', name: 'Blue Marble (NASA)', url: 'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief_Bathymetry/default/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpeg', attribution: 'NASA EOSDIS GIBS', maxNativeZoom: 8, dark: true },
];

export const DEFAULT_STYLE = 'natgeo';

export function mapStyle(id: string | undefined): MapStyle {
  return MAP_STYLES.find((s) => s.id === id) ?? MAP_STYLES[0]!;
}
