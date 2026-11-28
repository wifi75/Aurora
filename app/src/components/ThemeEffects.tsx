import { useEffect, useState } from 'react';
import { fpsOk, fxVisible, hardwarePowerful, inWinterPeriod, type FxPower, useThemeEffects } from '../lib/themeEffects';
import '../styles/themeEffects.css';

/*
 * Effetti a tema dell'intestazione: neve nel riquadro del nome «AURORA by IU3WJQ» e luci a filo sul bordo alto.
 * Solo CSS (trasformazioni e opacita'), poche decine di elementi, nessun clic intercettato; con la finestra nascosta si fermano.
 */

/** Verdetto sulla potenza del PC, misurato una volta per finestra. */
let powerVerdict: FxPower = 'unknown';
let measuring = false;
const listeners = new Set<(p: FxPower) => void>();

function publish(p: FxPower): void {
  powerVerdict = p;
  listeners.forEach((l) => l(p));
}

/** Prima l'hardware, poi ~2 s di fotogrammi qualche secondo dopo l'avvio (la schermata iniziale abbasserebbe la media). */
function measurePower(): void {
  if (measuring || powerVerdict !== 'unknown') return;
  measuring = true;
  const nav = navigator as Navigator & { deviceMemory?: number };
  if (!hardwarePowerful(nav.hardwareConcurrency, nav.deviceMemory)) {
    publish('slow');
    return;
  }
  const run = () => {
    // con la finestra nascosta i fotogrammi non arrivano: non vuol dire lento, si riprova quando torna visibile
    if (document.hidden) {
      document.addEventListener('visibilitychange', run, { once: true });
      return;
    }
    let frames = 0;
    let start = 0;
    const step = (now: number) => {
      if (document.hidden) {
        document.addEventListener('visibilitychange', run, { once: true });
        return;
      }
      if (!start) start = now;
      else frames++;
      if (now - start < 2000) requestAnimationFrame(step);
      else publish(fpsOk(frames, now - start) ? 'fast' : 'slow');
    };
    requestAnimationFrame(step);
  };
  window.setTimeout(run, 5000);
}

function usePower(needed: boolean): FxPower {
  const [p, setP] = useState<FxPower>(powerVerdict);
  useEffect(() => {
    listeners.add(setP);
    if (needed) measurePower();
    return () => {
      listeners.delete(setP);
    };
  }, [needed]);
  return p;
}

function useHidden(): boolean {
  const [hidden, setHidden] = useState(() => typeof document !== 'undefined' && document.hidden);
  useEffect(() => {
    const on = () => setHidden(document.hidden);
    document.addEventListener('visibilitychange', on);
    return () => document.removeEventListener('visibilitychange', on);
  }, []);
  return hidden;
}

/** Quali effetti mostrare ora, e se sono in pausa (finestra nascosta). La data si rilegge ogni ora. */
export function useHeaderEffects(): { snow: boolean; lights: boolean; paused: boolean } {
  const fx = useThemeEffects((s) => s.fx);
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 3_600_000);
    return () => window.clearInterval(id);
  }, []);
  const needed = inWinterPeriod(now) && (fx.snow === 'auto' || fx.lights === 'auto');
  const power = usePower(needed);
  const paused = useHidden();
  return { snow: fxVisible(fx.snow, now, power), lights: fxVisible(fx.lights, now, power), paused };
}

/** Posizioni fisse (pseudo-casuali ma sempre uguali) dei fiocchi: x e y in %, durata e ritardo in s, dimensione in px. */
const FLAKES = Array.from({ length: 14 }, (_, i) => {
  const r = (n: number) => ((Math.sin((i + 1) * n) + 1) / 2) % 1;
  return { x: Math.round(r(12.9898) * 96) + 2, y: Math.round(r(78.233) * 80) + 5, dur: 6 + r(3.17) * 5, delay: -r(5.71) * 11, size: 1.5 + r(9.3) * 1.5, dx: Math.round((r(4.1) - 0.5) * 12) };
});

export function SnowLayer({ paused }: { paused: boolean }) {
  return (
    <span className={`fx-snow${paused ? ' fx-paused' : ''}`} aria-hidden="true">
      {FLAKES.map((f, i) => (
        <i
          key={i}
          className="fx-flake"
          style={{ '--x': `${f.x}%`, '--y': `${f.y}%`, '--dur': `${f.dur.toFixed(2)}s`, '--delay': `${f.delay.toFixed(2)}s`, '--s': `${f.size.toFixed(1)}px`, '--dx': `${f.dx}px` } as React.CSSProperties}
        />
      ))}
      <i className="fx-snowcap" />
    </span>
  );
}

const BULBS = 48;

export function LightsStrip({ paused }: { paused: boolean }) {
  return (
    <span className={`fx-lights${paused ? ' fx-paused' : ''}`} aria-hidden="true">
      {Array.from({ length: BULBS }, (_, i) => (
        <i key={i} className={`fx-bulb fx-bulb--${(i % 4) + 1}`} style={{ '--delay': `${-((i * 7) % 11) * 0.37}s` } as React.CSSProperties} />
      ))}
    </span>
  );
}
