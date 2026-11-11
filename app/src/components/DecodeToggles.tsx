import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getEngineSettings, putEngineSettings } from '../api/client';
import { useT, type Key } from '../i18n';

/** Le voci del menu Decode di WSJT-X che stanno anche accanto al modo, come pulsanti accesi/spenti (le stesse del riquadro «Decodifica»). */
export type DecodeToggleKey = 'IncludeAveraging' | 'AutoClearAvg' | 'JT65AP' | 'IncludeCorrelation';

/**
 * Pulsanti rapidi delle opzioni dei decodificatori: cambiano subito la voce delle impostazioni del motore (vale dal periodo che segue),
 * come le voci del menu Decode di WSJT-X. Spenti se la voce non e' disponibile (motore non proprio, o impostazioni non leggibili).
 */
export function DecodeToggles({ keys, disabled = false }: { keys: readonly DecodeToggleKey[]; disabled?: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const settings = useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false });
  const doc = settings.data;

  const flip = async (key: DecodeToggleKey, on: boolean) => {
    try {
      client.setQueryData(['engine-settings'], await putEngineSettings({ [key]: on ? 'true' : 'false' }));
    } catch {
      // rifiutata (per esempio dal telefono): resta com'era, la prossima lettura rimette lo stato vero
      await client.invalidateQueries({ queryKey: ['engine-settings'] });
    }
  };

  return (
    <>
      {keys.map((key) => {
        const live = doc?.items[key]?.state === 'live';
        const on = live && doc?.values[key] === 'true';
        return (
          <button
            key={key}
            type="button"
            className={on ? 'glass glass--on mode-switch-sub mode-switch-flag' : 'glass mode-switch-sub mode-switch-flag'}
            aria-pressed={on}
            data-setting={key}
            title={t(`jxh.${key}` as Key)}
            disabled={disabled || !live}
            onClick={() => void flip(key, !on)}
          >
            {t(`dec.short.${key}` as Key)}
          </button>
        );
      })}
    </>
  );
}
