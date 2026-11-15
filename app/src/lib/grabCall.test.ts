import { describe, expect, it } from 'vitest';
import { grabCall } from './grabCall';

describe('grabCall', () => {
  it('prende l’ultimo nominativo ricevuto, non il proprio', () => {
    expect(grabCall('CQ TEST K1ABC K1ABC', 'IU3ABC')).toBe('K1ABC');
    expect(grabCall('IU3ABC DE DL1XYZ DL1XYZ K', 'IU3ABC')).toBe('DL1XYZ');
    expect(grabCall('OK1AA 599 001 IU3ABC', 'IU3ABC')).toBe('OK1AA');
  });
  it('dopo DE vale chi trasmette', () => {
    expect(grabCall('G4ABC DE F5XYZ 5NN 012', 'IU3ABC')).toBe('F5XYZ');
  });
  it('rapporti e numeri non sono nominativi; niente se manca', () => {
    expect(grabCall('5NN 599 TU 73', 'IU3ABC')).toBe('');
    expect(grabCall('', 'IU3ABC')).toBe('');
  });
});
