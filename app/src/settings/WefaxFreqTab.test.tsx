import { fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { WEFAX_FREQ_KEY, readWefaxFreq } from '../lib/wefaxFreqs';
import { formatKhz, wefaxPresets } from '../lib/wefaxStations';
import { WefaxFreqTab } from './WefaxFreqTab';

beforeEach(() => localStorage.clear());

const rows = () => screen.getAllByRole('row').slice(1);
const fill = (label: string, value: string) => fireEvent.change(screen.getByLabelText(label), { target: { value } });

describe('WefaxFreqTab', () => {
  it('mostra tutte le frequenze di fabbrica', () => {
    render(<WefaxFreqTab />);
    expect(rows()).toHaveLength(wefaxPresets().length);
    expect(screen.getByRole('cell', { name: formatKhz(13882.5) })).toBeInTheDocument();
  });

  it('aggiunge una frequenza con una banda nuova e rifiuta i doppioni e le frequenze sbagliate', () => {
    render(<WefaxFreqTab />);
    fill('Nominativo', 'IU3ABC');
    fill('Luogo', 'Test');
    fill('Frequenza (kHz)', '50');
    fireEvent.click(screen.getByRole('button', { name: 'Aggiungi' }));
    expect(screen.getByRole('alert')).toHaveTextContent('100 e 30000');
    fill('Frequenza (kHz)', '6000');
    fill('Banda', 'Mie');
    fireEvent.click(screen.getByRole('button', { name: 'Aggiungi' }));
    expect(rows()).toHaveLength(wefaxPresets().length + 1);
    const row = screen.getByRole('cell', { name: /IU3ABC/ }).closest('tr') as HTMLElement;
    expect(within(row).getByRole('cell', { name: 'Mie' })).toBeInTheDocument();
    expect(readWefaxFreq().added).toEqual([expect.objectContaining({ call: 'IU3ABC', khz: 6000, band: 'Mie' })]);
    fill('Nominativo', 'iu3abc');
    fill('Frequenza (kHz)', '6000');
    fireEvent.click(screen.getByRole('button', { name: 'Aggiungi' }));
    expect(screen.getByRole('alert')).toHaveTextContent('c’è già');
  });

  it('modifica e cancella una voce di fabbrica; il ripristino riporta tutto', () => {
    render(<WefaxFreqTab />);
    fireEvent.click(screen.getByRole('button', { name: `Modifica DDK3 ${formatKhz(7880)} kHz` }));
    expect(screen.getByLabelText('Frequenza (kHz)')).toHaveValue('7880');
    fill('Frequenza (kHz)', '7881');
    fireEvent.click(screen.getByRole('button', { name: 'Salva la modifica' }));
    expect(screen.getByRole('cell', { name: formatKhz(7881) })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: `Cancella DDK3 ${formatKhz(7881)} kHz` }));
    expect(rows()).toHaveLength(wefaxPresets().length - 1);
    expect(screen.getByText(/cancellate: 1/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Ripristina quelle di fabbrica' }));
    expect(rows()).toHaveLength(wefaxPresets().length);
    expect(localStorage.getItem(WEFAX_FREQ_KEY)).toBeNull();
    expect(screen.getByRole('cell', { name: formatKhz(7880) })).toBeInTheDocument();
  });
});
