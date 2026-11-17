import { DASH_COLS, type DashItem, type DashSpec } from './dashGrid';

/** Disposizione salvata di una griglia di schede: le schede e se la disposizione e' bloccata. */
export interface StoredLayout {
  items: DashItem[];
  locked: boolean;
}

/** Come si controlla e si scrive la disposizione salvata di una griglia di schede (Satelliti, contest). */
export interface DashLayoutRules {
  /** Versione dello schema salvato: se cambia, la disposizione salvata si scarta. */
  version: number;
  /** La scheda e' una di quelle note? Le altre non si accettano. */
  accepts(id: string): boolean;
  /** Misure minime (colonne x righe) della scheda. */
  mins(id: string): Pick<DashSpec, 'minW' | 'minH'>;
  /** Colonne massime della scheda. */
  maxCols(id: string): number;
  /**
   * Con un elenco: la disposizione vale solo se ha tutte e sole quelle schede (Satelliti). Senza: le schede sconosciute o rovinate
   * si lasciano da parte e quelle che mancano si aggiungono dopo (contest, dove le schede dei moltiplicatori dipendono dal contest).
   */
  exact?: readonly string[];
  /** L'altezza si salva sempre (contest: ogni scheda parte della misura scelta) e non segue il contenuto. */
  alwaysHeight?: boolean;
}

const isInt = (n: unknown): n is number => typeof n === 'number' && Number.isInteger(n);

/** Controlla una scheda salvata; null se non e' valida. */
function parseItem(row: unknown, rules: DashLayoutRules): DashItem | null {
  if (typeof row !== 'object' || row === null) return null;
  const r = row as Record<string, unknown>;
  const id = r.id;
  if (typeof id !== 'string' || !rules.accepts(id)) return null;
  const { x, y, w, h } = r;
  // `h` e' facoltativa: senza, l'altezza segue il contenuto
  if (!isInt(x) || !isInt(y) || !isInt(w) || (h !== undefined && !isInt(h))) return null;
  const min = rules.mins(id);
  if (x < 0 || y < 0 || w < min.minW || w > Math.min(rules.maxCols(id), DASH_COLS) || (h !== undefined && h < min.minH) || x + w > DASH_COLS || y > 10_000 || (h !== undefined && h > 10_000)) return null;
  if (h === undefined) return rules.alwaysHeight ? null : { id, x, y, w, h: min.minH };
  return { id, x, y, w, h, userH: true };
}

/** Legge e controlla la disposizione salvata; null se manca, e' corrotta, di un'altra versione o (con `exact`) non ha tutte e sole le schede note. */
export function parseDashLayout(raw: string | null, rules: DashLayoutRules): StoredLayout | null {
  if (!raw) return null;
  try {
    const data: unknown = JSON.parse(raw);
    if (typeof data !== 'object' || data === null) return null;
    const rec = data as Record<string, unknown>;
    if (rec.v !== rules.version || !Array.isArray(rec.items)) return null;
    const exact = rules.exact;
    if (exact && rec.items.length !== exact.length) return null;
    const seen = new Set<string>();
    const items: DashItem[] = [];
    for (const row of rec.items as unknown[]) {
      const it = parseItem(row, rules);
      if (!it || seen.has(it.id)) {
        if (exact) return null;
        continue;
      }
      seen.add(it.id);
      items.push(it);
    }
    return { items, locked: rec.locked === true };
  } catch {
    return null;
  }
}

export function serializeDashLayout(l: StoredLayout, rules: Pick<DashLayoutRules, 'version' | 'alwaysHeight'>): string {
  return JSON.stringify({
    v: rules.version,
    locked: l.locked,
    items: l.items.map((i) => (i.userH || rules.alwaysHeight ? { id: i.id, x: i.x, y: i.y, w: i.w, h: i.h } : { id: i.id, x: i.x, y: i.y, w: i.w })),
  });
}

/** La disposizione salvata nel browser sotto `key`, o null (manca, rovinata, memoria non disponibile). */
export function loadDashLayout(key: string, rules: DashLayoutRules): StoredLayout | null {
  try {
    return parseDashLayout(window.localStorage.getItem(key), rules);
  } catch {
    return null;
  }
}

export function saveDashLayout(key: string, l: StoredLayout, rules: Pick<DashLayoutRules, 'version' | 'alwaysHeight'>): void {
  try {
    window.localStorage.setItem(key, serializeDashLayout(l, rules));
  } catch {
    /* memoria del browser non disponibile */
  }
}
