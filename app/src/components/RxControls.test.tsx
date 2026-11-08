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

describe('RxControls', () => {
  it('puts the switches in one segmented bar, Monitora first, without a period bar (the clock is in the bottom status bar)', () => {
    const { container } = show(false);
    expect(container.querySelector('[role="progressbar"]')).toBeNull();
    const items = [...container.querySelectorAll('.rx-seg:not(.rx-seg--btns) .rx-seg-item')].map((b) => b.textContent);
    expect(items[0]).toBe('Monitora');
    expect(items).toContain('SWL');
  });

  it('keeps Ferma, Decodifica and Cancella together in their own bar', () => {
    const items = [...show(true).container.querySelectorAll('.rx-seg--btns .rx-seg-item')].map((b) => b.textContent);
    expect(items).toHaveLength(3);
  });
});
