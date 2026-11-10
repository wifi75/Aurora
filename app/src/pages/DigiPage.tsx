import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { ApiError, digiAbort, digiRx, digiTransmit, getDigi, putDigi, type DigiSettings, type DigiSnapshot } from '../api/client';
import { useT } from '../i18n';
import { appendDigiTx, clearDigiText, getDigiText, onDigi, onDigiText } from '../lib/digiBus';
import { expandMacro } from '../lib/digiMacros';
import { formatDuration } from '../lib/sstv';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';
import '../styles/digi.css';

const FIELDS_KEY = 'aurora.digi.campi';
const RTTY_SHIFTS = [85, 170, 200, 425, 450, 850];
const RTTY_BAUDS = [45, 45.45, 50, 75, 100];

interface Fields {
  rst: string;
  name: string;
  qth: string;
}

function readFields(): Fields {
  try {
    const raw = browserStorage()?.getItem(FIELDS_KEY);
    if (raw) {
      const p = JSON.parse(raw) as Partial<Fields>;
      return { rst: p.rst ?? '599', name: p.name ?? '', qth: p.qth ?? '' };
    }
  } catch {
    // senza archivio si riparte dai valori di base
  }
  return { rst: '599', name: '', qth: '' };
}

/** La pagina dei modi testuali: PSK31, PSK63 e RTTY. Ricezione del testo, casella di trasmissione e macro. */
export function DigiPage() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['digi'], queryFn: getDigi, retry: false, refetchInterval: 1000 });
  const data = snap.data;
  const refresh = () => void client.invalidateQueries({ queryKey: ['digi'] });

  // un evento di stato dal flusso aggiorna subito la pagina
  useEffect(() => onDigi((ev) => (ev.kind === 'rx_status' || ev.kind === 'tx' ? refresh() : undefined)), []);

  return (
    <div className="digi-page">
      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? <DigiBody snap={data} onChanged={refresh} /> : null}
    </div>
  );
}

function DigiBody({ snap, onChanged }: { snap: DigiSnapshot; onChanged: () => void }) {
  const t = useT();
  const client = useQueryClient();
  const mycall = useStation((s) => s.status?.de_call ?? '');
  const text = useSyncExternalStore(onDigiText, getDigiText);
  const box = useRef<HTMLPreElement>(null);
  const [tx, setTx] = useState('');
  const [call, setCall] = useState('');
  const [fields, setFields] = useState<Fields>(readFields);
  const [sendNow, setSendNow] = useState(false);
  const [problem, setProblem] = useState('');
  const [rxFreq, setRxFreq] = useState(snap.settings.rx_freq_hz);
  const [txFreq, setTxFreq] = useState(snap.settings.tx_freq_hz);
  const [macros, setMacros] = useState(snap.settings.macros);
  const [saved, setSaved] = useState(false);
  const s = snap.settings;
  const sending = snap.tx.state === 'sending';
  const badge = sending ? 'sending' : snap.rx.listening ? 'listening' : 'off';
  const level = Math.max(0, Math.min(100, ((snap.rx.level_db + 60) / 60) * 100));

  // il testo scorre da solo in fondo
  useEffect(() => {
    const el = box.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [text]);

  // le frequenze cambiate altrove (altra finestra, telefono) si rileggono
  useEffect(() => setRxFreq(s.rx_freq_hz), [s.rx_freq_hz]);
  useEffect(() => setTxFreq(s.tx_freq_hz), [s.tx_freq_hz]);

  const save = (patch: Partial<DigiSettings>) => {
    void putDigi({ ...s, ...patch }).then(
      (next) => {
        client.setQueryData(['digi'], next);
        onChanged();
      },
      () => setProblem(t('digi.saveError')),
    );
  };

  const setField = (patch: Partial<Fields>) => {
    const next = { ...fields, ...patch };
    setFields(next);
    try {
      browserStorage()?.setItem(FIELDS_KEY, JSON.stringify(next));
    } catch {
      // senza archivio i dati restano finche' la pagina e' aperta
    }
  };

  const send = (body: string) => {
    const message = body.trim();
    if (message === '') return;
    setProblem('');
    digiTransmit(message).then(
      () => {
        appendDigiTx(message);
        setTx('');
        onChanged();
      },
      (e: unknown) => {
        const code = e instanceof ApiError ? e.code : '';
        setProblem(code === 'tx_disabled' ? t('digi.tx.disabled') : code === 'busy' ? t('digi.tx.busy') : code === 'no_transmitter' ? t('digi.tx.notReady') : t('digi.tx.error'));
      },
    );
  };

  const pickMacro = (body: string) => {
    const filled = expandMacro(body, { mycall, call, rst: fields.rst, name: fields.name, qth: fields.qth });
    if (sendNow) send(filled);
    else setTx(filled);
  };

  const saveMacros = () => {
    save({ macros: macros.filter((m) => m.label.trim() !== '' && m.text.trim() !== '') });
    setSaved(true);
    window.setTimeout(() => setSaved(false), 2000);
  };

  const rtty = s.mode === 'rtty';
  return (
    <>
      <header className="digi-head">
        <h1>{t('digi.title')}</h1>
        <span className={`sstv-badge sstv-badge--${badge}`} role="status">
          <span className="sstv-dot" aria-hidden="true" />
          {t(`digi.state.${badge}`)}
        </span>
        <div className="sstv-views" role="tablist" aria-label={t('digi.mode')}>
          {snap.modes.map((m) => (
            <button key={m.id} type="button" role="tab" aria-selected={s.mode === m.id} className={s.mode === m.id ? 'on' : ''} onClick={() => (s.mode === m.id ? undefined : save({ mode: m.id as DigiSettings['mode'] }))}>
              {m.name}
            </button>
          ))}
        </div>
      </header>

      <div className="digi-grid">
        <section className="digi-rx" aria-label={t('digi.rx.title')}>
          <div className="digi-bar">
            <strong>{t('digi.rx.title')}</strong>
            <span className="sstv-meter digi-level" title={`${t('digi.level')}: ${Math.round(snap.rx.level_db)} dB`}>
              <span style={{ width: `${level}%` }} />
            </span>
            <button type="button" className="glass sstv-btn" onClick={() => void (snap.rx.listening ? digiRx('stop') : digiRx('start')).then(onChanged, onChanged)}>
              {snap.rx.listening ? t('digi.rx.stop') : t('digi.rx.start')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={() => clearDigiText()}>
              {t('digi.rx.clear')}
            </button>
          </div>
          <pre className="digi-text" ref={box} aria-live="off">
            {text === '' ? <span className="digi-empty">{t('digi.rx.empty')}</span> : text}
          </pre>
        </section>

        <aside className="digi-side">
          <div className="digi-freq">
            <label>
              <span>{t('digi.freq.rx')}</span>
              <input
                type="number"
                min={200}
                max={3500}
                step={1}
                value={rxFreq}
                onChange={(e) => setRxFreq(Number(e.target.value))}
                onBlur={() => (rxFreq !== s.rx_freq_hz && rxFreq >= 200 && rxFreq <= 3500 ? save({ rx_freq_hz: rxFreq }) : setRxFreq(s.rx_freq_hz))}
                onKeyDown={(e) => (e.key === 'Enter' ? e.currentTarget.blur() : undefined)}
              />
            </label>
            <input
              type="range"
              min={200}
              max={3500}
              step={1}
              value={rxFreq}
              aria-label={t('digi.freq.rx')}
              onChange={(e) => setRxFreq(Number(e.target.value))}
              onPointerUp={() => (rxFreq !== s.rx_freq_hz ? save({ rx_freq_hz: rxFreq }) : undefined)}
              onKeyUp={() => (rxFreq !== s.rx_freq_hz ? save({ rx_freq_hz: rxFreq }) : undefined)}
            />
            <p className="sstv-hint">{t('digi.freq.hint')}</p>
            <label className="sstv-check">
              <input type="checkbox" checked={s.tx_follow_rx} onChange={(e) => save({ tx_follow_rx: e.target.checked })} />
              {t('digi.freq.follow')}
            </label>
            {s.tx_follow_rx ? null : (
              <label>
                <span>{t('digi.freq.tx')}</span>
                <input
                  type="number"
                  min={200}
                  max={3500}
                  step={1}
                  value={txFreq}
                  onChange={(e) => setTxFreq(Number(e.target.value))}
                  onBlur={() => (txFreq !== s.tx_freq_hz && txFreq >= 200 && txFreq <= 3500 ? save({ tx_freq_hz: txFreq }) : setTxFreq(s.tx_freq_hz))}
                  onKeyDown={(e) => (e.key === 'Enter' ? e.currentTarget.blur() : undefined)}
                />
              </label>
            )}
            {rtty ? (
              <div className="digi-two">
                <label>
                  <span>{t('digi.rtty.shift')}</span>
                  <select value={s.rtty_shift_hz} onChange={(e) => save({ rtty_shift_hz: Number(e.target.value) })}>
                    {(RTTY_SHIFTS.includes(s.rtty_shift_hz) ? RTTY_SHIFTS : [...RTTY_SHIFTS, s.rtty_shift_hz].sort((a, b) => a - b)).map((v) => (
                      <option key={v} value={v}>
                        {v}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>{t('digi.rtty.baud')}</span>
                  <select value={s.rtty_baud} onChange={(e) => save({ rtty_baud: Number(e.target.value) })}>
                    {(RTTY_BAUDS.includes(s.rtty_baud) ? RTTY_BAUDS : [...RTTY_BAUDS, s.rtty_baud].sort((a, b) => a - b)).map((v) => (
                      <option key={v} value={v}>
                        {v}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            ) : null}
            <label className="sstv-check">
              <input type="checkbox" checked={s.auto_receive} onChange={(e) => save({ auto_receive: e.target.checked })} />
              {t('digi.autoReceive')}
            </label>
          </div>
        </aside>
      </div>

      <section className="digi-tx" aria-label={t('digi.tx.title')}>
        <div className="digi-bar">
          <strong>{t('digi.tx.title')}</strong>
        </div>
        <div className="digi-fields">
          <label>
            <span>{t('digi.f.call')}</span>
            <input value={call} maxLength={16} onChange={(e) => setCall(e.target.value.toUpperCase())} className="digi-call" />
          </label>
          <label>
            <span>{t('digi.f.rst')}</span>
            <input value={fields.rst} maxLength={4} onChange={(e) => setField({ rst: e.target.value })} className="digi-rst" />
          </label>
          <label>
            <span>{t('digi.f.name')}</span>
            <input value={fields.name} maxLength={30} onChange={(e) => setField({ name: e.target.value })} />
          </label>
          <label>
            <span>{t('digi.f.qth')}</span>
            <input value={fields.qth} maxLength={40} onChange={(e) => setField({ qth: e.target.value })} />
          </label>
        </div>

        <div className="digi-macros" role="group" aria-label={t('digi.macros.title')}>
          {s.macros.map((m, i) => (
            <button key={`${m.label}-${i}`} type="button" className="glass sstv-btn" title={m.text} disabled={sending} onClick={() => pickMacro(m.text)}>
              {m.label}
            </button>
          ))}
          <label className="sstv-check digi-sendnow">
            <input type="checkbox" checked={sendNow} onChange={(e) => setSendNow(e.target.checked)} />
            {t('digi.macros.sendNow')}
          </label>
        </div>
        <p className="sstv-hint">{t('digi.macros.hint')}</p>

        <textarea
          className="digi-box"
          rows={4}
          value={tx}
          placeholder={t('digi.tx.placeholder')}
          onChange={(e) => setTx(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && !sending) send(tx);
          }}
        />
        <div className="sstv-sendbar">
          {sending ? (
            <>
              <div className="sstv-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(snap.tx.progress * 100)}>
                <span style={{ width: `${snap.tx.progress * 100}%` }} />
                <em>{t('digi.tx.progress', { done: formatDuration(snap.tx.elapsed_s), total: formatDuration(snap.tx.total_s) })}</em>
              </div>
              <button type="button" className="glass sstv-btn sstv-btn--danger sstv-btn--big" onClick={() => void digiAbort().then(onChanged, onChanged)}>
                {t('digi.tx.stop')}
              </button>
            </>
          ) : (
            <>
              <span className="sstv-sendinfo">
                {snap.tx.state === 'done' ? <span className="sstv-ok">{t('digi.tx.done')}</span> : null}
                {snap.tx.state === 'aborted' ? <span className="sstv-warn">{t('digi.tx.aborted')}</span> : null}
                {snap.tx.state === 'error' ? <span className="sstv-bad">{snap.tx.message ?? t('digi.tx.error')}</span> : null}
              </span>
              <button type="button" className="glass sstv-btn sstv-btn--primary sstv-btn--big" disabled={!snap.can_transmit || tx.trim() === ''} onClick={() => send(tx)}>
                {t('digi.tx.send')}
              </button>
            </>
          )}
        </div>
        {!snap.can_transmit ? <p className="notice">{t('digi.tx.notReady')}</p> : null}
        {problem ? (
          <p className="notice sstv-bad" role="alert">
            {problem}
          </p>
        ) : null}

        <details className="sstv-more">
          <summary>{t('digi.macros.edit')}</summary>
          <p className="sstv-hint">{t('digi.macros.words')}</p>
          <div className="digi-edit">
            {macros.map((m, i) => (
              <div key={i} className="digi-edit-row">
                <input aria-label={t('digi.macros.label')} placeholder={t('digi.macros.label')} value={m.label} maxLength={60} onChange={(e) => setMacros(macros.map((x, j) => (j === i ? { ...x, label: e.target.value } : x)))} />
                <input aria-label={t('digi.macros.text')} placeholder={t('digi.macros.text')} value={m.text} maxLength={2000} onChange={(e) => setMacros(macros.map((x, j) => (j === i ? { ...x, text: e.target.value } : x)))} />
                <button type="button" className="glass sstv-btn" onClick={() => setMacros(macros.filter((_, j) => j !== i))}>
                  {t('digi.macros.remove')}
                </button>
              </div>
            ))}
            <div className="digi-edit-actions">
              <button type="button" className="glass sstv-btn" onClick={() => setMacros([...macros, { label: '', text: '' }])} disabled={macros.length >= 40}>
                {t('digi.macros.add')}
              </button>
              <button type="button" className="glass sstv-btn sstv-btn--primary" onClick={saveMacros}>
                {saved ? t('digi.saved') : t('digi.save')}
              </button>
            </div>
          </div>
        </details>
      </section>
    </>
  );
}
