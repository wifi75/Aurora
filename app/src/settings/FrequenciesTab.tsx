import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getFrequencies, putFrequencies, type FrequencyEntry } from '../api/client';
import { useT } from '../i18n';
import { BAND_NAMES } from '../lib/format';
import { Help } from './common';
import { Actions, Group, Message, Notes, Page } from './layout';

const MODES = ['FT8', 'FT4'] as const;

/** Scheda Frequenze: la frequenza di dial per banda e modo, usata quando si sceglie una banda. */
export function FrequenciesTab() {
  const t = useT();
  const client = useQueryClient();
  const query = useQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, retry: false });
  const [draft, setDraft] = useState<FrequencyEntry[] | null>(null);
  const save = useMutation({
    mutationFn: (entries: FrequencyEntry[]) => putFrequencies(entries),
    onSuccess: () => {
      setDraft(null);
      void client.invalidateQueries({ queryKey: ['frequencies'] });
    },
  });
  if (query.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!query.data) return <p className="muted">{t('set.loading')}</p>;
  const rows = draft ?? query.data;
  const change = (i: number, patch: Partial<FrequencyEntry>) => setDraft(rows.map((r, k) => (k === i ? { ...r, ...patch } : r)));
  return (
    <Page>
      <Group title={t('tab.frequencies')} span={2}>
        <div className="sx-tablewrap sx-tablewrap--scroll">
          <table className="sx-table">
            <thead>
              <tr>
                <th scope="col">{t('freq.band')}</th>
                <th scope="col">{t('freq.mode')}</th>
                <th scope="col">{t('freq.mhz')}</th>
                <th scope="col" aria-label={t('set.remove')} />
              </tr>
            </thead>
            <tbody>
              {rows.map((r, i) => (
                <tr key={`${r.band}-${r.mode}-${i}`} title={t('tip.freq.row')}>
                  <td>
                    <select aria-label={t('freq.band')} value={r.band} onChange={(e) => change(i, { band: e.target.value })}>
                      {BAND_NAMES.map((b) => (
                        <option key={b} value={b}>
                          {b}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    <select aria-label={t('freq.mode')} value={r.mode} onChange={(e) => change(i, { mode: e.target.value })}>
                      {MODES.map((m) => (
                        <option key={m} value={m}>
                          {m}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    <input
                      type="number"
                      step="0.001"
                      aria-label={t('freq.mhz')}
                      value={(r.hz / 1e6).toFixed(3)}
                      onChange={(e) => change(i, { hz: Math.round(Number(e.target.value) * 1e6) })}
                    />
                  </td>
                  <td>
                    <button type="button" className="glass" aria-label={`${t('set.remove')} ${r.band} ${r.mode}`} onClick={() => setDraft(rows.filter((_, k) => k !== i))}>
                      {t('set.remove')}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Group>
      <Group title={t('lay.actions')}>
        <Actions>
          <button type="button" className="glass" onClick={() => setDraft([...rows, { band: '20m', mode: 'FT8', hz: 14_074_000 }])}>
            {t('freq.addRow')}
          </button>
          <button type="button" className="glass glass--on" disabled={!draft} onClick={() => draft && save.mutate(draft)}>
            {t('set.save')}
          </button>
        </Actions>
        {save.isError ? <Message kind="alert">{t('freq.invalid')}</Message> : null}
      </Group>
      <Notes>
        <Help>{t('freq.help')}</Help>
      </Notes>
    </Page>
  );
}
