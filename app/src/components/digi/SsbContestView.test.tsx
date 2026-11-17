import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/contestTesto', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/contestTesto')>()),
  getTextContestCheck: vi.fn(),
}));
vi.mock('../../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/client')>()),
  getDxcc: vi.fn(() => Promise.resolve(null)),
  getLog4om: vi.fn(() => Promise.resolve({ total: 0, qsos: [] })),
  getLogQsos: vi.fn(() => Promise.resolve({ qsos: [] })),
}));
import { getTextContestCheck } from '../../api/contestTesto';
import { composeReceived } from '../../lib/contestSection';
import type { Qso } from './DigiLog';
import { SsbContestView } from './SsbContestView';

const UBA_LABEL = /Club \((sezione UBA|UBA section)\)/;
const ARRL_LABEL = /Stato \/ provincia|State \/ province/;
const GENERIC_LABEL = /Sezione \/ stato \/ club|Section \/ state \/ club/;
const base = { doppio: false, ammesso: true, motivo: null, punti: 1, nuovi: [] as string[] };
let registered: string[] = [];

function Harness({ contestName = 'UBA CW CONTEST' }: { contestName?: string }) {
  const [qso, setQso] = useState<Qso>({ call: '', name: '', qth: '', wwloc: '', notes: '' });
  const [client] = useState(() => new QueryClient());
  return (
    <QueryClientProvider client={client}>
      <SsbContestView
        qso={qso}
        rsts="599"
        rstr="599"
        nrsAuto="001"
        qsoNr={1}
        contest={null}
        inSet={() => true}
        noBand={false}
        noMode={false}
        windowNotice={false}
        onEditWindow={() => undefined}
        onQso={(p) => setQso((q) => ({ ...q, ...p }))}
        onRst={() => undefined}
        onRegister={() => {
          registered.push(`${qso.call}|${composeReceived(qso.nrr, qso.extra)}`);
          setQso({ call: '', name: '', qth: '', wwloc: '', notes: '' });
        }}
        band="10m"
        modeName="CW"
        contestName={contestName}
        score={undefined}
        showLog={false}
        log={null}
        cluster={null}
        rotor={() => null}
      />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  registered = [];
  vi.mocked(getTextContestCheck).mockReset();
  localStorage.clear();
});

const write = (el: HTMLElement, v: string) => fireEvent.change(el, { target: { value: v } });
// il nominativo si controlla 220 ms dopo l'ultimo tasto, poi arriva la risposta del core
const settle = () => waitFor(() => new Promise<void>((r) => setTimeout(r, 600)));

describe('riga di inserimento: terzo elemento dello scambio', () => {
  it('la casella compare solo se il core dice che il nominativo la da\'', async () => {
    vi.mocked(getTextContestCheck).mockImplementation((call) => Promise.resolve({ ...base, exch_atteso: ['OP4K', 'ON4ABC'].includes(call) }));
    render(<Harness />);
    const call = screen.getByLabelText(/Nominativo|Callsign/i);
    expect(screen.queryByLabelText(UBA_LABEL)).toBeNull();
    write(call, 'op4k');
    await settle();
    expect(screen.getByLabelText(UBA_LABEL)).toBeTruthy();
    write(call, 'SV1BJW');
    await settle();
    expect(screen.queryByLabelText(UBA_LABEL)).toBeNull();
  });

  it('con un core senza exch_atteso la casella non compare', async () => {
    vi.mocked(getTextContestCheck).mockImplementation(() => Promise.resolve({ ...base }));
    render(<Harness />);
    write(screen.getByLabelText(/Nominativo|Callsign/i), 'ON4ABC');
    await settle();
    expect(screen.queryByLabelText(UBA_LABEL)).toBeNull();
  });

  it('il nome segue il contest: ARRL stato / provincia, generico altrimenti', async () => {
    vi.mocked(getTextContestCheck).mockImplementation(() => Promise.resolve({ ...base, exch_atteso: true }));
    const { unmount } = render(<Harness contestName="ARRL 10 meter Contest" />);
    write(screen.getByLabelText(/Nominativo|Callsign/i), 'W1AW');
    await settle();
    expect(screen.getByLabelText(ARRL_LABEL)).toBeTruthy();
    unmount();
    render(<Harness contestName="Altro Contest" />);
    write(screen.getByLabelText(/Nominativo|Callsign/i), 'W1AW');
    await settle();
    expect(screen.getByLabelText(GENERIC_LABEL)).toBeTruthy();
  });

  it('Tab/Invio: nominativo, numero, casella extra, poi Invio registra con «numero sezione»', async () => {
    vi.mocked(getTextContestCheck).mockImplementation(() => Promise.resolve({ ...base, exch_atteso: true }));
    render(<Harness />);
    const call = screen.getByLabelText(/Nominativo|Callsign/i);
    write(call, 'OP4K');
    await settle();
    const nrr = screen.getByLabelText(/Seriale ric|Serial rcvd/i);
    const extra = screen.getByLabelText(UBA_LABEL);
    // il numero inviato non si tocca con Invio (data-skip): da nominativo si passa al numero ricevuto, poi alla casella extra
    call.focus();
    fireEvent.keyDown(call, { key: 'Enter' });
    expect(document.activeElement).toBe(nrr);
    write(nrr, '003');
    fireEvent.keyDown(nrr, { key: 'Enter' });
    expect(document.activeElement).toBe(extra);
    write(extra, 'acc');
    expect((extra as HTMLInputElement).value).toBe('ACC');
    expect(registered).toEqual([]);
    fireEvent.keyDown(extra, { key: 'Enter' });
    expect(registered).toEqual(['OP4K|003 ACC']);
  });

  it('senza la casella Invio dal numero ricevuto registra subito', async () => {
    vi.mocked(getTextContestCheck).mockImplementation(() => Promise.resolve({ ...base, exch_atteso: false }));
    render(<Harness />);
    const call = screen.getByLabelText(/Nominativo|Callsign/i);
    write(call, 'SV1BJW');
    await settle();
    const nrr = screen.getByLabelText(/Seriale ric|Serial rcvd/i);
    write(nrr, '012');
    fireEvent.keyDown(nrr, { key: 'Enter' });
    expect(registered).toEqual(['SV1BJW|012']);
  });
});
