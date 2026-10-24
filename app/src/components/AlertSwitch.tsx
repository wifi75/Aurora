import type { AlertPrefs } from '../lib/alerts';

interface Props {
  value: AlertPrefs;
  onChange(next: AlertPrefs): void;
}

/** Interruttore degli avvisi sonori con il volume; il volume si muove solo ad avvisi accesi. */
export function AlertSwitch({ value, onChange }: Props) {
  return (
    <div className="alert-switch" role="group" aria-label="Avvisi sonori">
      <button type="button" aria-pressed={value.enabled} onClick={() => onChange({ ...value, enabled: !value.enabled })}>
        Avvisi sonori
      </button>
      <label>
        <span className="visually-hidden">Volume degli avvisi</span>
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
