import { describe, expect, it } from 'vitest';
import { defaultLotwView, lotwDateToShow, lotwIsRecent, readLotwView } from './lotw';

const NOW = new Date(Date.UTC(2026, 10, 11, 12, 0, 0));

describe('lotwIsRecent', () => {
  it('conta solo l\'ultimo invio entro i mesi scelti', () => {
    expect(lotwIsRecent('2026-10-28', 3, NOW)).toBe(true);
    expect(lotwIsRecent('2026-08-11', 3, NOW)).toBe(true);
    expect(lotwIsRecent('2026-08-10', 3, NOW)).toBe(false);
    expect(lotwIsRecent('2025-11-11', 12, NOW)).toBe(true);
    expect(lotwIsRecent('2025-11-10', 12, NOW)).toBe(false);
    expect(lotwIsRecent('2024-11-11', 24, NOW)).toBe(true);
    expect(lotwIsRecent('2024-11-10', 24, NOW)).toBe(false);
  });
  it('con «qualunque» (0) vale ogni data valida, mai una data rotta', () => {
    expect(lotwIsRecent('2001-01-01', 0, NOW)).toBe(true);
    expect(lotwIsRecent('boh', 0, NOW)).toBe(false);
    expect(lotwIsRecent(null, 12, NOW)).toBe(false);
    expect(lotwIsRecent(undefined, 0, NOW)).toBe(false);
  });
});

describe('lotwDateToShow', () => {
  it('rispetta l\'interruttore e i mesi', () => {
    const view = defaultLotwView();
    expect(lotwDateToShow('2026-10-28', view, NOW)).toBe('2026-10-28');
    expect(lotwDateToShow('2020-01-01', view, NOW)).toBeNull();
    expect(lotwDateToShow('2020-01-01', { ...view, months: 0 }, NOW)).toBe('2020-01-01');
    expect(lotwDateToShow('2026-10-28', { ...view, enabled: false }, NOW)).toBeNull();
    expect(lotwDateToShow(null, view, NOW)).toBeNull();
  });
});

describe('readLotwView', () => {
  const stored = (v: unknown) => ({ getItem: () => JSON.stringify(v) });
  it('di fabbrica acceso, verde chiaro, 12 mesi', () => {
    expect(readLotwView(null)).toEqual({ enabled: true, color: '#5fd35f', months: 12 });
  });
  it('legge le scelte valide e scarta le altre', () => {
    expect(readLotwView(stored({ enabled: false, color: '#ABCDEF', months: 6 }))).toEqual({ enabled: false, color: '#abcdef', months: 6 });
    expect(readLotwView(stored({ color: 'rosso', months: 5 }))).toEqual(defaultLotwView());
  });
});
