import { describe, expect, it } from 'vitest';
import { foxCalls, foxSender, lineSender } from './fox';

describe('stazioni Fox/Hound', () => {
  it('riconosce le righe del Fox di WSJT-X e non le risposte multiple di MSHV', () => {
    expect(foxSender('K1ABC RR73; W9XYZ <KH1/KH7Z> -08')).toBe('KH1/KH7Z');
    expect(foxSender('IU3WJQ V26JJ R+03')).toBeNull();
    expect(foxSender('R2PA V26JJ RR73')).toBeNull();
    const fox = foxCalls(['K1ABC RR73; W9XYZ <KH1/KH7Z> -08', 'R2PA V26JJ RR73', 'R7RDE V26JJ -04']);
    expect([...fox]).toEqual(['KH1/KH7Z']);
    expect(lineSender('IU3ABC KH1/KH7Z -12')).toBe('KH1/KH7Z');
    expect(lineSender('CQ DX V26JJ FK97')).toBe('V26JJ');
  });

  it('ignores the DXpedition message of a normal station above 1000 Hz', () => {
    const m = 'SP6PST RR73; K1ABC <G3VCQ> -07';
    expect(foxCalls([{ text: m, freq_hz: 1650 }]).has('G3VCQ')).toBe(false);
    expect(foxCalls([{ text: m, freq_hz: 600 }]).has('G3VCQ')).toBe(true);
  });
});
