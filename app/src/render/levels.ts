/**
 * Nero, guadagno e contrasto della cascata, come i cursori di Decodium: agiscono nel disegno (non sui dati), quindi cambiano subito
 * tutta la storia gia' scorsa. Gli stessi tre passaggi stanno negli shader di WebGPU e WebGL2.
 */
export interface Levels {
  /** 0-0.6: sotto questo livello (frazione della scala) il fondo diventa nero. */
  black: number;
  /** 0.4-2.5: curva gamma, sopra 1 schiarisce i segnali deboli. */
  gain: number;
  /** 0.5-2.5: sposta i valori dal centro, piu' netti i segnali. */
  contrast: number;
}

export const DEFAULT_LEVELS: Levels = { black: 0, gain: 1, contrast: 1 };

export const LEVEL_KEYS = { black: 'aurora-wf-black', gain: 'aurora-wf-gain', contrast: 'aurora-wf-contrast' } as const;

const LIMITS: Record<keyof Levels, [number, number]> = { black: [0, 0.6], gain: [0.4, 2.5], contrast: [0.5, 2.5] };

export function clampLevel(name: keyof Levels, value: number): number {
  const [lo, hi] = LIMITS[name];
  return Number.isFinite(value) ? Math.min(hi, Math.max(lo, value)) : DEFAULT_LEVELS[name];
}

/** Legge i tre livelli dalle preferenze; i valori mancanti o fuori scala tornano a quelli di base. */
export function readLevels(read: (key: string) => string | null): Levels {
  const get = (name: keyof Levels) => {
    const raw = read(LEVEL_KEYS[name]);
    return raw === null || raw === '' ? DEFAULT_LEVELS[name] : clampLevel(name, Number(raw));
  };
  return { black: get('black'), gain: get('gain'), contrast: get('contrast') };
}

/** Lo stesso calcolo degli shader, per le prove e per chi disegna sul processore. */
export function toneMap(v: number, l: Levels): number {
  let x = Math.min(1, Math.max(0, (v - l.black) / Math.max(1 - l.black, 0.05)));
  x = Math.pow(x, 1 / l.gain);
  return Math.min(1, Math.max(0, (x - 0.5) * l.contrast + 0.5));
}
