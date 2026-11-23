import { describe, expect, it } from 'vitest';
import { DEFAULT_DIGI_WF, DIGI_CLASSIC, classicColor, digiWfTable, readDigiWf } from './digiWfPrefs';

const store = (value: string | null) => ({ getItem: () => value });

describe('colori della cascata dei modi testuali', () => {
  it('di fabbrica sono quelli di sempre: «Classica», guadagno 0 e zero 0', () => {
    expect(DEFAULT_DIGI_WF).toEqual({ palette: DIGI_CLASSIC, gain: 0, zero: 0 });
    const table = digiWfTable(DEFAULT_DIGI_WF);
    expect(table).toHaveLength(256);
    for (const v of [0, 40, 128, 200, 255]) expect(table[v]).toEqual(classicColor(v));
  });

  it('il guadagno e lo zero spostano la scala, una tavolozza di JTDX cambia i colori', () => {
    const base = digiWfTable(DEFAULT_DIGI_WF);
    const gained = digiWfTable({ ...DEFAULT_DIGI_WF, gain: 20 });
    // con piu' guadagno un valore medio arriva a un colore piu' alto della scala
    expect(gained[100]).toEqual(classicColor(Math.min(255, Math.round(100 * 10 ** 0.4))));
    expect(gained[100]).not.toEqual(base[100]);
    // lo zero abbassato scurisce il fondo
    expect(digiWfTable({ ...DEFAULT_DIGI_WF, zero: -50 })[60]).toEqual(classicColor(0));
    // la tavolozza «Banana» di JTDX parte dal nero e arriva al bianco
    const banana = digiWfTable({ palette: 'Banana', gain: 0, zero: 0 });
    expect(banana[0]).toEqual([0, 0, 0]);
    expect(banana[255]!.every((c) => c >= 250)).toBe(true);
  });

  it('una scelta salvata illeggibile o sconosciuta torna a quella di fabbrica', () => {
    expect(readDigiWf(store(null))).toEqual(DEFAULT_DIGI_WF);
    expect(readDigiWf(store('non json'))).toEqual(DEFAULT_DIGI_WF);
    expect(readDigiWf(store('{"palette":"Nixie","gain":999,"zero":"x"}'))).toEqual({ palette: DIGI_CLASSIC, gain: 50, zero: 0 });
    expect(readDigiWf(store('{"palette":"Banana","gain":-5,"zero":3}'))).toEqual({ palette: 'Banana', gain: -5, zero: 3 });
  });
});
