import { describe, expect, it } from 'vitest';
import { pageForSpot, readOpenModePage, writeOpenModePage } from './spotOptions';

describe('apri la pagina del modo', () => {
  it('di base e\' spento e si ricorda', () => {
    expect(readOpenModePage()).toBe(false);
    writeOpenModePage(true);
    expect(readOpenModePage()).toBe(true);
    writeOpenModePage(false);
    expect(readOpenModePage()).toBe(false);
  });

  it('porta alla pagina giusta per ogni modo', () => {
    expect(pageForSpot('cw', 'CW')).toBe('text');
    expect(pageForSpot('rtty', 'RTTY')).toBe('text');
    expect(pageForSpot(null, 'FT8')).toBe('main');
    expect(pageForSpot(null, 'ft4')).toBe('main');
    expect(pageForSpot(null, 'SSB')).toBe('none');
    expect(pageForSpot(null, null)).toBe('none');
  });
});
