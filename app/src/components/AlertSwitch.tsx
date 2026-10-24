import { useT } from '../i18n';
import type { AlertPrefs } from '../lib/alerts';

interface Props {
  value: AlertPrefs;
  onChange(next: AlertPrefs): void;
}

/** Interruttore degli avvisi sonori con il volume; il volume si muove solo ad avvisi accesi. */
export function AlertSwitch({ value, onChange }: Props) {
  const t = useT();
  return (
    <div className="alert-switch" role="group" aria-label={t('alert.button')} title={t('tip.alert')}>
      <button type="button" aria-pressed={value.enabled} onClick={() => onChange({ ...value, enabled: !value.enabled })}>
        {t('alert.button')}
      </button>
      <label>
        <span className="visually-hidden">{t('alert.volume')}</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.05"
          value={value.volume}
          disabled={!value.enabled}
          onChange={(e) => onChange({ ...value, volume: Number(e.target.value) })}
        />
      </label>
    </div>
  );
}
