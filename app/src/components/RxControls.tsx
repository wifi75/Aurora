import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState, type ReactNode } from 'react';
import { formatClock } from '../lib/format';
import { getControls, postControl, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';
import { useStation } from '../state/station';

/** Interruttori di ricezione del motore, nell'ordine della sua finestra. */
// «Sinc» (syncButton di JTDX) non e' collegato al motore proprio: non si mostra finche' non fa davvero qualcosa
const RX_TOGGLES: ControlId[] = ['hound', 'swl', 'jt_both', 'monitor', 'agc', 'filter', 'ap_list', 'mtd'];
/** Interruttori del solo decodificatore FT8. */
const FT8_ONLY: ControlId[] = ['ap_list', 'mtd'];
/** Pulsanti semplici: non hanno uno stato da leggere. */
const RX_BUTTONS: ControlId[] = ['stop_rx', 'decode', 'erase'];

/**
 * L'ora UTC del PC accanto ai comandi: sono i secondi che contano per i periodi. L'avanzamento del periodo (prima «9/15» qui) sta nella
 * barra di stato in fondo alla finestra: non si ripete.
 */
function PeriodClock({ clockLabel }: { clockLabel: string }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, []);
  return (
    <time className="period period--clock" dateTime={new Date(now).toISOString()} aria-label={clockLabel} title={clockLabel}>
      <span className="period-text data">{formatClock(now)}</span>
    </time>
  );
}

/** Comandi di ricezione del motore (Monitora, Decodifica, Ferma, Cancella, Modo SWL, Sinc, AGCc, Filtro) e avanzamento del periodo; `trailing` sta a destra, sulla stessa riga. */
export function RxControls({ mode, trailing }: { mode: string; /** Non serve piu' qui (il colore della barra del periodo se n'e' andato con la barra); resta per chi lo passa. */ transmitting?: boolean; trailing?: ReactNode }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const [note, setNote] = useState('');
  const state = controls.data ?? {};
  const reachable = !controls.isError;

  const run = async (id: ControlId, on?: boolean) => {
    // «Cancella»: come in JTDX svuota anche la finestra dei messaggi decodificati (e le nostre righe Tx), non solo la memoria del motore
    if (id === 'erase') useStation.setState({ decodes: [], txLog: [] });
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
        {/* «JT65+JT9» ha senso solo nei modi JT: in FT8 e FT4 non si mostra; «a7/a8» e «Multithread» solo in FT8 */}
        {RX_TOGGLES.filter((id) => (id !== 'jt_both' || mode.toUpperCase().startsWith('JT')) && (!FT8_ONLY.includes(id) || mode.toUpperCase() === 'FT8')).map((id) => {
          const on = state[id] === true;
          const button = (
            <button
              key={id}
              type="button"
              className={`glass chip${on ? ' glass--on' : ''}${id === 'filter' && on ? ' chip--warn' : ''}`}
              aria-pressed={on}
              disabled={!reachable}
              title={t(`tip.ctl.${id}` as Key)}
              onClick={() => void run(id, !on)}
            >
              {t(`ctl.${id}` as Key)}
            </button>
          );
          if (id === 'filter' && on) {
            return (
              <span key={id} className="chip-with-note">
                {button}
                <span className="live-note chip-note" role="status">
                  {t('ctl.filter.on', { hz: state.hound === true ? 290 : 60 })}
                </span>
              </span>
            );
          }
          return button;
        })}
        {RX_BUTTONS.map((id) => (
          <button key={id} type="button" className="glass chip" disabled={!reachable} title={t(`tip.ctl.${id}` as Key)} onClick={() => void run(id)}>
            {t(`ctl.${id}` as Key)}
          </button>
        ))}
      </div>
      <PeriodClock clockLabel={t('ctl.clock')} />
      {note ? <span className="live-note" role="status">{note}</span> : null}
      {trailing ? <div className="rx-trailing">{trailing}</div> : null}
    </div>
  );
}
