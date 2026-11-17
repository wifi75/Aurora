import { DASH_COLS, type DashItem, type DashSpec } from './dashGrid';
import { loadDashLayout, saveDashLayout, type DashLayoutRules, type StoredLayout } from './dashLayout';

/**
 * Disposizione delle schede della vista contest (SSB e modi testuali), sul modello dei Satelliti: griglia a 12 colonne, righe di 30 px,
 * una disposizione salvata per contest e per vista. Ogni scheda parte della misura che le serve (l'altezza e' sempre quella della
 * disposizione, il contenuto scorre dentro); le schede dei moltiplicatori sono una per tipo e dipendono dal contest.
 */

/** Versione dello schema salvato: se cambia, la disposizione salvata si scarta. */
export const CONTEST_LAYOUT_VERSION = 1;

/** Le viste: `ssb` (inserimento, log, cluster, punteggio) e `text` (CW, RTTY, PSK: in piu' ricezione, trasmissione e cascata). */
export type ContestView = 'ssb' | 'text';

export const CONTEST_BASE_IDS = ['entry', 'log', 'cluster', 'rate', 'stats'] as const;
export const CONTEST_TEXT_IDS = ['rx', 'tx', 'wf'] as const;
const STATIC_IDS: readonly string[] = [...CONTEST_BASE_IDS, ...CONTEST_TEXT_IDS];

export const multCardId = (tipo: string) => `mult:${tipo}`;
export const isMultCard = (id: string) => id.startsWith('mult:');
export const multCardTipo = (id: string) => id.slice('mult:'.length);

/** Chiave di memoria della disposizione di un contest in una vista. */
export const contestLayoutKey = (contestId: string, view: ContestView) => `aurora.contest.layout.${contestId}.${view}`;

/** Misure minime (colonne x righe): le piu' piccole che il contenuto sopporta; sotto, la scheda non si stringe. */
export const CONTEST_MINS: Record<string, Pick<DashSpec, 'minW' | 'minH'>> = {
  entry: { minW: 4, minH: 7 },
  log: { minW: 3, minH: 4 },
  cluster: { minW: 3, minH: 4 },
  rate: { minW: 2, minH: 5 },
  stats: { minW: 4, minH: 6 },
  rx: { minW: 3, minH: 3 },
  tx: { minW: 3, minH: 3 },
  wf: { minW: 4, minH: 4 },
  mult: { minW: 3, minH: 4 },
};

export const minsOf = (id: string): Pick<DashSpec, 'minW' | 'minH'> => CONTEST_MINS[isMultCard(id) ? 'mult' : id] ?? { minW: 2, minH: 3 };

/** Un tipo di moltiplicatore del contest (una scheda): `total` = i valori possibili se il core li da' (elenco chiuso). */
export interface MultCardInfo {
  tipo: string;
  total: number | null;
}

/** Sopra questo numero di valori la scheda dei moltiplicatori parte piu' alta (l'elenco scorre dentro). */
export const MULT_BIG = 80;

/** Le schede dei moltiplicatori di un punteggio: un tipo ciascuna, nell'ordine dei tipi del contest. */
export function multCardsOf(s: { tipi: readonly string[]; griglia: readonly { tipo: string }[]; mancanti: readonly { tipo: string; totali: number }[] }): MultCardInfo[] {
  const types = [...s.tipi, ...s.griglia.map((g) => g.tipo), ...s.mancanti.map((m) => m.tipo)].filter((x, i, all) => all.indexOf(x) === i);
  return types.map((tipo) => ({ tipo, total: s.mancanti.find((m) => m.tipo === tipo)?.totali ?? null }));
}

/** Le schede di una vista, nell'ordine di sempre. */
export function contestCardIds(view: ContestView, mults: readonly MultCardInfo[]): string[] {
  return [...CONTEST_BASE_IDS, ...(view === 'text' ? CONTEST_TEXT_IDS : []), ...mults.map((m) => multCardId(m.tipo))];
}

const item = (id: string, x: number, y: number, w: number, h: number): DashItem => ({ id, x, y, w, h, userH: true });

/**
 * La disposizione di partenza. Vista `ssb`: a sinistra inserimento (5 colonne) e log, al centro DX Cluster e statistiche, a destra il ritmo; sotto,
 * le schede dei moltiplicatori a due a due (6 colonne ciascuna). Vista `text`: la ricezione, la trasmissione e la cascata prendono la riga in alto e
 * il resto scende sotto.
 */
export function defaultContestLayout(view: ContestView, mults: readonly MultCardInfo[]): DashItem[] {
  const out: DashItem[] =
    view === 'ssb'
      ? [item('entry', 0, 0, 5, 9), item('log', 0, 9, 5, 9), item('cluster', 5, 0, 4, 9), item('rate', 9, 0, 3, 9), item('stats', 5, 9, 7, 9)]
      : [
          item('rx', 0, 0, 6, 6),
          item('tx', 0, 6, 6, 4),
          item('wf', 6, 0, 6, 10),
          item('entry', 0, 10, 5, 9),
          item('cluster', 5, 10, 4, 9),
          item('rate', 9, 10, 3, 9),
          item('log', 0, 19, 5, 9),
          item('stats', 5, 19, 7, 9),
        ];
  const top = Math.max(...out.map((i) => i.y + i.h));
  let x = 0;
  let y = top;
  let rowH = 0;
  for (const m of mults) {
    const h = m.tipo === 'dxcc' || (m.total ?? 0) > MULT_BIG ? 13 : 9;
    if (x + 6 > DASH_COLS) {
      x = 0;
      y += rowH;
      rowH = 0;
    }
    out.push(item(multCardId(m.tipo), x, y, 6, h));
    rowH = Math.max(rowH, h);
    x += 6;
  }
  return out;
}

/** Larghezza di partenza di ogni scheda (il doppio clic sul maniglione la ripristina). */
export const contestDefaultWidths = (view: ContestView, mults: readonly MultCardInfo[]): Record<string, number> => Object.fromEntries(defaultContestLayout(view, mults).map((i) => [i.id, i.w]));

const rulesOf = (view: ContestView): DashLayoutRules => ({
  version: CONTEST_LAYOUT_VERSION,
  accepts: (id) => isMultCard(id) || (STATIC_IDS.includes(id) && (view === 'text' || !(CONTEST_TEXT_IDS as readonly string[]).includes(id))),
  mins: minsOf,
  maxCols: () => DASH_COLS,
  alwaysHeight: true,
});

/**
 * Le schede che servono ora con le posizioni salvate; per quelle senza posizione salvata (un tipo di moltiplicatore nuovo, o la prima volta)
 * la posizione di partenza. Le salvate di schede che non ci sono piu' si tengono da parte (DashGrid non le disegna ma le conserva).
 */
export function mergeContestLayout(saved: readonly DashItem[], view: ContestView, mults: readonly MultCardInfo[]): DashItem[] {
  const defaults = defaultContestLayout(view, mults);
  const wanted = contestCardIds(view, mults);
  const byId = new Map(saved.map((i) => [i.id, i]));
  const kept = wanted.map((id) => byId.get(id) ?? defaults.find((d) => d.id === id)).filter((i): i is DashItem => i !== undefined);
  const absent = saved.filter((i) => !wanted.includes(i.id));
  return [...kept, ...absent];
}

/** La disposizione salvata del contest nella vista (con le posizioni di partenza dove manca), bloccata o no. */
export function loadContestLayout(contestId: string, view: ContestView, mults: readonly MultCardInfo[]): StoredLayout {
  const saved = loadDashLayout(contestLayoutKey(contestId, view), rulesOf(view));
  return { items: mergeContestLayout(saved?.items ?? [], view, mults), locked: saved?.locked ?? false };
}

export function saveContestLayout(contestId: string, view: ContestView, l: StoredLayout): void {
  saveDashLayout(contestLayoutKey(contestId, view), l, rulesOf(view));
}

export const contestLayoutRules = rulesOf;
