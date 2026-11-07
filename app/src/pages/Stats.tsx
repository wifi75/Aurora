import { useQuery } from '@tanstack/react-query';
import { getStats, type StatsResponse } from '../api/client';
import { DxccCard, useDxccMatrix } from '../components/DxccCard';
import { DxccCountries } from '../components/DxccCountries';
import { LogTransfer } from '../components/LogTransfer';
import { useT } from '../i18n';
import { subjectBand, subjectMode } from '../lib/dxccCard';
import { useStation, useStatusPick } from '../state/station';

/**
 * Avanzamento nei premi dal log dei QSO. Prima era una colonna di riquadri larghi tutta la pagina: ora i numeri in alto, bande e scheda
 * DXCC affiancate, e quello che si consulta di rado (paesi mancanti, log ADIF) chiuso finche' non si apre.
 */
export function Stats() {
  const t = useT();
  const query = useQuery({ queryKey: ['stats'], queryFn: getStats, retry: false });
  // la scheda DXCC segue la stazione DX attuale (o quella scelta con un doppio clic), sulla banda e nel modo della radio
  const status = useStatusPick(['dial_hz', 'dx_call', 'dx_grid', 'mode']);
  const pendingDx = useStation((st) => st.pendingDx);
  const matrix = useDxccMatrix({});

  return (
    <div className="stats-page">
      <h1 className="page-title">{t('stats.title')}</h1>
      {/* mentre il log si legge le schede sono gia' tutte al loro posto (con «…» al posto dei numeri): prima la scheda DXCC compariva
          a sinistra e in alto e poi saltava nella sua colonna quando arrivavano i dati */}
      {query.isError ? <p className="notice">{t('stats.unavailable')}</p> : <Numbers data={query.data} />}

      <div className="stats-columns">
        {query.isError ? null : <Bands data={query.data} />}
        <section className="panel stats-card dxcc-section" title={t('dxcc.tip.card')}>
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
      </div>

      {query.isError ? null : <Missing data={query.data} />}

      <details className="panel stats-card stats-fold" open>
        <summary className="panel-title">{t('logio.title')}</summary>
        <LogTransfer />
      </details>
    </div>
  );
}

/** I numeri principali in riquadri, con la barra di avanzamento dove c'e' un totale. */
function Numbers({ data }: { data?: StatsResponse }) {
  const t = useT();
  const s = data?.stats;
  const tiles: Array<{ label: string; value: number | null; total?: number }> = [
    { label: t('stats.qsos'), value: s?.qsos ?? null },
    { label: t('stats.calls'), value: s?.calls ?? null },
    { label: t('stats.dxcc'), value: s?.dxcc_worked ?? null, total: s?.dxcc_total },
    { label: t('stats.cq'), value: s?.cq_zones_worked ?? null, total: s?.cq_zones_total },
    { label: t('stats.itu'), value: s?.itu_zones_worked ?? null, total: s?.itu_zones_total },
    { label: t('stats.grids'), value: s?.grids ?? null },
    { label: t('stats.prefixes'), value: s?.prefixes ?? null },
  ];
  return (
    <section className="stats-tiles" aria-label={t('stats.awards')} title={t('tip.stats.award')}>
      {tiles.map((tile) => (
        <div key={tile.label} className="stats-tile" aria-busy={tile.value === null}>
          <strong className="data">
            {tile.value === null ? '…' : tile.value.toLocaleString()}
            {tile.total ? <small className="muted"> / {tile.total}</small> : null}
          </strong>
          <span className="muted">{tile.label}</span>
          {tile.total && tile.value !== null ? (
            <span className="stats-progress" aria-hidden="true">
              <i style={{ width: `${Math.min(100, (tile.value / tile.total) * 100)}%` }} />
            </span>
          ) : null}
        </div>
      ))}
    </section>
  );
}

/** Per banda: paesi e QSO con una barra, per vedere a colpo d'occhio dove si e' lavorato di piu'. */
const WAIT_BANDS = ['160m', '80m', '40m', '30m', '20m', '17m', '15m', '12m', '10m', '6m'];

function Bands({ data }: { data?: StatsResponse }) {
  const t = useT();
  const bands = data?.stats.bands ?? WAIT_BANDS.map((band) => ({ band, dxcc: Number.NaN, qsos: Number.NaN }));
  const maxQsos = Math.max(1, ...bands.map((b) => (Number.isNaN(b.qsos) ? 0 : b.qsos)));
  return (
    <section className="panel stats-card" aria-busy={!data}>
      <h2 className="panel-title">{t('stats.byBand')}</h2>
      <table className="stats-table stats-bands">
        <thead>
          <tr>
            <th>{t('stats.band')}</th>
            <th className="num" title={t('tip.ui.colCountries')}>{t('stats.bandDxcc')}</th>
            <th className="num">{t('stats.bandQsos')}</th>
            <th aria-hidden="true" />
          </tr>
        </thead>
        <tbody>
          {bands.map((b) => (
            <tr key={b.band}>
              <td className="data">{b.band}</td>
              <td className="num data">{Number.isNaN(b.dxcc) ? '…' : b.dxcc}</td>
              <td className="num data">{Number.isNaN(b.qsos) ? '…' : b.qsos.toLocaleString()}</td>
              <td className="stats-bar-cell" aria-hidden="true">
                <span className="stats-progress">
                  <i style={{ width: `${Number.isNaN(b.qsos) ? 0 : (b.qsos / maxQsos) * 100}%` }} />
                </span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

/** I paesi che mancano: chiusi, con il conteggio nel titolo. */
function Missing({ data }: { data?: StatsResponse }) {
  const t = useT();
  const missing = data?.missing_dxcc ?? null;
  return (
    <details className="panel stats-card stats-fold">
      <summary className="panel-title">
        {t('stats.missing')} <span className="muted">({missing ? missing.length : '…'})</span>
      </summary>
      {missing === null ? (
        <p className="muted">{t('set.loading')}</p>
      ) : missing.length === 0 ? (
        <p className="muted">{t('stats.missingNone')}</p>
      ) : (
        <ul className="chips stats-missing">
          {missing.map((name) => (
            <li key={name} className="pill">
              {name}
            </li>
          ))}
        </ul>
      )}
    </details>
  );
}
