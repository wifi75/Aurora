import { useT, useLang } from '../i18n';
import { useAlertPrefs } from '../lib/alerts';
import { gateOn, AUTO_SOUNDS, EXTRA_EVENTS, eventNameKey, useSoundAssign, type SoundEvent } from '../lib/soundAssign';
import { SOUNDS, SOUND_CATEGORIES, categoryName, previewSound, soundName, stopAll } from '../lib/soundBank';
import { useNotify } from '../lib/notify';
import { useNotifyView } from '../lib/notifyView';
import { Help } from './common';
import { Actions, Cell, Group, Message } from './layout';

/** Il menu «Suono» di una funzione, con i suoni raggruppati per categoria e «Nessuno» di base; e il ▶ «Prova». Piccolo, da mettere in una riga. */
export function SoundPicker({ event, label }: { event: SoundEvent; label?: string }) {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const id = useSoundAssign((s) => s.assign[event]);
  const volume = useAlertPrefs((s) => s.prefs.volume);
  const name = label ?? t(eventNameKey(event));
  const current = id ? soundName(id, lang) : t('snd.none');
  return (
    <span className="snd-pick" title={t('tip.snd.pick')}>
      <select
        className="snd-select"
        aria-label={t('snd.choose', { name })}
        value={id ?? ''}
        onChange={(e) => useSoundAssign.getState().set(event, e.target.value === '' ? null : e.target.value)}
      >
        <option value="">{t('snd.none')}</option>
        {SOUND_CATEGORIES.map((c) => (
          <optgroup key={c.id} label={categoryName(c.id, lang)}>
            {SOUNDS.filter((s) => s.category === c.id).map((s) => (
              <option key={s.id} value={s.id}>
                {soundName(s.id, lang)}
              </option>
            ))}
          </optgroup>
        ))}
      </select>
      <button
        type="button"
        className="glass snd-try"
        title={t('tip.snd.try')}
        aria-label={t('snd.tryName', { name: `${name} - ${current}` })}
        disabled={!id}
        onClick={() => id && previewSound(id, volume)}
      >
        <span aria-hidden="true">▶</span>
      </button>
    </span>
  );
}

function StateCell({ event }: { event: SoundEvent }) {
  const t = useT();
  const id = useSoundAssign((s) => s.assign[event]);
  const sound = useNotify((s) => s.notify.sound);
  const view = useNotifyView((s) => s.view);
  const on = gateOn(event, sound, view);
  const key = !id ? 'snd.state.none' : on ? 'snd.state.ok' : 'snd.state.off';
  return (
    <span className={`snd-state snd-state--${!id ? 'none' : on ? 'ok' : 'off'}`} role="status">
      {t(key)}
    </span>
  );
}

/** Il riquadro generale dei suoni: volume, gli avvisi che non sono un tipo di messaggio, assegnazione automatica e banco da provare. */
export function SoundsGroup() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const prefs = useAlertPrefs((s) => s.prefs);
  const volume = prefs.volume;
  return (
    <>
      <Group title={t('snd.title')} span={3}>
        <div className="snd-top">
          <Cell label={t('snd.volume')}>
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              aria-label={t('snd.volume')}
              value={volume}
              onChange={(e) => useAlertPrefs.getState().set({ ...prefs, volume: Number(e.target.value) })}
            />
            <span className="muted">{Math.round(volume * 100)}%</span>
          </Cell>
          <Actions>
            <button type="button" className="glass" onClick={() => useSoundAssign.getState().setAll({ ...AUTO_SOUNDS })}>
              {t('snd.auto')}
            </button>
            <button type="button" className="glass" onClick={() => useSoundAssign.getState().clear()}>
              {t('snd.clear')}
            </button>
            <button type="button" className="glass" onClick={() => stopAll()}>
              {t('snd.stop')}
            </button>
          </Actions>
        </div>
        {prefs.enabled ? null : <Message kind="muted">{t('snd.off')}</Message>}
        <div className="sx-tablewrap">
          <table className="sx-table snd-table">
            <colgroup>
              <col style={{ width: '40%' }} />
              <col />
              <col style={{ width: '20%' }} />
            </colgroup>
            <caption className="visually-hidden">{t('snd.extra')}</caption>
            <thead>
              <tr>
                <th scope="col">{t('snd.col.event')}</th>
                <th scope="col">{t('snd.pick')}</th>
                <th scope="col">{t('snd.col.state')}</th>
              </tr>
            </thead>
            <tbody>
              {EXTRA_EVENTS.map((e) => (
                <tr key={e}>
                  <th scope="row">{t(eventNameKey(e))}</th>
                  <td>
                    <SoundPicker event={e} />
                  </td>
                  <td>
                    <StateCell event={e} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <details className="snd-bank">
          <summary>{t('snd.bank', { n: SOUNDS.length })}</summary>
          {SOUND_CATEGORIES.map((c) => (
            <div key={c.id} className="snd-cat">
              <h4>{categoryName(c.id, lang)}</h4>
              <div className="snd-grid">
                {SOUNDS.filter((s) => s.category === c.id).map((s) => (
                  <button key={s.id} type="button" className="glass" onClick={() => previewSound(s.id, volume)}>
                    <span aria-hidden="true">▶ </span>
                    {soundName(s.id, lang)}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </details>
      </Group>
    </>
  );
}

/** Il testo di aiuto dei suoni, da mettere nelle note della scheda. */
export function SoundsHelp() {
  const t = useT();
  return <Help>{t('snd.help')}</Help>;
}
