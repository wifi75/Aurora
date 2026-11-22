import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { DigiSettings } from '../../api/client';
import { OLIVIA_DEFAULTS } from '../../lib/olivia';
import { OliviaFields } from './OliviaFields';

const settings = (patch: Partial<DigiSettings> = {}): DigiSettings =>
  ({ mode: 'olivia', rx_freq_hz: 1500, tx_freq_hz: 1500, olivia: { ...OLIVIA_DEFAULTS.olivia }, contestia: { ...OLIVIA_DEFAULTS.contestia }, ...patch }) as DigiSettings;

describe('OliviaFields', () => {
  it('toni e banda salvano tutto l oggetto del modo', () => {
    const onSave = vi.fn();
    render(<OliviaFields settings={settings()} mode="olivia" onSave={onSave} />);
    const [tones, bw] = screen.getAllByRole('combobox');
    if (!tones || !bw) throw new Error('tendine');
    fireEvent.change(tones, { target: { value: '32' } });
    expect(onSave).toHaveBeenLastCalledWith({ olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 32 } });
    fireEvent.change(bw, { target: { value: '1000' } });
    expect(onSave).toHaveBeenLastCalledWith({ olivia: { ...OLIVIA_DEFAULTS.olivia, bandwidth_hz: 1000 } });
  });

  it('nessun avviso con valori che entrano, avviso chiaro quando escono dalla banda audio', () => {
    const { rerender } = render(<OliviaFields settings={settings()} mode="olivia" onSave={vi.fn()} />);
    expect(screen.queryByRole('alert')).toBeNull();
    rerender(<OliviaFields settings={settings({ tx_freq_hz: 3500, olivia: { ...OLIVIA_DEFAULTS.olivia, tones: 32, bandwidth_hz: 2000 } })} mode="olivia" onSave={vi.fn()} />);
    expect(screen.getByRole('alert').textContent).toMatch(/32/);
    expect(screen.getByRole('alert').textContent).toMatch(/3500/);
  });

  it('il margine fuori da 2-32 non si salva', () => {
    const onSave = vi.fn();
    render(<OliviaFields settings={settings()} mode="olivia" onSave={onSave} />);
    const [margin] = screen.getAllByRole('spinbutton');
    if (!margin) throw new Error('margine');
    fireEvent.change(margin, { target: { value: '40' } });
    fireEvent.blur(margin);
    expect(onSave).not.toHaveBeenCalled();
    fireEvent.change(margin, { target: { value: '12' } });
    fireEvent.blur(margin);
    expect(onSave).toHaveBeenLastCalledWith({ olivia: { ...OLIVIA_DEFAULTS.olivia, sync_margin: 12 } });
  });
});
