import { useEffect, useRef, useState } from 'react';
import { putWefax, wefaxRx, type WefaxSettings, type WefaxSnapshot } from '../../api/client';
import { useT } from '../../i18n';

const SLANT_STEPS = [-10, -1, 1, 10] as const;

/** Un campo numerico che si conferma uscendo dal campo o con Invio (non a ogni tasto: ogni conferma e' una scrittura nel motore). */
function NumberField({ label, value, min, max, onCommit }: { label: string; value: number; min: number; max: number; onCommit: (v: number) => void }) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const commit = () => {
    const v = Number(draft);
    if (!Number.isFinite(v)) {
      setDraft(String(value));
      return;
    }
    const clamped = Math.min(max, Math.max(min, v));
    setDraft(String(clamped));
    if (clamped !== value) onCommit(clamped);
  };
  return (
    <label className="sstv-field">
      {label}
      <input type="number" inputMode="numeric" value={draft} min={min} max={max} onChange={(e) => setDraft(e.target.value)} onBlur={commit} onKeyDown={(e) => e.key === 'Enter' && commit()} />
    </label>
  );
}

/** Indicatori e comandi della ricezione WEFAX: ascolto, partenza a mano, chiusura, impostazioni del decodificatore. */
export function Controls({ snap, onChanged, onListen }: { snap: WefaxSnapshot; onChanged: () => void; onListen?: (on: boolean) => void }) {
  const t = useT();
  const { rx, settings } = snap;
  // le impostazioni che si cambiano a raffica (correzione dell'inclinazione) si sommano su quelle appena inviate, non su quelle lette un secondo fa
  const latest = useRef<WefaxSettings>(settings);
  latest.current = settings;
  const save = (patch: Partial<WefaxSettings>) => {
    latest.current = { ...latest.current, ...patch };
    putWefax(latest.current).then(onChanged, onChanged);
  };
  const act = (fn: () => Promise<void>) => () => {
    fn().then(onChanged, onChanged);
  };

  const phase = rx.receiving ? t(`wefax.phase.${rx.state === 'apt' || rx.state === 'phasing' ? rx.state : 'image'}`) : t('wefax.phase.idle');
  const syncPct = Math.round(Math.max(0, Math.min(1, rx.sync || 0)) * 100);
  const level01 = Math.max(0, Math.min(100, ((rx.level_db + 60) / 60) * 100));
  const lpmKey = settings.ioc === 288 ? 'lpm_288' : 'lpm_576';
  const lpm = settings[lpmKey];
  const lpmChoices = snap.lpm_values.includes(lpm) ? snap.lpm_values : [...snap.lpm_values, lpm];
  const active = snap.rx.receiving;

  return (
    <aside className="sstv-side wefax-side">
      <dl className="sstv-readouts">
        <div>
          <dt>{t('wefax.ro.state')}</dt>
          <dd className="sstv-big">{phase}</dd>
        </div>
        <div>
          <dt>{t('wefax.ro.rows')}</dt>
          <dd>
            <b>{rx.receiving || rx.rows > 0 ? `${rx.rows}` : '—'}</b>
            {rx.width > 0 ? <span className="sstv-hint">{`${t('wefax.ro.size')}: ${rx.width} px`}</span> : null}
          </dd>
        </div>
        <div>
          <dt>{t('wefax.ro.ioc')}</dt>
          <dd>
            <b>{rx.ioc || settings.ioc}</b>
            <span className="sstv-hint">{`${t('wefax.ro.lpm')}: ${Math.round(rx.lpm || lpm)}`}</span>
          </dd>
        </div>
        <div>
          <dt>{t('wefax.ro.sync')}</dt>
          <dd>
            <span className={`sstv-meter sstv-meter--${syncPct > 60 ? 'ok' : syncPct > 25 ? 'warn' : 'bad'}`}>
              <span style={{ width: `${syncPct}%` }} />
            </span>
            <b>{rx.receiving ? `${syncPct} %` : '—'}</b>
          </dd>
        </div>
        <div>
          <dt>{t('wefax.ro.level')}</dt>
          <dd>
            <span className="sstv-meter">
              <span style={{ width: `${level01}%` }} />
            </span>
            <b>{rx.listening ? `${Math.round(rx.level_db)} dB` : '—'}</b>
          </dd>
        </div>
      </dl>
      {(rx.dropped_blocks ?? 0) > 0 ? (
        <p className="wefax-dropped" role="status">
          <b>{t('wefax.dropped', { n: String(rx.dropped_blocks) })}</b>
          <span>{t('wefax.droppedTip')}</span>
        </p>
      ) : null}

      <div className="sstv-actions">
        {rx.listening ? (
          <button
            type="button"
            className="glass sstv-btn"
            onClick={act(() => {
              onListen?.(false);
              return wefaxRx('stop');
            })}
          >
            {t('wefax.rx.stop')}
          </button>
        ) : (
          <button
            type="button"
            className="glass sstv-btn sstv-btn--primary"
            onClick={act(() => {
              onListen?.(true);
              return wefaxRx('start');
            })}
          >
            {t('wefax.rx.start')}
          </button>
        )}
        <button
          type="button"
          className="glass sstv-btn"
          disabled={!rx.listening || active}
          title={t('wefax.rx.forceHint')}
          onClick={act(() => {
            onListen?.(true);
            return wefaxRx('force');
          })}
        >
          {t('wefax.rx.force')}
        </button>
        <button type="button" className="glass sstv-btn" disabled={!active} onClick={act(() => wefaxRx('finish'))}>
          {t('wefax.rx.finish')}
        </button>
        <button type="button" className="glass sstv-btn sstv-btn--danger" disabled={!active} onClick={act(() => wefaxRx('cancel'))}>
          {t('wefax.rx.cancel')}
        </button>
      </div>
      <p className="sstv-hint">{t('wefax.rx.forceHint')}</p>

      <fieldset className="wefax-set">
        <legend>{t('wefax.set.title')}</legend>
        <div className="wefax-grid">
          <label className="sstv-field">
            {t('wefax.set.ioc')}
            <select value={settings.ioc} onChange={(e) => save({ ioc: Number(e.target.value) })}>
              {snap.iocs.map((i) => (
                <option key={i.ioc} value={i.ioc}>
                  {`${i.ioc} (${i.width} px)`}
                </option>
              ))}
            </select>
          </label>
          <label className="sstv-field">
            {t('wefax.set.lpm')}
            <select value={lpm} onChange={(e) => save({ [lpmKey]: Number(e.target.value) } as Partial<WefaxSettings>)}>
              {lpmChoices.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
          </label>
          <label className="sstv-field">
            {t('wefax.set.filter')}
            <select value={settings.filter} onChange={(e) => save({ filter: e.target.value })}>
              {snap.filters.map((f) => (
                <option key={f} value={f}>
                  {f === 'narrow' || f === 'medium' || f === 'wide' ? t(`wefax.set.filter.${f}`) : f}
                </option>
              ))}
            </select>
          </label>
          <NumberField label={t('wefax.set.center')} value={settings.center_hz} min={1000} max={3000} onCommit={(v) => save({ center_hz: v })} />
          <NumberField label={t('wefax.set.shift')} value={settings.shift_hz} min={200} max={1500} onCommit={(v) => save({ shift_hz: v })} />
        </div>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.manual} onChange={(e) => save({ manual: e.target.checked })} />
          {t('wefax.set.manual')}
        </label>
        <label className="sstv-check" title={t('wefax.set.autoIocTip')}>
          <input type="checkbox" checked={settings.auto_ioc ?? true} onChange={(e) => save({ auto_ioc: e.target.checked })} />
          {t('wefax.set.autoIoc')}
        </label>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.auto_lpm} onChange={(e) => save({ auto_lpm: e.target.checked })} />
          {t('wefax.set.autoLpm')}
        </label>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.auto_center} onChange={(e) => save({ auto_center: e.target.checked })} />
          {t('wefax.set.autoCenter')}
        </label>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.auto_save} onChange={(e) => save({ auto_save: e.target.checked })} />
          {t('wefax.set.autoSave')}
        </label>
        <label className="sstv-check" title={t('wefax.set.autoSlantTip')}>
          <input type="checkbox" checked={settings.auto_slant ?? true} onChange={(e) => save({ auto_slant: e.target.checked })} />
          {t('wefax.set.autoSlant')}
        </label>
        <div className="sstv-field">
          <span>
            {t('wefax.set.slant')}
            <b>{settings.slant_ppm.toFixed(0)}</b>
          </span>
          <span className="wefax-slant">
            {SLANT_STEPS.map((d) => (
              <button key={d} type="button" className="glass sstv-btn" onClick={() => save({ slant_ppm: Math.round((latest.current.slant_ppm + d) * 10) / 10 })}>
                {d > 0 ? `+${d}` : `${d}`}
              </button>
            ))}
            <button type="button" className="glass sstv-btn" disabled={settings.slant_ppm === 0} onClick={() => save({ slant_ppm: 0 })}>
              {t('wefax.set.slantReset')}
            </button>
          </span>
          <span className="sstv-hint">{t('wefax.set.slantHint')}</span>
          <span title={t('wefax.set.learnedHint')}>
            {t('wefax.set.learned')}
            <b>{Math.round(settings.learned_ppm ?? 0)}</b>
          </span>
          <span className="wefax-slant">
            <button type="button" className="glass sstv-btn" disabled={!settings.learned_ppm} onClick={act(() => wefaxRx('reset_learned'))}>
              {t('wefax.set.slantReset')}
            </button>
          </span>
        </div>
      </fieldset>
    </aside>
  );
}
