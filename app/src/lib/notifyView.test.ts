import { describe, expect, it } from 'vitest';
import { defaultNotifyView, normalizeView, readNotifyView, saveNotifyView, workedLook } from './notifyView';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return {
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => {
      data[k] = v;
    },
    data,
  };
}

describe('notify view', () => {
  it('starts like the engine: worked messages are struck, no colour, nothing hidden', () => {
    const view = defaultNotifyView();
    expect(workedLook(view)).toEqual({ strike: true, underline: false, color: false });
    expect(view.workedDontShow).toBe(false);
    expect(view.invert).toBe(false);
    expect(view.redMarker).toBe(true);
    // il nominativo DX si evidenzia come Aurora ha sempre fatto; il resto parte spento
    expect(view).toMatchObject({ highlightDxCall: true, highlightDxGrid: false, differentBackground: false, diffYellow: false, beepFirst: false });
  });

  it('strike and underline exclude each other and blue needs red', () => {
    expect(normalizeView({ ...defaultNotifyView(), workedStriked: true, workedUnderlined: true }).workedUnderlined).toBe(false);
    expect(normalizeView({ ...defaultNotifyView(), workedStriked: false, workedUnderlined: true })).toMatchObject({ workedStriked: false, workedUnderlined: true });
    expect(normalizeView({ ...defaultNotifyView(), redMarker: false, blueMarker: true }).blueMarker).toBe(false);
  });

  it('saves, reads back and ignores rubbish', () => {
    const store = memory();
    saveNotifyView(store, { ...defaultNotifyView(), workedDontShow: true, workedStriked: false, workedUnderlined: true });
    const back = readNotifyView(store);
    expect(back).toMatchObject({ workedDontShow: true, workedStriked: false, workedUnderlined: true });
    expect(workedLook(back)).toEqual({ strike: false, underline: true, color: false });
    expect(readNotifyView(memory({ 'aurora-notify-view': '{rubbish' }))).toEqual(defaultNotifyView());
    expect(readNotifyView(memory({ 'aurora-notify-view': JSON.stringify({ workedColor: 'si', invert: true }) }))).toMatchObject({ workedColor: false, invert: true });
    expect(readNotifyView(null)).toEqual(defaultNotifyView());
  });
});
