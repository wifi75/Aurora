import { useEffect, useRef, useState } from 'react';
import { createBackend, type WaterfallBackend } from '../render/backend';

interface Props {
  /** Registra la funzione che riceve ogni riga di spettro; restituisce come annullare. */
  register(push: (bins: Uint8Array) => void): () => void;
}

type Phase = 'loading' | 'ready' | 'unsupported';

export function Waterfall({ register }: Props) {
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
        unregister = register((bins) => b.pushRow(bins));
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
      <canvas key={attempt.key} ref={canvasRef} role="img" aria-label="Cascata dello spettro audio" />
      {phase === 'unsupported' && (
        <p className="waterfall-note">
          Questo browser non supporta la grafica accelerata (WebGPU o WebGL2): la cascata non può essere mostrata.
        </p>
      )}
    </div>
  );
}
