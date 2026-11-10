import { describe, expect, it } from 'vitest';
import { filterWindowHz, hzAt, leftPercent, signalBandwidthHz } from './markers';

describe('markers', () => {
  it('places a frequency as a percentage of the audio band', () => {
    expect(leftPercent(0, 0, 3000)).toBe(0);
    expect(leftPercent(1500, 0, 3000)).toBe(50);
    expect(leftPercent(3000, 0, 3000)).toBe(100);
  });

  it('keeps markers inside the band', () => {
    expect(leftPercent(-50, 0, 3000)).toBe(0);
    expect(leftPercent(9000, 0, 3000)).toBe(100);
  });

  it('turns a position back into a whole number of hertz', () => {
    expect(hzAt(0.5, 0, 3000)).toBe(1500);
    expect(hzAt(0.3333, 0, 3000)).toBe(1000);
    expect(hzAt(-1, 0, 3000)).toBe(0);
    expect(hzAt(5, 0, 3000)).toBe(3000);
  });

  it('is the inverse of leftPercent', () => {
    for (const hz of [200, 958, 1500, 2999]) expect(hzAt(leftPercent(hz, 0, 3000) / 100, 0, 3000)).toBe(hz);
  });
});

describe('signalBandwidthHz', () => {
  it('is the distance between the first and the last tone, as the bars of JTDX', () => {
    expect(signalBandwidthHz('FT8')).toBe(43.75);
    expect(signalBandwidthHz('FT4')).toBe(62.5);
    expect(signalBandwidthHz('JT65')).toBeCloseTo(177.65, 1);
    expect(signalBandwidthHz('JT9')).toBeCloseTo(15.6, 1);
    expect(signalBandwidthHz('T10')).toBe(62.5);
  });

  it('is zero for the other modes', () => {
    expect(signalBandwidthHz('JT')).toBe(0);
    expect(signalBandwidthHz(undefined)).toBe(0);
  });
});

describe('filterWindowHz', () => {
  it('is asymmetric around Rx like the blue marks of JTDX', () => {
    expect(filterWindowHz('FT8')).toEqual({ below: 60, above: 110 });
    expect(filterWindowHz('FT8', true)).toEqual({ below: 290, above: 340 });
    expect(filterWindowHz('FT4')).toEqual({ below: 95, above: 179 });
    expect(filterWindowHz('JT65')).toEqual({ below: 200, above: 375 });
    expect(filterWindowHz('WSPR')).toBeNull();
    expect(filterWindowHz(undefined)).toBeNull();
  });
});

describe('MSK144 sulla cascata', () => {
  it('is centred on the frequency: the bar starts 1000 Hz earlier, the RX window is ±100 Hz', async () => {
    const m = await import('./markers');
    expect(m.signalStartOffsetHz('MSK144')).toBe(-1000);
    expect(m.signalStartOffsetHz('FT8')).toBe(0);
    expect(m.rxWindowHz('MSK144')).toBe(100);
    expect(m.rxWindowHz('FT8')).toBe(50);
  });
});
