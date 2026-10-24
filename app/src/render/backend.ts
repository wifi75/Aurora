import type { Levels } from './levels';

export type BackendKind = 'webgpu' | 'webgl2';

export interface WaterfallBackend {
  readonly kind: BackendKind;
  init(canvas: HTMLCanvasElement): Promise<void>;
  /** Aggiunge una riga di spettro (byte 0-255, riscalata alla larghezza della texture). */
  pushRow(row: Uint8Array): void;
  /** Nero, guadagno e contrasto del disegno (si applicano subito a tutta la storia). */
  setLevels(levels: Levels): void;
  resize(widthPx: number, heightPx: number): void;
  destroy(): void;
}

export interface BackendEnv {
  hasWebGPU: boolean;
  hasWebGL2: boolean;
  forced?: BackendKind;
}

/** Sceglie il renderer: WebGPU se c'e', altrimenti WebGL2; un'imposizione vale solo se disponibile. */
export function chooseBackend(env: BackendEnv): BackendKind | 'none' {
  const available = (k: BackendKind) => (k === 'webgpu' ? env.hasWebGPU : env.hasWebGL2);
  if (env.forced && available(env.forced)) return env.forced;
  if (env.hasWebGPU) return 'webgpu';
  if (env.hasWebGL2) return 'webgl2';
  return 'none';
}

async function detect(): Promise<{ hasWebGPU: boolean; hasWebGL2: boolean }> {
  let hasWebGPU = false;
  try {
    hasWebGPU = !!navigator.gpu && !!(await navigator.gpu.requestAdapter());
  } catch {
    hasWebGPU = false;
  }
  // canvas temporaneo: un canvas che ha preso un contesto non puo' cambiarlo
  const probe = document.createElement('canvas');
  const hasWebGL2 = !!probe.getContext('webgl2');
  return { hasWebGPU, hasWebGL2 };
}

async function instantiate(kind: BackendKind): Promise<WaterfallBackend> {
  if (kind === 'webgpu') return new (await import('./webgpu')).WebGpuWaterfall();
  return new (await import('./webgl2')).WebGl2Waterfall();
}

/** Crea e inizializza il miglior renderer disponibile; ripiega su WebGL2 se WebGPU fallisce. */
export async function createBackend(canvas: HTMLCanvasElement, forced?: BackendKind): Promise<WaterfallBackend | null> {
  const env = { ...(await detect()), forced };
  const first = chooseBackend(env);
  if (first === 'none') return null;
  try {
    const backend = await instantiate(first);
    await backend.init(canvas);
    return backend;
  } catch (error) {
    // un canvas che ha preso un contesto non puo' prenderne un altro: il ripiego
    // su WebGL2 spetta a chi chiama, con un canvas nuovo e `forced: 'webgl2'`
    console.warn(`Renderer ${first} non disponibile`, error);
    return null;
  }
}
