import { describe, expect, it } from 'vitest';
import { chooseBackend } from './backend';

describe('chooseBackend', () => {
  it('prefers webgpu', () => {
    expect(chooseBackend({ hasWebGPU: true, hasWebGL2: true })).toBe('webgpu');
  });

  it('falls back to webgl2', () => {
    expect(chooseBackend({ hasWebGPU: false, hasWebGL2: true })).toBe('webgl2');
  });

  it('returns none when neither exists', () => {
    expect(chooseBackend({ hasWebGPU: false, hasWebGL2: false })).toBe('none');
  });

  it('honors forced backend only if available', () => {
    expect(chooseBackend({ hasWebGPU: true, hasWebGL2: true, forced: 'webgl2' })).toBe('webgl2');
    expect(chooseBackend({ hasWebGPU: false, hasWebGL2: true, forced: 'webgpu' })).toBe('webgl2');
    expect(chooseBackend({ hasWebGPU: true, hasWebGL2: false, forced: 'webgl2' })).toBe('webgpu');
  });
});
