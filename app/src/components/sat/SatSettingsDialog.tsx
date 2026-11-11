import { useState } from 'react';
import type { SatSettings, SatSnapshot } from '../../api/client';
import { useT } from '../../i18n';
import { countdown } from '../../lib/satGeo';

/** Gli aggiornamenti automatici proposti, in ore (0 = solo a mano). */
const HOURS = [0, 6, 12, 24, 48, 168];

type Category = 'orbits' | 'map' | 'radio';
const CATEGORIES: Category[] = ['orbits', 'map', 'radio'];

interface Props {
  snap: SatSnapshot;
  busy: boolean;
  onSave(patch: Partial<SatSettings>): void;
  onRefresh(): void;
  onClose(): void;
}

/** Le impostazioni dei Satelliti, a categorie: dati delle orbite (server e aggiornamento), mappa, radio e rotore. */
export function SatSettingsDialog({ snap, busy, onSave, onRefresh, onClose }: Props) {
  const t = useT();
  const [cat, setCat] = useState<Category>('orbits');
  const s = snap.settings;
  const label: Record<Category, string> = { orbits: t('sat.set.catOrbits'), map: t('sat.set.catMap'), radio: t('sat.set.catRadio') };

  return (
    <div className="sat-modal" role="dialog" aria-modal="true" aria-label={t('sat.set.title')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <div className="sat-dialog">
        <header>
          <strong>{t('sat.set.title')}</strong>
          <button type="button" className="glass sstv-btn" onClick={onClose} aria-label={t('sat.set.close')}>
            ✕
          </button>
        </header>
        <div className="sat-set-body">
          <nav className="sat-set-cats" role="tablist" aria-orientation="vertical">
            {CATEGORIES.map((c) => (
              <button key={c} type="button" role="tab" aria-selected={cat === c} className={cat === c ? 'on' : ''} onClick={() => setCat(c)}>
                {label[c]}
              </button>
            ))}
          </nav>
          <div className="sat-set-pane" role="tabpanel">
            {cat === 'orbits' ? <OrbitsPane snap={snap} busy={busy} onSave={onSave} onRefresh={onRefresh} /> : null}
            {cat === 'map' ? (
              <fieldset>
                <legend>{t('sat.map.pick')}</legend>
                {snap.maps.length === 0 ? (
                  <p className="notice">{t('sat.map.none')}</p>
                ) : (
                  <select value={s.map && snap.maps.includes(s.map) ? s.map : ''} onChange={(e) => onSave({ map: e.target.value })}>
                    <option value="">{t('sat.set.mapAuto')}</option>
                    {snap.maps.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </select>
                )}
                <p className="sat-muted sat-small">{t('sat.map.addHint')}</p>
              </fieldset>
            ) : null}
            {cat === 'radio' ? (
              <fieldset>
                <legend>{t('sat.ctl.title')}</legend>
                <label className="sat-check">
                  <input type="checkbox" checked={s.doppler_on} onChange={(e) => onSave({ doppler_on: e.target.checked })} />
                  <span>{t('sat.ctl.doppler')}</span>
                </label>
                <label className="sat-check">
                  <input type="checkbox" checked={s.rotor_on} onChange={(e) => onSave({ rotor_on: e.target.checked })} />
                  <span>{t('sat.ctl.rotor')}</span>
                </label>
                <label className="sat-check">
                  <input type="checkbox" checked={s.rotor_preposition} disabled={!s.rotor_on} onChange={(e) => onSave({ rotor_preposition: e.target.checked })} />
                  <span>{t('sat.ctl.prepoint')}</span>
                </label>
                <label className="sat-row">
                  <span>{t('sat.ctl.minEl')}</span>
                  <input type="number" min={0} max={60} value={s.min_el_deg} onChange={(e) => onSave({ min_el_deg: Math.max(0, Math.min(60, Number(e.target.value) || 0)) })} />
                </label>
              </fieldset>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}

function OrbitsPane({ snap, busy, onSave, onRefresh }: Omit<Props, 'onClose'>) {
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
    <>
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
                {h === 0 ? t('sat.set.manual') : h === 24 ? t('sat.set.day') : h % 24 === 0 ? t('sat.set.days', { n: h / 24 }) : t('sat.set.hours', { n: h })}
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
    </>
  );
}
