import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';

/** Un blocco della dashboard, in celle della griglia (12 colonne e 24 righe che riempiono la finestra). */
export interface Cell {
  x: number;
  y: number;
  w: number;
  h: number;
}

export const COLS = 12;
/** Righe della finestra intera: la griglia segue la finestra in larghezza e in altezza, cosi' i blocchi la riempiono sempre. */
export const ROWS = 24;
const GAP = 6;

function overlaps(a: Cell, b: Cell): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/**
 * Sistema i blocchi senza sovrapposizioni, come una dashboard: quello che si sta muovendo resta dov'e', gli altri scendono quanto basta
 * e poi risalgono il piu' possibile (nessun buco in alto).
 */
export function arrange(cells: Record<string, Cell>, fixed?: string): Record<string, Cell> {
  const ids = Object.keys(cells).sort((a, b) => (a === fixed ? -1 : b === fixed ? 1 : cells[a]!.y - cells[b]!.y || cells[a]!.x - cells[b]!.x));
  const placed: Record<string, Cell> = {};
  for (const id of ids) {
    const c = { ...cells[id]! };
    c.w = Math.max(2, Math.min(COLS, c.w));
    c.h = Math.max(3, c.h);
    c.x = Math.max(0, Math.min(COLS - c.w, c.x));
    c.y = Math.max(0, c.y);
    if (id !== fixed) {
      // risale finche' puo', poi scende finche' non tocca nessuno
      while (c.y > 0 && !Object.values(placed).some((p) => overlaps({ ...c, y: c.y - 1 }, p))) c.y -= 1;
      while (Object.values(placed).some((p) => overlaps(c, p))) c.y += 1;
    }
    placed[id] = c;
  }
  return placed;
}

/** Dove mettere un blocco nuovo: il primo a tutta finestra, gli altri sotto, a meta' larghezza. */
export function freeCell(cells: Record<string, Cell>): Cell {
  const all = Object.values(cells);
  // il primo blocco riempie la finestra; gli altri vanno sotto, a meta' larghezza
  if (all.length === 0) return { x: 0, y: 0, w: COLS, h: ROWS };
  const bottom = all.reduce((m, c) => Math.max(m, c.y + c.h), 0);
  return { x: 0, y: bottom, w: 6, h: 12 };
}

interface Props {
  cells: Record<string, Cell>;
  onChange(cells: Record<string, Cell>): void;
  /** Il contenuto di un blocco: `grip` va messo sulla barretta che si trascina. */
  render(id: string, grip: { onPointerDown(e: ReactPointerEvent<HTMLElement>): void }): ReactNode;
  /** Bloccata: i blocchi non si spostano e non si ridimensionano, restano dove sono. */
  locked?: boolean;
}

/**
 * Dashboard a blocchi (secondo schermo): ogni blocco si trascina dalla sua barretta e si ridimensiona dall'angolo in basso a destra;
 * i blocchi si agganciano alla griglia e gli altri si spostano per fargli posto.
 */
export function GridBoard({ cells, onChange, render, locked = false }: Props) {
  const box = useRef<HTMLDivElement | null>(null);
  const [width, setWidth] = useState(1000);
  const [height, setHeight] = useState(700);
  const [live, setLive] = useState<Record<string, Cell> | null>(null);
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const fit = () => {
      setWidth(el.clientWidth);
      setHeight(el.parentElement?.clientHeight ?? el.clientHeight);
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const colW = (width - GAP * (COLS + 1)) / COLS;
  // altezza di una riga: la finestra divisa in ROWS righe (mai meno di 14 px, oltre si scorre)
  const ROW = Math.max(14, (height - GAP * (ROWS + 1)) / ROWS);
  const shown = live ?? cells;

  const track = (id: string, mode: 'move' | 'size') => (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const start = { x: e.clientX, y: e.clientY };
    const from = cells[id];
    if (!from) return;
    let last = cells;
    const move = (ev: PointerEvent) => {
      const dx = Math.round((ev.clientX - start.x) / (colW + GAP));
      const dy = Math.round((ev.clientY - start.y) / (ROW + GAP));
      const next = mode === 'move' ? { ...from, x: from.x + dx, y: Math.max(0, from.y + dy) } : { ...from, w: from.w + dx, h: from.h + dy };
      last = arrange({ ...cells, [id]: next }, id);
      setLive(last);
    };
    const stop = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', stop);
      setLive(null);
      onChange(arrange(last));
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', stop);
  };

  const bottom = Object.values(shown).reduce((m, c) => Math.max(m, c.y + c.h), 0);
  return (
    <div ref={box} className={`grid-board${live ? ' grid-board--moving' : ''}`} style={{ height: bottom * (ROW + GAP) + GAP }}>
      {Object.entries(shown).map(([id, c]) => (
        <div
          key={id}
          className="grid-block"
          style={{ left: GAP + c.x * (colW + GAP), top: GAP + c.y * (ROW + GAP), width: c.w * (colW + GAP) - GAP, height: c.h * (ROW + GAP) - GAP }}
        >
          {render(id, { onPointerDown: locked ? () => undefined : track(id, 'move') })}
          {locked ? null : <div className="grid-resize" onPointerDown={track(id, 'size')} aria-hidden="true" />}
        </div>
      ))}
    </div>
  );
}
