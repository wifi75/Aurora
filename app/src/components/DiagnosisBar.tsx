import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { getCat, getClock, getEngineSettings, putEngineSettings } from '../api/client';
import { useT, type Key } from '../i18n';
import { AudioLossHelp } from './AudioLossHelp';
import { coreText } from '../lib/coreText';
import { LEVEL_STALE_MS } from '../lib/audioLevel';
import { closeSlowPc, diagnose, slowPcClosedDepths, type Issue } from '../lib/diagnosis';
import { periodSecs } from '../lib/period';
import { useStation } from '../state/station';

function messageKey(issue: Issue): Key {
  if (issue.id === 'slow_pc' && issue.vars.next) return issue.vars.depth === '3' ? 'diag.slow_pc.deep' : 'diag.slow_pc.normal';
  // contest: il motivo riconosciuto si dice nella lingua scelta; uno sconosciuto con il testo del motore
  if (issue.id === 'contest') return (issue.vars.reason ? `diag.contest.${issue.vars.reason}` : 'diag.contest') as Key;
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
  // la profondita' di decodifica serve solo quando il motore dice che il PC e' lento
  const client = useQueryClient();
  const engine = useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false, enabled: !!notices.slow_pc });
  // lo stato del CAT serve solo con l'avviso della porta occupata: appena la radio si collega l'avviso sparisce
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, enabled: !!notices.port_busy, refetchInterval: notices.port_busy ? 5000 : false });
  const [depthBusy, setDepthBusy] = useState(false);
  const [depthError, setDepthError] = useState<string | null>(null);
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
    decodeDepth: engine.data?.values.DecodeDepth ?? null,
    slowPcDismissed: slowPcClosedDepths(),
    cat: cat.data ? { connected: cat.data.status.state === 'connected', at: cat.dataUpdatedAt } : null,
  });
  const keyOf = (issue: Issue) => `${issue.id}-${String(issue.vars.state ?? issue.vars.at ?? '')}`;
  const visible = issues.filter((issue) => !closed.includes(keyOf(issue)));
  const close = (issue: Issue) => {
    if (issue.id === 'slow_pc' && issue.vars.depth) closeSlowPc(String(issue.vars.depth));
    setClosed((c) => [...c, keyOf(issue)]);
  };
  // la profondita' si cambia solo da qui, a richiesta: le impostazioni del motore la applicano dal periodo che segue
  const setDepth = async (depth: string) => {
    setDepthBusy(true);
    setDepthError(null);
    try {
      client.setQueryData(['engine-settings'], await putEngineSettings({ DecodeDepth: depth }));
    } catch (e) {
      setDepthError(e instanceof Error ? e.message : String(e));
      void client.invalidateQueries({ queryKey: ['engine-settings'] });
    } finally {
      setDepthBusy(false);
    }
  };
  if (visible.length === 0) return help !== null ? <AudioLossHelp perMin={help} onClose={() => setHelp(null)} /> : null;

  // schede che galleggiano in basso a destra: compaiono e spariscono senza spostare i pannelli della stazione. Nel corpo della pagina,
  // fuori da `.app-shell`: con le Impostazioni aperte quella e' disattivata (inert) e l'avviso non si poteva chiudere
  return createPortal(
    <ul className="diag" aria-label={t('diag.aria')} aria-live="polite">
      {visible.map((issue) => (
        <li key={keyOf(issue)} className={`diag-item diag-item--${issue.severity}`}>
          {issue.id === 'audio_loss' ? (
            <button type="button" className="diag-text diag-text--link" onClick={() => setHelp(String(issue.vars.detail ?? ''))}>
              {t('diag.audio_loss', { ...issue.vars, detail: String(issue.vars.detail ?? '') })}
            </button>
          ) : (
            <span className="diag-text">
              {t(messageKey(issue), typeof issue.vars.detail === 'string' ? { ...issue.vars, detail: coreText(issue.vars.detail, t) } : issue.vars)}
              {issue.id === 'slow_pc' && depthError !== null ? <span className="diag-error">{t('diag.slow_pc.failed', { message: depthError })}</span> : null}
            </span>
          )}
          {issue.id === 'slow_pc' && issue.vars.next ? (
            <span className="diag-actions">
              <button type="button" className="diag-fix diag-fix--btn" disabled={depthBusy} onClick={() => void setDepth(String(issue.vars.next))}>
                {t(issue.vars.next === '2' ? 'diag.slow_pc.to2' : 'diag.slow_pc.to1')}
              </button>
              <a className="diag-fix diag-fix--minor" href={issue.href}>
                {t('diag.slow_pc.setting')}
              </a>
            </span>
          ) : issue.id === 'audio_loss' ? (
            <button type="button" className="diag-fix diag-fix--btn" onClick={() => setHelp(String(issue.vars.detail ?? ''))}>
              {t('diag.howto')}
            </button>
          ) : issue.href ? (
            <a className="diag-fix" href={issue.href}>
              {t('diag.fix')}
            </a>
          ) : null}
          <button type="button" className="diag-close" aria-label={t('diag.close')} title={t('diag.close')} onClick={() => close(issue)}>
            ×
          </button>
        </li>
      ))}
      {help !== null ? <AudioLossHelp perMin={help} onClose={() => setHelp(null)} /> : null}
    </ul>,
    document.body,
  );
}
