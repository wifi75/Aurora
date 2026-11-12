import type { AudioLevel, StationStatus } from '../api/schemas';
import { bandOf } from './format';
import { modeBase } from './period';

export type Severity = 'problem' | 'warning' | 'info';
export type IssueId = 'clock' | 'callsign' | 'audio' | 'quiet' | 'audio_loss' | 'no_radio' | 'no_cat' | 'ptt_failed' | 'power_knob' | 'omnirig' | 'port_busy' | 'slow_pc' | 'audio_missing' | 'audio_out_missing' | 'contest';

/** Quanto resta sullo schermo un avviso arrivato dal motore. */
export const NOTICE_MS = 10 * 60_000;
const NOTICES: Record<string, { id: IssueId; severity: Severity; href?: string }> = {
  slow_pc: { id: 'slow_pc', severity: 'warning' },
  audio_missing: { id: 'audio_missing', severity: 'problem', href: '#/impostazioni/audio' },
  audio_out_missing: { id: 'audio_out_missing', severity: 'problem', href: '#/impostazioni/audio' },
  audio_loss: { id: 'audio_loss', severity: 'warning', href: '#/impostazioni/audio' },
  no_radio: { id: 'no_radio', severity: 'warning', href: '#/impostazioni/radio' },
  no_cat: { id: 'no_cat', severity: 'warning', href: '#/impostazioni/radio' },
  ptt_failed: { id: 'ptt_failed', severity: 'problem', href: '#/impostazioni/radio' },
  power_knob: { id: 'power_knob', severity: 'info' },
  omnirig: { id: 'omnirig', severity: 'problem', href: '#/impostazioni/radio' },
  port_busy: { id: 'port_busy', severity: 'problem', href: '#/impostazioni/radio' },
  // contest acceso e banda o modo fuori regolamento (il motore non trasmette), o nominativo composto
  contest: { id: 'contest', severity: 'warning', href: '#/contest' },
};

/** Il motivo dell'avviso `contest` del motore (testo in italiano, core/crates/engine-native/src/tx.rs), da dire nella lingua scelta. */
export type ContestReason = { kind: 'band' | 'mode'; value: string } | { kind: 'myCompound' | 'dxCompound' };

/** Il modo senza periodo e sottomodo («FST4-60» → «FST4», «Q65-30A» → «Q65»), per confrontare lo stato con il nome del motore. */
const modeFamily = (mode: string) => modeBase(mode).toUpperCase().split('-')[0] ?? '';

export function parseContestNotice(detail: string): ContestReason | null {
  const band = /la banda (\S+) non e' ammessa/.exec(detail);
  if (band?.[1]) return { kind: 'band', value: band[1] };
  const mode = /il modo (\S+) non e' ammesso/.exec(detail);
  if (mode?.[1]) return { kind: 'mode', value: mode[1] };
  if (detail.includes('il tuo nominativo composto')) return { kind: 'myCompound' };
  if (detail.includes('nominativo composto del DX')) return { kind: 'dxCompound' };
  return null;
}

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
  /** Avvisi arrivati dal motore. */
  notices?: Record<string, { detail: string; at: number }>;
  /** La profondita' di decodifica attuale (`DecodeDepth` delle impostazioni del motore), se letta. */
  decodeDepth?: string | null;
  /** Le profondita' per cui l'operatore ha chiuso il consiglio del PC lento in questa sessione. */
  slowPcDismissed?: readonly string[];
  /** Lo stato del CAT letto per ultimo e quando (ms Unix), se letto: una radio collegata dopo l'avviso toglie `port_busy`. */
  cat?: { connected: boolean; at: number } | null;
}

/** Il dettaglio dell'avviso `slow_pc` del motore, `"11.7|3"`: secondi della decodifica e profondita' in uso (le versioni di prima mandano solo i secondi). */
export function parseSlowPc(detail: string): { secs: string; depth: string | null } {
  const [secs = '', depth = ''] = detail.split('|');
  return { secs, depth: /^[123]$/.test(depth) ? depth : null };
}

/** La profondita' da proporre a un PC lento: un gradino sotto quella in uso (Profonda → Normale → Veloce); a Veloce non c'e' altro da togliere. */
export function slowerDepth(depth: string | null): '2' | '1' | null {
  return depth === '3' ? '2' : depth === '2' ? '1' : null;
}

// Il consiglio chiuso non torna per la stessa profondita' fino al riavvio di Aurora (non si insiste: il PC resta quello)
const slowPcClosed = new Set<string>();
export function closeSlowPc(depth: string): void {
  slowPcClosed.add(depth);
}
export function slowPcClosedDepths(): string[] {
  return [...slowPcClosed];
}
/** Solo per le prove. */
export function resetSlowPcClosed(): void {
  slowPcClosed.clear();
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

  // in trasmissione l'ingresso dice poco: molte radio mettono a zero l'audio USB in TX (un periodo FT8 dura piu' degli 8 s del «muto»), altre
  // rimandano il segnale trasmesso. Luc, 2026-11-10: «L'ingresso audio e' muto da troppo tempo» compariva a ogni trasmissione
  const audio = i.status?.transmitting ? null : i.audio;
  if (audio?.state === 'overdrive') issues.push({ id: 'audio', severity: 'problem', vars: { state: 'overdrive' } });
  else if (audio?.state === 'silent') issues.push({ id: 'audio', severity: 'warning', vars: { state: 'silent' } });
  else if (audio?.state === 'low') issues.push({ id: 'audio', severity: 'info', vars: { state: 'low' } });

  if (i.status && i.status.de_call.trim() === '') issues.push({ id: 'callsign', severity: 'warning', href: '#/impostazioni/general', vars: {} });

  if (i.status && !i.status.transmitting && i.sessionStartAt !== null) {
    const since = Math.max(i.sessionStartAt, i.lastDecodeAt ?? 0);
    if (i.now - since >= QUIET_AFTER_MS) issues.push({ id: 'quiet', severity: 'info', vars: { min: Math.floor((i.now - since) / 60_000) } });
  }

  for (const [key, n] of Object.entries(i.notices ?? {})) {
    const def = NOTICES[key];
    // l'ingresso mancante non vale piu' appena l'audio arriva
    if (key === 'audio_missing' && i.audio) continue;
    // la porta occupata non vale piu' appena la radio risulta collegata (letto dopo l'avviso)
    if (key === 'port_busy' && i.cat?.connected && i.cat.at >= n.at) continue;
    if (key === 'contest') {
      if (i.now - n.at > NOTICE_MS) continue;
      const why = parseContestNotice(n.detail);
      // banda o modo gia' cambiati: l'avviso non vale piu'
      if (why?.kind === 'band' && i.status && (bandOf(i.status.dial_hz) ?? '').toLowerCase() !== why.value.toLowerCase()) continue;
      if (why?.kind === 'mode' && i.status && modeFamily(i.status.mode) !== modeFamily(why.value)) continue;
      const vars: Record<string, string | number> = { detail: n.detail, at: n.at, ...(why ? { reason: why.kind } : {}), ...(why && 'value' in why ? { value: why.value } : {}) };
      issues.push({ id: 'contest', severity: why?.kind === 'band' || why?.kind === 'mode' ? 'problem' : 'warning', href: '#/contest', vars });
      continue;
    }
    // PC lento: con la profondita' nota si propone il gradino sotto, con un pulsante; mai cambiata da sola
    if (key === 'slow_pc') {
      if (i.now - n.at > NOTICE_MS) continue;
      const { secs, depth } = parseSlowPc(n.detail);
      // chiuso per questa profondita', o profondita' gia' cambiata dopo l'avviso: si aspetta il prossimo avviso del motore
      if (depth && (i.slowPcDismissed?.includes(depth) || (i.decodeDepth && i.decodeDepth !== depth))) continue;
      const next = slowerDepth(depth);
      issues.push(
        next && depth
          ? { id: 'slow_pc', severity: 'warning', href: '#/impostazioni/filters', vars: { detail: secs, at: n.at, depth, next } }
          : { id: 'slow_pc', severity: 'warning', vars: { detail: secs, at: n.at, ...(depth ? { depth } : {}) } },
      );
      continue;
    }
    if (def && i.now - n.at <= NOTICE_MS) issues.push({ id: def.id, severity: def.severity, ...(def.href ? { href: def.href } : {}), vars: { detail: n.detail, at: n.at } });
  }

  const order: Record<Severity, number> = { problem: 0, warning: 1, info: 2 };
  return issues.sort((a, b) => order[a.severity] - order[b.severity]);
}
