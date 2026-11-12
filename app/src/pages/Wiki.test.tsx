import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { LANGS, STRINGS } from '../i18n';
import { GUIDE } from '../i18n/guide';
import type { Entry } from '../i18n/core';
import { Wiki, guideText } from './Wiki';

const everyEntry = (): Entry[] =>
  GUIDE.flatMap((c) => [c.title, ...c.sections.flatMap((s) => [s.title, s.text, s.tip, ...(s.steps ?? [])].filter((e): e is Entry => !!e))]);

describe('Wiki', () => {
  it('shows every chapter in the index and in the page, with numbered steps', () => {
    render(<Wiki />);
    for (const c of GUIDE) {
      const title = guideText(c.title, 'it');
      expect(screen.getByRole('link', { name: title })).toBeInTheDocument();
      expect(screen.getByRole('heading', { level: 2, name: new RegExp(title) })).toBeInTheDocument();
    }
    expect(document.querySelectorAll('.guide-steps li').length).toBeGreaterThan(20);
  });

  it('every interface name quoted in the guide exists, so nothing shows up as [[key]] in any language', () => {
    for (const entry of everyEntry()) {
      for (const text of entry) {
        for (const [, key] of text.matchAll(/\[\[([\w.]+)\]\]/g)) {
          expect(key! in STRINGS, `chiave sconosciuta nella guida: ${key}`).toBe(true);
        }
      }
      for (const lang of LANGS) {
        expect(guideText(entry, lang)).not.toMatch(/\[\[/);
        expect(guideText(entry, lang).trim()).not.toBe('');
      }
    }
  });

  it('uses the real interface names: the radio tab and the Filter button', () => {
    render(<Wiki />);
    expect(screen.getAllByText(/Impostazioni › Stazione › Radio/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Spegni Filtro/).length).toBe(1);
  });

  it('filters the sections with the search box', () => {
    render(<Wiki />);
    fireEvent.change(screen.getByRole('searchbox', { name: 'Cerca nella guida…' }), { target: { value: 'Log4OM' } });
    expect(screen.getByRole('heading', { level: 3, name: 'Log4OM: la radio tramite Aurora' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { level: 3, name: 'La scheda audio' })).toBeNull();
    fireEvent.change(screen.getByRole('searchbox', { name: 'Cerca nella guida…' }), { target: { value: 'zzzz-niente' } });
    expect(screen.getByText('Niente trovato per questa ricerca.')).toBeInTheDocument();
  });
});
