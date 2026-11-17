import { beforeEach, describe, expect, it } from 'vitest';
import { normalize, overlaps, type DashItem, type DashSpecs } from './dashGrid';
import {
  CONTEST_LAYOUT_VERSION,
  MULT_BIG,
  contestCardIds,
  contestDefaultWidths,
  contestLayoutKey,
  contestLayoutRules,
  defaultContestLayout,
  loadContestLayout,
  mergeContestLayout,
  minsOf,
  multCardId,
  multCardsOf,
  saveContestLayout,
  type ContestView,
  type MultCardInfo,
} from './contestLayout';
import { parseDashLayout, serializeDashLayout } from './dashLayout';

const dom: MultCardInfo = { tipo: 'dom', total: 60 };
const dxcc: MultCardInfo = { tipo: 'dxcc', total: 340 };
const mults: MultCardInfo[] = [dom, dxcc];
const specsOf = (items: DashItem[]): DashSpecs => Object.fromEntries(items.map((i) => [i.id, { ...minsOf(i.id) }]));
const VIEWS: ContestView[] = ['ssb', 'text'];
const noOverlap = (d: DashItem[]) => d.every((a, i) => d.every((b, j) => i === j || !overlaps(a, b)));
const plain = (d: DashItem[]) => d.map(({ id, x, y, w, h }) => ({ id, x, y, w, h }));

describe('disposizione di partenza del contest', () => {
  it.each(VIEWS)('vista %s: ogni scheda una volta sola, minimi rispettati, niente sovrapposizioni, dentro le 12 colonne', (view) => {
    const d = defaultContestLayout(view, mults);
    expect(d.map((i) => i.id).sort()).toEqual(contestCardIds(view, mults).sort());
    for (const i of d) {
      const m = minsOf(i.id);
      expect(i.w).toBeGreaterThanOrEqual(m.minW);
      expect(i.h).toBeGreaterThanOrEqual(m.minH);
      expect(i.x + i.w).toBeLessThanOrEqual(12);
      expect(i.userH).toBe(true);
    }
    expect(noOverlap(d)).toBe(true);
    // la compattazione non sposta nulla: la partenza e' gia' sistemata
    expect(plain(normalize(d, specsOf(d)))).toEqual(plain(d));
  });

  it('nessuna scheda parte schiacciata: i moltiplicatori sono grandi e quelli con molti valori piu alti', () => {
    const d = defaultContestLayout('ssb', mults);
    const a = d.find((i) => i.id === multCardId('dom'));
    const b = d.find((i) => i.id === multCardId('dxcc'));
    expect(a?.w).toBe(6);
    expect(a?.h).toBeGreaterThanOrEqual(8);
    expect(340).toBeGreaterThan(MULT_BIG);
    expect(b?.h).toBeGreaterThan(a?.h ?? 0);
    expect(d.find((i) => i.id === 'entry')?.w).toBeGreaterThanOrEqual(5);
    expect(d.find((i) => i.id === 'stats')?.h).toBeGreaterThanOrEqual(8);
  });

  it('vista SSB senza ricezione, trasmissione e cascata; vista testo con le tre schede in piu', () => {
    expect(contestCardIds('ssb', [])).not.toContain('rx');
    expect(contestCardIds('text', [])).toEqual(expect.arrayContaining(['rx', 'tx', 'wf']));
  });

  it('i Paesi DXCC (elenco aperto, senza totale) partono alti come gli elenchi lunghi', () => {
    const d = defaultContestLayout('ssb', [{ tipo: 'dxcc', total: null }]);
    expect(d.find((i) => i.id === 'mult:dxcc')?.h).toBeGreaterThan(9);
  });

  it('piu tipi di moltiplicatore vanno a due a due sotto, senza urtarsi', () => {
    const many: MultCardInfo[] = ['a', 'b', 'c', 'd', 'e'].map((tipo) => ({ tipo, total: 30 }));
    const d = defaultContestLayout('ssb', many);
    expect(noOverlap(d)).toBe(true);
    expect(d.filter((i) => i.id.startsWith('mult:')).map((i) => i.x)).toEqual([0, 6, 0, 6, 0]);
    expect(Object.keys(contestDefaultWidths('ssb', many))).toHaveLength(d.length);
  });

  it('le schede dei moltiplicatori vengono dai tipi del punteggio, senza doppioni, con i totali dell elenco chiuso', () => {
    const s = { tipi: ['dom', 'dxcc'], griglia: [{ tipo: 'dom' }, { tipo: 'cq' }], mancanti: [{ tipo: 'dom', totali: 60 }] };
    expect(multCardsOf(s)).toEqual([
      { tipo: 'dom', total: 60 },
      { tipo: 'dxcc', total: null },
      { tipo: 'cq', total: null },
    ]);
  });
});

describe('salvataggio per contest e per vista', () => {
  beforeEach(() => window.localStorage.clear());

  it('la chiave e una per contest e per vista', () => {
    expect(contestLayoutKey('arrl10', 'ssb')).toBe('aurora.contest.layout.arrl10.ssb');
    expect(contestLayoutKey('arrl10', 'text')).not.toBe(contestLayoutKey('arrl10', 'ssb'));
    expect(contestLayoutKey('cqww', 'ssb')).not.toBe(contestLayoutKey('arrl10', 'ssb'));
  });

  it('senza nulla di salvato vale la partenza, sbloccata', () => {
    const l = loadContestLayout('arrl10', 'ssb', mults);
    expect(l.locked).toBe(false);
    expect(l.items).toEqual(defaultContestLayout('ssb', mults));
  });

  it('salva e rilegge posizione, misura e blocco; un altro contest o un altra vista non li vedono', () => {
    const items = defaultContestLayout('ssb', mults).map((i) => (i.id === 'log' ? { ...i, x: 7, w: 5, h: 12 } : i));
    saveContestLayout('arrl10', 'ssb', { items, locked: true });
    const back = loadContestLayout('arrl10', 'ssb', mults);
    expect(back.locked).toBe(true);
    expect(back.items.find((i) => i.id === 'log')).toMatchObject({ x: 7, w: 5, h: 12 });
    expect(loadContestLayout('cqww', 'ssb', mults).items.find((i) => i.id === 'log')).toMatchObject({ x: 0, w: 5 });
    expect(loadContestLayout('arrl10', 'text', mults).locked).toBe(false);
  });

  it('scrive sempre l altezza e la versione', () => {
    const raw = serializeDashLayout({ items: defaultContestLayout('ssb', []), locked: false }, { version: CONTEST_LAYOUT_VERSION, alwaysHeight: true });
    const j = JSON.parse(raw) as { v: number; items: { h?: number }[] };
    expect(j.v).toBe(CONTEST_LAYOUT_VERSION);
    expect(j.items.every((i) => typeof i.h === 'number')).toBe(true);
  });

  it('scarta una versione diversa e i dati rovinati; le schede sconosciute o sbagliate restano fuori, le buone no', () => {
    const rules = contestLayoutRules('ssb');
    const good = JSON.parse(serializeDashLayout({ items: defaultContestLayout('ssb', mults), locked: true }, { version: CONTEST_LAYOUT_VERSION, alwaysHeight: true })) as { items: Record<string, unknown>[] };
    expect(parseDashLayout(JSON.stringify({ ...good, v: CONTEST_LAYOUT_VERSION + 1 }), rules)).toBeNull();
    expect(parseDashLayout('{rotto', rules)).toBeNull();
    expect(parseDashLayout(null, rules)).toBeNull();
    const junk = [{ id: 'zz', x: 0, y: 0, w: 4, h: 4 }, { id: 'log', x: 99, y: 0, w: 5, h: 5 }, { id: 'rx', x: 0, y: 0, w: 4, h: 4 }];
    const p = parseDashLayout(JSON.stringify({ ...good, items: [...good.items, ...junk] }), rules);
    // sconosciuta, doppia fuori dalla griglia, ricezione (non della vista SSB): fuori
    expect(p?.items.some((i) => i.id === 'zz' || i.id === 'rx')).toBe(false);
    expect(p?.items.filter((i) => i.id === 'log')).toHaveLength(1);
    expect(p?.items.find((i) => i.id === 'log')?.x).toBe(0);
    expect(p?.locked).toBe(true);
  });

  it('una scheda nuova (un tipo di moltiplicatore in piu) prende la posizione di partenza e non cancella le altre', () => {
    saveContestLayout('arrl10', 'ssb', { items: defaultContestLayout('ssb', [dom]).map((i) => (i.id === 'entry' ? { ...i, w: 6 } : i)), locked: false });
    const back = loadContestLayout('arrl10', 'ssb', mults);
    expect(back.items.find((i) => i.id === 'entry')?.w).toBe(6);
    expect(back.items.find((i) => i.id === multCardId('dxcc'))).toBeTruthy();
  });

  it('le schede salvate che ora non servono si tengono da parte', () => {
    const merged = mergeContestLayout(defaultContestLayout('ssb', mults), 'ssb', [dom]);
    expect(merged.some((i) => i.id === multCardId('dxcc'))).toBe(true);
    const wanted = contestCardIds('ssb', [dom]);
    expect(merged.filter((i) => wanted.includes(i.id))).toHaveLength(wanted.length);
  });
});
