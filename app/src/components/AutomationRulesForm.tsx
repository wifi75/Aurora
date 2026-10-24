import { useState } from 'react';
import type { AutomationRules } from '../api/automation';
import type { DecodeTag } from '../api/schemas';
import { useT, type Key } from '../i18n';

const WANTED: ReadonlyArray<{ tag: DecodeTag; label: Key }> = [
  { tag: 'new_dxcc', label: 'kind.new_dxcc' },
  { tag: 'new_dxcc_band', label: 'kind.new_dxcc_band' },
  { tag: 'new_zone', label: 'kind.new_zone' },
  { tag: 'new_grid', label: 'kind.new_grid' },
];

interface Props {
  rules: AutomationRules;
  /** Salva le regole; ritorna dopo che Aurora le ha accettate. */
  onSave(rules: AutomationRules): void;
  error?: boolean;
}

/** Regole di risposta ai CQ: quali novita', segnale minimo, tentativi per stazione, esclusioni. */
export function AutomationRulesForm({ rules: saved, onSave, error = false }: Props) {
  const t = useT();
  const [draft, setDraft] = useState<AutomationRules | null>(null);
  const rules = draft ?? saved;
  const toggleTag = (tag: DecodeTag) =>
    setDraft({ ...rules, reply_to: rules.reply_to.includes(tag) ? rules.reply_to.filter((x) => x !== tag) : [...rules.reply_to, tag] });
  return (
    <>
      <fieldset className="auto-tags" title={t('tip.auto.tags')}>
        <legend className="muted">{t('auto.replyTo')}</legend>
        {WANTED.map((w) => (
          <label key={w.tag}>
            <input type="checkbox" checked={rules.reply_to.includes(w.tag)} onChange={() => toggleTag(w.tag)} /> {t(w.label)}
          </label>
        ))}
      </fieldset>
      <label className="auto-wanted" title={t('tip.auto.wanted')}>
        <input type="checkbox" checked={rules.reply_to_wanted} onChange={(e) => setDraft({ ...rules, reply_to_wanted: e.target.checked })} /> {t('auto.wanted')}
      </label>
      <label className="auto-field" title={t('tip.auto.prefer')}>
        {t('auto.prefer')}
        <select value={rules.prefer} onChange={(e) => setDraft({ ...rules, prefer: e.target.value === 'snr' ? 'snr' : e.target.value === 'score' ? 'score' : 'rarity' })}>
          <option value="rarity">{t('auto.prefer.rarity')}</option>
          <option value="snr">{t('auto.prefer.snr')}</option>
          <option value="score">{t('auto.prefer.score')}</option>
        </select>
      </label>
      <label className="auto-field" title={t('tip.auto.cooldown')}>
        {t('auto.cooldown')}
        <input
          type="number"
          min={1}
          max={120}
          value={Math.round(rules.cooldown_secs / 60)}
          onChange={(e) => setDraft({ ...rules, cooldown_secs: Math.min(7200, Math.max(60, Number(e.target.value) * 60)) })}
        />
      </label>
      <label className="auto-wanted" title={t('tip.auto.direction')}>
        <input type="checkbox" checked={rules.respect_cq_direction} onChange={(e) => setDraft({ ...rules, respect_cq_direction: e.target.checked })} /> {t('auto.direction')}
      </label>
      <label className="auto-field" title={t('tip.auto.minSnr')}>
        {t('auto.minSnr')}
        <input type="number" min={-30} max={20} value={rules.min_snr} onChange={(e) => setDraft({ ...rules, min_snr: Number(e.target.value) })} />
      </label>
      <label className="auto-field" title={t('tip.auto.attempts')}>
        {t('auto.attempts')}
        <input
          type="number"
          min={1}
          max={10}
          value={rules.max_attempts_per_call}
          onChange={(e) => setDraft({ ...rules, max_attempts_per_call: Number(e.target.value) })}
        />
      </label>
      <label className="auto-field" title={t('tip.auto.excluded')}>
        {t('auto.excluded')}
        <input
          type="text"
          value={rules.excluded_calls.join(', ')}
          onChange={(e) => setDraft({ ...rules, excluded_calls: e.target.value.split(',').map((c) => c.trim()).filter(Boolean) })}
        />
      </label>
      <button
        type="button"
        className="glass"
        disabled={!draft}
        onClick={() => {
          if (draft) {
            onSave(draft);
            setDraft(null);
          }
        }}
      >
        {t('auto.saveRules')}
      </button>
      {error ? (
        <p className="live-note" role="alert">
          {t('auto.rejected')}
        </p>
      ) : null}
    </>
  );
}
