import { useEffect, useRef, useState } from 'react';
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

/** Selettore compatto del modo: un pulsante col modo in uso che apre una tendina con FT8 / FT4 / JT65 / JT9. */
export function ModeSwitch({ mode, onPick, locked = false }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const disabled = !onPick || locked;

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', esc);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', esc);
    };
  }, [open]);

  useEffect(() => {
    if (disabled) setOpen(false);
  }, [disabled]);

  const label = mode.toUpperCase() === 'JT' ? 'JT65' : mode || '—';
  return (
    <div className="mode-switch" ref={box}>
      <button
        type="button"
        className="glass mode-switch-btn"
        data-mode={label}
        aria-current="true"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t('mode.group')}
        disabled={disabled}
        title={onPick ? (locked ? t('mode.locked') : t('tip.mode.bar')) : t('band.noRig')}
        onClick={() => setOpen((o) => !o)}
      >
        {label}
        <span className="mode-switch-caret" aria-hidden="true">
          ▾
        </span>
      </button>
      {open && (
        <div className="mode-switch-menu glass" role="menu" aria-label={t('mode.group')}>
          {MODES.map((m) => (
            <button
              key={m}
              type="button"
              role="menuitemradio"
              data-mode={m}
              aria-checked={isActive(m, mode)}
              className={isActive(m, mode) ? 'is-on' : undefined}
              title={t('mode.pick', { mode: m })}
              onClick={() => {
                setOpen(false);
                if (!isActive(m, mode)) onPick?.(m);
              }}
            >
              {m}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
