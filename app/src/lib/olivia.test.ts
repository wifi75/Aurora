import { describe, expect, it, vi } from 'vitest';
import type { DigiSettings } from '../api/client';
import { MODI_IN_ARRIVO, applyAvailability, disponibileIn } from './menuModi';
import { macroState, runCommand, type MacroDeps } from './macroRuntime';
import { OLIVIA_DEFAULTS, effectiveTxHz, nearest, oliviaRounded, oliviaFits, oliviaModePatch, oliviaOf, oliviaPatch, oliviaProblem, oliviaSetPatch, oliviaValid, parseOliviaName } from './olivia';

const base = (patch: Partial<DigiSettings> = {}): DigiSettings =>
  ({
    mode: 'psk31',
    rx_freq_hz: 1500,
    tx_freq_hz: 1500,
    tx_follow_rx: true,
    cw_bandwidth_hz: 150,
    passband_on: false,
    passband_hz: 0,
    olivia: { ...OLIVIA_DEFAULTS.olivia },
    contestia: { ...OLIVIA_DEFAULTS.contestia },
    ...patch,
  }) as DigiSettings;

function deps(s: DigiSettings) {
  const save = vi.fn();
  const toast = vi.fn();
  const d = { t: (k: string) => k, toast, settings: () => s, save } as unknown as MacroDeps;
  return { d, save, toast };
}

describe('Olivia e Contestia: toni e banda nella banda audio', () => {
  it('i valori di base a 1500 Hz entrano', () => {
    expect(oliviaFits(8, 250, 1500)).toBe(true);
    expect(oliviaFits(16, 500, 1500)).toBe(true);
    expect(oliviaFits(32, 1000, 1500)).toBe(true);
  });

  it('32 toni in 2000 Hz a 3500 Hz escono, come nel motore', () => {
    expect(oliviaFits(32, 2000, 3500)).toBe(false);
    expect(oliviaFits(32, 2000, 3400)).toBe(false);
    expect(oliviaFits(8, 250, 3400)).toBe(true);
  });

  it('oliviaProblem dice se il guaio e in trasmissione o in ricezione, e niente negli altri modi', () => {
    expect(oliviaProblem(base({ mode: 'olivia' }))).toBeNull();
    expect(oliviaProblem(base({ mode: 'olivia', tx_follow_rx: false, tx_freq_hz: 3500, olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 32, bandwidth_hz: 2000 } }))).toBe('tx');
    expect(oliviaProblem(base({ mode: 'contestia', tx_follow_rx: false, rx_freq_hz: 3500, contestia: { ...OLIVIA_DEFAULTS.contestia, tones: 32, bandwidth_hz: 2000 } }))).toBe('rx');
    expect(oliviaProblem(base({ mode: 'psk31', tx_freq_hz: 3500 }))).toBeNull();
    // con la trasmissione che segue la ricezione conta la frequenza di ricezione, come nel motore (tx_freq_hz resta vecchio)
    expect(oliviaProblem(base({ mode: 'olivia', tx_follow_rx: true, rx_freq_hz: 3400, tx_freq_hz: 1000, olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 16, bandwidth_hz: 2000 } }))).toBe('tx');
    expect(effectiveTxHz(base({ tx_follow_rx: false, rx_freq_hz: 3400, tx_freq_hz: 1000 }))).toBe(1000);
  });
});

describe('Olivia e Contestia: impostazioni', () => {
  it('validazione con i limiti del motore', () => {
    expect(oliviaValid(OLIVIA_DEFAULTS.olivia)).toBe(true);
    expect(oliviaValid({ ...OLIVIA_DEFAULTS.olivia, tones: 12 })).toBe(false);
    expect(oliviaValid({ ...OLIVIA_DEFAULTS.olivia, bandwidth_hz: 125 })).toBe(false);
    expect(oliviaValid({ ...OLIVIA_DEFAULTS.olivia, sync_margin: 1 })).toBe(false);
    expect(oliviaValid({ ...OLIVIA_DEFAULTS.olivia, sync_integration: 33 })).toBe(false);
    expect(oliviaValid({ ...OLIVIA_DEFAULTS.olivia, squelch_snr_db: Number.NaN })).toBe(false);
    expect(oliviaValid({ ...OLIVIA_DEFAULTS.olivia, squelch_snr_db: 41 })).toBe(false);
  });

  it('senza i campi (motore di prima) valgono quelli di base', () => {
    const s = base();
    delete s.olivia;
    expect(oliviaOf(s, 'olivia')).toEqual(OLIVIA_DEFAULTS.olivia);
    expect(oliviaOf(s, 'contestia')).toEqual({ tones: 16, bandwidth_hz: 500, sync_margin: 8, sync_integration: 4, squelch_snr_db: 12 });
  });

  it('il cambio manda sempre tutto l oggetto del modo', () => {
    expect(oliviaPatch(base(), 'contestia', { tones: 32 })).toEqual({ contestia: { ...OLIVIA_DEFAULTS.contestia, tones: 32 } });
  });

  it('nearest arrotonda al valore ammesso', () => {
    expect(nearest([250, 500, 1000, 2000], 600)).toBe(500);
    expect(nearest([4, 8, 16, 32, 64], 40)).toBe(32);
  });
});

describe('Olivia e Contestia: nomi dei modi e macro', () => {
  it('legge i nomi di fldigi', () => {
    expect(parseOliviaName('OLIVIA')).toEqual({ mode: 'olivia' });
    expect(parseOliviaName('OLIVIA-8-250')).toEqual({ mode: 'olivia', tones: 8, bandwidth_hz: 250 });
    expect(parseOliviaName('Contestia 16/500')).toEqual({ mode: 'contestia', tones: 16, bandwidth_hz: 500 });
    expect(parseOliviaName('olivia-32-1000')).toEqual({ mode: 'olivia', tones: 32, bandwidth_hz: 1000 });
    expect(parseOliviaName('BPSK31')).toBeNull();
    expect(parseOliviaName('MFSK16')).toBeNull();
  });

  it('il cambio di modo porta toni e banda se il nome li dice', () => {
    expect(oliviaModePatch(base(), 'CONTESTIA')).toEqual({ mode: 'contestia' });
    expect(oliviaModePatch(base(), 'OLIVIA-32-1000')).toEqual({ mode: 'olivia', olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 32, bandwidth_hz: 1000 } });
    expect(oliviaModePatch(base(), 'RTTY')).toBeNull();
  });

  it('SET TONES, BW e SUBMODE', () => {
    const ol = base({ mode: 'olivia' });
    expect(oliviaSetPatch(ol, 'TONES', '16')).toEqual({ olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 16 } });
    expect(oliviaSetPatch(ol, 'BW', '1000')).toEqual({ olivia: { ...OLIVIA_DEFAULTS.olivia, bandwidth_hz: 1000 } });
    expect(oliviaSetPatch(ol, 'SUBMODE', '32/1000')).toEqual({ olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 32, bandwidth_hz: 1000 } });
    expect(oliviaSetPatch(ol, 'SUBMODE', 'CONTESTIA-16-500')).toEqual({ mode: 'contestia', contestia: { ...OLIVIA_DEFAULTS.contestia } });
    // negli altri modi TONES e BW non riguardano Olivia
    expect(oliviaSetPatch(base(), 'TONES', '16')).toBeNull();
    expect(oliviaSetPatch(base({ mode: 'cw' }), 'BW', '250')).toBeNull();
    expect(oliviaSetPatch(base(), 'SUBMODE', 'RTTY')).toBeNull();
  });

  it('un nome con toni o banda non ammessi si segnala', () => {
    expect(oliviaRounded('OLIVIA-8-125')).toBe(true);
    expect(oliviaRounded('OLIVIA-8-250')).toBe(false);
    expect(oliviaRounded('12/500')).toBe(true);
    expect(oliviaRounded('CONTESTIA')).toBe(false);
  });

  it('i comandi <SET ...> arrivano ai campi del modo', () => {
    macroState.modeLocked = false;
    const ol = base({ mode: 'olivia' });
    let r = deps(ol);
    runCommand('SET', 'TONES:32', r.d);
    expect(r.save).toHaveBeenCalledWith({ olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 32 } });
    r = deps(ol);
    runCommand('SET', 'BW:500', r.d);
    expect(r.save).toHaveBeenCalledWith({ olivia: { ...OLIVIA_DEFAULTS.olivia, bandwidth_hz: 500 } });
    r = deps(base());
    runCommand('SET', 'SUBMODE:OLIVIA-8-250', r.d);
    expect(r.save).toHaveBeenCalledWith({ mode: 'olivia', olivia: { ...OLIVIA_DEFAULTS.olivia } });
    r = deps(base());
    runCommand('SET', 'SUBMODE:CONTESTIA', r.d);
    expect(r.save).toHaveBeenCalledWith({ mode: 'contestia' });
  });

  it('negli altri modi i comandi <SET ...> fanno come prima', () => {
    macroState.modeLocked = false;
    let r = deps(base({ mode: 'cw' }));
    runCommand('SET', 'BW:240', r.d);
    expect(r.save).toHaveBeenCalledWith({ cw_bandwidth_hz: 250 });
    r = deps(base());
    runCommand('SET', 'BW:300', r.d);
    expect(r.save).toHaveBeenCalledWith({ passband_on: true, passband_hz: 300 });
    r = deps(base());
    runCommand('SET', 'SUBMODE:RTTY', r.d);
    expect(r.save).toHaveBeenCalledWith({ mode: 'rtty' });
    r = deps(base());
    runCommand('SET', 'TONES:16', r.d);
    expect(r.save).not.toHaveBeenCalled();
  });

  it('con il modo bloccato SUBMODE non cambia modo', () => {
    macroState.modeLocked = true;
    const r = deps(base());
    runCommand('SET', 'SUBMODE:OLIVIA', r.d);
    expect(r.save).not.toHaveBeenCalled();
    macroState.modeLocked = false;
  });

  it('MODEMACROS accetta Olivia e Contestia', () => {
    const setMacroMode = vi.fn();
    const d = { ...deps(base()).d, setMacroMode } as MacroDeps;
    runCommand('MODEMACROS', 'olivia.mc', d);
    expect(setMacroMode).toHaveBeenCalledWith('olivia');
    runCommand('MODEMACROS', 'contestia', d);
    expect(setMacroMode).toHaveBeenCalledWith('contestia');
  });
});

describe('menu dei modi', () => {
  it('Olivia e Contestia non sono piu fra i modi in arrivo', () => {
    expect(MODI_IN_ARRIVO).not.toContain('Olivia');
    expect(MODI_IN_ARRIVO).not.toContain('Contestia');
    expect(MODI_IN_ARRIVO).not.toContain('QPSK31');
    expect(MODI_IN_ARRIVO).toContain('MFSK');
  });

  it('le voci dei modi a testo valgono anche in Olivia e Contestia', () => {
    expect(disponibileIn('digi.mi.clearRx', 'olivia')).toBe(true);
    expect(disponibileIn('digi.mi.threshold', 'contestia')).toBe(true);
    expect(disponibileIn('digi.mi.clearRx', 'ssb')).toBe(false);
    const t = ((k: string) => k) as never;
    const menus = applyAvailability([{ label: 'digi.menu.mode', items: [{ label: 'digi.mi.threshold', onSelect: () => undefined }] }], 'olivia', t, 'tip');
    expect(menus[0]?.items[0]).toMatchObject({ label: 'digi.mi.threshold' });
    expect(menus[0]?.items[0]).not.toMatchObject({ disabled: true });
  });
});
