import { postControlValue } from '../api/client';
import { useControlValues } from '../state/controlValues';
import { useT } from '../i18n';

/** La tendina della potenza di WSJT-X (`TxPowerComboBox`): dBm e watt, da 1 mW a 1 kW. */
export const WSPR_POWERS: readonly (readonly [number, string])[] = [
  [0, '1 mW'], [3, '2 mW'], [7, '5 mW'], [10, '10 mW'], [13, '20 mW'], [17, '50 mW'], [20, '100 mW'], [23, '200 mW'], [27, '500 mW'],
  [30, '1 W'], [33, '2 W'], [37, '5 W'], [40, '10 W'], [43, '20 W'], [47, '50 W'], [50, '100 W'], [53, '200 W'], [57, '500 W'], [60, '1 kW'],
];

/** I modi del radiofaro: WSPR e FST4W (anche con il periodo, «FST4W-300»). */
export function isBeaconMode(mode: string): boolean {
  return /^(WSPR|FST4W)(-\d+)?$/i.test(mode.trim());
}

/** Accanto al modo, in WSPR e FST4W: la potenza dichiarata nel messaggio, «Tx Pct» (percentuale dei periodi in trasmissione) e «Prefer type 1
 *  messages», come WSJT-X (in FST4W la stessa pagina dei controlli, `WSPR_config`). */
export function WsprExtras({ disabled = false }: { disabled?: boolean }) {
  const t = useT();
  const dbm = useControlValues((s) => s.values.wspr_dbm) ?? 37;
  const pct = useControlValues((s) => s.values.wspr_pct) ?? 20;
  const type1 = (useControlValues((s) => s.values.wspr_type1) ?? 1) === 1;

  const send = async (id: 'wspr_dbm' | 'wspr_pct' | 'wspr_type1', value: number) => {
    try {
      await postControlValue(id, value);
    } catch {
      // il prossimo giro di lettura rimette il valore vero
    }
  };

  return (
    <>
      <select
        className="mode-switch-sub"
        aria-label={t('mode.wsprPower')}
        title={t('tip.mode.wsprPower')}
        disabled={disabled}
        value={dbm}
        onChange={(e) => void send('wspr_dbm', Number(e.target.value))}
      >
        {WSPR_POWERS.map(([d, w]) => (
          <option key={d} value={d}>
            {d} dBm {w}
          </option>
        ))}
      </select>
      <label className="mode-switch-sub" title={t('tip.mode.wsprPct')}>
        {t('mode.wsprPct')}{' '}
        <input
          type="number"
          min={0}
          max={100}
          step={1}
          aria-label={t('mode.wsprPct')}
          disabled={disabled}
          value={pct}
          onChange={(e) => {
            const v = Math.round(Number(e.target.value));
            if (Number.isFinite(v) && v >= 0 && v <= 100) void send('wspr_pct', v);
          }}
        />
      </label>
      <button
        type="button"
        className={type1 ? 'glass glass--on mode-switch-sub mode-switch-flag' : 'glass mode-switch-sub mode-switch-flag'}
        aria-pressed={type1}
        title={t('tip.mode.wsprType1')}
        disabled={disabled}
        onClick={() => void send('wspr_type1', type1 ? 0 : 1)}
      >
        {t('mode.wsprType1')}
      </button>
    </>
  );
}
