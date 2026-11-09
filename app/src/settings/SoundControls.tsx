import { useT, useLang } from '../i18n';
import { useAlertPrefs } from '../lib/alerts';
import { gateOn, AUTO_SOUNDS, EXTRA_EVENTS, eventNameKey, useSoundAssign, type SoundEvent } from '../lib/soundAssign';
import { SOUNDS, SOUND_CATEGORIES, categoryName, soundName, stopAll } from '../lib/soundBank';
import { FILE_PREFIX, VOICE_ID, addSoundFile, playAlert, removeSoundFile, setVoiceLang, useSoundFiles, useVoiceLang } from '../lib/voice';
import { toast } from '../lib/toast';
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
  const files = useSoundFiles();
  const name = label ?? t(eventNameKey(event));
  const current = id ? soundName(id, lang) : t('snd.none');
  const choose = (value: string) => {
    if (value === '__add') {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = 'audio/wav,audio/x-wav,audio/mpeg,audio/mp3,audio/ogg,.wav,.mp3,.ogg';
      input.onchange = () => {
        const f = input.files?.[0];
        if (!f) return;
        addSoundFile(f)
          .then((newId) => useSoundAssign.getState().set(event, newId))
          .catch(() => toast(t('snd.fileFail'), 'error'));
      };
      input.click();
      return;
    }
    useSoundAssign.getState().set(event, value === '' ? null : value);
  };
  return (
    <span className="snd-pick" title={t('tip.snd.pick')}>
      <select
        className="snd-select"
        aria-label={t('snd.choose', { name })}
        value={id ?? ''}
        onChange={(e) => choose(e.target.value)}
      >
        <option value="">{t('snd.none')}</option>
        <option value={VOICE_ID}>{soundName(VOICE_ID, lang)}</option>
        <optgroup label={t('snd.myFiles')}>
          {files.map((f) => (
            <option key={f} value={FILE_PREFIX + f}>
              {f}
            </option>
          ))}
          <option value="__add">{t('snd.addFile')}</option>
        </optgroup>
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
        onClick={() => id && playAlert(id, volume, { event, call: 'Z62NS' }, true)}
      >
        <span aria-hidden="true">▶</span>
      </button>
      {id?.startsWith(FILE_PREFIX) ? (
        <button type="button" className="glass snd-try" title={t('snd.removeFile')} aria-label={t('snd.removeFile')} onClick={() => void removeSoundFile(id.slice(FILE_PREFIX.length)).then(() => useSoundAssign.getState().set(event, null))}>
          <span aria-hidden="true">×</span>
        </button>
      ) : null}
    </span>
  );
}

/** La lingua in cui parla la voce: quella di Aurora o sempre l'inglese. */
export function VoiceLangPick() {
  const t = useT();
  const v = useVoiceLang();
  return (
    <label className="sw-field" title={t('tip.snd.voiceLang')}>
      <span className="sw-label">{t('snd.voiceLang')}</span>
      <select value={v} onChange={(e) => setVoiceLang(e.target.value === 'en' ? 'en' : 'ui')}>
        <option value="ui">{t('snd.voiceLang.ui')}</option>
        <option value="en">{t('snd.voiceLang.en')}</option>
      </select>
    </label>
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
          <Cell label={t('snd.volume')} hint={t('tip.snd.volume')}>
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
          <VoiceLangPick />
          <Actions>
            <button title={t('tip.ui.soundAuto')} type="button" className="glass" onClick={() => useSoundAssign.getState().setAll({ ...AUTO_SOUNDS })}>
              {t('snd.auto')}
            </button>
            <button title={t('tip.ui.soundClear')} type="button" className="glass" onClick={() => useSoundAssign.getState().clear()}>
              {t('snd.clear')}
            </button>
            <button title={t('tip.ui.soundStop')} type="button" className="glass" onClick={() => stopAll()}>
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
                  <button title={t('tip.ui.soundPreview')} key={s.id} type="button" className="glass" onClick={() => playAlert(s.id, volume, {}, true)}>
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
