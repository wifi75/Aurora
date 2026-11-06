import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getControls, postControl, postReport, postRxFrequency, postTxFrequency, type ControlId } from '../api/client';
import type { StationStatus } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';
import { holdControl, releaseControl, useShownControls } from '../lib/pendingControls';

interface Props {
  /** La trasmissione e' aperta su questo PC (AURORA_ALLOW_TX=1). */
  txAllowed: boolean;
  /** Frequenza audio di partenza (Hz). */
  initialFrequency?: number;
  /** Stato del motore: frequenze e rapporto. */
  status?: StationStatus | null;
}

/**
 * Comandi: Tx, Rx e Rapporto in grande, con le frecce che copiano una frequenza sull'altra, e il pulsante Hold Tx Freq. Il resto (stazione DX,
 * CQ, messaggio libero, Tx/Rx Split, Canc DX) e' stato tolto o spostato nella Trasmissione.
 */
export function TxCommands({ txAllowed, initialFrequency = 1500, status = null }: Props) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const state = useShownControls(controls.data ?? {});
  const reachable = !controls.isError;
  const [report, setReport] = useState(String(status?.report ?? '-10').replace(/^\+/, ''));
  const [txHz, setTxHz] = useState(String(status?.tx_df ?? initialFrequency));
  const [rxHz, setRxHz] = useState(String(status?.rx_df ?? initialFrequency));
  const [note, setNote] = useState('');

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
      if (on !== undefined) holdControl(id, on);
      await postControl(id, on).catch((e: unknown) => {
        releaseControl(id);
        throw e;
      });
      if (on !== undefined) client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, [id]: on }));
      await client.invalidateQueries({ queryKey: ['controls'] });
    });

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
  const applyReport = () => {
    const db = Math.round(Number(report));
    if (Number.isFinite(db) && db >= -50 && db <= 49) void run(t('cmd.reportSet', { db: `${db >= 0 ? '+' : ''}${db}` }), () => postReport(db));
  };

  const hold = state.hold_tx === true;
  return (
    <div className="tx-panel tx-commands">
      <div className="tx-main">
        {!txAllowed && <p className="muted tx-note">{t('tx.closed')}</p>}
        {!reachable && <p className="muted tx-note">{t('ctl.unavailable')}</p>}

        {/* Tx, Rx e Rapporto in grande: campo, unita' e il pulsante che copia l'altra frequenza */}
        <div className="cmd-big" role="group" aria-label={t('ctl.sec.freq')}>
          <label className="cmd-big-row">
            <span className="cmd-big-lab">Tx</span>
            <input type="number" className="cmd-big-in" aria-label="Tx" title={t('tip.ctl.tx')} min={200} max={3000} value={txHz} onChange={(e) => setTxHz(e.target.value)} onBlur={applyTx} onKeyDown={onEnter(applyTx)} />
            <span className="cmd-big-unit">Hz</span>
            <button type="button" className="glass cmd-big-btn" disabled={!reachable} title={t('tip.ctl.rx_to_tx')} aria-label="Tx=Rx" onClick={() => void setControl('rx_to_tx')}>
              ▲
            </button>
          </label>
          <label className="cmd-big-row">
            <span className="cmd-big-lab">Rx</span>
            <input type="number" className="cmd-big-in" aria-label="Rx" title={t('tip.ctl.rx')} min={100} max={3000} value={rxHz} onChange={(e) => setRxHz(e.target.value)} onBlur={applyRx} onKeyDown={onEnter(applyRx)} />
            <span className="cmd-big-unit">Hz</span>
            <button type="button" className="glass cmd-big-btn" disabled={!reachable} title={t('tip.ctl.tx_to_rx')} aria-label="Rx=Tx" onClick={() => void setControl('tx_to_rx')}>
              ▼
            </button>
          </label>
          <label className="cmd-big-row">
            <span className="cmd-big-lab" title={t('tip.cmd.report')}>
              {t('cmd.report')}
            </span>
            <input type="number" className="cmd-big-in" aria-label={t('cmd.report')} title={t('tip.cmd.report')} min={-50} max={49} value={report} onChange={(e) => setReport(e.target.value)} onBlur={applyReport} onKeyDown={onEnter(applyReport)} />
            <span className="cmd-big-unit">dB</span>
          </label>
          {/* Hold Tx Freq: a tutta larghezza sotto i campi, con lo stato ben visibile */}
          <button
            type="button"
            className={`glass chip cmd-hold${hold ? ' glass--on' : ''}`}
            aria-pressed={hold}
            disabled={!reachable}
            title={t('tip.ctl.hold_tx')}
            onClick={() => void setControl('hold_tx', !hold)}
          >
            {t('ctl.hold_tx' as Key)}
          </button>
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
