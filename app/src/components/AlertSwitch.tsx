import { useT } from "../i18n";
import type { AlertPrefs } from "../lib/alerts";

interface Props {
  value: AlertPrefs;
  onChange(next: AlertPrefs): void;
}

/** Icona degli avvisi sonori (altoparlante) con il volume accanto; il volume si muove solo ad avvisi accesi. */
export function AlertSwitch({ value, onChange }: Props) {
  const t = useT();
  return (
    <div
      className="alert-switch"
      role="group"
      aria-label={t("alert.button")}
      title={t("tip.alert")}
    >
      <button
        type="button"
        className="glass alert-icon"
        aria-pressed={value.enabled}
        aria-label={t("alert.button")}
        title={t("alert.button")}
        onClick={() => onChange({ ...value, enabled: !value.enabled })}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M11 5 6 9H3v6h3l5 4z" fill="currentColor" />
          {value.enabled ? (
            <>
              <path d="M15.5 8.5a5 5 0 0 1 0 7" />
              <path d="M18.5 5.5a9 9 0 0 1 0 13" />
            </>
          ) : (
            <path d="m16 9 6 6m0-6-6 6" />
          )}
        </svg>
      </button>
      <label>
        <span className="visually-hidden">{t("alert.volume")}</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.05"
          value={value.volume}
          disabled={!value.enabled}
          onChange={(e) =>
            onChange({ ...value, volume: Number(e.target.value) })
          }
        />
      </label>
    </div>
  );
}
