import { describe, expect, it } from 'vitest';
import type { DigiSignal } from '../api/schemas';
import { RADAR_COLORS, radarColor, radarStations } from './digiBand';

const sig = (id: number, freq: number, snr: number, calls: DigiSignal['calls'], cq = false): DigiSignal => ({ id, freq_hz: freq, snr_db: snr, text: '', unsure: [], calls, cq }) as DigiSignal;

describe('radar', () => {
  it('mostra solo i nominativi verificati, una riga per nominativo', () => {
    const list = [
      sig(1, 900, -10, [{ call: 'IU3ABC', role: 'cq', verified: true }], true),
      sig(2, 905, -4, [{ call: 'IU3ABC', role: 'de', verified: true }]),
      sig(3, 1500, -8, [{ call: 'K1XYZ', role: 'de', verified: false }]),
      sig(4, 1200, -6, [{ call: 'DL1ABC', role: 'de', verified: true }]),
    ];
    const st = radarStations(list);
    expect(st.map((s) => s.call)).toEqual(['IU3ABC', 'DL1ABC']);
    // dei due canali con lo stesso nominativo resta il piu' forte
    expect(st[0]?.freqHz).toBe(905);
  });

  it('i nominativi solo nominati da una stazione (il corrispondente) non sono stazioni', () => {
    const st = radarStations([sig(1, 900, -6, [{ call: 'SV5BYP', role: 'de', verified: true }, { call: 'IV3IPS', role: 'other', verified: true }])]);
    expect(st.map((s) => s.call)).toEqual(['SV5BYP']);
  });

  it('una stazione sentita adesso batte la stessa non sentita da tempo', () => {
    const st = radarStations([sig(1, 900, 5, [{ call: 'IU3ABC', role: 'de', verified: true, stale: true }]), sig(2, 1400, -9, [{ call: 'IU3ABC', role: 'de', verified: true }])]);
    expect(st).toHaveLength(1);
    expect(st[0]?.freqHz).toBe(1400);
    expect(st[0]?.stale).toBe(false);
  });

  it('il colore di una stazione e sempre lo stesso', () => {
    expect(radarColor('IU3ABC')).toBe(radarColor('IU3ABC'));
    const colors = new Set(['IU3ABC', 'K1XYZ', 'DL1ABC', 'G4ABC', 'JA1ABC', 'VK2ABC', 'F5ABC'].map(radarColor));
    expect(colors.size).toBeGreaterThan(3);
    for (const c of colors) expect(c).toBeLessThan(RADAR_COLORS);
  });
});
