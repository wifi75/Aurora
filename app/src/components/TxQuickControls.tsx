import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getControls, postControl, postLogQso, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { replyErrorMessage } from '../lib/replyError';
import { toast } from '../lib/toast';
import { bandOf } from '../lib/format';
import { AddQso } from '../pages/ExportPage';
import { useStation } from '../state/station';
import type { ManualQso } from '../api/client';

/** Tx/Rx Split e Canc DX: i due comandi che stavano nei Comandi, ora accanto allo stato della trasmissione. */
export function TxQuickControls({ splitOnly = false, inline = false }: { splitOnly?: boolean; inline?: boolean } = {}) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const reachable = !controls.isError;
  const split = controls.data?.split === true;
  // «Pulsante Log QSO» delle Impostazioni: registra a mano il QSO con la stazione DX (quando la sequenza non l'ha fatto)
  const showLog = usePrefs((p) => p.prefs.cmdLogQso);
  const status = useStation((s) => s.status);
  const [manual, setManual] = useState<Partial<ManualQso> | null>(null);
  // «Log QSO» come in WSJT-X: con un QSO in corso della sequenza (trasmissione abilitata e DX scelto) lo registra il motore, con i rapporti
  // della sequenza; altrimenti (chi ascolta soltanto, SWL, o un QSO fatto a voce) si apre la finestra di registrazione gia' compilata con la
  // stazione DX. Prima il motore senza QSO in corso non registrava nulla ma compariva comunque l'avviso «Log QSO» (segnalazione di uno SWL)
  const logNow = async () => {
    const call = (status?.dx_call ?? '').trim().toUpperCase();
    if (status?.tx_enabled === true && call !== '') {
      try {
        await postLogQso();
        toast(t('cmd.logQsoDone', { call }), 'ok');
      } catch (error) {
        toast(replyErrorMessage(error, t), 'error');
      }
      return;
    }
    const now = new Date();
    const hz = status?.dial_hz ?? 0;
    setManual({
      call,
      grid: (status?.dx_grid ?? '').toUpperCase(),
      date: now.toISOString().slice(0, 10),
      time: now.toISOString().slice(11, 16),
      freq_hz: hz,
      band: (hz > 0 ? bandOf(hz) : null) ?? '20m',
      mode: status?.mode || 'FT8',
      rst_sent: status?.report ?? '',
      rst_rcvd: '',
    });
  };
  const run = async (id: ControlId, on?: boolean) => {
    try {
      await postControl(id, on);
      await client.invalidateQueries({ queryKey: ['controls'] });
    } catch (error) {
      toast(replyErrorMessage(error, t), 'error');
    }
  };
  return (
    <span className={inline ? 'tx-quick tx-quick--inline' : 'tx-quick'} role="group" aria-label={`${t('ctl.split' as Key)} · ${t('ctl.clear_dx' as Key)}`}>
      <button type="button" className={`glass chip${split ? ' glass--on' : ''}`} aria-pressed={split} disabled={!reachable} title={t('tip.ctl.split')} onClick={() => void run('split', !split)}>
        {t('ctl.split' as Key)}
      </button>
      {splitOnly || !showLog ? null : (
        <button type="button" className="glass chip" disabled={!reachable} title={t('tip.cmd.logQso')} onClick={() => void logNow()}>
          {t('cmd.logQso')}
        </button>
      )}
      {splitOnly ? null : (
        <button type="button" className="glass chip" disabled={!reachable} title={t('tip.ctl.clear_dx')} onClick={() => void run('clear_dx')}>
          {t('ctl.clear_dx' as Key)}
        </button>
      )}
      {manual ? (
        <div className="logqso-backdrop" onClick={(e) => e.target === e.currentTarget && setManual(null)}>
          <div className="logqso-dialog" role="dialog" aria-modal="true" aria-label={t('cmd.logQsoTitle')}>
            <h3>{t('cmd.logQsoTitle')}</h3>
            <AddQso initial={manual} onCancel={() => setManual(null)} onDone={() => { setManual(null); void client.invalidateQueries({ queryKey: ['log'] }); }} />
          </div>
        </div>
      ) : null}
    </span>
  );
}
