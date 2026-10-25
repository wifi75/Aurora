import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { LANGS, useLang } from '../i18n';
import { LANG_FLAGS, LanguageSelect } from './LanguageSelect';
import { StopButton } from './StopButton';
import { TxBanner } from './TxBanner';

/** Apre il menu e sceglie la lingua con il mouse. */
function pick(name: string) {
  fireEvent.click(screen.getByRole('combobox'));
  fireEvent.click(screen.getByRole('option', { name }));
}

describe('LanguageSelect', () => {
  it('offers the six languages, each with its flag', () => {
    const { container } = render(<LanguageSelect />);
    fireEvent.click(screen.getByRole('combobox'));
    const options = screen.getAllByRole('option');
    expect(options).toHaveLength(LANGS.length);
    expect(options.map((o) => o.textContent)).toEqual(['Italiano', 'English', 'Deutsch', 'Français', 'Español', 'Português']);
    const flags = [...container.querySelectorAll('.lang-list .fi')].map((f) => [...f.classList].find((c) => c.startsWith('fi-')));
    expect(flags).toEqual(['fi-it', 'fi-gb', 'fi-de', 'fi-fr', 'fi-es', 'fi-pt']);
    expect(LANG_FLAGS.en).toBe('gb');
    // la bandiera della lingua scelta sta anche sul pulsante
    expect(container.querySelector('.lang-trigger .fi-it')).not.toBeNull();
  });

  it('has the right aria roles and states', () => {
    render(<LanguageSelect />);
    const trigger = screen.getByRole('combobox', { name: 'Lingua' });
    expect(trigger).toHaveAttribute('aria-haspopup', 'listbox');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('listbox')).toBeNull();
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    const list = screen.getByRole('listbox', { name: 'Lingua' });
    expect(trigger).toHaveAttribute('aria-controls', list.id);
    expect(screen.getByRole('option', { name: 'Italiano' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('option', { name: 'English' })).toHaveAttribute('aria-selected', 'false');
    expect(trigger).toHaveAttribute('aria-activedescendant', screen.getByRole('option', { name: 'Italiano' }).id);
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
    pick('Deutsch');
    expect(screen.getByText('EMPFANG')).toBeInTheDocument();
    expect(useLang.getState().lang).toBe('de');
    expect(localStorage.getItem('aurora-lang')).toBe('de');
    expect(document.documentElement.lang).toBe('de');
    // il menu si chiude dopo la scelta
    expect(screen.getByRole('combobox')).toHaveAttribute('aria-expanded', 'false');
    pick('Português');
    expect(screen.getByText('À ESCUTA')).toBeInTheDocument();
    pick('Italiano');
    expect(screen.getByText('IN ASCOLTO')).toBeInTheDocument();
  });

  it('works from the keyboard: arrows move, Enter chooses, Esc closes', () => {
    render(<LanguageSelect />);
    const trigger = screen.getByRole('combobox');
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    expect(trigger).toHaveAttribute('aria-activedescendant', screen.getByRole('option', { name: 'Deutsch' }).id);
    fireEvent.keyDown(trigger, { key: 'Enter' });
    expect(useLang.getState().lang).toBe('de');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');

    fireEvent.keyDown(trigger, { key: 'ArrowUp' });
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    expect(trigger).toHaveAttribute('aria-activedescendant', screen.getByRole('option', { name: 'Deutsch' }).id);
    fireEvent.keyDown(trigger, { key: 'Escape' });
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(useLang.getState().lang).toBe('de');

    fireEvent.keyDown(trigger, { key: 'End' });
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    fireEvent.keyDown(trigger, { key: 'End' });
    fireEvent.keyDown(trigger, { key: ' ' });
    expect(useLang.getState().lang).toBe('pt');
  });

  it('stops at the ends of the list and closes with a click outside', () => {
    render(<LanguageSelect />);
    const trigger = screen.getByRole('combobox');
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    for (let i = 0; i < 10; i++) fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    expect(trigger).toHaveAttribute('aria-activedescendant', screen.getByRole('option', { name: 'Português' }).id);
    for (let i = 0; i < 10; i++) fireEvent.keyDown(trigger, { key: 'ArrowUp' });
    expect(trigger).toHaveAttribute('aria-activedescendant', screen.getByRole('option', { name: 'Italiano' }).id);
    fireEvent.pointerDown(document.body);
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
  });

  it('gives the stop button a tooltip in the chosen language', () => {
    render(<StopButton />);
    expect(screen.getByRole('button', { name: 'STOP TX' })).toHaveAttribute('title', expect.stringContaining('Ferma subito'));
    useLang.getState().setLang('en');
  });
});
