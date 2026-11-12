import { useEffect, useState } from 'react';
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
  // «Tx Pct» si scrive in una bozza: al motore va solo a Invio o uscendo dal campo, cosi' i tasti non si perdono e il campo vuoto non manda 0
  const [pctDraft, setPctDraft] = useState<string | null>(null);
  useEffect(() => {
    setPctDraft(null);
  }, [pct]);

  const send = async (id: 'wspr_dbm' | 'wspr_pct' | 'wspr_type1', value: number) => {
    try {
      await postControlValue(id, value);
    } catch {
      // il prossimo giro di lettura rimette il valore vero
    }
  };

  /** Manda la bozza di «Tx Pct»: il vuoto o un valore non valido si scartano (torna quello del motore), il resto si tiene fra 0 e 100. */
  const commitPct = () => {
    if (pctDraft === null) return;
    const text = pctDraft.trim();
    const v = Math.round(Number(text.replace(',', '.')));
    setPctDraft(null);
    if (text === '' || !Number.isFinite(v)) return;
    const clamped = Math.min(100, Math.max(0, v));
    if (clamped !== pct) void send('wspr_pct', clamped);
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
          value={pctDraft ?? pct}
          onChange={(e) => setPctDraft(e.target.value)}
          onBlur={() => commitPct()}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commitPct();
            else if (e.key === 'Escape') setPctDraft(null);
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
