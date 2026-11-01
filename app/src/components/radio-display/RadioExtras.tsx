import { useQuery } from '@tanstack/react-query';
import { getCat } from '../../api/client';
import { useT } from '../../i18n';
import { agcName, formatVfo, useRadioExtras } from '../../lib/radioExtras';
import { useStation } from '../../state/station';

const pct = (v: number | null | undefined) => (v == null ? null : `${Math.round(v * 100)}`);

/** Una barra sottile da strumento, 0..1. */
function Bar({ value, tone }: { value: number; tone: string }) {
  return (
    <span className="rx-bar" aria-hidden="true">
      <i style={{ width: `${Math.round(Math.min(1, Math.max(0, value)) * 100)}%`, background: tone }} />
    </span>
  );
}

/**
 * Il display esteso sotto quello della radio: secondo VFO piu' piccolo, strumenti, filtri e impostazioni, ognuno solo se acceso
 * nelle impostazioni e se la radio lo da'. In ricezione lo S-meter, in trasmissione potenza, SWR e ALC.
 */
export function RadioExtras() {
  const t = useT();
  const prefs = useRadioExtras((s) => s.prefs);
  const status = useStation((s) => s.status);
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 });
  const rig = cat.data?.status.rig;
  const x = rig?.extras ?? {};
  const m = status?.meters ?? null;
  const tx = status?.transmitting ?? false;

  const chips: Array<{ key: string; text: string; on?: boolean }> = [];
  if (prefs.frontEnd) {
    if (x.preamp_db != null) chips.push({ key: 'pre', text: x.preamp_db > 0 ? `PRE ${Math.round(x.preamp_db)}` : 'PRE', on: x.preamp_db > 0 });
    if (x.att_db != null) chips.push({ key: 'att', text: x.att_db > 0 ? `ATT ${Math.round(x.att_db)}` : 'ATT', on: x.att_db > 0 });
  }
  if (prefs.agc && x.agc != null) chips.push({ key: 'agc', text: `AGC ${agcName(x.agc)}`, on: x.agc !== 0 });
  if (prefs.noise) {
    if (x.nb != null) chips.push({ key: 'nb', text: 'NB', on: x.nb });
    if (x.nr != null) chips.push({ key: 'nr', text: 'NR', on: x.nr });
    if (x.notch != null) chips.push({ key: 'notch', text: 'NOTCH', on: x.notch });
  }
  if (prefs.tuner && x.tuner != null) chips.push({ key: 'atu', text: 'ATU', on: x.tuner });
  if (prefs.ritXit) {
    if (x.rit_hz != null) chips.push({ key: 'rit', text: `RIT ${x.rit_hz > 0 ? '+' : ''}${x.rit_hz}`, on: x.rit_hz !== 0 });
    if (x.xit_hz != null) chips.push({ key: 'xit', text: `XIT ${x.xit_hz > 0 ? '+' : ''}${x.xit_hz}`, on: x.xit_hz !== 0 });
  }
  if (prefs.vfo && x.vfo) chips.push({ key: 'vfo', text: x.vfo.replace(/^VFO/, 'VFO ') });

  const gains = prefs.gains
    ? [
        ['RF', pct(x.rf_gain)],
        ['AF', pct(x.af_gain)],
        ['SQL', pct(x.squelch)],
        ['PWR', pct(x.power_set)],
        ['MIC', pct(x.mic_gain)],
      ].filter((g): g is [string, string] => g[1] !== null)
    : [];

  const meters: Array<{ key: string; label: string; value: number; tone: string }> = [];
  if (prefs.meters && m) {
    if (!tx && m.s_points != null) meters.push({ key: 's', label: m.s_over_db ? `S9+${Math.round(m.s_over_db)}` : `S${Math.round(m.s_points)}`, value: (m.s_points + (m.s_over_db ?? 0) / 10) / 15, tone: '#5dcaa5' });
    if (tx && m.power_w != null) meters.push({ key: 'po', label: `PO ${Math.round(m.power_w)} W`, value: m.power_w / 100, tone: '#ef9f27' });
    if (tx && m.swr != null) meters.push({ key: 'swr', label: `SWR ${m.swr.toFixed(1)}`, value: (m.swr - 1) / 2, tone: m.swr > 2 ? '#e24b4a' : '#5dcaa5' });
    if (tx && m.alc != null) meters.push({ key: 'alc', label: `ALC ${Math.round(m.alc * 100)}%`, value: m.alc, tone: '#85b7eb' });
  }
  const supply = prefs.supply && m && tx ? [m.vdd_v != null ? `VD ${m.vdd_v.toFixed(1)} V` : null, m.id_a != null ? `ID ${m.id_a.toFixed(1)} A` : null, m.comp_db != null ? `COMP ${Math.round(m.comp_db)} dB` : null].filter((v): v is string => v !== null) : [];
  const vfoB = prefs.vfoB && x.vfo_b_hz ? `${formatVfo(x.vfo_b_hz)}${x.vfo_b_mode ? ` ${x.vfo_b_mode}` : ''}` : null;

  if (!vfoB && meters.length === 0 && supply.length === 0 && chips.length === 0 && gains.length === 0) return null;
  return (
    <div className="rx-extras" aria-label={t('rx.grp')}>
      {vfoB ? (
        <div className="rx-vfob" title={t('tip.rx.vfoB')}>
          <span className="rx-vfob-tag">B</span>
          <span className="rx-vfob-freq">{vfoB}</span>
        </div>
      ) : null}
      {meters.length > 0 ? (
        <div className="rx-meters" title={t('tip.rx.meters')}>
          {meters.map((mt) => (
            <span key={mt.key} className="rx-meter">
              {mt.label}
              <Bar value={mt.value} tone={mt.tone} />
            </span>
          ))}
        </div>
      ) : null}
      {supply.length > 0 ? <div className="rx-line" title={t('tip.rx.supply')}>{supply.join(' · ')}</div> : null}
      {chips.length > 0 ? (
        <div className="rx-chips">
          {chips.map((c) => (
            <span key={c.key} className={`rx-chip${c.on ? ' rx-chip--on' : ''}`}>
              {c.text}
            </span>
          ))}
        </div>
      ) : null}
      {gains.length > 0 ? <div className="rx-line" title={t('tip.rx.gains')}>{gains.map(([k, v]) => `${k} ${v}`).join(' · ')}</div> : null}
    </div>
  );
}
