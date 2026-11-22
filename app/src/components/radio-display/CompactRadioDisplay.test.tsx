import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../../api/client', async () => {
  const real = await vi.importActual<typeof import('../../api/client')>('../../api/client');
  return { ...real, getCat: vi.fn(() => new Promise(() => undefined)), postCatExtra: vi.fn() };
});
import { DEMO_MODEL } from '../../lib/radioDisplay';
import { CompactRadioDisplay } from './CompactRadioDisplay';

function show(model: typeof DEMO_MODEL) {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <CompactRadioDisplay model={model} noTune />
    </QueryClientProvider>,
  );
}

describe('CompactRadioDisplay', () => {
  it('shows the ALC reading next to power and SWR when the radio sends it', () => {
    show({ ...DEMO_MODEL, meters: { ...DEMO_MODEL.meters, power_w: 40, swr: 1.4, alc: 0.35 } });
    const alc = screen.getByTitle('ALC');
    expect(alc.textContent).toContain('ALC');
    expect(alc.textContent).toContain('35 %');
    expect(screen.getByTitle('SWR').textContent).toContain('1.4');
  });

  it('shows a dash for the ALC when the radio does not send it', () => {
    show({ ...DEMO_MODEL, meters: { s_points: 7, s_over_db: 0 } });
    expect(screen.getByTitle('ALC').textContent).toContain('—');
  });
});
