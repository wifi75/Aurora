import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { useT } from '../i18n';
import { browserStorage } from '../lib/theme';
import { RadioDisplayHeader } from './radio-display/RadioDisplayHeader';

const KEY = 'aurora-display-scala';

function readScale(): number | null {
  try {
    const v = Number(browserStorage()?.getItem(KEY));
    return Number.isFinite(v) && v >= 0.3 && v <= 8 ? v : null;
  } catch {
    return null;
  }
}

function saveScale(v: number | null): void {
  try {
    if (v === null) browserStorage()?.removeItem(KEY);
    else browserStorage()?.setItem(KEY, String(v));
  } catch {
    // senza archivio la misura vale finche' la finestra resta aperta
  }
}

/**
 * Il display della radio in una finestra a parte. Di partenza si adatta alla finestra; con la maniglia nel suo angolo in basso a destra lo
 * si ridimensiona da solo, mantenendo le proporzioni e senza perdere qualita' (CSS `zoom`: testi e disegni si ridisegnano alla misura
 * nuova). Doppio clic sulla maniglia: torna ad adattarsi alla finestra. La misura scelta si ricorda.
 */
export function DisplayFit() {
  const t = useT();
  const box = useRef<HTMLDivElement | null>(null);
  const inner = useRef<HTMLDivElement | null>(null);
  const [fitScale, setFitScale] = useState(1);
  const [manual, setManual] = useState<number | null>(readScale);
  const scale = manual ?? fitScale;

  // la misura del display a grandezza 1: quella disegnata divisa per lo zoom gia' applicato
  const natural = () => {
    const content = inner.current;
    if (!content) return null;
    const z = Number(content.style.zoom || '1') || 1;
    const r = content.getBoundingClientRect();
    return r.width < 10 || r.height < 10 ? null : { w: r.width / z, h: r.height / z };
  };

  useEffect(() => {
    const outer = box.current;
    if (!outer) return;
    const fit = () => {
      const n = natural();
      if (!n || outer.clientWidth < 10 || outer.clientHeight < 10) return;
      const next = Math.max(0.3, Math.min(outer.clientWidth / n.w, outer.clientHeight / n.h) * 0.98);
      setFitScale((old) => (Math.abs(old - next) > 0.01 ? next : old));
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(outer);
    // il display cambia misura da solo (radio collegata, altro stile): si riprova ogni tanto
    const timer = window.setInterval(fit, 2000);
    return () => {
      ro.disconnect();
      window.clearInterval(timer);
    };
  }, []);

  const startResize = (e: ReactPointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    const n = natural();
    const rect = inner.current?.getBoundingClientRect();
    if (!n || !rect) return;
    const handle = e.currentTarget;
    handle.setPointerCapture(e.pointerId);
    let last = scale;
    const move = (ev: PointerEvent) => {
      // la nuova misura segue l'angolo, con le stesse proporzioni (vince il lato tirato di piu')
      const w = ev.clientX - rect.left;
      const h = ev.clientY - rect.top;
      last = Math.max(0.3, Math.min(8, Math.max(w / n.w, h / n.h)));
      setManual(last);
    };
    const stop = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      saveScale(last);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
  };

  return (
    <div ref={box} className={`display-fit${manual !== null ? ' display-fit--manual' : ''}`}>
      <div className="display-fit-frame">
        <div ref={inner} className="display-fit-inner" style={{ zoom: scale }}>
          <RadioDisplayHeader solo />
        </div>
        <div
          className="display-fit-handle"
          role="separator"
          title={t('win.displaySize')}
          aria-label={t('win.displaySize')}
          onPointerDown={startResize}
          onDoubleClick={() => {
            setManual(null);
            saveScale(null);
          }}
        />
      </div>
    </div>
  );
}
