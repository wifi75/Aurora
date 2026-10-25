import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { getDxccMatrix } from '../api/client';
import { useT, type Key } from '../i18n';
import {
  COUNTRY_LABELS,
  LABEL_IDS,
  LABEL_TEXT,
  subjectBand,
  subjectMode,
  bandHeader,
  cellKind,
  totalKind,
  type DxccLabels,
  type DxccMatrix,
  type DxccQuery,
  type LabelId,
} from '../lib/dxccCard';
import { useStation } from '../state/station';

/** La scheda DXCC dal core; si rilegge da sola quando si registra un QSO (evento `qso_logged`). */
export function useDxccMatrix(query: DxccQuery, enabled = true) {
  const logged = useStation((s) => s.qsosLogged);
  return useQuery({
    queryKey: ['dxcc-matrix', query.call ?? '', query.grid ?? '', query.band ?? '', query.mode ?? '', query.compact === true, logged],
    queryFn: () => getDxccMatrix(query),
    enabled,
    retry: false,
    staleTime: 10_000,
    placeholderData: keepPreviousData,
  });
}

interface LabelsProps {
  labels: DxccLabels | null | undefined;
  /** Le tre didascalie sotto le colonne (paese, banda, modo con il nominativo), come nella scheda di riferimento. */
  captions?: readonly [string, string, string];
  /** Senza dati (nessuna stazione o scheda non disponibile) le etichette restano spente. */
  className?: string;
}

/** Le sei etichette: il paese sopra (ambra), il locatore sotto (ciano); accese quelle che valgono per la stazione. */
export function DxccLabelStrip({ labels, captions, className = '' }: LabelsProps) {
  const t = useT();
  return (
    <ul className={`dxcc-labels ${className}`.trim()} aria-label={t('dxcc.labels')}>
      {LABEL_IDS.map((id: LabelId) => {
        const on = labels?.[id] === true;
        return (
          <li key={id} className={`dxcc-label dxcc-label--${COUNTRY_LABELS.includes(id) ? 'country' : 'grid'}`} data-on={on ? 'true' : 'false'} data-label={id} title={t(`dxcc.tip.${id}` as Key)}>
            {LABEL_TEXT[id]}
            <span className="visually-hidden">{on ? ' ✓' : ''}</span>
          </li>
        );
      })}
      {captions?.map((text, i) => (
        <li key={`cap${i}`} className="dxcc-caption" aria-hidden="true" style={{ gridColumn: i + 1, gridRow: 3 }}>
          {text}
        </li>
      ))}
    </ul>
  );
}

interface MatrixProps {
  matrix: DxccMatrix;
  /** Le caselle di una stazione (il suo paese); senza, la scheda mostra quanti paesi sono stati lavorati in ogni casella. */
  station: Record<string, Record<string, 'worked' | 'unconfirmed'>> | null;
}

/** La matrice: righe FT8, FT4, DIG; colonne 160...4 e U; in alto a sinistra il totale dei paesi lavorati. */
export function DxccGrid({ matrix, station }: MatrixProps) {
  const t = useT();
  const rows = matrix.rows;
  return (
    <table className="dxcc-grid" aria-label={t('dxcc.title')}>
      <thead>
        <tr>
          <th className="dxcc-total" scope="col" title={t('dxcc.total', { worked: matrix.worked, total: matrix.total })}>
            {matrix.worked}
          </th>
          {matrix.bands.map((b) => (
            <th key={b} scope="col" className="dxcc-band" title={b === 'U' ? t('dxcc.bandU') : b}>
              {bandHeader(b)}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row}>
            <th scope="row" className="dxcc-row" title={row === 'DIG' ? t('dxcc.rowDig') : t(`dxcc.row.${row}` as Key)}>
              {row}
            </th>
            {matrix.bands.map((band) => {
              const total = matrix.totals[row]?.[band];
              const kind = station ? cellKind(station[row]?.[band]) : totalKind(total);
              const text = t(`dxcc.cell.${kind}` as Key);
              const count = !station && total ? total.countries : 0;
              return (
                <td key={band} className="dxcc-cell" data-kind={kind} title={`${row} ${band}: ${count > 0 ? `${t('dxcc.cell.countries', { n: count })}, ` : ''}${text}`}>
                  {count > 0 ? <span className="dxcc-count">{count}</span> : <span className="visually-hidden">{text}</span>}
                </td>
              );
            })}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

interface CardProps {
  /** La stazione di cui dire le etichette (di norma la stazione DX attuale). */
  call?: string;
  grid?: string;
  band?: string;
  mode?: string;
  /** La scheda gia' letta, se chi la usa la legge per altri scopi (altrimenti la legge da se'). */
  matrix?: DxccMatrix;
  /** Nel pannello della pagina Stazione: larga quanto il pannello. */
  panel?: boolean;
}

/** La scheda: etichette della stazione, caselle del suo paese (o i totali) e il totale dei paesi lavorati. */
export function DxccCard({ call = '', grid, band, mode, matrix: given, panel = false }: CardProps) {
  const t = useT();
  const [typed, setTyped] = useState('');
  const [view, setView] = useState<'auto' | 'station' | 'totals'>('auto');
  const own = useDxccMatrix({}, given === undefined);
  const matrix = given ?? own.data;
  const chosen = typed.trim() || call.trim();
  // con un nominativo scritto a mano si vale solo il nominativo: banda e modo restano quelli della radio, il locatore no
  const subjectQuery: DxccQuery = { call: chosen, grid: typed.trim() ? undefined : grid, band, mode, compact: true };
  const subject = useDxccMatrix(subjectQuery, chosen !== '');
  const s = chosen !== '' ? (subject.data?.subject ?? null) : null;
  const showStation = (view === 'auto' ? s?.country != null : view === 'station') && s !== null && s.country !== null;

  if (own.isError && given === undefined) return <p className="notice">{t('dxcc.unavailable')}</p>;
  if (!matrix) return <p className="muted">{t('set.loading')}</p>;

  return (
    <div className={`dxcc-card${panel ? ' dxcc-card--panel' : ''}`} title={t('dxcc.tip.card')}>
      <DxccLabelStrip
        labels={s?.labels}
        captions={[
          `${s?.country ?? t('dxcc.cap.country')} - ${(s?.call ?? chosen) || t('dxcc.cap.call')}`,
          `${band ?? t('dxcc.cap.band')} - ${(s?.call ?? chosen) || t('dxcc.cap.call')}`,
          `${mode ?? t('dxcc.cap.mode')} - ${(s?.call ?? chosen) || t('dxcc.cap.call')}`,
        ]}
      />
      <div className="dxcc-subject">
        <span className="data dxcc-subject-call">
          {t('dxcc.station')}: <strong>{s ? s.call : chosen || t('dxcc.none')}</strong>
          {s ? <span className="muted"> · {s.country ?? t('dxcc.unknownCountry')}</span> : null}
        </span>
        <input
          type="text"
          className="dxcc-try"
          value={typed}
          placeholder={t('dxcc.tryCall')}
          aria-label={t('dxcc.tryCall')}
          maxLength={20}
          autoCapitalize="characters"
          spellCheck={false}
          onChange={(e) => setTyped(e.target.value.toUpperCase())}
        />
        <div className="dxcc-view" role="group">
          {(['station', 'totals'] as const).map((v) => (
            <button key={v} type="button" className={`glass${(showStation ? 'station' : 'totals') === v ? ' glass--on' : ''}`} aria-pressed={(showStation ? 'station' : 'totals') === v} disabled={v === 'station' && !(s && s.country)} onClick={() => setView(v)}>
              {t(`dxcc.view.${v}` as Key)}
            </button>
          ))}
        </div>
      </div>
      <DxccGrid matrix={matrix} station={showStation && s ? s.cells : null} />
      <p className="muted dxcc-foot">
        {t('dxcc.total', { worked: matrix.worked, total: matrix.total })} · {t('dxcc.qsos', { n: matrix.qsos })}
      </p>
      <p className="muted dxcc-legend">{t('dxcc.legend')}</p>
    </div>
  );
}

interface LiveProps {
  /** La stazione DX attuale (nominativo, locatore) e la frequenza e il modo della radio: banda e modo delle etichette. */
  call: string;
  grid?: string | null;
  dialHz?: number | null;
  mode?: string | null;
}

/** Le sei etichette della stazione DX attuale, in piccolo: stanno nel blocco della stazione DX. Senza nominativo restano spente. */
export function DxccLive({ call, grid, dialHz, mode }: LiveProps) {
  const t = useT();
  const who = call.trim().toUpperCase();
  const query = useDxccMatrix({ call: who, grid: grid ?? undefined, band: subjectBand(dialHz), mode: subjectMode(mode), compact: true }, who !== '');
  const subject = who !== '' ? (query.data?.subject ?? null) : null;
  // scheda non disponibile (senza cty.dat o senza log): niente da mostrare
  if (query.isError) return null;
  return (
    <div className="dxcc-live" aria-label={t('dxcc.title')}>
      <DxccLabelStrip labels={subject?.labels} className="dxcc-labels--small" />
      {subject?.country ? <span className="muted dxcc-live-country">{subject.country}</span> : null}
    </div>
  );
}
