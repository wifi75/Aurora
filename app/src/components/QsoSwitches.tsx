import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getControls, postControl, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';
import { periodLabel } from '../lib/txPeriod';
import { useEngineTx } from '../state/engineTx';
import { useStation } from '../state/station';
import { holdControl, releaseControl, useShownControls } from '../lib/pendingControls';

/** Interruttori del QSO e di trasmissione, nell'ordine in cui li usa chi lavora un DX. */
// sinistra: come si svolge il QSO, due colonne; destra: tutto cio' che fa trasmettere, in colonna (proposta 2 scelta dal proprietario)
const SEQUENCE: ControlId[] = ['autoseq', 'single_qso', 'answer_all', 'answer_b4', 'bypass', 'skip_tx1', 'rrr', 'tx_period'];
const SENDING: ControlId[] = ['enable_tx', 'auto_tx', 'wait_call', 'pounce'];
/** Accenderli puo' far trasmettere: serve la trasmissione aperta. */
const ARMING: ControlId[] = ['auto_tx', 'enable_tx', 'wait_call', 'pounce'];

/** Stato e comando degli interruttori del motore (AutoSeq, 1 QSO, Abilita Tx...). Spegnere e' sempre permesso. */
export function QsoSwitches({ txAllowed }: { txAllowed: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const [note, setNote] = useState('');
  const mode = useStation((st) => st.status?.mode);
  const minuteLabel = useEngineTx((st) => st.tx?.minute ?? '');
  const autoseqLabel = useEngineTx((st) => st.tx?.autoseq ?? '');
  const state = useShownControls(controls.data ?? {});
  const reachable = !controls.isError;

  const set = async (id: ControlId, on: boolean) => {
    try {
      holdControl(id, on);
      await postControl(id, on);
      client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, [id]: on }));
      await client.invalidateQueries({ queryKey: ['controls'] });
      setNote('');
    } catch (error) {
      releaseControl(id);
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
          disabled={!reachable || (arming && !on && !txAllowed) || (id === 'skip_tx1' && state.hound === true)}
          title={id === 'skip_tx1' && state.hound === true ? t('tip.ctl.skip_tx1.hound') : t(`tip.ctl.${id}` as Key)}
          onClick={() => void set(id, !on)}
        >
          {on ? <span className="chip-led" aria-hidden="true" /> : null}
          {/* AutoSeq porta il livello nel testo, come nel motore: «AutoSeq 3» */}
          {id === 'autoseq' && autoseqLabel !== '' ? autoseqLabel : t(`ctl.${id}` as Key)}
        </button>
      );
    });

  return (
    <section className="cmd-sec tx-block" aria-label={t('ctl.sec.qso')}>
      <h3 className="cmd-title">{t('ctl.sec.qso')}</h3>
      <div className="qso-cols">
        <div className="qso-col">
          <p className="qso-group">{t('ctl.grp.seq')}</p>
          <div className="qso-grid qso-grid--two">{chips(SEQUENCE)}</div>
        </div>
        <div className="qso-col">
          <p className="qso-group">{t('ctl.grp.tx')}</p>
          <div className="qso-grid">{chips(SENDING)}</div>
        </div>
      </div>
      {!reachable ? <p className="muted tx-note">{t('ctl.unavailable')}</p> : null}
      {note ? <p className="live-note" role="status">{note}</p> : null}
    </section>
  );
}
