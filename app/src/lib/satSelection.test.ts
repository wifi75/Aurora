import { describe, expect, it } from 'vitest';
import { clearSatellites, groupListing, normalizeSatSelection, pickSatellite, setTracked } from './satSelection';

describe('normalizeSatSelection', () => {
  it('keeps a selected satellite that is ticked', () => {
    expect(normalizeSatSelection({ selected: 25544, tracked: [25544, 44909] })).toEqual({ selected: 25544, changed: false });
  });
  it('on opening, a selection that is not ticked becomes the first ticked satellite', () => {
    expect(normalizeSatSelection({ selected: 25544, tracked: [44909, 7] })).toEqual({ selected: 44909, changed: true });
    expect(normalizeSatSelection({ selected: null, tracked: [7, 44909] })).toEqual({ selected: 7, changed: true });
  });
  it('with nothing ticked there is no selection', () => {
    expect(normalizeSatSelection({ selected: 25544, tracked: [] })).toEqual({ selected: null, changed: true });
    expect(normalizeSatSelection({ selected: null, tracked: [] })).toEqual({ selected: null, changed: false });
  });
});

describe('pickSatellite', () => {
  it('selects and ticks a satellite that was not ticked', () => {
    expect(pickSatellite({ selected: null, tracked: [1] }, 7)).toEqual({ selected: 7, tracked: [1, 7] });
  });
  it('does not duplicate an already ticked satellite', () => {
    expect(pickSatellite({ selected: 1, tracked: [1, 7] }, 7)).toEqual({ selected: 7, tracked: [1, 7] });
  });
});

describe('setTracked', () => {
  it('ticking with no selection also selects that satellite', () => {
    expect(setTracked({ selected: null, tracked: [] }, 2, true)).toEqual({ selected: 2, tracked: [2] });
  });
  it('ticking with a selection keeps it', () => {
    expect(setTracked({ selected: 1, tracked: [1] }, 2, true)).toEqual({ selected: 1, tracked: [1, 2] });
  });
  it('unticking the selected satellite moves the selection to the next ticked one', () => {
    expect(setTracked({ selected: 1, tracked: [1, 2, 3] }, 1, false)).toEqual({ selected: 2, tracked: [2, 3] });
  });
  it('unticking the last ticked satellite clears the selection', () => {
    expect(setTracked({ selected: 1, tracked: [1] }, 1, false)).toEqual({ selected: null, tracked: [] });
  });
  it('unticking another satellite keeps the selection', () => {
    expect(setTracked({ selected: 1, tracked: [1, 2] }, 2, false)).toEqual({ selected: 1, tracked: [1] });
  });
  it('ticking twice does not duplicate', () => {
    expect(setTracked({ selected: null, tracked: [2] }, 2, true).tracked).toEqual([2]);
  });
});

describe('clearSatellites and groupListing', () => {
  it('clears everything', () => {
    expect(clearSatellites()).toEqual({ selected: null, tracked: [] });
  });
  it('puts ticked satellites first and keeps each group in order', () => {
    const list = [{ norad: 3 }, { norad: 1 }, { norad: 2 }, { norad: 4 }];
    expect(groupListing(list, [4, 1])).toEqual({ ticked: [{ norad: 1 }, { norad: 4 }], rest: [{ norad: 3 }, { norad: 2 }] });
    expect(groupListing(list, [])).toEqual({ ticked: [], rest: list });
  });
});
