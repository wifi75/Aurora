import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ContestError } from '../../api/contest';

vi.mock('../../api/contestTesto', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/contestTesto')>()),
  getTextContests: vi.fn(() => Promise.resolve({ attivo: null, contest: [], macros: ['macros/x.mc'], domini: [] })),
  getTextContest: vi.fn(() => Promise.reject(new Error('non chiesto'))),
  saveTextContest: vi.fn(() => Promise.resolve({ ok: true, id: 'x' })),
  copyTextContest: vi.fn(() => Promise.resolve({ ok: true, id: 'ARRL 10 Meter Contest' })),
}));
import { copyTextContest, getTextContest, newTextContest, saveTextContest, type TextContest } from '../../api/contestTesto';
import { ContestEditDialog } from './ContestListDialog';

const OFF = /Non previsto da questo contest|Not used by this contest/;

const arrl10: TextContest = {
  ...newTextContest(),
  id: 'ARRL 10 Meter Contest',
  name: 'ARRL 10 Meter Contest',
  origine: 'cartella',
  score: 'bands=10\npoints=mode:CW=4; any=2\nmult=exch/mode/if:dxcc:K,VE; dxcc/mode/except:K,VE',
  valid_cw: true,
  valid_phone: true,
  valid_digital: false,
  auto_inc: true,
  grab_number: true,
  macros: 'macros/x.mc',
  scambio: { ricevuto: [{ nome: 'numero', tipo: 'numero' }], inviato: [{ nome: 'stato', tipo: 'testo' }, { nome: 'numero', tipo: 'numero', progressivo: true }] },
  dichiarazione: { bande: ['28 MHz'], modi: { ammessi: ['cw', 'ssb'], limite: [] }, categorie: {}, doppi: { per: 'modo' }, edizione: null, cabrillo: {} },
  regolamento: '# Regolamento\n\nSolo 10 metri.\n\n- CW e SSB\n- una volta per modo\n\n<script>alert(1)</script>',
};

const rtty: TextContest = { ...newTextContest(), id: 'Vecchio RTTY', name: 'Vecchio RTTY', origine: 'file', score: 'bands=80,40,20\nmult=cq/band; dxcc/band', valid_digital: true };

const show = (contest: TextContest, onSaved = vi.fn(), onClose = vi.fn()) =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <ContestEditDialog contest={contest} onClose={onClose} onSaved={onSaved} />
    </QueryClientProvider>,
  );

const box = (name: RegExp) => screen.getByRole('checkbox', { name });
/** La casella o il campo con il suo tooltip (sta sull'etichetta che lo contiene). */
const tipOf = (el: HTMLElement) => el.closest('label')?.getAttribute('title') ?? el.closest('[title]')?.getAttribute('title') ?? '';

beforeEach(() => {
  vi.mocked(saveTextContest).mockClear();
  vi.mocked(copyTextContest).mockClear();
  vi.mocked(getTextContest).mockClear();
});

describe('finestra delle impostazioni: voci non previste', () => {
  it('ARRL 10 in cartella: digitali, zone CQ/ITU, QTC e bande grigi con il tooltip; CW e fonia accesi ma in sola lettura', async () => {
    show(arrl10);
    const digital = box(/Digitali|Digital/);
    expect(digital).toBeDisabled();
    expect(tipOf(digital)).toMatch(OFF);
    expect(digital.closest('label')).toHaveClass('ctest-off');
    for (const re of [/^Zona CQ$|^CQ zone$/, /^Zona ITU$|^ITU zone$/]) {
      const c = box(re);
      expect(c).toBeDisabled();
      expect(tipOf(c)).toMatch(OFF);
    }
    // CW: previsto dal contest (non grigio) ma regole in sola lettura (disabilitato, senza il tooltip «non previsto»)
    const cw = box(/^CW$/);
    expect(cw).toBeChecked();
    expect(cw).toBeDisabled();
    expect(cw.closest('label')).not.toHaveClass('ctest-off');
    expect(tipOf(cw)).not.toMatch(OFF);
    // le bande: nessuna per banda
    for (const b of screen.getAllByRole('spinbutton', { name: /Peso|Weight/ })) expect(b).toBeDisabled();
    // il valore resta invariato
    expect(digital).not.toBeChecked();
  });

  it('La mia sezione resta modificabile in un contest in cartella (preferenza di questo computer)', async () => {
    show(arrl10);
    const sect = screen.getByLabelText(/La mia sezione|My section/);
    expect(sect).toBeEnabled();
    fireEvent.change(sect, { target: { value: 'ak' } });
    expect(sect).toHaveValue('AK');
  });

  it('mostra il regolamento come testo (paragrafi ed elenchi, nessun HTML)', async () => {
    const { container } = show(arrl10);
    const box_ = screen.getByTestId('ctest-rules');
    expect(box_).toHaveTextContent('Solo 10 metri.');
    expect(box_.querySelectorAll('li')).toHaveLength(2);
    expect(box_.querySelector('script')).toBeNull();
    expect(box_).toHaveTextContent('<script>alert(1)</script>');
    expect(container.querySelector('script')).toBeNull();
  });

  it('Copia nella mia cartella chiama il core e chiude la finestra', async () => {
    const onSaved = vi.fn();
    show(arrl10, onSaved);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /Copia nella mia cartella|Copy to my folder/ }));
    });
    expect(copyTextContest).toHaveBeenCalledWith('ARRL 10 Meter Contest');
    expect(onSaved).toHaveBeenCalledWith('ARRL 10 Meter Contest');
  });

  it('OK in un contest in cartella non manda niente al core', async () => {
    const onSaved = vi.fn();
    show(arrl10, onSaved);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    });
    expect(saveTextContest).not.toHaveBeenCalled();
    expect(onSaved).toHaveBeenCalled();
  });

  it('se il core risponde contest_readonly la finestra resta aperta', async () => {
    vi.mocked(saveTextContest).mockRejectedValueOnce(new ContestError(409, 'contest_readonly', 'contest incluso'));
    const onSaved = vi.fn();
    show(rtty, onSaved);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    });
    expect(saveTextContest).toHaveBeenCalledTimes(1);
    expect(onSaved).not.toHaveBeenCalled();
  });

  it('un vecchio file RTTY: CW e fonia grigi, digitali accesi e modificabili, zona CQ accesa e ITU grigia', async () => {
    show(rtty);
    expect(box(/^CW$/)).toBeDisabled();
    expect(tipOf(box(/^CW$/))).toMatch(OFF);
    expect(box(/Digitali|Digital/)).toBeEnabled();
    expect(box(/^Zona CQ$|^CQ zone$/)).toBeEnabled();
    expect(box(/^Zona ITU$|^ITU zone$/)).toBeDisabled();
    // niente riquadro del regolamento ne' avviso di sola lettura per un file
    expect(screen.queryByTestId('ctest-rules')).toBeNull();
    expect(screen.queryByRole('button', { name: /Copia nella mia cartella|Copy to my folder/ })).toBeNull();
    // le bande fuori da bands= sono grigie, quelle dentro accese
    const pesi = screen.getAllByRole('spinbutton', { name: /Peso|Weight/ });
    expect(pesi[0]).toBeDisabled(); // 160
    expect(pesi[1]).toBeEnabled(); // 80
  });

  it('un vecchio file si salva senza i campi della cartella', async () => {
    show({ ...rtty, regolamento: 'x', dichiarazione: null });
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    });
    const body = vi.mocked(saveTextContest).mock.calls[0]?.[0] as unknown as Record<string, unknown>;
    expect(body).toBeDefined();
    for (const k of ['origine', 'scambio', 'dichiarazione', 'regolamento']) expect(k in body).toBe(false);
  });
});
