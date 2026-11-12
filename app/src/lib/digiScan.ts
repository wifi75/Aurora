import type { DigiSettings } from '../api/client';

/**
 * La scansione e le memorie della pagina PSK · RTTY · CW (menu Modo): stanno nel browser, e si leggono e cambiano sia
 * dalla pagina sia da Impostazioni › Modi › PSK · RTTY · CW › Scansione e memorie. La pagina segue le modifiche fatte nell'altra
 * finestra con l'evento `storage`.
 */

export const SCAN_KEY = 'aurora.digi.scansione';
export const MEM_KEY = 'aurora.digi.memorie';
/** Quante memorie ha il menu. */
export const MEMORY_SLOTS = 20;

export interface ScanProps {
  /** Secondi di sosta su ogni stazione (almeno 2). */
  dwell: number;
  /** Fermarsi alla prima stazione trovata. */
  once: boolean;
  /** Scrivere l'ora di ogni stazione nel testo ricevuto. */
  timeMarks: boolean;
  /** Scrivere la frequenza di ogni stazione nel testo ricevuto. */
  freqMarks: boolean;
}

export const DEFAULT_SCAN: ScanProps = { dwell: 10, once: false, timeMarks: true, freqMarks: true };

/** Una memoria: il modo e le impostazioni di sintonia salvate col menu Modo › Memorie. */
export type DigiMemory = Partial<Pick<DigiSettings, 'mode' | 'rx_freq_hz' | 'tx_freq_hz' | 'tx_follow_rx' | 'rtty_shift_hz' | 'rtty_baud' | 'rtty_reverse' | 'afc_on' | 'snap_on' | 'squelch_on' | 'psk_squelch' | 'cw_squelch_snr_db' | 'rtty_squelch_snr_db'>>;

function read(storage: Storage | null | undefined, key: string): unknown {
  try {
    const raw = storage?.getItem(key);
    return raw ? (JSON.parse(raw) as unknown) : null;
  } catch {
    return null;
  }
}

function write(storage: Storage | null | undefined, key: string, value: unknown): void {
  try {
    storage?.setItem(key, JSON.stringify(value));
  } catch {
    // senza archivio la scelta vale finche' la pagina e' aperta
  }
}

/** Le proprieta' della scansione, con i valori di base per quelle mancanti o rovinate. */
export function readScan(storage: Storage | null | undefined): ScanProps {
  const v = read(storage, SCAN_KEY);
  if (!v || typeof v !== 'object') return DEFAULT_SCAN;
  const o = v as Partial<Record<keyof ScanProps, unknown>>;
  const dwell = typeof o.dwell === 'number' && Number.isFinite(o.dwell) ? Math.min(600, Math.max(2, Math.round(o.dwell))) : DEFAULT_SCAN.dwell;
  const flag = (k: 'once' | 'timeMarks' | 'freqMarks') => (typeof o[k] === 'boolean' ? o[k] : DEFAULT_SCAN[k]);
  return { dwell, once: flag('once'), timeMarks: flag('timeMarks'), freqMarks: flag('freqMarks') };
}

export function writeScan(storage: Storage | null | undefined, props: ScanProps): void {
  write(storage, SCAN_KEY, props);
}

/** Le memorie salvate, per numero (1-20). */
export function readMemories(storage: Storage | null | undefined): Record<string, DigiMemory> {
  const v = read(storage, MEM_KEY);
  return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, DigiMemory>) : {};
}

export function writeMemories(storage: Storage | null | undefined, all: Record<string, DigiMemory>): void {
  write(storage, MEM_KEY, all);
}

/** Svuota una memoria; le altre restano come sono. */
export function clearMemory(storage: Storage | null | undefined, slot: number): void {
  const all = readMemories(storage);
  delete all[String(slot)];
  writeMemories(storage, all);
}

const MODE_NAMES: Record<DigiSettings['mode'], string> = { psk31: 'PSK31', psk63: 'PSK63', rtty: 'RTTY', cw: 'CW' };

/** Riassunto di una memoria per l'elenco: modo, frequenza di ricezione e, in RTTY, spostamento e velocita'. */
export function memorySummary(m: DigiMemory): string {
  const parts: string[] = [];
  if (m.mode) parts.push(MODE_NAMES[m.mode] ?? m.mode);
  if (typeof m.rx_freq_hz === 'number') parts.push(`Rx ${Math.round(m.rx_freq_hz)} Hz`);
  if (m.tx_follow_rx === false && typeof m.tx_freq_hz === 'number') parts.push(`Tx ${Math.round(m.tx_freq_hz)} Hz`);
  if (m.mode === 'rtty' && typeof m.rtty_shift_hz === 'number' && typeof m.rtty_baud === 'number') parts.push(`${Math.round(m.rtty_shift_hz)} Hz / ${m.rtty_baud} bd`);
  return parts.join(' · ');
}
