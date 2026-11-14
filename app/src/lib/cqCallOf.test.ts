import { describe, expect, it } from 'vitest';
import { cqCallOf } from './digiBand';

describe('cqCallOf', () => {
  it('trova chi chiama CQ nelle ultime righe', () => {
    expect(cqCallOf('DL9ET 599-296-296-296\nTU  EC7YY CQ\nCQ EC7YY EC7YY\n')).toBe('EC7YY');
    expect(cqCallOf('CQ EC7YY')).toBe(null);
    expect(cqCallOf('IU3ABC IU3ABC 599 001 001')).toBe(null);
  });
});
