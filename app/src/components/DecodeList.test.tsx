import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { DecodeList } from './DecodeList';

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

  it('shows the country of the sender when known', () => {
    render(<DecodeList decodes={[decode({ country: 'Giappone' })]} onReply={() => {}} />);
    expect(screen.getByText('Giappone')).toBeInTheDocument();
  });

  it('shows the distance in km like JTDX when the own locator is known', () => {
    render(<DecodeList decodes={[decode({ country: 'Giappone' })]} myGrid="JN45" onReply={() => {}} />);
    expect(screen.getByText(/\[\d+ km\]/)).toBeInTheDocument();
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
    expect(badge).toHaveAttribute('title', 'Prefisso su nuova banda');
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
});
