import { useEffect, useRef, useState, type ReactNode } from 'react';

/**
 * Le schede dei modi testuali (CW, RTTY, PSK) nel contest: la finestra di ricezione, quella di trasmissione e la cascata sono gli stessi elementi
 * della pagina senza contest (decodifica, menu, ricerca, macro, tasti), messi dentro una scheda. Il contenitore ha la classe `digi-windows`
 * perche' gli stili delle finestre (sfondo, caratteri, colori dei temi) sono scritti per quella.
 */
export function RxTextCard({ children }: { children: ReactNode }) {
  return <div className="digi-windows ctw-win ctw-win--rx">{children}</div>;
}

export function TxTextCard({ children }: { children: ReactNode }) {
  return <div className="digi-windows ctw-win ctw-win--tx">{children}</div>;
}

/** L'altezza della cascata se la scheda non si e' ancora misurata. */
const WF_FALLBACK = 150;
/** Minimo e margine: la cascata ha la sua scala in alto e non scende sotto questa altezza. */
const WF_MIN_H = 70;

/**
 * La cascata nella scheda: prende tutta l'altezza e la larghezza della scheda (si ridisegna quando la scheda cambia misura); le maniglie di
 * misura della cascata non ci sono, la misura la da' la scheda. `render(h)` disegna la cascata alta `h` pixel.
 */
export function WaterfallCard({ render }: { render: (heightPx: number) => ReactNode }) {
  const host = useRef<HTMLDivElement>(null);
  const [h, setH] = useState(WF_FALLBACK);
  useEffect(() => {
    const el = host.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const measure = () => {
      const px = Math.floor(el.clientHeight);
      if (px > 0) setH(Math.max(WF_MIN_H, px));
    };
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    measure();
    return () => ro.disconnect();
  }, []);
  return (
    <div className="ctw-wfhost" ref={host}>
      {render(h)}
    </div>
  );
}
