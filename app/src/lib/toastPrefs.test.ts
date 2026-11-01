import { describe, expect, it } from 'vitest';
import { defaultToastPrefs, readToastPrefs } from './toastPrefs';

describe('toastPrefs', () => {
  it('di fabbrica mostra tutto, in alto al centro', () => {
    const p = defaultToastPrefs();
    expect(p.show).toEqual({ ok: true, info: true, error: true });
    expect(p.position).toBe('top');
  });

  it('gli errori non si possono spegnere e i secondi restano tra 1 e 30', () => {
    const p = readToastPrefs({ getItem: () => JSON.stringify({ show: { ok: false, error: false }, seconds: { ok: 0, info: 99, error: 12 }, position: 'bottom', size: 'large' }) });
    expect(p.show).toEqual({ ok: false, info: true, error: true });
    expect(p.seconds).toEqual({ ok: 1, info: 30, error: 12 });
    expect(p.position).toBe('bottom');
    expect(p.size).toBe('large');
  });

  it('una lettura rovinata torna ai valori di fabbrica', () => {
    expect(readToastPrefs({ getItem: () => '{non json' })).toEqual(defaultToastPrefs());
  });
});
