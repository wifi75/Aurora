import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getCat, getControls, postControl, postFox, postRxFrequency, postTxFrequency } from '../api/client';
import type { FoxSnapshot } from '../api/schemas';
import { FrequencyMarkers } from '../components/FrequencyMarkers';
import { FrequencyRuler } from '../components/FrequencyRuler';
import { RxControls } from '../components/RxControls';
import { StopButton } from '../components/StopButton';
import { Waterfall } from '../components/Waterfall';
import { WaterfallToolbar } from '../components/WaterfallToolbar';
import { TxQuickControls } from '../components/TxQuickControls';
import { SpotMarkers } from '../components/SpotMarkers';
import { FoxLogCard, FoxOtp, FoxPacket, FoxStats } from '../components/FoxPanels';
import { distanceKm, gridCenter } from '../lib/geo';
import { bandOf } from '../lib/format';
import { useT } from '../i18n';
import { filterWindowHz, signalBandwidthHz } from '../lib/markers';
import { registerSpectrum, spanFromPassband, useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';
import { useWfPrefs } from '../lib/wfPrefs';
import { useFox } from '../state/fox';
import { toast } from '../lib/toast';
import { useStation } from '../state/station';
import '../styles/fox.css';

type Caller = FoxSnapshot['callers'][number];
type SortKey = 'snr' | 'call' | 'grid' | 'freq_hz' | 'age' | 'order' | 'km';
type Row = Caller & { km: number | null; dupe: boolean };

/** Prima del primo stato dal motore: Fox spento, valori di base. */
const EMPTY: FoxSnapshot = {
  enabled: false,
  slots: 5,
  more_cqs: false,
  cq: 'CQ',
  auto_queue: false,
  ultra: false,
  callers: [],
  queue: [],
  in_progress: [],
  tx: [],
  periods: 0,
  logged: 0,
  otp: false,
  otp_interval: 1,
  log: [],
  params: { strikes: 3, wait: 3, cq_every: 10, queue_max: 10 },
};

const CQ_TARGETS = ['', 'DX', 'NA', 'SA', 'EU', 'AS', 'AF', 'OC'];

const readHeight = (key: string, fallback: number): number => {
  try {
    const v = Number(window.localStorage.getItem(key));
    return v >= 120 && v <= 1600 ? v : fallback;
  } catch {
    return fallback;
  }
};
const saveHeight = (key: string, h: number) => {
  try {
    if (h >= 120) window.localStorage.setItem(key, String(h));
  } catch {
    // senza archivio del browser l'altezza non si ricorda
  }
};

/**
 * Un'altezza che si cambia trascinando una maniglia col mouse (o col dito) e che si ricorda. `onPointerDown` la fa crescere
 * trascinando in giu'; `onPointerDownInverse` la fa crescere trascinando in su (la maniglia sopra la cascata: piu' giu' si porta,
 * piu' spazio va ai riquadri e meno alla cascata).
 */
function useDragHeight(key: string, fallback: number) {
  const [height, setHeight] = useState(() => readHeight(key, fallback));
  const drag = (sign: 1 | -1) => (e: React.PointerEvent<HTMLDivElement>) => {
    const startY = e.clientY;
    const startH = height;
    const el = e.currentTarget;
    el.setPointerCapture?.(e.pointerId);
    const at = (y: number) => Math.min(1600, Math.max(120, startH + sign * (y - startY)));
    const move = (ev: PointerEvent) => setHeight(at(ev.clientY));
    const up = (ev: PointerEvent) => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', up);
      saveHeight(key, at(ev.clientY));
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', up);
  };
  return { height, onPointerDown: drag(1), onPointerDownInverse: drag(-1) };
}

function DragHandle({ label, onPointerDown }: { label: string; onPointerDown(e: React.PointerEvent<HTMLDivElement>): void }) {
  return (
    <div className="foxc-wf-handle" role="separator" aria-orientation="horizontal" aria-label={label} title={label} onPointerDown={onPointerDown}>
      <span />
    </div>
  );
}

const snr = (v: number) => (v > 0 ? `+${v}` : `${v}`);

/**
 * La console della Fox: tutto quello che serve per lavorare da spedizione in una pagina sola. In testa, su due righe basse, lo stato,
 * i numeri della sessione in tesserine, le opzioni e i comandi; sotto «Chi chiama» (il riquadro piu' grande) con accanto QSO in corso,
 * In onda, In coda e Fox log, la cascata in basso e a destra i messaggi ricevuti.
 */
export function FoxPage() {
  const t = useT();
  const fox = useFox((s) => s.fox) ?? EMPTY;
  const status = useStation((s) => s.status);
  const txOn = status?.tx_enabled ?? false;
  const transmitting = status?.transmitting ?? false;
  const [sort, setSort] = useState<SortKey>('snr');
  const [sort2, setSort2] = useState<SortKey | ''>('age');
  const [snrMin, setSnrMin] = useState(-30);
  const [snrMax, setSnrMax] = useState(30);
  const [onlyNew, setOnlyNew] = useState(false);
  // i riquadri riempiono la finestra; si sceglie solo l'altezza della cascata, da tutte e due le maniglie
  const wf = useDragHeight('aurora-fox-wf-height-v3', 120);
  const [filter, setFilter] = useState('');

  // il pulsante cambia subito, senza aspettare lo stato del motore; se il comando fallisce torna com'era
  const toggleTx = (on: boolean) => {
    useStation.getState().patchStatus({ tx_enabled: on });
    postControl('enable_tx', on).catch(() => useStation.getState().patchStatus({ tx_enabled: !on }));
  };
  const options = (o: Partial<{ slots: number; more_cqs: boolean; cq: string; auto_queue: boolean; ultra: boolean }>) =>
    void postFox({ action: 'options', slots: fox.slots, more_cqs: fox.more_cqs, cq: fox.cq || 'CQ', auto_queue: fox.auto_queue, ultra: fox.ultra, ...o });
  const queued = new Set(fox.queue.map((q) => q.call));
  const band = status ? bandOf(status.dial_hz) : null;
  const home = gridCenter(status?.de_grid ?? '');
  const worked = new Set(fox.log.filter((q) => q.band === band).map((q) => q.call));
  const callers: Row[] = fox.callers
    .map((c) => {
      const p = gridCenter(c.grid);
      return { ...c, km: home && p ? distanceKm(home, p) : null, dupe: worked.has(c.call) };
    })
    .filter((c) => !queued.has(c.call) && c.call.toUpperCase().includes(filter.toUpperCase()) && c.snr >= snrMin && c.snr <= snrMax && !(onlyNew && c.dupe))
    .sort((a, b) => order(a, b, sort) || (sort2 ? order(a, b, sort2) : 0));
  const target = fox.cq.replace(/^CQ ?/, '');
  const state = !fox.enabled ? 'off' : !txOn ? 'waiting' : transmitting ? 'onair' : 'ready';
  // con la Fox spenta, l'istruzione arriva una volta come avviso in basso a destra (come nella Stazione), non come riquadro fisso
  const offHint = state === 'off';
  useEffect(() => {
    if (offHint) toast(t('fox.offHint'), 'info');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [offHint]);

  return (
    <section className={`foxc foxc--neon foxc--${state}`}>
      {/* testata compatta su due righe basse: 1) nome, stato, numeri in tesserine e le opzioni (Posti, CQ verso, UltraFOX, Piu' CQ,
          Coda automatica, Azzera); 2) tutti i comandi della stessa misura (Fox, Tx, STOP, ricezione, split) e la verifica OTP */}
      <header className={`foxc-hero foxc-hero--${state}`}>
        <div className="foxc-top">
          {/* Fox classica o UltraFOX si legge dall'interruttore UltraFOX; qui resta nel suggerimento per stare su una riga */}
          <div className="foxc-id" title={`${status?.de_call ?? '—'} · ${fox.ultra ? 'UltraFOX' : t('fox.classic')} · ${status?.mode ?? 'FT8'}`}>
            <span className="foxc-badge" aria-hidden="true">
              FOX
            </span>
            <h1>{t('fox.title')}</h1>
            <p className="foxc-sub">
              <span className="data">{status?.de_call ?? '—'}</span>
              <span className="foxc-dot" aria-hidden="true" />
              <span className="data">{status?.mode ?? 'FT8'}</span>
            </p>
          </div>
          <span className="foxc-state" role="status">
            <span className={`foxc-pill foxc-pill--${state}`} title={t('tip.fox.state')}>{t(`fox.state.${state}`)}</span>
          </span>
          <FoxStats fox={fox} />
          <div className="foxc-opts">
            <div className="foxc-field">
              <span className="foxc-label" title={t('tip.fox.slots')}>{t('fox.slots')}</span>
              <div className="foxc-seg" role="group" aria-label={t('fox.slots')} title={t('tip.fox.slots')}>
                {[1, 2, 3, 4, 5].map((n) => (
                  <button key={n} type="button" aria-pressed={fox.slots === n} className={fox.slots === n ? 'on' : undefined} onClick={() => options({ slots: n })}>
                    {n}
                  </button>
                ))}
              </div>
            </div>
            <div className="foxc-field">
              <span className="foxc-label" title={t('tip.fox.cq')}>{t('fox.cq')}</span>
              <div className="foxc-seg" role="group" aria-label={t('fox.cq')} title={t('tip.fox.cq')}>
                {CQ_TARGETS.map((c) => (
                  <button key={c || 'cq'} type="button" aria-pressed={target === c} className={target === c ? 'on' : undefined} onClick={() => options({ cq: c ? `CQ ${c}` : 'CQ' })}>
                    {c || 'CQ'}
                  </button>
                ))}
              </div>
            </div>
            <div className="foxc-switches">
              <Switch on={fox.ultra} label="UltraFOX" title={t('tip.fox.ultra')} onChange={(v) => options({ ultra: v })} />
              <Switch on={fox.more_cqs} label={t('fox.moreCqs')} title={t('tip.fox.moreCqs')} onChange={(v) => options({ more_cqs: v })} />
              <Switch on={fox.auto_queue} label={t('fox.autoQueue')} title={t('tip.fox.autoQueue')} onChange={(v) => options({ auto_queue: v })} />
            </div>
            <button type="button" className="foxc-btn foxc-btn--ghost foxc-btn--small" title={t('tip.fox.reset')} onClick={() => void postFox({ action: 'reset' })}>
              {t('fox.reset')}
            </button>
          </div>
        </div>
        <div className="foxc-cmdbar" role="toolbar" aria-label={t('fox.commands')}>
          <div className="foxc-main-actions">
            <button type="button" className={`foxc-btn${fox.enabled ? ' foxc-btn--accent' : ''}`} aria-pressed={fox.enabled} title={t('tip.fox.power')} onClick={() => void postControl('fox', !fox.enabled)}>
              {fox.enabled ? t('fox.on') : t('fox.off')}
            </button>
            <button type="button" className={`foxc-btn${txOn ? ' foxc-btn--tx' : ''}`} aria-pressed={txOn} title={t('tip.fox.enableTx')} onClick={() => toggleTx(!txOn)}>
              {t('fox.enableTx')}
            </button>
            <StopButton variant="bar" />
          </div>
          <span className="foxc-cmd-sep" aria-hidden="true" />
          <div className="foxc-cmd-rx">
            <RxControls mode={status?.mode ?? ''} transmitting={transmitting} />
            {/* Tx/Rx Split, come nella Trasmissione della pagina Stazione: la Tx resta ferma quando si sposta la Rx */}
            <TxQuickControls splitOnly />
          </div>
          <span className="foxc-cmd-sep" aria-hidden="true" />
          <FoxOtp fox={fox} />
        </div>
      </header>

      {fox.guard ? <p className="foxc-note">{t('fox.guard', { g: fox.guard })}</p> : null}
      {state === 'waiting' ? <p className="foxc-note">{t('fox.needTx')}</p> : null}

      {/* a sinistra chi chiama (il riquadro piu' grande), accanto QSO in corso, In onda, In coda e Fox log, sotto la cascata;
          a destra «Messaggi ricevuti» alto fino in fondo. I riquadri riempiono la finestra fino alla cascata. */}
      <div className="foxc-body">
        <div className="foxc-left">
      <div className="foxc-grid foxc-grid--main foxc-grid--two">
        <section className="foxc-card foxc-tone--lilac">
          <header className="foxc-card-head">
            <h2 title={t('tip.fox.callers')}>{t('fox.callers')}</h2>
            <span className="foxc-count data">{callers.length}</span>
            <div className="foxc-tools">
              <input className="foxc-input" type="search" placeholder={t('fox.filter')} aria-label={t('fox.filter')} title={t('tip.fox.filter')} value={filter} onChange={(e) => setFilter(e.target.value)} />
              <select className="foxc-input" aria-label={t('fox.sort')} title={t('tip.fox.sort')} value={sort} onChange={(e) => setSort(e.target.value as SortKey)}>
                <SortOptions />
              </select>
              <select className="foxc-input" aria-label={t('fox.sort2')} title={t('tip.fox.sort2')} value={sort2} onChange={(e) => setSort2(e.target.value as SortKey | '')}>
                <option value="">{t('fox.sort2')} —</option>
                <SortOptions />
              </select>
              <label className="foxc-label" title={t('tip.fox.colSnr')}>
                {t('fox.snrMin')}
                <input className="foxc-input foxc-input--num data" type="number" min={-30} max={30} value={snrMin} onChange={(e) => setSnrMin(Number(e.target.value) || -30)} />
                {t('fox.snrMax')}
                <input className="foxc-input foxc-input--num data" type="number" min={-30} max={30} value={snrMax} onChange={(e) => setSnrMax(Number(e.target.value) || 30)} />
              </label>
              <Switch on={onlyNew} label={t('fox.onlyNew')} title={t('tip.fox.onlyNew')} onChange={setOnlyNew} />
              <button type="button" className="foxc-btn foxc-btn--small" disabled={callers.length === 0} title={t('tip.fox.addBest')} onClick={() => callers.slice(0, 5).forEach((c) => void postFox({ action: 'select', call: c.call }))}>
                {t('fox.addBest')}
              </button>
            </div>
          </header>
          <div className="foxc-scroll">
            <table className="foxc-table">
              <thead>
                <tr>
                  <th title={t('tip.fox.colCall')}>{t('fox.col.call')}</th>
                  <th title={t('tip.fox.colGrid')}>{t('fox.col.grid')}</th>
                  <th className="num" title={t('tip.fox.colSnr')}>SNR</th>
                  <th className="num" title={t('tip.fox.colHz')}>Hz</th>
                  <th className="num" title={t('tip.fox.colAge')}>{t('fox.age')}</th>
                  <th className="num">km</th>
                  <th className="num" title={t('tip.fox.colOrder')}>#</th>
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
                    <td className="num data">{c.km ?? '—'}</td>
                    <td className="num data">{c.order || ''}</td>
                    <td className="foxc-actions">
                      {c.dupe ? <span className="foxc-flag">{t('fox.dupe')}</span> : null}
                      <button type="button" className="foxc-btn foxc-btn--small" title={t('tip.fox.add')} onClick={() => void postFox({ action: 'select', call: c.call })}>
                        {t('fox.add')}
                      </button>
                      <button type="button" className="foxc-btn foxc-btn--small foxc-btn--ghost" title={t('tip.fox.top')} onClick={() => void postFox({ action: 'select', call: c.call, top: true })}>
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
              <h2 title={t('tip.fox.inProgress')}>{t('fox.inProgress')}</h2>
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
                      <i className={q.rr73_sent ? 'done' : undefined} title={t('tip.fox.rr73')}>RR73</i>
                    </span>
                    <span className="muted data" title={t('tip.fox.tries')}>×{q.tries}</span>
                    <button type="button" className="foxc-btn foxc-btn--small foxc-btn--ghost" title={t('tip.fox.drop')} onClick={() => void postFox({ action: 'drop', call: q.call })}>
                      {t('fox.drop')}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
          <section className="foxc-card foxc-tone--peach">
            <header className="foxc-card-head">
              <h2 title={t('tip.fox.onAir')}>{t('fox.onAir')}</h2>
              <span className="foxc-count data">{fox.tx.length}</span>
            </header>
            <FoxPacket fox={fox} />
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
              <h2 title={t('tip.fox.queue')}>{t('fox.queue')}</h2>
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
                    <button type="button" className="foxc-btn foxc-btn--small foxc-btn--ghost" title={t('tip.fox.remove')} onClick={() => void postFox({ action: 'unqueue', call: q.call })}>
                      {t('fox.remove')}
                    </button>
                  </li>
                ))}
              </ol>
            )}
          </section>
          <FoxLogCard fox={fox} myCall={status?.de_call ?? ''} myGrid={status?.de_grid ?? ''} />
        </div>
      </div>

      <DragHandle label={t('fox.resizeMain')} onPointerDown={wf.onPointerDownInverse} />

      <Spectrum height={wf.height} onPointerDown={wf.onPointerDown} />
        </div>
        <div className="foxc-right">
          <Received />
        </div>
      </div>
    </section>
  );
}

/** La cascata con le barre Rx/Tx e i comandi di ricezione. */
function Spectrum({ height, onPointerDown }: { height: number; onPointerDown(e: React.PointerEvent<HTMLDivElement>): void }) {
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
  // la stessa cascata della pagina Stazione: barra degli strumenti, filtro, spot del cluster, righello con i messaggi e le barre Rx/Tx
  const switches = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false }).data;
  const filterWindow = switches?.filter === true ? filterWindowHz(status?.mode, switches.hound === true) : null;
  return (
    <section className="foxc-card foxc-card--flush foxc-tone--slate">
      <header className="foxc-card-head">
        <h2 title={t('tip.fox.waterfall')}>{t('fox.waterfall')}</h2>
        <span className="foxc-chip data" title={t('tip.fox.txChip')}>Tx {status?.tx_df ?? '—'} Hz</span>
        <span className="foxc-chip foxc-chip--rx data" title={t('tip.fox.rxChip')}>Rx {status?.rx_df ?? '—'} Hz</span>
      </header>
      {/* i comandi di ricezione e lo split sono saliti nella barra in alto; qui resta solo la barra degli strumenti della cascata */}
      <WaterfallToolbar />
      <div className="foxc-wf" style={{ height }}>
        <div className="wf-stack">
          <Waterfall register={registerSpectrum} mode={status?.mode} band={status ? (bandOf(status.dial_hz) ?? undefined) : undefined} />
          <FrequencyMarkers
            fMinHz={from}
            fMaxHz={to}
            rxHz={status?.rx_df}
            txHz={status?.tx_df}
            bandwidthHz={signalBandwidthHz(status?.mode)}
            filter={filterWindow}
            showBars={wf.bars}
            showFreq={wf.freq}
            onRx={(hz) => void postRxFrequency(hz)}
            onTx={(hz) => void postTxFrequency(hz)}
          />
          <SpotMarkers fMinHz={from} fMaxHz={to} dialHz={status?.dial_hz} onPick={(hz) => void postRxFrequency(hz)} />
        </div>
        <FrequencyRuler fMinHz={from} fMaxHz={to} decodes={decodes} rxHz={status?.rx_df} txHz={status?.tx_df} bandwidthHz={signalBandwidthHz(status?.mode)} />
      </div>
      <DragHandle label={t('fox.resize')} onPointerDown={onPointerDown} />
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
        <h2 title={t('tip.fox.received')}>{t('fox.received')}</h2>
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

function Empty({ text }: { text: string }) {
  return <p className="foxc-empty">{text}</p>;
}

/** Le chiavi di ordinamento dei chiamanti (come `sortHoundCalls` di WSJT-X, piu' Hz). */
function SortOptions() {
  const t = useT();
  return (
    <>
      <option value="snr">SNR</option>
      <option value="call">{t('fox.col.call')}</option>
      <option value="grid">{t('fox.col.grid')}</option>
      <option value="km">{t('fox.sort.km')}</option>
      <option value="age">{t('fox.age')}</option>
      <option value="freq_hz">Hz</option>
      <option value="order">{t('fox.sort.order')}</option>
    </>
  );
}

/** Confronto fra due chiamanti: SNR, distanza e valore esterno dal piu' alto, attesa dal piu' lungo, il resto in ordine crescente. */
function order(a: Row, b: Row, key: SortKey): number {
  if (key === 'call') return a.call.localeCompare(b.call);
  if (key === 'grid') return a.grid.localeCompare(b.grid);
  if (key === 'snr') return b.snr - a.snr;
  if (key === 'order') return b.order - a.order;
  if (key === 'km') return (b.km ?? -1) - (a.km ?? -1);
  if (key === 'age') return b.age - a.age;
  return a.freq_hz - b.freq_hz;
}
