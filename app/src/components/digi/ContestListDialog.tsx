import { useRef, useState, type ReactNode } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useT } from '../../i18n';
import { ContestError } from '../../api/contest';
import { copyTextContest, deleteTextContest, exportTextContest, importTextContest, getTextContest, getTextContests, newTextContest, saveTextContest, selectTextContest, type TextContest } from '../../api/contestTesto';
import { askConfirm } from '../ConfirmHost';
import { checkContestDates, staleWindowProposal, startSerial, windowProposals } from '../../lib/textContest';
import { toast } from '../../lib/toast';
import { EXTRA_MAX, cleanExtra, readSection, writeSection } from '../../lib/contestSection';
import { blocchiRegolamento, inCartella, voceAttiva, voceModificabile, type Voce } from '../../lib/contestVoci';
import { browserStorage } from '../../lib/theme';
import '../../styles/digi-contest.css';

const why = (e: unknown) => (e instanceof ContestError ? (e.detail ?? e.code) : String(e));

/** Le chiavi delle richieste dei contest dei modi testuali (elenco e contest attivo), da rileggere dopo ogni modifica. */
export const TEXT_CONTEST_KEYS = [['contest-testo'], ['contest-testo-attivo'], ['contest-testo-punteggio']] as const;

function useRefresh() {
  const client = useQueryClient();
  return () => {
    for (const k of TEXT_CONTEST_KEYS) void client.invalidateQueries({ queryKey: [...k] });
  };
}

/** Sceglie il contest attivo (`null` = «Nessun contest») e lo dice. */
export function useSelectTextContest() {
  const t = useT();
  const refresh = useRefresh();
  return async (id: string | null, name?: string) => {
    try {
      await selectTextContest(id);
      toast(id ? t('ctest.selected', { name: name ?? id }) : t('ctest.deselected'), 'info');
      refresh();
      return true;
    } catch (e) {
      toast(t('ctest.error', { why: why(e) }), 'error');
      return false;
    }
  };
}

/**
 * La finestra «Elenco dei contest»: colonne Attivo (freccia su quello attivo), Nome, Note; prima riga «Nessun contest»; pulsanti
 * Modifica…, Elimina, Aggiungi…, Importa contest…, Esporta contest…, Seleziona, Chiudi. Doppio clic su una riga = Seleziona.
 * Importa ed esporta usano il pacchetto `.zip` del core (`.contest` con le sue macro e i domini; si importa anche un `.contest`).
 */
export function ContestListDialog({ onClose, onSelected }: { onClose(): void; onSelected?(id: string | null): void }) {
  const t = useT();
  const refresh = useRefresh();
  const select = useSelectTextContest();
  const list = useQuery({ queryKey: ['contest-testo'], queryFn: getTextContests, retry: false });
  const [picked, setPicked] = useState<string | null | undefined>(undefined);
  const [editing, setEditing] = useState<TextContest | null>(null);
  const active = list.data?.attivo ?? null;
  const current = picked === undefined ? active : picked;
  const [query, setQuery] = useState('');
  const rows = [{ id: null as string | null, name: t('ctest.none'), notes: '' }, ...(list.data?.contest ?? [])];
  // la ricerca guarda nome e note; «Nessun contest» resta sempre in cima
  const q = query.trim().toLowerCase();
  const shown = q === '' ? rows : rows.filter((r) => r.id === null || `${r.name} ${r.notes}`.toLowerCase().includes(q));

  const choose = async (id: string | null) => {
    const name = rows.find((r) => r.id === id)?.name;
    if (await select(id, name ?? undefined)) {
      onSelected?.(id);
      onClose();
    }
  };
  const edit = async (id: string | null) => {
    if (!id) return;
    try {
      setEditing(await getTextContest(id));
    } catch (e) {
      toast(t('ctest.error', { why: why(e) }), 'error');
    }
  };
  const fileRef = useRef<HTMLInputElement>(null);
  const importFile = async (file: File | undefined) => {
    if (!file) return;
    try {
      const r = await importTextContest(file);
      toast(t('ctest.imported', { name: r.id }), 'ok');
      setPicked(r.id);
      refresh();
    } catch (e) {
      toast(t('ctest.error', { why: why(e) }), 'error');
    }
  };
  const exportOne = async (id: string | null) => {
    if (!id) return;
    try {
      const blob = await exportTextContest(id);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${id}.zip`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      toast(t('ctest.exported', { name: `${id}.zip` }), 'ok');
    } catch (e) {
      toast(t('ctest.error', { why: why(e) }), 'error');
    }
  };
  const remove = async (id: string | null) => {
    if (!id) return;
    const name = rows.find((r) => r.id === id)?.name ?? id;
    if (!(await askConfirm(t('ctest.deleteConfirm', { name })))) return;
    try {
      await deleteTextContest(id);
      toast(t('ctest.deleted'), 'info');
      setPicked(undefined);
      refresh();
    } catch (e) {
      toast(t('ctest.error', { why: why(e) }), 'error');
    }
  };

  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('ctest.listTitle')} onKeyDown={(e) => (e.key === 'Escape' && !editing ? onClose() : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <div className="digi-dialog ctest-dialog">
        <header>
          <strong>{t('ctest.listTitle')}</strong>
        </header>
        <input
          type="search"
          className="ctest-search"
          value={query}
          placeholder={t('ctest.search')}
          aria-label={t('ctest.search')}
          autoFocus
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => (e.key === 'Escape' && query ? (e.stopPropagation(), setQuery('')) : undefined)}
        />
        <div className="ctest-table-wrap">
          <table className="ctest-table">
            <thead>
              <tr>
                <th className="ctest-col-active">{t('ctest.colActive')}</th>
                <th>{t('ctest.colName')}</th>
                <th>{t('ctest.colNotes')}</th>
              </tr>
            </thead>
            <tbody>
              {list.isLoading ? (
                <tr>
                  <td colSpan={3}>{t('ctest.loading')}</td>
                </tr>
              ) : (
                shown.map((r) => (
                  <tr key={r.id ?? ''} className={r.id === current ? 'is-picked' : undefined} aria-selected={r.id === current} onClick={() => setPicked(r.id)} onDoubleClick={() => void choose(r.id)}>
                    <td className="ctest-col-active">{r.id === active ? '►' : ''}</td>
                    <td>{r.name}</td>
                    <td>{r.notes}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
        <footer>
          <button type="button" className="glass sstv-btn" disabled={!current} onClick={() => void edit(current)}>
            {t('ctest.edit')}
          </button>
          <button type="button" className="glass sstv-btn" disabled={!current} onClick={() => void remove(current)}>
            {t('ctest.delete')}
          </button>
          <button type="button" className="glass sstv-btn" onClick={() => setEditing(newTextContest())}>
            {t('ctest.add')}
          </button>
          <button type="button" className="glass sstv-btn" title={t('ctest.tip.import')} onClick={() => fileRef.current?.click()}>
            {t('ctest.import')}
          </button>
          <button type="button" className="glass sstv-btn" title={t('ctest.tip.export')} disabled={!current} onClick={() => void exportOne(current)}>
            {t('ctest.export')}
          </button>
          <input
            ref={fileRef}
            type="file"
            accept=".zip,.contest"
            hidden
            data-testid="ctest-import-file"
            onChange={(e) => {
              const f = e.target.files?.[0];
              e.target.value = '';
              void importFile(f);
            }}
          />
          <span className="rtty-spacer" />
          <button type="button" className="glass sstv-btn" onClick={() => void choose(current)}>
            {t('ctest.select')}
          </button>
          <button type="button" className="glass sstv-btn" onClick={onClose}>
            {t('ctest.close')}
          </button>
        </footer>
      </div>
      {editing ? (
        <ContestEditDialog
          contest={editing}
          onClose={() => setEditing(null)}
          onSaved={(id) => {
            setPicked(id);
            setEditing(null);
          }}
        />
      ) : null}
    </div>
  );
}

/** Un gruppo della finestra di modifica. */
function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="ctest-group">
      <legend>{title}</legend>
      {children}
    </fieldset>
  );
}

/** `aaaammgghhmmss` <-> valore di `<input type="datetime-local">` (UTC). */
export function toLocalInput(d: string): string {
  const m = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})$/.exec(d.trim());
  return m ? `${m[1]}-${m[2]}-${m[3]}T${m[4]}:${m[5]}:${m[6]}` : '';
}

export function fromLocalInput(v: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?/.exec(v.trim());
  return m ? `${m[1]}${m[2]}${m[3]}${m[4]}${m[5]}${m[6] ?? '00'}` : '';
}

/** La finestra di modifica di un contest (nuovo se senza id): tutti i campi del file, in gruppi, con le spiegazioni. */
export function ContestEditDialog({ contest, onClose, onSaved }: { contest: TextContest; onClose(): void; onSaved(id: string): void }) {
  const t = useT();
  const list = useQuery({ queryKey: ['contest-testo'], queryFn: getTextContests, retry: false }).data;
  const macroFiles = list?.macros ?? [];
  const domainFiles = list?.domini ?? [];
  const refresh = useRefresh();
  // un contest con una sola edizione conosciuta e date di un anno passato parte con le date dell'anno in corso (si salvano con OK)
  // (non per i contest in cartella: le date dell'edizione le calcola il core dalla regola)
  const readonly = inCartella(contest);
  const [proposed] = useState(() => (contest.id && !readonly ? staleWindowProposal(contest, Date.now()) : null));
  const [c, setC] = useState<TextContest>(() => (proposed ? { ...contest, start: proposed.start, end: proposed.end } : contest));
  const [busy, setBusy] = useState(false);
  // la propria sezione / stato / club di questo contest (preferenze del browser, per contest): serve ai contest con un moltiplicatore `exch`
  const [mySect, setMySect] = useState(() => readSection(browserStorage(), contest.id));
  const datesCheck = checkContestDates(c.start, c.end);
  const proposals = c.id && !readonly ? windowProposals(c.name, new Date().getUTCFullYear()) : [];
  // dichiarazione, scambio e regolamento di un contest in cartella: se non sono arrivati con il contest (il contest attivo non li porta) si chiedono al core
  const full = useQuery({ queryKey: ['contest-testo-voce', contest.id], queryFn: () => getTextContest(contest.id), enabled: readonly && contest.dichiarazione === undefined, retry: false }).data;
  // il contest com'e' dichiarato: serve a decidere quali voci valgono. Le validita' per modo e il QTC restano quelli del contest aperto, cosi' una casella spenta si puo' riaccendere
  const view: TextContest = { ...c, origine: contest.origine, scambio: contest.scambio ?? full?.scambio, dichiarazione: contest.dichiarazione ?? full?.dichiarazione, valid_cw: contest.valid_cw, valid_phone: contest.valid_phone, valid_digital: contest.valid_digital, qtc: contest.qtc };
  const regolamento = contest.regolamento ?? full?.regolamento ?? null;
  const off = (v: Voce) => !voceAttiva(view, v);
  const lock = (v: Voce) => off(v) || !voceModificabile(view, v);
  const tipOf = (v: Voce, tip?: string) => (off(v) ? t('ctest.off') : tip);
  const cls = (v: Voce, base = '') => [base, off(v) ? 'ctest-off' : ''].filter(Boolean).join(' ') || undefined;
  const day = (d: string) => `${d.slice(0, 4)}-${d.slice(4, 6)}-${d.slice(6, 8)}`;
  const set = (p: Partial<TextContest>) => setC((x) => ({ ...x, ...p }));
  const num = (v: string) => (Number.isFinite(Number(v)) ? Math.trunc(Number(v)) : 0);
  const check = (k: keyof TextContest, label: string, tip?: string) => (
    <label className={cls(k as Voce, 'ctest-check')} title={tipOf(k as Voce, tip)}>
      <input type="checkbox" checked={c[k] === true} disabled={lock(k as Voce)} onChange={(e) => set({ [k]: e.target.checked } as Partial<TextContest>)} />
      <span>{label}</span>
    </label>
  );
  const int = (k: keyof TextContest, label: string, tip?: string) => (
    <label className={cls(k as Voce)} title={tipOf(k as Voce, tip)}>
      <span>{label}</span>
      <input type="number" value={Number(c[k])} disabled={lock(k as Voce)} onChange={(e) => set({ [k]: num(e.target.value) } as Partial<TextContest>)} />
    </label>
  );
  // i tipi di moltiplicatore: 0 nessuno, 1 paese DXCC, 2 prefisso WPX, 3 zona CQ, 4 zona ITU, 5 dominio (un valore sconosciuto resta)
  const multSelect = (k: 'multipliers' | 'multipliers_option', label: string) => (
    <label className={cls(k)} title={tipOf(k, t('ctest.tip.mult'))}>
      <span>{label}</span>
      <select value={c[k]} disabled={lock(k)} onChange={(e) => set({ [k]: num(e.target.value) } as Partial<TextContest>)}>
        {[0, 1, 2, 3, 4, 5].map((n) => (
          <option key={n} value={n}>
            {t(`ctest.m.${n}` as 'ctest.m.0')}
          </option>
        ))}
        {c[k] < 0 || c[k] > 5 ? <option value={c[k]}>{c[k]}</option> : null}
      </select>
    </label>
  );
  const save = async () => {
    setBusy(true);
    try {
      // la propria sezione e' una preferenza di questo computer: vale anche per i contest in cartella, che non si salvano
      if (readonly) {
        if (voceAttiva(view, 'mySect')) writeSection(browserStorage(), c.id, mySect);
        toast(t('ctest.saved'), 'info');
        onSaved(c.id);
        return;
      }
      // al core vanno solo i campi del file: origine, scambio, dichiarazione e regolamento sono del contest in cartella
      const body: Record<string, unknown> = { ...c };
      for (const k of ['origine', 'scambio', 'dichiarazione', 'regolamento', 'schede']) delete body[k];
      const r = await saveTextContest(body as unknown as TextContest);
      if (c.id && voceAttiva(view, 'mySect')) writeSection(browserStorage(), r.id, mySect);
      toast(t('ctest.saved'), 'info');
      refresh();
      onSaved(r.id);
    } catch (e) {
      toast(e instanceof ContestError && e.code === 'contest_readonly' ? t('ctest.readonly', { why: e.detail ?? e.code }) : t('ctest.error', { why: why(e) }), 'error');
    } finally {
      setBusy(false);
    }
  };
  // «Copia nella mia cartella»: la copia sostituisce quella inclusa e si modifica nei suoi file
  const copy = async () => {
    setBusy(true);
    try {
      const r = await copyTextContest(c.id);
      toast(t('ctest.copied', { name: r.id }), 'ok');
      refresh();
      onSaved(r.id);
    } catch (e) {
      toast(t('ctest.error', { why: why(e) }), 'error');
    } finally {
      setBusy(false);
    }
  };
  const files = (list: readonly string[], current: string) => (current && !list.includes(current) ? [current, ...list] : list);
  const stored = t('ctest.tip.stored');
  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={c.id ? t('ctest.editTitle', { name: c.name }) : t('ctest.newTitle')} onKeyDown={(e) => (e.key === 'Escape' ? (e.stopPropagation(), onClose()) : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <div className="digi-dialog ctest-dialog ctest-edit">
        <header>
          <strong>{c.id ? t('ctest.editTitle', { name: contest.name }) : t('ctest.newTitle')}</strong>
        </header>
        {readonly ? (
          <div className="ctest-readonly" role="note">
            <span>{t('ctest.ro.text')}</span>
            <button type="button" className="glass sstv-btn" disabled={busy} title={t('ctest.ro.copyTip')} onClick={() => void copy()}>
              {t('ctest.ro.copy')}
            </button>
          </div>
        ) : null}
        <div className="ctest-groups">
          {readonly && regolamento && regolamento.trim() !== '' ? (
            <Group title={t('ctest.g.rules')}>
              <div className="ctest-rules" data-testid="ctest-rules">
                {blocchiRegolamento(regolamento).map((b, i) =>
                  b.tipo === 'titolo' ? (
                    <strong key={i}>{b.testo}</strong>
                  ) : b.tipo === 'elenco' ? (
                    <ul key={i}>
                      {b.voci.map((v, j) => (
                        <li key={j}>{v}</li>
                      ))}
                    </ul>
                  ) : (
                    <p key={i}>{b.testo}</p>
                  ),
                )}
              </div>
            </Group>
          ) : null}
          <Group title={t('ctest.g.general')}>
            <label className={cls('name')} title={tipOf('name')}>
              <span>{t('ctest.f.name')}</span>
              <input type="text" value={c.name} disabled={lock('name')} onChange={(e) => set({ name: e.target.value })} autoFocus={!readonly} />
            </label>
            <label className={cls('notes')} title={tipOf('notes')}>
              <span>{t('ctest.f.notes')}</span>
              <input type="text" value={c.notes} disabled={lock('notes')} onChange={(e) => set({ notes: e.target.value })} />
            </label>
            <label className={cls('start')} title={tipOf('start', t('ctest.tip.dates'))}>
              <span>{t('ctest.f.start')}</span>
              <input type="datetime-local" step={1} value={toLocalInput(c.start)} disabled={lock('start')} onChange={(e) => set({ start: fromLocalInput(e.target.value) })} />
            </label>
            <label className={cls('end')} title={tipOf('end', t('ctest.tip.dates'))}>
              <span>{t('ctest.f.end')}</span>
              <input type="datetime-local" step={1} value={toLocalInput(c.end)} disabled={lock('end')} onChange={(e) => set({ end: fromLocalInput(e.target.value) })} />
            </label>
            {datesCheck !== 'ok' ? (
              <p className="ctest-warn" role="alert">
                {t(datesCheck === 'order' ? 'ctest.err.order' : 'ctest.err.date')}
              </p>
            ) : null}
            {proposed && c.start === proposed.start && c.end === proposed.end ? <p className="ctest-hint">{t('ctest.proposed', { year: new Date().getUTCFullYear() })}</p> : null}
            {proposals.length > 0 ? (
              <div className="ctest-propose">
                {proposals.map((p) => (
                  <button key={p.tag} type="button" className="glass sstv-btn" title={t('ctest.tip.propose')} onClick={() => set({ start: p.start, end: p.end })}>
                    {t('ctest.propose', { what: `${p.tag ? `${p.tag} ` : ''}${day(p.start)} – ${day(p.end)}` })}
                  </button>
                ))}
              </div>
            ) : null}
            {int('work_time', t('ctest.f.workTime'), t('ctest.tip.workTime'))}
          </Group>
          <Group title={t('ctest.g.serial')}>
            <label className={cls('nrs')} title={tipOf('nrs', t('ctest.tip.nrs'))}>
              <span>{t('ctest.f.nrs')}</span>
              <input type="text" value={c.nrs} disabled={lock('nrs')} onChange={(e) => set({ nrs: e.target.value })} />
            </label>
            <label className={cls('mySect')} title={tipOf('mySect', t('ctest.tip.mySect'))}>
              <span>{t('ctest.f.mySect')}</span>
              <input type="text" value={mySect} disabled={lock('mySect')} maxLength={EXTRA_MAX} autoComplete="off" spellCheck={false} onChange={(e) => setMySect(cleanExtra(e.target.value))} />
            </label>
            {check('auto_inc', t('ctest.f.autoInc'), t('ctest.tip.autoInc'))}
            <p className="ctest-hint">{c.auto_inc ? t('ctest.nrsAuto', { n: String(startSerial(c.nrs)).padStart(3, '0') }) : c.nrs.trim() !== '' ? t('ctest.nrsFixed', { nrs: c.nrs.trim() }) : t('ctest.nrsFixedNone')}</p>
          </Group>
          <Group title={t('ctest.g.validity')}>
            {check('valid_cw', t('ctest.f.validCw'), t('ctest.tip.valid'))}
            {check('valid_phone', t('ctest.f.validPhone'), t('ctest.tip.valid'))}
            {check('valid_digital', t('ctest.f.validDigital'), t('ctest.tip.valid'))}
            {int('valid_contacts', t('ctest.f.validContacts'), stored)}
          </Group>
          <Group title={t('ctest.g.points')}>
            {int('points_my_dxcc', t('ctest.f.pointsMyDxcc'), t('ctest.tip.points'))}
            {int('points_my_cont', t('ctest.f.pointsMyCont'), t('ctest.tip.points'))}
            {int('points_dx', t('ctest.f.pointsDx'), t('ctest.tip.points'))}
          </Group>
          <Group title={t('ctest.g.mult')}>
            {multSelect('multipliers', t('ctest.f.multipliers'))}
            {multSelect('multipliers_option', t('ctest.f.multOption'))}
            {check('band_mult', t('ctest.f.bandMult'), t('ctest.tip.bandMult'))}
            <label className={cls('qtc')} title={tipOf('qtc', t('ctest.tip.qtc'))}>
              <span>{t('ctest.f.qtc')}</span>
              <select value={c.qtc} disabled={lock('qtc')} onChange={(e) => set({ qtc: num(e.target.value) })}>
                <option value={0}>{t('ctest.qtc0')}</option>
                <option value={1}>{t('ctest.qtc1')}</option>
                <option value={2}>{t('ctest.qtc2')}</option>
              </select>
            </label>
            <div className="ctest-bands">
              {c.bands.map((b, i) => (
                <div key={b.band} className={cls(`band:${b.band}`, 'ctest-band')} title={tipOf(`band:${b.band}`, t('ctest.tip.bandMult'))}>
                  <label className="ctest-check">
                    <input type="checkbox" checked={b.on} disabled={lock(`band:${b.band}`)} onChange={(e) => set({ bands: c.bands.map((x, j) => (j === i ? { ...x, on: e.target.checked } : x)) })} />
                    <span>{t('ctest.f.band', { band: b.band })}</span>
                  </label>
                  <input type="number" aria-label={`${t('ctest.f.weight')} ${b.band}`} value={b.weight} disabled={lock(`band:${b.band}`)} onChange={(e) => set({ bands: c.bands.map((x, j) => (j === i ? { ...x, weight: num(e.target.value) } : x)) })} />
                </div>
              ))}
            </div>
          </Group>
          <Group title={t('ctest.g.macros')}>
            <label className={cls('macros')} title={tipOf('macros', t('ctest.tip.macros'))}>
              <span>{t('ctest.f.macros')}</span>
              <select value={c.macros} disabled={lock('macros')} onChange={(e) => set({ macros: e.target.value })}>
                <option value="">{t('ctest.f.macrosNone')}</option>
                {files(macroFiles, c.macros).map((f) => (
                  <option key={f} value={f}>
                    {f.replace(/^macros\//, '')}
                  </option>
                ))}
              </select>
            </label>
            {check('merge_macros', t('ctest.f.merge'), t('ctest.tip.merge'))}
          </Group>
          <Group title={t('ctest.g.grab')}>
            {check('grab_number', t('ctest.f.grabNumber'), t('ctest.tip.grabNumber'))}
            <label className={cls('grab_number_type')} title={tipOf('grab_number_type', t('ctest.tip.grabType'))}>
              <span>{t('ctest.f.grabNumberType')}</span>
              <select value={c.grab_number_type} disabled={lock('grab_number_type')} onChange={(e) => set({ grab_number_type: num(e.target.value) })}>
                {[-1, 0, 1, 2].map((n) => (
                  <option key={n} value={n}>
                    {t(n === -1 ? 'ctest.gt.none' : n === 0 ? 'ctest.gt.serial' : n === 1 ? 'ctest.gt.itu' : 'ctest.gt.cq')}
                  </option>
                ))}
                {![-1, 0, 1, 2].includes(c.grab_number_type) ? <option value={c.grab_number_type}>{c.grab_number_type}</option> : null}
              </select>
            </label>
            {check('grab_time', t('ctest.f.grabTime'), stored)}
            {check('grab_state', t('ctest.f.grabState'), t('ctest.tip.grabState'))}
            <label className={cls('domain_file')} title={tipOf('domain_file', t('ctest.tip.domain'))}>
              <span>{t('ctest.f.domain')}</span>
              <select value={c.domain_file} disabled={lock('domain_file')} onChange={(e) => set({ domain_file: e.target.value })}>
                <option value="">{t('ctest.f.macrosNone')}</option>
                {files(domainFiles, c.domain_file).map((f) => (
                  <option key={f} value={f}>
                    {f.replace(/^dom\//, '')}
                  </option>
                ))}
              </select>
            </label>
            {int('grab_domain_to', t('ctest.f.grabDomainTo'), stored)}
          </Group>
          <Group title={t('ctest.g.suggest')}>
            {check('suggest_cq_zone', t('ctest.f.sCq'), t('ctest.tip.suggest'))}
            {check('suggest_itu_zone', t('ctest.f.sItu'), t('ctest.tip.suggest'))}
            {check('suggest_domain', t('ctest.f.sDomain'), stored)}
            {check('suggest_dxcc_pfx', t('ctest.f.sDxcc'), t('ctest.tip.suggest'))}
          </Group>
          <Group title={t('ctest.g.dupes')}>
            {check('dupe_no_band', t('ctest.f.noBand'), t('ctest.tip.noBand'))}
            {check('dupe_no_mode', t('ctest.f.noMode'), t('ctest.tip.noMode'))}
            {int('log_bar_mode', t('ctest.f.logBar'), stored)}
          </Group>
        </div>
        <footer>
          <span className="rtty-spacer" />
          <button type="button" className="glass sstv-btn" disabled={busy || c.name.trim() === '' || datesCheck !== 'ok'} onClick={() => void save()}>
            {t('confirm.ok')}
          </button>
          <button type="button" className="glass sstv-btn" onClick={onClose}>
            {t('confirm.cancel')}
          </button>
        </footer>
      </div>
    </div>
  );
}
