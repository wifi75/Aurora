import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { ANSWER_WINDOW_MS, isAnswerToMyCall, transmittedTo } from './answered';

const tx = (text: string, time_ms: number): DecodedMessage => ({ id: -1, time_ms, snr: 0, delta_time: 0, freq_hz: 1500, text, tag: null, country: null });

describe('risposta alla mia chiamata', () => {
  it('riconosce a chi ho trasmesso e ignora i CQ', () => {
    const sent = transmittedTo([tx('A65JR IU3WJQ JN55', 1000), tx('CQ IU3WJQ JN55', 2000)]);
    expect([...sent.keys()]).toEqual(['A65JR']);
  });

  it('un messaggio a me da una stazione a cui ho trasmesso da poco e\' la sua risposta', () => {
    const sent = transmittedTo([tx('A65JR IU3WJQ JN55', 10_000)]);
    expect(isAnswerToMyCall('IU3WJQ A65JR R+02', 40_000, sent)).toBe(true);
  });

  it('se mi chiama per prima, o e\' passato troppo tempo, resta una chiamata', () => {
    const sent = transmittedTo([tx('A65JR IU3WJQ JN55', 10_000)]);
    expect(isAnswerToMyCall('IU3WJQ XX9XX KN23', 40_000, sent)).toBe(false);
    expect(isAnswerToMyCall('IU3WJQ A65JR R+02', 10_000 + ANSWER_WINDOW_MS + 1, sent)).toBe(false);
  });
});
