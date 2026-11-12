import { describe, expect, it } from 'vitest';
import { RowAverager, seekSignal, snapToPair, snapToPeak, type SpectrumRow } from './digiSpectrum';

/** Una riga 0-4000 Hz con 1000 bin (4 Hz l'uno), rumore basso e dei picchi larghi 20 Hz. */
function row(peaksHz: number[]): SpectrumRow {
  const bins = new Uint8Array(1000);
  for (let i = 0; i < bins.length; i++) bins[i] = 20 + ((i * 7) % 5);
  for (const hz of peaksHz) {
    const c = Math.round(hz / 4);
    for (let d = -3; d <= 3; d++) bins[c + d] = 200 - Math.abs(d) * 25;
  }
  return { bins, fMinHz: 0, fMaxHz: 4000 };
}

function averaged(r: SpectrumRow): Float32Array {
  const a = new RowAverager();
  for (let i = 0; i < 4; i++) a.push(r);
  return a.values();
}

describe('snapToPeak', () => {
  it('porta il clic sul centro del segnale vicino', () => {
    const r = row([1500]);
    const hz = snapToPeak(averaged(r), r, 1512);
    expect(Math.abs(hz - 1502)).toBeLessThan(6);
  });

  it('senza segnale entro la finestra resta dove si e\' cliccato', () => {
    const r = row([2500]);
    expect(snapToPeak(averaged(r), r, 1000)).toBe(1000);
  });
});

describe('snapToPair', () => {
  it('porta il clic al centro fra i due toni RTTY, anche cliccando su un tono', () => {
    const r = row([1415, 1585]);
    const avg = averaged(r);
    for (const click of [1500, 1420, 1580, 1460]) expect(Math.abs(snapToPair(avg, r, click, 170) - 1502)).toBeLessThan(6);
  });

  it('con un tono solo vale la regola: sotto il clic è il basso, sopra l’alto', () => {
    const r = row([1400]);
    const avg = averaged(r);
    expect(Math.abs(snapToPair(avg, r, 1450, 170) - 1487)).toBeLessThan(6);
    expect(Math.abs(snapToPair(avg, r, 1350, 170) - 1317)).toBeLessThan(6);
  });

  it('senza segnale resta dove si è cliccato; con lo spostamento stretto si comporta come Snap', () => {
    const r = row([2500]);
    expect(snapToPair(averaged(r), r, 1000, 170)).toBe(1000);
    const n = row([1500]);
    expect(Math.abs(snapToPair(averaged(n), n, 1512, 23) - 1502)).toBeLessThan(6);
  });
});

describe('seekSignal', () => {
  const r = row([900, 1600, 2300]);
  const avg = averaged(r);
  const limits = { min: 200, max: 3500 };

  it('trova il segnale successivo a destra e a sinistra', () => {
    expect(Math.abs((seekSignal(avg, r, 1600, 'right', limits) ?? 0) - 2302)).toBeLessThan(8);
    expect(Math.abs((seekSignal(avg, r, 1600, 'left', limits) ?? 0) - 902)).toBeLessThan(8);
  });

  it('non ritrova la stazione su cui si e\' gia\'', () => {
    const next = seekSignal(avg, r, 900, 'right', limits);
    expect(next).not.toBeNull();
    expect(Math.abs((next ?? 0) - 900)).toBeGreaterThan(100);
  });

  it('senza altri segnali dice null', () => {
    expect(seekSignal(avg, r, 2300, 'right', limits)).toBeNull();
    expect(seekSignal(avg, r, 900, 'left', limits)).toBeNull();
  });
});
