import { useT } from '../i18n';

const MODES = ['FT8', 'FT4', 'JT'] as const;

/** il motore riporta JT65, JT9 o JT65+JT9: per il selettore sono tutti il modo JT. */
const isActive = (m: string, mode: string) => (m === 'JT' ? mode.toUpperCase().startsWith('JT') : m === mode);

interface Props {
  /** Modo in uso, come lo riporta il motore. */
  mode: string;
  /** Cambia modo; senza, il selettore mostra solo il modo in uso. */
  onPick?(mode: string): void;
  /** Bloccato (per esempio in trasmissione). */
  locked?: boolean;
}

/** Selettore FT8 / FT4 / JT come nel motore; il modo in uso e' acceso. */
export function ModeSwitch({ mode, onPick, locked = false }: Props) {
  const t = useT();
  return (
    <div className="mode-switch" role="group" aria-label={t('mode.group')} title={t('tip.mode.bar')}>
      {MODES.map((m) => (
        <button
          key={m}
          type="button"
          data-mode={m}
          className={`glass${isActive(m, mode) ? ' glass--on' : ''}`}
          aria-pressed={isActive(m, mode)}
          aria-current={isActive(m, mode) ? 'true' : undefined}
          disabled={!onPick || locked}
          title={onPick ? (locked ? t('mode.locked') : t('mode.pick', { mode: m })) : t('band.noRig')}
          onClick={() => {
            if (!isActive(m, mode)) onPick?.(m);
          }}
        >
          {m}
        </button>
      ))}
    </div>
  );
}
