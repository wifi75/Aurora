import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { usePrefs } from '../lib/prefs';
import { useNotifyView } from '../lib/notifyView';
import { useStation } from '../state/station';
import { DecodeList } from './DecodeList';

// il paese chiesto al core per i nominativi senza paese (le Fox nelle righe DXpedition): qui risponde solo per 3X4U
vi.mock('../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/client')>()),
  getDxcc: vi.fn((call: string) => (call === '3X4U' ? Promise.resolve({ country: 'Guinea' }) : Promise.reject(new Error('offline')))),
}));

const decode = (over: Partial<DecodedMessage> = {}): DecodedMessage => ({
  id: 7, time_ms: 45_015_000, snr: -12, delta_time: 0.3, freq_hz: 1500,
  text: 'CQ DX JA1XYZ PM95', tag: null, ...over,
});

describe('DecodeList', () => {
  it('shows NUOVO DXCC badge', () => {
    render(<DecodeList decodes={[decode({ tag: 'new_dxcc' })]} onReply={() => {}} />);
    expect(screen.getByText('NUOVO DXCC')).toBeInTheDocument();
  });

  it('shows AUTO and NUOVA ZONA badges', () => {
    render(<DecodeList decodes={[decode({ id: 1, tag: 'auto' }), decode({ id: 2, tag: 'new_zone' })]} onReply={() => {}} />);
    expect(screen.getByText('AUTO')).toBeInTheDocument();
    expect(screen.getByText('NUOVA ZONA')).toBeInTheDocument();
  });

  it('shows the new band and new grid badges', () => {
    render(<DecodeList decodes={[decode({ id: 1, tag: 'new_dxcc_band' }), decode({ id: 2, tag: 'new_grid' })]} onReply={() => {}} />);
    expect(screen.getByText('DXCC BANDA')).toBeInTheDocument();
    expect(screen.getByText('GRIGLIA')).toBeInTheDocument();
  });

  it('shows a grid new only because unconfirmed lighter, with the reason in the tooltip; a really new one stays full (issue #40)', () => {
    render(<DecodeList decodes={[decode({ id: 1, tag: 'new_grid', unconfirmed: true }), decode({ id: 2, tag: 'new_grid', text: 'CQ JA2XYZ PM96' })]} onReply={() => {}} />);
    const [light, full] = screen.getAllByText('GRIGLIA');
    expect(light?.closest('.decode-row')).toHaveClass('decode-row--unconfirmed');
    expect(light).toHaveAttribute('title', expect.stringContaining('non confermato'));
    expect(full?.closest('.decode-row')).not.toHaveClass('decode-row--unconfirmed');
  });

  it('shows the country of the sender when known', () => {
    render(<DecodeList decodes={[decode({ country: 'Giappone' })]} onReply={() => {}} />);
    expect(screen.getByText('Giappone')).toBeInTheDocument();
  });

  it('shows the distance in km like the old one when the own locator is known', () => {
    render(<DecodeList decodes={[decode({ country: 'Giappone' })]} myGrid="JN45" onReply={() => {}} />);
    expect(screen.getByText(/^\d+ km$/)).toBeInTheDocument();
  });

  it('puts the station being called in evidence in the messages that name it', () => {
    render(
      <DecodeList
        dxCall="BB2BB"
        decodes={[decode({ id: 1, text: 'AA1AA BB2BB -10' }), decode({ id: 2, text: 'CQ CC3CC FN42' }), decode({ id: 3, text: 'BB2BB DD4DD RR73' })]}
        onReply={() => {}}
      />,
    );
    const marks = document.querySelectorAll('mark.dx-call');
    expect(marks).toHaveLength(2);
    expect(marks[0]).toHaveTextContent('BB2BB');
    const rows = screen.getAllByRole('button');
    expect(rows[0]).toHaveClass('decode-row--dx');
    expect(rows[1]).not.toHaveClass('decode-row--dx');
    expect(rows[2]).toHaveClass('decode-row--dx');
  });

  it('matches the station case-insensitively and only as a whole call', () => {
    render(<DecodeList dxCall="bb2bb" decodes={[decode({ id: 1, text: 'AA1AA BB2BBX -10' }), decode({ id: 2, text: 'AA1AA BB2BB -10' })]} onReply={() => {}} />);
    expect(document.querySelectorAll('mark.dx-call')).toHaveLength(1);
  });

  it('drops the red mark once the QSO is closed (RR73 or 73 addressed to me)', () => {
    render(
      <DecodeList
        myCall="AA1AA"
        decodes={[decode({ id: 1, text: 'AA1AA BB2BB RR73' }), decode({ id: 2, text: 'AA1AA CC3CC 73' }), decode({ id: 3, text: 'AA1AA DD4DD -10' })]}
        onReply={() => {}}
      />,
    );
    const rows = screen.getAllByRole('button');
    expect(rows[0]?.querySelector('.me-mark')).toBeNull();
    expect(rows[1]?.querySelector('.me-mark')).toBeNull();
    expect(rows[2]?.querySelector('.me-mark')).not.toBeNull();
  });

  it('highlights nothing without a station being called', () => {
    render(<DecodeList decodes={[decode({ text: 'AA1AA BB2BB -10' })]} onReply={() => {}} />);
    expect(document.querySelector('mark.dx-call')).toBeNull();
  });

  it('shows a call addressed to me like the old program: strong row and a red mark', () => {
    render(
      <DecodeList
        myCall="AA1AA"
        decodes={[decode({ id: 1, text: 'AA1AA BB2BB -10' }), decode({ id: 2, text: 'BB2BB AA1AA -10' }), decode({ id: 3, text: 'CQ CC3CC FN42' })]}
        onReply={() => {}}
      />,
    );
    const rows = screen.getAllByRole('button');
    expect(rows[0]).toHaveClass('decode-row--me');
    expect(rows[0]?.querySelector('.me-mark')).not.toBeNull();
    expect(rows[0]).toHaveAttribute('title', expect.stringContaining('ti sta chiamando'));
    expect(rows[1]).not.toHaveClass('decode-row--me');
    expect(rows[2]).not.toHaveClass('decode-row--me');
  });

  it('marks nothing without my callsign', () => {
    render(<DecodeList decodes={[decode({ text: 'AA1AA BB2BB -10' })]} onReply={() => {}} />);
    expect(document.querySelector('.decode-row--me')).toBeNull();
  });

  it('shows the delta time column', () => {
    render(<DecodeList decodes={[decode({ delta_time: 0.3 })]} onReply={() => {}} />);
    expect(screen.getByText('0.3')).toBeInTheDocument();
  });

  it('keeps the badge short and puts the full name in its tooltip', () => {
    render(<DecodeList decodes={[decode({ tag: 'new_px_band' })]} onReply={() => {}} />);
    const badge = screen.getByText('PREF. BANDA');
    expect(badge).toHaveAttribute('title', 'Prefisso su nuova banda o modo');
    expect(badge.textContent!.length).toBeLessThanOrEqual(14);
  });

  it('shows the flag of the country next to its name', () => {
    const { container } = render(<DecodeList decodes={[decode({ country: 'Japan' })]} onReply={() => {}} />);
    const flag = container.querySelector('.fi.fi-jp');
    expect(flag).not.toBeNull();
    expect(flag).toHaveAttribute('aria-hidden', 'true');
    expect(screen.getByText('Japan')).toBeInTheDocument();
  });

  it('shows no flag for a country without one', () => {
    const { container } = render(<DecodeList decodes={[decode({ country: 'ITU HQ' })]} onReply={() => {}} />);
    expect(container.querySelector('.fi')).toBeNull();
    expect(screen.getByText('ITU HQ')).toBeInTheDocument();
  });

  it('keeps every field in its own fixed cell, even when empty, so the columns line up', () => {
    const { container } = render(
      <DecodeList decodes={[decode({ id: 1, country: 'Japan', text: 'CQ JA1XYZ PM95' }), decode({ id: 2, country: null, text: 'TNX 73' })]} myGrid="JN45" onReply={() => {}} />,
    );
    const rows = container.querySelectorAll('.decode-row');
    expect(rows).toHaveLength(2);
    for (const row of rows) {
      for (const cell of ['.c-time', '.c-snr', '.c-freq', '.decode-text', '.decode-where .w-flag', '.decode-where .w-country', '.decode-where .w-km']) {
        expect(row.querySelector(cell), cell).not.toBeNull();
      }
      expect(row.children).toHaveLength(7);
    }
  });

  it('shows no country text when unknown', () => {
    render(<DecodeList decodes={[decode({ country: null }), decode({ id: 8 })]} onReply={() => {}} />);
    expect(screen.queryByText('Giappone')).not.toBeInTheDocument();
  });

  it('empty state invites listening', () => {
    render(<DecodeList decodes={[]} onReply={() => {}} />);
    expect(screen.getByText('In ascolto: i messaggi compaiono qui')).toBeInTheDocument();
  });

  it('shows a custom empty message when given', () => {
    render(<DecodeList decodes={[]} onReply={() => {}} emptyText="Nessun CQ per ora" />);
    expect(screen.getByText('Nessun CQ per ora')).toBeInTheDocument();
  });

  it('a double click on a row replies to that station', () => {
    const onReply = vi.fn();
    render(<DecodeList decodes={[decode({ id: 42 })]} onReply={onReply} />);
    fireEvent.doubleClick(screen.getByRole('button', { name: /JA1XYZ/ }));
    expect(onReply).toHaveBeenCalledTimes(1);
    expect(onReply).toHaveBeenCalledWith(42);
  });

  it('a single click only selects the row and never transmits', () => {
    const onReply = vi.fn();
    render(<DecodeList decodes={[decode({ id: 42 }), decode({ id: 43, text: 'CQ BB2BB FN42' })]} onReply={onReply} />);
    const first = screen.getByRole('button', { name: /JA1XYZ/ });
    fireEvent.click(first);
    expect(onReply).not.toHaveBeenCalled();
    expect(first).toHaveClass('decode-row--selected');
    fireEvent.click(screen.getByRole('button', { name: /BB2BB/ }));
    expect(first).not.toHaveClass('decode-row--selected');
    expect(onReply).not.toHaveBeenCalled();
  });

  it('shows time snr and frequency of each row', () => {
    render(<DecodeList decodes={[decode()]} onReply={() => {}} />);
    expect(screen.getByText('12:30:15')).toBeInTheDocument();
    expect(screen.getByText('-12')).toBeInTheDocument();
    expect(screen.getByText('1500')).toBeInTheDocument();
  });

  it('shows our own transmissions as Tx rows that cannot be replied to', () => {
    const onReply = vi.fn();
    render(<DecodeList decodes={[decode({ id: -1, snr: 0, text: 'CQ AA1AA FN42', tag: 'cq' })]} onReply={onReply} myCall="AA1AA" />);
    const row = screen.getByRole('button', { name: /CQ AA1AA FN42/ });
    expect(row).toHaveClass('decode-row--tx');
    expect(row).not.toHaveClass('decode-row--me');
    expect(screen.getByText('Tx')).toBeInTheDocument();
    fireEvent.doubleClick(row);
    expect(onReply).not.toHaveBeenCalled();
  });

  it('with the double-click option off, a double click only selects the row', () => {
    usePrefs.getState().set({ doubleClickReply: false });
    const onReply = vi.fn();
    render(<DecodeList decodes={[decode()]} onReply={onReply} />);
    fireEvent.doubleClick(screen.getByRole('button', { name: /JA1XYZ/ }));
    expect(onReply).not.toHaveBeenCalled();
    usePrefs.getState().reset();
  });

  describe('stations already worked (same band and mode, from the log)', () => {
    const worked = (over: Partial<DecodedMessage> = {}) => decode({ id: 11, text: 'CQ K1ABC FN42', tag: 'worked_call', ...over });
    afterEach(() => {
      useNotifyView.getState().reset();
      usePrefs.getState().reset();
    });

    it('strikes the message by default, without the green colour', () => {
      render(<DecodeList decodes={[worked()]} onReply={() => {}} />);
      const row = screen.getByRole('button', { name: /K1ABC/ });
      expect(row).toHaveClass('decode-row--worked', 'decode-row--strike');
      expect(row).not.toHaveClass('decode-row--underline');
      expect(row).not.toHaveClass('decode-row--worked-color');
      // l'etichetta resta, ma neutra: il verde c'e' solo con «evidenzia gia' lavorati»
      expect(screen.getByText('GIÀ LAVORATA')).toHaveClass('badge--quiet');
      expect(row.style.getPropertyValue('--row-accent')).toBe('');
    });

    it('underlines instead of striking when asked', () => {
      useNotifyView.getState().set({ workedStriked: false, workedUnderlined: true });
      render(<DecodeList decodes={[worked()]} onReply={() => {}} />);
      const row = screen.getByRole('button', { name: /K1ABC/ });
      expect(row).toHaveClass('decode-row--underline');
      expect(row).not.toHaveClass('decode-row--strike');
    });

    it('with the colour option the green label and bar come back', () => {
      useNotifyView.getState().set({ workedColor: true });
      render(<DecodeList decodes={[worked()]} onReply={() => {}} />);
      const row = screen.getByRole('button', { name: /K1ABC/ });
      expect(row).toHaveClass('decode-row--worked-color');
      expect(screen.getByText('GIÀ LAVORATA')).not.toHaveClass('badge--quiet');
      expect(row.style.getPropertyValue('--row-accent')).toBe('var(--tag-worked-call)');
    });

    it('does not show them at all with «non mostrare», except calls to me and own Tx rows', () => {
      useNotifyView.getState().set({ workedDontShow: true });
      render(
        <DecodeList
          myCall="AA1AA"
          decodes={[worked({ id: 1 }), worked({ id: 2, text: 'AA1AA K1ABC -10' }), worked({ id: 3, text: 'CQ DX D2EFG KO10', tag: 'new_call' }), worked({ id: -4, text: 'AA1AA K1ABC RR73', tag: 'worked_call' })]}
          onReply={() => {}}
        />,
      );
      expect(screen.queryByRole('button', { name: /^.*CQ K1ABC FN42/ })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /AA1AA K1ABC -10/ })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /D2EFG/ })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /RR73/ })).toBeInTheDocument();
    });

    it('leaves a message with something new alone: no strike', () => {
      render(<DecodeList decodes={[worked({ tag: 'new_grid' })]} onReply={() => {}} />);
      expect(screen.getByRole('button', { name: /K1ABC/ })).not.toHaveClass('decode-row--strike');
    });
  });

  it('gives the calls to me a cyan or yellow background when asked, and highlights the DX call and grid only when asked', () => {
    const rows = [decode({ id: 1, text: 'AA1AA BB2BB -10', tag: null }), decode({ id: 2, text: 'CQ DD4DD JO31', tag: null })];
    useNotifyView.getState().set({ differentBackground: true });
    const { rerender } = render(<DecodeList decodes={rows} myCall="AA1AA" dxCall="BB2BB" dxGrid="JO31" onReply={() => {}} />);
    expect(screen.getByRole('button', { name: /AA1AA BB2BB/ })).toHaveClass('decode-row--me-cyan');
    useNotifyView.getState().set({ diffYellow: true });
    rerender(<DecodeList decodes={rows} myCall="AA1AA" dxCall="BB2BB" dxGrid="JO31" onReply={() => {}} />);
    expect(screen.getByRole('button', { name: /AA1AA BB2BB/ })).toHaveClass('decode-row--me-yellow');
    // di base si evidenzia il nominativo DX (come Aurora ha sempre fatto) e non il locatore
    expect(document.querySelectorAll('mark.dx-call')).toHaveLength(1);
    expect(document.querySelectorAll('mark.dx-grid')).toHaveLength(0);
    useNotifyView.getState().set({ highlightDxGrid: true });
    rerender(<DecodeList decodes={rows} myCall="AA1AA" dxCall="BB2BB" dxGrid="JO31" onReply={() => {}} />);
    expect(document.querySelectorAll('mark.dx-grid')).toHaveLength(1);
    useNotifyView.getState().set({ highlightDxCall: false });
    rerender(<DecodeList decodes={rows} myCall="AA1AA" dxCall="BB2BB" dxGrid="JO31" onReply={() => {}} />);
    expect(document.querySelectorAll('mark.dx-call:not(.dx-grid):not(.dx-call--answered)')).toHaveLength(0);
    expect(screen.getByRole('button', { name: /AA1AA BB2BB/ })).not.toHaveClass('decode-row--dx');
    useNotifyView.getState().reset();
  });

  it('inverts the colours when asked (text coloured, neutral background)', () => {
    useNotifyView.getState().set({ invert: true });
    render(<DecodeList decodes={[decode({ tag: 'new_dxcc' })]} onReply={() => {}} />);
    expect(screen.getByRole('region')).toHaveClass('decode-list--invert');
    useNotifyView.getState().reset();
  });

  it('marks the calls to me in red, in violet for the station that answers me, and not at all with the markers off', () => {
    // le avevo gia' trasmesso: il suo messaggio successivo e' la risposta
    useStation.setState({ txLog: [decode({ id: -1, time_ms: 45_000_000, text: 'BB2BB AA1AA -10' })] });
    const rows = [decode({ id: 1, text: 'AA1AA BB2BB -10', tag: null }), decode({ id: 2, text: 'AA1AA CC3CC -10', tag: null })];
    const { container, rerender } = render(<DecodeList decodes={rows} myCall="AA1AA" dxCall="BB2BB" onReply={() => {}} />);
    expect(container.querySelectorAll('.me-mark')).toHaveLength(2);
    // BB2BB e' la stazione che sto lavorando e le avevo gia' trasmesso: il suo messaggio e' la risposta (linguetta viola); CC3CC mi chiama per primo (rossa)
    expect(container.querySelectorAll('.me-mark--answer')).toHaveLength(1);
    expect(container.querySelectorAll('.me-mark:not(.me-mark--answer)')).toHaveLength(1);
    useNotifyView.getState().set({ redMarker: false });
    rerender(<DecodeList decodes={rows} myCall="AA1AA" dxCall="BB2BB" onReply={() => {}} />);
    expect(container.querySelectorAll('.me-mark')).toHaveLength(0);
    useNotifyView.getState().reset();
    useStation.setState({ txLog: [] });
  });

  it('keeps the first call from the current DX red until I actually reply to it', () => {
    // BB2BB mi ha appena chiamato per prima: non le ho ancora trasmesso nulla, anche se e' gia' la stazione DX in corso
    const rows = [decode({ id: 1, text: 'AA1AA BB2BB -10', tag: null })];
    const { container } = render(<DecodeList decodes={rows} myCall="AA1AA" dxCall="BB2BB" onReply={() => {}} />);
    expect(container.querySelectorAll('.me-mark--answer')).toHaveLength(0);
    expect(container.querySelectorAll('.me-mark:not(.me-mark--answer)')).toHaveLength(1);
  });

  it('puts a blank line between periods, shows the azimuth and can hide the own Tx rows', () => {
    usePrefs.getState().set({ insertBlank: true, showAzimuth: true, showTx: false });
    render(
      <DecodeList
        myGrid="JN45"
        decodes={[decode({ id: 1, time_ms: 45_030_000 }), decode({ id: 2, time_ms: 45_030_000 }), decode({ id: 3, time_ms: 45_015_000 }), decode({ id: -5, time_ms: 45_015_000, text: 'CQ AA1AA JN45' })]}
        onReply={() => {}}
      />,
    );
    expect(screen.getAllByRole('separator')).toHaveLength(1);
    expect(screen.getAllByText(/^\d+ km \d+°$/).length).toBeGreaterThan(0);
    expect(screen.queryByText('Tx')).not.toBeInTheDocument();
    usePrefs.getState().reset();
  });
  it('turns the callsign violet with a tooltip and drops the red mark when the station answers my call', () => {
    // le avevo gia' trasmesso: il suo messaggio successivo e' la risposta
    useStation.setState({ txLog: [decode({ id: -1, time_ms: 45_000_000, text: 'BB2BB AA1AA -10' })] });
    render(
      <DecodeList
        myCall="AA1AA"
        dxCall="BB2BB"
        decodes={[decode({ id: 1, text: 'AA1AA BB2BB R-10' }), decode({ id: 2, text: 'AA1AA CC3CC -07' })]}
        onReply={() => {}}
      />,
    );
    const rows = screen.getAllByRole('button');
    // BB2BB e' la stazione che sto lavorando e le avevo gia' trasmesso: il suo messaggio e' la risposta (etichetta viola, linguetta viola)
    const pill = rows[0]?.querySelector('mark.dx-call--answered');
    expect(pill).toHaveTextContent('BB2BB');
    expect(pill).toHaveAttribute('title', 'Ti ha risposto');
    expect(rows[0]?.querySelector('.me-mark--answer')).not.toBeNull();
    expect(rows[0]?.querySelector('.me-mark:not(.me-mark--answer)')).toBeNull();
    // CC3CC mi chiama per primo: linguetta rossa, nessuna etichetta viola
    expect(rows[1]?.querySelector('mark.dx-call--answered')).toBeNull();
    expect(rows[1]?.querySelector('.me-mark:not(.me-mark--answer)')).not.toBeNull();
    useStation.setState({ txLog: [] });
  });

  it('puts a station in the queue with the right button and shows the queued ones', () => {
    const onQueue = vi.fn();
    useStation.setState({ status: { dial_hz: 0, mode: 'FT8', tx_enabled: true, transmitting: false, decoding: true, de_call: 'AA1AA', de_grid: 'JN45', queue: ['CC3CC'] } });
    render(
      <DecodeList
        myCall="AA1AA"
        onQueue={onQueue}
        decodes={[decode({ id: 1, text: 'AA1AA BB2BB -10' }), decode({ id: 2, text: 'AA1AA CC3CC -07' })]}
        onReply={() => {}}
      />,
    );
    // le stazioni in coda stanno in cima alla lista: le righe si cercano per nominativo, non per posizione
    const rows = screen.getAllByRole('button');
    const bb = rows.find((r) => r.textContent?.includes('BB2BB')) as HTMLElement;
    const cc = rows.find((r) => r.textContent?.includes('CC3CC')) as HTMLElement;
    expect(bb).toHaveAttribute('title', expect.stringContaining('Tasto destro'));
    fireEvent.contextMenu(bb);
    expect(onQueue).toHaveBeenCalledWith(1);
    expect(cc).toHaveClass('decode-row--queued');
    // il pallino porta il posto in coda, con la spiegazione nel suggerimento, e resta fuori dal testo del messaggio
    const mark = screen.getByLabelText(/In coda, posto 1/);
    expect(mark).toHaveTextContent('1');
    expect(mark.closest('.data')).toBeNull();
    expect(bb).not.toHaveClass('decode-row--queued');
    useStation.setState({ status: null });
  });

  it('right-click on the callsign opens a menu with «Apri su QRZ.com» and the queue', () => {
    const onQueue = vi.fn();
    render(<DecodeList myCall="AA1AA" onQueue={onQueue} decodes={[decode({ id: 1, text: 'CQ EA8/BB2BB IL18' })]} onReply={() => {}} />);
    const call = document.querySelector('.decode-sender') as HTMLElement;
    expect(call).toHaveTextContent('EA8/BB2BB');
    fireEvent.contextMenu(call);
    // il tasto destro sul nominativo apre il menu e non mette subito in coda
    expect(onQueue).not.toHaveBeenCalled();
    const qrz = screen.getByRole('menuitem', { name: 'Apri su QRZ.com' });
    expect(qrz).toHaveAttribute('href', 'https://www.qrz.com/db/EA8%2FBB2BB');
    expect(qrz).toHaveAttribute('target', '_blank');
    fireEvent.click(screen.getByRole('menuitem', { name: 'Metti in coda' }));
    expect(onQueue).toHaveBeenCalledWith(1);
    expect(screen.queryByRole('menu')).toBeNull();
    // sul resto della riga il tasto destro mette in coda come prima, senza menu
    fireEvent.contextMenu(screen.getByText('-12'));
    expect(onQueue).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('the callsign menu says «Togli dalla coda» on every row of a queued station, not only on the newest', () => {
    useStation.setState({ status: { dial_hz: 0, mode: 'FT8', tx_enabled: true, transmitting: false, decoding: true, de_call: 'AA1AA', de_grid: 'JN45', queue: ['CC3CC'] } });
    render(
      <DecodeList myCall="AA1AA" onQueue={() => {}} decodes={[decode({ id: 2, text: 'AA1AA CC3CC -07' }), decode({ id: 1, text: 'CQ CC3CC FN42' })]} onReply={() => {}} />,
    );
    const older = screen.getAllByRole('button').find((r) => r.textContent?.includes('CQ CC3CC')) as HTMLElement;
    expect(older).not.toHaveClass('decode-row--queued');
    fireEvent.contextMenu(older.querySelector('.decode-sender') as HTMLElement);
    expect(screen.getByRole('menuitem', { name: 'Togli dalla coda' })).toBeInTheDocument();
    useStation.setState({ status: null });
  });

  it('closes the callsign menu with Esc', () => {
    render(<DecodeList decodes={[decode({ id: 1, text: 'CQ BB2BB IL18' })]} onReply={() => {}} />);
    fireEvent.contextMenu(document.querySelector('.decode-sender') as HTMLElement);
    expect(screen.getByRole('menu')).toBeInTheDocument();
    // senza onQueue (dove il tasto destro non metteva in coda) il menu ha solo QRZ.com
    expect(screen.getAllByRole('menuitem')).toHaveLength(1);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('shows a Fox row whole, with the FOX tag and the flag and country of the Fox (the call between < >)', async () => {
    // riga DXpedition (i3=0 n3=1) sotto i 1000 Hz: il motore non da' il paese (il mittente e' tra < >), lo si chiede al file dei prefissi
    const text = 'IU3ABC RR73; K1ABC <3X4U> -15';
    const { container } = render(<DecodeList decodes={[decode({ id: 1, text, freq_hz: 450, country: null })]} onReply={() => {}} />);
    const row = screen.getByRole('button');
    expect(row).toHaveClass('decode-row--fox');
    expect(row.querySelector('.data')?.textContent).toBe(text);
    expect(screen.getByText('FOX')).toHaveClass('badge--fox');
    expect(await screen.findByText('Guinea')).toBeInTheDocument();
    expect(container.querySelector('.fi.fi-gn')).not.toBeNull();
  });

  it('takes the country of a Fox from its other rows when the engine knows it there', () => {
    render(
      <DecodeList
        decodes={[decode({ id: 2, text: 'IU3ABC RR73; K1ABC <KH1/KH7Z> -08', freq_hz: 300, country: null }), decode({ id: 1, text: 'CQ KH1/KH7Z AJ10', freq_hz: 300, country: 'Baker & Howland Islands' })]}
        onReply={() => {}}
      />,
    );
    const fox = screen.getAllByRole('button').find((r) => r.textContent?.includes('RR73;')) as HTMLElement;
    expect(fox).toHaveClass('decode-row--fox');
    expect(fox.querySelector('.decode-country')).toHaveTextContent('Baker & Howland Islands');
  });

  it('treats a double message above 1000 Hz as a normal multi-answer, not as a Fox', () => {
    render(<DecodeList decodes={[decode({ id: 1, text: 'IU3ABC RR73; K1ABC <3X4U> -15', freq_hz: 1500, country: null })]} onReply={() => {}} />);
    const row = screen.getByRole('button');
    expect(row).not.toHaveClass('decode-row--fox');
    expect(screen.queryByText('FOX')).toBeNull();
  });

});

describe('distance without a grid in the message', () => {
  it('reuses the grid the station sent earlier', async () => {
    const { gridOfSender } = await import('./DecodeList');
    expect(gridOfSender('CQ S21BDK NL55')).toBe('NL55');
    expect(gridOfSender('JA2ORW S21BDK +07')).toBe('NL55');
    expect(gridOfSender('JA2ORW K9ZZZ -10')).toBeNull();
  });
});

describe('DecodeList and the colours asked over UDP', () => {
  afterEach(() => useStation.getState().reset());

  it('paints the rows of a highlighted callsign, only the newest with «last only», and removes it with invalid colours', () => {
    const rows = [decode({ id: 3, text: 'K1ABC W9XYZ -05' }), decode({ id: 2, text: 'CQ K1ABC FN42' }), decode({ id: 1, text: 'CQ W9XYZ EN52' })];
    const { apply } = useStation.getState();
    apply({ type: 'highlight_callsign', call: 'k1abc', background: '#ffff00', foreground: '#000000', last_only: false });
    const { container, rerender } = render(<DecodeList decodes={rows} onReply={() => {}} />);
    expect(container.querySelectorAll('.decode-row--hl-bg')).toHaveLength(2);
    apply({ type: 'highlight_callsign', call: 'K1ABC', background: '#ff0000', foreground: null, last_only: true });
    rerender(<DecodeList decodes={rows} onReply={() => {}} />);
    const painted = container.querySelectorAll<HTMLElement>('.decode-row--hl-bg');
    expect(painted).toHaveLength(1);
    expect(painted[0]?.style.getPropertyValue('--hl-bg')).toBe('#ff0000');
    expect(container.querySelectorAll('.decode-row--hl-fg')).toHaveLength(0);
    apply({ type: 'highlight_callsign', call: 'K1ABC', background: null, foreground: null, last_only: false });
    rerender(<DecodeList decodes={rows} onReply={() => {}} />);
    expect(container.querySelectorAll('.decode-row--hl-bg')).toHaveLength(0);
  });

  it('ignores colours that are not #rrggbb and empties the lists on a clear', () => {
    const { apply } = useStation.getState();
    apply({ type: 'highlight_callsign', call: 'K1ABC', background: 'red;x:y', foreground: null, last_only: false });
    expect(useStation.getState().highlights).toEqual({});
    apply({ type: 'decode', ...decode() });
    apply({ type: 'clear_decodes', window: 2 });
    expect(useStation.getState().decodes).toEqual([]);
  });
});

describe('DecodeList a griglia (Finestra RX)', () => {
  const KEY = 'test.rxw.cols';
  afterEach(() => {
    localStorage.removeItem(KEY);
    usePrefs.getState().reset();
  });
  const cellsOf = (row: HTMLElement) => Array.from(row.querySelectorAll<HTMLElement>('[data-dl-col]')).map((c) => c.dataset.dlCol);

  it('ha i titoli delle colonne e ogni riga ha le stesse colonne, nello stesso ordine', () => {
    render(<DecodeList decodes={[decode({ id: 42 })]} onReply={() => {}} columnsKey={KEY} />);
    const heads = screen.getAllByRole('columnheader').map((th) => th.dataset.col);
    expect(heads).toEqual(['time', 'snr', 'dt', 'msg', 'country']);
    const row = screen.getByRole('button', { name: /JA1XYZ/ });
    expect(cellsOf(row)).toEqual(heads);
    expect(row.querySelector('[data-dl-col="snr"]')).toHaveTextContent('-12');
    expect(row.querySelector('[data-dl-col="dt"]')).toHaveTextContent('0.3');
    // le linee di griglia: l'ultima colonna non ha la linea a destra
    expect(row.querySelector('[data-dl-col="country"]')).toHaveClass('dl-cell--last');
  });

  it('la colonna elastica e\' il messaggio (i messaggi Fox lunghi non si tagliano), il paese ha una larghezza fissa', () => {
    render(<DecodeList decodes={[decode({ id: 42 })]} onReply={() => {}} columnsKey={KEY} />);
    const section = screen.getByRole('region');
    expect(section.style.getPropertyValue('--dl-cols')).toBe('5.8em 3em 3em minmax(14.5em, 1fr) 11em');
  });

  it("le celle seguono l'ordine salvato e senza DT la colonna sparisce", () => {
    localStorage.setItem(KEY, JSON.stringify({ order: ['msg', 'time', 'snr', 'dt', 'country'], widths: {} }));
    usePrefs.getState().set({ showDt: false });
    render(<DecodeList decodes={[decode({ id: 42 })]} onReply={() => {}} columnsKey={KEY} />);
    expect(screen.getAllByRole('columnheader').map((th) => th.dataset.col)).toEqual(['msg', 'time', 'snr', 'country']);
    expect(cellsOf(screen.getByRole('button', { name: /JA1XYZ/ }))).toEqual(['msg', 'time', 'snr', 'country']);
  });

  it('il doppio clic risponde ancora e le mie trasmissioni ripetute mostrano ×N nella colonna del paese', () => {
    const onReply = vi.fn();
    render(
      <DecodeList
        decodes={[decode({ id: 42 }), decode({ id: -1, time_ms: 45_030_000, snr: 0, text: 'CQ AA1AA FN42' }), decode({ id: -2, time_ms: 45_060_000, snr: 0, text: 'CQ AA1AA FN42' })]}
        onReply={onReply}
        myCall="AA1AA"
        columnsKey={KEY}
      />,
    );
    fireEvent.doubleClick(screen.getByRole('button', { name: /JA1XYZ/ }));
    expect(onReply).toHaveBeenCalledWith(42);
    const tx = screen.getByRole('button', { name: /CQ AA1AA FN42/ });
    expect(tx.querySelector('[data-dl-col="country"] .tx-rep')).toHaveTextContent('×2');
    expect(tx.querySelector('[data-dl-col="snr"]')).toHaveTextContent('Tx');
  });

  it("senza chiave l'elenco resta quello di sempre, senza titoli", () => {
    render(<DecodeList decodes={[decode()]} onReply={() => {}} />);
    expect(screen.queryAllByRole('columnheader')).toHaveLength(0);
    expect(screen.getByText('1500')).toBeInTheDocument();
  });
  it('shows every own transmission of the QSO while it is still running (#42)', () => {
    render(
      <DecodeList
        dxCall="ON7AVL"
        myCall="R70V"
        decodes={[
          decode({ id: -2, time_ms: 45_045_000, snr: 0, text: 'ON7AVL R70V RR73' }),
          decode({ id: 3, time_ms: 45_030_000, text: 'R70V ON7AVL R-12' }),
          decode({ id: -1, time_ms: 45_015_000, snr: 0, text: 'ON7AVL R70V -10' }),
        ]}
        onReply={() => {}}
      />,
    );
    expect(screen.getByText(/R70V -10/)).toBeInTheDocument();
    expect(screen.getAllByText(/R70V RR73/)).toHaveLength(1);
  });

});
