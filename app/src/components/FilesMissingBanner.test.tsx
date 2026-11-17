import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getFilesCheck: vi.fn(), restoreHamlib: vi.fn() }));
import { getFilesCheck, restoreHamlib } from '../api/client';
import { FilesMissingBanner, isHamlibFile, seenKey } from './FilesMissingBanner';

const show = () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <FilesMissingBanner />
    </QueryClientProvider>,
  );
};

beforeEach(() => {
  localStorage.clear();
  vi.mocked(getFilesCheck).mockReset();
  vi.mocked(restoreHamlib).mockReset();
});
afterEach(cleanup);

describe('FilesMissingBanner', () => {
  it('knows the Hamlib files and keys the dismissal on the list', () => {
    expect(isHamlibFile('hamlib\\rigctld.exe')).toBe(true);
    expect(isHamlibFile('cty\\cty.dat')).toBe(false);
    expect(seenKey(['b', 'A'])).toBe(seenKey(['a', 'b']));
    expect(seenKey(['a'])).not.toBe(seenKey(['a', 'b']));
  });

  it('shows nothing when no file is missing', async () => {
    vi.mocked(getFilesCheck).mockResolvedValue({ missing: [], antivirus: '' });
    show();
    await waitFor(() => expect(getFilesCheck).toHaveBeenCalled());
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('names the antivirus and the files, with the Guide link and the Hamlib restore', async () => {
    vi.mocked(getFilesCheck).mockResolvedValue({ missing: ['hamlib\\rigctld.exe', 'cty\\cty.dat'], antivirus: 'Avast Free Antivirus' });
    show();
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('L’antivirus (Avast Free Antivirus) ha tolto dei file ad Aurora: hamlib\\rigctld.exe, cty\\cty.dat');
    expect(screen.getByRole('link', { name: 'Cosa fare' }).getAttribute('href')).toBe('#/guida/faq-antivirus');
    expect(screen.getByRole('button', { name: 'Ripristina Hamlib' })).toBeInTheDocument();
  });

  it('says only "L’antivirus" when the name is unknown and has no restore without Hamlib files', async () => {
    vi.mocked(getFilesCheck).mockResolvedValue({ missing: ['cty\\cty.dat'], antivirus: '' });
    show();
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('L’antivirus ha tolto dei file ad Aurora: cty\\cty.dat');
    expect(screen.queryByRole('button', { name: 'Ripristina Hamlib' })).toBeNull();
  });

  it('is hidden by "Ho capito" for that list only', async () => {
    vi.mocked(getFilesCheck).mockResolvedValue({ missing: ['cty\\cty.dat'], antivirus: '' });
    const first = show();
    fireEvent.click(await screen.findByRole('button', { name: 'Ho capito' }));
    expect(screen.queryByRole('alert')).toBeNull();
    expect(localStorage.getItem(seenKey(['cty\\cty.dat']))).toBe('1');
    first.unmount();
    // stesso elenco: resta nascosto
    show();
    await waitFor(() => expect(getFilesCheck).toHaveBeenCalledTimes(2));
    expect(screen.queryByRole('alert')).toBeNull();
    cleanup();
    // un altro elenco: ricompare
    vi.mocked(getFilesCheck).mockResolvedValue({ missing: ['cty\\cty.dat', 'dist\\index.html'], antivirus: '' });
    show();
    expect(await screen.findByRole('alert')).toBeInTheDocument();
  });

  it('restores Hamlib and asks the check again', async () => {
    vi.mocked(getFilesCheck).mockResolvedValueOnce({ missing: ['hamlib\\rigctld.exe'], antivirus: '' }).mockResolvedValue({ missing: [], antivirus: '' });
    vi.mocked(restoreHamlib).mockResolvedValue({} as never);
    show();
    fireEvent.click(await screen.findByRole('button', { name: 'Ripristina Hamlib' }));
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
    expect(restoreHamlib).toHaveBeenCalledTimes(1);
  });

  it('tells when the restore fails', async () => {
    vi.mocked(getFilesCheck).mockResolvedValue({ missing: ['hamlib\\rigctld.exe'], antivirus: '' });
    vi.mocked(restoreHamlib).mockRejectedValue(new Error('no'));
    show();
    fireEvent.click(await screen.findByRole('button', { name: 'Ripristina Hamlib' }));
    expect(await screen.findByText(/Non sono riuscito a ripristinare Hamlib/)).toBeInTheDocument();
  });
});
