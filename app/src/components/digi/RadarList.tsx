import { useT } from '../../i18n';
import type { RadarStation } from '../../lib/digiBand';
import { flagCode } from '../../lib/flags';
import '../../styles/digi-radar.css';

interface Props {
  stations: RadarStation[];
  /** CW: colonna della velocita'. */
  showWpm: boolean;
  /** RTTY non misura il S/N per canale: la colonna si nasconde. */
  showSnr: boolean;
  /** Frequenza della radio (Hz): con la radio nota la frequenza vera in kHz, se no il tono audio in Hz. */
  dialHz: number;
  /** La frequenza di ricezione adesso: la riga della stazione ascoltata si evidenzia. */
  rxHz: number;
  modeName: string;
  onTune(hz: number): void;
  onCall(call: string): void;
}

function freqLabel(hz: number, dialHz: number): string {
  return dialHz > 0 ? ((dialHz + hz) / 1000).toFixed(2) : `${Math.round(hz)} Hz`;
}

/**
 * Il Radar: le stazioni della banda decodificate con sicurezza, una riga per nominativo, in una griglia a colonne fisse. Ogni stazione ha
 * il suo colore. Clic: la ricezione va sulla stazione; doppio clic: il nominativo va nel QSO.
 */
export function RadarList({ stations, showWpm, showSnr, dialHz, rxHz, modeName, onTune, onCall }: Props) {
  const t = useT();
  return (
    <section className="radar" aria-label={`${t('radar.title')} ${modeName}`}>
      <header className="radar-head">
        <strong>
          {t('radar.title')} {modeName}
        </strong>
        <small>{t('radar.count', { n: stations.length })}</small>
      </header>
      <div className="radar-scroll">
        <table className="radar-grid">
          <colgroup>
            <col className="radar-col-call" />
            <col className="radar-col-freq" />
            {showWpm ? <col className="radar-col-num" /> : null}
            {showSnr ? <col className="radar-col-num" /> : null}
            <col className="radar-col-cq" />
          </colgroup>
          <thead>
            <tr>
              <th>{t('radar.colCall')}</th>
              <th>{t('radar.colFreq')}</th>
              {showWpm ? <th>WPM</th> : null}
              {showSnr ? <th>S/N</th> : null}
              <th>CQ</th>
            </tr>
          </thead>
          <tbody>
            {stations.length === 0 ? (
              <tr>
                <td className="radar-empty" colSpan={3 + (showWpm ? 1 : 0) + (showSnr ? 1 : 0)}>
                  {t('radar.empty')}
                </td>
              </tr>
            ) : (
              stations.map((s) => {
                const code = flagCode(s.country);
                const tuned = Math.abs(s.freqHz - rxHz) < 12;
                return (
                  <tr
                    key={s.call}
                    className={`radar-c${s.color}${tuned ? ' radar-on' : ''}${s.stale ? ' radar-stale' : ''}`}
                    title={`${s.country ? `${s.country} · ` : ''}${t('radar.rowTip')}`}
                    aria-selected={tuned}
                    onClick={() => onTune(s.freqHz)}
                    onDoubleClick={() => onCall(s.call)}
                  >
                    <td className="radar-call">
                      <span className="radar-dot" aria-hidden="true" />
                      {code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}
                      <b>{s.call}</b>
                    </td>
                    <td className="radar-num">{freqLabel(s.freqHz, dialHz)}</td>
                    {showWpm ? <td className="radar-num">{s.wpm != null ? Math.round(s.wpm) : ''}</td> : null}
                    {showSnr ? <td className="radar-num">{Math.round(s.snrDb)}</td> : null}
                    <td className="radar-cq">{s.cq ? <em>CQ</em> : ''}</td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
