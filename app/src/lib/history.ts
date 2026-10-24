export const PERIODS = ['1h', '24h', '7d', 'all'] as const;
export type Period = (typeof PERIODS)[number];

const PERIOD_MS: Record<Exclude<Period, 'all'>, number> = { '1h': 3_600_000, '24h': 86_400_000, '7d': 7 * 86_400_000 };

/** Da quando (ms Unix) cercare per il periodo scelto; `null` = senza limite. */
export function periodStart(period: Period, now: number): number | null {
  return period === 'all' ? null : now - PERIOD_MS[period];
}

export interface HistoryFilter {
  q: string;
  band: string;
  mode: string;
  period: Period;
}

export const MODES = ['FT8', 'FT4'] as const;

/** I parametri della richiesta allo storico: solo quelli scelti, il testo senza spazi ai lati. */
export function historyParams(filter: HistoryFilter, offset: number, limit: number, now: number): URLSearchParams {
  const params = new URLSearchParams();
  const q = filter.q.trim();
  if (q) params.set('q', q);
  if (filter.band) params.set('band', filter.band);
  if (filter.mode) params.set('mode', filter.mode);
  const from = periodStart(filter.period, now);
  if (from !== null) params.set('from_ms', String(Math.round(from)));
  params.set('limit', String(limit));
  if (offset > 0) params.set('offset', String(offset));
  return params;
}

/** Data e ora locali brevi, «24/10 16:46:08». */
export function formatWhen(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}
