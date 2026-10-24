import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { periodPosition } from '../lib/period';
import { getControls, postControl, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';

/** Interruttori di ricezione del motore, nell'ordine della sua finestra. */
const RX_TOGGLES: ControlId[] = ['hound', 'swl', 'monitor', 'sync', 'agc', 'filter'];
/** Pulsanti semplici: non hanno uno stato da leggere. */
const RX_BUTTONS: ControlId[] = ['stop_rx', 'decode', 'erase'];

/** Avanzamento del periodo, come l'indicatore "14/15" del motore, ma continuo: si ridisegna a ogni fotogramma. */
function PeriodBar({ mode, label, transmitting }: { mode: string; label: string; transmitting: boolean }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    let frame = 0;
    const tick = () => {
      setNow(Date.now());
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);
  const { seconds, fraction, length } = periodPosition(now, mode);
  return (
    <div className={`period${transmitting ? ' period--tx' : ''}`} role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={length} aria-valuenow={seconds} title={label}>
      <span className="period-fill" style={{ width: `${fraction * 100}%` }} />
      <span className="period-text data">
        {seconds}/{Math.round(length)}
      </span>
    </div>
  );
}

/** Comandi di ricezione del motore (Monitora, Decodifica, Ferma, Cancella, Modo SWL, Sinc, AGCc, Filtro) e avanzamento del periodo. */
export function RxControls({ mode, transmitting = false }: { mode: string; transmitting?: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const [note, setNote] = useState('');
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
      <PeriodBar mode={mode} label={t('ctl.period')} transmitting={transmitting} />
      {note ? <span className="live-note" role="status">{note}</span> : null}
    </div>
  );
}
