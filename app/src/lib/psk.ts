import type { PskReport } from '../api/client';
import { BAND_NAMES, bandOf } from './format';
import { distanceKm, gridCenter } from './geo';

export interface PskRow extends PskReport {
  band: string | null;
  /** Distanza dal proprio locatore, se si conoscono entrambi i locatori. */
  km: number | null;
}

/** I rapporti con la banda e la distanza dal proprio locatore (4 o 6 caratteri). */
export function pskRows(reports: readonly PskReport[], myGrid: string | null | undefined): PskRow[] {
  const mine = myGrid ? gridCenter(myGrid) : null;
  return reports.map((r) => {
    // il ricevente puo' dare 8 caratteri: bastano i primi 6 per la distanza
    const theirs = gridCenter(r.locator.slice(0, 6)) ?? gridCenter(r.locator.slice(0, 4));
    return { ...r, band: bandOf(r.freq_hz), km: mine && theirs ? distanceKm(mine, theirs) : null };
  });
}

export interface PskSummary {
  /** Stazioni diverse che ti hanno ricevuto. */
  stations: number;
  farthest: { call: string; km: number } | null;
}

export function summarize(rows: readonly PskRow[]): PskSummary {
  const stations = new Set(rows.map((r) => r.receiver)).size;
  let farthest: PskSummary['farthest'] = null;
  for (const r of rows) {
    if (r.km !== null && (!farthest || r.km > farthest.km)) farthest = { call: r.receiver, km: r.km };
  }
  return { stations, farthest };
}

/** Il gruppo di bande della propagazione calcolata (come lo scrive hamqsl.com) a cui appartiene una banda. */
export function conditionGroup(band: string | null): string | null {
  switch (band) {
    case '80m':
    case '40m':
      return '80m-40m';
    case '30m':
    case '20m':
      return '30m-20m';
    case '17m':
    case '15m':
      return '17m-15m';
    case '12m':
    case '10m':
      return '12m-10m';
    default:
      return null;
  }
}

/** Quante stazioni diverse hanno ricevuto su ogni banda, nell'ordine delle bande (dalla piu' bassa). */
export function bandCounts(rows: readonly PskRow[]): Array<{ band: string; count: number }> {
  const seen = new Map<string, Set<string>>();
  for (const r of rows) {
    if (!r.band) continue;
    const set = seen.get(r.band) ?? new Set<string>();
    set.add(r.receiver);
    seen.set(r.band, set);
  }
  return BAND_NAMES.filter((b) => seen.has(b)).map((band) => ({ band, count: seen.get(band)?.size ?? 0 }));
}

/** Il rapporto migliore ricevuto (il piu' alto), con chi l'ha dato. */
export function bestSnr(rows: readonly PskRow[]): { call: string; snr: number } | null {
  let best: { call: string; snr: number } | null = null;
  for (const r of rows) {
    if (r.snr !== null && (!best || r.snr > best.snr)) best = { call: r.receiver, snr: r.snr };
  }
  return best;
}

/** Un solo ricevente per banda (l'ultimo rapporto): serve a disegnare una linea per ognuno e non centinaia sovrapposte. */
export function latestPerReceiver(rows: readonly PskRow[]): PskRow[] {
  const byKey = new Map<string, PskRow>();
  for (const r of rows) {
    const key = `${r.receiver}|${r.band ?? ''}`;
    const old = byKey.get(key);
    if (!old || r.time_s > old.time_s) byKey.set(key, r);
  }
  return [...byKey.values()];
}
