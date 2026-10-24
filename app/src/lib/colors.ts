/** Le stesse categorie di colore di JTDX (piu' la risposta automatica di Aurora). */
export const COLOR_KEYS = [
  'cq',
  'my_call',
  'tx_msg',
  'new_dxcc',
  'new_dxcc_band',
  'new_zone',
  'new_zone_band',
  'new_itu',
  'new_itu_band',
  'new_grid',
  'new_grid_band',
  'new_px',
  'new_px_band',
  'new_call',
  'new_call_band',
  'standard_call',
  'worked_call',
  'auto',
] as const;
export type ColorKey = (typeof COLOR_KEYS)[number];
export type TagColors = Record<ColorKey, string>;

/** Chiave di traduzione del nome di ogni tipo di messaggio. */
export function kindKey(key: ColorKey): `kind.${ColorKey}` {
  return `kind.${key}`;
}

const KEY = 'aurora-colors';
const HEX = /^#[0-9a-fA-F]{6}$/;

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Valori di partenza presi dal tema scuro di JTDX 2.2.159. */
export function defaultColors(): TagColors {
  return {
    cq: '#e6dcd2',
    my_call: '#bf0000',
    tx_msg: '#aeae00',
    new_dxcc: '#800080',
    new_dxcc_band: '#904090',
    new_zone: '#804000',
    new_zone_band: '#806040',
    new_itu: '#507000',
    new_itu_band: '#809060',
    new_grid: '#006060',
    new_grid_band: '#409090',
    new_px: '#006000',
    new_px_band: '#10a050',
    new_call: '#606000',
    new_call_band: '#a0a030',
    standard_call: '#96aca2',
    worked_call: '#00bf00',
    auto: '#3fd6c8',
  };
}

export function readColors(storage: ReadableStorage): TagColors {
  const colors = defaultColors();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return colors;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null) return colors;
    for (const key of COLOR_KEYS) {
      const value = (parsed as Record<string, unknown>)[key];
      if (typeof value === 'string' && HEX.test(value)) colors[key] = value.toLowerCase();
    }
  } catch {
    // archivio assente o rovinato: restano i colori predefiniti
  }
  return colors;
}

export function saveColors(storage: WritableStorage, colors: TagColors): void {
  try {
    storage?.setItem(KEY, JSON.stringify(colors));
  } catch {
    // senza archivio i colori non vengono ricordati, ma l'interfaccia funziona lo stesso
  }
}

/** Testo nero o bianco, a seconda di quale si legge meglio sullo sfondo. */
export function inkFor(hex: string): string {
  const n = Number.parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  return luminance > 0.35 ? '#000000' : '#ffffff';
}

export function applyColors(root: HTMLElement, colors: TagColors): void {
  for (const key of COLOR_KEYS) {
    const name = `--tag-${key.replaceAll('_', '-')}`;
    root.style.setProperty(name, colors[key]);
    root.style.setProperty(`${name}-ink`, inkFor(colors[key]));
  }
}
