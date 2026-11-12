import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { ApiError, deleteLogQso, editLogQso, type LogQso, type LogQsoPatch } from '../api/client';
import { useT } from '../i18n';
import { toast } from '../lib/toast';
import { askConfirm } from './ConfirmHost';

/** I modi di un QSO scritto o corretto a mano, come li scrive l'ADIF (MODE), per tipo. */
export const MODE_GROUPS: ReadonlyArray<readonly ['exp.mg.digital' | 'exp.mg.voice' | 'exp.mg.cw' | 'exp.mg.other', readonly string[]]> = [
  ['exp.mg.digital', ['FT8', 'FT4', 'JT65', 'JT9', 'JT4', 'Q65', 'MSK144', 'FST4', 'FST4W', 'WSPR', 'JS8', 'RTTY', 'PSK31', 'PSK63', 'PSK125', 'BPSK', 'QPSK', 'OLIVIA', 'CONTESTIA', 'MFSK', 'DOMINO', 'THOR', 'HELL', 'PACKET', 'PKT', 'VARA', 'ARDOP', 'PACTOR', 'SSTV', 'FAX']],
  ['exp.mg.voice', ['SSB', 'USB', 'LSB', 'AM', 'FM', 'DIGITALVOICE', 'DSTAR', 'C4FM', 'DMR', 'FREEDV', 'M17']],
  ['exp.mg.cw', ['CW']],
  ['exp.mg.other', ['ATV', 'SATELLITE']],
];

export const BANDS_FOR_ADD = ['160m', '80m', '60m', '40m', '30m', '20m', '17m', '15m', '12m', '10m', '6m', '2m', '70cm'];

/** I valori della finestra, come testo: la frequenza in MHz, la data `AAAA-MM-GG`, l'ora `HH:MM:SS` (UTC). */
export interface LogEditValues {
  call: string;
  date: string;
  time: string;
  band: string;
  freq: string;
  mode: string;
  rst_sent: string;
  rst_rcvd: string;
  grid: string;
  name: string;
  qth: string;
  comment: string;
}

export type LogEditError = 'call' | 'time' | 'grid' | 'freq';

function valuesOf(q: LogQso): LogEditValues {
  return {
    call: q.call,
    date: q.date,
    time: q.time,
    band: q.band,
    freq: q.freq_hz > 0 ? (q.freq_hz / 1e6).toFixed(6) : '',
    mode: q.mode,
    rst_sent: q.rst_sent,
    rst_rcvd: q.rst_rcvd,
    grid: q.grid,
    name: q.name ?? '',
    qth: q.qth ?? '',
    comment: q.comment ?? '',
  };
}

/** La frequenza scritta in MHz, in Hz (vuota = 0; `null` se non e' un numero). */
function freqHz(text: string): number | null {
  const t = text.trim().replace(',', '.');
  if (t === '') return 0;
  const mhz = Number(t);
  return Number.isFinite(mhz) && mhz >= 0 && mhz < 1e6 ? Math.round(mhz * 1e6) : null;
}

/** Vero se data (`AAAA-MM-GG`) e ora (`HH:MM` o `HH:MM:SS`) esistono davvero (il 30 febbraio no). */
function validDateTime(date: string, time: string): boolean {
  const d = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date.trim());
  const h = /^(\d{2}):(\d{2})(?::(\d{2}))?$/.exec(time.trim());
  if (!d || !h) return false;
  const [y, m, g] = [Number(d[1]), Number(d[2]), Number(d[3])];
  if (y < 1970 || Number(h[1]) > 23 || Number(h[2]) > 59 || Number(h[3] ?? 0) > 59) return false;
  const real = new Date(Date.UTC(y, m - 1, g));
  return real.getUTCFullYear() === y && real.getUTCMonth() === m - 1 && real.getUTCDate() === g;
}

/** I campi che non vanno: nominativo (3-13 lettere, cifre o `/`), data e ora vere, locatore vuoto o di 4, 6, 8 caratteri, frequenza numerica. */
export function logEditErrors(v: LogEditValues): LogEditError[] {
  const bad: LogEditError[] = [];
  if (!/^[A-Z0-9/]{3,13}$/.test(v.call.trim().toUpperCase())) bad.push('call');
  if (!validDateTime(v.date, v.time)) bad.push('time');
  if (v.grid.trim() !== '' && !/^[A-R]{2}\d{2}([A-X]{2}(\d{2})?)?$/i.test(v.grid.trim())) bad.push('grid');
  if (freqHz(v.freq) === null) bad.push('freq');
  return bad;
}

/** I soli campi cambiati rispetto al QSO com'era (niente = oggetto vuoto). */
export function logEditPatch(q: LogQso, v: LogEditValues): LogQsoPatch {
  const before = valuesOf(q);
  const patch: LogQsoPatch = {};
  const call = v.call.trim().toUpperCase();
  if (call !== before.call) patch.call = call;
  if (v.date !== before.date || v.time !== before.time) {
    patch.date = v.date;
    patch.time = v.time;
  }
  if (v.band !== before.band) patch.band = v.band;
  if (v.mode !== before.mode) patch.mode = v.mode;
  const hz = freqHz(v.freq) ?? 0;
  if (hz !== q.freq_hz) patch.freq_hz = hz;
  for (const k of ['rst_sent', 'rst_rcvd', 'name', 'qth', 'comment'] as const) {
    if (v[k].trim() !== before[k]) patch[k] = v[k].trim();
  }
  const grid = v.grid.trim().toUpperCase();
  if (grid !== before.grid.toUpperCase()) patch.grid = grid;
  return patch;
}

const ERROR_TEXT = { call: 'exp.edit.badCall', time: 'exp.edit.badTime', grid: 'exp.edit.badGrid', freq: 'exp.edit.badFreq' } as const;

/**
 * La finestra per correggere o cancellare un QSO del log interno di Aurora. «Salva» manda i soli campi cambiati, «Elimina» chiede
 * conferma. `onClose` dice com'e' finita: `saved`, `deleted` oppure `null` (annullato).
 */
export function LogEditDialog({ qso, onClose }: { qso: LogQso; onClose(done: 'saved' | 'deleted' | null): void }) {
  const t = useT();
  const client = useQueryClient();
  const [v, setV] = useState<LogEditValues>(() => valuesOf(qso));
  const [busy, setBusy] = useState(false);
  // gli errori si mostrano dopo il primo «Salva», poi seguono cio' che si scrive
  const [checked, setChecked] = useState(false);
  const asking = useRef(false);
  const set = (patch: Partial<LogEditValues>) => setV((old) => ({ ...old, ...patch }));
  const errors = checked ? logEditErrors(v) : [];

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // con la conferma dell'eliminazione aperta Esc chiude solo quella
      if (e.key === 'Escape' && !asking.current) onClose(null);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  /** Il log e' cambiato: si rileggono la pagina Log e il pannello (i «gia' lavorati» li ricalcola Aurora). */
  const refresh = () => {
    void client.invalidateQueries({ queryKey: ['log-qsos'] });
    void client.invalidateQueries({ queryKey: ['log4om'] });
  };

  const failed = (error: unknown, fallback: string) => {
    if (error instanceof ApiError && error.code === 'local_only') toast(t('exp.localOnly'), 'error');
    else if (error instanceof ApiError && error.code === 'qso_not_found') {
      toast(t('exp.edit.gone'), 'error');
      refresh();
      onClose(null);
    } else if (error instanceof ApiError && error.code === 'bad_field' && error.detail && error.detail in ERROR_TEXT) toast(t(ERROR_TEXT[error.detail as LogEditError]), 'error');
    else toast(fallback, 'error');
  };

  const save = async (e: { preventDefault(): void }) => {
    e.preventDefault();
    setChecked(true);
    if (logEditErrors(v).length > 0) return;
    const patch = logEditPatch(qso, v);
    if (Object.keys(patch).length === 0) {
      onClose(null);
      return;
    }
    setBusy(true);
    try {
      const fixed = await editLogQso(qso.id, patch);
      toast(t('exp.edit.saved', { call: fixed.call }), 'ok');
      refresh();
      onClose('saved');
    } catch (error) {
      failed(error, t('exp.edit.failed'));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    asking.current = true;
    const yes = await askConfirm(t('exp.edit.deleteAsk', { call: qso.call, date: qso.date }));
    asking.current = false;
    if (!yes) return;
    setBusy(true);
    try {
      await deleteLogQso(qso.id);
      toast(t('exp.edit.deletedOne', { call: qso.call }), 'ok');
      refresh();
      onClose('deleted');
    } catch (error) {
      failed(error, t('exp.deleteFailed'));
    } finally {
      setBusy(false);
    }
  };

  // l'avviso sta sotto la casella, dentro la sua etichetta: la casella ha il nome scritto a parte, cosi' chi usa un lettore di schermo
  // sente «Nominativo» e non il nome seguito dall'avviso
  const problem = (field: LogEditError) =>
    errors.includes(field) ? (
      <span className="log-edit-error" role="alert">
        {t(ERROR_TEXT[field])}
      </span>
    ) : null;
  const bands = BANDS_FOR_ADD.includes(v.band) ? BANDS_FOR_ADD : [v.band, ...BANDS_FOR_ADD];
  const knownMode = MODE_GROUPS.some(([, modes]) => modes.includes(v.mode));

  return createPortal(
    <div className="confirm-overlay log-edit-overlay" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && onClose(null)}>
      <form className="confirm-card log-edit-card" role="dialog" aria-modal="true" aria-labelledby="log-edit-title" noValidate onSubmit={(e) => void save(e)}>
        <h2 id="log-edit-title" className="log-edit-title">
          {t('exp.edit.title')}
        </h2>
        <div className="log-edit-fields">
          <label>
            {t('exp.f.call')}
            <input autoFocus aria-label={t('exp.f.call')} value={v.call} aria-invalid={errors.includes('call')} onChange={(e) => set({ call: e.target.value.toUpperCase() })} maxLength={13} />
            {problem('call')}
          </label>
          <label>
            {t('exp.f.date')}
            <input type="date" value={v.date} aria-invalid={errors.includes('time')} onChange={(e) => set({ date: e.target.value })} />
          </label>
          <label>
            {t('exp.f.time')}
            <input type="time" step={1} aria-label={t('exp.f.time')} value={v.time} aria-invalid={errors.includes('time')} onChange={(e) => set({ time: e.target.value })} />
            {problem('time')}
          </label>
          <label>
            {t('exp.f.band')}
            <select value={v.band} onChange={(e) => set({ band: e.target.value })}>
              {bands.map((b) => (
                <option key={b} value={b}>
                  {b}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t('exp.f.freq')}
            <input inputMode="decimal" aria-label={t('exp.f.freq')} value={v.freq} placeholder="14.074" aria-invalid={errors.includes('freq')} onChange={(e) => set({ freq: e.target.value })} maxLength={12} />
            {problem('freq')}
          </label>
          <label>
            {t('exp.f.mode')}
            <select value={v.mode} onChange={(e) => set({ mode: e.target.value })}>
              {knownMode ? null : <option value={v.mode}>{v.mode}</option>}
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
            <input value={v.rst_sent} onChange={(e) => set({ rst_sent: e.target.value })} maxLength={8} />
          </label>
          <label>
            {t('exp.f.rcvd')}
            <input value={v.rst_rcvd} onChange={(e) => set({ rst_rcvd: e.target.value })} maxLength={8} />
          </label>
          <label>
            {t('exp.f.grid')}
            <input aria-label={t('exp.f.grid')} value={v.grid} aria-invalid={errors.includes('grid')} onChange={(e) => set({ grid: e.target.value.toUpperCase() })} maxLength={8} />
            {problem('grid')}
          </label>
          <label>
            {t('exp.f.name')}
            <input value={v.name} onChange={(e) => set({ name: e.target.value })} maxLength={60} />
          </label>
          <label>
            {t('exp.f.qth')}
            <input value={v.qth} onChange={(e) => set({ qth: e.target.value })} maxLength={60} />
          </label>
          <label className="log-edit-wide">
            {t('exp.f.comment')}
            <input value={v.comment} onChange={(e) => set({ comment: e.target.value })} maxLength={200} />
          </label>
        </div>
        <p className="muted log-edit-note">
          {t('exp.addUtc')} {t('exp.edit.note')}
        </p>
        <div className="log-edit-actions">
          <button type="button" className="glass export-danger" title={t('exp.edit.deleteTip')} disabled={busy} onClick={() => void remove()}>
            {t('exp.edit.delete')}
          </button>
          <button type="button" className="glass" disabled={busy} onClick={() => onClose(null)}>
            {t('exp.edit.cancel')}
          </button>
          <button type="submit" className="glass glass--on" title={t('exp.edit.saveTip')} disabled={busy}>
            {t('exp.edit.save')}
          </button>
        </div>
      </form>
    </div>,
    document.body,
  );
}
