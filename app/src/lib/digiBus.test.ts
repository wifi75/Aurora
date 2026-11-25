import { describe, expect, it, vi } from 'vitest';
import { clearDigiText, emitDigi, getDigiProvisional, getDigiText, getExtraProvisional, type DigiStreamEvent } from './digiBus';

const ev = (e: Record<string, unknown>) => ({ type: 'digi', ...e }) as unknown as DigiStreamEvent;

describe('CW veloce: testo provvisorio', () => {
  it('il provvisorio si sostituisce e non entra nel testo', () => {
    clearDigiText();
    emitDigi(ev({ kind: 'rx_prov', text: 'CQ TE' }));
    expect(getDigiProvisional()).toBe('CQ TE');
    emitDigi(ev({ kind: 'rx_text', text: 'CQ ' }));
    emitDigi(ev({ kind: 'rx_prov', text: 'TEST' }));
    expect(getDigiText()).toBe('CQ ');
    expect(getDigiProvisional()).toBe('TEST');
    clearDigiText();
    expect(getDigiProvisional()).toBe('');
  });

  it('quello di una finestra in piu\' resta suo', () => {
    emitDigi(ev({ kind: 'rx_prov', text: 'K1X', rx_id: 2 }));
    expect(getExtraProvisional(2)).toBe('K1X');
    expect(getDigiProvisional()).toBe('');
  });
});

describe('ricezione: a capo dopo una pausa', () => {
  it("in CW una pausa di 2 s fra due parole non va a capo (ON7KEC, #110), una di 6 s si'", () => {
    const now = vi.spyOn(Date, 'now');
    now.mockReturnValue(1_000_000);
    clearDigiText();
    emitDigi(ev({ kind: 'rx_text', text: 'CQ ', mode: 'cw' }));
    now.mockReturnValue(1_002_000);
    emitDigi(ev({ kind: 'rx_text', text: 'CQ ', mode: 'cw' }));
    expect(getDigiText()).toBe('CQ CQ ');
    now.mockReturnValue(1_008_000);
    emitDigi(ev({ kind: 'rx_text', text: 'OP4F ', mode: 'cw' }));
    expect(getDigiText()).toBe('CQ CQ \nOP4F ');
    now.mockRestore();
  });

  it("negli altri modi la pausa di 1,5 s resta quella di prima", () => {
    const now = vi.spyOn(Date, 'now');
    now.mockReturnValue(2_000_000);
    clearDigiText();
    emitDigi(ev({ kind: 'rx_text', text: 'CQ ', mode: 'psk31' }));
    now.mockReturnValue(2_002_000);
    emitDigi(ev({ kind: 'rx_text', text: 'DE ', mode: 'psk31' }));
    expect(getDigiText()).toBe('CQ \nDE ');
    now.mockRestore();
  });
});
