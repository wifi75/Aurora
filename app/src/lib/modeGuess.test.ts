import { describe, expect, it } from 'vitest';
import { DB_MIN, DB_STEP } from '../render/jtdxRow';
import type { SpectrumRow } from './digiSpectrum';
import { guessMode } from './modeGuess';

/** Righe come quelle del motore: 6827 canali su 0-5000 Hz (0,73 Hz), byte = dB da DB_MIN a passo DB_STEP. */
const N = 6827;
const F_MAX = 5000;
const W = F_MAX / N;
const SPR = 0.2; // una riga ogni 0,2 s (il client ne tiene una su tre)

/** Una riga: rumore a -10 dB (con piccole oscillazioni fisse) piu' i toni `[hz, dB sopra il fondo, larghezza Hz]`. */
function row(tones: [number, number, number][], seed = 0): SpectrumRow {
  const p = new Float64Array(N);
  for (let i = 0; i < N; i++) {
    const wiggle = (((i * 7919 + seed * 104_729) % 13) - 6) / 6; // ±1 dB
    p[i] = 10 ** ((-10 + wiggle) / 10);
  }
  for (const [hz, db, width] of tones) {
    const sigma = Math.max(W, width / 2.355);
    const top = 10 ** ((-10 + db) / 10);
    const c = hz / W - 0.5;
    for (let i = Math.max(0, Math.floor(c - 5 * sigma / W)); i <= Math.min(N - 1, Math.ceil(c + 5 * sigma / W)); i++) {
      const d = (i - c) * W;
      p[i] = (p[i] ?? 0) + top * Math.exp(-(d * d) / (2 * sigma * sigma));
    }
  }
  const bins = new Uint8Array(N);
  for (let i = 0; i < N; i++) bins[i] = Math.max(0, Math.min(255, Math.round((10 * Math.log10(p[i] ?? 1e-30) - DB_MIN) / DB_STEP)));
  return { bins, fMinHz: 0, fMaxHz: F_MAX };
}

/** `count` righe, `at(k)` da' i toni della riga k. */
function history(count: number, at: (k: number) => [number, number, number][]): SpectrumRow[] {
  return Array.from({ length: count }, (_, k) => row(at(k), k));
}

/** Un blocco piatto largo `width` Hz fatto di toni vicini (FT8: 8 toni a 6,25 Hz). */
function block(centre: number, width: number, db: number, step: number): [number, number, number][] {
  const out: [number, number, number][] = [];
  for (let f = centre - width / 2 + step / 2; f < centre + width / 2; f += step) out.push([f, db, step * 1.2]);
  return out;
}

const kinds = (r: ReturnType<typeof guessMode>) => r.guesses.map((g) => g.kind);

describe('guessMode', () => {
  it('nel solo rumore non riconosce niente', () => {
    const r = guessMode(history(100, () => []), 1500, SPR);
    expect(r.guesses).toEqual([]);
  });

  it('una riga stretta che va a parole e pause e\' CW', () => {
    // 2 s accesa a livello che varia, 1,6 s spenta
    const rows = history(150, (k) => (k % 18 < 10 ? [[1200, 20 + (k % 5) * 2, 3]] : []));
    const r = guessMode(rows, 1205, SPR);
    expect(r.guesses[0]?.kind).toBe('cw');
    expect(r.guesses[0]?.confidence).toBe('likely');
    expect(Math.abs((r.guesses[0]?.centerHz ?? 0) - 1200)).toBeLessThan(3);
  });

  it.each([30, 0])('un CW vero a 20 WPM (pausa fra le chiamate %i punti), visto con la finestra di 1,365 s del motore, resta CW', (pause) => {
    // manipolazione vera: punto 60 ms, linea 180, spazio fra lettere 180, fra parole 420; poi la media su 1,365 s ogni 0,2 s
    const code: Record<string, string> = { C: '-.-.', Q: '--.-', D: '-..', E: '.', I: '..', U: '..-', '3': '...--', A: '.-', B: '-...', K: '-.-' };
    const dit = 0.06;
    const keyed: boolean[] = [];
    const push = (on: boolean, units: number) => {
      for (let i = 0; i < Math.round((units * dit) / 0.01); i++) keyed.push(on);
    };
    for (let rep = 0; rep < 6; rep++) {
      for (const word of ['CQ', 'CQ', 'DE', 'IU3ABC', 'IU3ABC', 'K']) {
        for (const ch of word) {
          for (const el of code[ch] ?? '') {
            push(true, el === '-' ? 3 : 1);
            push(false, 1);
          }
          push(false, 2);
        }
        push(false, 4);
      }
      push(false, pause); // fine della chiamata
    }
    const win = Math.round(1.365 / 0.01);
    const rows = history(150, (k) => {
      const end = Math.min(keyed.length, k * 20 + win);
      let on = 0;
      for (let i = end - win; i < end; i++) if (keyed[i]) on++;
      const frac = on / win;
      return frac > 0 ? [[1200, 22 + 20 * Math.log10(frac), 3]] : [];
    });
    expect(guessMode(rows, 1200, SPR).guesses[0]?.kind).toBe('cw');
  });

  it('una riga ferma e continua e\' una portante (CW solo possibile)', () => {
    const r = guessMode(history(150, () => [[900, 25, 2]]), 900, SPR);
    expect(kinds(r)).toEqual(['carrier', 'cw']);
  });

  it('due righe a 170 Hz con lo stesso andamento sono RTTY, con il centro in mezzo', () => {
    const rows = history(150, (k) => {
      const lv = 18 + (k % 7) * 2;
      return k % 40 < 30 ? [[1415, lv, 30], [1585, lv - 2, 30]] : [];
    });
    const r = guessMode(rows, 1420, SPR);
    expect(r.guesses[0]?.kind).toBe('rtty');
    expect(r.guesses[0]?.confidence).toBe('likely');
    expect(r.guesses[0]?.shiftHz).toBeGreaterThan(165);
    expect(r.guesses[0]?.shiftHz).toBeLessThan(175);
    expect(Math.abs((r.guesses[0]?.centerHz ?? 0) - 1500)).toBeLessThan(3);
  });

  it('una coppia RTTY con i toni a livelli diversi resta RTTY', () => {
    const rows = history(150, (k) => {
      const lv = 22 + (k % 6) * 2;
      return [[1415, lv, 30], [1585, lv - 8, 30]];
    });
    expect(guessMode(rows, 1585, SPR).guesses[0]?.kind).toBe('rtty');
  });

  it('due righe a ±15,6 Hz insieme sono PSK31', () => {
    const rows = history(150, (k) => {
      const lv = 20 + (k % 9);
      return [[984.4, lv, 3], [1015.6, lv, 3]];
    });
    const r = guessMode(rows, 1000, SPR);
    expect(r.guesses[0]?.kind).toBe('psk31');
    expect(r.guesses[0]?.confidence).toBe('likely');
    expect(Math.abs((r.guesses[0]?.centerHz ?? 0) - 1000)).toBeLessThan(2);
  });

  it('due stazioni CW a 31 Hz l\'una dall\'altra, ognuna per conto suo, non sono un PSK', () => {
    const rows = history(150, (k) => {
      const a: [number, number, number][] = k % 14 < 7 ? [[984, 22, 3]] : [];
      const b: [number, number, number][] = k % 22 >= 11 ? [[1015, 22, 3]] : [];
      return [...a, ...b];
    });
    const r = guessMode(rows, 984, SPR);
    expect(kinds(r)).not.toContain('psk31');
    expect(r.guesses[0]?.kind).toBe('cw');
  });

  it('un blocco di 50 Hz a periodi di 15 s e\' FT8 probabile, PSK31 possibile', () => {
    // 12,6 s acceso e 2,4 s spento, a righe di 0,2 s: 63 accese e 12 spente
    const rows = history(150, (k) => (k % 75 < 63 ? block(1500, 50, 18, 6.25) : []));
    const r = guessMode(rows, 1500, SPR);
    expect(kinds(r)).toEqual(['ft8', 'psk31']);
    expect(r.guesses[0]?.confidence).toBe('likely');
  });

  it('FT8 visto con la finestra di 1,365 s del motore (pause smussate) resta FT8 probabile', () => {
    // 12,6 s acceso e 2,4 s spento; ogni riga e' la media su 1,365 s
    const rows = history(150, (k) => {
      const t = k * SPR;
      let on = 0;
      for (let i = 0; i < 137; i++) if ((t - i * 0.01 + 150) % 15 < 12.6) on++;
      const frac = on / 137;
      return frac > 0 ? block(1500, 50, 18 + 10 * Math.log10(frac), 6.25) : [];
    });
    const r = guessMode(rows, 1500, SPR);
    expect(kinds(r)).toEqual(['ft8', 'psk31']);
    expect(r.guesses[0]?.confidence).toBe('likely');
  });

  it('un blocco di 50 Hz continuo resta incerto fra FT8 e PSK31', () => {
    const r = guessMode(history(150, () => block(1500, 50, 18, 6.25)), 1500, SPR);
    expect(kinds(r)).toEqual(['ft8', 'psk31']);
    expect(r.guesses.every((g) => g.confidence === 'possible')).toBe(true);
  });

  it('un blocco di circa 90 Hz a pause e\' FT4', () => {
    const rows = history(150, (k) => (k % 37 < 23 ? block(2000, 84, 18, 20.8) : []));
    expect(guessMode(rows, 2000, SPR).guesses[0]?.kind).toBe('ft4');
  });

  it('la voce larga 2,4 kHz e\' solo «segnale largo», mai un modo stretto', () => {
    const voice: [number, number, number][] = [];
    for (let f = 350; f <= 2700; f += 25) voice.push([f, 14 + 6 * Math.sin(f / 180), 40]);
    const r = guessMode(history(100, () => voice), 1200, SPR);
    expect(kinds(r)).toEqual(['wide']);
    expect(r.guesses[0]?.confidence).toBe('possible');
  });

  it('senza righe o con righe di misure diverse non si rompe', () => {
    expect(guessMode([], 1000, SPR).guesses).toEqual([]);
    const mixed = [row([[1000, 20, 3]]), { bins: new Uint8Array(100), fMinHz: 0, fMaxHz: 5000 }];
    expect(guessMode(mixed, 1000, SPR).guesses).toEqual([]);
  });
});
