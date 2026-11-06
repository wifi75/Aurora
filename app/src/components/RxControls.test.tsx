import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getControls: vi.fn().mockResolvedValue({}), postControl: vi.fn() }));
import { RxControls } from './RxControls';

function show(transmitting: boolean) {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <RxControls mode="FT8" transmitting={transmitting} />
    </QueryClientProvider>,
  );
}

describe('RxControls period', () => {
  it('shows only the UTC clock: the period progress is in the bottom status bar', () => {
    const { container } = show(false);
    expect(container.querySelector('[role="progressbar"]')).toBeNull();
    expect(container.querySelector('.period--clock')).not.toBeNull();
  });

  it('keeps the clock while transmitting', () => {
    expect(show(true).container.querySelector('.period--clock')).not.toBeNull();
  });
});
