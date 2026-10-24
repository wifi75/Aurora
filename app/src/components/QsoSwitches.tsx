import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getControls, postControl, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';
import { periodLabel } from '../lib/txPeriod';
import { useEngineTx } from '../state/engineTx';
import { useStation } from '../state/station';

/** Interruttori del QSO e di trasmissione, nell'ordine in cui li usa chi lavora un DX. */
const SEQUENCE: ControlId[] = ['autoseq', 'single_qso', 'answer_b4', 'bypass', 'skip_tx1', 'tx_period'];
const SENDING: ControlId[] = ['auto_tx', 'enable_tx'];
/** Accenderli puo' far trasmettere: serve la trasmissione aperta. */
const ARMING: ControlId[] = ['auto_tx', 'enable_tx'];

/** Stato e comando degli interruttori del motore (AutoSeq, 1 QSO, Abilita Tx...). Spegnere e' sempre permesso. */
export function QsoSwitches({ txAllowed }: { txAllowed: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const [note, setNote] = useState('');
  const mode = useStation((st) => st.status?.mode);
  const minuteLabel = useEngineTx((st) => st.tx?.minute ?? '');
  const state = controls.data ?? {};
  const reachable = !controls.isError;

  const set = async (id: ControlId, on: boolean) => {
    try {
      await postControl(id, on);
      client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, [id]: on }));
      await client.invalidateQueries({ queryKey: ['controls'] });
      setNote('');
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  const chips = (ids: ControlId[]) =>
    ids.map((id) => {
      const on = state[id] === true;
      const arming = ARMING.includes(id);
      // il periodo di trasmissione e' una scelta tra due, non un acceso/spento: il testo dice quale periodo e' in uso
      const period = id === 'tx_period' ? periodLabel(on, mode, minuteLabel) : null;
      if (period) {
        return (
          <button
            key={id}
            type="button"
            className={`glass chip chip--period chip--period-${on ? 'even' : 'odd'}`}
            aria-pressed={on}
            disabled={!reachable}
            title={t(`tip.ctl.${id}` as Key)}
            onClick={() => void set(id, !on)}
          >
            {period}
          </button>
        );
      }
      return (
        <button
          key={id}
          type="button"
          className={`glass chip${on ? ' glass--on' : ''}${arming ? ' chip--arming' : ''}`}
          aria-pressed={on}
          disabled={!reachable || (arming && !on && !txAllowed)}
          title={t(`tip.ctl.${id}` as Key)}
          onClick={() => void set(id, !on)}
        >
          {on ? <span className="chip-led" aria-hidden="true" /> : null}
          {t(`ctl.${id}` as Key)}
        </button>
      );
    });

  return (
    <section className="cmd-sec tx-block" aria-label={t('ctl.sec.qso')}>
      <h3 className="cmd-title">{t('ctl.sec.qso')}</h3>
      <p className="qso-group">{t('ctl.grp.seq')}</p>
      <div className="cmd-chips">{chips(SEQUENCE)}</div>
      <p className="qso-group">{t('ctl.grp.tx')}</p>
      <div className="cmd-chips">{chips(SENDING)}</div>
      {!reachable ? <p className="muted tx-note">{t('ctl.unavailable')}</p> : null}
      {note ? <p className="live-note" role="status">{note}</p> : null}
    </section>
  );
}
