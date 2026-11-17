import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { DASH_COLS, moveItem, normalize, resizeItem, resizeWidth, snapCells, type DashItem, type DashSpecs } from '../lib/dashGrid';
import '../styles/dash.css';

export const DASH_ROW = 30;
export const DASH_GAP = 10;
/** Sotto questo spostamento (px) un gesto sulla barra non e' ancora un trascinamento. */
const MOVE_PX = 3;
/** Elementi che non fanno partire il trascinamento dalla barra del titolo. */
const NO_DRAG = 'button, input, select, textarea, a, label, table, [data-nodrag]';

export interface DashWidget {
  id: string;
  title: ReactNode;
  /** Comandi nella barra del titolo (campi, pulsanti): non fanno partire il trascinamento. */
  actions?: ReactNode;
  minW: number;
  minH: number;
  /** Colonne massime (la scheda non si allarga oltre). */
  maxW?: number;
  /** Righe massime prese dal contenuto: oltre, la scheda scorre dentro. */
  maxRows: number;
  /** Falso: l'altezza non segue il contenuto ma e' quella della disposizione (schede che scorrono dentro, come i moltiplicatori e il log). */
  autoHeight?: boolean;
  /** Larghezza / altezza fisse (es. 2 per 2/1): l'altezza segue la larghezza e la scheda non si deforma. */
  ratio?: number;
  /** Senza barra del titolo (la cartina): si afferra dalla maniglia in alto. */
  bare?: boolean;
  className?: string;
  children: ReactNode;
}

interface Props {
  widgets: DashWidget[];
  /** La disposizione salvata (di tutte le schede note, anche quelle ora assenti). */
  layout: DashItem[];
  onLayout(items: DashItem[]): void;
  locked: boolean;
  /** Finestra stretta: schede impilate a larghezza piena, senza spostamenti. */
  narrow: boolean;
  /** Larghezza di partenza (colonne) di ogni scheda: il doppio clic sul maniglione la ripristina. */
  defaultWidths: Record<string, number>;
  moveLabel: string;
  resizeLabel: string;
  widthLabel: string;
  /** Classe del riquadro di ogni scheda (di base quella dei Satelliti). */
  panelClass?: string;
}

interface Gesture {
  kind: 'move' | 'resize' | 'width';
  id: string;
  pointer: number;
  x0: number;
  y0: number;
  scale: number;
  base: { left: number; top: number; width: number; height: number };
  dx: number;
  dy: number;
  started: boolean;
  preview: DashItem[];
}

const GripDots = () => (
  <svg className="dash-grip" viewBox="0 0 10 16" width="10" height="16" aria-hidden="true" focusable="false">
    {[3, 8, 13].flatMap((y) => [2.5, 7.5].map((x) => <circle key={`${x}-${y}`} cx={x} cy={y} r="1.3" fill="currentColor" />))}
  </svg>
);

/** Griglia di schede spostabili (dalla barra del titolo) e ridimensionabili (dall'angolo) che resta sempre ordinata e piena. */
export function DashGrid({ widgets, layout, onLayout, locked, narrow, defaultWidths, moveLabel, resizeLabel, widthLabel, panelClass = 'sat-panel' }: Props) {
  const box = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [gesture, setGesture] = useState<Gesture | null>(null);
  const gref = useRef<Gesture | null>(null);
  const heightFloor = useRef(0);
  /** Altezza in righe del contenuto di ogni scheda, misurata sul disegno. */
  const [auto, setAuto] = useState<Record<string, number>>({});

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => setWidth(Math.floor(el.clientWidth));
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    measure();
    return () => ro.disconnect();
  }, []);

  // la barra di scorrimento della pagina puo' comparire dopo la misura: a ogni disegno si ricontrolla la larghezza
  useEffect(() => {
    const el = box.current;
    if (el && Math.floor(el.clientWidth) !== width) setWidth(Math.floor(el.clientWidth));
  });

  // l'altezza di ogni scheda segue il suo contenuto: si misura a ogni disegno e si cambia stato solo se cambia il numero di righe
  useLayoutEffect(() => {
    const root = box.current;
    if (!root || narrow || colPx <= 0) return;
    const next: Record<string, number> = {};
    for (const el of root.querySelectorAll<HTMLElement>('.dash-item')) {
      const id = el.dataset.dash;
      const card = el.querySelector<HTMLElement>('.dash-card');
      const body = el.querySelector<HTMLElement>('.dash-body');
      if (!id || !card || !body || card.classList.contains('dash-card--bare')) continue;
      const sum = (kids: Element[]) =>
        kids.reduce((s, k) => {
          const c = getComputedStyle(k);
          return s + (k as HTMLElement).offsetHeight + (parseFloat(c.marginTop) || 0) + (parseFloat(c.marginBottom) || 0);
        }, 0);
      let content = 0;
      for (const k of Array.from(body.children)) {
        // tabelle ed elenco si allungano da sole: conta l'altezza naturale di cio' che contengono
        const grows = k.classList.contains('sat-table') || k.classList.contains('sat-list') || k.hasAttribute('data-grow');
        const c = getComputedStyle(k);
        content += grows ? sum(Array.from(k.children)) + (parseFloat(c.marginTop) || 0) + (parseFloat(c.marginBottom) || 0) : sum([k]);
      }
      const chrome = card.offsetHeight - body.offsetHeight;
      next[id] = Math.max(1, Math.ceil((chrome + content + DASH_GAP) / (DASH_ROW + DASH_GAP)));
    }
    const same = Object.keys(next).length === Object.keys(auto).length && Object.keys(next).every((k) => next[k] === auto[k]);
    if (!same) setAuto(next);
  });

  const colPx = width > 0 ? (width - (DASH_COLS - 1) * DASH_GAP) / DASH_COLS : 0;
  const ids = widgets.map((w) => w.id).join(',');

  const specs = useMemo<DashSpecs>(() => {
    const out: DashSpecs = {};
    for (const w of widgets) {
      const ratio = w.ratio;
      out[w.id] = {
        minW: w.minW,
        minH: w.minH,
        maxW: w.maxW,
        rows: ratio || w.autoHeight === false ? undefined : Math.min(w.maxRows, auto[w.id] ?? 0) || undefined,
        hOf: ratio ? (cols: number) => Math.max(w.minH, Math.ceil(((cols * colPx + (cols - 1) * DASH_GAP) / ratio + DASH_GAP) / (DASH_ROW + DASH_GAP) - 0.001)) : undefined,
      };
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ids, colPx, auto]);

  const present = useMemo(() => layout.filter((i) => widgets.some((w) => w.id === i.id)), [layout, ids]); // eslint-disable-line react-hooks/exhaustive-deps
  const settled = useMemo(() => (colPx > 0 ? normalize(present, specs) : []), [present, specs, colPx]);
  const shown = gesture ? gesture.preview : settled;

  const rect = (it: DashItem) => ({
    left: it.x * (colPx + DASH_GAP),
    top: it.y * (DASH_ROW + DASH_GAP),
    width: it.w * colPx + (it.w - 1) * DASH_GAP,
    height: it.h * DASH_ROW + (it.h - 1) * DASH_GAP,
  });

  /** Salva le posizioni; l'altezza si salva solo per la scheda che l'utente ha ridimensionato dall'angolo, per le altre resta quella di prima (o automatica). */
  const commit = (items: DashItem[], id: string, withHeight: boolean) => {
    const start = settled.find((i) => i.id === id);
    const next = items.map((n) => {
      const old = layout.find((i) => i.id === n.id);
      const chosen = withHeight && n.id === id && start && n.h !== start.h;
      if (chosen) return { ...n, userH: true };
      return old?.userH ? { ...n, h: old.h, userH: true } : { id: n.id, x: n.x, y: n.y, w: n.w, h: old?.h ?? n.h };
    });
    const keep = layout.filter((i) => !items.some((n) => n.id === i.id));
    onLayout([...next, ...keep]);
  };

  const scaleOf = () => {
    const el = box.current;
    return el && el.offsetWidth > 0 ? el.getBoundingClientRect().width / el.offsetWidth : 1;
  };

  const begin = (e: ReactPointerEvent<HTMLElement>, kind: Gesture['kind'], id: string) => {
    const it = settled.find((i) => i.id === id);
    if (!it || locked || narrow || (e.pointerType === 'mouse' && e.button !== 0)) return;
    const g: Gesture = { kind, id, pointer: e.pointerId, x0: e.clientX, y0: e.clientY, scale: scaleOf(), base: rect(it), dx: 0, dy: 0, started: kind !== 'move', preview: settled };
    gref.current = g;
    heightFloor.current = Math.max(0, ...settled.map((i) => rect(i).top + rect(i).height));
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      /* il puntatore e' gia' finito */
    }
    if (kind !== 'move') setGesture(g);
  };

  const step = (e: ReactPointerEvent<HTMLElement>) => {
    const g = gref.current;
    if (!g || g.pointer !== e.pointerId) return;
    const dx = (e.clientX - g.x0) / g.scale;
    const dy = (e.clientY - g.y0) / g.scale;
    if (!g.started) {
      if (Math.hypot(dx, dy) < MOVE_PX) return;
      g.started = true;
    }
    const cw = colPx + DASH_GAP;
    const rh = DASH_ROW + DASH_GAP;
    if (g.kind === 'move') {
      g.preview = moveItem(settled, g.id, Math.round((g.base.left + dx) / cw), Math.round((g.base.top + dy) / rh), specs);
    } else if (g.kind === 'width') {
      g.preview = resizeWidth(settled, g.id, snapCells(g.base.width + dx, colPx, DASH_GAP), specs);
    } else {
      const ratio = widgets.find((w) => w.id === g.id)?.ratio;
      const wPx = ratio ? g.base.width + (ratio * (dx * ratio + dy)) / (ratio * ratio + 1) : g.base.width + dx;
      const hPx = g.base.height + dy;
      g.preview = resizeItem(settled, g.id, snapCells(wPx, colPx, DASH_GAP), snapCells(hPx, DASH_ROW, DASH_GAP), specs);
    }
    g.dx = dx;
    g.dy = dy;
    setGesture({ ...g });
  };

  const finish = (e: ReactPointerEvent<HTMLElement>, cancelled: boolean) => {
    const g = gref.current;
    if (!g || g.pointer !== e.pointerId) return;
    gref.current = null;
    setGesture(null);
    if (!cancelled && g.started) commit(g.preview, g.id, g.kind === 'resize');
  };

  const gestureProps = (kind: Gesture['kind'], id: string) => ({
    onPointerMove: step,
    onPointerUp: (e: ReactPointerEvent<HTMLElement>) => finish(e, false),
    onPointerCancel: (e: ReactPointerEvent<HTMLElement>) => finish(e, true),
    onPointerDown: (e: ReactPointerEvent<HTMLElement>) => {
      if (kind === 'move') {
        if ((e.target as HTMLElement).closest(NO_DRAG)) return;
      } else {
        e.stopPropagation();
      }
      begin(e, kind, id);
    },
  });

  const card = (w: DashWidget, pos?: CSSProperties, dragging = false) => (
    <div key={w.id} className={`dash-item${dragging ? ' dash-item--drag' : ''}${narrow ? ' dash-item--stack' : ''}`} style={pos} data-dash={w.id}>
      <section className={`${panelClass} dash-card${w.bare ? ' dash-card--bare' : ''}${w.className ? ` ${w.className}` : ''}`}>
        {w.bare ? (
          locked || narrow ? null : (
            <div className="dash-pill" title={moveLabel} aria-label={moveLabel} {...gestureProps('move', w.id)}>
              <GripDots />
            </div>
          )
        ) : (
          <div className={`dash-bar${locked || narrow ? '' : ' dash-bar--grab'}`} title={locked || narrow ? undefined : moveLabel} {...gestureProps('move', w.id)}>
            {locked || narrow ? null : <GripDots />}
            <h2 className="dash-title">{w.title}</h2>
            {w.actions ? <div className="dash-actions">{w.actions}</div> : null}
          </div>
        )}
        <div className="dash-body">{w.children}</div>
        {locked || narrow ? null : (
          <>
            <div
              className="dash-resize"
              role="separator"
              aria-label={resizeLabel}
              title={resizeLabel}
              onDoubleClick={(e) => {
                e.stopPropagation();
                if (!w.ratio) onLayout(layout.map((i) => (i.id === w.id ? { id: i.id, x: i.x, y: i.y, w: i.w, h: i.h } : i)));
              }}
              {...gestureProps('resize', w.id)}
            />
            {w.ratio ? null : (
              <div
                className="dash-edge"
                role="separator"
                aria-orientation="vertical"
                aria-label={widthLabel}
                title={widthLabel}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  const def = defaultWidths[w.id];
                  if (def !== undefined) commit(resizeWidth(settled, w.id, def, specs), w.id, false);
                }}
                {...gestureProps('width', w.id)}
              >
                <i />
              </div>
            )}
          </>
        )}
      </section>
    </div>
  );

  if (narrow) {
    const order = [...(settled.length > 0 ? settled : present)].sort((a, b) => a.y - b.y || a.x - b.x).map((i) => i.id);
    const rank = (id: string) => (order.includes(id) ? order.indexOf(id) : 99);
    return (
      <div className="dash dash--stack" ref={box}>
        {[...widgets].sort((a, b) => rank(a.id) - rank(b.id)).map((w) => card(w))}
      </div>
    );
  }

  const rows = shown.length === 0 ? 0 : Math.max(...shown.map((i) => rect(i).top + rect(i).height));
  const floor = gesture ? heightFloor.current : 0;
  const dragged = gesture && gesture.kind === 'move' && gesture.started ? gesture : null;

  const ph = dragged ? shown.find((i) => i.id === dragged.id) : undefined;

  return (
    <div className={`dash${gesture?.started ? ' dash--busy' : ''}`} ref={box} style={{ height: Math.max(rows, floor) }}>
      {ph ? <div className="dash-ph" style={rect(ph)} /> : null}
      {widgets.map((w) => {
        const it = shown.find((i) => i.id === w.id);
        if (!it) return null;
        if (dragged && dragged.id === w.id) return card(w, { left: dragged.base.left + dragged.dx, top: Math.max(0, dragged.base.top + dragged.dy), width: dragged.base.width, height: dragged.base.height }, true);
        return card(w, rect(it));
      })}
    </div>
  );
}
