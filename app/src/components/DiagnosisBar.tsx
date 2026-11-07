import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getClock } from '../api/client';
import { useT, type Key } from '../i18n';
import { AudioLossHelp } from './AudioLossHelp';
import { coreText } from '../lib/coreText';
import { LEVEL_STALE_MS } from '../lib/audioLevel';
import { diagnose, type Issue } from '../lib/diagnosis';
import { periodSecs } from '../lib/period';
import { useStation } from '../state/station';

function messageKey(issue: Issue): Key {
  return (issue.id === 'audio' ? `diag.audio.${issue.vars.state}` : `diag.${issue.id}`) as Key;
}

/** Cio' che non va nella stazione, in chiaro e con dove sistemarlo; non compare quando e' tutto a posto. */
export function DiagnosisBar() {
  const t = useT();
  const status = useStation((s) => s.status);
  const audio = useStation((s) => s.audioLevel);
  const audioAt = useStation((s) => s.audioLevelAt);
  const lastDecodeAt = useStation((s) => s.lastDecodeAt);
  const sessionStartAt = useStation((s) => s.sessionStartAt);
  const notices = useStation((s) => s.notices);
  const clock = useQuery({ queryKey: ['clock'], queryFn: getClock, retry: false, refetchInterval: 60_000 });
  const [now, setNow] = useState(() => Date.now());
  // gli avvisi chiusi restano chiusi finche' non cambiano (un altro problema, o lo stesso con un altro stato)
  const [closed, setClosed] = useState<string[]>([]);
  // scheda con i controlli da fare quando l'audio perde blocchi (si apre dall'avviso)
  const [help, setHelp] = useState<string | null>(null);
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 10_000);
    return () => clearInterval(id);
  }, []);

  const issues = diagnose({
    now,
    status,
    clockOffsetMs: clock.data?.last?.offset_ms ?? null,
    periodSecs: periodSecs(status?.mode ?? 'FT8'),
    audio: audio && now - audioAt <= LEVEL_STALE_MS ? audio : null,
    lastDecodeAt,
    sessionStartAt,
    notices,
  });
  const keyOf = (issue: Issue) => `${issue.id}-${String(issue.vars.state ?? issue.vars.at ?? '')}`;
  const visible = issues.filter((issue) => !closed.includes(keyOf(issue)));
  if (visible.length === 0) return help !== null ? <AudioLossHelp perMin={help} onClose={() => setHelp(null)} /> : null;

  // schede che galleggiano in basso a destra: compaiono e spariscono senza spostare i pannelli della stazione
  return (
    <ul className="diag" aria-label={t('diag.aria')} aria-live="polite">
      {visible.map((issue) => (
        <li key={keyOf(issue)} className={`diag-item diag-item--${issue.severity}`}>
          {issue.id === 'audio_loss' ? (
            <button type="button" className="diag-text diag-text--link" onClick={() => setHelp(String(issue.vars.detail ?? ''))}>
              {t('diag.audio_loss', { ...issue.vars, detail: String(issue.vars.detail ?? '') })}
            </button>
          ) : (
            <span className="diag-text">{t(messageKey(issue), typeof issue.vars.detail === 'string' ? { ...issue.vars, detail: coreText(issue.vars.detail, t) } : issue.vars)}</span>
          )}
          {issue.id === 'audio_loss' ? (
            <button type="button" className="diag-fix diag-fix--btn" onClick={() => setHelp(String(issue.vars.detail ?? ''))}>
              {t('diag.howto')}
            </button>
          ) : issue.href ? (
            <a className="diag-fix" href={issue.href}>
              {t('diag.fix')}
            </a>
          ) : null}
          <button type="button" className="diag-close" aria-label={t('diag.close')} title={t('diag.close')} onClick={() => setClosed((c) => [...c, keyOf(issue)])}>
            ×
          </button>
        </li>
      ))}
      {help !== null ? <AudioLossHelp perMin={help} onClose={() => setHelp(null)} /> : null}
    </ul>
  );
}
