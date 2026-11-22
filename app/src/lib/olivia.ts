/**
 * Olivia e Contestia nella pagina dei modi testuali: valori ammessi, impostazioni di base, controllo che toni e banda stiano nella banda
 * audio (lo stesso calcolo del motore, `core/crates/mode-digi/src/olivia.rs` e `olivia_rx.rs`), nomi dei modi nelle macro
 * (`OLIVIA-8-250`, `CONTESTIA 16/500`...) e i comandi `<SET TONES:n>`, `<SET BW:Hz>`, `<SET SUBMODE:n>`.
 */
import type { DigiSettings, OliviaSettings } from '../api/client';

export type OliviaMode = 'olivia' | 'contestia';

/** I toni e le bande che il motore accetta. */
export const OLIVIA_TONES = [4, 8, 16, 32, 64] as const;
export const OLIVIA_BANDWIDTHS = [250, 500, 1000, 2000] as const;
/** Limiti del sincronismo (margine e integrazione) e dello squelch, come nel motore. */
export const OLIVIA_SYNC_MIN = 2;
export const OLIVIA_SYNC_MAX = 32;
export const OLIVIA_SQUELCH_MAX = 40;
/** Il margine del sincronismo con cui il motore controlla la trasmissione (`tx_stream.rs`). */
const TX_SYNC_MARGIN = 8;

export const OLIVIA_DEFAULTS: Readonly<Record<OliviaMode, OliviaSettings>> = {
  olivia: { tones: 8, bandwidth_hz: 250, sync_margin: 8, sync_integration: 4, squelch_snr_db: 12 },
  contestia: { tones: 16, bandwidth_hz: 500, sync_margin: 8, sync_integration: 4, squelch_snr_db: 12 },
};

export function isOliviaMode(mode: string): mode is OliviaMode {
  return mode === 'olivia' || mode === 'contestia';
}

/** Le impostazioni del modo (quelle di base se un motore piu' vecchio non le manda). */
export function oliviaOf(s: DigiSettings, mode: OliviaMode): OliviaSettings {
  return { ...OLIVIA_DEFAULTS[mode], ...(s[mode] ?? {}) };
}

/** Il valore dell'elenco piu' vicino a `v`. */
export function nearest(list: readonly number[], v: number): number {
  return list.reduce((a, b) => (Math.abs(b - v) < Math.abs(a - v) ? b : a));
}

/** Le impostazioni dentro i limiti del motore (toni e banda dell'elenco, sincronismo 2-32, squelch 0-40 dB). */
export function oliviaValid(o: OliviaSettings): boolean {
  const inSync = (v: number) => Number.isInteger(v) && v >= OLIVIA_SYNC_MIN && v <= OLIVIA_SYNC_MAX;
  return (
    (OLIVIA_TONES as readonly number[]).includes(o.tones) &&
    (OLIVIA_BANDWIDTHS as readonly number[]).includes(o.bandwidth_hz) &&
    inSync(o.sync_margin) &&
    inSync(o.sync_integration) &&
    Number.isFinite(o.squelch_snr_db) &&
    o.squelch_snr_db >= 0 &&
    o.squelch_snr_db <= OLIVIA_SQUELCH_MAX
  );
}

/**
 * Toni e banda centrati su `centreHz` stanno nella banda audio? Stesso conto del motore: lunghezza del simbolo, primo tono in bin della FFT
 * e controllo del ricevitore (`olivia_rx.rs`, `Receiver::new`); in trasmissione il margine e' sempre 8.
 */
export function oliviaFits(tones: number, bandwidthHz: number, centreHz: number, syncMargin = TX_SYNC_MARGIN): boolean {
  const bits = Math.log2(2 ** Math.ceil(Math.log2(Math.max(tones, 2))));
  if (bits < 1 || bits > 8) return false;
  const bwLog = Math.floor(Math.log2(Math.max(1, Math.floor(Math.min(2000, Math.max(125, bandwidthHz)) / 125))));
  const symbolLen = 2 ** (bits + 7 - bwLog);
  const bandwidth = 2 ** bwLog * 125;
  const fcOffset = (bandwidth * (1 - 0.5 / tones)) / 2;
  const multiplier = (centreHz - fcOffset) / 500;
  // il motore converte a intero senza segno: un valore negativo diventa 0
  const firstCarrier = Math.max(0, Math.trunc((symbolLen / 16) * multiplier)) + 1;
  const margin = Math.min(Math.max(syncMargin, 2), firstCarrier);
  const decodeWidth = 2 ** bits * 2 - 1 + 2 * margin;
  return firstCarrier - margin + decodeWidth <= symbolLen / 2;
}

/** La frequenza a cui il motore trasmette: quella di ricezione se la trasmissione la segue (Lock spento), come `tx_freq()` del motore. */
export function effectiveTxHz(s: DigiSettings): number {
  return s.tx_follow_rx ? s.rx_freq_hz : s.tx_freq_hz;
}

/** Il modo scelto e' Olivia o Contestia e toni e banda non entrano nell'audio alla frequenza di trasmissione o di ricezione. */
export function oliviaProblem(s: DigiSettings): 'tx' | 'rx' | null {
  if (!isOliviaMode(s.mode)) return null;
  const o = oliviaOf(s, s.mode);
  if (!oliviaFits(o.tones, o.bandwidth_hz, effectiveTxHz(s))) return 'tx';
  if (!oliviaFits(o.tones, o.bandwidth_hz, s.rx_freq_hz, o.sync_margin)) return 'rx';
  return null;
}

/** La sigla corta del modo: «8/250». */
export function oliviaLabel(o: OliviaSettings): string {
  return `${o.tones}/${o.bandwidth_hz}`;
}

/** Il cambio di una parte delle impostazioni del modo: si manda sempre tutto l'oggetto (il motore rimette di base i campi mancanti). */
export function oliviaPatch(s: DigiSettings, mode: OliviaMode, part: Partial<OliviaSettings>): Partial<DigiSettings> {
  return { [mode]: { ...oliviaOf(s, mode), ...part } };
}

/**
 * Il nome di un modo Olivia o Contestia nelle macro e nei file di fldigi: `OLIVIA`, `CONTESTIA`, `OLIVIA-8-250`, `OLIVIA 8/250`,
 * `CONTESTIA-16-500` (toni e banda si arrotondano ai valori ammessi: vedi `oliviaRounded`). `null` per gli altri modi.
 */
export function parseOliviaName(name: string): { mode: OliviaMode; tones?: number; bandwidth_hz?: number } | null {
  const m = /^(OLIVIA|CONTESTIA)(?:[\s_-]+(\d+)(?:\s*[-/]\s*(\d+))?)?$/i.exec(name.trim());
  if (!m) return null;
  const mode = (m[1] ?? '').toLowerCase() as OliviaMode;
  const out: { mode: OliviaMode; tones?: number; bandwidth_hz?: number } = { mode };
  if (m[2] !== undefined) out.tones = nearest(OLIVIA_TONES, Number(m[2]));
  if (m[3] !== undefined) out.bandwidth_hz = nearest(OLIVIA_BANDWIDTHS, Number(m[3]));
  return out;
}

/** Toni o banda del nome diversi dai valori ammessi (per esempio 125 Hz): il valore scelto al loro posto non si legge con l'altra stazione. */
export function oliviaRounded(name: string): boolean {
  const m = /^(?:OLIVIA|CONTESTIA)[\s_-]+(\d+)(?:\s*[-/]\s*(\d+))?$/i.exec(name.trim()) ?? /^(\d+)\s*[-/]\s*(\d+)$/.exec(name.trim());
  if (!m) return false;
  const tones = Number(m[1]);
  const bw = m[2] === undefined ? null : Number(m[2]);
  return !(OLIVIA_TONES as readonly number[]).includes(tones) || (bw !== null && !(OLIVIA_BANDWIDTHS as readonly number[]).includes(bw));
}

/** Il cambio di modo (con toni e banda, se il nome li dice) per un nome Olivia o Contestia; `null` per gli altri modi. */
export function oliviaModePatch(s: DigiSettings, name: string): Partial<DigiSettings> | null {
  const p = parseOliviaName(name);
  if (!p) return null;
  const part: Partial<OliviaSettings> = {};
  if (p.tones !== undefined) part.tones = p.tones;
  if (p.bandwidth_hz !== undefined) part.bandwidth_hz = p.bandwidth_hz;
  return { mode: p.mode, ...(Object.keys(part).length > 0 ? oliviaPatch(s, p.mode, part) : {}) };
}

/**
 * `<SET TONES:n>`, `<SET BW:Hz>` e `<SET SUBMODE:...>` in Olivia e Contestia: il cambio da salvare, o `null` se il comando non riguarda
 * questi modi (allora vale quello che fa la pagina per gli altri). SUBMODE accetta `8/250`, `8-250` o un nome completo (`OLIVIA-8-250`).
 */
export function oliviaSetPatch(s: DigiSettings, key: string, value: string): Partial<DigiSettings> | null {
  const k = key.trim().toUpperCase();
  const val = value.trim().toUpperCase();
  if (k === 'SUBMODE') {
    const named = oliviaModePatch(s, val);
    if (named) return named;
    const pair = /^(\d+)\s*[-/]\s*(\d+)$/.exec(val);
    if (pair && isOliviaMode(s.mode)) return oliviaPatch(s, s.mode, { tones: nearest(OLIVIA_TONES, Number(pair[1])), bandwidth_hz: nearest(OLIVIA_BANDWIDTHS, Number(pair[2])) });
    return null;
  }
  if (!isOliviaMode(s.mode)) return null;
  const v = Number(val.replace(',', '.'));
  if (!Number.isFinite(v) || v <= 0) return null;
  if (k === 'TONES') return oliviaPatch(s, s.mode, { tones: nearest(OLIVIA_TONES, v) });
  if (k === 'BW') return oliviaPatch(s, s.mode, { bandwidth_hz: nearest(OLIVIA_BANDWIDTHS, v) });
  return null;
}
