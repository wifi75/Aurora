import { describe, expect, it } from 'vitest';
import { defaultDeviceName } from './deviceName';

describe('defaultDeviceName', () => {
  it('recognizes an iPhone and an iPad before anything else', () => {
    expect(defaultDeviceName('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit')).toBe('iPhone');
    expect(defaultDeviceName('Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit')).toBe('iPad');
  });

  it('recognizes an Android phone before Linux', () => {
    expect(defaultDeviceName('Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit Chrome Mobile')).toBe('Telefono Android');
  });

  it('recognizes Windows, Mac and Linux computers', () => {
    expect(defaultDeviceName('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit Chrome')).toBe('PC Windows');
    expect(defaultDeviceName('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit')).toBe('Mac');
    expect(defaultDeviceName('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit')).toBe('PC Linux');
  });

  it('falls back to a generic name', () => {
    expect(defaultDeviceName('')).toBe('Browser');
    expect(defaultDeviceName('qualcosa di sconosciuto')).toBe('Browser');
  });
});
