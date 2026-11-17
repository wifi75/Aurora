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
export const CONTEST_TEXT_IDS = ['rx', 'tx', 'wf', 'radar'] as const;
const STATIC_IDS: readonly string[] = [...CONTEST_BASE_IDS, ...CONTEST_TEXT_IDS];

export const multCardId = (tipo: string) => `mult:${tipo}`;
export const isMultCard = (id: string) => id.startsWith('mult:');
export const multCardTipo = (id: string) => id.slice('mult:'.length);

/** Chiave di memoria della disposizione di un contest in una vista. */
export const contestLayoutKey = (contestId: string, view: ContestView) => `aurora.contest.layout.${contestId}.${view}`;

/** Misure minime (colonne x righe): molto piccole, cosi' ognuno puo' stringere le schede a suo gusto (anche in pagine piccole); il contenuto scorre dentro la scheda. */
export const CONTEST_MINS: Record<string, Pick<DashSpec, 'minW' | 'minH'>> = {
  entry: { minW: 2, minH: 2 },
  log: { minW: 2, minH: 3 },
  cluster: { minW: 2, minH: 3 },
  rate: { minW: 2, minH: 3 },
  stats: { minW: 2, minH: 3 },
  rx: { minW: 2, minH: 2 },
  tx: { minW: 2, minH: 2 },
  wf: { minW: 2, minH: 3 },
  radar: { minW: 2, minH: 3 },
  mult: { minW: 2, minH: 3 },
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
  // `tipi` chiama «dominio» il moltiplicatore sullo scambio che la griglia e l'elenco dei mancanti chiamano «dom»: stessa scheda, non due
  const types = [...s.tipi.map((x) => (x === 'dominio' ? 'dom' : x)), ...s.griglia.map((g) => g.tipo), ...s.mancanti.map((m) => m.tipo)].filter((x, i, all) => all.indexOf(x) === i);
  return types.map((tipo) => ({ tipo, total: s.mancanti.find((m) => m.tipo === tipo)?.totali ?? null }));
}

/** Le schede di una vista, nell'ordine di sempre. */
export function contestCardIds(view: ContestView, mults: readonly MultCardInfo[]): string[] {
  return [...CONTEST_BASE_IDS, ...(view === 'text' ? CONTEST_TEXT_IDS : []), ...mults.map((m) => multCardId(m.tipo))];
}

const item = (id: string, x: number, y: number, w: number, h: number): DashItem => ({ id, x, y, w, h, userH: true });
/** L'inserimento QSO: l'altezza segue il contenuto (si allunga da sola se compaiono avvisi), salvo che l'utente non la scelga con l'angolo. */
const autoItem = (id: string, x: number, y: number, w: number, h: number): DashItem => ({ id, x, y, w, h });

/**
 * La disposizione di partenza. Vista `ssb`: a sinistra inserimento (5 colonne) e log, al centro DX Cluster e statistiche, a destra il ritmo; sotto,
 * le schede dei moltiplicatori a due a due (6 colonne ciascuna). Vista `text`: la ricezione, la trasmissione e la cascata prendono la riga in alto e
 * il resto scende sotto.
 */
export function defaultContestLayout(view: ContestView, mults: readonly MultCardInfo[]): DashItem[] {
  return view === 'ssb' ? defaultSsbLayout(mults) : defaultTextLayout(mults);
}

/**
 * Disposizione di partenza dei contest in CW, RTTY e PSK (la configurazione standard scelta dal proprietario, spazio tra le schede di 6 px): quattro
 * colonne da 3. Log a sinistra; ricezione sopra trasmissione; inserimento QSO, Paesi DXCC, statistiche e ritmo; Aurora BIGScan sopra il DX Cluster;
 * sotto il log e la ricezione la cascata e poi i moltiplicatori dello scambio.
 */
function defaultTextLayout(mults: readonly MultCardInfo[]): DashItem[] {
  const out: DashItem[] = [
    item('log', 0, 0, 3, 14),
    item('rx', 3, 0, 3, 11),
    item('tx', 3, 11, 3, 3),
    autoItem('entry', 6, 0, 3, 3),
    item('stats', 6, 10, 3, 9),
    item('rate', 6, 19, 3, 5),
    item('radar', 9, 0, 3, 9),
    item('cluster', 9, 9, 3, 15),
    item('wf', 0, 14, 6, 3),
  ];
  let y = 17;
  for (const m of mults) {
    if (m.tipo === 'dxcc') {
      out.push(item(multCardId(m.tipo), 6, 3, 3, 7));
      continue;
    }
    out.push(item(multCardId(m.tipo), 0, y, 6, 7));
    y += 7;
  }
  return out;
}

/**
 * Disposizione di partenza di tutti i contest in SSB (la configurazione standard scelta dal proprietario, spazio tra le schede di 6 px): a sinistra il log
 * (3 colonne), poi inserimento QSO sopra i Paesi DXCC (4 colonne), il DX Cluster sopra le statistiche (3 colonne), a destra il ritmo (2 colonne);
 * in fondo, a tutta la larghezza di log e inserimento, i moltiplicatori dello scambio (uno sopra l'altro se sono piu' d'uno).
 */
function defaultSsbLayout(mults: readonly MultCardInfo[]): DashItem[] {
  const out: DashItem[] = [item('log', 0, 0, 3, 13), autoItem('entry', 3, 0, 4, 5), item('cluster', 7, 0, 3, 9), item('rate', 10, 0, 2, 9), item('stats', 7, 9, 3, 10)];
  let y = 13;
  for (const m of mults) {
    if (m.tipo === 'dxcc') {
      out.push(item(multCardId(m.tipo), 3, 5, 4, 8));
      continue;
    }
    const h = (m.total ?? 0) > MULT_BIG ? 7 : 6;
    out.push(item(multCardId(m.tipo), 0, y, 7, h));
    y += h;
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
