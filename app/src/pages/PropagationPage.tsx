import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getOnline, getPsk, getSolar, refreshOnline, type SolarData } from '../api/client';
import { useT, type Key } from '../i18n';
import { bandOf, formatClock } from '../lib/format';
import { conditionGroup, pskRows, summarize } from '../lib/psk';
import { useStation } from '../state/station';

const GROUPS = ['80m-40m', '30m-20m', '17m-15m', '12m-10m'] as const;

function Conditions({ data, group }: { data: SolarData; group: string | null }) {
  const t = useT();
  const find = (name: string, time: string) => data.bands.find((b) => b.name === name && b.time === time)?.condition ?? '';
  const label = (condition: string) => (['Good', 'Fair', 'Poor'].includes(condition) ? t(`prop.cond.${condition}` as Key) : condition || '–');
  return (
    <table className="prop-table">
      <thead>
        <tr>
          <th />
          <th>{t('prop.day')}</th>
          <th>{t('prop.night')}</th>
        </tr>
      </thead>
      <tbody>
        {GROUPS.map((g) => (
          <tr key={g} className={g === group ? 'prop-row--mine' : undefined}>
            <th scope="row">{g}</th>
            {(['day', 'night'] as const).map((time) => {
              const condition = find(g, time);
              return (
                <td key={time}>
                  <span className={`prop-cond prop-cond--${condition.toLowerCase() || 'none'}`}>{label(condition)}</span>
                </td>
              );
            })}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Index({ name, value }: { name: string; value: string | number | null | undefined }) {
  return (
    <div className="prop-index">
      <span className="muted">{name}</span>
      <strong className="data">{value === null || value === undefined || value === '' ? '–' : value}</strong>
    </div>
  );
}

/** Propagazione e ricezione: condizioni calcolate (hamqsl.com) e chi ti ha ricevuto (PSK Reporter). Tutto a richiesta: spento finche' non si accende. */
export function PropagationPage() {
  const t = useT();
  const client = useQueryClient();
  const status = useStation((s) => s.status);
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const solar = useQuery({ queryKey: ['online-solar'], queryFn: getSolar, retry: false, refetchInterval: 60_000, enabled: online.data?.solar === true });
  const psk = useQuery({ queryKey: ['online-psk'], queryFn: getPsk, retry: false, refetchInterval: 60_000, enabled: online.data?.psk === true });
  const refresh = useMutation({
    mutationFn: refreshOnline,
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['online-solar'] });
      void client.invalidateQueries({ queryKey: ['online-psk'] });
    },
  });

  if (online.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!online.data) return <p className="muted">…</p>;
  if (!online.data.psk && !online.data.solar) {
    return (
      <div className="prop-page">
        <h1 className="page-title">{t('prop.title')}</h1>
        <p className="muted">{t('prop.off')}</p>
        <a className="glass glass--link" href="#/impostazioni/online">
          {t('prop.settings')}
        </a>
      </div>
    );
  }

  const data = solar.data?.solar.data ?? null;
  const solarError = solar.data?.solar.error ?? null;
  const pskState = psk.data?.psk ?? null;
  const rows = pskState ? pskRows(pskState.reports, status?.de_grid) : [];
  const summary = summarize(rows);
  const group = conditionGroup(status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null);

  return (
    <div className="prop-page">
      <h1 className="page-title">{t('prop.title')}</h1>
      <div>
        <button type="button" className="glass" onClick={() => refresh.mutate()} disabled={refresh.isPending}>
          {t('prop.refresh')}
        </button>
      </div>

      {online.data.solar ? (
        <section className="prop-card" aria-label={t('prop.solar')}>
          <h2>{t('prop.solar')}</h2>
          {data ? (
            <>
              <div className="prop-indices">
                <Index name={t('prop.sfi')} value={data.sfi} />
                <Index name={t('prop.a')} value={data.a_index} />
                <Index name={t('prop.k')} value={data.k_index} />
                <Index name={t('prop.xray')} value={data.xray} />
                <Index name={t('prop.spots')} value={data.sunspots} />
                <Index name={t('prop.wind')} value={data.solar_wind === null ? null : `${data.solar_wind.toFixed(0)} km/s`} />
                <Index name={t('prop.geomag')} value={data.geomag} />
                <Index name={t('prop.noise')} value={data.signal_noise} />
              </div>
              <Conditions data={data} group={group} />
              <p className="muted prop-credit">
                {t('prop.updated', { when: data.updated || '–' })} · {t('prop.credit')}
              </p>
            </>
          ) : solarError ? (
            <p className="notice">{t('prop.error', { message: solarError })}</p>
          ) : (
            <p className="muted">{t('prop.waiting')}</p>
          )}
        </section>
      ) : null}

      {online.data.psk ? (
        <section className="prop-card" aria-label={t('prop.psk')}>
          <h2>{t('prop.psk')}</h2>
          {pskState && pskState.fetched_ms > 0 ? (
            rows.length === 0 ? (
              <p className="muted">{t('prop.psk.none', { hours: pskState.hours })}</p>
            ) : (
              <>
                <p className="prop-summary">
                  {summary.farthest
                    ? t('prop.psk.summary', { n: summary.stations, hours: pskState.hours, call: summary.farthest.call, km: summary.farthest.km })
                    : t('prop.psk.summaryNoDist', { n: summary.stations, hours: pskState.hours })}
                </p>
                <div className="history-table-wrap">
                  <table className="history-table">
                    <thead>
                      <tr>
                        <th>{t('prop.col.when')}</th>
                        <th>{t('prop.col.who')}</th>
                        <th>{t('prop.col.grid')}</th>
                        <th>{t('prop.col.band')}</th>
                        <th>{t('prop.col.mode')}</th>
                        <th className="num">{t('prop.col.snr')}</th>
                        <th className="num">{t('prop.col.km')}</th>
                        <th>{t('prop.col.country')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {rows.map((r) => (
                        <tr key={`${r.receiver}-${r.time_s}-${r.freq_hz}`}>
                          <td className="data">{formatClock(r.time_s * 1000)}</td>
                          <td className="data">{r.receiver}</td>
                          <td className="data">{r.locator}</td>
                          <td>{r.band ?? '–'}</td>
                          <td>{r.mode}</td>
                          <td className="num data">{r.snr ?? '–'}</td>
                          <td className="num data">{r.km ?? '–'}</td>
                          <td>{r.country ?? ''}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )
          ) : pskState?.error ? (
            <p className="notice">{t('prop.error', { message: pskState.error })}</p>
          ) : (
            <p className="muted">{t('prop.waiting')}</p>
          )}
        </section>
      ) : null}
    </div>
  );
}
