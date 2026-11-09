import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getControls, postControl } from '../api/client';
import { useT } from '../i18n';

/** Gli interruttori Q65 portati da WSJT-X (2026-11-09): ritardo EME, deriva di frequenza, lista dei contest. */
const FLAGS = [
  { id: 'q65_eme', label: 'mode.q65eme', tip: 'tip.mode.q65eme' },
  { id: 'q65_drift', label: 'mode.q65drift', tip: 'tip.mode.q65drift' },
  { id: 'q65_contest', label: 'mode.q65contest', tip: 'tip.mode.q65contest' },
] as const;

/** Accanto a periodo e sottomodo di Q65: modello dell'evanescenza (lorentziano come WSJT-X, o gaussiano) e «Clear Avg» della media dei periodi. */
export function Q65Extras({ disabled = false }: { disabled?: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const gauss = controls.data?.q65_gauss === true;
  const off = disabled || controls.isError;

  const setFlag = async (id: 'q65_gauss' | 'q65_eme' | 'q65_drift' | 'q65_contest', on: boolean) => {
    try {
      await postControl(id, on);
      client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, [id]: on }));
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
        onChange={(e) => void setFlag('q65_gauss', e.target.value === 'gau')}
      >
        <option value="lor">{t('mode.q65lor')}</option>
        <option value="gau">{t('mode.q65gau')}</option>
      </select>
      {FLAGS.map((f) => {
        const on = (controls.data as Record<string, unknown> | undefined)?.[f.id] === true;
        return (
          <button
            key={f.id}
            type="button"
            className={on ? 'glass glass--on mode-switch-sub mode-switch-flag' : 'glass mode-switch-sub mode-switch-flag'}
            aria-pressed={on}
            title={t(f.tip)}
            disabled={off}
            onClick={() => void setFlag(f.id, !on)}
          >
            {t(f.label)}
          </button>
        );
      })}
      <button
        type="button"
        className="glass mode-switch-sub mode-switch-clear"
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
