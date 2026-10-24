import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { useT } from '../i18n';
import { movePanel, resizePanel, setSplit, toggleCollapsed, type Layout, type PanelId, type Zone } from '../lib/layout';

interface Props {
  layout: Layout;
  onChange(layout: Layout): void;
  titles: Record<PanelId, string>;
  /** Pannelli da non mostrare (per esempio la cascata nascosta). */
  hidden?: readonly PanelId[];
  render(id: PanelId): ReactNode;
}

/** Finestre che si disegnano affiancate quando sono vicine e aperte: la prima a sinistra, la seconda a destra. */
const PAIRS: Partial<Record<PanelId, PanelId>> = { all: 'log', cluster: 'station' };

interface DropSpot {
  zone: Zone;
  index: number;
}

/**
 * Due colonne di pannelli: ognuno si trascina dalla barra del titolo (anche in un'altra colonna o in basso),
 * si ridimensiona dal bordo inferiore, e il divisore sposta la larghezza delle colonne.
 * Su schermi stretti le colonne si impilano e il trascinamento e' spento.
 */
function capture(el: HTMLElement, pointerId: number): void {
  try {
    el.setPointerCapture(pointerId);
  } catch {
    // senza cattura il trascinamento funziona lo stesso: lo chiudono gli ascoltatori sulla finestra
  }
}

export function Dock({ layout, onChange, titles, hidden = [], render }: Props) {
  const t = useT();
  const columns = useRef<Record<Zone, HTMLDivElement | null>>({ left: null, right: null });
  const root = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState<PanelId | null>(null);
  const [spot, setSpot] = useState<DropSpot | null>(null);

  // Qualunque cosa interrompa il trascinamento (rilascio fuori dalla finestra, Esc, cambio di finestra,
  // tocco annullato) lo chiude: un pannello non deve restare "in volo" per sempre.
  useEffect(() => {
    if (!dragging) return;
    const end = () => {
      setDragging(null);
      setSpot(null);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') end();
    };
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
    window.addEventListener('blur', end);
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      window.removeEventListener('blur', end);
      window.removeEventListener('keydown', onKey);
    };
  }, [dragging]);

  const spotAt = (x: number, y: number, dragged: PanelId): DropSpot => {
    const left = columns.current.left?.getBoundingClientRect();
    const zone: Zone = left && x < left.right + 4 ? 'left' : 'right';
    const panels = Array.from(columns.current[zone]?.querySelectorAll<HTMLElement>('[data-panel]') ?? []).filter(
      (el) => el.dataset.panel !== dragged,
    );
    const index = panels.filter((el) => {
      const r = el.getBoundingClientRect();
      return y > r.top + r.height / 2;
    }).length;
    return { zone, index };
  };

  const startDrag = (id: PanelId) => (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    capture(e.currentTarget, e.pointerId);
    setDragging(id);
    setSpot(spotAt(e.clientX, e.clientY, id));
  };
  const moveDrag = (id: PanelId) => (e: ReactPointerEvent<HTMLElement>) => {
    if (dragging === id) setSpot(spotAt(e.clientX, e.clientY, id));
  };
  const endDrag = (id: PanelId) => (e: ReactPointerEvent<HTMLElement>) => {
    if (dragging !== id) return;
    const target = spotAt(e.clientX, e.clientY, id);
    setDragging(null);
    setSpot(null);
    onChange(movePanel(layout, id, target.zone, target.index));
  };

  const startResize = (id: PanelId) => (e: ReactPointerEvent<HTMLElement>) => {
    const panel = e.currentTarget.closest<HTMLElement>('[data-panel]');
    if (!panel) return;
    capture(e.currentTarget, e.pointerId);
    const startY = e.clientY;
    const startHeight = panel.getBoundingClientRect().height;
    const handle = e.currentTarget;
    const move = (ev: PointerEvent) => onChange(resizePanel(layout, id, startHeight + ev.clientY - startY));
    const stop = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  };

  const startSplit = (e: ReactPointerEvent<HTMLElement>) => {
    const box = root.current?.getBoundingClientRect();
    if (!box) return;
    capture(e.currentTarget, e.pointerId);
    const handle = e.currentTarget;
    const move = (ev: PointerEvent) => onChange(setSplit(layout, (ev.clientX - box.left) / box.width));
    const stop = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  };

  const column = (zone: Zone) => {
    const ids = layout[zone].filter((id) => !hidden.includes(id));
    const nodes: ReactNode[] = [];
    const marker = (key: string) => <div key={key} className="dock-drop" aria-hidden="true" />;
    let slot = 0; // posizione tra i pannelli che non si stanno trascinando
    // Attivita' di banda e Log vicini, aperti e non in volo: una riga sola, uno accanto all'altro
    const paired = (id: PanelId, next?: PanelId) =>
      next !== undefined && PAIRS[id] === next && ![id, next].some((p) => layout.collapsed.includes(p) || p === dragging);
    let row: ReactNode[] | null = null;
    let rowSecond: PanelId | null = null;
    ids.forEach((id, at) => {
      const flying = id === dragging;
      const inRow = row !== null && id === rowSecond;
      if (!flying && !inRow) {
        if (spot?.zone === zone && spot.index === slot) nodes.push(marker(`drop-${slot}`));
        slot += 1;
      }
      const folded = layout.collapsed.includes(id);
      const height = folded ? undefined : layout.heights[id];
      const startsRow = paired(id, ids[at + 1]);
      const target = inRow && row ? row : nodes;
      const section = (
        <section
          key={id}
          data-panel={id}
          className={`dock-panel dock-panel--${id}${folded ? ' dock-panel--folded' : ''}${flying ? ' dock-panel--flying' : ''}`}
          style={folded ? { flex: 'none', minHeight: 0 } : inRow || startsRow ? undefined : height ? { height, flex: 'none' } : undefined}
        >
          <header
            className="dock-grip"
            onPointerDown={startDrag(id)}
            onPointerMove={moveDrag(id)}
            onPointerUp={endDrag(id)}
            onDoubleClick={() => onChange(toggleCollapsed(layout, id))}
            title={t('tip.dock.grip')}
            onPointerCancel={() => {
              setDragging(null);
              setSpot(null);
            }}
          >
            <span className="dock-grip-dots" aria-hidden="true" />
            <h2>{titles[id]}</h2>
          </header>
          {!folded && <div className="dock-body">{render(id)}</div>}
          {!folded && <div className="dock-resize" role="separator" aria-orientation="horizontal" aria-label={t('dock.height', { title: titles[id] })} title={t('tip.dock.resize')} onPointerDown={startResize(id)} />}
        </section>
      );
      if (startsRow) {
        row = [section];
        rowSecond = PAIRS[id] ?? null;
        nodes.push(
          <div key={`pair-${id}`} className="dock-pair" style={height ? { height, flex: 'none' } : { minHeight: '14rem', flex: 'none' }}>
            {row}
          </div>,
        );
      } else {
        target.push(section);
        if (inRow) row = null;
      }
    });
    if (spot?.zone === zone && spot.index >= slot) nodes.push(marker('drop-end'));
    if (ids.length === 0 && !(spot?.zone === zone)) {
      nodes.push(
        <div key="empty" className="dock-empty">
          {t('dock.empty')}
        </div>,
      );
    }
    return nodes;
  };

  return (
    <div ref={root} className={`dock${dragging ? ' dock--dragging' : ''}`} style={{ ['--dock-split' as string]: `${layout.split}fr`, ['--dock-rest' as string]: `${1 - layout.split}fr` }}>
      <div className="dock-col" ref={(el) => { columns.current.left = el; }}>{column('left')}</div>
      <div className="dock-splitter" role="separator" aria-orientation="vertical" aria-label={t('dock.width')} onPointerDown={startSplit} />
      <div className="dock-col" ref={(el) => { columns.current.right = el; }}>{column('right')}</div>
    </div>
  );
}
