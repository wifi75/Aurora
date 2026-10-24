import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getInputLevel, putInputLevel } from '../api/client';
import { useT, type Key } from '../i18n';
import { LEVEL_STALE_MS, meterPercent, toDb } from '../lib/audioLevel';
import { replyErrorMessage } from '../lib/replyError';
import { useStation } from '../state/station';
import { VerticalSlider } from './VerticalSlider';

/**
 * Livello dell'audio in ingresso: barra del valore medio con la tacca del picco, avviso se manca, e' debole o e' saturo,
 * e il volume di registrazione di Windows (cursore e AUTO) con cui correggerlo.
 */
export function InputLevel() {
  const t = useT();
  const client = useQueryClient();
  const level = useStation((s) => s.audioLevel);
  const at = useStation((s) => s.audioLevelAt);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  // con AUTO acceso il volume cambia da solo: si rilegge piu' spesso
  const volume = useQuery({ queryKey: ['input-level'], queryFn: getInputLevel, retry: false, refetchInterval: (q) => (q.state.status === 'error' ? false : q.state.data?.auto ? 4000 : 20_000) });
  const [draft, setDraft] = useState<number | null>(null);
  const [note, setNote] = useState('');

  // senza misure recenti l'ingresso audio non e' attivo: niente indicatore
  if (!level || now - at > LEVEL_STALE_MS) return null;

  const detail = t('rxl.value', { peak: toDb(level.peak).toFixed(0), rms: toDb(level.rms).toFixed(0) });
  const hint = t(`rxl.hint.${level.state}` as Key);
  const warn = level.state === 'overdrive' || level.state === 'silent' || level.state === 'low';
  const control = volume.data;
  const shown = draft ?? control?.volume_pct ?? 0;

  const change = async (body: { volume_pct?: number; auto?: boolean }) => {
    try {
      await putInputLevel(body);
      await client.invalidateQueries({ queryKey: ['input-level'] });
      setNote('');
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    } finally {
      setDraft(null);
    }
  };

  return (
    <div className={`in-level in-level--${level.state}`} title={`${hint} (${detail})`}>
      <span className="in-level-title">{t('rxl.title')}</span>
      <div className="in-level-pair">
        <div className="in-level-meter" role="meter" aria-label={t('rxl.label')} aria-valuemin={-60} aria-valuemax={0} aria-valuenow={Math.round(toDb(level.peak))} aria-valuetext={`${t(`rxl.state.${level.state}` as Key)}, ${detail}`}>
          <span className="in-level-fill" style={{ height: `${meterPercent(level.rms)}%` }} />
          <span className="in-level-peak" style={{ bottom: `${meterPercent(level.peak)}%` }} />
        </div>
        {control ? (
          <VerticalSlider value={shown} label={t('rxl.volume')} onChange={setDraft} onCommit={(v) => void change({ volume_pct: v })} />
        ) : null}
      </div>
      {control ? (
        <>
          <span className="data muted in-level-value" title={t('rxl.volume.value', { pct: shown, device: control.device })}>{shown}%</span>
          <button type="button" className={`glass chip in-level-auto${control.auto ? ' glass--on' : ''}`} aria-pressed={control.auto} title={t('rxl.auto.tip')} onClick={() => void change({ auto: !control.auto })}>
            {t('rxl.auto')}
          </button>
        </>
      ) : null}
      <span className="in-level-state" role={warn ? 'status' : undefined}>{t(`rxl.state.${level.state}` as Key)}</span>
      {note ? <span className="live-note" role="status">{note}</span> : null}
    </div>
  );
}
