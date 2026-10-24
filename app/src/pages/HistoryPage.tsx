import { useInfiniteQuery, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { clearHistory, getHistory, getHistoryStats } from '../api/client';
import { useT, type Key } from '../i18n';
import { BAND_NAMES } from '../lib/format';
import { MODES, PERIODS, formatWhen, type HistoryFilter, type Period } from '../lib/history';

const PAGE = 100;

/** Storico dei messaggi ricevuti: ricerca per nominativo o testo, banda, modo e periodo; si carica a pagine. */
export function HistoryPage() {
  const t = useT();
  const client = useQueryClient();
  const [q, setQ] = useState('');
  const [debouncedQ, setDebouncedQ] = useState('');
  const [band, setBand] = useState('');
  const [mode, setMode] = useState('');
  const [period, setPeriod] = useState<Period>('24h');
  useEffect(() => {
    const id = setTimeout(() => setDebouncedQ(q), 350);
    return () => clearTimeout(id);
  }, [q]);

  const filter: HistoryFilter = { q: debouncedQ, band, mode, period };
  // il periodo si calcola a ogni richiesta: la chiave resta quella dei filtri, non l'orologio
  const rows = useInfiniteQuery({
    queryKey: ['history', filter],
    queryFn: ({ pageParam }) => getHistory(filter, pageParam, PAGE),
    initialPageParam: 0,
    getNextPageParam: (last, pages) => {
      const loaded = pages.reduce((n, p) => n + p.rows.length, 0);
      return loaded < last.total ? loaded : undefined;
    },
    retry: false,
  });
  const stats = useQuery({ queryKey: ['history-stats'], queryFn: getHistoryStats, retry: false, refetchInterval: 30_000 });

  const loaded = rows.data?.pages.flatMap((p) => p.rows) ?? [];
  const total = rows.data?.pages[0]?.total ?? 0;

  const wipe = async () => {
    if (!window.confirm(t('hist.clear.confirm'))) return;
    await clearHistory();
    await client.invalidateQueries({ queryKey: ['history'] });
    await client.invalidateQueries({ queryKey: ['history-stats'] });
  };

  return (
    <div className="history-page">
      <h1 className="page-title">{t('hist.title')}</h1>
      <p className="muted history-help">{t('hist.help')}</p>
      <div className="history-bar">
        <input type="search" aria-label={t('hist.search')} placeholder={t('hist.search')} value={q} onChange={(e) => setQ(e.target.value)} />
        <label className="map-select">
          {t('hist.band')}
          <select value={band} onChange={(e) => setBand(e.target.value)}>
            <option value="">{t('hist.all')}</option>
            {BAND_NAMES.map((b) => (
              <option key={b} value={b}>
                {b}
              </option>
            ))}
          </select>
        </label>
        <label className="map-select">
          {t('hist.mode')}
          <select value={mode} onChange={(e) => setMode(e.target.value)}>
            <option value="">{t('hist.all')}</option>
            {MODES.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </label>
        <label className="map-select">
          {t('hist.period')}
          <select value={period} onChange={(e) => setPeriod(e.target.value as Period)}>
            {PERIODS.map((p) => (
              <option key={p} value={p}>
                {t(`hist.p.${p}` as Key)}
              </option>
            ))}
          </select>
        </label>
        <button type="button" className="glass" onClick={() => void wipe()} disabled={rows.isError}>
          {t('hist.clear')}
        </button>
      </div>

      {rows.isError ? (
        <p className="notice">{t('hist.unavailable')}</p>
      ) : rows.isPending ? (
        <p className="muted">{t('hist.loading')}</p>
      ) : loaded.length === 0 ? (
        <p className="muted">{t('hist.empty')}</p>
      ) : (
        <div className="history-table-wrap">
          <table className="history-table">
            <thead>
              <tr>
                <th>{t('hist.col.when')}</th>
                <th>{t('hist.col.band')}</th>
                <th>{t('hist.col.mode')}</th>
                <th className="num">{t('hist.col.snr')}</th>
                <th className="num">{t('hist.col.freq')}</th>
                <th>{t('hist.col.text')}</th>
                <th>{t('hist.col.country')}</th>
              </tr>
            </thead>
            <tbody>
              {loaded.map((r) => (
                <tr key={r.id} className={r.call ? 'history-row' : undefined} title={r.call ? t('hist.pick') : undefined} onClick={() => r.call && setQ(r.call)}>
                  <td className="data">{formatWhen(r.ts_ms)}</td>
                  <td>{r.band ?? '–'}</td>
                  <td>{r.mode}</td>
                  <td className="num data">{r.snr}</td>
                  <td className="num data">{r.freq_hz}</td>
                  <td className="data">{r.text}</td>
                  <td>{r.country ?? ''}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="history-foot muted">
        {!rows.isError && !rows.isPending ? <span>{t('hist.count', { shown: loaded.length, total })}</span> : null}
        {rows.hasNextPage ? (
          <button type="button" className="glass" onClick={() => void rows.fetchNextPage()} disabled={rows.isFetchingNextPage}>
            {t('hist.more')}
          </button>
        ) : null}
        {stats.data && stats.data.total > 0 ? <span>{t('hist.total', { total: stats.data.total, calls: stats.data.calls, since: stats.data.oldest_ms ? formatWhen(stats.data.oldest_ms) : '–' })}</span> : null}
      </div>
    </div>
  );
}
