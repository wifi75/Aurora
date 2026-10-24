import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getControls, postControl, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';

/** Interruttori di ricezione di JTDX, nell'ordine della sua finestra. */
const RX_TOGGLES: ControlId[] = ['hound', 'jt', 'swl', 'monitor', 'sync', 'agc', 'filter'];
/** Pulsanti semplici: non hanno uno stato da leggere. */
const RX_BUTTONS: ControlId[] = ['stop_rx', 'decode', 'erase'];

/** Secondi di un periodo: FT8 15, FT4 7,5. */
function periodSecs(mode: string): number {
  return mode === 'FT4' ? 7.5 : 15;
}

/** Secondo del periodo in corso e durata, come l'indicatore "14/15" di JTDX. */
function usePeriod(mode: string): { at: number; of: number } {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(timer);
  }, []);
  const of = periodSecs(mode);
  return { at: Math.min(of, Math.floor((now / 1000) % of) + 1), of };
}

/** Comandi di ricezione di JTDX (Monitora, Decodifica, Ferma, Cancella, Modo SWL, Sinc, AGCc, Filtro) e avanzamento del periodo. */
export function RxControls({ mode }: { mode: string }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const [note, setNote] = useState('');
  const period = usePeriod(mode);
  const state = controls.data ?? {};
  const reachable = !controls.isError;

  const run = async (id: ControlId, on?: boolean) => {
    try {
      await postControl(id, on);
      if (on !== undefined) client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, [id]: on }));
      await client.invalidateQueries({ queryKey: ['controls'] });
      setNote('');
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  return (
    <div className="rx-row" aria-label={t('ctl.sec.rx')}>
      <div className="cmd-chips">
        {RX_TOGGLES.map((id) => {
          const on = state[id] === true;
          return (
            <button
              key={id}
              type="button"
              className={`glass chip${on ? ' glass--on' : ''}`}
              aria-pressed={on}
              disabled={!reachable}
              title={t(`tip.ctl.${id}` as Key)}
              onClick={() => void run(id, !on)}
            >
              {t(`ctl.${id}` as Key)}
            </button>
          );
        })}
        {RX_BUTTONS.map((id) => (
          <button key={id} type="button" className="glass chip" disabled={!reachable} title={t(`tip.ctl.${id}` as Key)} onClick={() => void run(id)}>
            {t(`ctl.${id}` as Key)}
          </button>
        ))}
      </div>
      <div
        className="period"
        role="progressbar"
        aria-label={t('ctl.period')}
        aria-valuemin={0}
        aria-valuemax={period.of}
        aria-valuenow={period.at}
        title={t('ctl.period')}
      >
        <span className="period-fill" style={{ width: `${(period.at / period.of) * 100}%` }} />
        <span className="period-text data">
          {period.at}/{period.of}
        </span>
      </div>
      {note ? <span className="live-note" role="status">{note}</span> : null}
    </div>
  );
}
