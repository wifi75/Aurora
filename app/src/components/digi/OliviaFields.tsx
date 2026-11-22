import { useEffect, useState } from 'react';
import type { DigiSettings } from '../../api/client';
import { useT } from '../../i18n';
import { effectiveTxHz, OLIVIA_BANDWIDTHS, OLIVIA_SQUELCH_MAX, OLIVIA_SYNC_MAX, OLIVIA_SYNC_MIN, OLIVIA_TONES, oliviaFits, oliviaOf, oliviaPatch, type OliviaMode } from '../../lib/olivia';

/**
 * Olivia o Contestia: toni, banda, margine e integrazione del sincronismo, soglia dello squelch. Sotto, l'avviso se toni e banda non entrano
 * nella banda audio alla frequenza di trasmissione o di ricezione (il motore rifiuterebbe la trasmissione e non riceverebbe niente).
 */
export function OliviaFields({ settings: s, mode, snrDb = null, onSave }: { settings: DigiSettings; mode: OliviaMode; snrDb?: number | null; onSave(patch: Partial<DigiSettings>): void }) {
  const t = useT();
  const o = oliviaOf(s, mode);
  const [sq, setSq] = useState(o.squelch_snr_db);
  const [margin, setMargin] = useState(String(o.sync_margin));
  const [integ, setInteg] = useState(String(o.sync_integration));
  useEffect(() => setSq(o.squelch_snr_db), [o.squelch_snr_db]);
  useEffect(() => setMargin(String(o.sync_margin)), [o.sync_margin]);
  useEffect(() => setInteg(String(o.sync_integration)), [o.sync_integration]);
  const put = (part: Parameters<typeof oliviaPatch>[2]) => onSave(oliviaPatch(s, mode, part));
  const sync = (raw: string, field: 'sync_margin' | 'sync_integration', reset: () => void) => {
    const v = Number(raw);
    if (Number.isInteger(v) && v >= OLIVIA_SYNC_MIN && v <= OLIVIA_SYNC_MAX) {
      if (v !== o[field]) put({ [field]: v });
    } else reset();
  };
  const txHz = effectiveTxHz(s);
  const txFits = oliviaFits(o.tones, o.bandwidth_hz, txHz);
  const rxFits = oliviaFits(o.tones, o.bandwidth_hz, s.rx_freq_hz, o.sync_margin);
  const live = s.mode === mode;
  return (
    <>
      <label title={t('digi.olivia.tonesTip')}>
        <span>{t('digi.olivia.tones')}</span>
        <select value={o.tones} onChange={(e) => put({ tones: Number(e.target.value) })}>
          {OLIVIA_TONES.map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      </label>
      <label title={t('digi.olivia.bandwidthTip')}>
        <span>{t('digi.olivia.bandwidth')}</span>
        <select value={o.bandwidth_hz} onChange={(e) => put({ bandwidth_hz: Number(e.target.value) })}>
          {OLIVIA_BANDWIDTHS.map((v) => (
            <option key={v} value={v}>
              {v} Hz
            </option>
          ))}
        </select>
      </label>
      <label title={t('digi.olivia.syncMarginTip')}>
        <span>{t('digi.olivia.syncMargin')}</span>
        <input type="number" min={OLIVIA_SYNC_MIN} max={OLIVIA_SYNC_MAX} step={1} value={margin} onChange={(e) => setMargin(e.target.value)} onBlur={() => sync(margin, 'sync_margin', () => setMargin(String(o.sync_margin)))} />
      </label>
      <label title={t('digi.olivia.syncIntegTip')}>
        <span>{t('digi.olivia.syncInteg')}</span>
        <input type="number" min={OLIVIA_SYNC_MIN} max={OLIVIA_SYNC_MAX} step={1} value={integ} onChange={(e) => setInteg(e.target.value)} onBlur={() => sync(integ, 'sync_integration', () => setInteg(String(o.sync_integration)))} />
      </label>
      <label title={t('digi.olivia.squelchHint')}>
        <span>{t('digi.olivia.squelch', { db: Math.round(sq) })}</span>
        <input type="range" min={0} max={OLIVIA_SQUELCH_MAX} step={1} value={sq} onChange={(e) => setSq(Number(e.target.value))} onPointerUp={() => put({ squelch_snr_db: sq })} onKeyUp={() => put({ squelch_snr_db: sq })} />
      </label>
      <p className="sstv-hint">
        {live && snrDb !== null ? <>{t('digi.olivia.syncSnrNow', { db: Math.round(snrDb) })} — </> : null}
        {t('digi.olivia.squelchHint')} {t('digi.olivia.same')}
      </p>
      {!txFits ? (
        <p className="sstv-hint digi-olivia-warn" role="alert">
          {t('digi.olivia.noFit', { tones: o.tones, bw: o.bandwidth_hz, hz: Math.round(txHz) })}
        </p>
      ) : !rxFits ? (
        <p className="sstv-hint digi-olivia-warn" role="alert">
          {t('digi.olivia.noFitRx', { tones: o.tones, bw: o.bandwidth_hz, hz: Math.round(s.rx_freq_hz) })}
        </p>
      ) : null}
    </>
  );
}
