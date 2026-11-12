import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, within } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', async (importOriginal) => {
  const real = await importOriginal<typeof import('../api/client')>();
  return {
    ...real,
    getHealth: vi.fn(() => Promise.resolve({ version: '0.11.239' })),
    getEngineSettings: vi.fn(() => Promise.reject(new Error('offline'))),
    getStationInfo: vi.fn(() => Promise.reject(new Error('offline'))),
    getDigi: vi.fn(() => Promise.reject(new Error('offline'))),
  };
});
import { allPages, INFO_ID } from '../lib/settingsTree';
import { SETTINGS_PAGES, SettingsWindow } from './SettingsWindow';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const tree = () => screen.getByRole('navigation', { name: 'Impostazioni' });

beforeEach(() => {
  localStorage.clear();
});

describe('SettingsWindow', () => {
  it('ogni pagina dell albero ha il suo contenuto, piu Informazioni', () => {
    const ids = SETTINGS_PAGES.map((p) => p.id);
    expect(ids).toEqual([...allPages().map((p) => p.page.id), INFO_ID]);
    for (const p of SETTINGS_PAGES) expect(p.render).toBeTypeOf('function');
  });

  it('aperta senza una pagina richiesta: famiglie tutte chiuse e al centro Informazioni', async () => {
    render(<SettingsWindow embedded onClose={() => undefined} />);
    const families = within(tree())
      .getAllByRole('button')
      .filter((b) => b.hasAttribute('aria-expanded'));
    expect(families).toHaveLength(5);
    for (const f of families) expect(f).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByRole('button', { name: /Informazioni/ })).toHaveAttribute('aria-current', 'page');
    expect(await screen.findByRole('heading', { name: 'AURORA' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Dona con PayPal/ })).toHaveAttribute('href', 'https://www.paypal.com/donate/?hosted_button_id=MY55Q6GDQEH8L');
    expect(await screen.findByText('0.11.239', { selector: '.sw-ver' })).toBeInTheDocument();
  });

  it('aperta da PSK · RTTY · CW: rami aperti fino al nodo del modo e percorso sopra il contenuto', () => {
    render(<SettingsWindow embedded initialTab="modedigi" onClose={() => undefined} />);
    expect(screen.getByRole('button', { name: /^Modi/ })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('button', { name: /^Stazione/ })).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByRole('button', { name: /^PSK · RTTY · CW/ })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('button', { name: 'Ricezione e trasmissione' })).toHaveAttribute('aria-current', 'page');
    const path = screen.getByRole('list', { name: 'Percorso' });
    expect(within(path).getAllByRole('listitem').map((li) => li.textContent)).toEqual(['Modi', 'PSK · RTTY · CW', 'Ricezione e trasmissione']);
  });

  it('un collegamento vecchio porta alla pagina nuova', () => {
    render(<SettingsWindow embedded initialTab="filters" onClose={() => undefined} />);
    expect(screen.getByRole('button', { name: 'Messaggi e filtri' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('button', { name: /^FT8 · FT4/ })).toHaveAttribute('aria-expanded', 'true');
  });

  it('le famiglie si aprono a fisarmonica: aprendone una le altre si chiudono', () => {
    render(<SettingsWindow embedded onClose={() => undefined} />);
    fireEvent.click(screen.getByRole('button', { name: /^Stazione/ }));
    expect(screen.getByRole('button', { name: /^Stazione/ })).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(screen.getByRole('button', { name: /^Modi/ }));
    expect(screen.getByRole('button', { name: /^Modi/ })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('button', { name: /^Stazione/ })).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(screen.getByRole('button', { name: /^SSTV/ }));
    fireEvent.click(screen.getByRole('button', { name: /^Satelliti/ }));
    expect(screen.getByRole('button', { name: /^SSTV/ })).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByRole('button', { name: /^Satelliti/ })).toHaveAttribute('aria-expanded', 'true');
  });
});
