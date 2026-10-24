import { describe, expect, it } from 'vitest';
import { decodeSpectrumFrame } from './stream';

function frame(type: number, seq: bigint, fmin: number, fmax: number, bins: number[]): ArrayBuffer {
  const buf = new ArrayBuffer(17 + bins.length);
  const view = new DataView(buf);
  view.setUint8(0, type);
  view.setBigUint64(1, seq);
  view.setUint32(9, fmin);
  view.setUint32(13, fmax);
  new Uint8Array(buf, 17).set(bins);
  return buf;
}

describe('decodeSpectrumFrame', () => {
  it('reads layout seq/fmin/fmax/bins', () => {
    const f = decodeSpectrumFrame(frame(1, 258n, 10, 3000, [7, 8, 9]));
    expect(f).not.toBeNull();
    expect(f?.seq).toBe(258);
    expect(f?.fMinHz).toBe(10);
    expect(f?.fMaxHz).toBe(3000);
    expect(Array.from(f?.bins ?? [])).toEqual([7, 8, 9]);
  });

  it('returns null on truncated frame', () => {
    expect(decodeSpectrumFrame(new ArrayBuffer(10))).toBeNull();
    expect(decodeSpectrumFrame(new ArrayBuffer(0))).toBeNull();
  });

  it('returns null on wrong type byte', () => {
    expect(decodeSpectrumFrame(frame(2, 1n, 0, 3000, [1, 2]))).toBeNull();
  });

  it('returns null when there are no bins or too many', () => {
    expect(decodeSpectrumFrame(frame(1, 1n, 0, 3000, []))).toBeNull();
    expect(decodeSpectrumFrame(frame(1, 1n, 0, 3000, new Array(4097).fill(0)))).toBeNull();
  });

  it('returns null on an inverted frequency range', () => {
    expect(decodeSpectrumFrame(frame(1, 1n, 3000, 3000, [1]))).toBeNull();
  });
});
