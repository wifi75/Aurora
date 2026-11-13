import { powerPct } from '../../lib/radioDisplay';
import { useState, useSyncExternalStore } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ApiError, getCat, postCatExtra, type CatExtraItem } from '../../api/client';
import { useT, type Key } from '../../i18n';
import { coreText } from '../../lib/coreText';
import { toast } from '../../lib/toast';
import { agcName, formatVfo, useRadioExtras } from '../../lib/radioExtras';
import { useStation } from '../../state/station';

/** Cosa fa il clic su ogni indicatore (suggerimento ed etichetta per gli screen reader). */
const ACTION_TIP = {
  preamp: 'rx.act.preamp',
  att: 'rx.act.att',
  agc: 'rx.act.agc',
  nb: 'rx.act.nb',
  nr: 'rx.act.nr',
  notch: 'rx.act.notch',
  atu: 'rx.act.atu',
  tune: 'rx.act.tune',
  pwr_up: 'rx.act.pwrUp',
  pwr_down: 'rx.act.pwrDown',
  vfo: 'rx.act.vfo',
  rit: 'rx.act.rit',
  xit: 'rx.act.xit',
} as const;

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
 * Lettura dei dati in piu' della radio e clic sugli indicatori, condivisi dal display esteso e dai display a pannello unico
 * (FT-2000, IC-7610): l'indicatore appena cliccato resta in attesa finche' la radio non ha risposto.
 */
/** Quanto resta accesa la scritta TX dopo «TUNE»: la radio accorda da sola (qualche secondo di portante) e il CAT non lo dice. */
export const TUNE_TX_MS = 6000;
const tuneTx = { until: 0, subs: new Set<() => void>() };

/** Segna l'accordatura appena avviata: il display accende TX per {@link TUNE_TX_MS}. */
export function markTuning(now = Date.now()) {
  tuneTx.until = now + TUNE_TX_MS;
  tuneTx.subs.forEach((f) => f());
  setTimeout(() => tuneTx.subs.forEach((f) => f()), TUNE_TX_MS + 50);
}

/** La radio sta accordando dopo un «TUNE» del display. */
export function useTuning(): boolean {
  return useSyncExternalStore(
    (f) => {
      tuneTx.subs.add(f);
      return () => tuneTx.subs.delete(f);
    },
    () => Date.now() < tuneTx.until,
  );
}

export function useCatExtras() {
  const t = useT();
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 });
  const queryClient = useQueryClient();
  const [waiting, setWaiting] = useState<CatExtraItem | null>(null);
  const click = async (item: CatExtraItem) => {
    if (waiting !== null) return;
    setWaiting(item);
    try {
      await postCatExtra(item);
      // l'accordatore della radio trasmette da solo: TX acceso sul display mentre accorda
      if (item === 'tune' && cat.data?.config.backend !== 'tci') markTuning();
    } catch (e) {
      const code = e instanceof ApiError ? e.code : '';
      const detail = e instanceof ApiError && e.detail ? coreText(e.detail, t) : undefined;
      if (code === 'local_only') toast(t('rx.err.local'), 'error');
      else if (code === 'transmitting') toast(t('rx.err.tx'), 'error');
      else if (code === 'unsupported') toast(detail ?? t('rx.err.unsupported'), 'error');
      else toast(detail ?? t('rx.err.generic'), 'error');
    } finally {
      setWaiting(null);
      await queryClient.invalidateQueries({ queryKey: ['cat'] });
    }
  };
  return { cat, rig: cat.data?.status.rig, waiting, click };
}

/** Suggerimento del clic su un indicatore. */
export function actionTip(item: CatExtraItem) {
  return ACTION_TIP[item];
}

/**
 * Il display esteso sotto quello della radio: secondo VFO piu' piccolo, strumenti, filtri e impostazioni, ognuno solo se acceso
 * nelle impostazioni e se la radio lo da'. In trasmissione SWR e ALC (S-meter e potenza sono gia' nel display).
 */
export function RadioExtras() {
  const t = useT();
  const prefs = useRadioExtras((s) => s.prefs);
  const status = useStation((s) => s.status);
  const { cat, rig, waiting, click } = useCatExtras();
  const x = rig?.extras ?? {};
  const m = status?.meters ?? null;
  const tx = status?.transmitting ?? false;

  const chips: Array<{ key: CatExtraItem; text: string; on?: boolean; tip?: Key }> = [];
  const tci = cat.data?.config.backend === 'tci';
  if (prefs.frontEnd) {
    if (x.preamp_db != null) chips.push({ key: 'preamp', text: x.preamp_db > 0 ? `PRE ${Math.round(x.preamp_db)}` : 'PRE', on: x.preamp_db > 0 });
    if (x.att_db != null) chips.push({ key: 'att', text: x.att_db > 0 ? `ATT ${Math.round(x.att_db)}` : 'ATT', on: x.att_db > 0 });
  }
  if (prefs.agc && x.agc != null) chips.push({ key: 'agc', text: `AGC ${agcName(x.agc)}`, on: x.agc !== 0 });
  if (prefs.noise) {
    if (x.nb != null) chips.push({ key: 'nb', text: 'NB', on: x.nb });
    if (x.nr != null) chips.push({ key: 'nr', text: 'NR', on: x.nr });
    if (x.notch != null) chips.push({ key: 'notch', text: 'NOTCH', on: x.notch });
  }
  if (prefs.tuner && x.tuner != null && tci) {
    // TCI: niente accordatore, TUNE accende e spegne la portante di accordo della SDR (acceso = portante in onda)
    chips.push({ key: 'tune', text: 'TUNE', on: x.tuner, tip: 'rx.act.tuneTci' });
  } else if (prefs.tuner && x.tuner != null) {
    chips.push({ key: 'atu', text: 'ATU', on: x.tuner });
    // avvia l'accordatura: la radio emette da sola una portante bassa per qualche secondo
    chips.push({ key: 'tune', text: 'TUNE', on: false });
  }
  // potenza RF impostata, a passi del 5%
  if (x.power_set != null) {
    chips.push({ key: 'pwr_down', text: `PWR ${powerPct(x.power_set)}% −`, on: false });
    chips.push({ key: 'pwr_up', text: '+', on: false });
  }
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
    // lo S-meter e la potenza ci sono gia' nel display in alto: qui solo cio' che il display non mostra
    if (tx && m.swr != null) meters.push({ key: 'swr', label: `SWR ${m.swr.toFixed(1)}`, value: (m.swr - 1) / 2, tone: m.swr > 2 ? '#e24b4a' : '#5dcaa5' });
    if (tx && m.alc != null) meters.push({ key: 'alc', label: `ALC ${Math.round(m.alc * 100)}%`, value: m.alc, tone: '#85b7eb' });
  }
  const supply = prefs.supply && m && tx ? [m.vdd_v != null ? `VD ${m.vdd_v.toFixed(1)} V` : null, m.id_a != null ? `ID ${m.id_a.toFixed(1)} A` : null, m.comp_db != null ? `COMP ${Math.round(m.comp_db)} dB` : null].filter((v): v is string => v !== null) : [];
  const vfoB = prefs.vfoB && x.vfo_b_hz ? `${formatVfo(x.vfo_b_hz)}${x.vfo_b_mode ? ` ${x.vfo_b_mode}` : ''}` : null;

  // le voci sono accese ma la radio non manda i dati in piu': lo si dice, invece di non mostrare niente
  const extrasOff = cat.data !== undefined && cat.data.config.read_extras !== true && (prefs.vfoB || prefs.frontEnd || prefs.agc || prefs.noise || prefs.gains || prefs.ritXit || prefs.tuner || prefs.vfo);
  if (!vfoB && meters.length === 0 && supply.length === 0 && chips.length === 0 && gains.length === 0 && !extrasOff) return null;
  return (
    <div className="rx-extras" aria-label={t('rx.grp')}>
      {extrasOff && !vfoB && chips.length === 0 && gains.length === 0 ? (
        <div className="rx-line" title={t('rx.off.tip')}>{t('rx.off')}</div>
      ) : null}
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
            <button
              key={c.key}
              type="button"
              className={`rx-chip rx-chip--btn${c.on ? ' rx-chip--on' : ''}${waiting === c.key ? ' rx-chip--wait' : ''}`}
              title={t(c.tip ?? ACTION_TIP[c.key])}
              aria-label={`${c.text}: ${t(c.tip ?? ACTION_TIP[c.key])}`}
              aria-busy={waiting === c.key}
              disabled={waiting !== null}
              onClick={() => void click(c.key)}
            >
              {c.text}
            </button>
          ))}
        </div>
      ) : null}
      {gains.length > 0 ? <div className="rx-line" title={t('tip.rx.gains')}>{gains.map(([k, v]) => `${k} ${v}`).join(' · ')}</div> : null}
    </div>
  );
}
