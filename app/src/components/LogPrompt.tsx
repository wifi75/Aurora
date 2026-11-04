import '../styles/first-run.css';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { ApiError, answerPendingQso, getPendingQsos, type PendingEdits, type PendingQso } from '../api/client';
import { useT } from '../i18n';

const FIELDS = ['call', 'grid', 'rst_sent', 'rst_rcvd', 'name', 'qth', 'comment'] as const;
type Field = (typeof FIELDS)[number];

/** «Richiedimi di loggare il QSO»: a QSO concluso questa finestra chiede di registrarlo, con i campi correggibili, o di scartarlo. */
export function LogPrompt() {
  const pending = useQuery({ queryKey: ['log-pending'], queryFn: getPendingQsos, refetchInterval: 2000, retry: false });
  const first = pending.data?.[0];
  if (!first) return null;
  // la chiave rimette i campi del QSO nuovo quando quello prima e' stato registrato o scartato
  return <LogPromptCard key={first.id} item={first} waiting={pending.data?.length ?? 1} />;
}

function LogPromptCard({ item, waiting }: { item: PendingQso; waiting: number }) {
  const t = useT();
  const client = useQueryClient();
  const [values, setValues] = useState<Record<Field, string>>(() => Object.fromEntries(FIELDS.map((f) => [f, item.qso[f] ?? ''])) as Record<Field, string>);
  const [error, setError] = useState('');
  const answer = useMutation({
    mutationFn: (action: 'log' | 'discard') => {
      const edits: PendingEdits = {};
      for (const f of FIELDS) if (values[f] !== (item.qso[f] ?? '')) edits[f] = values[f];
      return answerPendingQso(item.id, action, edits);
    },
    onSuccess: () => {
      setError('');
      void client.invalidateQueries({ queryKey: ['log-pending'] });
    },
    onError: (e) => setError(e instanceof ApiError && e.code === 'bad_field' ? t('lp.badField', { field: t(`lp.f.${(e.detail ?? 'call') as Field}`) }) : t('lp.error')),
  });
  // Invio registra (Esc resta a STOP TX, non chiude la finestra)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter' && !answer.isPending && (e.target as HTMLElement | null)?.tagName !== 'TEXTAREA') {
        e.preventDefault();
        answer.mutate('log');
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [answer]);

  const when = item.qso.time_on_unix_ms > 0 ? new Date(item.qso.time_on_unix_ms).toISOString().slice(0, 16).replace('T', ' ') + ' UTC' : '';
  const freq = item.qso.freq_hz > 0 ? `${(item.qso.freq_hz / 1e6).toFixed(3)} MHz` : '';
  return (
    <div className="first-run log-prompt" role="dialog" aria-modal="true" aria-labelledby="log-prompt-title">
      <div className="first-run__card">
        <div className="first-run__glow" aria-hidden="true" />
        <header className="first-run__head">
          <span className="first-run__brand">Aurora</span>
          {waiting > 1 ? <span className="muted">{t('lp.waiting', { n: waiting })}</span> : null}
        </header>
        <div className="first-run__body">
          <h2 id="log-prompt-title">{t('lp.title', { call: item.qso.call })}</h2>
          <p className="first-run__lead">{[item.qso.band, item.qso.mode, freq, when].filter(Boolean).join(' · ')}</p>
          <div className="first-run__fields log-prompt__fields">
            {FIELDS.map((f) => (
              <label key={f} className={f === 'comment' ? 'log-prompt__wide' : undefined}>
                <span>{t(`lp.f.${f}`)}</span>
                <input
                  value={values[f]}
                  autoFocus={f === 'call'}
                  spellCheck={false}
                  autoComplete="off"
                  maxLength={f === 'comment' ? 200 : f === 'name' || f === 'qth' ? 60 : 20}
                  onChange={(e) => setValues({ ...values, [f]: f === 'call' || f === 'grid' ? e.target.value.toUpperCase() : e.target.value })}
                />
              </label>
            ))}
          </div>
          {error ? (
            <p className="live-note" role="alert">
              {error}
            </p>
          ) : null}
        </div>
        <footer className="first-run__foot">
          <button type="button" className="first-run__back" disabled={answer.isPending} onClick={() => answer.mutate('discard')}>
            {t('lp.discard')}
          </button>
          <button type="button" className="first-run__next" disabled={answer.isPending || values.call.trim() === ''} onClick={() => answer.mutate('log')}>
            {t('lp.log')}
          </button>
        </footer>
      </div>
    </div>
  );
}
