import { useEffect, useRef } from 'react';
import { FrequencyRuler } from '../FrequencyRuler';
import { leftPercent } from '../../lib/markers';
import { registerSpectrum } from '../../lib/spectrumFeed';
import { ROW_SECS, useWfPrefs } from '../../lib/wfPrefs';
import { RowPacer, liftCurve, processRow, spectrumCurve } from '../../render/jtdxRow';
import { drawSpectrum } from '../../render/spectrumPlot';

/** Il tratto audio mostrato (Hz): VIS a 1100/1300, sincronismo a 1200, immagine da 1500 a 2300. */
export const SSTV_SPEC_MIN_HZ = 300;
export const SSTV_SPEC_MAX_HZ = 2800;
/** Il sincronismo dell'SSTV e la larghezza fino al bianco (2300 Hz): la barra Rx copre tutto il segnale. */
const SYNC_HZ = 1200;
const SIGNAL_HZ = 1100;

/**
 * Lo spettro sottile sotto il visore SSTV: la stessa curva verde su griglia a puntini dello spettro della Stazione (`drawSpectrum`,
 * righe del core passate da `processRow` come nella cascata), l'etichetta «Rx» come le barre della cascata e il righello delle frequenze.
 * In SSTV la frequenza audio non si sceglie (il sincronismo e' sempre a 1200 Hz): la barra Rx e' solo un riferimento, non si trascina.
 */
export function SstvSpectrum({ height = 46 }: { height?: number }) {
  const plot = useRef<HTMLCanvasElement>(null);
  const curve = useRef<Float32Array | null>(null);

  useEffect(() => {
    const canvas = plot.current;
    if (!canvas) return;
    const draw = () => {
      const dpr = window.devicePixelRatio || 1;
      const rect = canvas.getBoundingClientRect();
      const w = Math.max(1, Math.round(rect.width * dpr));
      const h = Math.max(1, Math.round(rect.height * dpr));
      if (canvas.width !== w) canvas.width = w;
      if (canvas.height !== h) canvas.height = h;
      drawSpectrum(canvas, curve.current, SSTV_SPEC_MIN_HZ, SSTV_SPEC_MAX_HZ);
    };
    draw();
    // nell'ambiente delle prove non c'e'
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(draw);
    observer?.observe(canvas);
    const pacer = new RowPacer();
    const unregister = registerSpectrum((bins, fMinHz, fMaxHz) => {
      const p = useWfPrefs.getState().prefs;
      const average = pacer.add(bins, ROW_SECS[p.speed], p.navg);
      if (!average) return;
      const binWidthHz = (fMaxHz - fMinHz) / Math.max(1, bins.length);
      const row = processRow(average, binWidthHz, { binsPerPixel: p.binsPerPixel, startHz: SSTV_SPEC_MIN_HZ, endHz: SSTV_SPEC_MAX_HZ, scale: p.scale, flatten: p.flatten });
      if (row.y.length < 2) return;
      curve.current = liftCurve(spectrumCurve(row.y, 'current', p.gain2d, p.zero2d, p.flatten));
      draw();
    });
    return () => {
      unregister();
      observer?.disconnect();
    };
  }, []);

  return (
    <div className="sstv-spec" data-testid="sstv-spectrum">
      <div className="sstv-spec-plot" style={{ height }}>
        <canvas ref={plot} className="sstv-spec-canvas" aria-hidden="true" />
        <div className="wf-marker wf-marker--rx wf-marker--band sstv-spec-rx" style={{ left: `${leftPercent(SYNC_HZ, SSTV_SPEC_MIN_HZ, SSTV_SPEC_MAX_HZ)}%`, width: `${(SIGNAL_HZ / (SSTV_SPEC_MAX_HZ - SSTV_SPEC_MIN_HZ)) * 100}%` }} aria-hidden="true">
          <span className="wf-marker-tag">{`Rx ${SYNC_HZ}`}</span>
          <span className="wf-marker-line" />
        </div>
      </div>
      <FrequencyRuler fMinHz={SSTV_SPEC_MIN_HZ} fMaxHz={SSTV_SPEC_MAX_HZ} decodes={[]} rxHz={SYNC_HZ} bandwidthHz={SIGNAL_HZ} />
    </div>
  );
}
