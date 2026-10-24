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

  it('empty state invites listening', () => {
    render(<DecodeList decodes={[]} onReply={() => {}} />);
    expect(screen.getByText('In ascolto: i messaggi compaiono qui')).toBeInTheDocument();
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
