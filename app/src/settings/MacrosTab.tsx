import { useState } from 'react';
import { useT } from '../i18n';
import { useMacros } from '../lib/macros';
import { Help } from './common';
import { Actions, Group, Message, Notes, Page } from './layout';

/** Scheda Macro TX: l'elenco delle macro (@ = DX, & = io, # = rapporto) che compaiono nel pannello Trasmissione. */
export function MacrosTab() {
  const t = useT();
  const macros = useMacros((s) => s.macros);
  const [text, setText] = useState('');
  return (
    <Page>
      <Group title={t('tab.macros')} span={2}>
        {macros.length === 0 ? <Message kind="muted">{t('macro.empty')}</Message> : null}
        <ul className="sx-list macro-list">
          {macros.map((m) => (
            <li key={m}>
              <span className="data">{m}</span>
              <button type="button" className="glass" aria-label={`${t('set.remove')} ${m}`} onClick={() => useMacros.getState().remove(m)}>
                {t('set.remove')}
              </button>
            </li>
          ))}
        </ul>
      </Group>
      <Group title={t('set.add')}>
        <div className="set-add">
          <input type="text" aria-label={t('macro.new')} placeholder={t('macro.new')} maxLength={30} value={text} onChange={(e) => setText(e.target.value)} />
        </div>
        <Actions>
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
          <button type="button" className="glass" onClick={() => useMacros.getState().reset()}>
            {t('set.reset')}
          </button>
        </Actions>
      </Group>
      <Notes>
        <Help>{t('macro.legend')}</Help>
      </Notes>
    </Page>
  );
}
