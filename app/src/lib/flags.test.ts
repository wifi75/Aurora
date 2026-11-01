import { describe, expect, it } from 'vitest';
import { flagCode } from './flags';

describe('flagCode', () => {
  it('recognises the NG3K spellings of DXCC names', () => {
    expect(flagCode('Gambia')).toBe('gm');
    expect(flagCode('Solomon Is')).toBe('sb');
    expect(flagCode('eSwatini')).toBe('sz');
    expect(flagCode('Ogasawara')).toBe('jp');
    expect(flagCode('Atlantide')).toBeNull();
  });
});
