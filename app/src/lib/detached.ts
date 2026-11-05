import { useEffect, useState } from 'react';
import type { PanelId } from './layout';
import { browserStorage } from './theme';
import { arrange, freeCell, type Cell } from '../components/GridBoard';

/** Cosa si puo' portare in una finestra a parte (secondo schermo): i pannelli della Stazione, la Mappa e la Propagazione. */
export type ItemId = PanelId | 'map' | 'propagation';

/** Una finestra a parte: uno o piu' elementi affiancati (in riga) o uno sopra l'altro (in colonna), con le loro proporzioni. */
export interface Group {
  id: string;
  items: ItemId[];
  dir: 'row' | 'col';
  /** Proporzioni degli elementi (somma 1). */
  sizes: number[];
  /** La finestra e' aperta adesso; chiusa dall'utente con la X, i suoi elementi tornano nella finestra principale. */
  open: boolean;
  /** `grid`: dashboard a blocchi (trascinabili e ridimensionabili); altrimenti in riga o in colonna con il divisore. */
  mode?: 'split' | 'grid';
  /** Posizione e misura di ogni blocco della dashboard. */
  cells?: Record<string, Cell>;
}

/** L'identificativo della dashboard a blocchi (una sola). */
export const DASHBOARD = 'dash';

const KEY = 'aurora-finestre';
const EVENT = 'aurora-finestre';

export function readGroups(): Group[] {
  try {
    const raw = browserStorage()?.getItem(KEY);
    const list = raw ? (JSON.parse(raw) as Group[]) : [];
    return Array.isArray(list) ? list.filter((g) => g && typeof g.id === 'string' && Array.isArray(g.items) && g.items.length > 0) : [];
  } catch {
    return [];
  }
}

function writeGroups(groups: Group[]): void {
  try {
    browserStorage()?.setItem(KEY, JSON.stringify(groups));
  } catch {
    // senza archivio le finestre a parte funzionano lo stesso, ma non si ricordano
  }
  window.dispatchEvent(new Event(EVENT));
}

/** Le finestre a parte, aggiornate anche quando le cambia un'altra finestra (evento `storage` dello stesso Aurora). */
export function useGroups(): Group[] {
  const [groups, setGroups] = useState(readGroups);
  useEffect(() => {
    const update = () => setGroups(readGroups());
    window.addEventListener('storage', update);
    window.addEventListener(EVENT, update);
    return () => {
      window.removeEventListener('storage', update);
      window.removeEventListener(EVENT, update);
    };
  }, []);
  return groups;
}

/** Gli elementi che stanno in una finestra a parte aperta: nella finestra principale non si mostrano (niente doppioni). */
export function detachedItems(groups: readonly Group[]): ItemId[] {
  return groups.filter((g) => g.open).flatMap((g) => g.items);
}

function evenSizes(n: number): number[] {
  return Array.from({ length: n }, () => 1 / n);
}

/** Apre (o porta davanti) la finestra di un gruppo: nel programma per Windows la crea il guscio, sul secondo schermo. */
export function openGroupWindow(id: string): void {
  // cosa contiene viaggia anche nell'indirizzo: se la finestra nuova non trova ancora il gruppo nell'archivio, lo ricostruisce da qui
  const g = readGroups().find((x) => x.id === id);
  const q = g ? `?items=${g.items.join(',')}${g.mode === 'grid' ? '&mode=grid' : ''}` : '';
  window.open(`${location.origin}/${q}#/finestra/${id}`, `aurora-${id}`, 'width=1000,height=700');
}

const ITEM_IDS: readonly string[] = ['waterfall', 'station', 'tx', 'cq', 'all', 'log', 'cluster', 'map', 'propagation'];

/** Dalla finestra a parte: il gruppo descritto nell'indirizzo (`?items=...`), da rimettere nell'archivio se manca. */
export function groupFromUrl(id: string): Group | null {
  const q = new URLSearchParams(location.search);
  const items = (q.get('items') ?? '').split(',').filter((i) => ITEM_IDS.includes(i)) as ItemId[];
  if (items.length === 0) return null;
  return { id, items, dir: 'row', sizes: evenSizes(items.length), open: true, mode: q.get('mode') === 'grid' ? 'grid' : 'split' };
}

/** Rimette un gruppo nell'archivio (se non c'e' gia'). */
export function restoreGroup(group: Group): void {
  const groups = readGroups();
  if (groups.some((g) => g.id === group.id)) return;
  writeGroups([...groups.map((g) => ({ ...g, items: g.items.filter((i) => !group.items.includes(i)) })).filter((g) => g.items.length > 0), group]);
}

/** Porta un elemento in una finestra a parte: nuova, o accanto a quelli di una finestra gia' aperta. */
export function detach(item: ItemId, into?: string): void {
  const groups = readGroups().map((g) => ({ ...g, items: g.items.filter((i) => i !== item) })).filter((g) => g.items.length > 0);
  let id = into;
  const target = into ? groups.find((g) => g.id === into) : undefined;
  if (target) {
    target.items.push(item);
    target.sizes = evenSizes(target.items.length);
    target.open = true;
    if (target.mode === 'grid') target.cells = arrange({ ...(target.cells ?? {}), [item]: freeCell(target.cells ?? {}) });
  } else if (into === DASHBOARD) {
    groups.push({ id: DASHBOARD, items: [item], dir: 'row', sizes: [1], open: true, mode: 'grid', cells: { [item]: { x: 0, y: 0, w: 12, h: 24 } } });
  } else {
    let n = 1;
    while (groups.some((g) => g.id === `f${n}`)) n += 1;
    id = `f${n}`;
    groups.push({ id, items: [item], dir: 'row', sizes: [1], open: true });
  }
  writeGroups(groups);
  if (id) openGroupWindow(id);
}

/** Rimette un elemento nella finestra principale; la finestra a parte rimasta vuota si chiude da sola. */
export function attach(item: ItemId): void {
  const groups = readGroups()
    .map((g) => {
      const items = g.items.filter((i) => i !== item);
      const cells = g.cells ? Object.fromEntries(Object.entries(g.cells).filter(([k]) => k !== item)) : undefined;
      return { ...g, items, cells, sizes: items.length === g.items.length ? g.sizes : evenSizes(items.length) };
    })
    .filter((g) => g.items.length > 0);
  writeGroups(groups);
}

export function updateGroup(id: string, change: Partial<Group>): void {
  writeGroups(readGroups().map((g) => (g.id === id ? { ...g, ...change } : g)));
}

/** La finestra di un gruppo e' stata chiusa (X): segnata chiusa; la finestra principale, se e' viva, poi rimette tutto al suo posto. */
export function markClosed(id: string): void {
  updateGroup(id, { open: false });
}

/** Dalla finestra principale: le finestre chiuse con la X tornano dentro; le altre si riaprono all'avvio. */
export function dropClosed(): void {
  const groups = readGroups();
  if (groups.some((g) => !g.open)) writeGroups(groups.filter((g) => g.open));
}

/** Tutto torna nella finestra principale e le finestre a parte si chiudono (ognuna si chiude da sola quando resta vuota). */
export function attachAll(): void {
  writeGroups([]);
}

export function reopenAll(): void {
  const groups = readGroups();
  if (groups.length === 0) return;
  writeGroups(groups.map((g) => ({ ...g, open: true })));
  for (const g of groups) openGroupWindow(g.id);
}
