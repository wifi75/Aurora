import { useEffect, useRef, useState } from 'react';
import { useT } from '../i18n';

/** I modi che Aurora propone: gli stessi sul PC e sul telefono. */
/** I sottomodi di JT4: la distanza fra i toni e' 4,375 Hz per 1, 2, 4, 9, 18, 36, 72. */
const JT4_SUBMODES = ['A', 'B', 'C', 'D', 'E', 'F', 'G'] as const;

/** I sottomodi di JT65: la distanza fra i toni e' 2,69 Hz per 1, 2, 4 (B e C per l'EME e le VHF, come WSJT-X). */
const JT65_SUBMODES = ['A', 'B', 'C'] as const;

/** I periodi di Q65 in secondi e i sottomodi (distanza fra i toni = velocita' x 1, 2, 4, 8, 16), come WSJT-X. */
const Q65_PERIODS = ['15', '30', '60', '120', '300'] as const;
const Q65_SUBMODES = ['A', 'B', 'C', 'D', 'E'] as const;

/** «Q65-60B» -> ['60', 'B']; «Q65» da solo e' il Q65-30A di prima. */
export function q65Parts(mode: string): [string, string] | null {
  const up = mode.toUpperCase();
  if (up === 'Q65') return ['30', 'A'];
  const m = /^Q65-(15|30|60|120|300)([A-E])$/.exec(up);
  return m?.[1] && m[2] ? [m[1], m[2]] : null;
}

/** I periodi di FST4 e FST4W in secondi, come WSJT-X (FST4W non ha quelli sotto i 2 minuti). */
const FST4_PERIODS = ['15', '30', '60', '120', '300', '900', '1800'] as const;
const FST4W_PERIODS = ['120', '300', '900', '1800'] as const;

/** «FST4-60» -> ['FST4', '60'], «FST4W-120» -> ['FST4W', '120']; senza periodo (il motore lo ricorda) null. */
export function fst4Parts(mode: string): ['FST4' | 'FST4W', string] | null {
  const m = /^(FST4W?)-(\d+)$/.exec(mode.toUpperCase());
  if (!m?.[1] || !m[2]) return null;
  const list: readonly string[] = m[1] === 'FST4W' ? FST4W_PERIODS : FST4_PERIODS;
  return list.includes(m[2]) ? [m[1] as 'FST4' | 'FST4W', m[2]] : null;
}

export const MODES = ['FT8', 'FT4', 'JT65', 'JT9', 'JT4', 'Q65', 'MSK144', 'FST4', 'FST4W'] as const;

/** Il modo in uso: JTDX con il pulsante JT puo' riportare «JT65+JT9» (si accendono tutti e due) o «JT» (il vecchio nome, acceso JT65). */
const isActive = (m: string, mode: string) => {
  const up = mode.toUpperCase();
  if (m === 'JT65' && up === 'JT') return true;
  // JT4A ... JT4G accendono JT4
  if (m === 'JT4' && /^JT4[A-G]$/.test(up)) return true;
  // JT65A, JT65B, JT65C accendono JT65
  if (m === 'JT65' && /^JT65[A-C]$/.test(up)) return true;
  // Q65-15A ... Q65-300E accendono Q65
  if (m === 'Q65' && q65Parts(up)) return true;
  // FST4-15 ... FST4-1800 accendono FST4, FST4W-120 ... FST4W-1800 accendono FST4W
  if ((m === 'FST4' || m === 'FST4W') && fst4Parts(up)?.[0] === m) return true;
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

/** Selettore compatto del modo: un pulsante col modo in uso che apre una tendina con FT8 / FT4 / JT65 / JT9 / JT4 / Q65 / MSK144 / FST4 / FST4W. */
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
  const q65 = q65Parts(mode);
  const fst4 = fst4Parts(mode);
  return (
    <div className="mode-switch" ref={box}>
      <button
        type="button"
        className="glass mode-switch-btn"
        // JT65B e JT65C hanno il colore di JT65
        data-mode={/^JT65[A-C]$/i.test(label) ? 'JT65' : fst4 ? fst4[0] : label}
        aria-current="true"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t('mode.group')}
        // in trasmissione il modo non si cambia, ma il pulsante non si spegne di colore (scelta del proprietario): si ignora solo il clic
        disabled={!onPick}
        aria-disabled={locked || undefined}
        title={onPick ? (locked ? t('mode.locked') : t('tip.mode.bar')) : t('band.noRig')}
        onClick={() => {
          if (!locked) setOpen((o) => !o);
        }}
      >
        {label}
        <span className="mode-switch-caret" aria-hidden="true">
          ▾
        </span>
      </button>
      {/^JT4[A-G]?$/.test(mode.toUpperCase()) ? (
        <select
          className="mode-switch-sub"
          aria-label={t('mode.jt4sub')}
          title={t('tip.mode.jt4sub')}
          disabled={disabled}
          value={mode.toUpperCase().slice(3) || 'A'}
          onChange={(e) => onPick?.(`JT4${e.target.value}`)}
        >
          {JT4_SUBMODES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      ) : null}
      {/^JT65[A-C]?$/.test(mode.toUpperCase()) ? (
        <select
          className="mode-switch-sub"
          aria-label={t('mode.jt65sub')}
          title={t('tip.mode.jt65sub')}
          disabled={disabled}
          value={mode.toUpperCase().slice(4) || 'A'}
          onChange={(e) => onPick?.(`JT65${e.target.value}`)}
        >
          {JT65_SUBMODES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      ) : null}
      {q65 ? (
        <>
          <select
            className="mode-switch-sub"
            aria-label={t('mode.q65period')}
            title={t('tip.mode.q65period')}
            disabled={disabled}
            value={q65[0]}
            onChange={(e) => onPick?.(`Q65-${e.target.value}${q65[1]}`)}
          >
            {Q65_PERIODS.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
          <select
            className="mode-switch-sub"
            aria-label={t('mode.q65sub')}
            title={t('tip.mode.q65sub')}
            disabled={disabled}
            value={q65[1]}
            onChange={(e) => onPick?.(`Q65-${q65[0]}${e.target.value}`)}
          >
            {Q65_SUBMODES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </>
      ) : null}
      {fst4 ? (
        <select
          className="mode-switch-sub"
          aria-label={t('mode.fst4period')}
          title={t('tip.mode.fst4period')}
          disabled={disabled}
          value={fst4[1]}
          onChange={(e) => onPick?.(`${fst4[0]}-${e.target.value}`)}
        >
          {(fst4[0] === 'FST4W' ? FST4W_PERIODS : FST4_PERIODS).map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
      ) : null}
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
