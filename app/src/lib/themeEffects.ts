import { create } from 'zustand';
import { browserStorage } from './theme';

/*
 * Effetti a tema dell'intestazione (Impostazioni › Aspetto › «Effetti a tema»): la neve nel riquadro del nome e le luci a filo sul bordo
 * alto. Ognuno ha tre stati: «Automatico» (solo nel periodo invernale e solo sui PC potenti), «Acceso» (sempre), «Spento».
 * Di fabbrica la neve e' automatica e le luci spente. Una scelta per dispositivo, come le altre preferenze dell'interfaccia.
 */

export type FxMode = 'auto' | 'on' | 'off';
export type FxPower = 'fast' | 'slow' | 'unknown';

export interface ThemeEffects {
  snow: FxMode;
  lights: FxMode;
}

export const THEME_EFFECTS_KEY = 'aurora-theme-effects';
export const DEFAULT_THEME_EFFECTS: ThemeEffects = { snow: 'auto', lights: 'off' };
/** Sotto questa media di fotogrammi al secondo il PC si considera lento per l'«Automatico». */
export const MIN_FPS = 45;

/** Dal 1° dicembre al 6 gennaio compresi, secondo l'ora locale del PC. */
export function inWinterPeriod(date: Date): boolean {
  const m = date.getMonth();
  return m === 11 || (m === 0 && date.getDate() <= 6);
}

/** Potente per l'hardware: almeno 6 processori logici e, se il browser lo dice, almeno 4 GB di memoria. */
export function hardwarePowerful(cores: number | undefined, memoryGb: number | undefined): boolean {
  if (typeof cores !== 'number' || cores < 6) return false;
  return typeof memoryGb !== 'number' || memoryGb >= 4;
}

/** Media dei fotogrammi misurati: sopra la soglia il PC regge l'effetto. */
export function fpsOk(frames: number, ms: number): boolean {
  if (ms <= 0) return false;
  return (frames * 1000) / ms >= MIN_FPS;
}

/** Mostra l'effetto? «Acceso» sempre, «Spento» mai, «Automatico» solo nel periodo e su un PC risultato potente. */
export function fxVisible(mode: FxMode, date: Date, power: FxPower): boolean {
  if (mode === 'on') return true;
  if (mode === 'off') return false;
  return inWinterPeriod(date) && power === 'fast';
}

const isMode = (v: unknown): v is FxMode => v === 'auto' || v === 'on' || v === 'off';

export function readThemeEffects(storage: Pick<Storage, 'getItem'> | null): ThemeEffects {
  try {
    const raw = storage?.getItem(THEME_EFFECTS_KEY);
    if (!raw) return { ...DEFAULT_THEME_EFFECTS };
    const p: unknown = JSON.parse(raw);
    if (typeof p !== 'object' || p === null) return { ...DEFAULT_THEME_EFFECTS };
    const o = p as Record<string, unknown>;
    return { snow: isMode(o.snow) ? o.snow : DEFAULT_THEME_EFFECTS.snow, lights: isMode(o.lights) ? o.lights : DEFAULT_THEME_EFFECTS.lights };
  } catch {
    return { ...DEFAULT_THEME_EFFECTS };
  }
}

export function saveThemeEffects(storage: Pick<Storage, 'setItem'> | null, fx: ThemeEffects): void {
  try {
    storage?.setItem(THEME_EFFECTS_KEY, JSON.stringify(fx));
  } catch {
    // senza archivio la scelta non viene ricordata
  }
}

interface ThemeEffectsState {
  fx: ThemeEffects;
  set(next: ThemeEffects): void;
}

export const useThemeEffects = create<ThemeEffectsState>((set) => ({
  fx: readThemeEffects(browserStorage()),
  set(next) {
    saveThemeEffects(browserStorage(), next);
    set({ fx: next });
  },
}));

// le scelte fatte in un'altra finestra (Impostazioni staccate) valgono subito anche qui
if (typeof window !== 'undefined') {
  window.addEventListener('storage', (e) => {
    if (e.key === null || e.key === THEME_EFFECTS_KEY) useThemeEffects.setState({ fx: readThemeEffects(browserStorage()) });
  });
}
