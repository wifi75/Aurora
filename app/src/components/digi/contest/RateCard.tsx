import { useT } from '../../../i18n';
import type { TextContest, TextContestScore } from '../../../api/contestTesto';
import { ContestHours, ContestProgress } from '../ContestStatsDialog';

interface Props {
  score: TextContestScore | undefined;
  contest: Pick<TextContest, 'start' | 'end'> | null;
  /** Gli istanti (ms) dei QSO del contest, per i QSO per ora. */
  qsoTimes: number[];
}

/**
 * Punteggio e ritmo: QSO, punti, moltiplicatori e punteggio in quattro tessere, il ritmo (ultimi 10 minuti, ultima ora, media) in una tabellina, il
 * tempo trascorso e rimanente del contest e i QSO per ora. Il core non da' ancora il ritmo sugli ultimi 10, 50 e 100 QSO ne' i punti all'ora
 * (vedi `docs/contest-schede.md`): qui ci sono solo i dati che ha.
 */
export function RateCard({ score, contest, qsoTimes }: Props) {
  const t = useT();
  if (!score || score.qso + score.doppi === 0) return <p className="ssbc-muted">{t('ssbc.waiting')}</p>;
  return (
    <div className="ctw-rate">
      <div className="ssbc-tiles ctw-tiles">
        <div>
          <span>{t('ssbc.qso')}</span>
          <b>{score.qso}</b>
        </div>
        <div>
          <span>{t('ssbc.points')}</span>
          <b>{score.punti}</b>
        </div>
        <div>
          <span>{t('ssbc.mult')}</span>
          <b>{score.molt}</b>
        </div>
        <div className="ssbc-tiles-total">
          <span>{t('ssbc.total')}</span>
          <b>{score.punteggio}</b>
        </div>
      </div>
      <table className="ctw-table ctw-ratetable">
        <caption>{t('ctw.rateTitle')}</caption>
        <tbody>
          <tr>
            <th scope="row">{t('ctw.rate10')}</th>
            <td>{score.rate10}</td>
          </tr>
          <tr>
            <th scope="row">{t('ctw.rate60')}</th>
            <td>{score.rate60}</td>
          </tr>
          <tr>
            <th scope="row">{t('ctw.rateAvg')}</th>
            <td>{score.media_ora.toFixed(1)}</td>
          </tr>
        </tbody>
      </table>
      <ContestProgress contest={contest ?? undefined} />
      <ContestHours qsoTimes={qsoTimes} />
    </div>
  );
}
