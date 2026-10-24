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
    expect(screen.getByText('DXCC NUOVA BANDA')).toBeInTheDocument();
    expect(screen.getByText('NUOVA GRIGLIA')).toBeInTheDocument();
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

  it('shows the delta time column', () => {
    render(<DecodeList decodes={[decode({ delta_time: 0.3 })]} onReply={() => {}} />);
    expect(screen.getByText('0.3')).toBeInTheDocument();
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

  it('row click calls onReply with id', () => {
    const onReply = vi.fn();
    render(<DecodeList decodes={[decode({ id: 42 })]} onReply={onReply} />);
    fireEvent.click(screen.getByRole('button', { name: /JA1XYZ/ }));
    expect(onReply).toHaveBeenCalledWith(42);
  });

  it('shows time snr and frequency of each row', () => {
    render(<DecodeList decodes={[decode()]} onReply={() => {}} />);
    expect(screen.getByText('12:30:15')).toBeInTheDocument();
    expect(screen.getByText('-12')).toBeInTheDocument();
    expect(screen.getByText('1500')).toBeInTheDocument();
  });
});
