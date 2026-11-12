import { useCallback, useEffect, useRef, useState, type CSSProperties, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type ReactNode, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { useT } from '../i18n';
import { clampColumnEm, defaultColumnLayout, placeColumn, readColumnLayout, saveColumnLayout, type ColumnLayout } from '../lib/movableColumns';
import '../styles/movable-grid.css';

/** Quanto si sposta il mouse prima che il clic sul titolo diventi un trascinamento della colonna. */
const DRAG_PX = 5;

export interface MovableColumnsSpec<C extends string> {
  /** Dove si ricordano ordine e larghezze, su questo dispositivo. */
  storageKey: string;
  /** Le colonne nell'ordine di partenza. */
  columns: readonly C[];
  /** Larghezza di partenza di ogni colonna, in em (della colonna elastica: il minimo quando prende lo spazio che avanza). */
  defaultEm: Record<C, number>;
  /** La colonna che, finche' non le si sceglie una larghezza, prende lo spazio che avanza. */
  elastic?: C;
  /** Colonne da non disegnare (restano al loro posto nell'ordine). */
  hidden?: readonly C[];
  /** Le celle da misurare per adattare la colonna al contenuto (doppio clic sul bordo): di base quelle della tabella in quella posizione. */
  fitCells?: (col: C, index: number) => Iterable<Element>;
}

export interface MovableColumns<C extends string> {
  layout: ColumnLayout<C>;
  /** Le colonne da disegnare, nell'ordine scelto. */
  visible: C[];
  widthOf: (c: C) => number;
  /** La colonna elastica prende lo spazio che avanza (non le e' stata scelta una larghezza). */
  elastic: boolean;
  elasticCol: C | undefined;
  tableRef: RefObject<HTMLTableElement | null>;
  /** Larghezza della tabella: minima con la colonna elastica, la somma delle colonne senza. */
  tableStyle: CSSProperties;
  drag: { col: C; before: C | null } | null;
  menu: { x: number; y: number } | null;
  startMove: (e: ReactPointerEvent<HTMLTableCellElement>, col: C) => void;
  startResize: (e: ReactPointerEvent<HTMLSpanElement>, col: C) => void;
  autoFit: (col: C) => void;
  /** Sostituisce tutte le larghezze scelte a mano (`{}`: tutte di partenza) e le ricorda. */
  setWidths: (widths: Partial<Record<C, number>>) => void;
  keyMove: (e: ReactKeyboardEvent<HTMLTableCellElement>, col: C) => void;
  /** Tasto destro (o il tasto Menu) sui titoli: apre il menu con «Ripristina colonne». */
  openMenu: (e: ReactMouseEvent<HTMLElement>) => void;
  closeMenu: () => void;
  reset: () => void;
}

/**
 * Colonne come in un foglio di calcolo: trascinando il titolo la colonna si sposta, trascinando il bordo destro si allarga,
 * doppio clic sul bordo la adatta al contenuto; da tastiera Alt+frecce e Alt+Maiusc+frecce; tasto destro «Ripristina colonne».
 * Va chiamato prima di ogni `return` anticipato del componente: la tabella si aggancia con `tableRef` quando c'e'.
 */
export function useMovableColumns<C extends string>({ storageKey, columns, defaultEm, elastic: elasticCol, hidden, fitCells }: MovableColumnsSpec<C>): MovableColumns<C> {
  const [layout, setLayoutState] = useState<ColumnLayout<C>>(() => readColumnLayout(storageKey, columns));
  const [drag, setDrag] = useState<{ col: C; before: C | null } | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const tableRef = useRef<HTMLTableElement | null>(null);
  const focusCol = useRef<C | null>(null);
  const stopPointer = useRef<(() => void) | null>(null);
  const setLayout = useCallback(
    (update: (l: ColumnLayout<C>) => ColumnLayout<C>) =>
      setLayoutState((l) => {
        const next = update(l);
        saveColumnLayout(storageKey, next);
        return next;
      }),
    [storageKey],
  );
  // spostata da tastiera la colonna resta quella attiva, anche se React ne sposta il titolo
  useEffect(() => {
    const col = focusCol.current;
    focusCol.current = null;
    if (col) tableRef.current?.querySelector<HTMLElement>(`th[data-col="${col}"]`)?.focus();
  }, [layout.order]);
  // un trascinamento a meta' quando la finestra si chiude non lascia ascoltatori appesi
  useEffect(() => () => stopPointer.current?.(), []);

  const visible = layout.order.filter((c) => !hidden?.includes(c));
  const widthOf = (c: C) => layout.widths[c] ?? defaultEm[c];
  const elastic = elasticCol !== undefined && visible.includes(elasticCol) && layout.widths[elasticCol] === undefined;
  const fixedEm = visible.reduce((n, c) => n + (elastic && c === elasticCol ? 0 : widthOf(c)), 0);
  const tableStyle: CSSProperties = elastic && elasticCol !== undefined ? { minWidth: `${fixedEm + defaultEm[elasticCol]}em` } : { width: `${fixedEm}em` };
  /** Il corpo del carattere della tabella in px: le larghezze sono in em. */
  const emPx = () => parseFloat(getComputedStyle(tableRef.current ?? document.body).fontSize) || 12;
  const setWidth = (c: C, em: number) => setLayout((l) => ({ ...l, widths: { ...l.widths, [c]: clampColumnEm(em) } }));

  /** Trascinando il titolo la colonna si sposta: lasciata a meta' di un altro titolo, va prima o dopo di quello. Esc la annulla. */
  const startMove = (e: ReactPointerEvent<HTMLTableCellElement>, col: C) => {
    if (e.button !== 0) return;
    const row = e.currentTarget.parentElement;
    if (!row) return;
    stopPointer.current?.();
    const startX = e.clientX;
    let active = false;
    let before: C | null = null;
    const slotAt = (x: number): C | null => {
      for (const th of Array.from(row.querySelectorAll<HTMLElement>('th[data-col]'))) {
        const r = th.getBoundingClientRect();
        if (x < r.left + r.width / 2) return th.dataset.col as C;
      }
      return null;
    };
    const move = (m: PointerEvent) => {
      if (!active && Math.abs(m.clientX - startX) < DRAG_PX) return;
      const next = slotAt(m.clientX);
      // si ridisegna solo quando cambia il posto di arrivo, non a ogni movimento del mouse
      if (active && next === before) return;
      active = true;
      before = next;
      setDrag({ col, before });
    };
    const end = (apply: boolean) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', drop);
      window.removeEventListener('pointercancel', cancel);
      window.removeEventListener('keydown', esc);
      window.removeEventListener('blur', cancel);
      stopPointer.current = null;
      setDrag(null);
      if (apply && active) {
        const to = before;
        setLayout((l) => ({ ...l, order: placeColumn(l.order, col, to) }));
      }
    };
    const drop = () => end(true);
    const cancel = () => end(false);
    const esc = (k: KeyboardEvent) => {
      if (k.key === 'Escape') end(false);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', drop);
    window.addEventListener('pointercancel', cancel);
    window.addEventListener('keydown', esc);
    window.addEventListener('blur', cancel);
    stopPointer.current = cancel;
  };

  /** Trascinando il bordo destro del titolo la colonna si allarga o si stringe, come in Excel. */
  const startResize = (e: ReactPointerEvent<HTMLSpanElement>, col: C) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    stopPointer.current?.();
    const startX = e.clientX;
    // la colonna elastica parte dalla larghezza che ha adesso
    const shown = e.currentTarget.parentElement?.getBoundingClientRect().width ?? 0;
    const em = emPx();
    const start = layout.widths[col] ?? (shown > 0 ? shown / em : defaultEm[col]);
    let last = start;
    const move = (m: PointerEvent) => {
      last = clampColumnEm(start + (m.clientX - startX) / em);
      setLayoutState((l) => ({ ...l, widths: { ...l.widths, [col]: last } }));
    };
    const end = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      stopPointer.current = null;
      if (last !== start) setWidth(col, last);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
    stopPointer.current = end;
  };

  /** Doppio clic sul bordo: la colonna prende la larghezza del suo contenuto piu' lungo. */
  const autoFit = (col: C) => {
    const el = tableRef.current;
    const i = visible.indexOf(col);
    if (!el || i < 0) return;
    let max = 0;
    for (const cell of fitCells ? fitCells(col, i) : el.querySelectorAll(`tr > :nth-child(${i + 1})`)) {
      if (cell instanceof HTMLElement) max = Math.max(max, contentWidth(cell));
    }
    if (max > 0) setWidth(col, max / emPx() + 0.3);
  };
  const setWidths = (widths: Partial<Record<C, number>>) => {
    const next: Partial<Record<C, number>> = {};
    for (const c of columns) {
      const v = widths[c];
      if (v !== undefined) next[c] = clampColumnEm(v);
    }
    setLayout((l) => ({ ...l, widths: next }));
  };

  /** Da tastiera, sul titolo: Alt+frecce sposta la colonna, Alt+Maiusc+frecce la allarga o la stringe. */
  const keyMove = (e: ReactKeyboardEvent<HTMLTableCellElement>, col: C) => {
    if (!e.altKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
    e.preventDefault();
    const left = e.key === 'ArrowLeft';
    if (e.shiftKey) {
      // come trascinando il bordo: la colonna elastica senza larghezza scelta parte da quella che ha adesso, non dal minimo
      const shown = e.currentTarget.getBoundingClientRect().width;
      const start = layout.widths[col] ?? (shown > 0 ? shown / emPx() : defaultEm[col]);
      setWidth(col, start + (left ? -0.5 : 0.5));
      return;
    }
    const i = visible.indexOf(col);
    if (left ? i <= 0 : i < 0 || i >= visible.length - 1) return;
    const before = left ? (visible[i - 1] ?? null) : (visible[i + 2] ?? null);
    focusCol.current = col;
    setLayout((l) => ({ ...l, order: placeColumn(l.order, col, before) }));
  };

  const openMenu = (e: ReactMouseEvent<HTMLElement>) => {
    e.preventDefault();
    let { clientX: x, clientY: y } = e;
    if (x === 0 && y === 0) {
      const r = (e.target as HTMLElement).getBoundingClientRect();
      x = r.left;
      y = r.bottom;
    }
    setMenu({ x, y });
  };
  const closeMenu = useCallback(() => setMenu(null), []);
  const reset = () => {
    setMenu(null);
    setLayout(() => defaultColumnLayout(columns));
  };

  return { layout, visible, widthOf, elastic, elasticCol, tableRef, tableStyle, drag, menu, startMove, startResize, autoFit, setWidths, keyMove, openMenu, closeMenu, reset };
}

/**
 * Quanto e' largo il contenuto di una cella, in px: anche quello nascosto dai «…» di un elemento interno (il messaggio della Finestra RX
 * sta in uno span che si accorcia da se'), contando da dove l'elemento comincia dentro la cella piu' il margine destro della cella.
 */
export function contentWidth(cell: HTMLElement): number {
  let w = cell.scrollWidth;
  const inner = cell.querySelectorAll<HTMLElement>('*');
  if (inner.length === 0) return w;
  const left = cell.getBoundingClientRect().left;
  const pad = parseFloat(getComputedStyle(cell).paddingRight) || 0;
  inner.forEach((el) => {
    if (el.scrollWidth > el.clientWidth) w = Math.max(w, el.getBoundingClientRect().left - left + el.scrollWidth + pad);
  });
  return w;
}

/** Le larghezze delle colonne, nell'ordine scelto (la colonna elastica senza larghezza prende lo spazio che avanza). */
export function MovableColGroup<C extends string>({ cols }: { cols: MovableColumns<C> }) {
  return (
    <colgroup>
      {cols.visible.map((c) => (
        <col key={c} style={cols.elastic && c === cols.elasticCol ? undefined : { width: `${cols.widthOf(c)}em` }} />
      ))}
    </colgroup>
  );
}

/** Il titolo di una colonna: si trascina per spostarla, il bordo destro per allargarla. */
export function MovableTh<C extends string>({ cols, col, index, className, title, ariaLabel, children }: { cols: MovableColumns<C>; col: C; index: number; className?: string; title?: string; ariaLabel?: string; children?: ReactNode }) {
  const t = useT();
  const { drag, visible } = cols;
  const dropBefore = drag !== null && drag.before === col && drag.col !== col;
  const dropAfter = drag !== null && drag.before === null && index === visible.length - 1 && drag.col !== col;
  return (
    <th
      data-col={col}
      scope="col"
      tabIndex={0}
      aria-label={ariaLabel}
      className={`cl-h cl-h--${col}${className ? ` ${className}` : ''}${drag?.col === col ? ' cl-h--dragging' : ''}${dropBefore ? ' cl-h--drop-before' : ''}${dropAfter ? ' cl-h--drop-after' : ''}`}
      title={title ? `${title}\n${t('tip.cluster.colHead')}` : t('tip.cluster.colHead')}
      onPointerDown={(e) => cols.startMove(e, col)}
      onKeyDown={(e) => cols.keyMove(e, col)}
    >
      {children}
      <span className="cl-grip" role="separator" aria-orientation="vertical" title={t('digi.log.colGrip')} onPointerDown={(e) => cols.startResize(e, col)} onDoubleClick={() => cols.autoFit(col)} />
    </th>
  );
}

/** Ingombro del menu dei titoli (una voce), per tenerlo dentro la finestra. */
const MENU_W = 200;
const MENU_H = 48;

/** Il menu del tasto destro sui titoli: «Ripristina colonne». Si chiude con Esc, con un clic fuori o se la finestra cambia. */
export function ColumnsMenu<C extends string>({ cols }: { cols: MovableColumns<C> }) {
  const t = useT();
  const box = useRef<HTMLDivElement | null>(null);
  const { menu, closeMenu } = cols;
  useEffect(() => {
    if (!menu) return;
    const out = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node)) closeMenu();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeMenu();
    };
    document.addEventListener('pointerdown', out);
    window.addEventListener('keydown', key);
    window.addEventListener('blur', closeMenu);
    window.addEventListener('resize', closeMenu);
    return () => {
      document.removeEventListener('pointerdown', out);
      window.removeEventListener('keydown', key);
      window.removeEventListener('blur', closeMenu);
      window.removeEventListener('resize', closeMenu);
    };
  }, [menu, closeMenu]);
  if (!menu) return null;
  // dentro la finestra anche aperto vicino al bordo destro o in fondo, come il menu dei nominativi
  const left = Math.max(4, Math.min(menu.x, window.innerWidth - MENU_W));
  const top = Math.max(4, Math.min(menu.y, window.innerHeight - MENU_H));
  return createPortal(
    <div ref={box} className="cl-menu" role="menu" style={{ left, top }}>
      <button type="button" role="menuitem" autoFocus onClick={cols.reset}>
        {t('cluster.cols.reset')}
      </button>
    </div>,
    document.body,
  );
}
