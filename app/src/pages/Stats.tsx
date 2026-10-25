import { useQuery } from '@tanstack/react-query';
import { getStats, type StatsResponse } from '../api/client';
import { DxccCard, useDxccMatrix } from '../components/DxccCard';
import { DxccCountries } from '../components/DxccCountries';
import { LogTransfer } from '../components/LogTransfer';
import { useT } from '../i18n';
import { subjectBand, subjectMode } from '../lib/dxccCard';
import { useStation } from '../state/station';

/** Avanzamento nei premi dal log dei QSO e paesi che mancano. */
export function Stats() {
  const t = useT();
  const query = useQuery({ queryKey: ['stats'], queryFn: getStats, retry: false });
  // la scheda DXCC segue la stazione DX attuale (o quella scelta con un doppio clic), sulla banda e nel modo della radio
  const status = useStation((st) => st.status);
  const pendingDx = useStation((st) => st.pendingDx);
  const matrix = useDxccMatrix({});

  return (
    <div className="settings">
      <h1 className="page-title">{t('stats.title')}</h1>
      <section className="panel settings-block dxcc-section" title={t('dxcc.tip.card')}>
        <h2 className="panel-title">{t('dxcc.title')}</h2>
        {matrix.isError ? (
          <p className="notice">{t('dxcc.unavailable')}</p>
        ) : (
          <>
            <DxccCard
              call={pendingDx || status?.dx_call || ''}
              grid={pendingDx ? undefined : (status?.dx_grid ?? undefined)}
              band={subjectBand(status?.dial_hz)}
              mode={subjectMode(status?.mode)}
              matrix={matrix.data}
            />
            {matrix.data ? (
              <details className="dxcc-details">
                <summary>{t('dxcc.countries.title')}</summary>
                <DxccCountries matrix={matrix.data} />
              </details>
            ) : null}
          </>
        )}
      </section>
      {query.isError ? <p className="notice">{t('stats.unavailable')}</p> : query.data ? <StatsBlocks data={query.data} /> : <p className="muted">{t('set.loading')}</p>}
      <section className="panel settings-block">
        <h2 className="panel-title">{t('logio.title')}</h2>
        <LogTransfer />
      </section>
    </div>
  );
}

function StatsBlocks({ data }: { data: StatsResponse }) {
  const t = useT();
  const { stats: s, missing_dxcc: missing } = data;

  const rows: Array<[string, string]> = [
    [t('stats.dxcc'), t('stats.of', { n: s.dxcc_worked, total: s.dxcc_total })],
    [t('stats.cq'), t('stats.of', { n: s.cq_zones_worked, total: s.cq_zones_total })],
    [t('stats.itu'), t('stats.of', { n: s.itu_zones_worked, total: s.itu_zones_total })],
    [t('stats.grids'), String(s.grids)],
    [t('stats.prefixes'), String(s.prefixes)],
    [t('stats.qsos'), String(s.qsos)],
    [t('stats.calls'), String(s.calls)],
  ];

  return (
    <>
      <section className="panel settings-block" title={t('tip.stats.award')}>
        <h2 className="panel-title">{t('stats.awards')}</h2>
        <dl className="kv">
          {rows.map(([label, value]) => (
            <div key={label} className="kv-row">
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
      </section>
      <section className="panel settings-block">
        <h2 className="panel-title">{t('stats.byBand')}</h2>
        <table className="stats-table">
          <thead>
            <tr>
              <th>{t('stats.band')}</th>
              <th>{t('stats.bandDxcc')}</th>
              <th>{t('stats.bandQsos')}</th>
            </tr>
          </thead>
          <tbody>
            {s.bands.map((b) => (
              <tr key={b.band}>
                <td>{b.band}</td>
                <td>{b.dxcc}</td>
                <td>{b.qsos}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      <section className="panel settings-block">
        <h2 className="panel-title">{t('stats.missing')}</h2>
        {missing.length === 0 ? (
          <p className="muted">{t('stats.missingNone')}</p>
        ) : (
          <ul className="chips">
            {missing.map((name) => (
              <li key={name} className="pill">
                {name}
              </li>
            ))}
          </ul>
        )}
      </section>
    </>
  );
}
