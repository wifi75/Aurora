import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type ReactNode } from 'react';
import { ApiError, getAutomation, getAutomationCountries, putAutomation } from '../api/client';
import { askConfirm } from '../components/ConfirmHost';
import type { AutomationRules, DecisionKind } from '../api/automation';
import type { DecodeTag } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { coreText } from '../lib/coreText';
import { formatClock } from '../lib/format';
import { selectSettingsTab } from '../lib/settingsWindow';
import { toast } from '../lib/toast';
import { Help } from './layout';
import { useDraft, useSettings } from './NativeSettings';
import { useSharedDraft } from './session';

/** Le novita' a cui si puo' rispondere (le etichette di «CQ e novita'»); il «nuovo modo» e' una regola a parte, calcolata dal log. */
const TAGS: ReadonlyArray<{ tag: DecodeTag; label: Key; tip: Key }> = [
  { tag: 'new_dxcc', label: 'kind.new_dxcc', tip: 'tip.auto.tag.new_dxcc' },
  { tag: 'new_dxcc_band', label: 'kind.new_dxcc_band', tip: 'tip.auto.tag.new_dxcc_band' },
  { tag: 'new_grid', label: 'kind.new_grid', tip: 'tip.auto.tag.new_grid' },
  { tag: 'new_zone', label: 'kind.new_zone', tip: 'tip.auto.tag.new_zone' },
];

const PREFER = [
  { value: 'rarity', label: 'auto.prefer.rarity', tip: 'tip.auto.prefer.rarity.one' },
  { value: 'snr', label: 'auto.prefer.snr', tip: 'tip.auto.prefer.snr.one' },
  { value: 'score', label: 'auto.prefer.score', tip: 'tip.auto.prefer.score.one' },
] as const satisfies ReadonlyArray<{ value: AutomationRules['prefer']; label: Key; tip: Key }>;

const CONTINENTS = ['EU', 'NA', 'SA', 'AF', 'AS', 'OC', 'AN'] as const;

const KIND_KEY = {
  reply: 'auto.kind.reply',
  would_reply: 'auto.kind.would_reply',
  skipped: 'auto.kind.skipped',
  blocked: 'auto.kind.blocked',
  halted: 'auto.kind.halted',
  paused: 'auto.kind.paused',
  info: 'auto.kind.info',
  call_cq: 'auto.kind.call_cq',
  would_call_cq: 'auto.kind.would_call_cq',
} as const satisfies Record<DecisionKind, Key>;

/** Il colore della riga del registro: verde le azioni (risposte e CQ), arancio i saltati, rosso i blocchi e lo STOP, neutro il resto. */
function tone(kind: DecisionKind): 'go' | 'skip' | 'stop' | 'note' {
  if (kind === 'reply' || kind === 'would_reply' || kind === 'call_cq' || kind === 'would_call_cq') return 'go';
  if (kind === 'skipped') return 'skip';
  if (kind === 'blocked' || kind === 'halted') return 'stop';
  return 'note';
}

/** Minuti dalla mezzanotte <-> «HH:MM» per i campi dell'ora. */
const hhmm = (m: number) => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
const minutesOf = (v: string, fallback: number) => {
  const r = /^(\d{1,2}):(\d{2})$/.exec(v);
  if (!r) return fallback;
  const m = Number(r[1]) * 60 + Number(r[2]);
  return m >= 0 && m < 1440 ? m : fallback;
};
const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, Math.round(n)));

/**
 * Campo numerico che non salta mentre si scrive: un valore dentro i limiti va subito nella bozza, uno fuori (o «-» a meta') resta solo
 * scritto finche' si esce dal campo, e allora si riporta dentro i limiti.
 */
function NumInput({ value, min, max, disabled, onChange }: { value: number; min: number; max: number; disabled?: boolean; onChange(n: number): void }) {
  const [text, setText] = useState<string | null>(null);
  return (
    <input
      type="number"
      min={min}
      max={max}
      disabled={disabled}
      value={text ?? String(value)}
      onChange={(e) => {
        const v = e.target.value;
        setText(v);
        const n = Number(v);
        if (v !== '' && Number.isFinite(n) && n >= min && n <= max) onChange(Math.round(n));
      }}
      onBlur={() => {
        if (text === null) return;
        const n = Number(text);
        onChange(clamp(text !== '' && Number.isFinite(n) ? n : value, min, max));
        setText(null);
      }}
    />
  );
}

/** Chiave con cui la bozza delle regole sta nella bozza unica della finestra delle Impostazioni (la legge `SettingsWindow`). */
export const AUTOMATION_DRAFT = 'automation';

/** Un blocco della griglia: titolo con il «?» che spiega cosa fa. */
function Block({ title, tip, children, className = '' }: { title: string; tip: string; children: ReactNode; className?: string }) {
  return (
    <section className={`sw-box au-block ${className}`.trim()} aria-label={title}>
      <h3>
        {title}
        <Help tip={tip} />
      </h3>
      {children}
    </section>
  );
}

/**
 * Scheda Automazioni: in alto la testata sempre visibile (stato, modo, numeri del giorno, Accendi/Spegni), poi cinque blocchi di regole
 * (a chi rispondere, chi prima, limiti, esclusioni, quando non c'e' nessuno) e il registro delle decisioni colorato. Le regole si salvano
 * con Applica o Salva come le altre voci; il Band Hopping e' una voce del motore e va nella stessa bozza.
 */
export function AutomationTab() {
  const t = useT();
  const client = useQueryClient();
  const shared = useSharedDraft();
  const state = useQuery({ queryKey: ['automation'], queryFn: getAutomation, refetchInterval: 2000, retry: false });
  const countries = useQuery({ queryKey: ['automation-countries'], queryFn: getAutomationCountries, retry: false, staleTime: Infinity });
  const engine = useSettings();
  const engineDraft = useDraft();
  const toggle = useMutation({
    mutationFn: (enabled: boolean) => putAutomation({ enabled }),
    onSuccess: (next) => client.setQueryData(['automation'], next),
  });
  // prova a vuoto <-> modo reale: il modo reale chiede prima la conferma dell'avviso (il core la pretende: `accept_license_notice`)
  const setMode = useMutation({
    mutationFn: async (mode: 'dry_run' | 'live') => {
      if (mode === 'live' && !(await askConfirm(t('auto.liveNotice')))) return null;
      return putAutomation(mode === 'live' ? { mode, accept_license_notice: true } : { mode });
    },
    onSuccess: (next) => {
      if (next) client.setQueryData(['automation'], next);
    },
    onError: (e) => {
      const code = e instanceof ApiError ? e.code : '';
      toast(code === 'local_only' ? t('auto.liveLocalOnly') : code === 'tx_disabled' ? t('auto.liveTxClosed') : t('auto.liveFailed'), 'error');
    },
  });
  if (state.isError) return <p className="notice">{t('auto.unavailable')}</p>;
  if (!state.data) return <p className="muted">{t('set.loading')}</p>;
  const a = state.data;
  const rules: AutomationRules = (shared?.drafts[AUTOMATION_DRAFT] as AutomationRules | undefined) ?? a.rules;
  const edit = (patch: Partial<AutomationRules>) => {
    const next = { ...rules, ...patch };
    shared?.setDraftOf(AUTOMATION_DRAFT, JSON.stringify(next) === JSON.stringify(a.rules) ? undefined : next);
  };
  const toggleTag = (tag: DecodeTag) => edit({ reply_to: rules.reply_to.includes(tag) ? rules.reply_to.filter((x) => x !== tag) : [...rules.reply_to, tag] });
  const toggleContinent = (c: string) => edit({ continents: rules.continents.includes(c) ? rules.continents.filter((x) => x !== c) : [...rules.continents, c] });
  const live = a.mode === 'live';

  // le ultime stazioni chiamate dalle Automazioni (anche quelle «avrei risposto» della prova a vuoto), una volta sola ciascuna, la piu' recente prima
  const recent = [...a.log]
    .reverse()
    .filter((d) => (d.kind === 'reply' || d.kind === 'would_reply') && d.call)
    .filter((d, i, all) => all.findIndex((x) => x.call === d.call) === i)
    .slice(0, 10);

  // l'ultima azione vera (risposta o CQ, anche in prova a vuoto)
  const last = [...a.log].reverse().find((d) => tone(d.kind) === 'go');
  const lastText = last ? `${formatClock((last.at % 86_400) * 1000).slice(0, 5)} ${last.call ?? 'CQ'}` : '—';

  // Band Hopping: voce del motore (scheda Sequenza); senza motore proprio non c'e'
  const hopItem = engine.data?.items.BandHopping;
  const hopLive = hopItem?.state === 'live';
  const hopCurrent = engine.data?.values.BandHopping;
  const hopOn = (engineDraft.changes.BandHopping ?? hopCurrent) === 'true';

  const available = countries.data ?? [];
  const excludedSet = new Set(rules.excluded_countries);
  const autoCqOn = rules.auto_cq_after_min > 0;

  return (
    <div className="sw-page au-page">
      <header className="au-head" aria-label={t('auto.status')}>
        <span className={`pill ${a.enabled ? 'pill--rx' : ''}`} title={t('tip.auto.state')}>
          {a.enabled ? t('auto.on') : t('auto.off')}
        </span>
        <div className="au-mode" role="radiogroup" aria-label={t('auto.modeChoice')} title={`${t('tip.auto.mode')}${a.live_allowed ? '' : t('auto.txClosed')}`}>
          <button type="button" role="radio" aria-checked={!live} className={live ? '' : 'is-on'} disabled={setMode.isPending} onClick={() => live && setMode.mutate('dry_run')}>
            {t('auto.modeDry')}
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={live}
            className={live ? 'is-on is-live' : ''}
            disabled={setMode.isPending || (!live && !a.live_allowed)}
            onClick={() => !live && setMode.mutate('live')}
          >
            {t('auto.modeLive')}
          </button>
        </div>
        {a.enabled && !a.in_window ? <span className="pill au-pill--wait">{t('auto.outWindow')}</span> : null}
        <dl className="au-stats" title={t('tip.auto.today')}>
          <div>
            <dt>{live ? t('auto.today.replies') : t('auto.today.would')}</dt>
            <dd className="num">{live ? a.replies_today : a.would_replies_today}</dd>
          </div>
          <div>
            <dt>{t('auto.today.qsos')}</dt>
            <dd className="num">{a.qsos_today}</dd>
          </div>
          <div>
            <dt>{t('auto.today.last')}</dt>
            <dd className="num">{lastText}</dd>
          </div>
        </dl>
        <button type="button" className="glass au-switch" title={t('tip.auto.toggle')} disabled={toggle.isPending} onClick={() => toggle.mutate(!a.enabled)}>
          {a.enabled ? t('auto.turnOff') : t('auto.turnOn')}
        </button>
      </header>

      <div className="au-grid">
        <Block title={t('auto.b1')} tip={t('tip.auto.b1')}>
          <div className="au-checks au-chips">
            {TAGS.map((w) => (
              <label key={w.tag} className="sw-check au-chip" title={t(w.tip)}>
                <input type="checkbox" checked={rules.reply_to.includes(w.tag)} onChange={() => toggleTag(w.tag)} /> {t(w.label)}
              </label>
            ))}
            <label className="sw-check au-chip" title={t('tip.auto.newMode')}>
              <input type="checkbox" checked={rules.reply_to_new_mode} onChange={(e) => edit({ reply_to_new_mode: e.target.checked })} /> {t('auto.newMode')}
            </label>
            <label className="sw-check au-chip" title={t('tip.auto.wanted')}>
              <input type="checkbox" checked={rules.reply_to_wanted} onChange={(e) => edit({ reply_to_wanted: e.target.checked })} /> {t('tab.wishlist')}
            </label>
          </div>
          <div className="au-row">
            <label className="sw-field au-num" title={t('tip.auto.minSnr')}>
              <span className="sw-label">{t('auto.minSnr')}</span>
              <NumInput min={-30} max={20} value={rules.min_snr} onChange={(n) => edit({ min_snr: n })} />
            </label>
            <label className="sw-check" title={t('tip.auto.direction')}>
              <input type="checkbox" checked={rules.respect_cq_direction} onChange={(e) => edit({ respect_cq_direction: e.target.checked })} /> {t('auto.direction')}
            </label>
          </div>
        </Block>

        <Block title={t('auto.b2')} tip={t('tip.auto.prefer')}>
          <div className="au-checks au-checks--one" role="radiogroup" aria-label={t('auto.b2')}>
            {PREFER.map((p) => (
              <label key={p.value} className="sw-check" title={t(p.tip)}>
                <input type="radio" name="au-prefer" checked={rules.prefer === p.value} onChange={() => edit({ prefer: p.value })} /> {t(p.label)}
              </label>
            ))}
          </div>
        </Block>

        <Block title={t('auto.b3')} tip={t('tip.auto.b3')}>
          <div className="au-row">
            <label className="sw-field au-num" title={t('tip.auto.attempts')}>
              <span className="sw-label">{t('auto.attempts')}</span>
              <NumInput min={1} max={10} value={rules.max_attempts_per_call} onChange={(n) => edit({ max_attempts_per_call: n })} />
            </label>
            <label className="sw-field au-num" title={t('tip.auto.cooldown')}>
              <span className="sw-label">{t('auto.cooldown')}</span>
              <NumInput min={1} max={120} value={Math.round(rules.cooldown_secs / 60)} onChange={(n) => edit({ cooldown_secs: n * 60 })} />
            </label>
            <label className="sw-field au-num" title={t('tip.auto.perHour')}>
              <span className="sw-label">{t('auto.perHour')}</span>
              <NumInput min={0} max={120} value={rules.max_qso_per_hour} onChange={(n) => edit({ max_qso_per_hour: n })} />
            </label>
          </div>
          <div className="au-row">
            <label className="sw-check" title={t('tip.auto.window')}>
              <input type="checkbox" checked={rules.window_enabled} onChange={(e) => edit({ window_enabled: e.target.checked })} /> {t('auto.window')}
            </label>
            <label className="au-time" title={t('tip.auto.windowTimes')}>
              {t('auto.windowFrom')}
              <input type="time" disabled={!rules.window_enabled} value={hhmm(rules.window_from_min)} onChange={(e) => edit({ window_from_min: minutesOf(e.target.value, rules.window_from_min) })} />
            </label>
            <label className="au-time" title={t('tip.auto.windowTimes')}>
              {t('auto.windowTo')}
              <input type="time" disabled={!rules.window_enabled} value={hhmm(rules.window_to_min)} onChange={(e) => edit({ window_to_min: minutesOf(e.target.value, rules.window_to_min) })} />
            </label>
          </div>
          <p className="sw-note" title={t('tip.auto.pcLimits')}>
            {t('auto.pcLimits', { cycles: a.limits.max_consecutive_cycles, perDay: a.limits.max_per_day, dead: a.limits.dead_man_secs })}
          </p>
        </Block>

        <Block title={t('auto.b4')} tip={t('tip.auto.b4')}>
          <label className="sw-field" title={t('tip.auto.excluded')}>
            <span className="sw-label">{t('auto.excluded')}</span>
            <input type="text" value={rules.excluded_calls.join(', ')} onChange={(e) => edit({ excluded_calls: e.target.value.split(',').map((c) => c.trim().toUpperCase()).filter(Boolean) })} />
          </label>
          <div className="sw-field au-countries" title={t('tip.auto.countries')}>
            <span className="sw-label">{t('auto.countries')}</span>
            {available.length > 0 ? (
              <select
                aria-label={t('auto.countries')}
                value=""
                onChange={(e) => {
                  const name = e.target.value;
                  if (name && !excludedSet.has(name)) edit({ excluded_countries: [...rules.excluded_countries, name] });
                }}
              >
                <option value="">{t('auto.addCountry')}</option>
                {available
                  .filter((c) => !excludedSet.has(c))
                  .map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
              </select>
            ) : (
              <span className="muted au-small">{t('auto.noCty')}</span>
            )}
            {rules.excluded_countries.length > 0 ? (
              <ul className="au-chips">
                {rules.excluded_countries.map((c) => (
                  <li key={c}>
                    {c}
                    <button type="button" aria-label={t('auto.removeCountry', { name: c })} title={t('auto.removeCountry', { name: c })} onClick={() => edit({ excluded_countries: rules.excluded_countries.filter((x) => x !== c) })}>
                      ×
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
          <div className="sw-field" title={t('tip.auto.continents')}>
            <span className="sw-label">{t('auto.continents')}</span>
            <div className="au-conts au-chips">
              {CONTINENTS.map((c) => (
                <label key={c} className="sw-check au-chip" title={t(`auto.cont.${c}` as Key)}>
                  <input type="checkbox" checked={rules.continents.includes(c)} onChange={() => toggleContinent(c)} /> {c}
                </label>
              ))}
            </div>
          </div>
        </Block>

        <Block title={t('auto.b5')} tip={t('tip.auto.b5')}>
          <div className="au-row">
            <label className="sw-check" title={t('tip.auto.autoCq')}>
              <input type="checkbox" checked={autoCqOn} onChange={(e) => edit({ auto_cq_after_min: e.target.checked ? 5 : 0 })} /> {t('auto.autoCqOn')}
            </label>
            <label className="sw-field au-num" title={t('tip.auto.autoCq')}>
              <span className="sw-label">{t('auto.autoCq')}</span>
              <NumInput min={2} max={120} disabled={!autoCqOn} value={autoCqOn ? rules.auto_cq_after_min : 5} onChange={(n) => edit({ auto_cq_after_min: n })} />
            </label>
          </div>
          <div className="au-row">
            {hopLive ? (
              <label className="sw-check" title={t('tip.auto.bandHop')}>
                <input type="checkbox" checked={hopOn} onChange={(e) => engineDraft.set('BandHopping', e.target.checked ? 'true' : 'false', hopCurrent)} /> {t('auto.bandHop')}
              </label>
            ) : (
              <span className="muted au-small" title={t('tip.auto.bandHop')}>
                {t('auto.bandHopNa')}
              </span>
            )}
            <button type="button" className="sw-linkbtn" title={t('tip.auto.bandHopWhere')} onClick={() => selectSettingsTab('sequence')}>
              {t('auto.bandHopWhere')}
            </button>
          </div>
        </Block>
      </div>

      <section className="sw-box au-recent" aria-label={t('auto.queue')}>
        <h3>{t('auto.queue')}</h3>
        <ol className="au-chips au-recent-list">
          {a.queue.map((q, i) => (
            <li key={q.call} className="au-recent-item" title={coreText(q.why, t)}>
              <span className="num muted">{i + 1}</span> <strong>{q.call}</strong> <span className="num muted">{q.snr > 0 ? `+${q.snr}` : q.snr} dB</span>
            </li>
          ))}
          {a.queue.length === 0 ? <li className="muted">{t('auto.noQueue')}</li> : null}
        </ol>
      </section>

      <section className="sw-box au-recent" aria-label={t('auto.recent')}>
        <h3>{t('auto.recent')}</h3>
        <ul className="au-chips au-recent-list">
          {recent.map((d) => (
            <li key={`${d.at}-${d.call}`} className="au-recent-item" data-live={d.kind === 'reply' ? 'si' : 'no'}>
              <span className="num muted">{formatClock((d.at % 86_400) * 1000).slice(0, 5)}</span> <strong>{d.call}</strong>
            </li>
          ))}
          {recent.length === 0 ? <li className="muted">{t('auto.noRecent')}</li> : null}
        </ul>
      </section>

      <section className="sw-box au-logbox" aria-label={t('auto.log')}>
        <h3>
          {t('auto.log')}
          <Help tip={t('tip.auto.log')} />
        </h3>
        <ol className="au-log">
          {[...a.log].reverse().map((d, i) => (
            <li key={`${d.at}-${i}`} data-tone={tone(d.kind)}>
              <span className="num muted">{formatClock((d.at % 86_400) * 1000)}</span>
              <span className="au-kind">{t(KIND_KEY[d.kind])}</span>
              <span className="au-why">{coreText(d.reason, t)}</span>
            </li>
          ))}
          {a.log.length === 0 ? <li className="muted">{t('auto.noLog')}</li> : null}
        </ol>
      </section>
    </div>
  );
}
