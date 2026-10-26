import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getFrequencies, getFrequencyDefaults, putFrequencies, type FrequencyEntry, type IaruRegion } from '../api/client';
import { useT, type Key } from '../i18n';
import { BAND_NAMES } from '../lib/format';
import { Help } from './common';
import { Actions, Group, Message, Notes, Page } from './layout';
import { useSettings } from './NativeSettings';

const MODES = ['FT8', 'FT4', 'JT65', 'JT9'] as const;
const REGIONS: readonly { value: IaruRegion; label: Key }[] = [
  { value: 'ALL', label: 'gen2.regionAll' },
  { value: 'R1', label: 'gen2.region1' },
  { value: 'R2', label: 'gen2.region2' },
  { value: 'R3', label: 'gen2.region3' },
];

/** Una riga si vede se la regione scelta e' «tutte», se la riga vale ovunque o se e' della regione scelta (come il filtro di JTDX). */
export function seenBy(chosen: IaruRegion, row: IaruRegion): boolean {
  return chosen === 'ALL' || row === 'ALL' || row === chosen;
}

/** Scheda Frequenze: la frequenza di dial per banda e modo, usata quando si sceglie una banda; la regione IARU (Generali) nasconde le righe delle altre regioni. */
export function FrequenciesTab() {
  const t = useT();
  const client = useQueryClient();
  const query = useQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, retry: false });
  const settings = useSettings();
  const chosen = (settings.data?.values.Region ?? 'ALL') as IaruRegion;
  const [draft, setDraft] = useState<FrequencyEntry[] | null>(null);
  const save = useMutation({
    mutationFn: (entries: FrequencyEntry[]) => putFrequencies(entries),
    onSuccess: () => {
      setDraft(null);
      void client.invalidateQueries({ queryKey: ['frequencies'] });
    },
  });
  const defaults = useMutation({ mutationFn: getFrequencyDefaults, onSuccess: (rows) => setDraft(rows) });
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
                <th scope="col">{t('freq.region')}</th>
                <th scope="col">{t('freq.preferred')}</th>
                <th scope="col" aria-label={t('set.remove')} />
              </tr>
            </thead>
            <tbody>
              {rows.map((r, i) => {
                const hidden = !seenBy(chosen, r.region);
                return (
                  <tr key={`${r.band}-${r.mode}-${r.hz}-${i}`} className={hidden ? 'sx-row-hidden' : undefined} title={hidden ? t('freq.hidden') : t('tip.freq.row')}>
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
                      <select aria-label={t('freq.region')} value={r.region} onChange={(e) => change(i, { region: e.target.value as IaruRegion })}>
                        {REGIONS.map((o) => (
                          <option key={o.value} value={o.value}>
                            {t(o.label)}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>
                      <input type="checkbox" aria-label={t('freq.preferred')} checked={r.preferred} onChange={(e) => change(i, { preferred: e.target.checked })} />
                    </td>
                    <td>
                      <button type="button" className="glass" aria-label={`${t('set.remove')} ${r.band} ${r.mode} ${(r.hz / 1e6).toFixed(3)}`} onClick={() => setDraft(rows.filter((_, k) => k !== i))}>
                        {t('set.remove')}
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </Group>
      <Group title={t('lay.actions')}>
        <Actions>
          <button type="button" className="glass" onClick={() => setDraft([...rows, { band: '20m', mode: 'FT8', hz: 14_074_000, region: 'ALL', preferred: true }])}>
            {t('freq.addRow')}
          </button>
          <button type="button" className="glass" disabled={defaults.isPending} onClick={() => defaults.mutate()}>
            {t('freq.reset')}
          </button>
          <button type="button" className="glass glass--on" disabled={!draft} onClick={() => draft && save.mutate(draft)}>
            {t('set.save')}
          </button>
        </Actions>
        {save.isError ? <Message kind="alert">{t('freq.invalid')}</Message> : null}
        {defaults.isError ? <Message kind="alert">{t('set.loadError')}</Message> : null}
      </Group>
      <Notes>
        <Help>{t('freq.help')}</Help>
      </Notes>
    </Page>
  );
}
