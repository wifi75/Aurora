import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { NOVITA } from '../lib/novita';
import { News, groupByVersion, releaseLabel } from './News';

describe('News', () => {
  it('shows every entry as one compact row with its version, date and time', () => {
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <News />
      </QueryClientProvider>,
    );
    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument();
    expect(document.querySelectorAll('.news-row')).toHaveLength(NOVITA.length);
    expect(document.querySelectorAll('time[datetime]').length).toBe(NOVITA.length);
  });

  it('groups consecutive entries of the same version and formats the release time', () => {
    const g = groupByVersion(NOVITA);
    expect(g.flatMap((x) => x.items)).toHaveLength(NOVITA.length);
    expect(releaseLabel('2026-11-01T07:42:35Z', 'it')).toMatch(/2026/);
    expect(releaseLabel('non una data', 'it')).toBe('non una data');
  });
});
