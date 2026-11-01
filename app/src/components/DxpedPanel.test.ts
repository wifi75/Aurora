import { describe, expect, it } from 'vitest';
import { dxpedList, dxpedState } from './DxpedPanel';

const d = (call: string, start: string, end: string) => ({ call, start, end, country: 'X', qsl: '', info: '' });

describe('DXpedition list', () => {
  it('tells active, coming and recently ended apart', () => {
    expect(dxpedState(d('A', '2026-10-01', '2026-11-04'), '2026-11-01', 15)).toBe('active');
    expect(dxpedState(d('B', '2026-11-20', '2026-12-01'), '2026-11-01', 15)).toBe('soon');
    expect(dxpedState(d('C', '2027-03-01', '2027-03-10'), '2026-11-01', 15)).toBeNull();
    expect(dxpedState(d('D', '2026-10-01', '2026-10-25'), '2026-11-01', 15)).toBe('ended');
    expect(dxpedState(d('E', '2026-08-01', '2026-08-25'), '2026-11-01', 15)).toBeNull();
  });

  it('puts the ones heard now first, then active, coming and ended', () => {
    const list = dxpedList([d('OLD', '2026-10-01', '2026-10-25'), d('SOON', '2026-11-20', '2026-12-01'), d('ON', '2026-10-01', '2026-11-30'), d('HEARD', '2026-10-15', '2026-11-10')], new Set(['HEARD']), '2026-11-01', 15);
    expect(list.map((x) => x.call)).toEqual(['HEARD', 'ON', 'SOON', 'OLD']);
  });
});
