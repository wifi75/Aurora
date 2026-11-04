import { describe, expect, it } from 'vitest';
import { ROW_SECS, WF_STYLES, activeStyle, clampWf, defaultWfPrefs, paletteColors, readWfPrefs, saveWfPrefs, USER_PALETTE } from './wfPrefs';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}

describe('waterfall preferences', () => {
  it('start from the factory values of the WSJT-X Wide Graph', () => {
    const p = defaultWfPrefs();
    expect(p).toMatchObject({ speed: 'wsjtx', binsPerPixel: 4, startHz: 0, navg: 1, flatten: true, spectrumMode: 'current', periodLine: true });
    expect(activeStyle(p)).toBe('wsjtx');
  });

  it('JTDX style keeps the old factory values', () => {
    expect({ ...defaultWfPrefs(), ...WF_STYLES.jtdx }).toMatchObject({
      speed: 'jtdx',
      binsPerPixel: 5,
      startHz: 0,
      navg: 1,
      palette: 'Default',
      scale: true,
      flatten: false,
      gain: 0,
      zero: 0,
      spectrumPercent: 30,
      spectrumMode: 'current',
      gain2d: 0,
      zero2d: 0,
      bars: true,
      freq: false,
    });
  });

  it('row pace: ~52 rows per FT8 period like WSJT-X, 225 like JTDX', () => {
    expect(15 / ROW_SECS.wsjtx).toBeCloseTo(52.08, 1);
    expect(15 / ROW_SECS.jtdx).toBeCloseTo(225, 6);
  });

  it('switch once to the WSJT-X style on upgrade, then keep the user choices', () => {
    const old = { ...defaultWfPrefs(), ...WF_STYLES.jtdx, binsPerPixel: 7, gain: 5 } as Record<string, unknown>;
    delete old.speed;
    delete old.styleApplied;
    const store = memory({ 'aurora-wf-prefs': JSON.stringify(old) });
    const first = readWfPrefs(store);
    expect(activeStyle(first)).toBe('wsjtx');
    expect(first.styleApplied).toBe(true);
    saveWfPrefs(store, { ...first, speed: 'jtdx', binsPerPixel: 6 });
    const again = readWfPrefs(store);
    expect(again.speed).toBe('jtdx');
    expect(again.binsPerPixel).toBe(6);
    expect(activeStyle(again)).toBeNull();
  });

  it('are kept and read back', () => {
    const store = memory();
    saveWfPrefs(store, { ...defaultWfPrefs(), binsPerPixel: 3, gain: -12, palette: 'Linrad', spectrumMode: 'cumulative', flatten: true });
    const back = readWfPrefs(store);
    expect(back.binsPerPixel).toBe(3);
    expect(back.gain).toBe(-12);
    expect(back.palette).toBe('Linrad');
    expect(back.spectrumMode).toBe('cumulative');
    expect(back.flatten).toBe(true);
  });

  it('go back to the factory value when out of the JTDX limits, like WideGraph::WideGraph', () => {
    const store = memory({ 'aurora-wf-prefs': JSON.stringify({ binsPerPixel: 500, startHz: 5000, navg: 0, gain: 99, spectrumPercent: 101, palette: 'Nope', scale: 'yes' }) });
    const p = readWfPrefs(store);
    expect(p.binsPerPixel).toBe(4);
    expect(p.startHz).toBe(0);
    expect(p.navg).toBe(1);
    expect(p.gain).toBe(0);
    expect(p.spectrumPercent).toBe(30);
    expect(p.palette).toBe('Default');
    expect(p.scale).toBe(true);
  });

  it('survive a broken or missing store', () => {
    expect(readWfPrefs(null)).toEqual(defaultWfPrefs());
    expect(readWfPrefs(memory({ 'aurora-wf-prefs': '{nope' }))).toEqual(defaultWfPrefs());
    expect(() =>
      saveWfPrefs(
        {
          setItem: () => {
            throw new Error('no');
          },
        },
        defaultWfPrefs(),
      ),
    ).not.toThrow();
  });

  it('clamp the controls to the limits', () => {
    expect(clampWf('binsPerPixel', 0)).toBe(1);
    expect(clampWf('binsPerPixel', 1000)).toBe(100);
    expect(clampWf('startHz', 9999)).toBe(4900);
    expect(clampWf('gain', -80)).toBe(-50);
    expect(clampWf('navg', Number.NaN)).toBe(1);
  });

  it('give the colour points of the chosen palette or the user one', () => {
    const p = defaultWfPrefs();
    expect(paletteColors(p)).toHaveLength(9);
    const user = { ...p, palette: USER_PALETTE, userPalette: [[0, 0, 0], [255, 255, 255]] as Array<[number, number, number]> };
    expect(paletteColors(user)).toHaveLength(2);
  });
});
