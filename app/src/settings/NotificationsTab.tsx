import { useState } from 'react';
import { AlertSwitch } from '../components/AlertSwitch';
import { useT } from '../i18n';
import { readAlerts, saveAlerts, type AlertPrefs } from '../lib/alerts';
import { COLOR_KEYS, applyColors, defaultColors, kindKey, readColors, saveColors, type TagColors } from '../lib/colors';
import { useNotify } from '../lib/notify';
import { browserStorage } from '../lib/theme';
import { JtdxValues } from './JtdxValues';
import { Help } from './common';

/** Scheda Notifiche: per ogni tipo di messaggio colore, evidenziazione e suono, come in JTDX. */
export function NotificationsTab() {
  const t = useT();
  const [colors, setColors] = useState<TagColors>(() => readColors(browserStorage()));
  const [alerts, setAlerts] = useState<AlertPrefs>(() => readAlerts(browserStorage()));
  const notify = useNotify((s) => s.notify);

  const commit = (next: TagColors) => {
    setColors(next);
    saveColors(browserStorage(), next);
    applyColors(document.documentElement, next);
  };

  return (
    <>
      <Help>{t('not.intro')}</Help>
      <AlertSwitch
        value={alerts}
        onChange={(next) => {
          saveAlerts(browserStorage(), next);
          setAlerts(next);
        }}
      />
      <Help>{t('not.soundHelp')}</Help>
      <table className="freq-table notify-table">
        <thead>
          <tr>
            <th>{t('not.kind')}</th>
            <th>{t('not.color')}</th>
            <th>{t('not.highlight')}</th>
            <th>{t('not.sound')}</th>
          </tr>
        </thead>
        <tbody>
          {COLOR_KEYS.map((key) => {
            const name = t(kindKey(key));
            return (
              <tr key={key}>
                <td>
                  <span className="color-chip" style={{ background: colors[key], color: colors[key] }} aria-hidden="true" /> {name}
                </td>
                <td title={t('tip.not.color')}>
                  <input type="color" aria-label={name} value={colors[key]} onChange={(e) => commit({ ...colors, [key]: e.target.value })} />
                </td>
                <td title={t('tip.not.highlight')}>
                  <input
                    type="checkbox"
                    aria-label={`${t('not.highlight')}: ${name}`}
                    checked={notify.highlight[key]}
                    onChange={(e) => useNotify.getState().setFlag('highlight', key, e.target.checked)}
                  />
                </td>
                <td title={t('tip.not.sound')}>
                  <input
                    type="checkbox"
                    aria-label={`${t('not.sound')}: ${name}`}
                    checked={notify.sound[key]}
                    onChange={(e) => useNotify.getState().setFlag('sound', key, e.target.checked)}
                  />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
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
      <JtdxValues group="notifications" />
    </>
  );
}
