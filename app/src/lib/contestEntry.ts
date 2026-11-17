/**
 * Aiuti della riga di inserimento dei contest: suggerimenti dal nominativo (zona CQ, zona ITU, prefisso DXCC), scambio ricevuto
 * precompilato, QSO precedenti per banda e riepilogo dei moltiplicatori per tipo.
 */
import type { TextContest } from '../api/contestTesto';
import { qsoTimeMs } from './logWindow';

type DxccLike = { prefix: string; cq_zone: number; itu_zone: number };

export interface Suggestion {
  kind: 'cq' | 'itu' | 'dxcc';
  value: string;
}

/** I suggerimenti accesi nel contest per un nominativo (zona CQ, zona ITU, prefisso DXCC), dal database dei prefissi. */
export function contestSuggestions(c: Pick<TextContest, 'suggest_cq_zone' | 'suggest_itu_zone' | 'suggest_dxcc_pfx'>, info: DxccLike | null | undefined): Suggestion[] {
  if (!info) return [];
  const out: Suggestion[] = [];
  if (c.suggest_cq_zone && info.cq_zone > 0) out.push({ kind: 'cq', value: String(info.cq_zone) });
  if (c.suggest_itu_zone && info.itu_zone > 0) out.push({ kind: 'itu', value: String(info.itu_zone) });
  if (c.suggest_dxcc_pfx && info.prefix) out.push({ kind: 'dxcc', value: info.prefix });
  return out;
}

/** Cosa si prende da solo (`m_iGrabNumberType`): 0 il seriale dal testo ricevuto, 1 la zona ITU, 2 la zona CQ; -1 niente. */
export function grabKind(c: Pick<TextContest, 'grab_number' | 'grab_number_type'>): 'serial' | 'itu' | 'cq' | null {
  if (!c.grab_number) return null;
  return c.grab_number_type === 2 ? 'cq' : c.grab_number_type === 1 ? 'itu' : c.grab_number_type === 0 ? 'serial' : null;
}

/** Lo scambio ricevuto che si precompila dal nominativo (solo la zona CQ o ITU: il seriale non si puo' indovinare), o `null`. */
export function autoExchange(c: Pick<TextContest, 'grab_number' | 'grab_number_type'>, info: DxccLike | null | undefined): string | null {
  if (!info) return null;
  const k = grabKind(c);
  if (k === 'cq') return info.cq_zone > 0 ? String(info.cq_zone) : null;
  if (k === 'itu') return info.itu_zone > 0 ? String(info.itu_zone) : null;
  return null;
}

// ---------------------------------------------------------------- QSO precedenti per banda

export const HF_BANDS = ['160m', '80m', '40m', '20m', '15m', '10m'] as const;

type PrevSrc = { call: string; band: string; mode: string; date: string; time: string; comment?: string };

export interface PreviousQso {
  band: string;
  /** `null` = non ancora lavorato su questa banda. */
  qso: { mode: string; time_ms: number; nr: string } | null;
}

/** Lo scambio ricevuto scritto in coda al commento del QSO (`NRS 001 NRR 015`, o con la sezione `NRR 003 ACC`). */
export function exchangeRcvdOf(comment: string | undefined): string {
  return /NRR (.+)$/.exec(comment ?? '')?.[1]?.trim() ?? '';
}

/**
 * I QSO gia' fatti con un nominativo nel contest, una riga per banda: le sei HF dei contest (160-10 m, senza WARC). Per ogni
 * banda conta l'ultimo QSO (modo, ora, scambio ricevuto). `inContest` dice se il QSO e' del contest in corso.
 */
export function previousQsos<T extends PrevSrc>(qsos: readonly T[], call: string, inContest: (q: T) => boolean, bands: readonly string[] = HF_BANDS): PreviousQso[] {
  const key = call.trim().toUpperCase();
  const last = new Map<string, { mode: string; time_ms: number; nr: string }>();
  if (key.length >= 3) {
    for (const q of qsos) {
      if (q.call.trim().toUpperCase() !== key || !inContest(q)) continue;
      const t = qsoTimeMs(q);
      const b = q.band.toLowerCase();
      const cur = last.get(b);
      if (!cur || t >= cur.time_ms) last.set(b, { mode: q.mode, time_ms: t, nr: exchangeRcvdOf(q.comment) });
    }
  }
  return bands.map((band) => ({ band, qso: last.get(band) ?? null }));
}

// ---------------------------------------------------------------- statistiche per tipo di moltiplicatore

interface ScoreMults {
  tipi: string[];
  bande: { band: string }[];
  griglia: { tipo: string; valore: string; bande: string[] }[];
  mancanti: { tipo: string; per_banda: boolean; totali: number; valori: string[] }[];
}

export interface MultSummary {
  tipo: string;
  worked: number;
  /** I valori possibili, solo per i tipi con un elenco chiuso (zone, continenti, aree): per paesi e prefissi il core non ha un totale. */
  total: number | null;
  /** Per banda: quanti valori lavorati; `null` se il tipo conta una volta sola per tutto il contest. */
  perBand: Record<string, number> | null;
}

/** Per ogni tipo di moltiplicatore del contest: lavorati, totali (se noti) e lavorati per banda. */
export function multSummary(s: ScoreMults): MultSummary[] {
  const types = [...s.tipi, ...s.griglia.map((g) => g.tipo), ...s.mancanti.map((m) => m.tipo)].filter((x, i, all) => all.indexOf(x) === i);
  return types.map((tipo) => {
    const rows = s.griglia.filter((g) => g.tipo === tipo);
    const miss = s.mancanti.find((m) => m.tipo === tipo);
    const perBand = rows.some((r) => r.bande.length > 0) || miss?.per_banda === true;
    return {
      tipo,
      worked: rows.length,
      total: miss ? miss.totali : null,
      perBand: perBand ? Object.fromEntries(s.bande.map((b) => [b.band, rows.filter((r) => r.bande.includes(b.band)).length])) : null,
    };
  });
}

export interface CallMultRow {
  tipo: string;
  value: string;
  fresh: boolean;
}

const sameValue = (a: string, b: string) => (/^\d+$/.test(a) && /^\d+$/.test(b) ? Number(a) === Number(b) : a.toUpperCase() === b.toUpperCase());

/**
 * I moltiplicatori che porta un nominativo, uno per tipo noto dal database dei prefissi (paese, zona CQ, zona ITU, continente):
 * `fresh` se non sono ancora nella griglia del contest (sulla banda) o il core li da' per nuovi, altrimenti gia' lavorati. I tipi senza valore dal database (WPX, domini,
 * aree) compaiono solo se nuovi, dai `nuovi` del core.
 */
export function callMultipliers(
  tipi: readonly string[],
  info: (DxccLike & { continent?: string }) | null | undefined,
  nuovi: readonly string[],
  griglia: readonly { tipo: string; valore: string; bande: string[] }[] = [],
  band = '',
): CallMultRow[] {
  const fromInfo: Record<string, string> = info ? { dxcc: info.prefix, cq: info.cq_zone > 0 ? String(info.cq_zone) : '', itu: info.itu_zone > 0 ? String(info.itu_zone) : '', cont: info.continent ?? '' } : {};
  const rows: CallMultRow[] = [];
  for (const tipo of tipi) {
    const v = fromInfo[tipo];
    if (!v) continue;
    // gia' lavorato solo se sta nella griglia del contest (sulla banda, per i tipi per banda); altrimenti e' nuovo
    const worked = griglia.some((g) => g.tipo === tipo && sameValue(g.valore, v) && (g.bande.length === 0 || band === '' || g.bande.includes(band)));
    rows.push({ tipo, value: v, fresh: !worked || nuovi.some((n) => n.startsWith(`${tipo}:`) && sameValue(n.slice(tipo.length + 1), v)) });
  }
  for (const n of nuovi) {
    const i = n.indexOf(':');
    if (i < 0) continue;
    const tipo = n.slice(0, i);
    const value = n.slice(i + 1);
    if (!rows.some((r) => r.tipo === tipo && sameValue(r.value, value))) rows.push({ tipo, value, fresh: true });
  }
  return rows;
}
