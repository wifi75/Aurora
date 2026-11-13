import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query';
import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { HOLD_EVERY_MS, inOrder, useListenWhileOpen, type PageCommand, type RxCommand } from './listenWhileOpen';

interface Snap {
  rx: { listening: boolean };
  settings: { mode: string; auto_receive: boolean };
}

/** Come la pagina dei modi testuali: il corpo appare con i dati del servizio (anche quelli in memoria della visita di prima). */
function Page({ rx, fresh }: { rx: RxCommand; fresh: () => Promise<Snap> }) {
  const snap = useQuery({ queryKey: ['digi'], queryFn: fresh, retry: false });
  return snap.data ? <Body snap={snap.data} rx={rx} /> : null;
}

function Body({ snap, rx }: { snap: Snap; rx: RxCommand }) {
  useListenWhileOpen(rx, () => snap.settings.auto_receive, () => undefined);
  return <p>{snap.settings.mode}</p>;
}

const flush = () => new Promise((r) => setTimeout(r, 0));

describe('useListenWhileOpen', () => {
  it('issue #45: dall FT8 alla pagina con il CW scelto l ascolto riparte anche se in memoria risulta ancora acceso', async () => {
    const calls: string[] = [];
    const rx: RxCommand = vi.fn((a) => {
      calls.push(a);
      return Promise.resolve();
    });
    const client = new QueryClient();
    // la prima visita: il servizio ascoltava e la pagina lo teneva in memoria
    client.setQueryData<Snap>(['digi'], { rx: { listening: true }, settings: { mode: 'cw', auto_receive: false } });
    const fresh = () => Promise.resolve<Snap>({ rx: { listening: false }, settings: { mode: 'cw', auto_receive: false } });
    const first = render(
      <QueryClientProvider client={client}>
        <Page rx={rx} fresh={fresh} />
      </QueryClientProvider>,
    );
    await flush();
    // si torna all'FT8: l'ascolto si spegne, ma in memoria puo' restare «acceso»
    first.unmount();
    client.setQueryData<Snap>(['digi'], { rx: { listening: true }, settings: { mode: 'cw', auto_receive: false } });
    expect(calls).toEqual(['start', 'stop']);
    // si rientra dall'FT8: il decodificatore CW deve ripartire subito, senza passare da Satelliti o VHF
    const again = render(
      <QueryClientProvider client={client}>
        <Page rx={rx} fresh={fresh} />
      </QueryClientProvider>,
    );
    await flush();
    expect(calls).toEqual(['start', 'stop', 'start']);
    again.unmount();
  });

  it('con il conteggio delle pagine: battito all apertura e ogni 20 s, uscita allo smontaggio (lo spegnimento lo decide il core)', async () => {
    vi.useFakeTimers();
    try {
      const rx = vi.fn<RxCommand>(() => Promise.resolve());
      const page = vi.fn<PageCommand>(() => Promise.resolve());
      function Counted() {
        useListenWhileOpen(rx, () => false, () => undefined, page);
        return null;
      }
      const view = render(<Counted />);
      expect(page.mock.calls.map((c) => c[0])).toEqual(['hold']);
      vi.advanceTimersByTime(HOLD_EVERY_MS * 2);
      expect(page.mock.calls.map((c) => c[0])).toEqual(['hold', 'hold', 'hold']);
      view.unmount();
      expect(page.mock.calls.map((c) => c[0])).toEqual(['hold', 'hold', 'hold', 'release']);
      // stesso identificativo per tutta la visita
      expect(new Set(page.mock.calls.map((c) => c[1])).size).toBe(1);
      // niente «stop» dalla pagina: con un'altra finestra ancora aperta l'ascolto deve restare
      expect(rx.mock.calls.map((c) => c[0])).toEqual(['start']);
      vi.advanceTimersByTime(HOLD_EVERY_MS * 3);
      expect(page).toHaveBeenCalledTimes(4);
    } finally {
      vi.useRealTimers();
    }
  });

  it('con «Ascolto acceso all avvio» uscendo l ascolto resta acceso', async () => {
    const rx = vi.fn<RxCommand>(() => Promise.resolve());
    const client = new QueryClient();
    client.setQueryData<Snap>(['digi'], { rx: { listening: true }, settings: { mode: 'rtty', auto_receive: true } });
    const view = render(
      <QueryClientProvider client={client}>
        <Page rx={rx} fresh={() => new Promise<Snap>(() => undefined)} />
      </QueryClientProvider>,
    );
    await flush();
    view.unmount();
    expect(rx.mock.calls.map((c) => c[0])).toEqual(['start']);
  });
});

describe('inOrder', () => {
  it('lo spegnimento e la riaccensione arrivano al core nell ordine in cui sono partiti', async () => {
    const queue = inOrder();
    const seen: string[] = [];
    let releaseStop: () => void = () => undefined;
    const stop = queue(
      () =>
        new Promise<void>((r) => {
          releaseStop = () => {
            seen.push('stop');
            r();
          };
        }),
    );
    const start = queue(() => {
      seen.push('start');
      return Promise.resolve();
    });
    await flush();
    // finche' lo spegnimento non ha risposto, l'accensione aspetta
    expect(seen).toEqual([]);
    releaseStop();
    await Promise.all([stop, start]);
    expect(seen).toEqual(['stop', 'start']);
  });

  it('un comando fallito non blocca i successivi', async () => {
    const queue = inOrder();
    await expect(queue(() => Promise.reject(new Error('rete')))).rejects.toThrow('rete');
    await expect(queue(() => Promise.resolve(7))).resolves.toBe(7);
  });
});
