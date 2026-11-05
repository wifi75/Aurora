import { useEffect, useRef, useState } from 'react';
import { RadioDisplayHeader } from './radio-display/RadioDisplayHeader';

/**
 * Il display della radio in una finestra a parte: si ingrandisce o si rimpicciolisce con la finestra mantenendo le proporzioni, senza
 * perdere qualita' (CSS `zoom`: testi e disegni si ridisegnano alla misura nuova, non si stira un'immagine).
 */
export function DisplayFit() {
  const box = useRef<HTMLDivElement | null>(null);
  const inner = useRef<HTMLDivElement | null>(null);
  const [scale, setScale] = useState(1);
  useEffect(() => {
    const outer = box.current;
    const content = inner.current;
    if (!outer || !content) return;
    const fit = () => {
      // misura vera del display a grandezza 1: quella disegnata divisa per lo zoom gia' applicato
      const z = Number(content.style.zoom || '1') || 1;
      const r = content.getBoundingClientRect();
      const w = r.width / z;
      const h = r.height / z;
      if (w < 10 || h < 10 || outer.clientWidth < 10 || outer.clientHeight < 10) return;
      const next = Math.max(0.3, Math.min(outer.clientWidth / w, outer.clientHeight / h) * 0.98);
      setScale((old) => (Math.abs(old - next) > 0.01 ? next : old));
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
  return (
    <div ref={box} className="display-fit">
      <div ref={inner} className="display-fit-inner" style={{ zoom: scale }}>
        <RadioDisplayHeader solo />
      </div>
    </div>
  );
}
