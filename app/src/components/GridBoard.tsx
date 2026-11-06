import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';

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
/** Fin dove una finestra puo' scendere: oltre le 24 righe visibili la pagina scorre. */
export const MAX_ROWS = 60;
const GAP = 6;
/** Quanto vicino (px) un bordo deve arrivare a un altro per agganciarsi. */
const SNAP_PX = 10;

function overlaps(a: Cell, b: Cell): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/**
 * Sistema i blocchi senza sovrapposizioni, come una dashboard: quello che si sta muovendo resta dov'e', gli altri scendono quanto basta
 * e poi risalgono il piu' possibile (nessun buco in alto).
 */
export function arrange(cells: Record<string, Cell>, fixed?: string, lift = true): Record<string, Cell> {
  const ids = Object.keys(cells).sort((a, b) => (a === fixed ? -1 : b === fixed ? 1 : cells[a]!.y - cells[b]!.y || cells[a]!.x - cells[b]!.x));
  const placed: Record<string, Cell> = {};
  for (const id of ids) {
    const c = { ...cells[id]! };
    c.w = Math.max(2, Math.min(COLS, c.w));
    c.h = Math.max(3, c.h);
    c.x = Math.max(0, Math.min(COLS - c.w, c.x));
    c.y = Math.max(0, c.y);
    if (id !== fixed) {
      // risale finche' puo' (solo se `lift`: senza, il blocco resta dove l'hai messo), poi scende finche' non tocca nessuno
      while (lift && c.y > 0 && !Object.values(placed).some((p) => overlaps({ ...c, y: c.y - 1 }, p))) c.y -= 1;
      while (Object.values(placed).some((p) => overlaps(c, p))) c.y += 1;
    }
    if (!lift) c.y = Math.max(0, Math.min(c.y, MAX_ROWS - c.h));
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
  /** Posizionamento libero: i blocchi non risalgono da soli per riempire i buchi, restano dove li lasci. */
  free?: boolean;
  /** Con la pagina bloccata: la barretta si trascina solo a destra e a sinistra e scambia di posto il blocco con il vicino della stessa altezza. */
  swap?: boolean;
}

/**
 * Dashboard a blocchi (secondo schermo): ogni blocco si trascina dalla sua barretta e si ridimensiona dall'angolo in basso a destra;
 * i blocchi si agganciano alla griglia e gli altri si spostano per fargli posto.
 */
export function GridBoard({ cells, onChange, render, locked = false, free = false, swap = false }: Props) {
  const box = useRef<HTMLDivElement | null>(null);
  const [width, setWidth] = useState(1000);
  const [height, setHeight] = useState(700);
  const [live, setLive] = useState<Record<string, Cell> | null>(null);
  /** Le linee guida mostrate mentre si trascina: dove il bordo si e' agganciato a un'altra finestra o al bordo della pagina. */
  const [guides, setGuides] = useState<Array<{ axis: 'x' | 'y'; at: number }>>([]);
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
  const shown = live ?? cells;
  // la riga e' sempre 1/24 della finestra: una finestra puo' allungarsi oltre il fondo e la pagina scorre (le altre non cambiano misura)
  const ROW = Math.max(14, (height - GAP * (ROWS + 1)) / ROWS);

  const track = (id: string, mode: 'move' | 'size') => (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const start = { x: e.clientX, y: e.clientY };
    const from = cells[id];
    if (!from) return;
    let last = cells;
    let frame = 0;
    let latest: PointerEvent | null = null;
    const apply = () => {
      frame = 0;
      const ev = latest;
      if (!ev) return;
      const dx = (ev.clientX - start.x) / (colW + GAP);
      const dy = (ev.clientY - start.y) / (ROW + GAP);
      if (!free) {
        const next = mode === 'move' ? { ...from, x: from.x + Math.round(dx), y: Math.max(0, from.y + Math.round(dy)) } : { ...from, w: from.w + Math.round(dx), h: from.h + Math.round(dy) };
        last = arrange({ ...cells, [id]: next }, id, true);
        setLive(last);
        return;
      }
      // libero e preciso: i bordi si agganciano a quelli delle altre finestre e della pagina (con le linee guida), altrimenti a un quarto di cella
      const others = Object.entries(cells).filter(([k]) => k !== id).map(([, c]) => c);
      const thX = SNAP_PX / (colW + GAP);
      const thY = SNAP_PX / (ROW + GAP);
      const xEdges = [0, COLS, ...others.flatMap((o) => [o.x, o.x + o.w])];
      const yEdges = [0, ...others.flatMap((o) => [o.y, o.y + o.h])];
      const found: Array<{ axis: 'x' | 'y'; at: number }> = [];
      /** Il valore di `edge` agganciato al bordo piu' vicino (entro la soglia) oppure al quarto di cella; registra la guida. */
      const snapEdge = (edge: number, edges: number[], th: number, axis: 'x' | 'y'): { value: number; hit: boolean } => {
        let best = edge;
        let bestDistance = th;
        for (const c of edges) {
          const d = Math.abs(edge - c);
          if (d < bestDistance) {
            bestDistance = d;
            best = c;
          }
        }
        if (best !== edge) {
          found.push({ axis, at: best });
          return { value: best, hit: true };
        }
        return { value: Math.round(edge * 4) / 4, hit: false };
      };
      let next: Cell;
      if (mode === 'move') {
        let x = from.x + dx;
        let y = Math.max(0, from.y + dy);
        // si prova il bordo sinistro e poi quello destro: vince quello che si aggancia
        const left = snapEdge(x, xEdges, thX, 'x');
        if (left.hit) x = left.value;
        else {
          const right = snapEdge(x + from.w, xEdges, thX, 'x');
          x = right.hit ? right.value - from.w : left.value;
        }
        const top = snapEdge(y, yEdges, thY, 'y');
        if (top.hit) y = top.value;
        else {
          const bottom = snapEdge(y + from.h, yEdges, thY, 'y');
          y = bottom.hit ? bottom.value - from.h : top.value;
        }
        next = { ...from, x: Math.max(0, Math.min(COLS - from.w, x)), y: Math.max(0, y) };
      } else {
        const right = snapEdge(from.x + from.w + dx, xEdges, thX, 'x').value;
        const bottom = snapEdge(from.y + from.h + dy, yEdges, thY, 'y').value;
        next = { ...from, w: Math.max(2, right - from.x), h: Math.max(3, bottom - from.y) };
      }
      setGuides(found);
      last = arrange({ ...cells, [id]: next }, id, false);
      setLive(last);
    };
    const move = (ev: PointerEvent) => {
      latest = ev;
      if (!frame) frame = requestAnimationFrame(apply);
    };
    const stop = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', stop);
      if (frame) cancelAnimationFrame(frame);
      apply();
      setLive(null);
      setGuides([]);
      // libero: `last` e' gia' sistemato con la finestra mossa ferma dove l'hai messa; risistemarlo senza di lei la rimetterebbe dove stava
      onChange(free ? last : arrange(last, undefined, true));
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', stop);
  };

  /** Scambio a destra e a sinistra (pagina bloccata): le misure restano, cambiano solo i posti dei due blocchi vicini della stessa altezza. */
  const trackSwap = (id: string) => (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const from = cells[id];
    const rect = box.current?.getBoundingClientRect();
    if (!from || !rect) return;
    const band = Object.entries(cells)
      .filter(([, c]) => Math.abs(c.y - from.y) < 0.01 && Math.abs(c.h - from.h) < 0.01)
      .sort(([, a], [, b]) => a.x - b.x);
    const index = band.findIndex(([k]) => k === id);
    const swapped = (otherId: string): Record<string, Cell> => {
      const a = cells[id]!;
      const b = cells[otherId]!;
      const minX = Math.min(a.x, b.x);
      const maxEnd = Math.max(a.x + a.w, b.x + b.w);
      const aLeft = a.x < b.x;
      return aLeft ? { ...cells, [otherId]: { ...b, x: minX }, [id]: { ...a, x: maxEnd - a.w } } : { ...cells, [id]: { ...a, x: minX }, [otherId]: { ...b, x: maxEnd - b.w } };
    };
    let target: string | null = null;
    const pick = (ev: PointerEvent) => {
      const gx = (ev.clientX - rect.left - GAP) / (colW + GAP);
      const right = band[index + 1];
      const left = band[index - 1];
      if (right && gx > right[1].x + right[1].w / 2) return right[0];
      if (left && gx < left[1].x + left[1].w / 2) return left[0];
      return null;
    };
    const move = (ev: PointerEvent) => {
      target = pick(ev);
      setLive(target ? swapped(target) : null);
    };
    const stop = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', stop);
      setLive(null);
      if (target) onChange(swapped(target));
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', stop);
  };

  // il contenuto di ogni blocco si ridisegna solo quando cambia qualcosa di vero (dati, misure salvate): non a ogni fotogramma del trascinamento
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const nodes = useMemo(() => Object.fromEntries(Object.keys(cells).map((id) => [id, render(id, { onPointerDown: locked ? (swap ? trackSwap(id) : () => undefined) : track(id, 'move') })])), [cells, render, locked, free, swap, colW, ROW, onChange]);
  const bottom = Object.values(shown).reduce((m, c) => Math.max(m, c.y + c.h), 0);
  return (
    <div ref={box} className={`grid-board${live ? ' grid-board--moving' : ''}`} style={{ height: Math.floor(bottom * (ROW + GAP) + GAP) }}>
      {guides.map((g, i) => (
        <div
          key={`${g.axis}${i}`}
          className={`grid-guide grid-guide--${g.axis}`}
          style={g.axis === 'x' ? { left: GAP + g.at * (colW + GAP) - GAP / 2 } : { top: GAP + g.at * (ROW + GAP) - GAP / 2 }}
          aria-hidden="true"
        />
      ))}
      {Object.entries(shown).map(([id, c]) => (
        <div
          key={id}
          className="grid-block"
          style={{ left: GAP + c.x * (colW + GAP), top: GAP + c.y * (ROW + GAP), width: c.w * (colW + GAP) - GAP, height: c.h * (ROW + GAP) - GAP }}
        >
          {nodes[id] ?? render(id, { onPointerDown: locked ? () => undefined : track(id, 'move') })}
          {locked ? null : <div className="grid-resize" onPointerDown={track(id, 'size')} aria-hidden="true" />}
        </div>
      ))}
    </div>
  );
}
