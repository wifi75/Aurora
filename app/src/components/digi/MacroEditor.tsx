import { useRef, useState } from 'react';
import { TABLE_LANGS, useLang, useT } from '../../i18n';
import { commandsOf, type CommandGroup } from '../../lib/macroCommands';
import { EVENT_MACROS, keyNames, cssColor, type Macro } from '../../lib/macroLang';

export type MacroScope = 'all' | 'mode';

interface Props {
  /** La macro da modificare (vuota se il tasto non ne ha). */
  macro: Macro;
  /** Il nome del modo in uso, per «vale solo per...». */
  modeName: string;
  /** L'insieme in cui la macro si trova adesso. */
  scope: MacroScope;
  onSave(macro: Macro, scope: MacroScope): void;
  onCancel(): void;
}

const GROUPS: readonly (CommandGroup | '')[] = ['', 'text', 'program', 'frequency', 'autocq', 'macros', 'mode', 'rotor', 'rtty', 'sstv', 'cw', 'hell'];

/** La finestra «Edit user macro»: tasto, etichetta, colore, testo dello script, elenco dei comandi e per chi vale (tutti i modi o questo). */
export function MacroEditor({ macro, modeName, scope, onSave, onCancel }: Props) {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const box = useRef<HTMLTextAreaElement>(null);
  const [name, setName] = useState(macro.name);
  const [label, setLabel] = useState(macro.label);
  const [text, setText] = useState(macro.text);
  const [color, setColor] = useState<number | null>(macro.color);
  const [group, setGroup] = useState<CommandGroup | ''>('');
  const [picked, setPicked] = useState('');
  const [where, setWhere] = useState<MacroScope>(scope);
  // l'olandese non ha una colonna nelle tabelle dei comandi: si usa l'inglese
  const idx = lang === 'nl' ? 1 : Math.max(0, TABLE_LANGS.indexOf(lang));
  const commands = commandsOf(group);

  const insert = () => {
    if (picked === '') return;
    const el = box.current;
    const at = el ? el.selectionStart : text.length;
    const end = el ? el.selectionEnd : text.length;
    const next = text.slice(0, at) + picked + text.slice(end);
    setText(next);
    window.setTimeout(() => {
      el?.focus();
      el?.setSelectionRange(at + picked.length, at + picked.length);
    }, 0);
  };

  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.edit.title')}>
      <div className="digi-dialog">
        <header>
          <strong>{t('digi.edit.title')}</strong>
        </header>
        <div className="digi-edit-top">
          <label>
            <span>{t('digi.edit.macro')}</span>
            <select value={name} onChange={(e) => setName(e.target.value)}>
              {[...keyNames(), ...EVENT_MACROS].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>{t('digi.edit.label')}</span>
            <input value={label} maxLength={60} onChange={(e) => setLabel(e.target.value)} />
          </label>
          <label>
            <span>{t('digi.edit.color')}</span>
            <input type="color" value={cssColor(color) ?? '#f0f0f0'} onChange={(e) => setColor(Number.parseInt(e.target.value.slice(1), 16))} />
          </label>
          <label>
            <span>{t('digi.edit.scope')}</span>
            <select value={where} onChange={(e) => setWhere(e.target.value as MacroScope)}>
              <option value="all">{t('digi.edit.scopeAll')}</option>
              <option value="mode">{t('digi.edit.scopeMode', { mode: modeName })}</option>
            </select>
          </label>
        </div>
        <div className="digi-edit-body">
          <label className="digi-edit-text">
            <span>{t('digi.edit.text')}</span>
            <textarea ref={box} rows={9} value={text} onChange={(e) => setText(e.target.value)} spellCheck={false} />
          </label>
          <div className="digi-edit-cmds">
            <label>
              <span>{t('digi.edit.subset')}</span>
              <select value={group} onChange={(e) => setGroup(e.target.value as CommandGroup | '')}>
                {GROUPS.map((g) => (
                  <option key={g || 'all'} value={g}>
                    {t(`digi.edit.group.${g || 'all'}` as 'digi.edit.group.all')}
                  </option>
                ))}
              </select>
            </label>
            <ul className="digi-cmd-list" role="listbox" aria-label={t('digi.edit.subset')}>
              {commands.map((c) => (
                <li key={c.tag}>
                  <button type="button" role="option" aria-selected={picked === c.tag} className={picked === c.tag ? 'on' : ''} onClick={() => setPicked(c.tag)} onDoubleClick={() => { setPicked(c.tag); window.setTimeout(insert, 0); }}>
                    <b>{c.tag}</b> — {c.desc[idx]}{c.soon ? ` (${t('digi.edit.soon')})` : ''}
                  </button>
                </li>
              ))}
            </ul>
            <button type="button" className="glass sstv-btn" disabled={picked === ''} onClick={insert}>
              {t('digi.edit.insert')}
            </button>
          </div>
        </div>
        <footer>
          <button type="button" className="glass sstv-btn" onClick={() => { setLabel(''); setText(''); setColor(null); }}>
            {t('digi.edit.clear')}
          </button>
          <span style={{ flex: 1 }} />
          <button type="button" className="glass sstv-btn" onClick={onCancel}>
            {t('digi.edit.cancel')}
          </button>
          <button type="button" className="glass sstv-btn sstv-btn--primary" onClick={() => onSave({ name, label, text, color }, where)}>
            {t('digi.edit.ok')}
          </button>
        </footer>
      </div>
    </div>
  );
}
