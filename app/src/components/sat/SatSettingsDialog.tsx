import { useState } from 'react';
import type { SatSettings, SatSnapshot } from '../../api/client';
import { useT } from '../../i18n';
import { countdown } from '../../lib/satGeo';

/** Gli aggiornamenti automatici proposti, in ore (0 = solo a mano). */
const HOURS = [0, 6, 12, 24, 48, 168];

interface Props {
  snap: SatSnapshot;
  busy: boolean;
  onSave(patch: Partial<SatSettings>): void;
  onRefresh(): void;
  onClose(): void;
}

/** Le impostazioni dei Satelliti: da quali server si scaricano i dati delle orbite e ogni quanto si aggiornano da soli. */
export function SatSettingsDialog({ snap, busy, onSave, onRefresh, onClose }: Props) {
  const t = useT();
  const s = snap.settings;
  const presets = snap.tle.presets ?? [];
  const known = new Set(presets.map((p) => p.url));
  const custom = s.tle_sources.filter((u) => !known.has(u));
  const [url, setUrl] = useState('');
  const valid = /^https?:\/\/\S+$/i.test(url.trim());
  const toggle = (u: string, on: boolean) => onSave({ tle_sources: on ? [...s.tle_sources, u] : s.tle_sources.filter((x) => x !== u) });
  const hours = HOURS.includes(s.tle_update_hours) ? HOURS : [...HOURS, s.tle_update_hours].sort((a, b) => a - b);
  const age = snap.tle.age_s === null ? t('sat.tle.none') : t('sat.tle.age', { t: countdown(snap.tle.age_s) });

  return (
    <div className="sat-modal" role="dialog" aria-modal="true" aria-label={t('sat.set.title')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <div className="sat-dialog">
        <header>
          <strong>{t('sat.set.title')}</strong>
          <button type="button" className="glass sstv-btn" onClick={onClose} aria-label={t('sat.set.close')}>
            ✕
          </button>
        </header>

        <fieldset>
          <legend>{t('sat.set.servers')}</legend>
          <p className="sat-muted sat-small">{t('sat.set.serversHint')}</p>
          {presets.map((p) => (
            <label key={p.url} className="sat-check">
              <input type="checkbox" checked={s.tle_sources.includes(p.url)} onChange={(e) => toggle(p.url, e.target.checked)} />
              <span>
                {p.name}
                <small className="sat-muted"> · {p.url.replace(/^https?:\/\//, '').split('/')[0]}</small>
              </span>
            </label>
          ))}
          {custom.map((u) => (
            <label key={u} className="sat-check">
              <input type="checkbox" checked onChange={() => toggle(u, false)} />
              <span className="sat-url">{u}</span>
            </label>
          ))}
          <div className="sat-add">
            <input type="url" value={url} placeholder={t('sat.set.addPlaceholder')} onChange={(e) => setUrl(e.target.value)} />
            <button
              type="button"
              className="glass sstv-btn"
              disabled={!valid || s.tle_sources.includes(url.trim())}
              onClick={() => {
                toggle(url.trim(), true);
                setUrl('');
              }}
            >
              {t('sat.set.add')}
            </button>
          </div>
          {s.tle_sources.length === 0 ? <p className="notice">{t('sat.set.noServer')}</p> : null}
        </fieldset>

        <fieldset>
          <legend>{t('sat.set.update')}</legend>
          <label className="sat-row">
            <span>{t('sat.set.every')}</span>
            <select value={s.tle_update_hours} onChange={(e) => onSave({ tle_update_hours: Number(e.target.value) })}>
              {hours.map((h) => (
                <option key={h} value={h}>
                  {h === 0 ? t('sat.set.manual') : h % 24 === 0 ? t('sat.set.days', { n: h / 24 }) : t('sat.set.hours', { n: h })}
                </option>
              ))}
            </select>
          </label>
          <div className="sat-row">
            <span className="sat-muted">{age}</span>
            <button type="button" className="glass sstv-btn" disabled={busy || s.tle_sources.length === 0} onClick={onRefresh}>
              {t('sat.tle.refresh')}
            </button>
          </div>
        </fieldset>
      </div>
    </div>
  );
}
