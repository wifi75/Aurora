import { useState } from 'react';
import { useT } from '../i18n';
import { useBandFilter } from '../lib/bandFilter';
import { BAND_NAMES } from '../lib/format';
import { ALL_PANELS, defaultLayout, readLayout, saveLayout, toggleHidden } from '../lib/layout';
import type { Key } from '../i18n';
import { browserStorage } from '../lib/theme';
import { Check, Help } from './common';
import { Actions, Cells, Group, Notes, Page } from './layout';

/** Scheda Finestre: ripristina la disposizione dei pannelli. */
export function WindowsTab() {
  const t = useT();
  const [note, setNote] = useState('');
  const [layout, setLayout] = useState(() => readLayout(browserStorage()));
  const hidden = layout.hidden ?? [];
  return (
    <Page>
      <Group title={t('win.shown')}>
        <Cells min={12}>
          {ALL_PANELS.map((id) => (
            <Check
              key={id}
              label={t(`panel.${id}` as Key)}
              hint={t('tip.win.shown')}
              checked={!hidden.includes(id)}
              onChange={() => {
                const next = toggleHidden(layout, id);
                saveLayout(browserStorage(), next);
                setLayout(next);
              }}
            />
          ))}
        </Cells>
      </Group>
      <Group title={t('tab.windows')}>
        <Actions>
          <button title={t('tip.ui.reset')}
            type="button"
            className="glass"
            onClick={() => {
              saveLayout(browserStorage(), defaultLayout());
              setLayout(defaultLayout());
              setNote(t('win.done'));
            }}
          >
            {t('win.reset')}
          </button>
        </Actions>
        <p className="live-note sx-message" role="status">
          {note}
        </p>
      </Group>
      <Notes span={2}>
        <Help>{t('win.help')}</Help>
      </Notes>
    </Page>
  );
}

/** Scheda Bande: quali bande compaiono nella barra della stazione (effetto immediato). */
export function BandsTab() {
  const t = useT();
  const bands = useBandFilter((s) => s.bands);
  const { toggle, reset } = useBandFilter.getState();
  return (
    <Page>
      <Group title={t('tab.bands')} span={3}>
        <div className="band-choice">
          {BAND_NAMES.map((name) => (
            <label key={name} className={`glass${bands.includes(name) ? ' glass--on' : ''}`}>
              <input title={t('tip.ui.bandShown')} type="checkbox" checked={bands.includes(name)} onChange={() => toggle(name)} aria-label={name} />
              <span>{name}</span>
            </label>
          ))}
        </div>
        <Actions>
          <button title={t('tip.ui.allBands')} type="button" className="glass" onClick={reset}>
            {t('bands.all')}
          </button>
        </Actions>
      </Group>
      <Notes span={3}>
        <Help>{t('bands.help')}</Help>
      </Notes>
    </Page>
  );
}
