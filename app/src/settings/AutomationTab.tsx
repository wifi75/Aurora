import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getAutomation, putAutomation } from '../api/client';
import type { AutomationRules } from '../api/automation';
import type { DecodeTag } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { coreText } from '../lib/coreText';
import { formatClock } from '../lib/format';
import { useSharedDraft } from './session';

const WANTED: ReadonlyArray<{ tag: DecodeTag; label: Key }> = [
  { tag: 'new_dxcc', label: 'kind.new_dxcc' },
  { tag: 'new_dxcc_band', label: 'kind.new_dxcc_band' },
  { tag: 'new_zone', label: 'kind.new_zone' },
  { tag: 'new_grid', label: 'kind.new_grid' },
];

const KIND_KEY = {
  reply: 'auto.kind.reply',
  would_reply: 'auto.kind.would_reply',
  skipped: 'auto.kind.skipped',
  blocked: 'auto.kind.blocked',
  halted: 'auto.kind.halted',
  paused: 'auto.kind.paused',
  info: 'auto.kind.info',
} as const satisfies Record<string, Key>;

/** Chiave con cui la bozza delle regole sta nella bozza unica della finestra delle Impostazioni (la legge `SettingsWindow`). */
export const AUTOMATION_DRAFT = 'automation';

/**
 * Scheda Automazioni: lo stato con l'interruttore, le regole con cui Aurora risponde da sola ai CQ (si salvano con Applica o Salva
 * come le altre voci) e il registro delle decisioni. Prima era una pagina a parte e le regole stavano anche dentro «Sequenza».
 */
export function AutomationTab() {
  const t = useT();
  const client = useQueryClient();
  const shared = useSharedDraft();
  const state = useQuery({ queryKey: ['automation'], queryFn: getAutomation, refetchInterval: 2000, retry: false });
  const toggle = useMutation({
    mutationFn: (enabled: boolean) => putAutomation({ enabled }),
    onSuccess: (next) => client.setQueryData(['automation'], next),
  });
  if (state.isError) return <p className="notice">{t('auto.unavailable')}</p>;
  if (!state.data) return <p className="muted">{t('set.loading')}</p>;
  const a = state.data;
  const rules: AutomationRules = (shared?.drafts[AUTOMATION_DRAFT] as AutomationRules | undefined) ?? a.rules;
  const edit = (next: AutomationRules) => shared?.setDraftOf(AUTOMATION_DRAFT, JSON.stringify(next) === JSON.stringify(a.rules) ? undefined : next);
  const toggleTag = (tag: DecodeTag) => edit({ ...rules, reply_to: rules.reply_to.includes(tag) ? rules.reply_to.filter((x) => x !== tag) : [...rules.reply_to, tag] });
  return (
    <div className="sw-page">
      <section className="sw-box" aria-label={t('auto.status')}>
        <h3>{t('auto.status')}</h3>
        <div className="sw-row">
          <span className={`pill ${a.enabled ? 'pill--rx' : ''}`}>{a.enabled ? t('auto.on') : t('auto.off')}</span>
          <span className="pill">{a.mode === 'live' ? t('auto.modeLive') : t('auto.modeDry')}</span>
          <button type="button" className="glass" title={t('tip.auto.toggle')} disabled={toggle.isPending} onClick={() => toggle.mutate(!a.enabled)}>
            {a.enabled ? t('auto.turnOff') : t('auto.turnOn')}
          </button>
        </div>
        <p className="sw-note">
          {t('auto.liveOnPc')}
          {a.live_allowed ? '' : t('auto.txClosed')}
        </p>
        <p className="sw-note">
          {t('auto.limits', { cycles: a.limits.max_consecutive_cycles, perDay: a.limits.max_per_day, dead: a.limits.dead_man_secs, today: a.replies_today })}
        </p>
      </section>

      <section className="sw-box" aria-label={t('auto.replyTo')}>
        <h3>{t('auto.replyTo')}</h3>
        <div className="sw-grid sw-grid--4" title={t('tip.auto.tags')}>
          {WANTED.map((w) => (
            <label key={w.tag} className="sw-check">
              <input type="checkbox" checked={rules.reply_to.includes(w.tag)} onChange={() => toggleTag(w.tag)} /> {t(w.label)}
            </label>
          ))}
        </div>
        <div className="sw-grid sw-grid--2c">
          <label className="sw-check" title={t('tip.auto.wanted')}>
            <input type="checkbox" checked={rules.reply_to_wanted} onChange={(e) => edit({ ...rules, reply_to_wanted: e.target.checked })} /> {t('auto.wanted')}
          </label>
          <label className="sw-check" title={t('tip.auto.direction')}>
            <input type="checkbox" checked={rules.respect_cq_direction} onChange={(e) => edit({ ...rules, respect_cq_direction: e.target.checked })} /> {t('auto.direction')}
          </label>
        </div>
      </section>

      <section className="sw-box" aria-label={t('auto.rules')}>
        <h3>{t('auto.rules')}</h3>
        <div className="sw-grid sw-grid--4">
          <label className="sw-field" title={t('tip.auto.prefer')}>
            <span className="sw-label">{t('auto.prefer')}</span>
            <select value={rules.prefer} onChange={(e) => edit({ ...rules, prefer: e.target.value === 'snr' ? 'snr' : e.target.value === 'score' ? 'score' : 'rarity' })}>
              <option value="rarity">{t('auto.prefer.rarity')}</option>
              <option value="snr">{t('auto.prefer.snr')}</option>
              <option value="score">{t('auto.prefer.score')}</option>
            </select>
          </label>
          <label className="sw-field" title={t('tip.auto.minSnr')}>
            <span className="sw-label">{t('auto.minSnr')}</span>
            <input type="number" min={-30} max={20} value={rules.min_snr} onChange={(e) => edit({ ...rules, min_snr: Number(e.target.value) })} />
          </label>
          <label className="sw-field" title={t('tip.auto.attempts')}>
            <span className="sw-label">{t('auto.attempts')}</span>
            <input type="number" min={1} max={10} value={rules.max_attempts_per_call} onChange={(e) => edit({ ...rules, max_attempts_per_call: Number(e.target.value) })} />
          </label>
          <label className="sw-field" title={t('tip.auto.cooldown')}>
            <span className="sw-label">{t('auto.cooldown')}</span>
            <input
              type="number"
              min={1}
              max={120}
              value={Math.round(rules.cooldown_secs / 60)}
              onChange={(e) => edit({ ...rules, cooldown_secs: Math.min(7200, Math.max(60, Number(e.target.value) * 60)) })}
            />
          </label>
        </div>
        <label className="sw-field sw-field--wide" title={t('tip.auto.excluded')}>
          <span className="sw-label">{t('auto.excluded')}</span>
          <input
            type="text"
            value={rules.excluded_calls.join(', ')}
            onChange={(e) => edit({ ...rules, excluded_calls: e.target.value.split(',').map((c) => c.trim()).filter(Boolean) })}
          />
        </label>
      </section>

      <section className="sw-box" aria-label={t('auto.log')}>
        <h3>{t('auto.log')}</h3>
        <ol className="sw-log">
          {[...a.log].reverse().map((d, i) => (
            <li key={`${d.at}-${i}`}>
              <span className="num muted">{formatClock((d.at % 86_400) * 1000)}</span>
              <span className={`badge badge--${d.kind === 'reply' ? 'auto' : 'rare'}`}>{t(KIND_KEY[d.kind])}</span>
              <span>{coreText(d.reason, t)}</span>
            </li>
          ))}
          {a.log.length === 0 ? <li className="muted">{t('auto.noLog')}</li> : null}
        </ol>
      </section>
    </div>
  );
}
