import { useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent } from 'react';
import { useT } from '../../i18n';
import { browserStorage } from '../../lib/theme';

/** Il carattere dei pulsanti macro (famiglia vuota e dimensione 0 = quelli di base) e il rilievo 3D. */
export type FontStyleId = 'normal' | 'italic' | 'bold' | 'bolditalic';
export interface MacroLook {
  family: string;
  style: FontStyleId;
  size: number;
}
export const LOOK_KEY = 'aurora.digi.macrolook2';
const STYLES: readonly FontStyleId[] = ['normal', 'italic', 'bold', 'bolditalic'];
const SIZES = [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36];
/** Il carattere e la dimensione che il dialogo propone la prima volta. */
const FIRST_FAMILY = 'MS Sans Serif';
const FIRST_SIZE = 10;
/** I caratteri proposti: nell'elenco compaiono solo quelli presenti sul computer (misura con un canvas, senza permessi). */
const CURATED = ['MS Sans Serif', 'Segoe UI', 'Tahoma', 'Verdana', 'Arial', 'Calibri', 'Trebuchet MS', 'Consolas', 'Courier New', 'Lucida Console'];
const FALLBACK = 'system-ui, sans-serif';

export function readLook(): MacroLook {
  try {
    const v = JSON.parse(browserStorage()?.getItem(LOOK_KEY) ?? 'null') as Partial<MacroLook> | null;
    const size = Number(v?.size);
    return {
      family: typeof v?.family === 'string' ? v.family : '',
      style: STYLES.includes(v?.style as FontStyleId) ? (v?.style as FontStyleId) : 'normal',
      size: Number.isFinite(size) && size >= 6 && size <= 48 ? size : 0,
    };
  } catch {
    return { family: '', style: 'normal', size: 0 };
  }
}

const isBold = (s: FontStyleId) => s === 'bold' || s === 'bolditalic';
const isItalic = (s: FontStyleId) => s === 'italic' || s === 'bolditalic';

/** Le variabili CSS che la barra passa alle etichette dei pulsanti. */
export function macroVars(look: MacroLook): CSSProperties | undefined {
  const vars: Record<string, string> = {};
  if (look.family) vars['--macro-ff'] = `"${look.family}", ${FALLBACK}`;
  if (look.size > 0) vars['--macro-fs'] = `${look.size}px`;
  if (isBold(look.style)) vars['--macro-fw'] = '800';
  if (isItalic(look.style)) vars['--macro-fst'] = 'italic';
  return Object.keys(vars).length > 0 ? (vars as CSSProperties) : undefined;
}

interface Props {
  value: MacroLook;
  /** Ogni cambio si vede subito sui pulsanti veri (anteprima viva). */
  onPreview(v: MacroLook): void;
  onApply(v: MacroLook): void;
  onClose(): void;
}

/** Il carattere e' installato? Si confronta la larghezza di un testo con quella dei caratteri generici: se cambia, esiste. */
function isInstalled(family: string): boolean {
  try {
    const ctx = document.createElement('canvas').getContext('2d');
    if (!ctx) return true;
    const probe = 'mmmmmmmmmmlli0Ww@#';
    return ['monospace', 'sans-serif', 'serif'].some((g) => {
      ctx.font = `32px ${g}`;
      const w0 = ctx.measureText(probe).width;
      ctx.font = `32px "${family}", ${g}`;
      return ctx.measureText(probe).width !== w0;
    });
  } catch {
    return true;
  }
}

/** Un campo di testo con la sua lista sotto: la voce scelta si evidenzia, si cambia con le frecce o scrivendo le prime lettere. */
function Chooser({ label, items, selected, text, onText, onPick, render, numeric }: { label: string; items: string[]; selected: string; text: string; onText(v: string): void; onPick(v: string): void; render?: (v: string) => CSSProperties | undefined; numeric?: boolean }) {
  const list = useRef<HTMLUListElement>(null);
  // la voce scelta resta in vista
  useEffect(() => {
    list.current?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: 'nearest' });
  }, [selected]);
  const key = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    const i = items.indexOf(selected);
    const next = items[Math.min(items.length - 1, Math.max(0, i + (e.key === 'ArrowDown' ? 1 : -1)))];
    if (next !== undefined) onPick(next);
  };
  return (
    <div className="macro-font-col">
      <label>
        {label}
        <input type="text" inputMode={numeric ? 'numeric' : undefined} value={text} onChange={(e) => onText(e.target.value)} onKeyDown={key} autoComplete="off" spellCheck={false} />
      </label>
      <ul role="listbox" aria-label={label} ref={list}>
        {items.map((v) => (
          <li key={v} role="option" aria-selected={v === selected} style={render?.(v)} onClick={() => onPick(v)}>
            {v}
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * La scelta del carattere dei pulsanti macro: campi con elenco per tipo di carattere, stile e punti, e l'esempio aggiornato dal vivo.
 * Invio = OK, Esc = Annulla; «Predefinito» riporta il carattere di base.
 */
export function MacroFontDialog({ value, onPreview, onApply, onClose }: Props) {
  const t = useT();
  const [family, setFamily] = useState(value.family || FIRST_FAMILY);
  const [style, setStyle] = useState<FontStyleId>(value.style);
  const [size, setSize] = useState(value.size > 0 ? value.size : FIRST_SIZE);
  const [familyText, setFamilyText] = useState(value.family || FIRST_FAMILY);
  const [sizeText, setSizeText] = useState(String(value.size > 0 ? value.size : FIRST_SIZE));
  // solo i caratteri dell'elenco che ci sono davvero (misura con un canvas, senza chiedere permessi al browser)
  const families = useMemo(() => {
    const here = CURATED.filter(isInstalled);
    const base = here.length > 0 ? here : CURATED;
    return value.family && !base.includes(value.family) ? [value.family, ...base] : base;
  }, [value.family]);
  const styleLabel: Record<FontStyleId, string> = { normal: t('digi.bar.styleNormal'), italic: t('digi.bar.styleItalic'), bold: t('digi.bar.styleBold'), bolditalic: t('digi.bar.styleBoldItalic') };
  const styleByLabel = (l: string) => STYLES.find((s) => styleLabel[s] === l) ?? 'normal';

  // scrivendo nel campo la lista segue il primo nome che comincia con le lettere date
  const typeFamily = (v: string) => {
    setBase(false);
    setFamilyText(v);
    const hit = families.find((f) => f.toLowerCase().startsWith(v.toLowerCase()));
    if (hit && v !== '') setFamily(hit);
  };
  const typeStyle = (v: string) => {
    setBase(false);
    const hit = STYLES.find((s) => styleLabel[s].toLowerCase().startsWith(v.toLowerCase()));
    if (hit && v !== '') setStyle(hit);
    setStyleText(v);
  };
  const [styleText, setStyleText] = useState(styleLabel[value.style]);
  const typeSize = (v: string) => {
    setBase(false);
    setSizeText(v);
    const n = Number(v);
    if (Number.isFinite(n) && n >= 6 && n <= 48) setSize(Math.round(n));
  };
  // «Predefinito» riporta il carattere di base (famiglia vuota, dimensione 0), anche questo in diretta
  const [base, setBase] = useState(false);
  const current: MacroLook = base ? { family: '', style: 'normal', size: 0 } : { family, style, size };
  useEffect(() => {
    onPreview(current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [family, style, size, base]);
  const ok = () => onApply(current);
  const close = () => onClose();
  const sample: CSSProperties = { fontFamily: `"${family}", ${FALLBACK}`, fontSize: size, fontWeight: isBold(style) ? 800 : 400, fontStyle: isItalic(style) ? 'italic' : 'normal' };

  return (
    <div
      className="digi-modal macro-font-modal"
      role="dialog"
      aria-modal="true"
      aria-label={t('digi.bar.fontTitle')}
      onKeyDown={(e) => {
        if (e.key === 'Escape') onClose();
        else if (e.key === 'Enter') {
          e.preventDefault();
          ok();
        }
      }}
    >
      <div className="digi-dialog macro-font">
        <header className="cop-head">
          <strong>{t('digi.bar.fontTitle')}</strong>
        </header>
        <div className="macro-font-body">
          <Chooser
            label={t('digi.bar.fontFamily')}
            items={families}
            selected={family}
            text={familyText}
            onText={typeFamily}
            onPick={(v) => {
              setBase(false);
              setFamily(v);
              setFamilyText(v);
            }}
            render={(v) => ({ fontFamily: `"${v}", ${FALLBACK}` })}
          />
          <Chooser
            label={t('digi.bar.fontStyle')}
            items={STYLES.map((s) => styleLabel[s])}
            selected={styleLabel[style]}
            text={styleText}
            onText={typeStyle}
            onPick={(v) => {
              setBase(false);
              setStyle(styleByLabel(v));
              setStyleText(v);
            }}
            render={(v) => ({ fontWeight: isBold(styleByLabel(v)) ? 700 : 400, fontStyle: isItalic(styleByLabel(v)) ? 'italic' : 'normal' })}
          />
          <Chooser
            label={t('digi.bar.fontSize')}
            items={SIZES.map(String)}
            selected={String(size)}
            text={sizeText}
            numeric
            onText={typeSize}
            onPick={(v) => {
              setBase(false);
              setSize(Number(v));
              setSizeText(v);
            }}
          />
          <div className="macro-font-side">
            <button type="button" className="digi-tool" onClick={ok} autoFocus>
              OK
            </button>
            <button type="button" className="digi-tool" onClick={close}>
              {t('digi.mi.cancel')}
            </button>
            <button
              type="button"
              className="digi-tool"
              onClick={() => {
                setFamily(FIRST_FAMILY);
                setFamilyText(FIRST_FAMILY);
                setStyle('normal');
                setStyleText(styleLabel.normal);
                setSize(FIRST_SIZE);
                setSizeText(String(FIRST_SIZE));
                setBase(true);
              }}
            >
              {t('digi.bar.fontDefault')}
            </button>
          </div>
        </div>
        <fieldset className="macro-font-sample">
          <legend>{t('digi.bar.fontSample')}</legend>
          <div style={sample}>AaBbYyZz</div>
        </fieldset>
      </div>
    </div>
  );
}
