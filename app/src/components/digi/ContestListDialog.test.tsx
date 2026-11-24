import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/contestTesto', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/contestTesto')>()),
  getTextContests: vi.fn(() => Promise.resolve({ attivo: 'B', contest: [{ id: 'A', name: 'Alfa RTTY', notes: 'RTTY', start: '', end: '' }, { id: 'B', name: 'Beta PSK', notes: 'PSK63', start: '', end: '' }], macros: [], domini: [] })),
  selectTextContest: vi.fn(() => Promise.resolve({ ok: true, attivo: 'A' })),
  deleteTextContest: vi.fn(() => Promise.resolve({ ok: true })),
  importTextContest: vi.fn(() => Promise.resolve({ ok: true, id: 'Gamma' })),
  exportTextContest: vi.fn(() => Promise.resolve(new Blob(['PK']))),
}));
vi.mock('../ConfirmHost', () => ({ askConfirm: vi.fn(() => Promise.resolve(true)) }));
import { deleteTextContest, exportTextContest, getTextContests, importTextContest, selectTextContest } from '../../api/contestTesto';
import { within } from '@testing-library/react';
import { ContestListDialog, fromLocalInput, toLocalInput } from './ContestListDialog';

const dialog = (onClose = vi.fn()) => (
  <QueryClientProvider client={new QueryClient()}>
    <ContestListDialog onClose={onClose} />
  </QueryClientProvider>
);

beforeEach(() => {
  vi.mocked(selectTextContest).mockClear();
  vi.mocked(deleteTextContest).mockClear();
});

describe('elenco dei contest', () => {
  it('prima riga «Nessun contest», freccia su quello attivo, doppio clic seleziona', async () => {
    const onClose = vi.fn();
    render(dialog(onClose));
    const alfa = await screen.findByText('Alfa RTTY');
    const rows = screen.getAllByRole('row');
    // titolo, «Nessun contest», Alfa, Beta
    expect(rows).toHaveLength(4);
    expect(rows[3]).toHaveTextContent('►');
    expect(rows[2]).not.toHaveTextContent('►');
    await act(async () => {
      fireEvent.doubleClick(alfa);
    });
    expect(selectTextContest).toHaveBeenCalledWith('A');
    expect(onClose).toHaveBeenCalled();
  });

  it('Elimina chiede conferma e cancella la riga scelta; «Nessun contest» non si elimina', async () => {
    render(dialog());
    const beta = await screen.findByText('Beta PSK');
    fireEvent.click(beta);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /Elimina|Delete/ }));
    });
    expect(deleteTextContest).toHaveBeenCalledWith('B');
    fireEvent.click(screen.getAllByRole('row')[1] as HTMLElement);
    expect(screen.getByRole('button', { name: /Elimina|Delete/ })).toBeDisabled();
  });

  it('Importa manda il file scelto, Esporta scarica il pacchetto del contest scelto', async () => {
    URL.createObjectURL = vi.fn(() => 'blob:x');
    URL.revokeObjectURL = vi.fn();
    render(dialog());
    const alfa = await screen.findByText('Alfa RTTY');
    const file = new File(['[Contest]'], 'Gamma.zip');
    await act(async () => {
      fireEvent.change(screen.getByTestId('ctest-import-file'), { target: { files: [file] } });
    });
    expect(importTextContest).toHaveBeenCalledWith(file);
    fireEvent.click(alfa);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /Esporta contest|Export contest/ }));
    });
    expect(exportTextContest).toHaveBeenCalledWith('A');
  });

  it('date del file e campo data/ora', () => {
    expect(toLocalInput('20150926000001')).toBe('2015-09-26T00:00:01');
    expect(toLocalInput('')).toBe('');
    expect(fromLocalInput('2015-09-26T00:00')).toBe('20150926000000');
    expect(fromLocalInput('')).toBe('');
  });
});

describe('linguette per modo', () => {
  const ref = (id: string, name: string, notes: string) => ({ id, name, notes, start: '', end: '' });
  const many = () =>
    vi.mocked(getTextContests).mockImplementationOnce(() =>
      Promise.resolve({ attivo: null, contest: [ref('1', 'Uno', 'CW & SSB'), ref('2', 'Due', 'RTTY 75 Baudot'), ref('3', 'Tre', 'Phone - CW - Digital'), ref('4', 'Quattro', 'Sprint')], macros: [], domini: [] }),
    );
  const tabs = () => within(screen.getByRole('tablist'));
  const names = () => screen.getAllByRole('row').slice(2).map((r) => r.querySelectorAll('td')[1]?.textContent);

  it('un contest con due modi compare in entrambe le linguette; il numero sta accanto al nome', async () => {
    many();
    render(dialog());
    await screen.findByText('Uno');
    expect(tabs().getByRole('tab', { name: /Tutti|All/ })).toHaveTextContent('4');
    expect(tabs().getByRole('tab', { name: /SSB/ })).toHaveTextContent('2');
    expect(tabs().getByRole('tab', { name: /CW/ })).toHaveTextContent('2');
    fireEvent.click(tabs().getByRole('tab', { name: /SSB/ }));
    expect(names()).toEqual(['Uno', 'Tre']);
    fireEvent.click(tabs().getByRole('tab', { name: /CW/ }));
    expect(names()).toEqual(['Uno', 'Tre']);
    fireEvent.click(tabs().getByRole('tab', { name: /RTTY/ }));
    expect(names()).toEqual(['Due']);
    fireEvent.click(tabs().getByRole('tab', { name: /Altri|Other/ }));
    expect(names()).toEqual(['Quattro']);
  });

  it('la ricerca vale dentro la linguetta; una linguetta vuota e disattivata; frecce per spostarsi', async () => {
    many();
    render(dialog());
    // la linguetta scelta nella prova prima (Altri) si ricorda: si riparte da li'
    await screen.findByText('Quattro');
    expect(tabs().getByRole('tab', { name: /Altri|Other/ })).toHaveAttribute('aria-selected', 'true');
    fireEvent.click(tabs().getByRole('tab', { name: /Tutti|All/ }));
    await screen.findByText('Uno');
    expect(tabs().getByRole('tab', { name: /PSK/ })).toBeDisabled();
    fireEvent.click(tabs().getByRole('tab', { name: /SSB/ }));
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'tre' } });
    expect(names()).toEqual(['Tre']);
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: '' } });
    fireEvent.keyDown(tabs().getByRole('tab', { name: /SSB/ }), { key: 'ArrowRight' });
    expect(tabs().getByRole('tab', { name: /CW/ })).toHaveAttribute('aria-selected', 'true');
    fireEvent.keyDown(tabs().getByRole('tab', { name: /CW/ }), { key: 'ArrowLeft' });
    expect(tabs().getByRole('tab', { name: /SSB/ })).toHaveAttribute('aria-selected', 'true');
  });
});

describe('ufficiali e Beta contest', () => {
  const ref = (id: string, name: string, beta: boolean) => ({ id, name, notes: 'RTTY', start: '', end: '', beta });
  const names = () => screen.getAllByRole('row').slice(2).map((r) => r.querySelectorAll('td')[1]?.textContent);

  it('i contest beta stanno in una scheda a parte, con l avviso; gli ufficiali restano nell elenco di base', async () => {
    vi.mocked(getTextContests).mockImplementationOnce(() => Promise.resolve({ attivo: null, contest: [ref('1', 'Ufficiale RTTY', false), ref('2', 'Vecchio RTTY', true), ref('3', 'Altro RTTY', true)], macros: [], domini: [] }));
    render(dialog());
    await screen.findByText('Ufficiale RTTY');
    expect(names()).toEqual(['Ufficiale RTTY']);
    const sets = within(screen.getByRole('tablist', { name: /Elenco dei contest|Contest list/ }));
    expect(sets.getByRole('tab', { name: /Beta/ })).toHaveTextContent('2');
    fireEvent.click(sets.getByRole('tab', { name: /Beta/ }));
    expect(names()).toEqual(['Vecchio RTTY', 'Altro RTTY']);
    expect(document.querySelector('.ctest-beta-note')).not.toBeNull();
    fireEvent.click(sets.getByRole('tab', { name: /ufficiali|Official/ }));
    expect(names()).toEqual(['Ufficiale RTTY']);
    expect(document.querySelector('.ctest-beta-note')).toBeNull();
  });

  it('si apre sulla scheda del contest attivo anche se la lista arriva dopo, e cambiando scheda la riga scelta si azzera', async () => {
    vi.mocked(getTextContests).mockImplementationOnce(() => Promise.resolve({ attivo: '2', contest: [ref('1', 'Ufficiale RTTY', false), ref('2', 'Vecchio RTTY', true)], macros: [], domini: [] }));
    render(dialog());
    await screen.findByText('Vecchio RTTY');
    expect(names()).toEqual(['Vecchio RTTY']);
    const sets = within(screen.getByRole('tablist', { name: /Elenco dei contest|Contest list/ }));
    fireEvent.click(screen.getByText('Vecchio RTTY'));
    fireEvent.click(sets.getByRole('tab', { name: /ufficiali|Official/ }));
    expect(names()).toEqual(['Ufficiale RTTY']);
    expect(screen.queryByText('Vecchio RTTY')).toBeNull();
  });
});
