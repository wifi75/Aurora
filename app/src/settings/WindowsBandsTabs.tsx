import { useState } from 'react';
import { useT } from '../i18n';
import { useBandFilter } from '../lib/bandFilter';
import { BAND_NAMES } from '../lib/format';
import { defaultLayout, saveLayout } from '../lib/layout';
import { browserStorage } from '../lib/theme';
import { Help } from './common';

/** Scheda Finestre: ripristina la disposizione dei pannelli. */
export function WindowsTab() {
  const t = useT();
  const [note, setNote] = useState('');
  return (
    <>
      <Help>{t('win.help')}</Help>
      <button
        type="button"
        className="glass"
        onClick={() => {
          saveLayout(browserStorage(), defaultLayout());
          setNote(t('win.done'));
        }}
      >
        {t('win.reset')}
      </button>
      <p className="live-note" role="status">
        {note}
      </p>
    </>
  );
}

/** Scheda Bande: quali bande compaiono nella barra della stazione (effetto immediato). */
export function BandsTab() {
  const t = useT();
  const bands = useBandFilter((s) => s.bands);
  const { toggle, reset } = useBandFilter.getState();
  return (
    <>
      <Help>{t('bands.help')}</Help>
      <div className="band-choice">
        {BAND_NAMES.map((name) => (
          <label key={name} className={`glass${bands.includes(name) ? ' glass--on' : ''}`}>
            <input type="checkbox" checked={bands.includes(name)} onChange={() => toggle(name)} aria-label={name} />
            <span>{name}</span>
          </label>
        ))}
      </div>
      <button type="button" className="glass" onClick={reset}>
        {t('bands.all')}
      </button>
    </>
  );
}
