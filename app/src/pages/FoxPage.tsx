import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getCat, postControl, postFox, postRxFrequency, postTxFrequency } from '../api/client';
import type { FoxSnapshot } from '../api/schemas';
import { FrequencyMarkers } from '../components/FrequencyMarkers';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { RxControls } from '../components/RxControls';
import { StopButton } from '../components/StopButton';
import { Waterfall } from '../components/Waterfall';
import { useT } from '../i18n';
import { signalBandwidthHz } from '../lib/markers';
import { registerSpectrum, spanFromPassband, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { useWfPrefs } from '../lib/wfPrefs';
import { useFox } from '../state/fox';
import { useStation } from '../state/station';
import '../styles/fox.css';

type Caller = FoxSnapshot['callers'][number];
type SortKey = 'snr' | 'call' | 'freq_hz' | 'age';

/** Prima del primo stato dal motore: Fox spento, valori di base. */
const EMPTY: FoxSnapshot = { enabled: false, slots: 5, more_cqs: false, cq: 'CQ', auto_queue: false, ultra: false, callers: [], queue: [], in_progress: [], tx: [], periods: 0, logged: 0 };

const CQ_TARGETS = ['', 'DX', 'NA', 'SA', 'EU', 'AS', 'AF', 'OC'];

const WF_KEY = 'aurora-fox-wf-height';
const readHeight = (): number => {
  try {
    const v = Number(window.localStorage.getItem(WF_KEY));
    return v >= 120 && v <= 1200 ? v : 256;
  } catch {
    return 256;
  }
};
const saveHeight = (h: number) => {
  try {
    if (h >= 120) window.localStorage.setItem(WF_KEY, String(h));
  } catch {
    // senza archivio del browser l'altezza non si ricorda
  }
};

const snr = (v: number) => (v > 0 ? `+${v}` : `${v}`);

/**
 * La console della Fox: tutto quello che serve per lavorare da spedizione in una pagina sola. In testa lo stato (Fox, trasmissione, modo)
 * con i comandi principali, poi le opzioni, i numeri della sessione, la cascata con i messaggi ricevuti e infine chi chiama, la coda,
 * i QSO in corso e quello che e' in onda.
 */
export function FoxPage() {
  const t = useT();
  const fox = useFox((s) => s.fox) ?? EMPTY;
  const status = useStation((s) => s.status);
  const txOn = status?.tx_enabled ?? false;
  const transmitting = status?.transmitting ?? false;
  const [sort, setSort] = useState<SortKey>('snr');
  const [filter, setFilter] = useState('');

  // il pulsante cambia subito, senza aspettare lo stato del motore; se il comando fallisce torna com'era
  const toggleTx = (on: boolean) => {
    useStation.getState().patchStatus({ tx_enabled: on });
    postControl('enable_tx', on).catch(() => useStation.getState().patchStatus({ tx_enabled: !on }));
  };
  const options = (o: Partial<{ slots: number; more_cqs: boolean; cq: string; auto_queue: boolean; ultra: boolean }>) =>
    void postFox({ action: 'options', slots: fox.slots, more_cqs: fox.more_cqs, cq: fox.cq || 'CQ', auto_queue: fox.auto_queue, ultra: fox.ultra, ...o });
  const queued = new Set(fox.queue.map((q) => q.call));
  const callers = fox.callers.filter((c) => !queued.has(c.call) && c.call.toUpperCase().includes(filter.toUpperCase())).sort((a, b) => order(a, b, sort));
  const target = fox.cq.replace(/^CQ ?/, '');
  const state = !fox.enabled ? 'off' : !txOn ? 'waiting' : transmitting ? 'onair' : 'ready';

  return (
    <section className="foxc">
      <header className={`foxc-hero foxc-hero--${state}`}>
        <div className="foxc-id">
          <span className="foxc-badge" aria-hidden="true">
            FOX
          </span>
          <div>
            <h1>{t('fox.title')}</h1>
            <p className="foxc-sub">
              <span className="data">{status?.de_call ?? '—'}</span>
              <span className="foxc-dot" aria-hidden="true" />
              <span>{fox.ultra ? 'UltraFOX' : t('fox.classic')}</span>
              <span className="foxc-dot" aria-hidden="true" />
              <span className="data">{status?.mode ?? 'FT8'}</span>
            </p>
          </div>
        </div>
        <div className="foxc-state" role="status">
          <span className={`foxc-pill foxc-pill--${state}`}>{t(`fox.state.${state}`)}</span>
          <div className="foxc-kpis">
            <Kpi tone="ok" label={t('fox.logged')} value={fox.logged} />
            <Kpi tone="accent" label={t('fox.inProgress')} value={fox.in_progress.length} />
            <Kpi tone="warn" label={t('fox.queue')} value={fox.queue.length} />
            <Kpi tone="muted" label={t('fox.callers')} value={fox.callers.length} />
            <Kpi tone="muted" label={t('fox.periods')} value={fox.periods} />
          </div>
        </div>
        <div className="foxc-main-actions">
          <button type="button" className={`foxc-btn${fox.enabled ? ' foxc-btn--accent' : ''}`} aria-pressed={fox.enabled} onClick={() => void postControl('fox', !fox.enabled)}>
            {fox.enabled ? t('fox.on') : t('fox.off')}
          </button>
          <button type="button" className={`foxc-btn${txOn ? ' foxc-btn--tx' : ''}`} aria-pressed={txOn} onClick={() => toggleTx(!txOn)}>
            {t('fox.enableTx')}
          </button>
          <StopButton variant="bar" />
        </div>
      </header>

      {state === 'waiting' ? <p className="foxc-note">{t('fox.needTx')}</p> : null}
      {state === 'off' ? <p className="foxc-note foxc-note--info">{t('fox.offHint')}</p> : null}

      <div className="foxc-options">
        <div className="foxc-field">
          <span className="foxc-label">{t('fox.slots')}</span>
          <div className="foxc-seg" role="group" aria-label={t('fox.slots')}>
            {[1, 2, 3, 4, 5].map((n) => (
              <button key={n} type="button" aria-pressed={fox.slots === n} className={fox.slots === n ? 'on' : undefined} onClick={() => options({ slots: n })}>
                {n}
              </button>
            ))}
          </div>
        </div>
        <div className="foxc-field">
          <span className="foxc-label">{t('fox.cq')}</span>
          <div className="foxc-seg" role="group" aria-label={t('fox.cq')}>
            {CQ_TARGETS.map((c) => (
              <button key={c || 'cq'} type="button" aria-pressed={target === c} className={target === c ? 'on' : undefined} onClick={() => options({ cq: c ? `CQ ${c}` : 'CQ' })}>
                {c || 'CQ'}
              </button>
            ))}
          </div>
        </div>
        <div className="foxc-switches">
          <Switch on={fox.ultra} label="UltraFOX" title={t('tip.fox.ultra')} onChange={(v) => options({ ultra: v })} />
          <Switch on={fox.more_cqs} label={t('fox.moreCqs')} onChange={(v) => options({ more_cqs: v })} />
          <Switch on={fox.auto_queue} label={t('fox.autoQueue')} onChange={(v) => options({ auto_queue: v })} />
        </div>
        <button type="button" className="foxc-btn foxc-btn--ghost" onClick={() => void postFox({ action: 'reset' })}>
          {t('fox.reset')}
        </button>
      </div>


      <div className="foxc-grid foxc-grid--main">
        <section className="foxc-card foxc-tone--lilac">
          <header className="foxc-card-head">
            <h2>{t('fox.callers')}</h2>
            <span className="foxc-count data">{callers.length}</span>
            <div className="foxc-tools">
              <input className="foxc-input" type="search" placeholder={t('fox.filter')} value={filter} onChange={(e) => setFilter(e.target.value)} />
              <select className="foxc-input" aria-label={t('fox.sort')} value={sort} onChange={(e) => setSort(e.target.value as SortKey)}>
                <option value="snr">SNR</option>
                <option value="call">{t('fox.col.call')}</option>
                <option value="freq_hz">Hz</option>
                <option value="age">{t('fox.age')}</option>
              </select>
              <button type="button" className="foxc-btn foxc-btn--small" disabled={callers.length === 0} onClick={() => callers.slice(0, 5).forEach((c) => void postFox({ action: 'select', call: c.call }))}>
                {t('fox.addBest')}
              </button>
            </div>
          </header>
          <div className="foxc-scroll">
            <table className="foxc-table">
              <thead>
                <tr>
                  <th>{t('fox.col.call')}</th>
                  <th>{t('fox.col.grid')}</th>
                  <th className="num">SNR</th>
                  <th className="num">Hz</th>
                  <th className="num">{t('fox.age')}</th>
                  <th aria-label={t('fox.add')} />
                </tr>
              </thead>
              <tbody>
                {callers.map((c) => (
                  <tr key={c.call} onDoubleClick={() => void postFox({ action: 'select', call: c.call })}>
                    <td className="data foxc-call">{c.call}</td>
                    <td className="data">{c.grid || '—'}</td>
                    <td className="num">
                      <span className={`foxc-snr foxc-snr--${c.snr >= -5 ? 'hi' : c.snr >= -15 ? 'mid' : 'lo'}`}>{snr(c.snr)}</span>
                    </td>
                    <td className="num data">{c.freq_hz}</td>
                    <td className="num data">{c.age}</td>
                    <td className="foxc-actions">
                      <button type="button" className="foxc-btn foxc-btn--small" onClick={() => void postFox({ action: 'select', call: c.call })}>
                        {t('fox.add')}
                      </button>
                      <button type="button" className="foxc-btn foxc-btn--small foxc-btn--ghost" onClick={() => void postFox({ action: 'select', call: c.call, top: true })}>
                        {t('fox.top')}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {callers.length === 0 ? <Empty text={t('fox.noCallers')} /> : null}
          </div>
        </section>

        <div className="foxc-stack">
          <section className="foxc-card foxc-tone--mint">
            <header className="foxc-card-head">
              <h2>{t('fox.inProgress')}</h2>
              <span className="foxc-count data">{fox.in_progress.length}</span>
            </header>
            {fox.in_progress.length === 0 ? (
              <Empty text={t('fox.empty')} />
            ) : (
              <ul className="foxc-list">
                {fox.in_progress.map((q) => (
                  <li key={q.call}>
                    <span className="data foxc-call">{q.call}</span>
                    <span className="foxc-steps" aria-label={t('fox.progress')}>
                      <i className="done" title={t('fox.sent')}>{q.sent}</i>
                      <i className={q.rcvd ? 'done' : undefined} title={t('fox.rcvd')}>{q.rcvd ?? '…'}</i>
                      <i className={q.rr73_sent ? 'done' : undefined}>RR73</i>
                    </span>
                    <span className="muted data">×{q.tries}</span>
                    <button type="button" className="foxc-btn foxc-btn--small foxc-btn--ghost" onClick={() => void postFox({ action: 'drop', call: q.call })}>
                      {t('fox.drop')}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
          <section className="foxc-card foxc-tone--peach">
            <header className="foxc-card-head">
              <h2>{t('fox.onAir')}</h2>
              <span className="foxc-count data">{fox.tx.length}</span>
            </header>
            {fox.tx.length === 0 ? (
              <Empty text={t('fox.empty')} />
            ) : (
              <ol className="foxc-slots">
                {fox.tx.map((m, i) => (
                  <li key={i}>
                    <span className="foxc-slot">{i + 1}</span>
                    <span className="data">{m}</span>
                  </li>
                ))}
              </ol>
            )}
          </section>
          <section className="foxc-card foxc-tone--sky">
            <header className="foxc-card-head">
              <h2>{t('fox.queue')}</h2>
              <span className="foxc-count data">{fox.queue.length}</span>
            </header>
            {fox.queue.length === 0 ? (
              <Empty text={t('fox.empty')} />
            ) : (
              <ol className="foxc-list foxc-queue">
                {fox.queue.map((q, i) => (
                  <li key={q.call}>
                    <span className="foxc-slot">{i + 1}</span>
                    <span className="data foxc-call">{q.call}</span>
                    <span className={`foxc-snr foxc-snr--${q.snr >= -5 ? 'hi' : q.snr >= -15 ? 'mid' : 'lo'}`}>{snr(q.snr)}</span>
                    <button type="button" className="foxc-btn foxc-btn--small foxc-btn--ghost" onClick={() => void postFox({ action: 'unqueue', call: q.call })}>
                      {t('fox.remove')}
                    </button>
                  </li>
                ))}
              </ol>
            )}
          </section>
        </div>
        <Received />
      </div>

      <Spectrum />
    </section>
  );
}

/** La cascata con le barre Rx/Tx e i comandi di ricezione. */
function Spectrum() {
  const t = useT();
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const passbandHz = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 }).data?.status.rig?.passband_hz;
  const spanHz = useSpectrumSpan((s) => s.spanHz);
  const wf = useWfPrefs((s) => s.prefs);
  const { from, to } = visibleRange(wf.startHz, spanHz);
  useEffect(() => {
    useSpectrumSpan.getState().set(spanFromPassband(passbandHz));
  }, [passbandHz]);
  const [height, setHeight] = useState(readHeight);
  // la maniglia sotto la cascata: si trascina col mouse (o col dito) e l'altezza si ricorda
  const drag = (e: React.PointerEvent<HTMLDivElement>) => {
    const startY = e.clientY;
    const startH = height;
    const el = e.currentTarget;
    el.setPointerCapture?.(e.pointerId);
    const move = (ev: PointerEvent) => setHeight(Math.min(1200, Math.max(120, startH + ev.clientY - startY)));
    const up = (ev: PointerEvent) => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', up);
      saveHeight(Math.min(1200, Math.max(120, startH + ev.clientY - startY)));
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', up);
  };
  return (
    <section className="foxc-card foxc-card--flush foxc-tone--slate">
      <header className="foxc-card-head">
        <h2>{t('fox.waterfall')}</h2>
        <span className="foxc-chip data">Tx {status?.tx_df ?? '—'} Hz</span>
        <span className="foxc-chip foxc-chip--rx data">Rx {status?.rx_df ?? '—'} Hz</span>
      </header>
      <div className="foxc-rx-controls">
        <RxControls mode={status?.mode ?? ''} transmitting={status?.transmitting ?? false} />
      </div>
      <div className="foxc-wf" style={{ height }}>
        <div className="wf-stack">
          <Waterfall register={registerSpectrum} mode={status?.mode} />
          <FrequencyMarkers
            fMinHz={from}
            fMaxHz={to}
            rxHz={status?.rx_df}
            txHz={status?.tx_df}
            bandwidthHz={signalBandwidthHz(status?.mode)}
            filter={null}
            showBars={wf.bars}
            showFreq={wf.freq}
            onRx={(hz) => void postRxFrequency(hz)}
            onTx={(hz) => void postTxFrequency(hz)}
          />
        </div>
        <FrequencyRuler fMinHz={from} fMaxHz={to} decodes={decodes} />
      </div>
      <div className="foxc-wf-handle" role="separator" aria-orientation="horizontal" aria-label={t('fox.resize')} title={t('fox.resize')} onPointerDown={drag}>
        <span />
      </div>
    </section>
  );
}

/** Gli ultimi messaggi ricevuti; quelli rivolti a noi in evidenza. */
function Received() {
  const t = useT();
  const myCall = useStation((s) => s.status?.de_call ?? '');
  const decodes = useStation((s) => s.decodes);
  const recent = decodes.slice(0, 60);
  return (
    <section className="foxc-card foxc-tone--lemon">
      <header className="foxc-card-head">
        <h2>{t('fox.received')}</h2>
      </header>
      <div className="foxc-scroll foxc-scroll--rx">
        {recent.length === 0 ? (
          <Empty text={t('fox.empty')} />
        ) : (
          <ul className="foxc-rx">
            {recent.map((d) => (
              <li key={d.id} className={myCall && d.text.split(' ')[0] === myCall ? 'mine' : undefined}>
                <span className="data num">{snr(d.snr)}</span>
                <span className="data num muted">{d.freq_hz}</span>
                <span className="data">{d.text}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function Switch({ on, label, title, onChange }: { on: boolean; label: string; title?: string; onChange(v: boolean): void }) {
  return (
    <button type="button" role="switch" aria-checked={on} title={title} className={`foxc-switch${on ? ' on' : ''}`} onClick={() => onChange(!on)}>
      <span className="foxc-switch-track" aria-hidden="true">
        <span className="foxc-switch-thumb" />
      </span>
      {label}
    </button>
  );
}

function Kpi({ tone, label, value }: { tone: 'ok' | 'accent' | 'warn' | 'muted'; label: string; value: number }) {
  return (
    <div className={`foxc-kpi foxc-kpi--${tone}`}>
      <span className="foxc-kpi-label">{label}</span>
      <strong className="data">{value}</strong>
    </div>
  );
}

function Empty({ text }: { text: string }) {
  return <p className="foxc-empty">{text}</p>;
}

function order(a: Caller, b: Caller, key: SortKey): number {
  if (key === 'call') return a.call.localeCompare(b.call);
  if (key === 'snr') return b.snr - a.snr;
  return a[key] - b[key];
}
