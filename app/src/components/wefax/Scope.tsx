import { useEffect, useRef } from 'react';
import { useT } from '../../i18n';
import { RowAverager } from '../../lib/digiSpectrum';
import { registerSpectrum } from '../../lib/spectrumFeed';
import { DigiWaterfall } from '../digi/DigiWaterfall';
import '../../styles/digi.css';

/** Il tratto dell'audio mostrato (Hz): il fax (1500-2300 Hz) sta nel mezzo. */
export const SCOPE_MIN_HZ = 1000;
export const SCOPE_MAX_HZ = 3000;
/** La cascata colorata e' bassa (la scala in Hz e' nascosta e si somma ai 22 px che la scala occuperebbe): conta di piu' la curva che si muove. */
const WATERFALL_H = 36 + 22;

/** Colori dal tema (lo spettro segue il chiaro e lo scuro dell'app). */
function themeColor(el: Element, name: string, fallback: string): string {
  const v = getComputedStyle(el).getPropertyValue(name).trim();
  return v === '' ? fallback : v;
}

/**
 * Spettro e cascata della pagina WEFAX: in alto la curva dell'audio con due righe rosse sul nero e sul bianco (centro
 * ± escursione/2, spostati dall'AFC) e una riga scura sul centro; sotto la cascata con il righello in Hz e la coppia di segni. Un clic
 * su spettro o cascata porta il centro li'.
 */
export function WefaxScope({ centerHz, shiftHz, afcHz, onCenter }: { centerHz: number; shiftHz: number; afcHz: number; onCenter(hz: number): void }) {
  const t = useT();
  const canvas = useRef<HTMLCanvasElement>(null);
  const avg = useRef(new RowAverager());
  const row = useRef<{ fMinHz: number; fMaxHz: number; n: number } | null>(null);
  const rx = centerHz + afcHz;
  // il campo dei valori mostrato dalla curva (come la cascata, che esalta le differenze): fondo e picco si seguono con una media lenta
  const range = useRef<{ lo: number; hi: number } | null>(null);
  const marks = useRef({ rx, shiftHz });
  marks.current = { rx, shiftHz };

  const draw = () => {
    const c = canvas.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.round(c.clientWidth * dpr));
    const h = Math.max(1, Math.round(c.clientHeight * dpr));
    if (c.width !== w) c.width = w;
    if (c.height !== h) c.height = h;
    const xOf = (hz: number) => ((hz - SCOPE_MIN_HZ) / (SCOPE_MAX_HZ - SCOPE_MIN_HZ)) * w;
    ctx.fillStyle = themeColor(c, '--panel-raised', '#10151f');
    ctx.fillRect(0, 0, w, h);
    // la curva: media delle ultime righe di spettro
    const values = avg.current.values();
    const r = row.current;
    if (r && values.length > 0) {
      const binHz = (r.fMaxHz - r.fMinHz) / Math.max(1, values.length);
      // fondo = 5% dei valori della parte mostrata, picco = il massimo: la curva riempie la striscia e segue i colori della cascata sotto
      const first = Math.max(0, Math.floor((SCOPE_MIN_HZ - r.fMinHz) / binHz));
      const last = Math.min(values.length, Math.ceil((SCOPE_MAX_HZ - r.fMinHz) / binHz));
      const shown = Array.from(values.slice(first, Math.max(first + 1, last))).sort((a, b) => a - b);
      const lo0 = shown[Math.floor(shown.length * 0.05)] ?? 0;
      const hi0 = Math.max(lo0 + 24, shown[shown.length - 1] ?? 255);
      const prev = range.current;
      range.current = prev ? { lo: prev.lo + (lo0 - prev.lo) * 0.2, hi: prev.hi + (hi0 - prev.hi) * 0.2 } : { lo: lo0, hi: hi0 };
      const { lo, hi } = range.current;
      ctx.strokeStyle = '#3b82f6';
      ctx.lineWidth = Math.max(1, dpr);
      ctx.beginPath();
      for (let x = 0; x < w; x++) {
        const hz = SCOPE_MIN_HZ + (x / w) * (SCOPE_MAX_HZ - SCOPE_MIN_HZ);
        const v = values[Math.min(values.length - 1, Math.max(0, Math.floor((hz - r.fMinHz) / binHz)))] ?? 0;
        const y = h - 2 * dpr - Math.min(1, Math.max(0, (v - lo) / (hi - lo))) * (h - 4 * dpr);
        if (x === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
    }
    const { rx: centre, shiftHz: shift } = marks.current;
    // la riga del centro (ricezione) e le due del nero e del bianco
    ctx.fillStyle = themeColor(c, '--text', '#e6edf7');
    ctx.globalAlpha = 0.55;
    ctx.fillRect(Math.round(xOf(centre)) - dpr, 0, 2 * dpr, h);
    ctx.globalAlpha = 1;
    ctx.fillStyle = '#ef4444';
    for (const hz of [centre - shift / 2, centre + shift / 2]) ctx.fillRect(Math.round(xOf(hz)) - dpr, 0, 2 * dpr, h);
  };

  useEffect(
    () =>
      registerSpectrum((bins, fMinHz, fMaxHz) => {
        avg.current.push({ bins, fMinHz, fMaxHz }, 4);
        row.current = { fMinHz, fMaxHz, n: bins.length };
        draw();
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
  // i segni si spostano subito, anche senza righe nuove
  useEffect(draw, [rx, shiftHz]);

  const pick = (clientX: number, el: HTMLElement) => {
    const b = el.getBoundingClientRect();
    const hz = SCOPE_MIN_HZ + ((clientX - b.left) / Math.max(1, b.width)) * (SCOPE_MAX_HZ - SCOPE_MIN_HZ);
    onCenter(Math.round(Math.min(3000, Math.max(1000, hz))));
  };

  return (
    <div className="wefax-scope">
      <canvas
        ref={canvas}
        className="wefax-scope-spec"
        role="img"
        aria-label={t('wefax.scope.aria')}
        title={t('wefax.scope.tip')}
        onClick={(e) => pick(e.clientX, e.currentTarget)}
      />
      <DigiWaterfall
        minHz={SCOPE_MIN_HZ}
        maxHz={SCOPE_MAX_HZ}
        rxHz={rx}
        txHz={null}
        bandwidthHz={shiftHz}
        transmitting={false}
        height={WATERFALL_H}
        pileup={false}
        hideScale
        onPick={(hz, _shift, commit) => {
          if (commit) onCenter(Math.round(Math.min(3000, Math.max(1000, hz))));
        }}
      />
    </div>
  );
}
