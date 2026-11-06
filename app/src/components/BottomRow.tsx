import { useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { useT } from '../i18n';
import { BOTTOM_MAX_HEIGHT, BOTTOM_MIN_HEIGHT, EDIT_LAYOUT, useBottom, type BottomKey } from '../lib/bottomLayout';

export interface BottomPanel {
  title: string;
  body: ReactNode;
  /** Classe in piu' per lo stile del pannello. */
  extra?: string;
}

/**
 * La riga in basso: cascata, Trasmissione (messaggi) e QSO come tre finestre separate. Si regolano a mano: l'altezza dal bordo alto, la
 * larghezza dai divisori fra le finestre e l'ordine trascinando la barra del titolo. La disposizione si salva da sola.
 */
export function BottomRow({ panels, hidden }: { panels: Record<BottomKey, BottomPanel>; hidden: readonly BottomKey[] }) {
  const t = useT();
  const layout = useBottom((s) => s.layout);
  const row = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState<BottomKey | null>(null);
  const shown = layout.order.filter((k) => !hidden.includes(k));
  if (shown.length === 0) return null;
  const share = (k: BottomKey) => layout.widths[layout.order.indexOf(k)] ?? 0.33;

  /** Altezza della riga: si trascina il bordo alto. */
  const startHeight = (e: ReactPointerEvent<HTMLDivElement>) => {
    const handle = e.currentTarget;
    handle.setPointerCapture(e.pointerId);
    const y0 = e.clientY;
    const h0 = layout.heightPx;
    const move = (ev: PointerEvent) => useBottom.getState().set({ heightPx: Math.min(BOTTOM_MAX_HEIGHT, Math.max(BOTTOM_MIN_HEIGHT, h0 + (y0 - ev.clientY))) });
    const stop = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  };

  /** Larghezza: il divisore fra due finestre sposta la quota dall'una all'altra. */
  const startSplit = (left: BottomKey, right: BottomKey) => (e: ReactPointerEvent<HTMLDivElement>) => {
    const box = row.current?.getBoundingClientRect();
    const leftEl = row.current?.querySelector<HTMLElement>(`[data-panel="${left}"]`);
    if (!box || !leftEl) return;
    const handle = e.currentTarget;
    handle.setPointerCapture(e.pointerId);
    const widths = [...layout.widths] as [number, number, number];
    const a = layout.order.indexOf(left);
    const b = layout.order.indexOf(right);
    const sum = widths[a]! + widths[b]!;
    const leftEdge = leftEl.getBoundingClientRect().left;
    const move = (ev: PointerEvent) => {
      const fraction = (ev.clientX - leftEdge) / Math.max(1, box.width);
      const next = [...widths] as [number, number, number];
      next[a] = Math.min(sum - 0.12, Math.max(0.12, fraction));
      next[b] = sum - next[a]!;
      useBottom.getState().set({ widths: next });
    };
    const stop = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  };

  /** Ordine: si trascina la barra del titolo sopra un'altra finestra, che prende il suo posto. */
  const startDrag = (key: BottomKey) => (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest('button')) return;
    const handle = e.currentTarget;
    handle.setPointerCapture(e.pointerId);
    setDragging(key);
    const targetAt = (x: number): BottomKey => {
      let best = key;
      let bestDistance = Infinity;
      for (const k of shown) {
        const el = row.current?.querySelector<HTMLElement>(`[data-panel="${k}"]`);
        if (!el) continue;
        const r = el.getBoundingClientRect();
        const distance = Math.abs(x - (r.left + r.width / 2));
        if (distance < bestDistance) {
          bestDistance = distance;
          best = k;
        }
      }
      return best;
    };
    const stop = (ev: PointerEvent) => {
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
      setDragging(null);
      const target = targetAt(ev.clientX);
      if (target === key) return;
      const order = [...layout.order] as typeof layout.order;
      const from = order.indexOf(key);
      const to = order.indexOf(target);
      // le quote di larghezza seguono le finestre: si scambiano insieme
      const widths = [...layout.widths] as [number, number, number];
      [order[from], order[to]] = [order[to]!, order[from]!];
      [widths[from], widths[to]] = [widths[to]!, widths[from]!];
      useBottom.getState().set({ order, widths });
    };
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  };

  return (
    <>
      {EDIT_LAYOUT ? <div className="live-bottom-resizer" role="separator" aria-orientation="horizontal" aria-label={t('bottom.height')} title={t('bottom.height')} onPointerDown={startHeight} /> : null}
      <div ref={row} className={`live-bottom${EDIT_LAYOUT ? ' live-bottom--edit' : ''}`} style={{ height: layout.heightPx }}>
        {shown.map((key, i) => {
          const p = panels[key];
          return (
            <div key={key} className="live-bottom-slot" style={{ flex: `${share(key)} 1 0` }}>
              <section className={`dock-panel dock-panel--${key} live-bottom-panel ${p.extra ?? ''}${dragging === key ? ' dock-panel--flying' : ''}`} data-panel={key}>
                <header className={`dock-grip${EDIT_LAYOUT ? '' : ' dock-grip--fixed'}`} onPointerDown={EDIT_LAYOUT ? startDrag(key) : undefined} title={EDIT_LAYOUT ? t('tip.dock.grip') : undefined}>
                  <span className="dock-grip-dots" aria-hidden="true" />
                  <h2>{p.title}</h2>
                </header>
                <div className="dock-body">{p.body}</div>
              </section>
              {EDIT_LAYOUT && i < shown.length - 1 ? <div className="live-bottom-split" role="separator" aria-orientation="vertical" aria-label={t('dock.width')} onPointerDown={startSplit(key, shown[i + 1]!)} /> : null}
            </div>
          );
        })}
      </div>
    </>
  );
}
