import { describe, expect, it } from 'vitest';
import { txAudioWarning } from './txAudio';

describe('Tx audio frequency warning', () => {
  it('warns when the signal goes above 2700 Hz or starts below 200 Hz, counting the width of the mode', () => {
    expect(txAudioWarning(1500, 'FT8')).toBeNull();
    expect(txAudioWarning(2650, 'FT8')).toBeNull();
    expect(txAudioWarning(2680, 'FT8')).toBe('high');
    expect(txAudioWarning(2600, 'JT65A')).toBe('high');
    expect(txAudioWarning(150, 'FT8')).toBe('low');
    expect(txAudioWarning(200, 'FT8')).toBeNull();
  });

  it('says nothing without a frequency', () => {
    expect(txAudioWarning(null, 'FT8')).toBeNull();
    expect(txAudioWarning(Number.NaN, 'FT8')).toBeNull();
    expect(txAudioWarning(0, 'FT8')).toBeNull();
  });
});
