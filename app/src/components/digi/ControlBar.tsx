import { useEffect, useRef, useState } from 'react';
import { PopupMenu } from '../PopupMenu';
import { macroVars } from './MacroFontDialog';
import { browserStorage } from '../../lib/theme';
import { MacroFontDialog, type MacroLook, readLook, LOOK_KEY } from './MacroFontDialog';
import '../../styles/digi-macro-style.css';
import { useT } from '../../i18n';
import { cssColor, MACRO_ROWS, type Macro } from '../../lib/macroLang';
import { readMacroColorsNow, themeColor, useThemeName } from '../../lib/macroThemeColors';


const ROW_LABELS = ['', 'Ctrl', 'Shift', 'Ctrl+Shift'] as const;

interface Props {
  macros: ReadonlyMap<string, Macro>;
  /** Barra estesa (le righe scelte con `rows`) o compatta (una riga: quella dei modificatori tenuti premuti). */
  extended: boolean;
  /** Barra multipla (`true`: le righe scelte, di base 4) o singola (`false`: una riga). */
  onSetExtended(multi: boolean): void;
  /** Tutte e 4 le righe sempre in vista. */
  /** Quante righe di macro mostrare: un numero da 1 a 4, oppure `auto` (solo le righe che hanno almeno una macro). */
  rows?: number | 'auto';
  disabled?: boolean;
  onRun(macro: Macro): void;
  /** Clic destro: si modifica la macro di quel tasto (esista o no). */
  onEdit(name: string): void;
  /** Stato del CQ automatico: i tasti che lo comandano (<AUTOCQ>, <ACQ>) si vedono accesi o in pausa. */
  autoCq?: 'off' | 'on' | 'paused';
  /** Trascinamento (clic tenuto) o Alt+freccia: i due tasti si scambiano le macro. */
  onMove?(from: string, to: string): void;
}

/** Quanto tenere premuto un tasto prima di poterlo spostare (ms). */
export const HOLD_MS = 380;

/** Il tasto vicino nella griglia 4×12 (Alt+freccia), o `null` al bordo. */
export function neighborSlot(name: string, dir: 'left' | 'right' | 'up' | 'down'): string | null {
  const m = /^(Ctrl-)?(Shift-)?F(\d+)$/.exec(name);
  if (!m) return null;
  let row = MACRO_ROWS.indexOf(`${m[1] ?? ''}${m[2] ?? ''}` as (typeof MACRO_ROWS)[number]);
  let col = Number(m[3]) - 1;
  if (dir === 'left') col--;
  else if (dir === 'right') col++;
  else if (dir === 'up') row--;
  else row++;
  if (row < 0 || row > 3 || col < 0 || col > 11) return null;
  return `${MACRO_ROWS[row]}F${col + 1}`;
}

const ARROWS: Record<string, 'left' | 'right' | 'up' | 'down'> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };

/** Il tasto di una macro in breve, per l'angolo del pulsante: «F1», «C-F1» (Ctrl), «S-F1» (Shift), «CS-F1» (Ctrl+Shift). */
export function shortKey(name: string): string {
  const m = /^(Ctrl-)?(Shift-)?(F\d+)$/.exec(name);
  if (!m) return name;
  const mods = `${m[1] ? 'C' : ''}${m[2] ? 'S' : ''}`;
  return mods ? `${mods}-${m[3]}` : (m[3] ?? name);
}

/** Vero se la macro accende o spegne il CQ automatico. */
const isAutoCqMacro = (text: string) => /<\s*(AUTOCQ|ACQ)\s*>/i.test(text);


/** I tasti Ctrl e Shift tenuti premuti: a barra compatta mostrano la riga corrispondente. */
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
export function ControlBar({ macros, extended, onSetExtended, rows: rowsWanted = 4, disabled = false, onRun, onEdit, autoCq = 'off', onMove }: Props) {
  const t = useT();
  // l'ingranaggio apre un piccolo menu: barra singola o multipla, carattere dei pulsanti e rilievo 3D (le ultime due scelte restano salvate)
  const [gearAt, setGearAt] = useState<{ x: number; y: number } | null>(null);
  const [look, setLookState] = useState<MacroLook>(readLook);
  const [fontOpen, setFontOpen] = useState(false);
  // mentre il dialogo del carattere e' aperto i pulsanti mostrano subito la scelta in corso
  const [live, setLive] = useState<MacroLook | null>(null);
  const setLook = (next: MacroLook) => {
    setLookState(next);
    try {
      browserStorage()?.setItem(LOOK_KEY, JSON.stringify(next));
    } catch {
      // la scelta vale comunque finche' la pagina e' aperta
    }
  };
  // clic tenuto: dopo HOLD_MS il tasto si accende (bordo) e si puo' trascinare su un altro; un clic breve esegue come sempre
  const hold = useRef<{ name: string; x: number; y: number; timer: number | null } | null>(null);
  const [armed, setArmed] = useState<string | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const suppressClick = useRef(false);
  const toolbar = useRef<HTMLDivElement>(null);
  const clearHold = () => {
    if (hold.current?.timer != null) window.clearTimeout(hold.current.timer);
    hold.current = null;
    setArmed(null);
    setOver(null);
  };
  const slotAt = (x: number, y: number): string | null => {
    const el = typeof document.elementFromPoint === 'function' ? document.elementFromPoint(x, y) : null;
    return el?.closest<HTMLElement>('[data-macro]')?.dataset.macro ?? null;
  };
  // solo la prima riga ha i tasti F1-F12, le altre si usano col clic; le righe mostrate si scelgono dal menu Vista (o `auto`: solo quelle con macro)
  const filled = (r: number) => Array.from({ length: 12 }, (_, i) => macros.get(`${MACRO_ROWS[r]}F${i + 1}`)).some((m) => m && (m.label.trim() !== '' || m.text.trim() !== ''));
  const theme = useThemeName();
  const themeColors = readMacroColorsNow();
  const held = useHeldModifiers();
  const current = held.ctrl && held.shift ? 3 : held.ctrl ? 1 : held.shift ? 2 : 0;
  const all = [0, 1, 2, 3];
  const shownRows = rowsWanted === 'auto' ? (all.filter(filled).length > 0 ? all.filter(filled) : [0]) : all.slice(0, Math.min(4, Math.max(1, Math.round(rowsWanted))));
  const rows = extended ? shownRows : [current];

  return (
    <div ref={toolbar} className={`digi-bar-keys${armed ? ' digi-bar-keys--dragging' : ''}`} style={macroVars(live ?? look)} role="toolbar" aria-label={t('digi.bar.title')}>
      <div className="digi-keys">
        {rows.map((r) => (
          <div key={r} className="digi-keyrow">
            {Array.from({ length: 12 }, (_, i) => {
              const name = `${MACRO_ROWS[r]}F${i + 1}`;
              const macro = macros.get(name);
              const blank = !macro || (macro.label.trim() === '' && macro.text.trim() === '');
              const keyColor = themeColor(themeColors, theme, name, macro?.color ?? null);
              const acq = !blank && autoCq !== 'off' && isAutoCqMacro(macro?.text ?? '') ? ` digi-key--acq-${autoCq}` : '';
              return (
                <button
                  key={name}
                  type="button"
                  data-macro={name}
                  className={`digi-key${armed === name ? ' digi-key--armed' : ''}${over === name && armed !== name ? ' digi-key--drop' : ''}${blank ? ' digi-key--blank' : ''}${i % 4 === 0 && i > 0 ? ' digi-key--gap' : ''}${acq}`}
                  aria-pressed={acq ? true : undefined}
                  style={
                    !blank && keyColor != null
                      ? {
                          // il colore della macro (uno per tema) tinge lo sfondo e il bordo; il testo e' bianco col tema scuro e nero col chiaro
                          background: `color-mix(in srgb, ${cssColor(keyColor)} 26%, var(--key-base))`,
                          borderColor: `color-mix(in srgb, ${cssColor(keyColor)} 70%, transparent)`,
                        }
                      : undefined
                  }
                  title={blank ? `${name} — ${t('digi.bar.emptyHint')}` : `${name} — ${macro?.text ?? ''}${onMove ? `
${t('digi.bar.moveHint')}` : ''}`}
                  disabled={disabled && !blank}
                  onPointerDown={(e) => {
                    if (!onMove || blank || e.button !== 0) return;
                    const timer = window.setTimeout(() => {
                      if (hold.current) hold.current.timer = null;
                      setArmed(name);
                    }, HOLD_MS);
                    hold.current = { name, x: e.clientX, y: e.clientY, timer };
                    try {
                      e.currentTarget.setPointerCapture(e.pointerId);
                    } catch {
                      // senza cattura si trascina lo stesso finche' il puntatore resta sulla barra
                    }
                  }}
                  onPointerMove={(e) => {
                    const h = hold.current;
                    if (!h) return;
                    if (h.timer !== null) {
                      // mosso prima della soglia: niente spostamento, resta un clic normale
                      if (Math.hypot(e.clientX - h.x, e.clientY - h.y) > 8) clearHold();
                      return;
                    }
                    setOver(slotAt(e.clientX, e.clientY));
                  }}
                  onPointerUp={(e) => {
                    const h = hold.current;
                    if (!h) return;
                    if (h.timer === null) {
                      // dopo la soglia il rilascio non esegue mai la macro; sposta solo se si e' su un altro tasto
                      suppressClick.current = true;
                      const to = slotAt(e.clientX, e.clientY);
                      if (to && to !== h.name) onMove?.(h.name, to);
                    }
                    clearHold();
                  }}
                  onPointerCancel={clearHold}
                  onKeyDown={(e) => {
                    const dir = ARROWS[e.key];
                    if (!onMove || !e.altKey || !dir || blank) return;
                    e.preventDefault();
                    const to = neighborSlot(name, dir);
                    if (!to) return;
                    onMove(name, to);
                    toolbar.current?.querySelector<HTMLElement>(`[data-macro="${to}"]`)?.focus();
                  }}
                  onClick={() => {
                    if (suppressClick.current) {
                      suppressClick.current = false;
                      return;
                    }
                    if (macro && !blank) onRun(macro);
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    // al tocco il clic tenuto apre anche il menu: mentre si sposta non si apre l'editor
                    if (armed || suppressClick.current) return;
                    onEdit(name);
                  }}
                >
                  <span className="digi-key-label">{blank ? '' : macro?.label}</span>
                  {r === 0 ? <span className="digi-key-hint" aria-hidden="true">{shortKey(name)}</span> : null}
                </button>
              );
            })}
            <span className="digi-keyrow-tail">
              <span className="digi-keyrow-label">{ROW_LABELS[r]}</span>
            {r === rows[0] ? (
                <button type="button" className="digi-gear" aria-haspopup="menu" aria-expanded={gearAt !== null} title={t('digi.bar.gear')} aria-label={t('digi.bar.gear')} onClick={(e) => { const r = e.currentTarget.getBoundingClientRect(); setGearAt(gearAt ? null : { x: r.right - 210, y: r.bottom + 2 }); }}>
                ⚙
              </button>
            ) : null}
            </span>
          </div>
        ))}
      </div>
      <PopupMenu at={gearAt} onClose={() => setGearAt(null)} className="call-menu digi-st-menu" width={220} height={150} label={t('digi.bar.gear')}>
        <button type="button" role="menuitemradio" aria-checked={!extended} onClick={() => { onSetExtended(false); setGearAt(null); }}>
          <span className="digi-st-menu-check" aria-hidden="true">{!extended ? '✓' : ''}</span>
          {t('digi.bar.single')}
        </button>
        <button type="button" role="menuitemradio" aria-checked={extended} onClick={() => { onSetExtended(true); setGearAt(null); }}>
          <span className="digi-st-menu-check" aria-hidden="true">{extended ? '✓' : ''}</span>
          {t('digi.bar.extended')}
        </button>
        <hr />
        <button type="button" role="menuitem" className="digi-st-menu-flush" onClick={() => { setGearAt(null); setFontOpen(true); }}>
          {t('digi.bar.font')}
        </button>
      </PopupMenu>
      {fontOpen ? <MacroFontDialog value={look} onPreview={setLive} onApply={(v) => { setLook(v); setLive(null); setFontOpen(false); }} onClose={() => { setLive(null); setFontOpen(false); }} /> : null}
    </div>
  );
}