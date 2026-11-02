import { useState } from 'react';
import { postControl, postFox } from '../api/client';
import type { FoxSnapshot } from '../api/schemas';
import { useT } from '../i18n';
import { useFox } from '../state/fox';
import { useStation } from '../state/station';
import { FoxStation } from '../components/FoxStation';
import '../styles/fox.css';

type Caller = FoxSnapshot['callers'][number];
type SortKey = 'snr' | 'call' | 'freq_hz' | 'age';

/** Prima del primo stato dal motore: Fox spento, valori di WSJT-X. */
const EMPTY: FoxSnapshot = { enabled: false, slots: 5, more_cqs: false, cq: 'CQ', auto_queue: false, ultra: false, callers: [], queue: [], in_progress: [], tx: [], periods: 0, logged: 0 };

const CQ_TARGETS = ['', 'DX', 'NA', 'SA', 'EU', 'AS', 'AF', 'OC'];

/**
 * La pagina Fox (bozza approvata docs/mockup-fox.html): in alto l'interruttore e le opzioni, i numeri della sessione,
 * a sinistra chi chiama (si mette in coda), a destra la coda, i QSO in corso e i messaggi dell'ultima trasmissione.
 */
export function FoxPage() {
  const t = useT();
  const fox = useFox((s) => s.fox) ?? EMPTY;
  const txEnabled = useStation((s) => s.status?.tx_enabled ?? false);
  const [sort, setSort] = useState<SortKey>('snr');
  const [filter, setFilter] = useState('');

  const options = (o: Partial<{ slots: number; more_cqs: boolean; cq: string; auto_queue: boolean; ultra: boolean }>) =>
    void postFox({ action: 'options', slots: fox.slots, more_cqs: fox.more_cqs, cq: fox.cq || 'CQ', auto_queue: fox.auto_queue, ultra: fox.ultra, ...o });
  const queued = new Set(fox.queue.map((q) => q.call));
  const callers = fox.callers
    .filter((c) => !queued.has(c.call) && c.call.toUpperCase().includes(filter.toUpperCase()))
    .sort((a, b) => order(a, b, sort));

  return (
    <section className="fox">
      <header className="fox-head">
        <h1>{t('fox.title')}</h1>
        <button type="button" className={`glass${fox.enabled ? ' glass--on' : ''}`} onClick={() => void postControl('fox', !fox.enabled)}>
          {fox.enabled ? t('fox.on') : t('fox.off')}
        </button>
        <label>
          {t('fox.slots')}
          <select value={fox.slots} onChange={(e) => options({ slots: Number(e.target.value) })}>
            {[1, 2, 3, 4, 5].map((n) => <option key={n} value={n}>{n}</option>)}
          </select>
        </label>
        <label>
          {t('fox.cq')}
          <select value={fox.cq.replace(/^CQ ?/, '')} onChange={(e) => options({ cq: e.target.value ? `CQ ${e.target.value}` : 'CQ' })}>
            {CQ_TARGETS.map((c) => <option key={c} value={c}>{c || 'CQ'}</option>)}
          </select>
        </label>
        <label><input type="checkbox" checked={fox.more_cqs} onChange={(e) => options({ more_cqs: e.target.checked })} /> {t('fox.moreCqs')}</label>
        <label title={t('tip.fox.ultra')}><input type="checkbox" checked={fox.ultra} onChange={(e) => options({ ultra: e.target.checked })} /> UltraFOX</label>
        <label><input type="checkbox" checked={fox.auto_queue} onChange={(e) => options({ auto_queue: e.target.checked })} /> {t('fox.autoQueue')}</label>
        <button type="button" className="glass" onClick={() => void postFox({ action: 'reset' })}>{t('fox.reset')}</button>
      </header>
      <p className="muted fox-dev">{t('fox.dev')}</p>
      {fox.enabled && !txEnabled ? (
        <p className="fox-hint" role="status">
          {t('fox.needTx')}{' '}
          <button type="button" className="glass glass--on chip" onClick={() => void postControl('enable_tx', true)}>
            {t('fox.enableTx')}
          </button>
        </p>
      ) : null}
      {!fox.enabled ? <p className="fox-hint" role="status">{t('fox.offHint')}</p> : null}

      <FoxStation />

      <div className="fox-stats">
        <Stat label={t('fox.logged')} value={fox.logged} />
        <Stat label={t('fox.periods')} value={fox.periods} />
        <Stat label={t('fox.callers')} value={fox.callers.length} />
        <Stat label={t('fox.queue')} value={fox.queue.length} />
      </div>

      <div className="fox-cols">
        <div className="fox-box">
          <div className="fox-box-head">
            <h2>{t('fox.callers')}</h2>
            <input type="search" placeholder={t('fox.filter')} value={filter} onChange={(e) => setFilter(e.target.value)} />
            <select aria-label={t('fox.sort')} value={sort} onChange={(e) => setSort(e.target.value as SortKey)}>
              <option value="snr">SNR</option>
              <option value="call">Call</option>
              <option value="freq_hz">Hz</option>
              <option value="age">{t('fox.age')}</option>
            </select>
            <button type="button" className="glass chip" onClick={() => callers.slice(0, 5).forEach((c) => void postFox({ action: 'select', call: c.call }))}>{t('fox.addBest')}</button>
          </div>
          <table className="fox-table data">
            <tbody>
              {callers.length === 0 ? <tr><td className="muted">{t('fox.empty')}</td></tr> : null}
              {callers.map((c) => (
                <tr key={c.call} onDoubleClick={() => void postFox({ action: 'select', call: c.call })}>
                  <td className="fox-call">{c.call}</td>
                  <td>{c.grid}</td>
                  <td>{c.snr > 0 ? `+${c.snr}` : c.snr}</td>
                  <td>{c.freq_hz}</td>
                  <td>{c.age}</td>
                  <td className="fox-act">
                    <button type="button" className="glass chip" onClick={() => void postFox({ action: 'select', call: c.call })}>{t('fox.add')}</button>
                    <button type="button" className="glass chip" onClick={() => void postFox({ action: 'select', call: c.call, top: true })}>{t('fox.top')}</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="fox-side">
          <div className="fox-box">
            <h2>{t('fox.inProgress')}</h2>
            <table className="fox-table data">
              <tbody>
                {fox.in_progress.length === 0 ? <tr><td className="muted">{t('fox.empty')}</td></tr> : null}
                {fox.in_progress.map((q) => (
                  <tr key={q.call} className={q.rr73_sent ? 'fox-done' : undefined}>
                    <td className="fox-call">{q.call}</td>
                    <td>{q.sent}</td>
                    <td>{q.rcvd ?? '—'}</td>
                    <td>×{q.tries}</td>
                    <td className="fox-act"><button type="button" className="glass chip" onClick={() => void postFox({ action: 'drop', call: q.call })}>{t('fox.drop')}</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="fox-box">
            <h2>{t('fox.queue')}</h2>
            <ol className="fox-queue data">
              {fox.queue.length === 0 ? <li className="muted">{t('fox.empty')}</li> : null}
              {fox.queue.map((q) => (
                <li key={q.call}>
                  <span className="fox-call">{q.call}</span> <span className="muted">{q.snr} dB</span>
                  <button type="button" className="glass chip" onClick={() => void postFox({ action: 'unqueue', call: q.call })}>{t('fox.remove')}</button>
                </li>
              ))}
            </ol>
          </div>
          <div className="fox-box">
            <h2>{t('fox.tx')}</h2>
            <ul className="fox-tx data">
              {fox.tx.length === 0 ? <li className="muted">{t('fox.empty')}</li> : fox.tx.map((m, i) => <li key={i}>{m}</li>)}
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="fox-stat">
      <strong className="data">{value}</strong>
      <span className="muted">{label}</span>
    </div>
  );
}

function order(a: Caller, b: Caller, key: SortKey): number {
  if (key === 'call') return a.call.localeCompare(b.call);
  if (key === 'snr') return b.snr - a.snr;
  return a[key] - b[key];
}
