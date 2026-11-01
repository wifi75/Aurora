import { useQuery } from '@tanstack/react-query';
import { getDxped, type Dxpedition } from '../api/client';
import { useT } from '../i18n';
import { flagCode } from '../lib/flags';

/** Oggi come `AAAA-MM-GG` (UTC, come il calendario NG3K). */
function today(now = new Date()): string {
  return now.toISOString().slice(0, 10);
}

/** «2026-11-04» → «04/11». */
function short(date: string): string {
  return `${date.slice(8, 10)}/${date.slice(5, 7)}`;
}

export type DxpedState = 'active' | 'soon' | 'ended';

/** In onda, in arrivo (entro 30 giorni) o finita da poco (entro `keepDays`); le altre non si mostrano. */
export function dxpedState(d: Dxpedition, day: string, keepDays: number): DxpedState | null {
  if (d.start <= day && d.end >= day) return 'active';
  const ms = (a: string, b: string) => (Date.parse(a) - Date.parse(b)) / 86_400_000;
  if (d.start > day) return ms(d.start, day) <= 30 ? 'soon' : null;
  return ms(day, d.end) <= keepDays ? 'ended' : null;
}

/** Le DXpedition da mostrare: prima quelle in onda (le sentite adesso in cima), poi le prossime, poi le finite da poco. */
export function dxpedList(items: Dxpedition[], heard: Set<string>, day: string, keepDays: number): Array<Dxpedition & { state: DxpedState; heard: boolean }> {
  const rank = { active: 0, soon: 1, ended: 2 } as const;
  return items
    .map((d) => ({ ...d, state: dxpedState(d, day, keepDays), heard: heard.has(d.call.toUpperCase()) }))
    .filter((d): d is Dxpedition & { state: DxpedState; heard: boolean } => d.state !== null)
    .sort((a, b) => rank[a.state] - rank[b.state] || Number(b.heard) - Number(a.heard) || (a.state === 'ended' ? b.end.localeCompare(a.end) : a.start.localeCompare(b.start)));
}

/** Il calendario delle DXpedition (NG3K) accanto ai CQ: quelle in onda, quelle sentite adesso evidenziate, e lo storico da scorrere. */
export function DxpedPanel({ heardCalls, foxCalls = [], keepDays = 15 }: { heardCalls: string[]; foxCalls?: string[]; keepDays?: number }) {
  const t = useT();
  // finche' il core non ha ancora scaricato il calendario si richiede spesso; poi ogni 10 minuti
  const q = useQuery({ queryKey: ['online-dxped'], queryFn: getDxped, refetchInterval: (query) => (query.state.data?.dxped.fetched_ms ? 10 * 60_000 : 15_000), retry: false });
  const heard = new Set(heardCalls.map((c) => c.toUpperCase()));
  const items = dxpedList(q.data?.dxped.items ?? [], heard, today(), keepDays);
  return (
    <section className="dxped" aria-label={t('dxp.title')}>
      <header className="dxped-head">
        <strong>{t('dxp.title')}</strong>
        <span className="muted">NG3K</span>
      </header>
      {q.data?.dxped.error ? <p className="muted dxped-note">{q.data.dxped.error}</p> : null}
      {items.length === 0 ? <p className="muted dxped-note">{q.data?.dxped.fetched_ms ? t('dxp.none') : t('dxp.loading')}</p> : null}
      <ul className="dxped-list">
        {items.map((d) => (
          <li key={`${d.call}-${d.start}`} className={`dxped-item dxped-item--${d.state}${d.heard ? ' dxped-item--heard' : ''}`} title={d.info}>
            <div className="dxped-line">
              <strong className="data dxped-call">{d.call}</strong>
              <span className="dxped-country">
                {flagCode(d.country) ? <span className={`fi fi-${flagCode(d.country)} dxped-flag`} aria-hidden="true" /> : null}
                {d.country}
              </span>
              {foxCalls.includes(d.call.toUpperCase()) ? (
                <span className="badge badge--fox" title={t('tip.dxp.fox')}>
                  Fox/Hound
                </span>
              ) : null}
              <span className="dxped-when">
                {d.heard ? t('dxp.heard') : d.state === 'active' ? t('dxp.until', { date: short(d.end) }) : d.state === 'soon' ? t('dxp.from', { date: short(d.start) }) : t('dxp.ended', { date: short(d.end) })}
              </span>
            </div>
            {d.state === 'active' ? (
              <div className="dxped-info muted">
                {d.info}
                {d.qsl ? ` · QSL ${d.qsl}` : ''}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    </section>
  );
}
