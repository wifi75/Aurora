import { useEffect, useState } from 'react';
import type { SatSettings, SatSnapshot } from '../../api/client';
import { useT } from '../../i18n';
import { toast } from '../../lib/toast';

/** I valori di fabbrica dell'inseguimento (prudenti: nessun parcheggio automatico, anticipo 60 s). */
export const TRACKING_DEFAULTS: Pick<
  SatSettings,
  'rotor_preposition' | 'rotor_lead_s' | 'rotor_start_el_deg' | 'rotor_interval_s' | 'rotor_min_move_deg' | 'min_el_deg' | 'rotor_after' | 'rotor_park_az_deg' | 'rotor_park_el_deg' | 'rotor_after_delay_s'
> = {
  rotor_preposition: true,
  rotor_lead_s: 60,
  rotor_start_el_deg: 0,
  rotor_interval_s: 2,
  rotor_min_move_deg: 2,
  min_el_deg: 0,
  rotor_after: 'stay',
  rotor_park_az_deg: 0,
  rotor_park_el_deg: 0,
  rotor_after_delay_s: 10,
};

/** Porta `v` dentro `min`..`max` e sul passo piu' vicino. */
export function clampStep(v: number, min: number, max: number, step: number): number {
  if (!Number.isFinite(v)) return min;
  const snapped = Math.round((v - min) / step) * step + min;
  const fixed = Number(snapped.toFixed(4));
  return Math.max(min, Math.min(max, fixed));
}

interface FieldProps {
  label: string;
  help: string;
  example: string;
  unit: string;
  value: number;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  onCommit(v: number): void;
}

/** Un valore regolabile con un cursore e con una casella numerica, collegati: muovi uno e cambia l'altro; un valore fuori limite va al piu' vicino. */
function NumSlider({ label, help, example, unit, value, min, max, step, disabled, onCommit }: FieldProps) {
  const t = useT();
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const shown = Number.isFinite(Number(draft)) && draft.trim() !== '' ? Number(draft) : value;
  const commit = (raw: number, warn: boolean) => {
    const v = clampStep(raw, min, max, step);
    if (warn && Number.isFinite(raw) && (raw < min || raw > max)) toast(t('sat.trk.clamped', { v, min, max }), 'info');
    setDraft(String(v));
    if (v !== value) onCommit(v);
  };
  return (
    <div className="sat-trk-field">
      <div className="sat-trk-head">
        <span className="sat-trk-label">{label}</span>
        <span className="sat-trk-num">
          <input
            type="number"
            min={min}
            max={max}
            step={step}
            value={draft}
            disabled={disabled}
            aria-label={label}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => commit(draft.trim() === '' ? value : Number(draft), true)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') commit(draft.trim() === '' ? value : Number(draft), true);
            }}
          />
          <span className="sat-muted">{unit}</span>
        </span>
      </div>
      <input
        type="range"
        className="sat-trk-range"
        min={min}
        max={max}
        step={step}
        value={Math.max(min, Math.min(max, shown))}
        disabled={disabled}
        aria-label={label}
        onChange={(e) => setDraft(e.target.value)}
        onPointerUp={() => commit(shown, false)}
        onKeyUp={() => commit(shown, false)}
        onBlur={() => commit(shown, false)}
      />
      <p className="sat-muted sat-small">{help}</p>
      <p className="sat-muted sat-small sat-trk-ex">{example}</p>
    </div>
  );
}

function clock(unix: number): string {
  return new Date(unix * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

/** Il riepilogo in una riga delle scelte fatte. */
export function trackingSummary(s: SatSettings, t: ReturnType<typeof useT>): string {
  const before = s.rotor_preposition && s.rotor_lead_s > 0 ? t('sat.trk.sumBefore', { n: s.rotor_lead_s }) : t('sat.trk.sumBeforeOff');
  const start = s.rotor_start_el_deg > 0 ? t('sat.trk.sumStart', { e: s.rotor_start_el_deg }) : '';
  const during = t('sat.trk.sumDuring', { i: s.rotor_interval_s, d: s.rotor_min_move_deg, start });
  const after =
    s.rotor_after === 'park'
      ? t('sat.trk.sumPark', { n: s.rotor_after_delay_s, az: s.rotor_park_az_deg, el: s.rotor_park_el_deg })
      : s.rotor_after === 'next'
        ? t('sat.trk.sumNext', { n: s.rotor_after_delay_s })
        : t('sat.trk.sumStay');
  return t('sat.trk.summary', { before, during, after });
}

/** L'anteprima in testo del prossimo passaggio del satellite scelto, con gli orari reali. */
export function trackingPreview(s: SatSettings, snap: SatSnapshot, t: ReturnType<typeof useT>): string {
  const p = snap.now.next_pass;
  if (!p) return t('sat.trk.nextNone');
  const lead = s.rotor_preposition && s.rotor_lead_s > 0 ? s.rotor_lead_s : 0;
  const after = s.rotor_after === 'park' ? t('sat.trk.nextPark', { az: s.rotor_park_az_deg, el: s.rotor_park_el_deg }) : s.rotor_after === 'next' ? t('sat.trk.nextNext') : '';
  const az = Math.round((p.aos_az_deg + s.rotor_az_offset_deg + 360) % 360);
  const name = snap.now.name || '';
  return lead > 0 ? t('sat.trk.next', { name, t: clock(p.aos_unix), tr: clock(p.aos_unix - lead), az, after }) : t('sat.trk.nextAos', { name, t: clock(p.aos_unix), az, after });
}

interface Props {
  snap: SatSnapshot;
  onSave(patch: Partial<SatSettings>): void;
}

/**
 * Impostazioni › Modi › Satelliti › Inseguimento e rotore: cosa fa il rotore prima, durante e dopo il passaggio. Ogni numero ha un cursore e una
 * casella collegati. L'avviso sonoro del rotore (pagina Satelliti) e questo movimento sono due cose separate.
 */
export function SatTrackingPane({ snap, onSave }: Props) {
  const t = useT();
  const s = snap.settings;
  const off = !s.rotor_on;
  const parked = s.rotor_after === 'park';
  const after = s.rotor_after === 'park' || s.rotor_after === 'next';
  return (
    <>
      <p className="sat-muted sat-small">{t('sat.trk.intro')}</p>
      <label className="sat-check">
        <input type="checkbox" checked={s.rotor_on} onChange={(e) => onSave({ rotor_on: e.target.checked })} />
        <span>{t('sat.trk.master')}</span>
      </label>
      <p className="sat-muted sat-small">{t('sat.trk.masterHelp')}</p>

      <div className="sat-trk-summary" aria-live="polite">
        <p>{trackingSummary(s, t)}</p>
        <p className="sat-muted">{trackingPreview(s, snap, t)}</p>
        <p className="sat-muted sat-small">{t('sat.trk.wear')}</p>
      </div>

      <fieldset className="sat-trk-block">
        <legend>{t('sat.trk.before')}</legend>
        <label className="sat-check">
          <input type="checkbox" checked={s.rotor_preposition} disabled={off} onChange={(e) => onSave({ rotor_preposition: e.target.checked })} />
          <span>{t('sat.trk.preOn')}</span>
        </label>
        <NumSlider label={t('sat.trk.lead')} help={t('sat.trk.leadHelp')} example={t('sat.trk.leadEx')} unit="s" value={s.rotor_lead_s} min={0} max={600} step={5} disabled={off || !s.rotor_preposition} onCommit={(v) => onSave({ rotor_lead_s: v })} />
      </fieldset>

      <fieldset className="sat-trk-block">
        <legend>{t('sat.trk.during')}</legend>
        <NumSlider label={t('sat.trk.minEl')} help={t('sat.trk.minElHelp')} example={t('sat.trk.minElEx')} unit="°" value={s.min_el_deg} min={0} max={60} step={1} disabled={off} onCommit={(v) => onSave({ min_el_deg: v })} />
        <NumSlider label={t('sat.trk.startEl')} help={t('sat.trk.startElHelp')} example={t('sat.trk.startElEx')} unit="°" value={s.rotor_start_el_deg} min={0} max={30} step={1} disabled={off} onCommit={(v) => onSave({ rotor_start_el_deg: v })} />
        <NumSlider label={t('sat.trk.interval')} help={t('sat.trk.intervalHelp')} example={t('sat.trk.intervalEx')} unit="s" value={s.rotor_interval_s} min={1} max={10} step={1} disabled={off} onCommit={(v) => onSave({ rotor_interval_s: v })} />
        <NumSlider label={t('sat.trk.tol')} help={t('sat.trk.tolHelp')} example={t('sat.trk.tolEx')} unit="°" value={Math.min(s.rotor_min_move_deg, 10)} min={0} max={10} step={0.5} disabled={off} onCommit={(v) => onSave({ rotor_min_move_deg: v })} />
      </fieldset>

      <fieldset className="sat-trk-block">
        <legend>{t('sat.trk.after')}</legend>
        <p className="sat-muted sat-small">{t('sat.trk.afterHelp')}</p>
        {(['stay', 'park', 'next'] as const).map((k) => (
          <label key={k} className="sat-check">
            <input type="radio" name="sat-rotor-after" checked={s.rotor_after === k} disabled={off} onChange={() => onSave({ rotor_after: k })} />
            <span>{t(k === 'stay' ? 'sat.trk.afterStay' : k === 'park' ? 'sat.trk.afterPark' : 'sat.trk.afterNext')}</span>
          </label>
        ))}
        <NumSlider label={t('sat.trk.parkAz')} help={t('sat.trk.parkAzHelp')} example={t('sat.trk.parkAzEx')} unit="°" value={s.rotor_park_az_deg} min={0} max={360} step={1} disabled={off || !parked} onCommit={(v) => onSave({ rotor_park_az_deg: v })} />
        <NumSlider label={t('sat.trk.parkEl')} help={t('sat.trk.parkElHelp')} example={t('sat.trk.parkElEx')} unit="°" value={s.rotor_park_el_deg} min={0} max={90} step={1} disabled={off || !parked} onCommit={(v) => onSave({ rotor_park_el_deg: v })} />
        <NumSlider label={t('sat.trk.delay')} help={t('sat.trk.delayHelp')} example={t('sat.trk.delayEx')} unit="s" value={s.rotor_after_delay_s} min={0} max={300} step={5} disabled={off || !after} onCommit={(v) => onSave({ rotor_after_delay_s: v })} />
      </fieldset>

      <div className="sat-row">
        <button type="button" className="glass sstv-btn" onClick={() => onSave({ ...TRACKING_DEFAULTS })}>
          {t('sat.trk.reset')}
        </button>
      </div>
    </>
  );
}
