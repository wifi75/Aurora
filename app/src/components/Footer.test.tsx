import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Footer } from './Footer';

function renderFooter() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <Footer />
    </QueryClientProvider>,
  );
}

describe('Footer', () => {
  it('shows name version and author on one line', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ status: 'ok', name: 'Aurora', version: '0.1.0', author: 'Tiziano Cassone' }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );
    renderFooter();
    expect(await screen.findByText('Aurora v0.1.0 — Progettazione e sviluppo: Tiziano Cassone')).toBeInTheDocument();
  });

  it('shows a clear message when the core is unreachable', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('rete assente')));
    renderFooter();
    expect(await screen.findByText(/Core non raggiungibile/)).toBeInTheDocument();
  });
});
