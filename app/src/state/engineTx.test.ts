import { describe, expect, it } from 'vitest';
import { parseEngineTx } from './engineTx';

describe('parseEngineTx', () => {
  it('accepts six messages and a selection from 1 to 6', () => {
    const six = ['a', 'b', 'c', 'd', 'e', 'f'];
    expect(parseEngineTx({ messages: six, selected: 2 })).toEqual({ messages: six, selected: 2 });
    expect(parseEngineTx({ messages: six, selected: null })).toEqual({ messages: six, selected: null });
    expect(parseEngineTx({ messages: six, selected: 9 })?.selected).toBeNull();
  });

  it('refuses anything else', () => {
    expect(parseEngineTx(null)).toBeNull();
    expect(parseEngineTx({})).toBeNull();
    expect(parseEngineTx({ messages: ['a'], selected: 1 })).toBeNull();
    expect(parseEngineTx({ messages: [1, 2, 3, 4, 5, 6], selected: 1 })).toBeNull();
  });
});
