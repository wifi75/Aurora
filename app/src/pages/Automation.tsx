import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getAutomation, putAutomation, type AutomationChange } from '../api/client';
import { AutomationRulesForm } from '../components/AutomationRulesForm';
import { useT, type Key } from '../i18n';
import { formatClock } from '../lib/format';

const KIND_KEY = {
  reply: 'auto.kind.reply',
  would_reply: 'auto.kind.would_reply',
  skipped: 'auto.kind.skipped',
  blocked: 'auto.kind.blocked',
  halted: 'auto.kind.halted',
  paused: 'auto.kind.paused',
  info: 'auto.kind.info',
} as const satisfies Record<string, Key>;

/** Risposte automatiche ai CQ che interessano: parte spenta e in prova a vuoto; il modo reale si sceglie dal PC. */
export function Automation() {
  const t = useT();
  const client = useQueryClient();
  const state = useQuery({ queryKey: ['automation'], queryFn: getAutomation, refetchInterval: 2000, retry: false });
  const change = useMutation({
    mutationFn: (c: AutomationChange) => putAutomation(c),
    onSuccess: (next) => client.setQueryData(['automation'], next),
  });

  if (state.isError) return <p className="notice">{t('auto.unavailable')}</p>;
  if (!state.data) return <p className="muted">{t('set.loading')}</p>;
  const a = state.data;

  return (
    <div className="settings">
      <h1 className="page-title">{t('auto.title')}</h1>
      <section className="panel settings-block">
        <h2 className="panel-title">{t('auto.status')}</h2>
        <p>
          <span className={`pill ${a.enabled ? 'pill--rx' : ''}`}>{a.enabled ? t('auto.on') : t('auto.off')}</span>{' '}
          <span className="pill">{a.mode === 'live' ? t('auto.modeLive') : t('auto.modeDry')}</span>
        </p>
        <button type="button" className="glass" title={t('tip.auto.toggle')} onClick={() => change.mutate({ enabled: !a.enabled })}>
          {a.enabled ? t('auto.turnOff') : t('auto.turnOn')}
        </button>
        <p className="muted settings-hint">
          {t('auto.liveOnPc')}
          {a.live_allowed ? '' : t('auto.txClosed')}
        </p>
        <p className="muted settings-hint">
          {t('auto.limits', {
            cycles: a.limits.max_consecutive_cycles,
            perDay: a.limits.max_per_day,
            dead: a.limits.dead_man_secs,
            today: a.replies_today,
          })}
        </p>
      </section>
      <section className="panel settings-block">
        <h2 className="panel-title">{t('auto.rules')}</h2>
        <AutomationRulesForm rules={a.rules} onSave={(rules) => change.mutate({ rules })} error={change.isError} />
      </section>
      <section className="panel settings-block">
        <h2 className="panel-title">{t('auto.log')}</h2>
        <ol className="auto-log">
          {[...a.log].reverse().map((d, i) => (
            <li key={`${d.at}-${i}`}>
              <span className="num muted">{formatClock((d.at % 86_400) * 1000)}</span>
              <span className={`badge badge--${d.kind === 'reply' ? 'auto' : 'rare'}`}>{t(KIND_KEY[d.kind])}</span>
              <span>{d.reason}</span>
            </li>
          ))}
          {a.log.length === 0 ? <li className="muted">{t('auto.noLog')}</li> : null}
        </ol>
      </section>
    </div>
  );
}
