import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getControls, postControl, postControlValue } from '../api/client';
import { useControlValues } from '../state/controlValues';
import { useT } from '../i18n';

/** I periodi T/R di MSK144 in secondi, come la casella «T/R» di WSJT-X (`ui->sbTR->values ({5, 10, 15, 30})`). */
export const MSK144_PERIODS = [5, 10, 15, 30] as const;

/** Accanto al modo, in MSK144: il periodo T/R e «Sh» (messaggi brevi), come WSJT-X. Il periodo lo ricorda il motore per banda. */
export function MskExtras({ disabled = false }: { disabled?: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const off = disabled || controls.isError;
  const period = useControlValues((s) => s.values.msk144_period) ?? 15;
  const sh = controls.data?.msk_sh === true;

  const setPeriod = async (value: number) => {
    try {
      await postControlValue('msk144_period', value);
    } catch {
      // il prossimo giro di lettura rimette il valore vero
    }
    await client.invalidateQueries({ queryKey: ['controls'] });
  };

  const setSh = async (on: boolean) => {
    try {
      await postControl('msk_sh', on);
      // «Sh» acceso spegne «SWL» (in WSJT-X si escludono)
      client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, msk_sh: on, ...(on ? { swl: false } : {}) }));
    } catch {
      // il prossimo giro di lettura rimette lo stato vero
    }
    await client.invalidateQueries({ queryKey: ['controls'] });
  };

  return (
    <>
      <select
        className="mode-switch-sub"
        aria-label={t('mode.mskperiod')}
        title={t('tip.mode.mskperiod')}
        disabled={off}
        value={(MSK144_PERIODS as readonly number[]).includes(period) ? period : 15}
        onChange={(e) => void setPeriod(Number(e.target.value))}
      >
        {MSK144_PERIODS.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </select>
      <button
        type="button"
        className={sh ? 'glass glass--on mode-switch-sub mode-switch-flag' : 'glass mode-switch-sub mode-switch-flag'}
        aria-pressed={sh}
        title={t('tip.mode.mskSh')}
        disabled={off}
        onClick={() => void setSh(!sh)}
      >
        {t('mode.mskSh')}
      </button>
    </>
  );
}
