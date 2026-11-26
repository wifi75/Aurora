import { useT } from '../../../i18n';
import type { TextContestScore } from '../../../api/contestTesto';
import { multSummary } from '../../../lib/contestEntry';
import { multsByMode } from '../../../lib/contestMultGrid';
import { multName, scoreLine } from '../ContestStatsDialog';

/** Punti per QSO (la media), una cifra decimale; trattino se non ci sono QSO. */
export const avgPoints = (punti: number, qso: number): string => (qso > 0 ? (punti / qso).toFixed(1) : '-');

/**
 * Le statistiche come tabella vera: una riga per banda con QSO, doppi, punti, media (punti per QSO) e moltiplicatori (uno per tipo di moltiplicatore se il
 * contest li conta per banda, piu' il totale), la riga del totale in evidenza e il punteggio finale grande.
 */
export function StatsCard({ score: s }: { score: TextContestScore }) {
  const t = useT();
  const mults = multSummary(s).filter((m) => m.perBand !== null);
  // moltiplicatori per modo (ARRL 10): una riga in piu' solo se il core li da'
  const byMode = multsByMode(s);
  return (
    <div className="ctw-stats">
      <div className="ctw-score" aria-label={t('ctw.score')}>
        <span className="ctw-scoreline">{scoreLine(t, s)}</span>
        <b>{s.punteggio.toLocaleString()}</b>
      </div>
      <table className="ctw-table">
        <thead>
          <tr>
            <th>{t('ctest.c.band')}</th>
            <th>{t('ctest.c.qso')}</th>
            <th>{t('ctest.c.dupes')}</th>
            <th>{t('ctest.c.points')}</th>
            <th>{t('ctw.c.avg')}</th>
            {mults.map((m) => (
              <th key={m.tipo}>{multName(t, m.tipo)}</th>
            ))}
            <th>{t('ctest.c.mult')}</th>
          </tr>
        </thead>
        <tbody>
          {s.bande.map((b) => (
            <tr key={b.band}>
              <th scope="row">{b.band || '—'}</th>
              <td>{b.qso}</td>
              <td>{b.doppi}</td>
              <td>{b.punti}</td>
              <td>{avgPoints(b.punti, b.qso)}</td>
              {mults.map((m) => (
                <td key={m.tipo}>{m.perBand?.[b.band] ?? 0}</td>
              ))}
              <td>{b.molt}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row">{t('ctest.c.total')}</th>
            <td>{s.qso}</td>
            <td>{s.doppi}</td>
            <td>{s.punti}</td>
            <td>{avgPoints(s.punti, s.qso)}</td>
            {mults.map((m) => (
              <td key={m.tipo}>{m.total != null ? t('ssbc.multOf', { n: m.worked, tot: m.total }) : m.worked}</td>
            ))}
            <td>{s.molt}</td>
          </tr>
        </tfoot>
      </table>
      {byMode.length > 0 ? (
        <p className="ctest-rate" data-testid="ctw-mult-modes">
          {t('ctest.c.mult')}: {byMode.map((m) => `${m.mode} ${m.n}`).join(' · ')}
        </p>
      ) : null}
      {s.errore_script ? (
        <p className="ctest-rate" role="alert">
          {t('ctest.scriptError', { msg: s.errore_script })}
        </p>
      ) : null}
      {s.esclusi > 0 ? <p className="ctest-rate">{t('ctest.excluded', { n: s.esclusi })}</p> : null}
    </div>
  );
}
