import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { applyCqPrefs, defaultCqPrefs, readCqPrefs } from './cqPrefs';

const msg = (text: string, tag: string | null): DecodedMessage => ({ id: 1, time_ms: 0, snr: 0, dt: 0, df: 0, text, tag }) as unknown as DecodedMessage;

describe('applyCqPrefs', () => {
  const list = [msg('CQ JA1ABC PM95', 'cq'), msg('CQ K1XYZ FN42', 'worked_call'), msg('CQ EA8AA IL18', 'new_dxcc'), msg('K1A K2B R-05', null)];
  it('di fabbrica elenca CQ e novità, non i messaggi senza etichetta', () => {
    expect(applyCqPrefs(list, defaultCqPrefs())).toHaveLength(3);
  });
  it('si può togliere una categoria', () => {
    const p = defaultCqPrefs();
    p.tags.worked_call = false;
    expect(applyCqPrefs(list, p).map((d) => d.text)).not.toContain('CQ K1XYZ FN42');
  });
  it('solo le stazioni indicate, con prefisso', () => {
    const p = defaultCqPrefs();
    p.only = ['JA*', 'EA8AA'];
    expect(applyCqPrefs(list, p)).toHaveLength(2);
  });
  it('le stazioni nascoste non compaiono', () => {
    const p = defaultCqPrefs();
    p.hide = ['JA*'];
    expect(applyCqPrefs(list, p).map((d) => d.text)).not.toContain('CQ JA1ABC PM95');
  });
  it('una lettura rovinata torna al valore di fabbrica', () => {
    expect(readCqPrefs({ getItem: () => '{non json' })).toEqual(defaultCqPrefs());
  });
});
