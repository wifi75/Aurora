import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { ApiError, getControls, getEngineSettings, postControl, postCq, postFreeText, postRxFrequency, postSpotDxsummit, postTxFrequency, type ControlId } from '../api/client';
import type { StationStatus } from '../api/schemas';
import { useT, type Key, type T } from '../i18n';
import { expandMacro, useMacros } from '../lib/macros';
import { replyErrorMessage } from '../lib/replyError';
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
  const [dx, setDx] = useState(false);
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

  return (
    <div className="tx-panel tx-commands">
      <div className="tx-main">
        {!txAllowed && <p className="muted tx-note">{t('tx.closed')}</p>}
        {!reachable && <p className="muted tx-note">{t('ctl.unavailable')}</p>}

        <div className="cmd-line" role="group" aria-label={t('ctl.sec.freq')}>
          <label className="cmd-hz" title={t('tip.ctl.tx')}>
            <span>{t('ctl.txHz')}</span>
            <input type="number" min={200} max={3000} value={txHz} onChange={(e) => setTxHz(e.target.value)} onBlur={applyTx} onKeyDown={onEnter(applyTx)} />
          </label>
          <div className="cmd-arrows">
            <button type="button" className="glass chip chip--arrow" disabled={!reachable} title={t('tip.ctl.rx_to_tx')} aria-label={t('ctl.rx_to_tx')} onClick={() => void setControl('rx_to_tx')}>
              ▲
            </button>
            <button type="button" className="glass chip chip--arrow" disabled={!reachable} title={t('tip.ctl.tx_to_rx')} aria-label={t('ctl.tx_to_rx')} onClick={() => void setControl('tx_to_rx')}>
              ▼
            </button>
          </div>
          <label className="cmd-hz" title={t('tip.ctl.rx')}>
            <span>{t('ctl.rxHz')}</span>
            <input type="number" min={100} max={3000} value={rxHz} onChange={(e) => setRxHz(e.target.value)} onBlur={applyRx} onKeyDown={onEnter(applyRx)} />
          </label>
          {toggle('split')}
        </div>

        <div className="cmd-line cmd-dx" role="group" aria-label={t('ctl.sec.dx')}>
          <span className="cmd-tag">DX</span>
          <strong className="data">{pendingDx || status?.dx_call || t('ctl.dxNone')}</strong>
          {status?.dx_grid && !pendingDx ? <span className="data muted">{status.dx_grid}</span> : null}
          {press('clear_dx')}
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

        <div className="cmd-line" role="group" aria-label={t('ctl.sec.send')}>
          <button
            type="button"
            className="glass glass--on chip chip--main"
            disabled={!txAllowed}
            title={t('tip.tx.cq')}
            onClick={() => run(dx ? t('tx.cqDxAsked') : t('tx.cqAsked'), () => postCq(dx ? 'DX' : ''))}
          >
            {dx ? t('tx.cqDx') : t('tx.cq')}
          </button>
          <label className="tx-check">
            <input type="checkbox" checked={dx} onChange={(e) => setDx(e.target.checked)} /> {t('tx.dxCheck')}
          </label>
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
