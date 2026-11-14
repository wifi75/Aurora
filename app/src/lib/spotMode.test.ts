import { describe, expect, it } from 'vitest';
import type { ClusterSpot } from '../api/client';
import { setPendingSpot, takePendingSpot, textModeOfSpot } from './spotMode';

const spot = (khz: number, mode: string | null): ClusterSpot => ({ freq_khz: khz, mode, dx_call: 'LZ5R' }) as ClusterSpot;

describe('modo di uno spot', () => {
  it('prima il piano di banda, poi il cluster', () => {
    // vicino alla frequenza di chiamata RTTY dei 15 m (21080): RTTY anche se il cluster non dice niente
    expect(textModeOfSpot(spot(21_081, null))).toBe('rtty');
    // lontano dalle frequenze di chiamata: decide il cluster
    expect(textModeOfSpot(spot(21_109, 'RTTY'))).toBe('rtty');
    expect(textModeOfSpot(spot(14_030, 'CW'))).toBe('cw');
    // 40 m regione 1: PSK e RTTY alla stessa frequenza, decide il cluster
    expect(textModeOfSpot(spot(7_040, 'RTTY'))).toBe('rtty');
    // FT8, SSB: nessun modo testuale
    expect(textModeOfSpot(spot(14_074, 'FT8'))).toBeNull();
    expect(textModeOfSpot(spot(21_265, 'SSB'))).toBeNull();
  });

  it('lo spot in attesa si prende una volta sola', () => {
    setPendingSpot(spot(21_109, 'RTTY'));
    expect(takePendingSpot()?.freq_khz).toBe(21_109);
    expect(takePendingSpot()).toBeNull();
  });
});
