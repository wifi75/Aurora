import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { postWefaxLineStart, postWefaxRotate, postWefaxSlant, wefaxAudioUrl, wefaxImageUrl, type WefaxImageEntry, type WefaxSlantPoints } from '../../api/client';
import { useT } from '../../i18n';
import { fitBoxScale, stepZoom } from '../../lib/wefax';
import { askConfirm } from '../ConfirmHost';
import { LineStartOverlay } from './LineStartOverlay';
import { SlantOverlay } from './SlantOverlay';
import '../../styles/wefax-ruota.css';
import '../../styles/wefax-grande.css';

/** Data e ora UTC della cartina: «2026-11-21 09:30Z». */
export function whenUtc(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}Z`;
}

type Zoom = 'fit' | number;

/** Dove va rimesso il punto che stava sotto il cursore dopo uno zoom: frazione dell'immagine e distanza dal bordo del riquadro. */
interface Anchor {
  fx: number;
  fy: number;
  px: number;
  py: number;
}

/** Un campo di testo (o simile) tiene per se' i tasti: le scorciatoie della finestra non devono rubarli. */
function isTyping(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  if (!el || !el.tagName) return false;
  const tag = el.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable === true;
}

/** Lo schermo intero si puo' chiedere (l'API c'e' e non e' vietata). */
function canFullscreen(): boolean {
  return typeof document !== 'undefined' && typeof document.documentElement.requestFullscreen === 'function' && document.fullscreenEnabled !== false;
}

/**
 * La cartina aperta in grande: si apre adattata (tutta visibile), con zoom (pulsanti, Ctrl + rotella, doppio clic fra adatta e 1:1),
 * spostamento con il mouse (trascinando) o con il dito, schermo intero, rotazione, misura dell'inclinazione, scarico di immagine e audio.
 * Tasti: + − zoom, 0 adatta, 1 pixel reali, F schermo intero, R / L ruota, Esc chiude.
 */
export function Lightbox({ entry, onClose, onDelete, onFixed, live = null }: { entry: WefaxImageEntry; onClose: () => void; onDelete: () => void; onFixed: (e: WefaxImageEntry) => void; /** Righe della cartina che sta arrivando adesso (`null`: nessuna ricezione): la ricezione non si ferma mai guardando le cartine salvate. */ live?: number | null }) {
  const t = useT();
  const root = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState<Zoom>('fit');
  const [view, setView] = useState({ w: 0, h: 0 });
  const [fs, setFs] = useState(false);
  const [fsBroken, setFsBroken] = useState(false);
  const [dragging, setDragging] = useState(false);
  // misura dell'inclinazione a due clic sulla cartina salvata: si raddrizza e si salva come nuova copia (o si sostituisce, dopo conferma)
  const [measuring, setMeasuring] = useState(false);
  const [placing, setPlacing] = useState(false);
  const [replace, setReplace] = useState(false);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  // una cartina sostituita ha lo stesso indirizzo: il numero fa ricaricare l'immagine
  const [version, setVersion] = useState(0);
  const src = `${wefaxImageUrl(entry.id)}${version > 0 ? `?v=${version}` : ''}`;
  const anchor = useRef<Anchor | null>(null);
  const drag = useRef<{ x: number; y: number; left: number; top: number } | null>(null);

  // la misura del riquadro segue la finestra, lo schermo intero e le barre di scorrimento
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const measureBox = () => setView({ w: el.clientWidth, h: el.clientHeight });
    measureBox();
    window.addEventListener('resize', measureBox);
    if (typeof ResizeObserver === 'undefined') return () => window.removeEventListener('resize', measureBox);
    const ro = new ResizeObserver(measureBox);
    ro.observe(el);
    return () => {
      window.removeEventListener('resize', measureBox);
      ro.disconnect();
    };
  }, []);

  const fitScale = fitBoxScale(view.w, view.h, entry.width, entry.height);
  const scale = zoom === 'fit' ? fitScale : zoom;
  const sheetW = Math.max(1, Math.floor(entry.width * scale));
  const sheetH = Math.max(1, Math.floor(entry.height * scale));
  const overflows = sheetW > view.w + 1 || sheetH > view.h + 1;

  // dopo uno zoom il punto sotto il cursore (o il centro) resta dov'era
  useLayoutEffect(() => {
    const a = anchor.current;
    const el = scroller.current;
    anchor.current = null;
    if (!a || !el) return;
    el.scrollLeft = Math.max(0, a.fx * sheetW - a.px);
    el.scrollTop = Math.max(0, a.fy * sheetH - a.py);
  }, [scale, sheetW, sheetH]);

  /** Cambia lo zoom tenendo fermo il punto dato (coordinate dello schermo) o, senza, il centro del riquadro. */
  const zoomTo = useCallback(
    (z: Zoom, at?: { x: number; y: number }) => {
      const el = scroller.current;
      if (el && z !== 'fit') {
        const r = el.getBoundingClientRect();
        const px = at ? at.x - r.left : el.clientWidth / 2;
        const py = at ? at.y - r.top : el.clientHeight / 2;
        const w = Math.max(1, entry.width * scale);
        const h = Math.max(1, entry.height * scale);
        // il foglio piu' piccolo del riquadro sta al centro: la sua origine non e' quella del riquadro
        const ox = Math.max(0, (el.clientWidth - w) / 2);
        const oy = Math.max(0, (el.clientHeight - h) / 2);
        anchor.current = { fx: Math.min(1, Math.max(0, (el.scrollLeft + px - ox) / w)), fy: Math.min(1, Math.max(0, (el.scrollTop + py - oy) / h)), px, py };
      }
      setZoom(z);
    },
    [entry.width, entry.height, scale],
  );
  const step = useCallback((dir: 1 | -1, at?: { x: number; y: number }) => zoomTo(stepZoom(scale, dir), at), [zoomTo, scale]);

  // Ctrl + rotella: zoom (serve un ascoltatore non passivo per fermare lo zoom della pagina)
  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const wheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      step(e.deltaY < 0 ? 1 : -1, { x: e.clientX, y: e.clientY });
    };
    el.addEventListener('wheel', wheel, { passive: false });
    return () => el.removeEventListener('wheel', wheel);
  }, [step]);

  // schermo intero sull'elemento della finestra grande
  useEffect(() => {
    const el = root.current;
    const sync = () => setFs(!!el && document.fullscreenElement === el);
    document.addEventListener('fullscreenchange', sync);
    return () => {
      document.removeEventListener('fullscreenchange', sync);
      if (el && document.fullscreenElement === el) void document.exitFullscreen().catch(() => undefined);
    };
  }, []);
  const toggleFullscreen = useCallback(async () => {
    const el = root.current;
    if (!el || !canFullscreen()) return;
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await el.requestFullscreen();
    } catch {
      setFsBroken(true);
    }
  }, []);

  // ruota di 90 gradi e salva subito, al posto dell'originale (nessuna conferma: ogni clic gira ancora di un quarto); poi si riadatta
  const rotate = useCallback(
    async (dir: 'left' | 'right') => {
      if (busy) return;
      setMeasuring(false);
      setBusy(true);
      try {
        const e = await postWefaxRotate(entry.id, dir);
        setNote(null);
        setVersion((v) => v + 1);
        setZoom('fit');
        onFixed(e);
      } catch {
        setNote({ ok: false, text: t('wefax.rotate.error') });
      } finally {
        setBusy(false);
      }
    },
    [busy, entry.id, onFixed, t],
  );
  const measure = async (p: WefaxSlantPoints) => {
    setMeasuring(false);
    if (replace && !(await askConfirm(t('wefax.slant.replaceAsk')))) return;
    setBusy(true);
    try {
      const r = await postWefaxSlant({ points: p, id: entry.id, replace });
      setNote({ ok: true, text: t(replace ? 'wefax.slant.replaced' : 'wefax.slant.copied', { ppm: String(Math.round(r.ppm)) }) });
      if (r.image) {
        setVersion((v) => v + 1);
        onFixed(r.image);
      }
    } catch {
      setNote({ ok: false, text: t('wefax.slant.error') });
    } finally {
      setBusy(false);
    }
  };

  const placeStart = async (x: number) => {
    setPlacing(false);
    if (replace && !(await askConfirm(t('wefax.slant.replaceAsk')))) return;
    setBusy(true);
    try {
      const r = await postWefaxLineStart({ x, id: entry.id, replace });
      setNote({ ok: true, text: t(replace ? 'wefax.start.replaced' : 'wefax.start.copied') });
      if (r.image) {
        setVersion((v) => v + 1);
        onFixed(r.image);
      }
    } catch {
      setNote({ ok: false, text: t('wefax.start.error') });
    } finally {
      setBusy(false);
    }
  };

  // il fuoco alla finestra: i tasti funzionano subito
  useEffect(() => {
    root.current?.focus({ preventScroll: true });
  }, []);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return;
      if (e.key === 'Escape') {
        // in schermo intero Esc esce da li' (lo fa il browser); la misura in corso si annulla senza chiudere
        if (document.fullscreenElement) return;
        if (measuring) setMeasuring(false);
        else onClose();
        return;
      }
      if (isTyping(e.target) || e.ctrlKey || e.metaKey || e.altKey) return;
      const k = e.key.toLowerCase();
      if (k === '+' || k === '=') step(1);
      else if (k === '-' || k === '_') step(-1);
      else if (k === '0') zoomTo('fit');
      else if (k === '1') zoomTo(1);
      else if (k === 'f') void toggleFullscreen();
      else if (k === 'r') void rotate('right');
      else if (k === 'l') void rotate('left');
      else return;
      e.preventDefault();
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [onClose, measuring, step, zoomTo, toggleFullscreen, rotate]);

  // trascinare con il mouse sposta l'immagine (con il dito scorre il riquadro da solo)
  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    const el = scroller.current;
    if (!el || e.pointerType !== 'mouse' || e.button !== 0 || measuring || !overflows) return;
    drag.current = { x: e.clientX, y: e.clientY, left: el.scrollLeft, top: el.scrollTop };
    setDragging(true);
    try {
      el.setPointerCapture(e.pointerId);
    } catch {
      /* senza cattura il trascinamento vale solo dentro il riquadro */
    }
  };
  const move = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    const el = scroller.current;
    if (!d || !el) return;
    el.scrollLeft = d.left - (e.clientX - d.x);
    el.scrollTop = d.top - (e.clientY - d.y);
  };
  const up = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    drag.current = null;
    setDragging(false);
    try {
      scroller.current?.releasePointerCapture(e.pointerId);
    } catch {
      /* gia' rilasciato */
    }
  };

  const pct = Math.round(scale * 100);
  const showFs = !fsBroken && canFullscreen();
  const fullLabel = fs ? t('wefax.big.exitFullscreen') : t('wefax.big.fullscreen');
  return (
    <div ref={root} tabIndex={-1} className={`sstv-lightbox wefax-lightbox${fs ? ' wefax-lightbox--full' : ''}`} role="dialog" aria-modal="true" aria-label={whenUtc(entry.time_ms)} onClick={onClose}>
      <figure onClick={(e) => e.stopPropagation()}>
        {live !== null ? (
          <p className="wefax-lb-live" role="status">
            <span className="wefax-lb-live-dot" aria-hidden="true" />
            {t('wefax.lb.live', { rows: live })}
          </p>
        ) : null}
        <div
          ref={scroller}
          className={`wefax-lb-scroll${overflows ? ' wefax-lb-scroll--pan' : ''}${dragging ? ' wefax-lb-scroll--drag' : ''}`}
          data-testid="wefax-lb-scroll"
          aria-label={t('wefax.big.pan')}
          onPointerDown={down}
          onPointerMove={move}
          onPointerUp={up}
          onPointerCancel={up}
          onDoubleClick={(e) => {
            if (measuring) return;
            if (zoom === 'fit') zoomTo(1, { x: e.clientX, y: e.clientY });
            else zoomTo('fit');
          }}
        >
          <div className="wefax-lb-sheet" style={{ width: sheetW, height: sheetH }}>
            <img src={src} alt="" width={entry.width} height={entry.height} draggable={false} className={`wefax-lb-img${scale >= 1 ? ' wefax-lb-img--crisp' : ''}`} style={{ width: sheetW, height: sheetH }} />
            {measuring ? <SlantOverlay width={entry.width} height={entry.height} onMeasure={(p) => void measure(p)} onCancel={() => setMeasuring(false)} /> : null}
            {placing ? <LineStartOverlay width={entry.width} height={entry.height} onPick={(x) => void placeStart(x)} onCancel={() => setPlacing(false)} /> : null}
          </div>
        </div>
        {note ? (
          <p className={`wefax-slant-note${note.ok ? '' : ' wefax-slant-note--bad'}`} role="status">
            {note.text}
          </p>
        ) : null}
        <figcaption>
          <span className="wefax-big-info">
            <b>{whenUtc(entry.time_ms)}</b> · IOC {entry.ioc} · {entry.width}×{entry.height}
          </span>
          <div className="wefax-big-tools" role="toolbar" aria-label={t('wefax.big.tools')}>
            <span className="wefax-big-group" role="group" aria-label={t('wefax.big.zoom')} title={t('wefax.big.keysHint')}>
              <button type="button" className="glass sstv-btn wefax-big-btn" onClick={() => step(-1)} title={t('wefax.view.zoomOut')} aria-label={t('wefax.view.zoomOut')}>
                −
              </button>
              <output className="wefax-big-pct" aria-live="off" data-testid="wefax-lb-pct">
                {pct} %
              </output>
              <button type="button" className="glass sstv-btn wefax-big-btn" onClick={() => step(1)} title={t('wefax.view.zoomIn')} aria-label={t('wefax.view.zoomIn')}>
                +
              </button>
              <button type="button" className={`glass sstv-btn wefax-big-btn${zoom === 'fit' ? ' sstv-btn--primary' : ''}`} aria-pressed={zoom === 'fit'} onClick={() => zoomTo('fit')} title={t('wefax.big.fitHint')}>
                {t('wefax.view.fit')}
              </button>
              <button type="button" className={`glass sstv-btn wefax-big-btn${zoom === 1 ? ' sstv-btn--primary' : ''}`} aria-pressed={zoom === 1} onClick={() => zoomTo(1)} title={t('wefax.view.actualHint')}>
                {t('wefax.view.actual')}
              </button>
            </span>
            {showFs ? (
              <button type="button" className="glass sstv-btn wefax-big-btn wefax-big-fs" aria-pressed={fs} onClick={() => void toggleFullscreen()} title={fullLabel} aria-label={fullLabel}>
                <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                  {fs ? (
                    <path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
                  ) : (
                    <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
                  )}
                </svg>
              </button>
            ) : null}
            <span className="wefax-rotate" role="group" aria-label={t('wefax.rotate.group')}>
              <button type="button" className="glass sstv-btn wefax-rotate-btn" disabled={busy} title={t('wefax.rotate.left')} aria-label={t('wefax.rotate.left')} onClick={() => void rotate('left')}>
                <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                  <path d="M9 4L5 8l4 4M5 8h8a6 6 0 1 1-6 6" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </button>
              <button type="button" className="glass sstv-btn wefax-rotate-btn" disabled={busy} title={t('wefax.rotate.right')} aria-label={t('wefax.rotate.right')} onClick={() => void rotate('right')}>
                <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                  <path d="M15 4l4 4-4 4M19 8h-8a6 6 0 1 0 6 6" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </button>
            </span>
            <button type="button" className={`glass sstv-btn${measuring ? ' sstv-btn--primary' : ''}`} disabled={busy} aria-pressed={measuring} title={t('wefax.slant.measureTip')} onClick={() => setMeasuring((m) => !m)}>
              {t('wefax.slant.measure')}
            </button>
            <button type="button" className={`glass sstv-btn${placing ? ' sstv-btn--primary' : ''}`} disabled={busy} aria-pressed={placing} title={t('wefax.start.tip')} onClick={() => setPlacing((m) => !m)}>
              {t('wefax.start.button')}
            </button>
            <label className="sstv-check" title={t('wefax.slant.replaceTip')}>
              <input type="checkbox" checked={replace} onChange={(e) => setReplace(e.target.checked)} />
              {t('wefax.slant.replace')}
            </label>
            <a className="glass sstv-btn sstv-btn--primary" href={src} download={entry.file}>
              {t('wefax.hist.download')}
            </a>
            {entry.has_audio ? (
              <a className="glass sstv-btn wefax-big-audio" href={wefaxAudioUrl(entry.id)} download={entry.audio} title={t('wefax.big.audio')} aria-label={t('wefax.big.audio')}>
                {t('wefax.big.audio')}
              </a>
            ) : null}
            <button type="button" className="glass sstv-btn sstv-btn--danger" onClick={onDelete}>
              {t('wefax.hist.delete')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={onClose}>
              {t('wefax.hist.close')}
            </button>
          </div>
        </figcaption>
      </figure>
    </div>
  );
}
