import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getClock } from '../api/client';
import { useT, type Key } from '../i18n';
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
  const clock = useQuery({ queryKey: ['clock'], queryFn: getClock, retry: false, refetchInterval: 60_000 });
  const [now, setNow] = useState(() => Date.now());
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
  });
  if (issues.length === 0) return null;

  return (
    <ul className="diag" aria-label={t('diag.aria')}>
      {issues.map((issue) => (
        <li key={`${issue.id}-${String(issue.vars.state ?? '')}`} className={`diag-item diag-item--${issue.severity}`}>
          <span className="diag-text">{t(messageKey(issue), issue.vars)}</span>
          {issue.href ? (
            <a className="glass glass--link" href={issue.href}>
              {t('diag.fix')}
            </a>
          ) : null}
        </li>
      ))}
    </ul>
  );
}
