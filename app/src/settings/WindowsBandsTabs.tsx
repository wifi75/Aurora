import { useState } from 'react';
import { useT } from '../i18n';
import { useBandFilter } from '../lib/bandFilter';
import { BAND_NAMES } from '../lib/format';
import { defaultLayout, saveLayout } from '../lib/layout';
import { browserStorage } from '../lib/theme';
import { Help } from './common';
import { attachAll } from '../lib/detached';
import { WhereGroups } from './WhereTab';
import { Actions, Group, Notes, Page } from './layout';

/** Scheda Finestre: ripristina la disposizione dei pannelli. */
export function WindowsTab() {
  const t = useT();
  const [note, setNote] = useState('');
  return (
    <Page>
      <WhereGroups />
      <Group title={t('tab.windows')}>
        <Actions>
          <button title={t('tip.ui.reset')}
            type="button"
            className="glass"
            onClick={() => {
              // tutto come all'inizio: disposizione di partenza, niente nascosto e niente in finestre a parte
              saveLayout(browserStorage(), defaultLayout());
              attachAll();
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
