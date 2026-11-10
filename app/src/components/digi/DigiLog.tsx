import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { getLogQsos } from '../../api/client';
import { useT } from '../../i18n';

export interface Qso {
  call: string;
  name: string;
  qth: string;
  wwloc: string;
  notes: string;
}

interface Props {
  qso: Qso;
  rsts: string;
  rstr: string;
  modeName: string;
  /** Frequenza della radio in Hz (0 se non nota). */
  dialHz: number;
  onQso(patch: Partial<Qso>): void;
  onRst(patch: { rsts?: string; rstr?: string }): void;
  onSave(): void;
  onClear(): void;
}

const mhz = (hz: number) => (hz > 0 ? (hz / 1_000_000).toFixed(6).replace(/0+$/, '').replace(/\.$/, '') : '');

/** Il log di lato: la riga del QSO in corso (si scrive qui) e sotto i QSO fatti, piu' recenti per primi. */
export function DigiLog({ qso, rsts, rstr, modeName, dialHz, onQso, onRst, onSave, onClear }: Props) {
  const t = useT();
  const [filter, setFilter] = useState('');
  const log = useQuery({ queryKey: ['log-qsos'], queryFn: getLogQsos, retry: false, refetchInterval: 20_000 });
  const rows = useMemo(() => {
    const all = [...(log.data?.qsos ?? [])].reverse();
    const f = filter.trim().toUpperCase();
    return (f === '' ? all : all.filter((q) => q.call.includes(f))).slice(0, 80);
  }, [log.data, filter]);
  const now = new Date();

  return (
    <aside className="digi-log" aria-label={t('digi.log.title')}>
      <div className="digi-log-tools">
        <strong>{t('digi.log.title')}</strong>
        <button type="button" className="digi-tool" title={t('digi.log.save')} aria-label={t('digi.log.save')} onClick={onSave}>
          💾
        </button>
        <button type="button" className="digi-tool digi-tool--red" title={t('digi.log.clear')} aria-label={t('digi.log.clear')} onClick={onClear}>
          ✕
        </button>
        <input className="digi-log-search" placeholder={t('digi.log.search')} aria-label={t('digi.log.search')} value={filter} onChange={(e) => setFilter(e.target.value)} />
      </div>

      <div className="digi-log-current" role="group" aria-label={t('digi.log.current')}>
        <label>
          <span>{t('digi.log.call')}</span>
          <input value={qso.call} maxLength={16} onChange={(e) => onQso({ call: e.target.value.toUpperCase() })} className="digi-call" />
        </label>
        <label>
          <span>{t('digi.log.name')}</span>
          <input value={qso.name} maxLength={30} onChange={(e) => onQso({ name: e.target.value })} />
        </label>
        <label>
          <span>{t('digi.log.qth')}</span>
          <input value={qso.qth} maxLength={40} onChange={(e) => onQso({ qth: e.target.value })} />
        </label>
        <label className="digi-rst">
          <span>{t('digi.f.rsts')}</span>
          <input value={rsts} maxLength={4} onChange={(e) => onRst({ rsts: e.target.value })} />
        </label>
        <label className="digi-rst">
          <span>{t('digi.f.rstr')}</span>
          <input value={rstr} maxLength={4} onChange={(e) => onRst({ rstr: e.target.value })} />
        </label>
        <label className="digi-wide">
          <span>{t('digi.f.notes')}</span>
          <input value={qso.notes} maxLength={80} onChange={(e) => onQso({ notes: e.target.value })} />
        </label>
        <p className="digi-log-now">
          {modeName} · {mhz(dialHz) || '—'} MHz · {now.toISOString().slice(0, 10)} {now.toISOString().slice(11, 16)}z
        </p>
      </div>

      <div className="digi-log-table">
        <table>
          <thead>
            <tr>
              <th>{t('digi.log.mode')}</th>
              <th>{t('digi.log.freq')}</th>
              <th>{t('digi.log.date')}</th>
              <th>{t('digi.log.utc')}</th>
              <th>{t('digi.log.call')}</th>
              <th>{t('digi.log.country')}</th>
              <th>{t('digi.log.rst')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((q) => (
              <tr key={q.id}>
                <td>{q.mode}</td>
                <td>{mhz(q.freq_hz)}</td>
                <td>{q.date}</td>
                <td>{q.time.slice(0, 5)}</td>
                <td>
                  <b>{q.call}</b>
                </td>
                <td>{q.country}</td>
                <td>{`${q.rst_sent}/${q.rst_rcvd}`}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {rows.length === 0 ? <p className="digi-log-empty">{t('digi.log.empty')}</p> : null}
      </div>
    </aside>
  );
}
