import { useT } from '../i18n';

const MODES = ['FT8', 'FT4', 'JT65', 'JT9'] as const;

/** Il modo in uso: JTDX con il pulsante JT puo' riportare «JT65+JT9» (si accendono tutti e due) o «JT» (il vecchio nome, acceso JT65). */
const isActive = (m: string, mode: string) => {
  const up = mode.toUpperCase();
  if (m === 'JT65' && up === 'JT') return true;
  return up === m || (m.startsWith('JT') && up.split('+').includes(m));
};

interface Props {
  /** Modo in uso, come lo riporta il motore. */
  mode: string;
  /** Cambia modo; senza, il selettore mostra solo il modo in uso. */
  onPick?(mode: string): void;
  /** Bloccato (per esempio in trasmissione). */
  locked?: boolean;
}

/** Selettore FT8 / FT4 / JT65 / JT9 come nel motore; il modo in uso e' acceso. */
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
