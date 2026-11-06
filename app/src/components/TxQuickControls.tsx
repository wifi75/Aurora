import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getControls, postControl, type ControlId } from '../api/client';
import { useT, type Key } from '../i18n';
import { replyErrorMessage } from '../lib/replyError';
import { toast } from '../lib/toast';

/** Tx/Rx Split e Canc DX: i due comandi che stavano nei Comandi, ora accanto allo stato della trasmissione. */
export function TxQuickControls({ splitOnly = false }: { splitOnly?: boolean } = {}) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const reachable = !controls.isError;
  const split = controls.data?.split === true;
  const run = async (id: ControlId, on?: boolean) => {
    try {
      await postControl(id, on);
      await client.invalidateQueries({ queryKey: ['controls'] });
    } catch (error) {
      toast(replyErrorMessage(error, t), 'error');
    }
  };
  return (
    <span className="tx-quick" role="group" aria-label={`${t('ctl.split' as Key)} · ${t('ctl.clear_dx' as Key)}`}>
      <button type="button" className={`glass chip${split ? ' glass--on' : ''}`} aria-pressed={split} disabled={!reachable} title={t('tip.ctl.split')} onClick={() => void run('split', !split)}>
        {t('ctl.split' as Key)}
      </button>
      {splitOnly ? null : (
        <button type="button" className="glass chip" disabled={!reachable} title={t('tip.ctl.clear_dx')} onClick={() => void run('clear_dx')}>
          {t('ctl.clear_dx' as Key)}
        </button>
      )}
    </span>
  );
}
