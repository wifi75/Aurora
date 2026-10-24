import { useEffect, useRef, useState } from 'react';
import { useT } from '../i18n';
import { createBackend, type WaterfallBackend } from '../render/backend';
import { RowAverager } from '../render/smooth';

/** Righe del core (15 al secondo) mediate a gruppi: circa 4 al secondo, la velocita' di una cascata leggibile. */
const ROWS_PER_STEP = 4;

interface Props {
  /** Registra la funzione che riceve ogni riga di spettro; restituisce come annullare. */
  register(push: (bins: Uint8Array) => void): () => void;
}

type Phase = 'loading' | 'ready' | 'unsupported';

export function Waterfall({ register }: Props) {
  const t = useT();
  const [phase, setPhase] = useState<Phase>('loading');
  const [kind, setKind] = useState('');
  // WebGPU puo' aver gia' preso il canvas: per ripiegare su WebGL2 serve un canvas nuovo
  const [attempt, setAttempt] = useState<{ forced?: 'webgl2'; key: number }>({ key: 0 });
  const canvasRef = useRef<HTMLCanvasElement>(null);

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
        setKind(b.kind);
        setPhase('ready');
        const fit = () => {
          const dpr = window.devicePixelRatio || 1;
          const rect = canvas.getBoundingClientRect();
          b.resize(Math.round(rect.width * dpr), Math.round(rect.height * dpr));
        };
        fit();
        observer = new ResizeObserver(fit);
        observer.observe(canvas);
        const average = new RowAverager(ROWS_PER_STEP);
        unregister = register((bins) => {
          const row = average.add(bins);
          if (row) b.pushRow(row);
        });
      })
      .catch(() => {
        if (!cancelled) setPhase('unsupported');
      });

    return () => {
      cancelled = true;
      unregister();
      observer?.disconnect();
      backend?.destroy();
    };
  }, [register, attempt]);

  return (
    <div className="waterfall" data-backend={kind} data-phase={phase}>
      <canvas key={attempt.key} ref={canvasRef} role="img" aria-label={t('wf.aria')} title={t('tip.panel.waterfall')} />
      {phase === 'unsupported' && (
        <p className="waterfall-note">{t('wf.unsupported')}</p>
      )}
    </div>
  );
}
