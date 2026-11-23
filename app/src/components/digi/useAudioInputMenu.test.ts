import { describe, expect, it } from 'vitest';
import type { AudioDevices } from '../../api/client';
import { isInputChosen } from './useAudioInputMenu';

const devices = (input: string): AudioDevices =>
  ({ inputs: [], outputs: [], input_in_use: null, output_in_use: null, output_pending: null, default_save_dir: '', system_default: 'default', values: { input } }) as unknown as AudioDevices;

describe('ingresso audio nel menu Configura', () => {
  it('riconosce il dispositivo scelto, anche senza distinguere le maiuscole', () => {
    expect(isInputChosen(devices('Line 1 (Virtual Audio Cable)'), 'line 1 (virtual audio cable)')).toBe(true);
    expect(isInputChosen(devices('Line 1 (Virtual Audio Cable)'), 'Mic 1 (Virtual Audio Cable)')).toBe(false);
  });

  it('vuoto e «predefinito del sistema» valgono lo stesso', () => {
    expect(isInputChosen(devices(''), 'default')).toBe(true);
    expect(isInputChosen(devices('default'), 'default')).toBe(true);
    expect(isInputChosen(devices('Line 1 (Virtual Audio Cable)'), 'default')).toBe(false);
  });
});
