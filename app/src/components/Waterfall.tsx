import { useEffect, useRef, useState } from 'react';
import { useT } from '../i18n';
import { createBackend, type WaterfallBackend } from '../render/backend';
import { jtdxPaletteLut } from '../render/colormap';
import { PeriodAverage, RowPacer, cumulativeCurve, fitWidth, processRow, spectrumCurve } from '../render/jtdxRow';
import { drawSpectrum } from '../render/spectrumPlot';
import { WATERFALL_WIDTH, rowsPerPixel } from '../render/webgl2';
import { periodSecs } from '../lib/period';
import { type SpectrumListener, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { ROW_SECS, WF_LIMITS, clampWf, paletteColors, useWfPrefs } from '../lib/wfPrefs';

interface Props {
  /** Registra la funzione che riceve ogni riga di spettro; restituisce come annullare. */
  register(push: SpectrumListener): () => void;
  /** Il modo in uso (FT8, FT4...): da qui la durata del periodo per le righe verdi e per la media «Cumulative». */
  mode?: string;
  /** Telefono: lo spettro sempre visibile sotto la cascata, alto questi pixel (invece della quota `Spec %`). */
  spectrumPx?: number;
  /** La banda in uso (20m...): si scrive accanto all'ora sulla cascata. */
  band?: string;
}

/** Quota massima dell'altezza per lo spettro (%). */
const MAX_SPECTRUM_SHARE = 85;

type Phase = 'loading' | 'ready' | 'unsupported';

/**
 * La cascata e lo spettro del Wide Graph di JTDX: le righe del core (canali da 0,732 Hz) diventano pixel con le opzioni
 * delle Impostazioni → Cascata (Bins/Pixel, Start, N Avg, Scale, Flatten, tavolozza, Gain, Zero, spettro Current/Cumulative).
 */
export function Waterfall({ register, mode, spectrumPx, band }: Props) {
  const t = useT();
  const [phase, setPhase] = useState<Phase>('loading');
  const [kind, setKind] = useState('');
  // WebGPU puo' aver gia' preso il canvas: per ripiegare su WebGL2 serve un canvas nuovo
  const [attempt, setAttempt] = useState<{ forced?: 'webgl2'; key: number }>({ key: 0 });
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const plotRef = useRef<HTMLCanvasElement>(null);
  const backendRef = useRef<WaterfallBackend | null>(null);
  const prefs = useWfPrefs((s) => s.prefs);
  const spanHz = useSpectrumSpan((s) => s.spanHz);
  const range = visibleRange(prefs.startHz, spanHz);
  // come `if(m_h2 > m_h - 30)` di JTDX: la cascata non sparisce mai del tutto
  const specPercent = Math.min(prefs.spectrumPercent, MAX_SPECTRUM_SHARE);

  // le opzioni correnti per il ricevitore delle righe, che vive per tutta la vita del renderer
  const live = useRef({ prefs, from: range.from, to: range.to, mode, band });
  live.current = { prefs, from: range.from, to: range.to, mode, band };
  // «Time stamp»: l'ora di ogni inizio di periodo scende con la cascata; `age` conta le righe arrivate dopo
  const stampsRef = useRef<HTMLCanvasElement>(null);
  const marks = useRef<Array<{ label: string; age: number }>>([]);
  const drawStamps = () => {
    const stamps = stampsRef.current;
    const main = canvasRef.current;
    if (!stamps || !main) return;
    const dpr = window.devicePixelRatio || 1;
    stamps.style.top = `${main.offsetTop}px`;
    stamps.style.height = `${main.clientHeight}px`;
    const w = Math.max(1, Math.round(main.clientWidth * dpr));
    const h = Math.max(1, Math.round(main.clientHeight * dpr));
    if (stamps.width !== w) stamps.width = w;
    if (stamps.height !== h) stamps.height = h;
    const ctx = stamps.getContext('2d');
    if (!ctx) return;
    ctx.clearRect(0, 0, w, h);
    if (!live.current.prefs.timeStamp) return;
    const rpp = rowsPerPixel(main.height);
    ctx.font = `${Math.round(11 * dpr)}px 'IBM Plex Sans', system-ui, sans-serif`;
    ctx.textBaseline = 'top';
    for (const m of marks.current) {
      const y = Math.round((m.age / rpp) * (h / Math.max(1, main.height)));
      if (y > h) continue;
      const pad = 3 * dpr;
      const tw = ctx.measureText(m.label).width;
      ctx.fillStyle = 'rgba(0, 0, 0, 0.55)';
      ctx.fillRect(4 * dpr, y + 2 * dpr, tw + pad * 2, 14 * dpr);
      ctx.fillStyle = '#e8f1ff';
      ctx.fillText(m.label, 4 * dpr + pad, y + 4 * dpr);
    }
  };
  // ogni riga nuova allontana di una riga i segni gia' scritti; quelli fuori dall'altezza si tolgono
  const bumpStamps = (startsPeriod: boolean) => {
    for (const m of marks.current) m.age += 1;
    if (startsPeriod) {
      const hhmmss = new Date().toISOString().slice(11, 19);
      marks.current.push({ label: live.current.band ? `${hhmmss}  ${live.current.band}` : hhmmss, age: 0 });
    }
    const limit = (canvasRef.current?.height ?? 600) * 2;
    marks.current = marks.current.filter((m) => m.age <= limit).slice(-40);
    drawStamps();
  };
  const curveRef = useRef<Float32Array | null>(null);
  const drawPlot = () => {
    const canvas = plotRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    const w = Math.max(1, Math.round(rect.width * dpr));
    const h = Math.max(1, Math.round(rect.height * dpr));
    if (canvas.width !== w) canvas.width = w;
    if (canvas.height !== h) canvas.height = h;
    drawSpectrum(canvas, curveRef.current, live.current.from, live.current.to);
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let cancelled = false;
    let backend: WaterfallBackend | null = null;
    let unregister = () => {};
    let observer: ResizeObserver | null = null;

    createBackend(canvas, attempt.forced)
      .then((b) => {
        if (cancelled) {
          b?.destroy();
          return;
        }
        if (!b) {
          if (!attempt.forced) setAttempt({ forced: 'webgl2', key: attempt.key + 1 });
          else setPhase('unsupported');
          return;
        }
        backend = b;
        backendRef.current = b;
        b.setPalette(jtdxPaletteLut(paletteColors(live.current.prefs)));
        b.setTone(live.current.prefs.gain, live.current.prefs.zero);
        setKind(b.kind);
        setPhase('ready');
        const fit = () => {
          const dpr = window.devicePixelRatio || 1;
          const rect = canvas.getBoundingClientRect();
          b.resize(Math.round(rect.width * dpr), Math.round(rect.height * dpr));
        };
        fit();
        observer = new ResizeObserver(() => {
          fit();
          drawPlot();
          drawStamps();
        });
        observer.observe(canvas);
        if (plotRef.current) observer.observe(plotRef.current);

        const pacer = new RowPacer();
        const periodAverage = new PeriodAverage();
        let lastPhase = 0;
        // l'inizio del periodo resta in attesa finche' non e' pronta una riga (con WSJT-X una ogni 0,288 s)
        let pendingWrap = false;
        unregister = register((bins, fMinHz, fMaxHz) => {
          const { prefs: p, from, to, mode: m } = live.current;
          // inizio del periodo (come `tr < m_tr0` di WideGraph::dataSink2): una riga verde al posto dei dati
          const period = periodSecs(m ?? 'FT8');
          const inPeriod = (Date.now() / 1000) % period;
          const wrapped = inPeriod < lastPhase;
          lastPhase = inPeriod;
          if (wrapped) periodAverage.reset();
          if (wrapped) pendingWrap = true;
          periodAverage.add(bins);
          const average = pacer.add(bins, ROW_SECS[p.speed], p.navg);
          if (!average) return;
          const startsPeriod = pendingWrap;
          pendingWrap = false;
          // la riga si puo' spegnere (Impostazioni > Cascata): il periodo nuovo comincia comunque
          if (startsPeriod && p.periodLine) {
            b.pushRow(new Uint8Array(1), 1);
            bumpStamps(true);
            return;
          }
          const binWidthHz = (fMaxHz - fMinHz) / bins.length;
          const options = { binsPerPixel: p.binsPerPixel, startHz: from, endHz: to, scale: p.scale, flatten: p.flatten };
          const row = processRow(average, binWidthHz, options);
          if (row.codes.length === 0) return;
          b.pushRow(fitWidth(row.codes, WATERFALL_WIDTH));
          bumpStamps(false);
          if (Math.min(p.spectrumPercent, MAX_SPECTRUM_SHARE) > 0) {
            const cumulative = p.spectrumMode === 'cumulative' ? periodAverage.average() : null;
            const values = cumulative ? cumulativeCurve(cumulative, binWidthHz, options) : row.y;
            curveRef.current = spectrumCurve(values, cumulative ? 'cumulative' : 'current', p.gain2d, p.zero2d, p.flatten);
            drawPlot();
          }
        });
      })
      .catch(() => {
        if (!cancelled) setPhase('unsupported');
      });

    return () => {
      cancelled = true;
      backendRef.current = null;
      unregister();
      observer?.disconnect();
      backend?.destroy();
    };
  }, [register, attempt]);

  // Gain, Zero e tavolozza valgono subito su tutta la cascata gia' disegnata
  useEffect(() => {
    backendRef.current?.setTone(prefs.gain, prefs.zero);
  }, [prefs.gain, prefs.zero, phase]);
  const colors = paletteColors(prefs);
  useEffect(() => {
    backendRef.current?.setPalette(jtdxPaletteLut(colors));
  }, [colors, phase]);
  // il grafico si ridisegna anche se non arriva una riga (altezza, Start, ampiezza)
  useEffect(() => {
    drawPlot();
  }, [prefs.spectrumPercent, range.from, range.to]);

  const quick = (name: 'gain' | 'zero') => (
    <label key={name} className="wf-slider" title={t(name === 'gain' ? 'tip.ui.wfGain' : 'tip.ui.wfZero')}>
      <span>{`${t(`wf.${name}`)} ${prefs[name]}`}</span>
      <input
        type="range"
        min={WF_LIMITS[name][0]}
        max={WF_LIMITS[name][1]}
        step={1}
        value={prefs[name]}
        aria-label={t(`wf.${name}`)}
        onChange={(e) => useWfPrefs.getState().set({ [name]: clampWf(name, Number(e.target.value)) })}
      />
    </label>
  );

  // il divisore fra cascata e spettro si trascina in alto e in basso: la quota dello spettro (`Spec %`) segue il mouse e resta salvata
  const startSpectrumResize = (e: React.PointerEvent<HTMLDivElement>) => {
    const box = boxRef.current;
    if (!box) return;
    const handle = e.currentTarget;
    try {
      handle.setPointerCapture(e.pointerId);
    } catch {
      // senza cattura il divisore funziona lo stesso: gli ascoltatori sono sul divisore e sulla finestra
    }
    const move = (ev: PointerEvent) => {
      const r = box.getBoundingClientRect();
      const px = Math.round(ev.clientY - r.top);
      useWfPrefs.getState().set({ mainPx: Math.min(Math.max(40, r.height - 36 - 7), Math.max(40, px)) });
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

  // la barra dei comandi sopra la cascata (non sul telefono, dove lo spazio e' poco)
  const bar = !spectrumPx;
  return (
    <div ref={boxRef} className={`waterfall${bar ? ' waterfall--bar' : ''}`} data-backend={kind} data-phase={phase}>
      <canvas key={attempt.key} ref={canvasRef} className="wf-main" style={{ flex: spectrumPx ? '1 1 0px' : bar ? `0 1 ${prefs.mainPx}px` : `${100 - specPercent} 1 0px` }} role="img" aria-label={t('wf.aria')} title={t('tip.panel.waterfall')} />
      {bar && specPercent > 0 ? (
        <div
          className="wf-splitter"
          role="separator"
          aria-orientation="horizontal"
          aria-label={t('wf.specHeight')}
          title={t('wf.specHeight')}
          onPointerDown={startSpectrumResize}
        />
      ) : null}
      <canvas
        ref={plotRef}
        className="wf-spectrum"
        aria-hidden="true"
        style={spectrumPx ? { flex: `0 0 ${spectrumPx}px`, display: 'block' } : { flex: bar ? '1 1 0px' : `${specPercent} 1 0px`, display: specPercent > 0 ? 'block' : 'none' }}
      />
      <canvas ref={stampsRef} className="wf-stamps" aria-hidden="true" />
      {phase === 'unsupported' && <p className="waterfall-note">{t('wf.unsupported')}</p>}
      <div className="wf-controls">
        {quick('gain')}
        {quick('zero')}
      </div>
    </div>
  );
}
