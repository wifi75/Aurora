import type { HealthResources, ResourceSample } from '../api/client';
import type { Issue, IssueId } from './diagnosis';

/** Stato di una riga della spia e della spia intera. */
export type Level = 'ok' | 'warn' | 'bad' | 'none';

export const CPU_WARN = 50;
export const CPU_BAD = 80;
/** Per quanto la CPU deve restare sopra la soglia prima di cambiare colore. */
export const CPU_HOLD_MS = 10_000;
/** Oltre questa parte del periodo la decodifica e' lenta (come il motore). */
export const DECODE_WARN = 0.7;
/** Parte dei campioni con la decodifica lenta oltre la quale si consiglia «PC lento». */
export const SLOW_OFTEN = 0.3;

const RANK: Record<Level, number> = { none: 0, ok: 1, warn: 2, bad: 3 };
export const worst = (...levels: Level[]): Level => levels.reduce<Level>((a, b) => (RANK[b] > RANK[a] ? b : a), 'none');

/** `true` se gli ultimi campioni restano sopra `limit` da almeno `CPU_HOLD_MS`. */
export function cpuAbove(history: readonly ResourceSample[], limit: number): boolean {
  const last = history.at(-1);
  if (!last || last.cpu_pct <= limit) return false;
  let first = last;
  for (let i = history.length - 1; i >= 0; i--) {
    const s = history[i];
    if (!s || s.cpu_pct <= limit) break;
    first = s;
  }
  return last.t - first.t >= CPU_HOLD_MS;
}

export function cpuLevel(r: HealthResources | null | undefined): Level {
  if (!r || r.history.length === 0) return 'none';
  if (cpuAbove(r.history, CPU_BAD)) return 'bad';
  if (cpuAbove(r.history, CPU_WARN)) return 'warn';
  return 'ok';
}

export function decodeLevel(r: HealthResources | null | undefined): Level {
  if (!r || r.last_decode_ms === null || !r.period_ms) return 'none';
  if (r.last_decode_ms > r.period_ms) return 'bad';
  if (r.last_decode_ms > r.period_ms * DECODE_WARN) return 'warn';
  return 'ok';
}

/** La decodifica sfora spesso: tra i campioni (almeno un minuto) con una decodifica, almeno `SLOW_OFTEN` oltre il 70 % del tempo. */
export function decodeSlowOften(r: HealthResources | null | undefined): boolean {
  const withDecode = (r?.history ?? []).filter((s) => s.period_ms > 0);
  if (withDecode.length < 12) return false;
  return withDecode.filter((s) => s.decode_ms > s.period_ms * DECODE_WARN).length / withDecode.length >= SLOW_OFTEN;
}

const GROUPS: Record<'audio' | 'cat' | 'clock', readonly IssueId[]> = {
  audio: ['audio', 'quiet', 'audio_loss', 'audio_missing', 'audio_out_missing'],
  cat: ['no_radio', 'no_cat', 'ptt_failed', 'omnirig', 'port_busy'],
  clock: ['clock'],
};

/** Lo stato di audio, CAT o orologio dagli avvisi gia' calcolati: problema = rosso, avviso = ambra, nessuno = verde. */
export function issueLevel(issues: readonly Issue[], group: keyof typeof GROUPS): Level {
  let level: Level = 'ok';
  for (const i of issues) {
    if (!GROUPS[group].includes(i.id)) continue;
    level = worst(level, i.severity === 'problem' ? 'bad' : i.severity === 'warning' ? 'warn' : 'ok');
  }
  return level;
}

export interface HealthRows {
  cpu: Level;
  decode: Level;
  audio: Level;
  cat: Level;
  clock: Level;
}

export function healthRows(r: HealthResources | null | undefined, issues: readonly Issue[]): HealthRows {
  return { cpu: cpuLevel(r), decode: decodeLevel(r), audio: issueLevel(issues, 'audio'), cat: issueLevel(issues, 'cat'), clock: issueLevel(issues, 'clock') };
}

/** Il colore della spia: il peggiore delle righe; senza nessun dato resta verde. */
export function overall(rows: HealthRows): Exclude<Level, 'none'> {
  const w = worst(rows.cpu, rows.decode, rows.audio, rows.cat, rows.clock);
  return w === 'none' ? 'ok' : w;
}
