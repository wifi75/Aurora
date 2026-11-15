import { useQuery } from '@tanstack/react-query';
import { useT } from '../../i18n';
import { ContestError } from '../../api/contest';
import { getTextContestCabrillo, getTextContestScore, type TextContestScore } from '../../api/contestTesto';
import { saveText } from '../../lib/foxLog';
import { toast } from '../../lib/toast';
import '../../styles/digi-contest.css';

/** La chiave del punteggio del contest attivo dei modi testuali (da rileggere dopo ogni QSO salvato). */
export const TEXT_CONTEST_SCORE_KEY = ['contest-testo-punteggio'] as const;

/** Il punteggio del contest attivo, letto dal core ogni 30 secondi e dopo ogni QSO salvato. */
export function useTextContestScore(active: boolean) {
  return useQuery({ queryKey: [...TEXT_CONTEST_SCORE_KEY], queryFn: getTextContestScore, enabled: active, retry: false, refetchInterval: 30_000, refetchOnWindowFocus: false });
}

/** Salva il Cabrillo del contest attivo (il core lo scrive dai QSO del log). */
export function useExportTextCabrillo() {
  const t = useT();
  return async () => {
    try {
      const r = await getTextContestCabrillo();
      if (r.qso === 0) {
        toast(t('ctest.cabrilloEmpty'), 'info');
        return;
      }
      saveText(r.nome, r.testo);
      toast(t('ctest.cabrilloSaved', { name: r.nome, n: r.qso }), 'ok');
    } catch (e) {
      toast(t('ctest.error', { why: e instanceof ContestError ? (e.detail ?? e.code) : String(e) }), 'error');
    }
  };
}

/** La riga del punteggio: punti × moltiplicatori = punteggio (o i soli punti se il contest non ha moltiplicatori). */
export function scoreLine(t: ReturnType<typeof useT>, s: TextContestScore): string {
  if (s.qtc > 0) return t('ctest.scoreQtc', { points: s.punti, qtc: s.qtc, mult: s.tipi.length ? s.molt : 1, score: s.punteggio });
  return s.tipi.length ? t('ctest.scoreLine', { points: s.punti, mult: s.molt, score: s.punteggio }) : t('ctest.scoreNoMult', { score: s.punteggio });
}

type MultType = 'dxcc' | 'cq' | 'itu' | 'wpx' | 'cont' | 'area' | 'dom' | 'call';
const MULT_TYPES: MultType[] = ['cq', 'itu', 'dxcc', 'area', 'wpx', 'dom', 'cont', 'call'];
const multName = (t: ReturnType<typeof useT>, tipo: string) => (MULT_TYPES.includes(tipo as MultType) ? t(`ctest.mt.${tipo as MultType}`) : tipo);
/** `dxcc:DL` -> `DL (Paesi DXCC)`. */
const multLabel = (t: ReturnType<typeof useT>, m: string) => {
  const i = m.indexOf(':');
  return i < 0 ? m : `${m.slice(i + 1)} (${multName(t, m.slice(0, i))})`;
};

/** La griglia banda × moltiplicatori del contest, un riquadro per tipo, con quelli che mancano. */
function MultGrid({ s }: { s: TextContestScore }) {
  const t = useT();
  const bands = s.bande.map((b) => b.band);
  // prima i tipi conosciuti, poi quelli dati dallo script del contest (`rda`, `state`...)
  const known: string[] = MULT_TYPES.filter((m) => s.griglia.some((g) => g.tipo === m) || s.mancanti.some((x) => x.tipo === m));
  const types = [...known, ...Array.from(new Set(s.griglia.map((g) => g.tipo))).filter((x) => !MULT_TYPES.includes(x as MultType))];
  if (!types.length) return null;
  return (
    <section className="ctest-grid">
      <h4>{t('ctest.grid')}</h4>
      {types.map((tipo) => {
        const rows = s.griglia.filter((g) => g.tipo === tipo);
        const perBand = rows.some((r) => r.bande.length > 0);
        const miss = s.mancanti.find((m) => m.tipo === tipo);
        return (
          <details key={tipo} open={types.length <= 3}>
            <summary>
              {multName(t, tipo)}: {rows.length}
              {miss ? ` · ${t('ctest.missing', { n: miss.valori.length, tot: miss.totali })}` : ''}
            </summary>
            {rows.length > 0 && (
              <div className="ctest-table-wrap ctest-grid-wrap">
                <table className="ctest-table ctest-stats">
                  <thead>
                    <tr>
                      <th>{multName(t, tipo)}</th>
                      {perBand ? bands.map((b) => <th key={b}>{b}</th>) : <th>{t('ctest.once')}</th>}
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((r) => (
                      <tr key={r.valore}>
                        <td>{r.valore}</td>
                        {perBand ? bands.map((b) => <td key={b}>{r.bande.includes(b) ? '✓' : ''}</td>) : <td>✓</td>}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {miss && miss.valori.length > 0 && <p className="ctest-missing">{miss.valori.join(' ')}</p>}
          </details>
        );
      })}
    </section>
  );
}

/** La finestrella delle statistiche del contest attivo: per banda QSO, doppi, punti, moltiplicatori e categorie, poi il totale. */
export function ContestStatsDialog({ name, onClose }: { name: string; onClose(): void }) {
  const t = useT();
  const exportCabrillo = useExportTextCabrillo();
  const q = useTextContestScore(true);
  const s = q.data;
  const sum = (k: 'cw' | 'phone' | 'digital') => s?.bande.reduce((a, b) => a + b[k], 0) ?? 0;
  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('ctest.statsTitle', { name })} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <div className="digi-dialog ctest-dialog">
        <header>
          <strong>{t('ctest.statsTitle', { name })}</strong>
        </header>
        {!s ? (
          <p>{q.isError ? t('ctest.error', { why: q.error instanceof ContestError ? (q.error.detail ?? q.error.code) : String(q.error) }) : t('ctest.loading')}</p>
        ) : s.qso + s.doppi === 0 ? (
          <p>{t('ctest.noQsos')}</p>
        ) : (
          <>
            <div className="ctest-table-wrap">
              <table className="ctest-table ctest-stats">
                <thead>
                  <tr>
                    <th>{t('ctest.c.band')}</th>
                    <th>{t('ctest.c.qso')}</th>
                    <th>{t('ctest.c.dupes')}</th>
                    <th>{t('ctest.c.points')}</th>
                    <th>{t('ctest.c.mult')}</th>
                    <th>CW</th>
                    <th>{t('ctest.c.phone')}</th>
                    <th>{t('ctest.c.digital')}</th>
                  </tr>
                </thead>
                <tbody>
                  {s.bande.map((b) => (
                    <tr key={b.band}>
                      <td>{b.band || '—'}</td>
                      <td>{b.qso}</td>
                      <td>{b.doppi}</td>
                      <td>{b.punti}</td>
                      <td>{b.molt}</td>
                      <td>{b.cw}</td>
                      <td>{b.phone}</td>
                      <td>{b.digital}</td>
                    </tr>
                  ))}
                  <tr className="ctest-total">
                    <td>{t('ctest.c.total')}</td>
                    <td>{s.qso}</td>
                    <td>{s.doppi}</td>
                    <td>{s.punti}</td>
                    <td>{s.molt}</td>
                    <td>{sum('cw')}</td>
                    <td>{sum('phone')}</td>
                    <td>{sum('digital')}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p className="ctest-score">{scoreLine(t, s)}</p>
            <p className="ctest-rate">{t('ctest.rate', { r10: s.rate10, r60: s.rate60, avg: s.media_ora })}</p>
            {s.errore_script && <p className="ctest-rate" role="alert">{t('ctest.scriptError', { msg: s.errore_script })}</p>}
            {s.esclusi > 0 && <p className="ctest-rate">{t('ctest.excluded', { n: s.esclusi })}</p>}
            {s.recenti.length > 0 && (
              <p className="ctest-rate">
                {t('ctest.recent')}: {s.recenti.map((r) => `${r.call} ${r.band}: ${r.nuovi.map((m) => multLabel(t, m)).join(', ')}`).join(' · ')}
              </p>
            )}
            <MultGrid s={s} />
          </>
        )}
        <footer>
          <button type="button" className="glass sstv-btn" disabled={!s || s.qso + s.doppi === 0} onClick={() => void exportCabrillo()}>
            {t('ctest.cabrillo')}
          </button>
          <span className="rtty-spacer" />
          <button type="button" className="glass sstv-btn" onClick={onClose}>
            {t('ctest.close')}
          </button>
        </footer>
      </div>
    </div>
  );
}
