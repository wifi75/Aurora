import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { DxpedDetail } from './DxpedDetail';
import { getDxcc, getDxccMatrix, getDxped, type Dxpedition, getClusterSpots, type ClusterSpot } from '../api/client';
import { loadAtlas } from './CountryShape';
import { tuneSpot } from './ClusterPanel';
import { useStation } from '../state/station';
import { senderOf } from '../lib/wishlist';
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
export function DxpedPanel({ heardCalls, foxCalls = [], keepDays = 15, soonDays = 30, onCall }: { heardCalls: string[]; foxCalls?: string[]; keepDays?: number; /** Quanti giorni in anticipo mostrare quelle in arrivo. */ soonDays?: number; /** Doppio clic su una spedizione sentita: la si chiama (come il doppio clic in Attivita' di banda). */ onCall?(decodeId: number): void }) {
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
  const client = useQueryClient();
  const clickTimer = useRef<number | undefined>(undefined);
  // sentita da me: l'ultimo messaggio ricevuto da quella stazione (ora, segnale, frequenza) e la frequenza della radio
  const decodes = useStation((st) => st.decodes);
  const dial = useStation((st) => st.status?.dial_hz ?? 0);
  const mode = useStation((st) => st.status?.mode ?? '');
  const heardFrom = (call: string) => decodes.find((d) => d.id >= 0 && senderOf(d.text)?.call.toUpperCase() === call.toUpperCase());
  // le DXpedition di paesi che non ho ancora lavorato: l'elenco si salva con le impostazioni, cosi' l'agente nel tray (anche con
  // Aurora chiusa) avvisa solo per quelle
  const matrix = useQuery({ queryKey: ['dxcc-matrix-all'], queryFn: () => getDxccMatrix(), staleTime: 30 * 60_000, retry: false });
  useEffect(() => {
    const countries = matrix.data?.countries;
    if (!countries || items.length === 0) return;
    const worked = new Set(countries.filter((c) => Object.keys(c.cells ?? {}).length > 0).map((c) => c.name.toUpperCase()));
    const missing: string[] = [];
    for (const d of items) {
      const info = client.getQueryData<{ country?: string }>(['dxcc', d.call]);
      const country = (info?.country ?? d.country).toUpperCase();
      if (!worked.has(country)) missing.push(d.call.toUpperCase());
    }
    try {
      localStorage.setItem('aurora-dxped-mancanti', JSON.stringify({ calls: missing, at: Date.now() }));
    } catch {
      // senza archivio l'agente avvisa per tutte
    }
  }, [matrix.data, items, client]);
  // quelle spottate adesso salgono in cima (nel loro ordine); dopo 15 minuti senza spot tornano al loro posto
  const onAirFirst = [...items].sort((a, b) => Number(onAir.has(b.call.toUpperCase())) - Number(onAir.has(a.call.toUpperCase())));
  // la scheda si apre subito: confini del mondo e paese di ogni DXpedition si preparano in anticipo, appena c'e' il calendario
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
            tabIndex={0}
            title={d.heard && onCall ? t('tip.dxp.call') : t('tip.dxp.open')}
            onClick={() => {
              // un clic apre la scheda; un doppio clic su una spedizione sentita la chiama (la scheda allora non si apre)
              window.clearTimeout(clickTimer.current);
              clickTimer.current = window.setTimeout(() => setOpen(d), 260);
            }}
            onDoubleClick={() => {
              window.clearTimeout(clickTimer.current);
              const h = d.heard ? heardFrom(d.call) : undefined;
              if (h && onCall) onCall(h.id);
              else setOpen(d);
            }}
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
              // sentita da me in Attivita' di banda (non spottata): dove, quando e con che segnale
              if (onAir.has(d.call.toUpperCase())) return null;
              const h = d.heard ? heardFrom(d.call) : undefined;
              if (!h) return null;
              const sec = Math.floor((h.time_ms % 86_400_000) / 1000);
              const hhmm = `${String(Math.floor(sec / 3600)).padStart(2, '0')}:${String(Math.floor((sec % 3600) / 60)).padStart(2, '0')}`;
              return (
                <div className="dxped-onair dxped-heard-line">
                  <span className="dxped-onair-dot dxped-heard-dot" aria-hidden="true" />
                  <b>{t('dxp.heardByMe')}</b>
                  {dial > 0 ? <span className="data">{((dial + h.freq_hz) / 1e6).toFixed(4)} MHz</span> : null}
                  {mode ? <span className="data">{mode}</span> : null}
                  <span className="data">{h.snr} dB</span>
                  <span className="muted">{hhmm} UTC</span>
                </div>
              );
            })()}
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
