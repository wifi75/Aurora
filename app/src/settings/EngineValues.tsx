import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, applyEngineConfig, getEngineConfig } from '../api/client';
import { useT, type Key, type T } from '../i18n';
import { ENGINE_GROUPS, type EngineGroup, type EngineItem } from '../lib/engineGroups';
import { useEngineDraft } from '../lib/engineDraft';
import { Help } from './common';

/** I valori ammessi per le voci a scelta (come li scrive il file del motore). */
const CHOICES: Record<string, readonly string[]> = { 'enum:save': ['0', '1', '2'], 'enum:reply': ['1', '2', '3'], 'enum:depth': ['1', '2', '3'] };

/** Il valore di una voce a scelta come testo leggibile; un valore sconosciuto resta com'e'. */
export function choiceLabel(kind: string, raw: string, t: T): string {
  const key = `jx.${kind.slice(5)}.${raw}` as Key;
  const text = t(key);
  return text === key ? raw : text;
}

function errorText(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'transmitting') return t('jx.err.transmitting');
    if (error.code === 'local_only') return t('jx.err.local');
    if (error.code === 'bad_setting') return t('jx.err.bad');
  }
  return t('jx.err.failed');
}

function Field({ item, current, draft, label }: { item: EngineItem; current: string | undefined; draft: string | undefined; label: string }) {
  const t = useT();
  const set = useEngineDraft((s) => s.set);
  const value = draft ?? current ?? '';
  const change = (next: string) => set(item.key, next, current);
  const id = `jx-${item.key}`;
  let control;
  if (item.kind === 'bool') {
    control = <input id={id} type="checkbox" checked={value === 'true' || value === '1'} onChange={(e) => change(e.target.checked ? 'true' : 'false')} />;
  } else if (item.kind === 'num') {
    control = <input id={id} type="number" className="set-number" value={value} onChange={(e) => change(e.target.value)} />;
  } else if (item.kind === 'text') {
    control = <input id={id} type="text" className="jx-text" value={value} maxLength={200} onChange={(e) => change(e.target.value)} />;
  } else {
    const choices = CHOICES[item.kind] ?? [];
    const options = value !== '' && !choices.includes(value) ? [value, ...choices] : choices;
    control = (
      <select id={id} value={value} onChange={(e) => change(e.target.value)}>
        {value === '' ? <option value="" /> : null}
        {options.map((o) => (
          <option key={o} value={o}>
            {choiceLabel(item.kind, o, t)}
          </option>
        ))}
      </select>
    );
  }
  return (
    <label className={`jx-row${draft !== undefined ? ' jx-row--changed' : ''}`} htmlFor={id} title={t('tip.jx')}>
      <span className="jx-name">{label}</span>
      {control}
    </label>
  );
}

/** Barra in fondo: quante modifiche aspettano, con "Applica e riavvia" e "Annulla". */
function ApplyBar() {
  const t = useT();
  const client = useQueryClient();
  const changes = useEngineDraft((s) => s.changes);
  const clear = useEngineDraft((s) => s.clear);
  const [note, setNote] = useState('');
  const apply = useMutation({
    mutationFn: () => applyEngineConfig(changes),
    onSuccess: () => {
      clear();
      setNote(t('jx.applied'));
      // il motore impiega qualche secondo a riaprirsi: poi si rilegge il suo file
      setTimeout(() => void client.invalidateQueries({ queryKey: ['engine-config'] }), 6000);
    },
    onError: (e) => setNote(errorText(e, t)),
  });
  const count = Object.keys(changes).length;
  if (count === 0 && !note) return null;
  return (
    <div className="jx-bar" role="status">
      {count > 0 ? <strong>{t('jx.pending', { n: count })}</strong> : null}
      {count > 0 ? (
        <>
          <button
            type="button"
            className="glass glass--on"
            disabled={apply.isPending}
            onClick={() => {
              if (window.confirm(t('jx.confirm'))) apply.mutate();
            }}
          >
            {t('jx.apply')}
          </button>
          <button type="button" className="glass" disabled={apply.isPending} onClick={() => { clear(); setNote(''); }}>
            {t('jx.discard')}
          </button>
        </>
      ) : null}
      {note ? <span className="live-note">{note}</span> : null}
    </div>
  );
}

/** Le impostazioni del motore (il motore) di un gruppo, lette dal suo file e modificabili qui. */
export function EngineValues({ group, title }: { group: EngineGroup; title?: string }) {
  const t = useT();
  const config = useQuery({ queryKey: ['engine-config'], queryFn: getEngineConfig, retry: false, refetchInterval: 15_000 });
  const changes = useEngineDraft((s) => s.changes);
  return (
    <section aria-label={title ?? t('jx.title')}>
      <h3 className="set-sub">{title ?? t('jx.title')}</h3>
      {config.isError ? (
        <p className="muted">{t('jx.unavailable')}</p>
      ) : !config.data ? (
        <p className="muted">{t('set.loading')}</p>
      ) : (
        <>
          <div className="jx-list">
            {ENGINE_GROUPS[group].map((item) => (
              <Field key={item.key} item={item} current={config.data[item.key]} draft={changes[item.key]} label={t(`jx.${item.key}` as Key)} />
            ))}
          </div>
          <Help>{t('jx.note')}</Help>
          <ApplyBar />
        </>
      )}
    </section>
  );
}
