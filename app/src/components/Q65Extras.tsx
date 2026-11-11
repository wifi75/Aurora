import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getControls, postControl, postControlValue } from '../api/client';
import { useControlValues } from '../state/controlValues';
import { useT } from '../i18n';
import { DecodeToggles } from './DecodeToggles';

/** Gli interruttori Q65 portati da WSJT-X (2026-11-09): ritardo EME e lista dei contest. */
const FLAGS = [
  { id: 'q65_eme', label: 'mode.q65eme', tip: 'tip.mode.q65eme' },
  { id: 'q65_contest', label: 'mode.q65contest', tip: 'tip.mode.q65contest' },
] as const;

/** I valori della deriva massima, come la tendina «Max Drift» di WSJT-X: 0 (spenta) ... 50 a passi di 5. */
const DRIFTS = Array.from({ length: 11 }, (_, i) => i * 5);

/** Accanto a periodo e sottomodo di Q65: modello dell'evanescenza (lorentziano come WSJT-X, o gaussiano), «Enable averaging» e «Auto Clear Avg» del
 * menu Decode di WSJT-X (voci delle Impostazioni, riquadro «Decodifica») e «Clear Avg» della media dei periodi. */
export function Q65Extras({ disabled = false }: { disabled?: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const gauss = controls.data?.q65_gauss === true;
  const off = disabled || controls.isError;

  const drift = useControlValues((s) => s.values.q65_drift) ?? 0;

  const setDrift = async (value: number) => {
    try {
      await postControlValue('q65_drift', value);
    } catch {
      // il prossimo giro di lettura rimette il valore vero
    }
    await client.invalidateQueries({ queryKey: ['controls'] });
  };

  const setFlag = async (id: 'q65_gauss' | 'q65_eme' | 'q65_contest', on: boolean) => {
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
      <select
        className="mode-switch-sub"
        aria-label={t('mode.q65drift')}
        title={t('tip.mode.q65drift')}
        disabled={off}
        value={DRIFTS.includes(drift) ? drift : 0}
        onChange={(e) => void setDrift(Number(e.target.value))}
      >
        {DRIFTS.map((d) => (
          <option key={d} value={d}>
            {`${t('mode.q65drift')} ${d}`}
          </option>
        ))}
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
      <DecodeToggles keys={['IncludeAveraging', 'AutoClearAvg']} disabled={off} />
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
