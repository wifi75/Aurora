import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LogQso, Valore } from '../api/contest';
import type { DecodedMessage } from '../api/schemas';

vi.mock('../api/contest', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/contest')>()),
  getContestLog: vi.fn(),
  modificaQso: vi.fn(async () => ({ ok: true })),
}));
import { getContestLog } from '../api/contest';
import { useT, type T } from '../i18n';
import { LogContest, TabellaDecodifiche } from './ContestPage';

const heads = () => screen.getAllByRole('columnheader').map((th) => th.textContent);
const dec = (id: number, text: string, country: string) => ({ id, time_ms: Date.UTC(2026, 0, 1, 12, 0, 15), snr: -5, freq_hz: 1200, text, country }) as unknown as DecodedMessage;
const val = (call: string, v: Partial<Valore> = {}): Valore => ({ call, doppio: false, punti: 2, km: 0, molt_nuovi: [], bande_utili: [], fuori_regola: false, ...v });

type Riga = { d: DecodedMessage; call: string; grid: string; v?: Valore };

function Decodifiche({ righe, rispondi }: { righe: Riga[]; rispondi: (x: Riga) => void }) {
  const t: T = useT();
  return (
    <div className="ctc">
      <TabellaDecodifiche righe={righe} dx="K1ABC" stella="JA1XYZ" nonContest={new Set()} rispondi={rispondi} t={t} />
    </div>
  );
}

function Log() {
  const t: T = useT();
  return (
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <div className="ctc">
        <LogContest t={t} qso={1} />
      </div>
    </QueryClientProvider>
  );
}

const righe: Riga[] = [
  { d: dec(1, 'CQ TEST JA1XYZ PM95', 'Japan'), call: 'JA1XYZ', grid: 'PM95', v: val('JA1XYZ', { molt_nuovi: ['PM95'] }) },
  { d: dec(2, 'IU3ABC K1ABC FN42', 'United States'), call: 'K1ABC', grid: 'FN42', v: val('K1ABC') },
  { d: dec(3, 'CQ TEST DL1AA JO62', 'Germany'), call: 'DL1AA', grid: 'JO62', v: val('DL1AA', { doppio: true }) },
];

const qso = (n: number, call: string, extra: Partial<LogQso> = {}): LogQso => ({
  n, time_ms: Date.UTC(2026, 0, 1, 12, 3), call, band: '20m', mode: 'FT4', freq_hz: 14_080_000, grid: 'FN42', rst_sent: '', rst_rcvd: '', exch_sent: '', exch_rcvd: 'FN42',
  stato: 'valido', modificato: false, doppio: false, punti: 1, km: 0, molt_nuovi: [], scambio_ok: true, ...extra,
});

beforeEach(() => localStorage.clear());

describe('Decodifiche del contest', () => {
  it('draws a grid with movable headers and keeps the highlights and the double click', () => {
    const rispondi = vi.fn();
    render(<Decodifiche righe={righe} rispondi={rispondi} />);
    expect(screen.getByRole('table')).toHaveClass('mv-grid');
    expect(heads()).toEqual(['UTC', 'dB', 'Hz', 'Messaggio', 'Paese', 'km', 'Novità', 'Valore']);
    expect(screen.getByRole('columnheader', { name: 'Messaggio' }).querySelector('.cl-grip')).not.toBeNull();
    const top = screen.getByText('Japan').closest('tr');
    expect(top).toHaveClass('top');
    expect(screen.getByText('Germany').closest('tr')).toHaveClass('dup');
    expect(screen.getByText('United States').closest('tr')).toHaveClass('qso');
    expect(screen.getByText('★ 2 pt', { exact: false })).toBeInTheDocument();
    if (!top) throw new Error('manca la riga');
    fireEvent.doubleClick(top);
    expect(rispondi).toHaveBeenCalledWith(expect.objectContaining({ call: 'JA1XYZ' }));
  });

  it('moves a column with Alt+arrows, the cells follow, and the order is remembered under its own key', () => {
    const first = render(<Decodifiche righe={righe} rispondi={vi.fn()} />);
    fireEvent.keyDown(screen.getByRole('columnheader', { name: 'Valore' }), { key: 'ArrowLeft', altKey: true });
    expect(heads()).toEqual(['UTC', 'dB', 'Hz', 'Messaggio', 'Paese', 'km', 'Valore', 'Novità']);
    const row = screen.getByText('Japan').closest('tr');
    expect(row?.children[6]).toHaveClass('ctc-c--value');
    expect(row?.children[7]).toHaveClass('ctc-c--new');
    expect(JSON.parse(localStorage.getItem('aurora.contest.dec.cols') ?? 'null')?.order).toEqual(['utc', 'db', 'hz', 'msg', 'country', 'km', 'value', 'new']);
    expect(localStorage.getItem('aurora.cluster.cols')).toBeNull();
    first.unmount();
    render(<Decodifiche righe={righe} rispondi={vi.fn()} />);
    expect(heads()[6]).toBe('Valore');
  });

  it('the header menu resets the columns', () => {
    localStorage.setItem('aurora.contest.dec.cols', JSON.stringify({ order: ['value', 'utc', 'db', 'hz', 'msg', 'country', 'km', 'new'], widths: { msg: 30 } }));
    render(<Decodifiche righe={righe} rispondi={vi.fn()} />);
    expect(heads()[0]).toBe('Valore');
    fireEvent.contextMenu(screen.getByRole('columnheader', { name: 'UTC' }), { clientX: 10, clientY: 10 });
    fireEvent.click(screen.getByRole('menuitem', { name: 'Ripristina colonne' }));
    expect(heads()).toEqual(['UTC', 'dB', 'Hz', 'Messaggio', 'Paese', 'km', 'Novità', 'Valore']);
    expect(JSON.parse(localStorage.getItem('aurora.contest.dec.cols') ?? 'null')).toEqual({ order: ['utc', 'db', 'hz', 'msg', 'country', 'km', 'new', 'value'], widths: {} });
  });

  it('the empty message spans every column', () => {
    render(<Decodifiche righe={[]} rispondi={vi.fn()} />);
    expect(screen.getByText(/./, { selector: 'td.ctc-muted' })).toHaveAttribute('colspan', '8');
  });
});

describe('Log del contest', () => {
  it('draws a grid with movable headers; the right-click on a row still opens the row menu', async () => {
    vi.mocked(getContestLog).mockResolvedValue({ qso: [qso(1, 'K1ABC'), qso(2, 'JA1XYZ', { stato: 'incerto' })], totali: null });
    render(<Log />);
    await screen.findByText('K1ABC');
    expect(screen.getByRole('table')).toHaveClass('mv-grid');
    expect(heads()).toEqual(['UTC', 'Banda', 'Nominativo', 'Ric.', 'Punti', 'Stato']);
    expect(screen.getByText('JA1XYZ').closest('tr')).toHaveClass('warn');
    fireEvent.contextMenu(screen.getByText('K1ABC'), { clientX: 30, clientY: 30 });
    expect(screen.getByRole('menuitem', { name: 'Non in contest (0 punti)' })).toBeInTheDocument();
    expect(screen.queryByRole('menuitem', { name: 'Ripristina colonne' })).toBeNull();
  });

  it('dragging the right edge of a header resizes the column under its own key', async () => {
    vi.mocked(getContestLog).mockResolvedValue({ qso: [qso(1, 'K1ABC')], totali: null });
    render(<Log />);
    await screen.findByText('K1ABC');
    const grip = screen.getByRole('columnheader', { name: 'Nominativo' }).querySelector('.cl-grip');
    if (!grip) throw new Error('manca la maniglia');
    fireEvent.pointerDown(grip, { button: 0, clientX: 100 });
    act(() => {
      window.dispatchEvent(new MouseEvent('pointermove', { clientX: 116, button: 0 }));
      window.dispatchEvent(new MouseEvent('pointerup', { clientX: 116, button: 0 }));
    });
    const saved = JSON.parse(localStorage.getItem('aurora.contest.log.cols') ?? 'null') as { widths: Record<string, number> } | null;
    expect(saved?.widths.call ?? 0).toBeCloseTo(7.5, 5);
    fireEvent.contextMenu(screen.getByRole('columnheader', { name: 'Punti' }), { clientX: 10, clientY: 10 });
    fireEvent.click(screen.getByRole('menuitem', { name: 'Ripristina colonne' }));
    expect(JSON.parse(localStorage.getItem('aurora.contest.log.cols') ?? 'null')).toEqual({ order: ['utc', 'band', 'call', 'rcvd', 'points', 'state'], widths: {} });
  });
});
