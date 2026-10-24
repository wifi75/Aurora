import { useEffect, useState } from 'react';
import { useT, type Key } from '../i18n';
import { LEVEL_STALE_MS, meterPercent, toDb } from '../lib/audioLevel';
import { useStation } from '../state/station';

/** Indicatore del livello dell'audio in ingresso: barra del valore medio, tacca del picco, avviso se manca, e' debole o e' saturo. */
export function InputLevel() {
  const t = useT();
  const level = useStation((s) => s.audioLevel);
  const at = useStation((s) => s.audioLevelAt);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  // senza misure recenti l'ingresso audio non e' attivo: niente indicatore
  if (!level || now - at > LEVEL_STALE_MS) return null;

  const detail = t('rxl.value', { peak: toDb(level.peak).toFixed(0), rms: toDb(level.rms).toFixed(0) });
  const hint = t(`rxl.hint.${level.state}` as Key);
  const warn = level.state === 'overdrive' || level.state === 'silent' || level.state === 'low';
  return (
    <div className={`in-level in-level--${level.state}`} title={`${hint} (${detail})`}>
      <span className="in-level-title">{t('rxl.title')}</span>
      <div className="in-level-meter" role="meter" aria-label={t('rxl.label')} aria-valuemin={-60} aria-valuemax={0} aria-valuenow={Math.round(toDb(level.peak))} aria-valuetext={`${t(`rxl.state.${level.state}` as Key)}, ${detail}`}>
        <span className="in-level-fill" style={{ height: `${meterPercent(level.rms)}%` }} />
        <span className="in-level-peak" style={{ bottom: `${meterPercent(level.peak)}%` }} />
      </div>
      <span className="in-level-state" role={warn ? 'status' : undefined}>{t(`rxl.state.${level.state}` as Key)}</span>
    </div>
  );
}
