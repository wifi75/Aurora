import { useState } from 'react';
import { AlertSwitch } from '../components/AlertSwitch';
import { useT } from '../i18n';
import { useAlertPrefs } from '../lib/alerts';
import { armAudio } from '../lib/soundBank';
import { useSoundAssign } from '../lib/soundAssign';
import { COLOR_KEYS, applyColors, defaultColors, kindKey, readColors, saveColors, type TagColors } from '../lib/colors';
import { useNotify } from '../lib/notify';
import { browserStorage } from '../lib/theme';
import { EngineValues } from './EngineValues';
import { NativeNotifications } from './JtdxTabs';
import { useIsNative } from './NativeSettings';
import { Help } from './common';
import { SoundPicker, SoundsGroup, SoundsHelp } from './SoundControls';
import { Actions, Cell, Group, Notes, Page } from './layout';

/** Scheda Notifiche: con il motore proprio come nel motore (evidenziazioni, gia' lavorati, colori e marcatori); con JTDX colore, evidenziazione e suono per tipo. */
export function NotificationsTab() {
  const native = useIsNative();
  return native ? <NativeNotifications /> : <JtdxNotificationsTab />;
}

function JtdxNotificationsTab() {
  const t = useT();
  const [colors, setColors] = useState<TagColors>(() => readColors(browserStorage()));
  const alerts = useAlertPrefs((s) => s.prefs);
  const notify = useNotify((s) => s.notify);

  const commit = (next: TagColors) => {
    setColors(next);
    saveColors(browserStorage(), next);
    applyColors(document.documentElement, next);
  };

  return (
    <Page>
      <Group title={t('not2.grpAlerts')}>
        <Cell label={t('alert.button')}>
          <AlertSwitch
            value={alerts}
            onChange={(next) => {
              if (next.enabled) armAudio();
              useAlertPrefs.getState().set(next);
            }}
          />
        </Cell>
        <Actions>
          <button
            type="button"
            className="glass"
            onClick={() => {
              commit(defaultColors());
              useNotify.getState().reset();
            }}
          >
            {t('not.restoreColors')}
          </button>
        </Actions>
      </Group>
      <Group title={t('not2.grpTable')} span={2}>
        <div className="sx-tablewrap">
          <table className="sx-table notify-table">
            <thead>
              <tr>
                <th scope="col">{t('not.kind')}</th>
                <th scope="col">{t('not.color')}</th>
                <th scope="col">{t('not.highlight')}</th>
                <th scope="col">{t('not.sound')}</th>
              </tr>
            </thead>
            <tbody>
              {COLOR_KEYS.map((key) => {
                const name = t(kindKey(key));
                return (
                  <tr key={key}>
                    <th scope="row">
                      <span className="color-chip" style={{ background: colors[key], color: colors[key] }} aria-hidden="true" /> {name}
                    </th>
                    <td title={t('tip.not.color')}>
                      <span className="sx-cellbox">
                        <input type="color" aria-label={name} value={colors[key]} onChange={(e) => commit({ ...colors, [key]: e.target.value })} />
                      </span>
                    </td>
                    <td title={t('tip.not.highlight')}>
                      <span className="sx-cellbox">
                        <input
                          type="checkbox"
                          aria-label={`${t('not.highlight')}: ${name}`}
                          checked={notify.highlight[key]}
                          onChange={(e) => useNotify.getState().setFlag('highlight', key, e.target.checked)}
                        />
                      </span>
                    </td>
                    <td title={t('tip.not.sound')}>
                      <span className="sx-cellbox">
                        <input
                          type="checkbox"
                          aria-label={`${t('not.sound')}: ${name}`}
                          checked={notify.sound[key]}
                          onChange={(e) => {
                            useNotify.getState().setFlag('sound', key, e.target.checked);
                            if (e.target.checked && key !== 'auto') useSoundAssign.getState().ensure([key]);
                          }}
                        />
                        {key !== 'auto' ? <SoundPicker event={key} label={name} /> : null}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </Group>
      <Group title={t('jx.title')} span={3}>
        <EngineValues group="notifications" />
      </Group>
      <Notes span={3}>
        <Help>{t('not.intro')}</Help>
        <Help>{t('not.soundHelp')}</Help>
        <SoundsHelp />
      </Notes>
      <SoundsGroup />
    </Page>
  );
}
