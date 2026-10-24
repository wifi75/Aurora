import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import type { AutomationRules } from '../api/automation';
import { getAutomation, putAutomation, type AutomationChange } from '../api/client';
import type { DecodeTag } from '../api/schemas';
import { formatClock } from '../lib/format';

const WANTED: ReadonlyArray<{ tag: DecodeTag; label: string }> = [
  { tag: 'new_dxcc', label: 'Nuovo DXCC' },
  { tag: 'new_dxcc_band', label: 'DXCC su nuova banda' },
  { tag: 'new_zone', label: 'Nuova zona' },
  { tag: 'new_grid', label: 'Nuova griglia' },
];

const KIND_LABEL = {
  reply: 'Risposta',
  would_reply: 'Prova',
  skipped: 'Saltato',
  blocked: 'Bloccato',
  halted: 'STOP',
  paused: 'Spenta',
  info: 'Info',
} as const;

/** Risposte automatiche ai CQ che interessano: parte spenta e in prova a vuoto; il modo reale si sceglie dal PC. */
export function Automation() {
  const client = useQueryClient();
  const state = useQuery({ queryKey: ['automation'], queryFn: getAutomation, refetchInterval: 2000, retry: false });
  const change = useMutation({
    mutationFn: (c: AutomationChange) => putAutomation(c),
    onSuccess: (next) => client.setQueryData(['automation'], next),
  });
  const [draft, setDraft] = useState<AutomationRules | null>(null);

  if (state.isError) return <p className="notice">Automazione non disponibile su questo Aurora.</p>;
  if (!state.data) return <p className="muted">Caricamento…</p>;
  const a = state.data;
  const rules = draft ?? a.rules;
  const toggleTag = (tag: DecodeTag) =>
    setDraft({
      ...rules,
      reply_to: rules.reply_to.includes(tag) ? rules.reply_to.filter((t) => t !== tag) : [...rules.reply_to, tag],
    });

  return (
    <div className="settings">
      <h1 className="page-title">Automazioni</h1>
      <section className="panel settings-block">
        <h2 className="panel-title">Stato</h2>
        <p>
          <span className={`pill ${a.enabled ? 'pill--rx' : ''}`}>{a.enabled ? 'Accesa' : 'Spenta'}</span>{' '}
          <span className="pill">{a.mode === 'live' ? 'Modo reale' : 'Prova a vuoto'}</span>
        </p>
        <button type="button" className="glass" onClick={() => change.mutate({ enabled: !a.enabled })}>
          {a.enabled ? 'Spegni' : 'Accendi'}
        </button>
        <p className="muted settings-hint">
          Il modo reale si attiva dal PC.
          {a.live_allowed ? '' : ' Ora la trasmissione è chiusa: Aurora è avviata con AURORA_ALLOW_TX=0.'}
        </p>
        <p className="muted settings-hint">
          Limiti impostati sul PC: {a.limits.max_consecutive_cycles} risposte di fila senza QSO, {a.limits.max_per_day} al giorno,
          spenta dopo {a.limits.dead_man_secs} s senza dispositivi collegati. Oggi: {a.replies_today} risposte.
        </p>
      </section>
      <section className="panel settings-block">
        <h2 className="panel-title">Regole</h2>
        <fieldset className="auto-tags">
          <legend className="muted">Rispondi ai CQ con</legend>
          {WANTED.map((w) => (
            <label key={w.tag}>
              <input type="checkbox" checked={rules.reply_to.includes(w.tag)} onChange={() => toggleTag(w.tag)} /> {w.label}
            </label>
          ))}
        </fieldset>
        <label className="auto-field">
          Segnale minimo (dB)
          <input type="number" min={-30} max={20} value={rules.min_snr} onChange={(e) => setDraft({ ...rules, min_snr: Number(e.target.value) })} />
        </label>
        <label className="auto-field">
          Tentativi per stazione
          <input
            type="number"
            min={1}
            max={10}
            value={rules.max_attempts_per_call}
            onChange={(e) => setDraft({ ...rules, max_attempts_per_call: Number(e.target.value) })}
          />
        </label>
        <label className="auto-field">
          Stazioni escluse (separate da virgola)
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
            if (draft) change.mutate({ rules: draft }, { onSuccess: () => setDraft(null) });
          }}
        >
          Salva le regole
        </button>
        {change.isError ? (
          <p className="live-note" role="alert">
            Modifica non accettata da Aurora.
          </p>
        ) : null}
      </section>
      <section className="panel settings-block">
        <h2 className="panel-title">Registro delle decisioni</h2>
        <ol className="auto-log">
          {[...a.log].reverse().map((d, i) => (
            <li key={`${d.at}-${i}`}>
              <span className="num muted">{formatClock((d.at % 86_400) * 1000)}</span>
              <span className={`badge badge--${d.kind === 'reply' ? 'auto' : 'rare'}`}>{KIND_LABEL[d.kind]}</span>
              <span>{d.reason}</span>
            </li>
          ))}
          {a.log.length === 0 ? <li className="muted">Nessuna decisione ancora.</li> : null}
        </ol>
      </section>
    </div>
  );
}
