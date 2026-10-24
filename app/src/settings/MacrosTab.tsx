import { useState } from 'react';
import { useT } from '../i18n';
import { useMacros } from '../lib/macros';
import { Help } from './common';

/** Scheda Macro TX: l'elenco delle macro (@ = DX, & = io, # = rapporto) che compaiono nel pannello Trasmissione. */
export function MacrosTab() {
  const t = useT();
  const macros = useMacros((s) => s.macros);
  const [text, setText] = useState('');
  return (
    <>
      <Help>{t('macro.legend')}</Help>
      <div className="set-add">
        <input
          type="text"
          aria-label={t('macro.new')}
          placeholder={t('macro.new')}
          maxLength={30}
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <button
          type="button"
          className="glass"
          disabled={text.trim() === ''}
          onClick={() => {
            useMacros.getState().add(text);
            setText('');
          }}
        >
          {t('set.add')}
        </button>
      </div>
      {macros.length === 0 ? <p className="muted">{t('macro.empty')}</p> : null}
      <ul className="macro-list">
        {macros.map((m) => (
          <li key={m}>
            <span className="data">{m}</span>
            <button type="button" className="glass" aria-label={`${t('set.remove')} ${m}`} onClick={() => useMacros.getState().remove(m)}>
              {t('set.remove')}
            </button>
          </li>
        ))}
      </ul>
      <button type="button" className="glass" onClick={() => useMacros.getState().reset()}>
        {t('set.reset')}
      </button>
    </>
  );
}
