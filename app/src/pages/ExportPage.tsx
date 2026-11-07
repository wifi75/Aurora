import { flagCode } from '../lib/flags';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { addLogQso, ApiError, deleteLogQsos, getLogQsos, logExportUrl, saveLogAdif, type LogQso, type ManualQso } from '../api/client';
import { askConfirm } from '../components/ConfirmHost';
import { useT } from '../i18n';
import { BAND_NAMES } from '../lib/format';
import { toast } from '../lib/toast';
import { Help } from '../settings/layout';
import { coreText } from '../lib/coreText';

/** Righe disegnate per volta nella tabella del log. */
const PAGE_ROWS = 300;

/** Le bande nell'ordine della radio (160m prima di 80m), quelle sconosciute in fondo. */
function bandOrder(a: string, b: string): number {
  const ia = BAND_NAMES.indexOf(a);
  const ib = BAND_NAMES.indexOf(b);
  return (ia < 0 ? 999 : ia) - (ib < 0 ? 999 : ib) || a.localeCompare(b);
}

export interface ExportFilter {
  call: string;
  from: string;
  to: string;
  band: string;
  mode: string;
}

const NO_FILTER: ExportFilter = { call: '', from: '', to: '', band: '', mode: '' };

/** I QSO che passano i filtri: nominativo (parte del nome), date UTC `AAAA-MM-GG` comprese, banda e modo. */
export function filterQsos(qsos: readonly LogQso[], f: ExportFilter): LogQso[] {
  const call = f.call.trim().toUpperCase();
  return qsos.filter(
    (q) => (!call || q.call.toUpperCase().includes(call)) && (!f.from || q.date >= f.from) && (!f.to || q.date <= f.to) && (!f.band || q.band === f.band) && (!f.mode || q.mode === f.mode),
  );
}

/** Pagina di esportazione del log interno: elenco dei QSO con selezione, filtri e «Salva ADIF…» (finestra di Windows) o il download. */
export function ExportPage() {
  const t = useT();
  const log = useQuery({ queryKey: ['log-qsos'], queryFn: getLogQsos, retry: false, refetchOnWindowFocus: false });
  const [filter, setFilter] = useState<ExportFilter>(NO_FILTER);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [busy, setBusy] = useState(false);
  const [adding, setAdding] = useState(false);
  const client = useQueryClient();
  const reload = () => client.invalidateQueries({ queryKey: ['log-qsos'] });
  const remove = async (all: boolean) => {
    const n = all ? (log.data?.qsos.length ?? 0) : selected.size;
    if (n === 0) return;
    if (!(await askConfirm(t(all ? 'exp.clearAsk' : 'exp.deleteAsk', { n })))) return;
    setBusy(true);
    try {
      const r = await deleteLogQsos(all ? { all: true } : { ids: [...selected] });
      setSelected(new Set());
      toast(t('exp.deleted', { n: r.deleted }), 'ok');
      await reload();
    } catch (error) {
      toast(error instanceof ApiError && error.code === 'local_only' ? t('exp.localOnly') : t('exp.deleteFailed'), 'error');
    } finally {
      setBusy(false);
    }
  };
  // la prima volta che arriva il log sono scelti tutti: il caso piu' comune e' esportare l'intero log
  const primed = useRef(false);
  useEffect(() => {
    if (!log.data || primed.current) return;
    primed.current = true;
    setSelected(new Set(log.data.qsos.map((q) => q.id)));
  }, [log.data]);

  const qsos = useMemo(() => log.data?.qsos ?? [], [log.data]);
  const bands = useMemo(() => [...new Set(qsos.map((q) => q.band))].sort(bandOrder), [qsos]);
  const modes = useMemo(() => [...new Set(qsos.map((q) => q.mode))].sort(), [qsos]);
  const shown = useMemo(() => filterQsos(qsos, filter), [qsos, filter]);
  // con log grandi (decine di migliaia di QSO) disegnare tutte le righe bloccava la pagina: se ne disegnano a blocchi, gli altri
  // compaiono scorrendo in fondo o con «Mostra altri»; filtri, selezione ed esportazione valgono sempre per tutti
  const [limit, setLimit] = useState(PAGE_ROWS);
  useEffect(() => setLimit(PAGE_ROWS), [filter]);
  const shownSelected = shown.reduce((n, q) => n + (selected.has(q.id) ? 1 : 0), 0);
  const hiddenSelected = selected.size - shownSelected;
  const everything = qsos.length > 0 && selected.size === qsos.length;
  const filtered = filter.call !== '' || filter.from !== '' || filter.to !== '' || filter.band !== '' || filter.mode !== '';

  const headBox = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (headBox.current) headBox.current.indeterminate = shownSelected > 0 && shownSelected < shown.length;
  }, [shownSelected, shown.length]);

  const set = (patch: Partial<ExportFilter>) => setFilter((f) => ({ ...f, ...patch }));
  const toggle = (id: number) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const selectShown = (on: boolean) =>
    setSelected((s) => {
      const next = new Set(s);
      for (const q of shown) {
        if (on) next.add(q.id);
        else next.delete(q.id);
      }
      return next;
    });

  const save = async () => {
    if (selected.size === 0) {
      toast(t('exp.noneSelected'), 'info');
      return;
    }
    setBusy(true);
    try {
      const r = await saveLogAdif(everything ? { all: true } : { ids: [...selected] });
      if (r.cancelled) toast(t('exp.cancelled'), 'info');
      else toast(t('exp.saved', { n: r.count, path: r.path }), 'ok');
    } catch (error) {
      if (error instanceof ApiError && error.code === 'save_busy') toast(t('exp.busy'), 'info');
      else if (error instanceof ApiError && (error.code === 'no_selection' || error.code === 'empty_export')) toast(t('exp.noneSelected'), 'info');
      else if (error instanceof ApiError && error.detail) toast(t('exp.failed', { why: coreText(error.detail, t) }), 'error');
      else toast(t('exp.error'), 'error');
    } finally {
      setBusy(false);
    }
  };

  const today = new Date().toISOString().slice(0, 10).replace(/-/g, '');

  let body;
  if (log.isError) body = <p className="notice">{t('exp.unavailable')}</p>;
  else if (log.isPending) body = <p className="muted">{t('exp.loading')}</p>;
  else if (qsos.length === 0) body = <p className="notice">{t('exp.empty')}</p>;
  else if (shown.length === 0) body = <p className="muted">{t('exp.noMatch')}</p>;
  else
    body = (
      <div className="export-table-wrap">
        <table className="export-table">
          <thead>
            <tr>
              <th className="export-check">
                <input
                  ref={headBox}
                  type="checkbox"
                  aria-label={t('exp.selectShown')}
                  title={t('tip.ui.selectShown')}
                  checked={shown.length > 0 && shownSelected === shown.length}
                  onChange={(e) => selectShown(e.target.checked)}
                />
              </th>
              <th>{t('exp.col.date')}</th>
              <th>{t('exp.col.time')}</th>
              <th>{t('exp.col.call')}</th>
              <th>{t('exp.col.band')}</th>
              <th>{t('exp.col.mode')}</th>
              <th className="num" title={t('tip.ui.colSent')}>{t('exp.col.sent')}</th>
              <th className="num">
                {t('exp.col.rcvd')}
                <Help tip={t('exp.rstTip')} />
              </th>
              <th className="export-grid" title={t('tip.ui.colGrid')}>{t('exp.col.grid')}</th>
              <th>{t('exp.col.country')}</th>
            </tr>
          </thead>
          <tbody>
            {shown.slice(0, limit).map((q) => {
              const on = selected.has(q.id);
              return (
                <tr key={q.id} className={on ? 'export-row export-row--on' : 'export-row'} onClick={() => toggle(q.id)}>
                  <td className="export-check">
                    <input type="checkbox" title={t('tip.ui.selectRow')} aria-label={t('exp.selectRow', { call: q.call })} checked={on} onChange={() => toggle(q.id)} onClick={(e) => e.stopPropagation()} />
                  </td>
                  <td className="data">{q.date}</td>
                  <td className="data">{q.time.slice(0, 5)}</td>
                  <td className="data export-call">{q.call}</td>
                  <td className="data">{q.band}</td>
                  <td className="data">{q.mode}</td>
                  <td className="data num">{q.rst_sent}</td>
                  <td className="data num">{q.rst_rcvd}</td>
                  <td className="data export-grid">{q.grid}</td>
                  <td>
                    <span className="export-country">
                      {flagCode(q.country) ? <span className={`fi fi-${flagCode(q.country)} export-flag`} aria-hidden="true" /> : null}
                      {q.country}
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {shown.length > limit ? (
          <div className="export-more">
            <span className="muted">{t('exp.moreShown', { shown: limit, total: shown.length })}</span>
            <button type="button" className="glass" onClick={() => setLimit((n) => n + PAGE_ROWS * 2)}>
              {t('exp.more')}
            </button>
          </div>
        ) : null}
      </div>
    );

  const hasLog = !!log.data && qsos.length > 0;
  return (
    <div className="export-page">
      <div className="export-head">
        <h1 className="page-title">{t('exp.title')}</h1>
        <button type="button" className="glass glass--on" onClick={() => setAdding((v) => !v)} aria-expanded={adding}>
          {t('exp.add')}
        </button>
        <a className="glass export-back" href="#/">
          {t('exp.back')}
        </a>
      </div>
      <p className="muted export-help">{t('exp.help')}</p>
      {adding ? (
        <AddQso
          onDone={() => {
            setAdding(false);
            // il QSO passa dal motore (log, file ADIF, programmi collegati): si rilegge il log dopo un attimo
            setTimeout(() => void reload(), 800);
          }}
          onCancel={() => setAdding(false)}
        />
      ) : null}

      {hasLog ? (
        <>
          <div className="export-bar">
            <input type="search" title={t('tip.ui.search')} aria-label={t('exp.search')} placeholder={t('exp.search')} value={filter.call} onChange={(e) => set({ call: e.target.value })} />
            <label className="map-select">
              {t('exp.from')}
              <input type="date" title={t('tip.ui.dateFrom')} value={filter.from} max={filter.to || undefined} onChange={(e) => set({ from: e.target.value })} />
            </label>
            <label className="map-select">
              {t('exp.to')}
              <input type="date" value={filter.to} min={filter.from || undefined} onChange={(e) => set({ to: e.target.value })} />
              <Help tip={t('exp.datesTip')} />
            </label>
            <label className="map-select">
              {t('exp.band')}
              <select title={t('tip.ui.band')} value={filter.band} onChange={(e) => set({ band: e.target.value })}>
                <option value="">{t('exp.any')}</option>
                {bands.map((b) => (
                  <option key={b} value={b}>
                    {b}
                  </option>
                ))}
              </select>
            </label>
            <label className="map-select">
              {t('exp.mode')}
              <select title={t('tip.ui.mode')} value={filter.mode} onChange={(e) => set({ mode: e.target.value })}>
                <option value="">{t('exp.anyMode')}</option>
                {modes.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
            </label>
            {filtered ? (
              <button type="button" className="glass" title={t('tip.ui.clearFilters')} onClick={() => setFilter(NO_FILTER)}>
                {t('exp.clear')}
              </button>
            ) : null}
          </div>

          <div className="export-actions">
            <button type="button" className="glass" title={t('exp.allTip')} onClick={() => selectShown(true)} disabled={shown.length === 0}>
              {t('exp.all')}
            </button>
            <button type="button" className="glass" title={t('exp.noneTip')} onClick={() => selectShown(false)} disabled={shownSelected === 0}>
              {t('exp.none')}
            </button>
            <span className="muted export-count" role="status">
              {t('exp.count', { selected: selected.size, shown: shown.length, total: qsos.length })}
              {hiddenSelected > 0 ? <span className="export-hidden"> · {t('exp.hidden', { n: hiddenSelected })}</span> : null}
            </span>
            {busy ? <span className="muted export-count">{t('exp.saving')}</span> : null}
            <button type="button" className="glass export-danger" title={t('exp.deleteTip')} disabled={busy || selected.size === 0} onClick={() => void remove(false)}>
              {t('exp.delete', { n: selected.size })}
            </button>
            <button type="button" className="glass export-danger" title={t('exp.clearLogTip')} disabled={busy} onClick={() => void remove(true)}>
              {t('exp.clearLog')}
            </button>
            {log.data?.save_dialog ? (
              <button type="button" className="glass glass--on export-save" title={t('exp.saveTip')} disabled={busy || selected.size === 0} onClick={() => void save()}>
                {t('exp.save')}
              </button>
            ) : selected.size > 0 ? (
              <a
                className="glass glass--on export-save"
                href={logExportUrl(undefined, undefined, everything ? undefined : selected)}
                download={`aurora_log_${today}.adi`}
                title={t('exp.downloadTip')}
                onClick={() => toast(t('exp.downloaded', { n: selected.size }), 'ok')}
              >
                {t('exp.download')}
              </a>
            ) : (
              <button type="button" className="glass export-save" title={t('exp.downloadTip')} disabled>
                {t('exp.download')}
              </button>
            )}
          </div>
        </>
      ) : null}

      {body}
    </div>
  );
}

/** I modi del QSO scritto a mano, come li scrive l'ADIF (MODE), per tipo. */
const MODE_GROUPS: ReadonlyArray<readonly ['exp.mg.digital' | 'exp.mg.voice' | 'exp.mg.cw' | 'exp.mg.other', readonly string[]]> = [
  ['exp.mg.digital', ['FT8', 'FT4', 'JT65', 'JT9', 'JT4', 'Q65', 'MSK144', 'FST4', 'FST4W', 'WSPR', 'JS8', 'RTTY', 'PSK31', 'PSK63', 'PSK125', 'BPSK', 'QPSK', 'OLIVIA', 'CONTESTIA', 'MFSK', 'DOMINO', 'THOR', 'HELL', 'PACKET', 'PKT', 'VARA', 'ARDOP', 'PACTOR', 'SSTV', 'FAX']],
  ['exp.mg.voice', ['SSB', 'USB', 'LSB', 'AM', 'FM', 'DIGITALVOICE', 'DSTAR', 'C4FM', 'DMR', 'FREEDV', 'M17']],
  ['exp.mg.cw', ['CW']],
  ['exp.mg.other', ['ATV', 'SATELLITE']],
];

const BANDS_FOR_ADD = ['160m', '80m', '60m', '40m', '30m', '20m', '17m', '15m', '12m', '10m', '6m', '2m', '70cm'];

/** Il modulo per scrivere un QSO a mano: nominativo, data e ora UTC, banda (o frequenza), modo, rapporti, locatore, nome e commento. */
function AddQso({ onDone, onCancel }: { onDone(): void; onCancel(): void }) {
  const t = useT();
  const now = new Date();
  const [q, setQ] = useState<ManualQso>({
    call: '',
    date: now.toISOString().slice(0, 10),
    time: now.toISOString().slice(11, 16),
    band: '20m',
    freq_hz: 0,
    mode: 'FT8',
    rst_sent: '',
    rst_rcvd: '',
    grid: '',
    name: '',
    comment: '',
  });
  const [busy, setBusy] = useState(false);
  const set = (patch: Partial<ManualQso>) => setQ((v) => ({ ...v, ...patch }));
  const submit = async (e: { preventDefault(): void }) => {
    e.preventDefault();
    setBusy(true);
    try {
      await addLogQso({ ...q, call: q.call.trim().toUpperCase() });
      toast(t('exp.added', { call: q.call.trim().toUpperCase() }), 'ok');
      onDone();
    } catch (error) {
      const field = error instanceof ApiError ? error.detail : undefined;
      const label = field === 'call' ? t('exp.f.call') : field === 'mode' ? t('exp.f.mode') : field === 'band' ? t('exp.f.band') : field === 'time' ? t('exp.f.time') : undefined;
      toast(label ? t('exp.addBad', { field: label }) : error instanceof ApiError && error.code === 'local_only' ? t('exp.localOnly') : t('exp.addFailed'), 'error');
    } finally {
      setBusy(false);
    }
  };
  return (
    <form className="export-add" onSubmit={(e) => void submit(e)} aria-label={t('exp.add')}>
      <label>
        {t('exp.f.call')}
        <input required autoFocus value={q.call} onChange={(e) => set({ call: e.target.value.toUpperCase() })} maxLength={13} />
      </label>
      <label>
        {t('exp.f.date')}
        <input type="date" required value={q.date} onChange={(e) => set({ date: e.target.value })} />
      </label>
      <label>
        {t('exp.f.time')}
        <input type="time" required value={q.time} onChange={(e) => set({ time: e.target.value })} />
      </label>
      <label>
        {t('exp.f.band')}
        <select value={q.band} onChange={(e) => set({ band: e.target.value })}>
          {BANDS_FOR_ADD.map((b) => (
            <option key={b} value={b}>
              {b}
            </option>
          ))}
        </select>
      </label>
      <label>
        {t('exp.f.freq')}
        <input type="number" min={0} step={0.001} value={q.freq_hz ? q.freq_hz / 1e6 : ''} placeholder="14.074" onChange={(e) => set({ freq_hz: Math.round(Number(e.target.value) * 1e6) || 0 })} />
      </label>
      <label>
        {t('exp.f.mode')}
        <select value={q.mode} onChange={(e) => set({ mode: e.target.value })}>
          {MODE_GROUPS.map(([group, modes]) => (
            <optgroup key={group} label={t(group)}>
              {modes.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
      </label>
      <label>
        {t('exp.f.sent')}
        <input value={q.rst_sent} onChange={(e) => set({ rst_sent: e.target.value })} maxLength={8} />
      </label>
      <label>
        {t('exp.f.rcvd')}
        <input value={q.rst_rcvd} onChange={(e) => set({ rst_rcvd: e.target.value })} maxLength={8} />
      </label>
      <label>
        {t('exp.f.grid')}
        <input value={q.grid} onChange={(e) => set({ grid: e.target.value.toUpperCase() })} maxLength={8} />
      </label>
      <label>
        {t('exp.f.name')}
        <input value={q.name} onChange={(e) => set({ name: e.target.value })} maxLength={60} />
      </label>
      <label className="export-add-wide">
        {t('exp.f.comment')}
        <input value={q.comment} onChange={(e) => set({ comment: e.target.value })} maxLength={200} />
      </label>
      <div className="export-add-actions">
        <span className="muted">{t('exp.addUtc')}</span>
        <button type="button" className="glass" onClick={onCancel}>
          {t('exp.addCancel')}
        </button>
        <button type="submit" className="glass glass--on" disabled={busy || q.call.trim().length < 3}>
          {t('exp.addSave')}
        </button>
      </div>
    </form>
  );
}
