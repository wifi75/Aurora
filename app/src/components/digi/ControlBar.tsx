import { Icon } from '../Icon';
import { useEffect, useState } from 'react';
import { useT } from '../../i18n';
import { cssColor, MACRO_ROWS, type Macro } from '../../lib/macroLang';

const ROW_LABELS = ['', 'Ctrl', 'Shift', 'Ctrl+Shift'] as const;

interface Props {
  macros: ReadonlyMap<string, Macro>;
  /** Tutte e 4 le righe sempre in vista. */
  extended: boolean;
  disabled?: boolean;
  onToggleExtended(): void;
  onRun(macro: Macro): void;
  /** Clic destro: si modifica la macro di quel tasto (esista o no). */
  onEdit(name: string): void;
  /** Stato del CQ automatico: i tasti che lo comandano (<AUTOCQ>, <ACQ>) si vedono accesi o in pausa. */
  autoCq?: 'off' | 'on' | 'paused';
}

/** Vero se la macro accende o spegne il CQ automatico. */
const isAutoCqMacro = (text: string) => /<\s*(AUTOCQ|ACQ)\s*>/i.test(text);

/** I tasti Ctrl e Shift tenuti premuti: a barra normale mostrano la riga corrispondente. */
function useHeldModifiers(): { ctrl: boolean; shift: boolean } {
  const [held, setHeld] = useState({ ctrl: false, shift: false });
  useEffect(() => {
    const update = (e: KeyboardEvent) => setHeld((h) => (h.ctrl === e.ctrlKey && h.shift === e.shiftKey ? h : { ctrl: e.ctrlKey, shift: e.shiftKey }));
    const clear = () => setHeld({ ctrl: false, shift: false });
    window.addEventListener('keydown', update);
    window.addEventListener('keyup', update);
    window.addEventListener('blur', clear);
    return () => {
      window.removeEventListener('keydown', update);
      window.removeEventListener('keyup', update);
      window.removeEventListener('blur', clear);
    };
  }, []);
  return held;
}

/**
 * La barra dei comandi: 12 tasti per riga a gruppi di 4 (F1-F4 · F5-F8 · F9-F12), 4 righe (nessun modificatore,
 * Ctrl, Shift, Ctrl+Shift). Ogni pulsante ha etichetta e colore; clic o tasto eseguono la macro, clic destro la modificano.
 */
export function ControlBar({ macros, extended, disabled = false, onToggleExtended, onRun, onEdit, autoCq = 'off' }: Props) {
  const t = useT();
  const held = useHeldModifiers();
  const current = held.ctrl && held.shift ? 3 : held.ctrl ? 1 : held.shift ? 2 : 0;
  const rows = extended ? [0, 1, 2, 3] : [current];

  return (
    <div className="digi-bar-keys" role="toolbar" aria-label={t('digi.bar.title')}>
      <div className="digi-keys">
        {rows.map((r) => (
          <div key={r} className="digi-keyrow">
            {Array.from({ length: 12 }, (_, i) => {
              const name = `${MACRO_ROWS[r]}F${i + 1}`;
              const macro = macros.get(name);
              const blank = !macro || (macro.label.trim() === '' && macro.text.trim() === '');
              const acq = !blank && autoCq !== 'off' && isAutoCqMacro(macro?.text ?? '') ? ` digi-key--acq-${autoCq}` : '';
              return (
                <button
                  key={name}
                  type="button"
                  className={`digi-key${blank ? ' digi-key--blank' : ''}${i % 4 === 0 && i > 0 ? ' digi-key--gap' : ''}${acq}`}
                  aria-pressed={acq ? true : undefined}
                  style={
                    !blank && macro?.color != null
                      ? {
                          // il colore della macro tinge lo sfondo e il bordo; il testo e' bianco col tema scuro e nero col chiaro
                          background: `color-mix(in srgb, ${cssColor(macro.color)} 26%, var(--key-base))`,
                          borderColor: `color-mix(in srgb, ${cssColor(macro.color)} 70%, transparent)`,
                        }
                      : undefined
                  }
                  title={blank ? `${name} — ${t('digi.bar.emptyHint')}` : `${name} — ${macro?.text ?? ''}`}
                  disabled={disabled && !blank}
                  onClick={() => (macro && !blank ? onRun(macro) : undefined)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    onEdit(name);
                  }}
                >
                  {blank ? '' : macro?.label}
                </button>
              );
            })}
            <span className="digi-keyrow-label">{ROW_LABELS[r]}</span>
            {r === rows[0] ? (
              <button type="button" className="digi-gear icon-btn icon-btn--sm" aria-pressed={extended} title={extended ? t('digi.bar.compact') : t('digi.bar.extended')} aria-label={extended ? t('digi.bar.compact') : t('digi.bar.extended')} onClick={onToggleExtended}>
                <Icon name="gear" />
              </button>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
}
