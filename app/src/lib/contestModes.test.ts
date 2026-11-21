import { describe, expect, it } from 'vitest';
import { contestModes } from './contestModes';

describe('modi di un contest dalle note', () => {
  it.each([
    ['CW & SSB', ['ssb', 'cw']],
    ['CW @ SSB', ['ssb', 'cw']],
    ['Phone - CW - Digital', ['ssb', 'cw', 'digi']],
    ['RTTY 75 Baudot', ['rtty']],
    ['PSK63', ['psk']],
    ['Solo CW, sei bande da 160 a 10 metri...', ['cw']],
    ['SSB', ['ssb']],
    ['Digital', ['digi']],
    ['Fonia e CW', ['ssb', 'cw']],
    ['CW/USB/LSB + FT8', ['ssb', 'cw', 'digi']],
    ['FT4 and MSK144', ['digi']],
    ['', ['other']],
    ['Sprint a staffetta', ['other']],
  ])('%s', (notes, expected) => {
    expect(contestModes(notes)).toEqual(expected);
  });

  it('senza modo nelle note guarda il nome, poi «Altri»', () => {
    expect(contestModes('', 'CQ WW RTTY')).toEqual(['rtty']);
    expect(contestModes('Sei bande', 'Gara di primavera')).toEqual(['other']);
    expect(contestModes('CW', 'Alfa RTTY')).toEqual(['cw']);
  });
});
