import { browserStorage } from './theme';

export interface MapOptions {
  /** Sfondo con terreno e confini (tessere da internet). */
  tiles: boolean;
  /** Zona di notte. */
  night: boolean;
}

const KEY = 'aurora-map-options';

export function readMapOptions(storage: Pick<Storage, 'getItem'> | null = browserStorage()): MapOptions {
  const defaults: MapOptions = { tiles: true, night: true };
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return defaults;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    return { tiles: typeof parsed.tiles === 'boolean' ? parsed.tiles : defaults.tiles, night: typeof parsed.night === 'boolean' ? parsed.night : defaults.night };
  } catch {
    return defaults;
  }
}

export function saveMapOptions(options: MapOptions, storage: Pick<Storage, 'setItem'> | null = browserStorage()): void {
  try {
    storage?.setItem(KEY, JSON.stringify(options));
  } catch {
    // senza archivio le scelte non si ricordano, la mappa funziona lo stesso
  }
}
