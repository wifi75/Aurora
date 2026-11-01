import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { ApiError, getControls, getEngineSettings, postControl, postCq, postDx, postFreeText, postLogQso, postReport, postTune, postRxFrequency, postSpotDxsummit, postTxFrequency, type ControlId } from '../api/client';
import type { StationStatus } from '../api/schemas';
import { useT, type Key, type T } from '../i18n';
import { expandMacro, useMacros } from '../lib/macros';
import { replyErrorMessage } from '../lib/replyError';
import { usePrefs } from '../lib/prefs';
import { useStation } from '../state/station';

interface Props {
  /** La trasmissione e' aperta su questo PC (AURORA_ALLOW_TX=1). */
  txAllowed: boolean;
  /** Frequenza audio di partenza (Hz). */
  initialFrequency?: number;
  /** Stato del motore: frequenze, stazione DX e dati per riempire le macro. */
  status?: StationStatus | null;
}

const MAX_TEXT = 13;
const MAX_SPOT_TEXT = 60;
/** I CQ con destinazione, come la tendina di WSJT-X 3.3 (comboBoxCQ). */
const CQ_KINDS = ['CQ', 'CQ DX', 'CQ NA', 'CQ SA', 'CQ EU', 'CQ AS', 'CQ AF', 'CQ OC', 'CQ POTA', 'CQ SOTA', 'CQ TEST'] as const;

/** Perche' lo spot non e' partito, nella lingua scelta. */
function spotErrorMessage(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    switch (error.code) {
      case 'spot_off':
        return t('err.spot.off');
      case 'spot_no_call':
        return t('err.spot.noCall');
      case 'spot_bad_call':
        return t('err.spot.badCall');
      case 'spot_already':
        return t('err.spot.already');
      case 'spot_no_frequency':
        return t('err.spot.noFreq');
      case 'bad_spot':
        return t('err.spot.badText');
      case 'spot_failed':
        return t('err.spot.failed', { reason: error.detail ?? '—' });
      case 'local_only':
        return t('err.spot.local');
    }
  }
  return replyErrorMessage(error, t);
}

/** Pannello comandi come quello del motore, ma raccolto: QSO, frequenze, stazione DX, CQ e messaggio libero. */
export function TxCommands({ txAllowed, initialFrequency = 1500, status = null }: Props) {
  const t = useT();
  const pendingDx = useStation((s) => s.pendingDx);
  const client = useQueryClient();
  const macros = useMacros((s) => s.macros);
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const state = controls.data ?? {};
  const reachable = !controls.isError;
  const prefs = usePrefs((st) => st.prefs);
  const [cqKind, setCqKind] = useState('CQ');
  const [tuning, setTuning] = useState(false);
  const [dxInput, setDxInput] = useState(status?.dx_call ?? '');
  const [gridInput, setGridInput] = useState(status?.dx_grid ?? '');
  const [report, setReport] = useState(String(status?.report ?? '-10').replace(/^\+/, ''));
  const [text, setText] = useState('');
  const [txHz, setTxHz] = useState(String(status?.tx_df ?? initialFrequency));
  const [rxHz, setRxHz] = useState(String(status?.rx_df ?? initialFrequency));
  const [note, setNote] = useState('');
  const [spotText, setSpotText] = useState('');
  const [spotting, setSpotting] = useState(false);
  // la stazione gia' segnalata: finche' resta quella scelta il pulsante dice «Spottato» (come in JTDX, torna normale se si cambia stazione)
  const [spottedDx, setSpottedDx] = useState('');
  // lo spot si offre solo se acceso nelle impostazioni (motore proprio: con JTDX la rotta non c'e')
  const settings = useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false });
  const spotOn = settings.data?.values.AllowSpotsDXSummit === 'true' && settings.data.items.AllowSpotsDXSummit?.state === 'live';
  const dxCall = (pendingDx || status?.dx_call || '').trim();

  // la frequenza vera e' quella del motore: quando cambia lì si aggiorna anche qui
  useEffect(() => {
    if (status?.tx_df != null) setTxHz(String(status.tx_df));
  }, [status?.tx_df]);
  useEffect(() => {
    if (status?.rx_df != null) setRxHz(String(status.rx_df));
  }, [status?.rx_df]);
  // il DX scelto con il doppio clic (o dal motore) riempie i campi
  useEffect(() => {
    setDxInput(pendingDx || status?.dx_call || '');
    setGridInput(pendingDx ? '' : (status?.dx_grid ?? ''));
  }, [pendingDx, status?.dx_call, status?.dx_grid]);
  // Tune dura un periodo: finita la trasmissione il pulsante torna spento
  useEffect(() => {
    if (!status?.transmitting) setTuning(false);
  }, [status?.transmitting]);

  const run = async (what: string, action: () => Promise<void>) => {
    try {
      await action();
      setNote(what);
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  const setControl = (id: ControlId, on?: boolean) =>
    run(t('ctl.done'), async () => {
      await postControl(id, on);
      if (on !== undefined) client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, [id]: on }));
      await client.invalidateQueries({ queryKey: ['controls'] });
    });

  const toggle = (id: ControlId) => {
    const on = state[id] === true;
    const locked = !reachable;
    return (
      <button
        key={id}
        type="button"
        className={`glass chip${on ? ' glass--on' : ''}`}
        aria-pressed={on}
        disabled={locked}
        title={t(`tip.ctl.${id}` as Key)}
        onClick={() => void setControl(id, !on)}
      >
        {t(`ctl.${id}` as Key)}
      </button>
    );
  };

  const press = (id: ControlId) => (
    <button key={id} type="button" className="glass chip" disabled={!reachable} title={t(`tip.ctl.${id}` as Key)} onClick={() => void setControl(id)}>
      {t(`ctl.${id}` as Key)}
    </button>
  );

  const spot = async () => {
    setSpotting(true);
    try {
      const done = await postSpotDxsummit({ dx_call: dxCall, dx_grid: pendingDx ? '' : (status?.dx_grid ?? ''), text: spotText.trim() });
      setSpottedDx(dxCall);
      setNote(t('ctl.spotOk', { call: done.dx_call, khz: (done.frequency_hz / 1000).toFixed(1) }));
      await client.invalidateQueries({ queryKey: ['engine-settings'] });
    } catch (error) {
      setNote(spotErrorMessage(error, t));
    } finally {
      setSpotting(false);
    }
  };

  const applyTx = () => {
    const hz = Number(txHz);
    if (Number.isFinite(hz)) void run(t('tx.freqSet'), () => postTxFrequency(hz));
  };
  const applyRx = () => {
    const hz = Number(rxHz);
    if (Number.isFinite(hz)) void run(t('ctl.rxSet'), () => postRxFrequency(hz));
  };
  const onEnter = (apply: () => void) => (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') apply();
  };

  // il DX scritto a mano: si manda al motore con Invio o uscendo dal campo
  const applyDx = () => {
    const call = dxInput.trim().toUpperCase();
    const grid = gridInput.trim().toUpperCase();
    if (call === (status?.dx_call ?? '') && grid === (status?.dx_grid ?? '')) return;
    void run(t('cmd.dxSet', { call: call || '—' }), () => postDx(call, grid));
  };
  const applyReport = () => {
    const db = Math.round(Number(report));
    if (Number.isFinite(db) && db >= -50 && db <= 49) void run(t('cmd.reportSet', { db: `${db >= 0 ? '+' : ''}${db}` }), () => postReport(db));
  };
  const cqModifier = cqKind === 'CQ' ? '' : cqKind.replace(/^CQ /, '');

  return (
    <div className="tx-panel tx-commands">
      <div className="tx-main">
        {!txAllowed && <p className="muted tx-note">{t('tx.closed')}</p>}
        {!reachable && <p className="muted tx-note">{t('ctl.unavailable')}</p>}

        <p className="cmd-cap">{t('cmd.cap.dx')}</p>
        <div className="cmd-line cmd-dx" role="group" aria-label={t('ctl.sec.dx')}>
          <label className="cmd-field" title={t('tip.cmd.dxCall')}>
            <span>{t('cmd.dxCall')}</span>
            <input type="text" className="data cmd-call" value={dxInput} maxLength={12} onChange={(e) => setDxInput(e.target.value.toUpperCase())} onBlur={applyDx} onKeyDown={onEnter(applyDx)} />
          </label>
          <label className="cmd-field" title={t('tip.cmd.dxGrid')}>
            <span>{t('cmd.dxGrid')}</span>
            <input type="text" className="data cmd-grid" value={gridInput} maxLength={6} onChange={(e) => setGridInput(e.target.value.toUpperCase())} onBlur={applyDx} onKeyDown={onEnter(applyDx)} />
          </label>
          <a className="glass chip" href={dxCall ? `https://www.qrz.com/db/${encodeURIComponent(dxCall)}` : undefined} target="_blank" rel="noreferrer" title={t('tip.cmd.lookup')} aria-disabled={!dxCall}>
            {t('cmd.lookup')}
          </a>
          {press('clear_dx')}
        </div>
        <div className="cmd-line" role="group" aria-label={t('cmd.report')}>
          <label className="cmd-field" title={t('tip.cmd.report')}>
            <span>{t('cmd.report')}</span>
            <input type="number" className="cmd-report" min={-50} max={49} value={report} onChange={(e) => setReport(e.target.value)} onBlur={applyReport} onKeyDown={onEnter(applyReport)} />
            <span className="muted">dB</span>
          </label>
          {press('gen_msgs')}
          {spotOn && dxCall !== '' ? (
            <>
              <input
                type="text"
                className="cmd-free cmd-spot-text"
                value={spotText}
                maxLength={MAX_SPOT_TEXT}
                aria-label={t('ctl.spotText')}
                title={t('tip.ctl.spotText')}
                placeholder={t('ctl.spotText')}
                onChange={(e) => setSpotText(e.target.value)}
              />
              <button type="button" className="glass chip" disabled={spotting || spottedDx === dxCall} title={t('tip.ctl.spot')} onClick={() => void spot()}>
                {spotting ? t('ctl.spotSending') : spottedDx === dxCall ? t('ctl.spotted') : t('ctl.spot')}
              </button>
            </>
          ) : null}
        </div>

        <p className="cmd-cap">{t('cmd.cap.freq')}</p>
        <div className="cmd-line" role="group" aria-label={t('ctl.sec.freq')}>
          <label className="cmd-hz" title={t('tip.ctl.tx')}>
            <span>Tx</span>
            <input type="number" min={200} max={3000} value={txHz} onChange={(e) => setTxHz(e.target.value)} onBlur={applyTx} onKeyDown={onEnter(applyTx)} />
          </label>
          <button type="button" className="glass chip" disabled={!reachable} title={t('tip.ctl.rx_to_tx')} onClick={() => void setControl('rx_to_tx')}>
            ▲ Tx=Rx
          </button>
          <button type="button" className="glass chip" disabled={!reachable} title={t('tip.ctl.tx_to_rx')} onClick={() => void setControl('tx_to_rx')}>
            ▼ Rx=Tx
          </button>
          <label className="cmd-hz" title={t('tip.ctl.rx')}>
            <span>Rx</span>
            <input type="number" min={100} max={3000} value={rxHz} onChange={(e) => setRxHz(e.target.value)} onBlur={applyRx} onKeyDown={onEnter(applyRx)} />
          </label>
        </div>
        <div className="cmd-line">
          {toggle('split')}
          {toggle('hold_tx')}
        </div>

        <p className="cmd-cap">{t('cmd.cap.cq')}</p>
        <div className="cmd-line" role="group" aria-label={t('ctl.sec.send')}>
          <button type="button" className="glass glass--on chip chip--main" disabled={!txAllowed} title={t('tip.tx.cq')} onClick={() => run(t('tx.cqAsked'), () => postCq(cqModifier))}>
            CQ
          </button>
          <select value={cqKind} aria-label={t('cmd.cqKind')} title={t('tip.cmd.cqKind')} onChange={(e) => setCqKind(e.target.value)}>
            {CQ_KINDS.map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
          {prefs.cmdTune ? (
            <button type="button" className={`glass chip${tuning ? ' chip--tune-on' : ''}`} disabled={!txAllowed && !tuning} title={t('tip.cmd.tune')} aria-pressed={tuning} onClick={() => {
              const next = !tuning;
              setTuning(next);
              void run(next ? t('cmd.tuneOn') : t('cmd.tuneOff'), () => postTune(next));
            }}>
              {t('cmd.tune')}
            </button>
          ) : null}
          {prefs.cmdLogQso ? (
            <button type="button" className="glass chip" disabled={!dxCall} title={t('tip.cmd.logQso')} onClick={() => run(t('cmd.logged', { call: dxCall }), () => postLogQso())}>
              {t('cmd.logQso')}
            </button>
          ) : null}
        </div>

        <p className="cmd-cap">{t('cmd.cap.free')}</p>
        <div className="cmd-line">
          <input
            type="text"
            className="cmd-free"
            value={text}
            maxLength={MAX_TEXT}
            aria-label={t('tx.free')}
            title={t('tip.tx.free')}
            onChange={(e) => setText(e.target.value.toUpperCase().slice(0, MAX_TEXT))}
            placeholder={t('tx.free')}
          />
          <button type="button" className="glass chip" disabled={text.trim() === ''} title={t('tip.tx.free')} onClick={() => run(t('tx.prepared'), () => postFreeText(text.trim(), false))}>
            {t('tx.prepare')}
          </button>
          <button
            type="button"
            className="glass glass--on chip"
            disabled={!txAllowed || text.trim() === ''}
            title={t('tip.tx.free')}
            onClick={() => run(t('tx.sent'), () => postFreeText(text.trim(), true))}
          >
            {t('tx.send')}
          </button>
        </div>

        <div className="macro-chips" role="group" aria-label={t('macro.chips')}>
          {macros.map((m) => (
            <button
              key={m}
              type="button"
              className="glass"
              title={t('tip.macro.chip')}
              onClick={() => setText(expandMacro(m, { dx: pendingDx || status?.dx_call || '', me: status?.de_call ?? '', report: status?.report ?? '' }))}
            >
              {m}
            </button>
          ))}
        </div>
        {note ? (
          <p className="live-note" role="status">
            {note}
          </p>
        ) : null}
      </div>
    </div>
  );
}
