import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getControls, postControl } from '../api/client';
import { useT } from '../i18n';

/** Accanto a periodo e sottomodo di Q65: modello dell'evanescenza (lorentziano come WSJT-X, o gaussiano) e «Clear Avg» della media dei periodi. */
export function Q65Extras({ disabled = false }: { disabled?: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const gauss = controls.data?.q65_gauss === true;
  const off = disabled || controls.isError;

  const setGauss = async (on: boolean) => {
    try {
      await postControl('q65_gauss', on);
      client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, q65_gauss: on }));
    } catch {
      // il prossimo giro di lettura rimette lo stato vero
    }
    await client.invalidateQueries({ queryKey: ['controls'] });
  };

  return (
    <>
      <select
        className="mode-switch-sub"
        aria-label={t('mode.q65fading')}
        title={t('tip.mode.q65fading')}
        disabled={off}
        value={gauss ? 'gau' : 'lor'}
        onChange={(e) => void setGauss(e.target.value === 'gau')}
      >
        <option value="lor">{t('mode.q65lor')}</option>
        <option value="gau">{t('mode.q65gau')}</option>
      </select>
      <button
        type="button"
        className="mode-switch-sub mode-switch-clear"
        aria-label={t('mode.q65clearAvg')}
        title={t('tip.mode.q65clearAvg')}
        disabled={off}
        onClick={() => void postControl('q65_clear_avg').catch(() => undefined)}
      >
        ⟲
      </button>
    </>
  );
}
