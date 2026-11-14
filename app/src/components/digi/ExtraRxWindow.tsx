import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { useT } from '../../i18n';
import { getExtraText, onExtraText } from '../../lib/digiBus';
import { browserStorage } from '../../lib/theme';
import '../../styles/digi-extra-rx.css';

interface Props {
  id: number;
  freqHz: number;
  modeName: string;
  /** Posizione di partenza la prima volta (le finestre si mettono in scala). */
  index: number;
  /** L'AFC di questa finestra (suo, non quello della barra in fondo) e chi lo cambia. */
  afc: boolean;
  onAfc(on: boolean): void;
  onClose(): void;
  /** Doppio clic su una parola: la si prende (nominativo nel QSO). */
  onWord?(word: string): void;
}

interface Saved {
  x?: number;
  y?: number;
  w?: number;
  h?: number;
}

function readSaved(key: string): Saved | null {
  try {
    const v = JSON.parse(browserStorage()?.getItem(key) ?? 'null') as Saved | null;
    return v && typeof v === 'object' ? v : null;
  } catch {
    return null;
  }
}

/**
 * Una finestra RX in piu': piccola, si trascina dal titolo, si allarga dall'angolo in basso a destra in ogni direzione, la X la chiude;
 * posizione e misura restano ricordate (per numero di finestra). Il pulsante AFC accende l'AFC solo di questa finestra.
 */
export function ExtraRxWindow({ id, freqHz, modeName, index, afc, onAfc, onClose, onWord }: Props) {
  const t = useT();
  const text = useSyncExternalStore(onExtraText, () => getExtraText(id));
  const key = `aurora.xrx.${id}`;
  const [saved] = useState(() => readSaved(key));
  const [pos, setPos] = useState<{ x: number; y: number }>(() => ({
    x: Math.min(window.innerWidth - 120, Math.max(0, saved?.x ?? window.innerWidth - 340 - index * 28)),
    y: Math.min(window.innerHeight - 40, Math.max(0, saved?.y ?? 140 + index * 34)),
  }));
  const box = useRef<HTMLElement>(null);
  const body = useRef<HTMLPreElement>(null);
  const remember = (p: { x: number; y: number }) => {
    const el = box.current;
    try {
      browserStorage()?.setItem(key, JSON.stringify({ x: Math.round(p.x), y: Math.round(p.y), w: el?.offsetWidth, h: el?.offsetHeight }));
    } catch {
      // senza archivio la posizione vale finche' la finestra e' aperta
    }
  };
  const rememberRef = useRef(remember);
  rememberRef.current = remember;
  // la misura cambiata trascinando l'angolo si ricorda anche lei
  useEffect(() => {
    const el = box.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const ro = new ResizeObserver(() => {
      clearTimeout(timer);
      timer = setTimeout(() => rememberRef.current({ x: el.offsetLeft, y: el.offsetTop }), 300);
    });
    ro.observe(el);
    return () => {
      clearTimeout(timer);
      ro.disconnect();
    };
  }, []);
  useEffect(() => {
    const el = body.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [text]);
  const drag = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const sx = e.clientX - pos.x;
    const sy = e.clientY - pos.y;
    let last = pos;
    const move = (m: PointerEvent) => {
      last = { x: Math.min(window.innerWidth - 120, Math.max(0, m.clientX - sx)), y: Math.min(window.innerHeight - 40, Math.max(0, m.clientY - sy)) };
      setPos(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      remember(last);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  return (
    <section ref={box} className="xrx" style={{ left: pos.x, top: pos.y, ...(saved?.w ? { width: saved.w } : {}), ...(saved?.h ? { height: saved.h } : {}) }} aria-label={`RX (${id})`}>
      <header className="xrx-head" onPointerDown={drag} title={t('xrx.dragTip')}>
        <strong>RX ({id})</strong>
        <span>
          {modeName} · {Math.round(freqHz)} Hz
        </span>
        <button type="button" className={`xrx-afc${afc ? ' xrx-afc--on' : ''}`} aria-pressed={afc} title={t('xrx.afcTip')} onPointerDown={(e) => e.stopPropagation()} onClick={() => onAfc(!afc)}>
          AFC
        </button>
        <button type="button" className="xrx-close" aria-label={t('xrx.close')} title={t('xrx.close')} onPointerDown={(e) => e.stopPropagation()} onClick={onClose}>
          ×
        </button>
      </header>
      <pre
        ref={body}
        className="xrx-text"
        onDoubleClick={() => {
          const w = window.getSelection()?.toString().trim().toUpperCase() ?? '';
          if (w && onWord) onWord(w);
        }}
      >
        {text || <span className="xrx-empty">{t('xrx.empty')}</span>}
      </pre>
    </section>
  );
}
