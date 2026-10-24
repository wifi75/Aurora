import type { AudioLevel, StationStatus } from '../api/schemas';

export type Severity = 'problem' | 'warning' | 'info';
export type IssueId = 'clock' | 'callsign' | 'audio' | 'quiet';

export interface Issue {
  id: IssueId;
  severity: Severity;
  /** Dove si sistema, se c'e' una pagina per farlo. */
  href?: string;
  /** Valori da mettere nel testo. */
  vars: Record<string, string | number>;
}

export interface DiagnosisInput {
  now: number;
  status: StationStatus | null;
  /** Scarto dell'orologio del PC dai server NTP (ms), se misurato. */
  clockOffsetMs: number | null;
  /** Durata del periodo in secondi (15 per FT8, 7,5 per FT4). */
  periodSecs: number;
  /** L'ultima misura dell'audio in ingresso, solo se recente; altrimenti `null`. */
  audio: AudioLevel | null;
  /** Quando e' arrivato l'ultimo messaggio decodificato (ms Unix). */
  lastDecodeAt: number | null;
  /** Quando e' arrivato il primo stato del motore in questa sessione (ms Unix). */
  sessionStartAt: number | null;
}

/** Soglie dell'orologio in parti del periodo: oltre il 4 % i messaggi deboli si perdono, oltre l'8 % la decodifica peggiora molto. */
export const CLOCK_WARN_FRACTION = 0.04;
export const CLOCK_PROBLEM_FRACTION = 0.08;
/** Senza nessun messaggio per tanto, vale la pena dirlo. */
export const QUIET_AFTER_MS = 3 * 60_000;

/** Cio' che non va, dal piu' grave. Vuoto = tutto a posto (o non abbastanza dati per dirlo). */
export function diagnose(i: DiagnosisInput): Issue[] {
  const issues: Issue[] = [];

  if (i.clockOffsetMs !== null && i.periodSecs > 0) {
    const fraction = Math.abs(i.clockOffsetMs) / (i.periodSecs * 1000);
    if (fraction >= CLOCK_WARN_FRACTION) {
      issues.push({
        id: 'clock',
        severity: fraction >= CLOCK_PROBLEM_FRACTION ? 'problem' : 'warning',
        href: '#/impostazioni/clock',
        vars: { ms: Math.round(Math.abs(i.clockOffsetMs)) },
      });
    }
  }

  if (i.audio?.state === 'overdrive') issues.push({ id: 'audio', severity: 'problem', vars: { state: 'overdrive' } });
  else if (i.audio?.state === 'silent') issues.push({ id: 'audio', severity: 'warning', vars: { state: 'silent' } });
  else if (i.audio?.state === 'low') issues.push({ id: 'audio', severity: 'info', vars: { state: 'low' } });

  if (i.status && i.status.de_call.trim() === '') issues.push({ id: 'callsign', severity: 'warning', href: '#/impostazioni/general', vars: {} });

  if (i.status && !i.status.transmitting && i.sessionStartAt !== null) {
    const since = Math.max(i.sessionStartAt, i.lastDecodeAt ?? 0);
    if (i.now - since >= QUIET_AFTER_MS) issues.push({ id: 'quiet', severity: 'info', vars: { min: Math.floor((i.now - since) / 60_000) } });
  }

  const order: Record<Severity, number> = { problem: 0, warning: 1, info: 2 };
  return issues.sort((a, b) => order[a.severity] - order[b.severity]);
}
