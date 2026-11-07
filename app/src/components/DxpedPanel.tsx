import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { DxpedDetail } from './DxpedDetail';
import { getDxcc, getDxped, type Dxpedition, getClusterSpots, type ClusterSpot } from '../api/client';
import { loadAtlas } from './CountryShape';
import { tuneSpot } from './ClusterPanel';
import { useT } from '../i18n';
import { flagCode } from '../lib/flags';
import { coreText } from '../lib/coreText';

/** Oggi come `AAAA-MM-GG` (UTC, come il calendario NG3K). */
export function today(now = new Date()): string {
  return now.toISOString().slice(0, 10);
}

/** «2026-11-04» → «04/11». */
function short(date: string): string {
  return `${date.slice(8, 10)}/${date.slice(5, 7)}`;
}

export type DxpedState = 'active' | 'soon' | 'ended';

/** In onda, in arrivo (entro 30 giorni) o finita da poco (entro `keepDays`); le altre non si mostrano. */
export function dxpedState(d: Dxpedition, day: string, keepDays: number, soonDays = 30): DxpedState | null {
  if (d.start <= day && d.end >= day) return 'active';
  const ms = (a: string, b: string) => (Date.parse(a) - Date.parse(b)) / 86_400_000;
  if (d.start > day) return ms(d.start, day) <= soonDays ? 'soon' : null;
  return ms(day, d.end) <= keepDays ? 'ended' : null;
}

/** Le DXpedition da mostrare: prima quelle in onda (le sentite adesso in cima), poi le prossime, poi le finite da poco. */
export function dxpedList(items: Dxpedition[], heard: Set<string>, day: string, keepDays: number, soonDays = 30): Array<Dxpedition & { state: DxpedState; heard: boolean }> {
  const rank = { active: 0, soon: 1, ended: 2 } as const;
  return items
    .map((d) => ({ ...d, state: dxpedState(d, day, keepDays, soonDays), heard: heard.has(d.call.toUpperCase()) }))
    .filter((d): d is Dxpedition & { state: DxpedState; heard: boolean } => d.state !== null)
    .sort((a, b) => rank[a.state] - rank[b.state] || Number(b.heard) - Number(a.heard) || (a.state === 'ended' ? b.end.localeCompare(a.end) : a.start.localeCompare(b.start)));
}

/** Il calendario delle DXpedition (NG3K) accanto ai CQ: quelle in onda, quelle sentite adesso evidenziate, e lo storico da scorrere. */
export function DxpedPanel({ heardCalls, foxCalls = [], keepDays = 15, soonDays = 30 }: { heardCalls: string[]; foxCalls?: string[]; keepDays?: number; /** Quanti giorni in anticipo mostrare quelle in arrivo. */ soonDays?: number }) {
  const t = useT();
  // finche' il core non ha ancora scaricato il calendario si richiede spesso; poi ogni 10 minuti
  const q = useQuery({ queryKey: ['online-dxped'], queryFn: getDxped, refetchInterval: (query) => (query.state.data?.dxped.fetched_ms ? 10 * 60_000 : 15_000), retry: false });
  const [open, setOpen] = useState<(Dxpedition & { state: DxpedState }) | null>(null);
  const heard = new Set(heardCalls.map((c) => c.toUpperCase()));
  // spot del DX Cluster degli ultimi 15 minuti: una spedizione spottata adesso e' «in onda» (stessa lettura del pannello DX Cluster)
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), refetchInterval: 4000, retry: false });
  const onAir = new Map<string, ClusterSpot>();
  for (const sp of cluster.data?.spots ?? []) {
    if (Date.now() - sp.received_ms > 15 * 60_000) continue;
    const call = sp.dx_call.toUpperCase();
    const old = onAir.get(call);
    if (!old || sp.received_ms > old.received_ms) onAir.set(call, sp);
  }
  const items = dxpedList(q.data?.dxped.items ?? [], heard, today(), keepDays, soonDays);
  // quelle spottate adesso salgono in cima (nel loro ordine); dopo 15 minuti senza spot tornano al loro posto
  const onAirFirst = [...items].sort((a, b) => Number(onAir.has(b.call.toUpperCase())) - Number(onAir.has(a.call.toUpperCase())));
  // la scheda si apre subito: confini del mondo e paese di ogni DXpedition si preparano in anticipo, appena c'e' il calendario
  const client = useQueryClient();
  const calls = items.map((d) => d.call).join(' ');
  useEffect(() => {
    if (calls === '') return;
    const run = () => {
      void loadAtlas();
      for (const call of calls.split(' ')) {
        void client.prefetchQuery({ queryKey: ['dxcc', call], queryFn: () => getDxcc(call), staleTime: Infinity, retry: false });
      }
    };
    const idle = (window as Window & { requestIdleCallback?: (cb: () => void) => number }).requestIdleCallback;
    if (idle) idle(run);
    else setTimeout(run, 500);
  }, [calls, client]);
  return (
    <section className="dxped" aria-label={t('dxp.title')}>
      <header className="dxped-head">
        <strong>{t('dxp.title')}</strong>
        <span className="muted">NG3K</span>
      </header>
      {q.data?.dxped.error ? <p className="muted dxped-note">{coreText(q.data.dxped.error, t)}</p> : null}
      {items.length === 0 ? <p className="muted dxped-note">{q.data?.dxped.fetched_ms ? t('dxp.none') : t('dxp.loading')}</p> : null}
      <ul className="dxped-list">
        {onAirFirst.map((d) => (
          <li
            key={`${d.call}-${d.start}`}
            className={`dxped-item dxped-item--${d.state}${d.heard ? ' dxped-item--heard' : ''}${onAir.has(d.call.toUpperCase()) ? ' dxped-item--onair' : ''}`}
            title={t('tip.dxp.open')}
            tabIndex={0}
            onClick={() => setOpen(d)}
            onKeyDown={(e) => e.key === 'Enter' && setOpen(d)}
          >
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
            {(() => {
              // spottata adesso: una riga sua sotto il nominativo (mai sopra il testo), con frequenza, modo e da quanto
              const sp = onAir.get(d.call.toUpperCase());
              if (!sp) return null;
              const min = Math.max(0, Math.round((Date.now() - sp.received_ms) / 60_000));
              return (
                <div className="dxped-onair" title={sp.comment || undefined}>
                  <span className="dxped-onair-dot" aria-hidden="true" />
                  <b>{t('dxp.onAir')}</b>
                  <span
                    className="data dxped-onair-freq"
                    role="button"
                    tabIndex={0}
                    title={t('tip.dxp.tune')}
                    onClick={(e) => e.stopPropagation()}
                    onDoubleClick={(e) => {
                      e.stopPropagation();
                      void tuneSpot(sp).catch(() => undefined);
                    }}
                  >
                    {(sp.freq_khz / 1000).toFixed(3)} MHz
                  </span>
                  <span className="data">{spotMode(sp)}</span>
                  <span className="muted">{t('dxp.spotAgo', { min })}</span>
                  {sp.spotter ? (
                    <span className="muted">
                      {t('dxp.spotBy')} <span className="data">{sp.spotter}</span>
                    </span>
                  ) : null}
                </div>
              );
            })()}
            {d.state === 'active' ? (
              <div className="dxped-info muted">
                {d.info}
                {d.qsl ? ` · QSL ${d.qsl}` : ''}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
      {open ? <DxpedDetail key={open.call} d={open} onClose={() => setOpen(null)} /> : null}
    </section>
  );
}

/** Le frequenze FT8 e FT4 di ogni banda (kHz), per riconoscere il modo di uno spot che non lo dice. */
const DIGI: readonly [number, string][] = [
  [1840, 'FT8'], [3573, 'FT8'], [3575, 'FT4'], [5357, 'FT8'], [7074, 'FT8'], [7047.5, 'FT4'], [10136, 'FT8'], [10140, 'FT4'],
  [14074, 'FT8'], [14080, 'FT4'], [18100, 'FT8'], [18104, 'FT4'], [21074, 'FT8'], [21140, 'FT4'], [24915, 'FT8'], [24919, 'FT4'],
  [28074, 'FT8'], [28180, 'FT4'], [50313, 'FT8'], [50318, 'FT4'],
];
/** Dove finisce il CW in ogni banda (kHz): sotto e' CW, sopra (fuori dalle frequenze digitali) SSB. */
const CW_TOP: readonly [number, number, number][] = [
  [1800, 2000, 1840], [3500, 4000, 3570], [7000, 7300, 7040], [10100, 10150, 10130], [14000, 14350, 14070], [18068, 18168, 18095],
  [21000, 21450, 21070], [24890, 24990, 24915], [28000, 29700, 28070], [50000, 54000, 50100],
];

/** Il modo dello spot: quello del cluster, altrimenti dal commento, altrimenti dalla frequenza (FT8/FT4, CW, SSB). */
function spotMode(sp: ClusterSpot): string {
  if (sp.mode) return sp.mode.toUpperCase();
  const c = (sp.comment ?? '').toUpperCase();
  for (const m of ['FT8', 'FT4', 'RTTY', 'PSK', 'CW', 'SSB', 'USB', 'LSB', 'JT65', 'MSK144']) if (new RegExp(`\\b${m}\\b`).test(c)) return m === 'USB' || m === 'LSB' ? 'SSB' : m;
  const k = sp.freq_khz;
  const digi = DIGI.find(([f]) => Math.abs(k - f) <= 3);
  if (digi) return digi[1];
  const band = CW_TOP.find(([lo, hi]) => k >= lo && k <= hi);
  if (band) return k < band[2] ? 'CW' : 'SSB';
  return '?';
}
