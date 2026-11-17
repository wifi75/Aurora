import { useQuery } from '@tanstack/react-query';
import { useT } from '../../i18n';
import { ContestError } from '../../api/contest';
import { getTextContestCabrillo, getTextContestScore, type TextContest, type TextContestScore } from '../../api/contestTesto';
import { contestWindow } from '../../lib/textContest';
import { multSummary } from '../../lib/contestEntry';
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
// il core chiama `dominio` il tipo dello scambio ricevuto (stati, province, prefetture)
const normType = (tipo: string) => (tipo === 'dominio' ? 'dom' : tipo);
export const multName = (t: ReturnType<typeof useT>, tipo: string) => (MULT_TYPES.includes(normType(tipo) as MultType) ? t(`ctest.mt.${normType(tipo) as MultType}`) : tipo);
/** `dxcc:DL` -> `DL (Paesi DXCC)`. */
export const multLabel = (t: ReturnType<typeof useT>, m: string) => {
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

const HOUR = 3_600_000;
/** `4 h 05` da una durata in ms. */
const hm = (ms: number) => `${Math.floor(ms / HOUR)} h ${String(Math.floor((ms % HOUR) / 60_000)).padStart(2, '0')}`;

/** L'avanzamento del contest: barra, tempo trascorso e rimanente (niente se il contest non ha una finestra di date). */
export function ContestProgress({ contest }: { contest?: Pick<TextContest, 'start' | 'end'> }) {
  const t = useT();
  const now = Date.now();
  const w = contest ? contestWindow(contest, now) : null;
  const span = w && w.from != null && w.to != null && w.to > w.from ? { from: w.from, to: w.to } : null;
  if (!span) return null;
  const progress = Math.min(1, Math.max(0, (now - span.from) / (span.to - span.from)));
  return (
    <div className="ctest-progress" aria-label={t('ssbc.elapsed', { t: hm(Math.max(0, now - span.from)) })}>
      <div className="ctest-progress-bar">
        <i style={{ width: `${Math.round(progress * 100)}%` }} />
      </div>
      <p className="ctest-rate">
        {t('ssbc.elapsed', { t: hm(Math.max(0, now - span.from)) })} · {t('ssbc.remaining', { t: hm(Math.max(0, span.to - now)) })}
      </p>
    </div>
  );
}

/** I QSO per ora delle ultime 12 ore (istogramma). */
export function ContestHours({ qsoTimes }: { qsoTimes?: number[] }) {
  const t = useT();
  if (!qsoTimes) return null;
  const now = Date.now();
  const bars = Array.from({ length: 12 }, (_, i) => {
    const start = Math.floor(now / HOUR) * HOUR - (11 - i) * HOUR;
    return qsoTimes.filter((ms) => ms >= start && ms < start + HOUR).length;
  });
  const top = Math.max(1, ...bars);
  return (
    <figure className="ctest-hours">
      <figcaption className="ctest-rate">{t('ssbc.perHour')}</figcaption>
      <svg viewBox="0 0 120 34" role="img" aria-label={t('ssbc.perHour')}>
        {bars.map((n, i) => (
          <rect key={i} x={i * 10 + 1} y={30 - (n / top) * 26} width={8} height={Math.max(n > 0 ? 1 : 0, (n / top) * 26)} rx={1} className={i === 11 ? 'now' : undefined}>
            <title>{n}</title>
          </rect>
        ))}
        <line x1={0} y1={30.5} x2={120} y2={30.5} />
      </svg>
    </figure>
  );
}

/**
 * La scheda delle statistiche del contest attivo, la stessa nel dialogo e nella vista SSB: tabella per banda con il totale, ritmo, ultimi
 * moltiplicatori nuovi, avvisi e, se si danno la finestra del contest e gli orari dei QSO, avanzamento e QSO per ora (ultime 12 ore).
 */
export function ContestStatsCard({ s, contest, qsoTimes }: { s: TextContestScore; contest?: Pick<TextContest, 'start' | 'end'>; qsoTimes?: number[] }) {
  const t = useT();
  const sum = (k: 'cw' | 'phone' | 'digital') => s.bande.reduce((a, b) => a + b[k], 0);
  // la tabella segue il contest: una colonna per tipo di moltiplicatore delle sue regole (zone, paesi, prefissi, province...) e le
  // categorie CW / fonia / digitale solo se nel log ce n'e' piu' di una
  const mults = multSummary(s);
  const cats = (['cw', 'phone', 'digital'] as const).filter((k) => sum(k) > 0);
  const showCats = cats.length > 1;
  return (
    <div className="ctest-card">
      <div className="ctest-table-wrap">
        <table className="ctest-table ctest-stats">
          <thead>
            <tr>
              <th>{t('ctest.c.band')}</th>
              <th>{t('ctest.c.qso')}</th>
              <th>{t('ctest.c.dupes')}</th>
              <th>{t('ctest.c.points')}</th>
              {mults.map((m) => (
                <th key={m.tipo}>{multName(t, m.tipo)}</th>
              ))}
              {mults.length > 0 ? <th>{t('ctest.c.mult')}</th> : null}
              {showCats ? <th>CW</th> : null}
              {showCats ? <th>{t('ctest.c.phone')}</th> : null}
              {showCats ? <th>{t('ctest.c.digital')}</th> : null}
            </tr>
          </thead>
          <tbody>
            {s.bande.map((b) => (
              <tr key={b.band}>
                <td>{b.band || '—'}</td>
                <td>{b.qso}</td>
                <td>{b.doppi}</td>
                <td>{b.punti}</td>
                {mults.map((m) => (
                  <td key={m.tipo}>{m.perBand ? (m.perBand[b.band] ?? 0) : ''}</td>
                ))}
                {mults.length > 0 ? <td>{b.molt}</td> : null}
                {showCats ? <td>{b.cw}</td> : null}
                {showCats ? <td>{b.phone}</td> : null}
                {showCats ? <td>{b.digital}</td> : null}
              </tr>
            ))}
            <tr className="ctest-total">
              <td>{t('ctest.c.total')}</td>
              <td>{s.qso}</td>
              <td>{s.doppi}</td>
              <td>{s.punti}</td>
              {mults.map((m) => (
                <td key={m.tipo}>{m.total != null ? t('ssbc.multOf', { n: m.worked, tot: m.total }) : m.worked}</td>
              ))}
              {mults.length > 0 ? <td>{s.molt}</td> : null}
              {showCats ? <td>{sum('cw')}</td> : null}
              {showCats ? <td>{sum('phone')}</td> : null}
              {showCats ? <td>{sum('digital')}</td> : null}
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
      <ContestProgress contest={contest} />
      <ContestHours qsoTimes={qsoTimes} />
    </div>
  );
}

/** La finestrella delle statistiche del contest attivo: per banda QSO, doppi, punti, moltiplicatori e categorie, poi il totale. */
export function ContestStatsDialog({ name, onClose }: { name: string; onClose(): void }) {
  const t = useT();
  const exportCabrillo = useExportTextCabrillo();
  const q = useTextContestScore(true);
  const s = q.data;
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
            <ContestStatsCard s={s} />
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
