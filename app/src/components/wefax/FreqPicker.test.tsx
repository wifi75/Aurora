import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { wefaxGroups, wefaxPresets } from '../../lib/wefaxStations';
import { FreqPicker } from './FreqPicker';

const scroll = vi.fn();
beforeEach(() => {
  scroll.mockClear();
  Element.prototype.scrollIntoView = scroll as unknown as typeof Element.prototype.scrollIntoView;
});

describe('FreqPicker', () => {
  it('all apertura evidenzia la stazione impostata per ultima e la porta al centro dell elenco', () => {
    const id = wefaxPresets().find((p) => p.khz === 12750)?.id ?? '';
    render(<FreqPicker groups={wefaxGroups()} value={id} onSelect={() => undefined} />);
    expect(screen.queryByRole('listbox')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Stazione e frequenza' }));
    const selected = screen.getByRole('option', { selected: true });
    expect(selected).toHaveTextContent('12 750 kHz');
    expect(selected).toHaveAttribute('data-active', 'true');
    expect(scroll).toHaveBeenCalledWith({ block: 'center' });
    // l'elemento portato in vista e' proprio quello evidenziato
    expect(scroll.mock.instances[0]).toBe(selected);
  });

  it('un clic su una stazione la sceglie e chiude l elenco; Esc chiude senza scegliere', () => {
    const onSelect = vi.fn();
    render(<FreqPicker groups={wefaxGroups()} value="" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole('button', { name: 'Stazione e frequenza' }));
    fireEvent.click(screen.getByRole('option', { name: /4.228 kHz/ }));
    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ khz: 4228 }));
    expect(screen.queryByRole('listbox')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Stazione e frequenza' }));
    fireEvent.keyDown(screen.getByRole('listbox'), { key: 'Escape' });
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(onSelect).toHaveBeenCalledTimes(1);
  });
});
