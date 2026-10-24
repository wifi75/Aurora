import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getSchedule, putSchedule, type ScheduleState } from '../api/client';
import { useT } from '../i18n';
import { BAND_NAMES } from '../lib/format';
import { Check, Help } from './common';

const ROWS = 5;

/** Scheda Programmazione: cambio banda automatico a orari UTC (fino a cinque righe). */
export function ScheduleTab() {
  const t = useT();
  const client = useQueryClient();
  const query = useQuery({ queryKey: ['schedule'], queryFn: getSchedule, retry: false });
  const [draft, setDraft] = useState<ScheduleState | null>(null);
  const save = useMutation({
    mutationFn: (s: ScheduleState) => putSchedule(s),
    onSuccess: () => {
      setDraft(null);
      void client.invalidateQueries({ queryKey: ['schedule'] });
    },
  });
  if (query.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!query.data) return <p className="muted">{t('set.loading')}</p>;
  const state = draft ?? query.data;
  const slots = Array.from({ length: ROWS }, (_, i) => state.entries[i] ?? null);
  const setRow = (i: number, patch: Partial<{ hour: number; minute: number; band: string }> | null) => {
    const next = [...slots];
    next[i] = patch === null ? null : { hour: 0, minute: 0, band: '20m', ...(slots[i] ?? {}), ...patch };
    setDraft({ ...state, entries: next.filter((x): x is NonNullable<typeof x> => x !== null) });
  };
  return (
    <>
      <Help>{t('sch.help')}</Help>
      <Check label={t('sch.use')} hint={t('tip.sch.use')} checked={state.enabled} onChange={(v) => setDraft({ ...state, enabled: v })} />
      <table className="freq-table">
        <thead>
          <tr>
            <th>{t('sch.hour')}</th>
            <th>{t('sch.minute')}</th>
            <th>{t('sch.band')}</th>
          </tr>
        </thead>
        <tbody>
          {slots.map((row, i) => (
            <tr key={i}>
              <td>
                <input type="number" min={0} max={23} aria-label={`${t('sch.hour')} ${i + 1}`} value={row?.hour ?? ''} placeholder="hh" onChange={(e) => setRow(i, { hour: Number(e.target.value) })} />
              </td>
              <td>
                <input type="number" min={0} max={59} aria-label={`${t('sch.minute')} ${i + 1}`} value={row?.minute ?? ''} placeholder="mm" onChange={(e) => setRow(i, { minute: Number(e.target.value) })} />
              </td>
              <td>
                <select aria-label={`${t('sch.band')} ${i + 1}`} value={row?.band ?? ''} onChange={(e) => setRow(i, e.target.value === '' ? null : { band: e.target.value })}>
                  <option value="">{t('sch.none')}</option>
                  {BAND_NAMES.map((b) => (
                    <option key={b} value={b}>
                      {b}
                    </option>
                  ))}
                </select>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <button type="button" className="glass glass--on" disabled={!draft} onClick={() => draft && save.mutate(draft)}>
        {t('set.save')}
      </button>
      {save.isError ? (
        <p className="live-note" role="alert">
          {t('sch.invalid')}
        </p>
      ) : null}
    </>
  );
}
