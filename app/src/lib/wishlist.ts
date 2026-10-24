/** La lista dei desiderati come la salva il core (`core/crates/wishlist`). */
export interface Wishlist {
  enabled: boolean;
  calls: string[];
  prefixes: string[];
  countries: string[];
  grids: string[];
  bands: string[];
  cooldown_min: number;
  from_decodes: boolean;
  from_cluster: boolean;
  notify_url: string;
}

export const EMPTY_WISHLIST: Wishlist = {
  enabled: false,
  calls: [],
  prefixes: [],
  countries: [],
  grids: [],
  bands: [],
  cooldown_min: 15,
  from_decodes: true,
  from_cluster: true,
  notify_url: '',
};

export type WishReason = 'call' | 'prefix' | 'country' | 'grid';

/** Il nominativo vero di «K1ABC/P» o «F/K1ABC»: il pezzo più lungo che ha una cifra. */
export function baseCall(call: string): string {
  const parts = call.split('/').filter((p) => /\d/.test(p));
  if (parts.length === 0) return call;
  return parts.reduce((best, p) => (p.length > best.length ? p : best));
}

/**
 * La stazione è desiderata? Stessa regola del core: prima il nominativo, poi il prefisso, il paese, la griglia; con un filtro di banda,
 * una banda nota fuori dall'elenco esclude e una banda ignota no.
 */
export function wishReason(list: Wishlist, call: string, grid?: string | null, country?: string | null, band?: string | null): WishReason | null {
  if (!list.enabled) return null;
  if (list.bands.length > 0 && band && !list.bands.some((b) => b.toLowerCase() === band.toLowerCase())) return null;
  const c = call.trim().toUpperCase();
  if (c === '') return null;
  const base = baseCall(c);
  if (list.calls.some((x) => x === c || x === base)) return 'call';
  if (list.prefixes.some((p) => c.startsWith(p) || base.startsWith(p))) return 'prefix';
  if (country && list.countries.some((x) => x.toLowerCase() === country.toLowerCase())) return 'country';
  if (grid && grid.length >= 4 && list.grids.some((g) => g.toUpperCase() === grid.slice(0, 4).toUpperCase())) return 'grid';
  return null;
}

const GRID4 = /^[A-R]{2}\d{2}$/;
const CALL_LIKE = /^[A-Z0-9/]{3,15}$/;

/** Chi trasmette un messaggio e il suo locatore, se lo dice (come nel core). «RR73» ha la forma di un locatore ma è un saluto. */
export function senderOf(text: string): { call: string; grid: string | null } | null {
  const tokens = text.trim().toUpperCase().split(/\s+/).filter((t) => t !== '');
  if (tokens.length < 2) return null;
  const last = tokens[tokens.length - 1]!;
  if (tokens.length >= 3 && last !== 'RR73' && GRID4.test(last)) {
    const call = tokens[tokens.length - 2]!;
    return CALL_LIKE.test(call) && /\d/.test(call) ? { call, grid: last } : null;
  }
  let call: string | undefined;
  if (tokens[0] === 'CQ' || tokens[0] === 'QRZ' || tokens[0] === 'DE') {
    const t1 = tokens[1]!;
    call = t1.length <= 4 && !/\d/.test(t1) ? tokens[2] : t1;
  } else {
    call = tokens[1];
  }
  return call && CALL_LIKE.test(call) && /\d/.test(call) ? { call, grid: null } : null;
}

/** Le voci di un elenco scritto una per riga (o separato da virgole). */
export function parseLines(text: string): string[] {
  return text.split(/[\n,;]+/).map((s) => s.trim()).filter((s) => s !== '');
}
