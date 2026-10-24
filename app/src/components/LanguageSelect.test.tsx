import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { LANGS, useLang } from '../i18n';
import { LanguageSelect } from './LanguageSelect';
import { StopButton } from './StopButton';
import { TxBanner } from './TxBanner';

describe('LanguageSelect', () => {
  it('offers the six languages', () => {
    render(<LanguageSelect />);
    const options = screen.getAllByRole('option').map((o) => (o as HTMLOptionElement).value);
    expect(options).toEqual([...LANGS]);
  });

  it('changes every text of the interface at once and remembers the choice', () => {
    render(
      <>
        <LanguageSelect />
        <TxBanner status={{ dial_hz: 1, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'AA1AA', de_grid: 'FN42' }} />
        <StopButton />
      </>,
    );
    expect(screen.getByText('IN ASCOLTO')).toBeInTheDocument();
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'de' } });
    expect(screen.getByText('EMPFANG')).toBeInTheDocument();
    expect(useLang.getState().lang).toBe('de');
    expect(localStorage.getItem('aurora-lang')).toBe('de');
    expect(document.documentElement.lang).toBe('de');
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'pt' } });
    expect(screen.getByText('À ESCUTA')).toBeInTheDocument();
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'it' } });
    expect(screen.getByText('IN ASCOLTO')).toBeInTheDocument();
  });

  it('gives the stop button a tooltip in the chosen language', () => {
    render(<StopButton />);
    expect(screen.getByRole('button', { name: 'STOP TX' })).toHaveAttribute('title', expect.stringContaining('Ferma subito'));
    useLang.getState().setLang('en');
  });
});
