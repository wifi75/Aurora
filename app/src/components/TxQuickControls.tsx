import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getControls, postControl, postLogQso, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { replyErrorMessage } from '../lib/replyError';
import { toast } from '../lib/toast';

/** Tx/Rx Split e Canc DX: i due comandi che stavano nei Comandi, ora accanto allo stato della trasmissione. */
export function TxQuickControls({ splitOnly = false, inline = false }: { splitOnly?: boolean; inline?: boolean } = {}) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const reachable = !controls.isError;
  const split = controls.data?.split === true;
  // «Pulsante Log QSO» delle Impostazioni: registra a mano il QSO con la stazione DX (quando la sequenza non l'ha fatto)
  const showLog = usePrefs((p) => p.prefs.cmdLogQso);
  const logNow = async () => {
    try {
      await postLogQso();
      toast(t('cmd.logQso'), 'ok');
    } catch (error) {
      toast(replyErrorMessage(error, t), 'error');
    }
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
      {splitOnly || !(showLog || inline) ? null : (
        <button type="button" className="glass chip" disabled={!reachable} title={t('tip.cmd.logQso')} onClick={() => void logNow()}>
          {t('cmd.logQso')}
        </button>
      )}
      {splitOnly ? null : (
        <button type="button" className="glass chip" disabled={!reachable} title={t('tip.ctl.clear_dx')} onClick={() => void run('clear_dx')}>
          {t('ctl.clear_dx' as Key)}
        </button>
      )}
    </span>
  );
}
