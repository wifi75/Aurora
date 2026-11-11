import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type ReactNode } from 'react';
import { getControls, postControl, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';
import { useStation } from '../state/station';
import { AutoSwitch } from './AutoSwitch';

/** Interruttori di ricezione del motore, nell'ordine della sua finestra. */
// «Sinc» (syncButton di JTDX) non e' collegato al motore proprio: non si mostra finche' non fa davvero qualcosa
// Monitora per primo: e' l'interruttore che si guarda di piu' (barra a segmenti, scelta del proprietario 2026-11-08)
const RX_TOGGLES: ControlId[] = ['monitor', 'hound', 'swl', 'jt_both', 'agc', 'filter', 'ap_list', 'mtd'];
/** Interruttori del solo decodificatore FT8. */
const FT8_ONLY: ControlId[] = ['ap_list', 'mtd'];
/** Pulsanti semplici: non hanno uno stato da leggere. */
// «Cancella» non c'e' piu' qui: era un doppione dei pulsanti «Pulisci» delle finestre dei messaggi (scelta del proprietario 2026-11-11)
const RX_BUTTONS: ControlId[] = ['stop_rx', 'decode'];

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
        {/* gli interruttori in una sola barra a segmenti (acceso = verde); «JT65+JT9» ha senso solo nei modi JT: in FT8 e FT4 non si mostra; «a7/a8» e «Multithread» solo in FT8 */}
        <div className="rx-seg" role="group" aria-label={t('ctl.sec.rx')}>
        {RX_TOGGLES.filter((id) => (id !== 'jt_both' || /^JT(65|9)?(\+|$)/.test(mode.toUpperCase())) && (!FT8_ONLY.includes(id) || mode.toUpperCase() === 'FT8')).map((id) => {
          const on = state[id] === true;
          const button = (
            <button
              key={id}
              type="button"
              className={`rx-seg-item${on ? ' rx-seg-item--on' : ''}${id === 'filter' && on ? ' rx-seg-item--warn' : ''}`}
              aria-pressed={on}
              disabled={!reachable}
              // col Filtro acceso la spiegazione «Decodifico solo ±60 Hz attorno a Rx» sta solo nel suggerimento, non scritta accanto
              title={id === 'filter' && on ? `${t('ctl.filter.on', { hz: state.hound === true ? 290 : 60 })} · ${t('tip.ctl.filter' as Key)}` : t(`tip.ctl.${id}` as Key)}
              onClick={() => void run(id, !on)}
            >
              {t(`ctl.${id}` as Key)}
            </button>
          );
          return button;
        })}
        </div>
        {/* Ferma, Decodifica e, per ultimo, Auto: una barra a parte, sempre insieme (se non c'e' spazio vanno a capo insieme) */}
        <div className="rx-seg rx-seg--btns" role="group">
          {RX_BUTTONS.map((id) => (
            <button key={id} type="button" className="rx-seg-item" disabled={!reachable} title={t(`tip.ctl.${id}` as Key)} onClick={() => void run(id)}>
              {t(`ctl.${id}` as Key)}
            </button>
          ))}
          <AutoSwitch />
        </div>
      </div>
      {note ? <span className="live-note" role="status">{note}</span> : null}
      {trailing ? <div className="rx-trailing">{trailing}</div> : null}
    </div>
  );
}
