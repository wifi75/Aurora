import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getControls: vi.fn().mockResolvedValue({}), getEngineSettings: vi.fn().mockRejectedValue(new Error('404')), postControl: vi.fn() }));
import { RxControls } from './RxControls';

function show(transmitting: boolean) {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <RxControls mode="FT8" transmitting={transmitting} />
    </QueryClientProvider>,
  );
}

describe('RxControls period bar', () => {
  it('is green while receiving', () => {
    expect(show(false).container.querySelector('.period')).not.toHaveClass('period--tx');
  });

  it('turns red while transmitting', () => {
    expect(show(true).container.querySelector('.period')).toHaveClass('period--tx');
  });
});
