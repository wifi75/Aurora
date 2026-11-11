import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { placeColumn, readColumnLayout } from '../lib/movableColumns';
import { ColumnsMenu, MovableColGroup, MovableTh, contentWidth, useMovableColumns } from './MovableColumns';

const COLS = ['a', 'b', 'c', 'd'] as const;
type Col = (typeof COLS)[number];
const EM: Record<Col, number> = { a: 4, b: 5, c: 6, d: 8 };
const KEY = 'test.cols';

function Grid({ hidden }: { hidden?: Col[] }) {
  const cols = useMovableColumns({ storageKey: KEY, columns: COLS, defaultEm: EM, elastic: 'd', hidden });
  return (
    <>
      <table ref={cols.tableRef} className="mv-grid" style={cols.tableStyle}>
        <MovableColGroup cols={cols} />
        <thead onContextMenu={cols.openMenu}>
          <tr>
            {cols.visible.map((c, i) => (
              <MovableTh key={c} cols={cols} col={c} index={i}>
                {c.toUpperCase()}
              </MovableTh>
            ))}
          </tr>
        </thead>
        <tbody>
          <tr>
            {cols.visible.map((c) => (
              <td key={c}>{`v${c}`}</td>
            ))}
          </tr>
        </tbody>
      </table>
      <ColumnsMenu cols={cols} />
    </>
  );
}

const heads = () => screen.getAllByRole('columnheader').map((th) => th.textContent);
const saved = () => JSON.parse(localStorage.getItem(KEY) ?? 'null') as { order: string[]; widths: Record<string, number> } | null;
/** jsdom non impagina: ogni titolo largo 50 px, uno dopo l'altro. */
function layoutHeads() {
  screen.getAllByRole('columnheader').forEach((th, i) => {
    th.getBoundingClientRect = () => ({ left: i * 50, right: i * 50 + 50, width: 50, top: 0, bottom: 20, height: 20, x: i * 50, y: 0, toJSON: () => ({}) });
  });
}
const pointer = (type: string, clientX: number) =>
  act(() => {
    window.dispatchEvent(new MouseEvent(type, { clientX, button: 0 }));
  });

beforeEach(() => localStorage.clear());

describe('useMovableColumns', () => {
  it('draws the columns in the default order, with the elastic one taking the free space', () => {
    const { container } = render(<Grid />);
    expect(heads()).toEqual(['A', 'B', 'C', 'D']);
    const widths = Array.from(container.querySelectorAll('col')).map((c) => c.style.width);
    expect(widths).toEqual(['4em', '5em', '6em', '']);
    // larga almeno la somma delle fisse piu' il minimo dell'elastica
    expect(screen.getByRole('table').style.minWidth).toBe('23em');
  });

  it('dragging a header moves the column, the cells follow and the order survives a reload', () => {
    const first = render(<Grid />);
    layoutHeads();
    fireEvent.pointerDown(screen.getByRole('columnheader', { name: 'C' }), { button: 0, clientX: 125 });
    pointer('pointermove', 10);
    expect(screen.getByRole('columnheader', { name: 'A' })).toHaveClass('cl-h--drop-before');
    expect(screen.getByRole('columnheader', { name: 'C' })).toHaveClass('cl-h--dragging');
    pointer('pointerup', 10);
    expect(heads()).toEqual(['C', 'A', 'B', 'D']);
    expect(screen.getByText('vc').closest('tr')?.firstElementChild).toBe(screen.getByText('vc'));
    expect(saved()?.order).toEqual(['c', 'a', 'b', 'd']);
    first.unmount();
    render(<Grid />);
    expect(heads()).toEqual(['C', 'A', 'B', 'D']);
  });

  it('a click without moving and Escape during the drag leave the order as it is', () => {
    render(<Grid />);
    layoutHeads();
    fireEvent.pointerDown(screen.getByRole('columnheader', { name: 'D' }), { button: 0, clientX: 175 });
    pointer('pointermove', 177);
    pointer('pointerup', 177);
    fireEvent.pointerDown(screen.getByRole('columnheader', { name: 'D' }), { button: 0, clientX: 175 });
    pointer('pointermove', 10);
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    pointer('pointerup', 10);
    expect(heads()).toEqual(['A', 'B', 'C', 'D']);
    expect(saved()).toBeNull();
  });

  it('Alt+arrows move the column, Alt+Shift+arrows resize it', () => {
    render(<Grid />);
    fireEvent.keyDown(screen.getByRole('columnheader', { name: 'A' }), { key: 'ArrowRight', altKey: true });
    expect(heads()).toEqual(['B', 'A', 'C', 'D']);
    expect(document.activeElement).toBe(screen.getByRole('columnheader', { name: 'A' }));
    fireEvent.keyDown(screen.getByRole('columnheader', { name: 'B' }), { key: 'ArrowRight', altKey: true, shiftKey: true });
    expect(saved()?.widths.b).toBe(5.5);
  });

  it('dragging the right edge resizes the column and the width survives a reload', () => {
    const first = render(<Grid />);
    const grip = screen.getByRole('columnheader', { name: 'B' }).querySelector('.cl-grip');
    if (!grip) throw new Error('manca la maniglia');
    fireEvent.pointerDown(grip, { button: 0, clientX: 100 });
    pointer('pointermove', 132);
    pointer('pointerup', 132);
    // 5 em di partenza + 32 px (a 16 px per em in jsdom)
    expect(saved()?.widths.b ?? 0).toBeCloseTo(7, 5);
    expect(heads()).toEqual(['A', 'B', 'C', 'D']);
    first.unmount();
    const again = render(<Grid />);
    expect(again.container.querySelectorAll('col')[1]?.style.width).toBe('7em');
  });

  it('the header menu resets order and widths', () => {
    localStorage.setItem(KEY, JSON.stringify({ order: ['d', 'c', 'b', 'a'], widths: { a: 20, d: 9 } }));
    const { container } = render(<Grid />);
    expect(heads()).toEqual(['D', 'C', 'B', 'A']);
    // la colonna elastica con una larghezza scelta non e' piu' elastica: la tabella e' larga la somma
    expect(screen.getByRole('table').style.width).toBe('40em');
    fireEvent.contextMenu(screen.getByRole('columnheader', { name: 'B' }), { clientX: 40, clientY: 10 });
    fireEvent.click(screen.getByRole('menuitem', { name: 'Ripristina colonne' }));
    expect(heads()).toEqual(['A', 'B', 'C', 'D']);
    expect(saved()).toEqual({ order: ['a', 'b', 'c', 'd'], widths: {} });
    expect(container.querySelectorAll('col')[0]?.style.width).toBe('4em');
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('Escape closes the menu without touching the columns', () => {
    render(<Grid />);
    fireEvent.contextMenu(screen.getByRole('columnheader', { name: 'A' }), { clientX: 40, clientY: 10 });
    expect(screen.getByRole('menu')).toBeInTheDocument();
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(saved()).toBeNull();
  });

  it('hidden columns are not drawn and keep their place', () => {
    render(<Grid hidden={['b']} />);
    expect(heads()).toEqual(['A', 'C', 'D']);
  });
});

describe('movableColumns storage', () => {
  it('drops unknown or repeated columns, adds new ones in their place and clamps the widths', () => {
    localStorage.setItem(KEY, JSON.stringify({ order: ['c', 'x', 'c', 'a'], widths: { a: 1, c: 99, x: 5, d: 'no' } }));
    expect(readColumnLayout(KEY, COLS)).toEqual({ order: ['c', 'b', 'a', 'd'], widths: { a: 2.5, c: 48 } });
  });

  it('falls back to the default layout when the saved value is unreadable', () => {
    localStorage.setItem(KEY, '{nope');
    expect(readColumnLayout(KEY, COLS)).toEqual({ order: ['a', 'b', 'c', 'd'], widths: {} });
  });

  it('places a column before another or at the end', () => {
    expect(placeColumn(['a', 'b', 'c'], 'c', 'a')).toEqual(['c', 'a', 'b']);
    expect(placeColumn(['a', 'b', 'c'], 'a', null)).toEqual(['b', 'c', 'a']);
    expect(placeColumn(['a', 'b', 'c'], 'b', 'b')).toEqual(['a', 'b', 'c']);
  });
});

/** Una griglia che non e' una tabella (la Finestra RX): le celle da misurare le dice chi la usa. */
function Outside({ cells }: { cells: () => HTMLElement[] }) {
  const cols = useMovableColumns({ storageKey: KEY, columns: COLS, defaultEm: EM, elastic: 'd', fitCells: cells });
  return (
    <>
      <table ref={cols.tableRef} className="mv-grid" style={cols.tableStyle}>
        <MovableColGroup cols={cols} />
        <thead>
          <tr>
            {cols.visible.map((c, i) => (
              <MovableTh key={c} cols={cols} col={c} index={i}>
                {c.toUpperCase()}
              </MovableTh>
            ))}
          </tr>
        </thead>
      </table>
      <button type="button" onClick={() => cols.setWidths({ a: 10, b: 100 })}>
        widths
      </button>
      <button type="button" onClick={() => cols.setWidths({})}>
        reset
      </button>
    </>
  );
}

describe('useMovableColumns, larghezze e misure', () => {
  it('setWidths sostituisce tutte le larghezze (entro i limiti) e le ricorda; {} torna a quelle di partenza', () => {
    const { container } = render(<Outside cells={() => []} />);
    fireEvent.click(screen.getByRole('button', { name: 'widths' }));
    expect(saved()?.widths).toEqual({ a: 10, b: 48 });
    expect(Array.from(container.querySelectorAll('col')).map((c) => c.style.width)).toEqual(['10em', '48em', '6em', '']);
    fireEvent.click(screen.getByRole('button', { name: 'reset' }));
    expect(saved()?.widths).toEqual({});
  });

  it('il doppio clic sul bordo misura le celle indicate da fitCells', () => {
    const cell = document.createElement('span');
    Object.defineProperty(cell, 'scrollWidth', { value: 120 });
    render(<Outside cells={() => [cell]} />);
    fireEvent.doubleClick(screen.getByRole('columnheader', { name: 'B' }).querySelector('.cl-grip') as Element);
    // 120 px a 16 px per em (il corpo del carattere di jsdom), piu' 0,3 em di respiro
    expect(saved()?.widths.b).toBeCloseTo(120 / 16 + 0.3, 1);
  });

  it('contentWidth conta anche il testo nascosto dai «…» di un elemento interno', () => {
    const cell = document.createElement('span');
    const inner = document.createElement('span');
    cell.appendChild(inner);
    Object.defineProperty(cell, 'scrollWidth', { value: 80 });
    Object.defineProperty(inner, 'scrollWidth', { value: 150 });
    Object.defineProperty(inner, 'clientWidth', { value: 70 });
    inner.getBoundingClientRect = () => ({ left: 10, right: 80, width: 70, top: 0, bottom: 10, height: 10, x: 10, y: 0, toJSON: () => ({}) });
    expect(contentWidth(cell)).toBe(160);
  });
});
