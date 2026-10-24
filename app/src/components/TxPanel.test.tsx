import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ postCq: vi.fn().mockResolvedValue(undefined), postFreeText: vi.fn().mockResolvedValue(undefined), postTxFrequency: vi.fn().mockResolvedValue(undefined) }));
import { postCq, postFreeText, postTxFrequency } from '../api/client';
import { TxPanel } from './TxPanel';

beforeEach(() => {
  vi.mocked(postCq).mockClear();
  vi.mocked(postFreeText).mockClear();
  vi.mocked(postTxFrequency).mockClear();
});

describe('TxPanel', () => {
  it('with transmission closed it explains and disables what would transmit', () => {
    render(<TxPanel txAllowed={false} />);
    expect(screen.getByText(/trasmissione chiusa/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^CQ/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: /invia/i })).toBeDisabled();
  });

  it('calls CQ, with DX when chosen', async () => {
    render(<TxPanel txAllowed />);
    fireEvent.click(screen.getByRole('button', { name: /^CQ$/ }));
    await waitFor(() => expect(postCq).toHaveBeenCalledWith(''));
    fireEvent.click(screen.getByLabelText('DX'));
    fireEvent.click(screen.getByRole('button', { name: /^CQ DX$/ }));
    await waitFor(() => expect(postCq).toHaveBeenLastCalledWith('DX'));
  });

  it('sends a free text in uppercase, limited to 13 characters', async () => {
    render(<TxPanel txAllowed />);
    const box = screen.getByLabelText(/messaggio libero/i);
    fireEvent.change(box, { target: { value: 'tnx 73 gl dx e altro' } });
    expect(box).toHaveValue('TNX 73 GL DX ');
    fireEvent.click(screen.getByRole('button', { name: /invia/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('TNX 73 GL DX', true));
  });

  it('prepares a message without transmitting, even with transmission closed', async () => {
    render(<TxPanel txAllowed={false} />);
    fireEvent.change(screen.getByLabelText(/messaggio libero/i), { target: { value: '73' } });
    fireEvent.click(screen.getByRole('button', { name: /prepara/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('73', false));
  });

  it('sets the transmit frequency within the audio band', async () => {
    render(<TxPanel txAllowed={false} />);
    fireEvent.change(screen.getByLabelText(/frequenza di trasmissione/i), { target: { value: '1800' } });
    fireEvent.click(screen.getByRole('button', { name: /imposta/i }));
    await waitFor(() => expect(postTxFrequency).toHaveBeenCalledWith(1800));
  });
});
