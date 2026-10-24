import { useEffect, useRef, useState } from 'react';
import { useT } from '../i18n';
import { createBackend, type WaterfallBackend } from '../render/backend';
import { LEVEL_KEYS, clampLevel, readLevels, type Levels } from '../render/levels';
import { RowAverager } from '../render/smooth';

/** Righe del core (15 al secondo) mediate a gruppi. Di base nessuna media (15 righe al secondo, come Decodium: scorre fluida);
 * con la chiave `aurora-wf-rows` (1-4) si puo' mediare di piu', per una cascata piu' lenta e meno rumorosa. */
const DEFAULT_ROWS_PER_STEP = 1;

export function rowsPerStep(raw: string | null | undefined): number {
  const n = Number(raw);
  return Number.isInteger(n) && n >= 1 && n <= 4 ? n : DEFAULT_ROWS_PER_STEP;
}

const ROWS_KEY = 'aurora-wf-rows';

function readPref(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writePref(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // senza archivio del browser le regolazioni valgono fino alla chiusura della pagina
  }
}

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
  const backendRef = useRef<WaterfallBackend | null>(null);
  const [levels, setLevels] = useState<Levels>(() => readLevels(readPref));
  const [rows, setRows] = useState(() => rowsPerStep(readPref(ROWS_KEY)));
  const rowsRef = useRef(rows);
  const levelsRef = useRef(levels);

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
        b.setLevels(levelsRef.current);
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
        let average = new RowAverager(rowsRef.current);
        unregister = register((bins) => {
          if (average.every !== rowsRef.current) average = new RowAverager(rowsRef.current);
          const row = average.add(bins);
          if (row) b.pushRow(row);
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

  return (
    <div className="waterfall" data-backend={kind} data-phase={phase}>
      <canvas key={attempt.key} ref={canvasRef} role="img" aria-label={t('wf.aria')} title={t('tip.panel.waterfall')} />
      {phase === 'unsupported' && (
        <p className="waterfall-note">{t('wf.unsupported')}</p>
      )}
      <div className="wf-controls">
        {(['black', 'gain', 'contrast'] as const).map((name) => (
          <label key={name} className="wf-slider">
            <span>{t(`wf.${name}`)}</span>
            <input
              type="range"
              min={name === 'black' ? 0 : name === 'gain' ? 0.4 : 0.5}
              max={name === 'black' ? 0.6 : 2.5}
              step={0.05}
              value={levels[name]}
              aria-label={t(`wf.${name}`)}
              onChange={(e) => {
                const next = { ...levelsRef.current, [name]: clampLevel(name, Number(e.target.value)) };
                levelsRef.current = next;
                setLevels(next);
                backendRef.current?.setLevels(next);
                writePref(LEVEL_KEYS[name], String(next[name]));
              }}
            />
          </label>
        ))}
        <label className="wf-slider">
          <span>{t('wf.speed')}</span>
          <select
            value={rows}
            aria-label={t('wf.speed')}
            onChange={(e) => {
              const next = rowsPerStep(e.target.value);
              rowsRef.current = next;
              setRows(next);
              writePref(ROWS_KEY, String(next));
            }}
          >
            {[1, 2, 3, 4].map((n) => (
              <option key={n} value={n}>
                {n === 1 ? '15/s' : `${(15 / n).toFixed(1).replace('.0', '')}/s`}
              </option>
            ))}
          </select>
        </label>
      </div>
    </div>
  );
}
