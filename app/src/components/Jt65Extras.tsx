import { postControl } from '../api/client';
import { useT } from '../i18n';
import { DecodeToggles } from './DecodeToggles';

/**
 * I controlli di JT65 accanto al modo, come nel menu Decode e nei pulsanti di WSJT-X: «Enable AP», «Enable deep search» e «Clear Avg»
 * (svuota la media dei periodi, `avg65`). Lo stesso stile dei controlli di Q65.
 */
export function Jt65Extras({ disabled = false }: { disabled?: boolean }) {
  const t = useT();
  return (
    <>
      <DecodeToggles keys={['JT65AP', 'IncludeCorrelation']} disabled={disabled} />
      <button
        type="button"
        className="glass mode-switch-sub mode-switch-clear"
        aria-label={t('mode.jt65clearAvg')}
        title={t('tip.mode.jt65clearAvg')}
        disabled={disabled}
        onClick={() => void postControl('jt65_clear_avg').catch(() => undefined)}
      >
        ⟲
      </button>
    </>
  );
}
