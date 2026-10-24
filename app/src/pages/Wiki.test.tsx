import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { STRINGS } from '../i18n';
import { WIKI_TOPICS } from '../i18n/strings/wiki';
import { Wiki } from './Wiki';

describe('Wiki', () => {
  it('shows every topic with its title and text', () => {
    render(<Wiki />);
    expect(screen.getByRole('heading', { level: 1, name: 'Guida rapida' })).toBeInTheDocument();
    for (const id of WIKI_TOPICS) {
      expect(screen.getByRole('heading', { level: 2, name: STRINGS[`wiki.${id}.t`][0] })).toBeInTheDocument();
    }
  });

  it('has a title and a text for every listed topic', () => {
    for (const id of WIKI_TOPICS) {
      expect(STRINGS[`wiki.${id}.t`]).toBeDefined();
      expect(STRINGS[`wiki.${id}.b`]).toBeDefined();
    }
  });
});
