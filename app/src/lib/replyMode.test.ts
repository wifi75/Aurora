import { describe, expect, it } from 'vitest';
import { replyAccepted } from './replyMode';

describe('replyAccepted', () => {
  it('accepts everything when the engine takes any message or the setting is unknown', () => {
    expect(replyAccepted('3', 'SP8ADU GI7KMC -08')).toBe(true);
    expect(replyAccepted(undefined, 'SP8ADU GI7KMC -08')).toBe(true);
  });

  it('with "CQ only" (the value the engine has by default) accepts only CQs', () => {
    expect(replyAccepted('1', 'CQ DX F5ABC JN18')).toBe(true);
    expect(replyAccepted('1', 'F5DGQ OY1HJ 73')).toBe(false);
    expect(replyAccepted('1', 'SP8ADU GI7KMC -08')).toBe(false);
    expect(replyAccepted('0', 'F5DGQ OY1HJ 73')).toBe(false);
  });

  it('with "CQ and 73" accepts CQs and the 73 and RR73 messages, not the rest', () => {
    expect(replyAccepted('2', 'CQ SV2CLJ KN10')).toBe(true);
    expect(replyAccepted('2', 'F5DGQ OY1HJ 73')).toBe(true);
    expect(replyAccepted('2', 'S56IZW W3EWL RR73')).toBe(true);
    expect(replyAccepted('2', 'SP8ADU GI7KMC -08')).toBe(false);
    expect(replyAccepted('2', 'K5BNF SV2CLJ KN10')).toBe(false);
  });
});
