import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
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
  it('no longer shows the signature (it is in the window title) once the core answers', async () => {
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
    await waitFor(() => expect(screen.queryByText(/Aurora v0\.1\.0/)).toBeNull());
    expect(screen.getByTitle('UTC')).toBeInTheDocument();
  });

  it('shows a clear message when the core is unreachable', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('rete assente')));
    renderFooter();
    expect(await screen.findByText(/Core non raggiungibile/)).toBeInTheDocument();
  });
});
