import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { useState } from 'react';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/contestTesto', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/contestTesto')>()),
  getTextContestCheck: vi.fn(() => Promise.resolve({ doppio: false, ammesso: true, motivo: null, punti: 1, nuovi: [] as string[] })),
}));
vi.mock('../../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/client')>()),
  getDxcc: vi.fn(() => Promise.resolve(null)),
  getLog4om: vi.fn(() => Promise.resolve({ total: 0, qsos: [] })),
  getLogQsos: vi.fn(() => Promise.resolve({ qsos: [] })),
}));
import type { TextContest, TextContestScore } from '../../api/contestTesto';
import { contestLayoutKey, defaultContestLayout, type ContestView } from '../../lib/contestLayout';
import { ContestWorkspace } from './ContestWorkspace';
import type { Qso } from './DigiLog';

const contest = { id: 'arrl10', name: 'ARRL 10 meter Contest', start: '', end: '' } as TextContest;
const score: TextContestScore = {
  qso: 3,
  doppi: 0,
  punti: 6,
  molt: 2,
  punteggio: 12,
  qtc: 0,
  tipi: ['dom', 'dxcc'],
  bande: [{ band: '10m', qso: 3, doppi: 0, punti: 6, molt: 2, cw: 1, phone: 2, digital: 0 }],
  esclusi: 0,
  griglia: [
    { tipo: 'dom', valore: 'CA', bande: ['10m'] },
    { tipo: 'dom', valore: 'NY', bande: ['10m'] },
    { tipo: 'dxcc', valore: 'DL', bande: [] },
  ],
  mancanti: [{ tipo: 'dom', per_banda: false, totali: 4, valori: ['TX', 'AK'] }],
  recenti: [{ call: 'W6ABC', band: '10m', time_ms: 0, nuovi: ['dom:CA'] }],
  rate10: 12,
  rate60: 30,
  media_ora: 18.5,
};

beforeAll(() => {
  // jsdom non misura nulla: la griglia ha bisogno di una larghezza per disporre le schede
  Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => 1200 });
});
beforeEach(() => {
  window.localStorage.clear();
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});

function Harness({ id = 'arrl10', view = 'ssb', withScore = true, text = false }: { id?: string; view?: ContestView; withScore?: boolean; text?: boolean }) {
  const [qso, setQso] = useState<Qso>({ call: '', name: '', qth: '', wwloc: '', notes: '' });
  const [client] = useState(() => new QueryClient());
  return (
    <QueryClientProvider client={client}>
      <ContestWorkspace
        view={view}
        contest={{ ...contest, id }}
        score={withScore ? score : undefined}
        qso={qso}
        rsts="59"
        rstr="59"
        nrsAuto="001"
        qsoNr={4}
        inSet={() => true}
        noBand={false}
        noMode={false}
        windowNotice={false}
        onEditWindow={() => undefined}
        onQso={(p) => setQso((q) => ({ ...q, ...p }))}
        onRst={() => undefined}
        onRegister={() => undefined}
        band="10m"
        modeName="SSB"
        showLog
        log={<p>LOG-CONTENT</p>}
        cluster={<p>CLUSTER-CONTENT</p>}
        rx={text ? <p>RX-CONTENT</p> : undefined}
        tx={text ? <p>TX-CONTENT</p> : undefined}
        waterfall={text ? () => <p>WF-CONTENT</p> : undefined}
        rotor={() => null}
      />
    </QueryClientProvider>
  );
}

const stored = (id = 'arrl10', view: ContestView = 'ssb') => {
  const raw = window.localStorage.getItem(contestLayoutKey(id, view));
  return raw ? (JSON.parse(raw) as { v: number; locked: boolean; items: { id: string; x: number; y: number; w: number; h: number }[] }) : null;
};

describe('vista del contest a schede', () => {
  it('mostra le schede del contest: inserimento, log, cluster, ritmo, statistiche e una per tipo di moltiplicatore', () => {
    render(<Harness />);
    expect(screen.getByLabelText(/Nominativo|Callsign/i)).toBeTruthy();
    expect(screen.getByText('LOG-CONTENT')).toBeTruthy();
    expect(screen.getByText('CLUSTER-CONTENT')).toBeTruthy();
    expect(screen.getByRole('heading', { name: /Punteggio e ritmo|Score and rate/ })).toBeTruthy();
    expect(screen.getByRole('heading', { name: /Statistiche|Statistics/ })).toBeTruthy();
    // due schede dei moltiplicatori, una per tipo
    expect(screen.getByRole('list', { name: /stati, province|states, provinces/ })).toBeTruthy();
    expect(screen.getByRole('list', { name: /Paesi DXCC|DXCC countries/ })).toBeTruthy();
    expect(screen.queryByText('RX-CONTENT')).toBeNull();
  });

  it('la scheda dei moltiplicatori e una griglia di caselle: lavorate con le bande, da lavorare spente, conteggio nel titolo', () => {
    render(<Harness />);
    const grid = screen.getByRole('list', { name: /stati, province|states, provinces/ });
    const cells = within(grid).getAllByRole('listitem');
    expect(cells.map((c) => c.textContent)).toEqual(['AK', 'CA', 'NY', 'TX']);
    const worked = cells.filter((c) => c.className.includes(' on'));
    expect(worked.map((c) => c.textContent)).toEqual(['CA', 'NY']);
    // CA e il nuovo dell ultimo QSO, i punti delle bande ci sono solo sui lavorati
    expect(cells[1]?.className).toContain('new');
    expect(cells[1]?.querySelectorAll('.ctw-dots u')).toHaveLength(1);
    expect(cells[0]?.querySelectorAll('.ctw-dots u')).toHaveLength(0);
    expect(screen.getByRole('heading', { name: /2 su 4|2 of 4/ })).toBeTruthy();
  });

  it('la tabella delle statistiche ha banda, QSO, doppi, punti, media, moltiplicatori e la riga del totale', () => {
    render(<Harness />);
    const table = screen.getAllByRole('table').find((t) => t.className.includes('ctw-table') && !t.className.includes('ctw-ratetable'));
    expect(table).toBeTruthy();
    const head = within(table as HTMLElement).getAllByRole('columnheader').map((h) => h.textContent);
    expect(head).toEqual(expect.arrayContaining(['Banda', 'QSO', 'Doppi', 'Punti', 'Media', 'Molt.']));
    const rows = within(table as HTMLElement).getAllByRole('row');
    const total = rows[rows.length - 1] as HTMLElement;
    expect(within(total).getByText(/Totale|Total/)).toBeTruthy();
    // media punti per QSO: 6 / 3
    expect(within(total).getByText('2.0')).toBeTruthy();
  });

  it('blocca e sblocca la disposizione (e la ricorda), ripristina la disposizione di partenza', () => {
    render(<Harness />);
    const lock = screen.getByRole('button', { name: /Blocca la disposizione|Lock the layout/ });
    expect(lock.getAttribute('aria-pressed')).toBe('false');
    fireEvent.click(lock);
    expect(screen.getByRole('button', { name: /Sblocca la disposizione|Unlock the layout/ }).getAttribute('aria-pressed')).toBe('true');
    expect(stored()?.locked).toBe(true);
    // bloccata: niente maniglie di ridimensionamento
    expect(document.querySelector('.dash-resize')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /Sblocca la disposizione|Unlock the layout/ }));
    expect(stored()?.locked).toBe(false);
    expect(document.querySelector('.dash-resize')).not.toBeNull();
    // una disposizione cambiata a mano si rimette com era
    window.localStorage.setItem(contestLayoutKey('arrl10', 'ssb'), JSON.stringify({ v: 1, locked: false, items: [{ id: 'entry', x: 6, y: 0, w: 6, h: 12 }] }));
    fireEvent.click(screen.getByRole('button', { name: /Ripristina disposizione|Restore layout/ }));
    const saved = stored();
    const entry = saved?.items.find((i) => i.id === 'entry');
    const def = defaultContestLayout('ssb', []).find((i) => i.id === 'entry');
    expect(entry).toMatchObject({ x: def?.x, y: def?.y, w: def?.w, h: def?.h });
  });

  it('ogni contest ha la sua disposizione salvata', () => {
    window.localStorage.setItem(contestLayoutKey('arrl10', 'ssb'), JSON.stringify({ v: 1, locked: true, items: [] }));
    const { unmount } = render(<Harness id="arrl10" />);
    expect(screen.getByRole('button', { name: /Sblocca la disposizione|Unlock the layout/ })).toBeTruthy();
    unmount();
    render(<Harness id="cqww" />);
    expect(screen.getByRole('button', { name: /Blocca la disposizione|Lock the layout/ })).toBeTruthy();
  });

  it('senza punteggio ci sono le schede che non lo richiedono e nessuna dei moltiplicatori', () => {
    render(<Harness withScore={false} />);
    expect(screen.getByLabelText(/Nominativo|Callsign/i)).toBeTruthy();
    expect(screen.queryByRole('list', { name: /stati, province|states, provinces/ })).toBeNull();
    expect(screen.queryByRole('heading', { name: /Statistiche|Statistics/ })).toBeNull();
  });

  it('nei modi testuali ci sono anche ricezione, trasmissione e cascata', () => {
    render(<Harness view="text" text />);
    expect(screen.getByText('RX-CONTENT')).toBeTruthy();
    expect(screen.getByText('TX-CONTENT')).toBeTruthy();
    expect(screen.getByText('WF-CONTENT')).toBeTruthy();
    expect(screen.getByLabelText(/Nominativo|Callsign/i)).toBeTruthy();
  });
});
