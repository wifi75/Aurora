import { useEffect, useState } from 'react';
import { bandOf } from './format';
import { browserStorage } from './theme';

/**
 * Split DX: il commento di uno spot del DX Cluster dice dove ascolta la DXpedition («UP 5», «5UP», «10up», «UP 1-3», «DN 2», «QSX 14.030»).
 * Con un doppio clic sullo spot la radio riceve sulla frequenza dello spot (VFO A) e trasmette sull'altro VFO spostato (split vero), come
 * il doppio clic. Le regole:
 * - numero e UP/DN in qualunque ordine, attaccati o con lo spazio, maiuscole o minuscole, in qualunque punto del commento; kHz (anche «5K»,
 *   «5KHZ», «1,5»);
 * - «UP» o «DN» da soli: 1 kHz, la consuetudine dei pile-up (soprattutto in CW) quando la DXpedition non dice quanto;
 * - un intervallo («UP 1-3», «UP 5/10») vale il suo centro: la DXpedition ascolta in tutta la finestra, chi arriva dal cluster si ferma quasi
 *   sempre sul bordo basso, e il centro lascia margine per ritoccare il VFO B da entrambe le parti;
 * - «QSX» dice la frequenza di trasmissione: intera («QSX 14030»), in MHz («QSX 14.030») o solo le ultime cifre («QSX 030», dentro lo stesso
 *   MHz dello spot o in quello accanto, la piu' vicina);
 * - un numero che non e' uno spostamento (oltre 50 kHz, un rapporto come «599 UP») non si indovina: niente split;
 * - «UP 14.025» o «UP 14025» nella stessa banda dello spot si leggono come la frequenza dove ascolta, come QSX.
 * Negli spot FT8, FT4 e degli altri modi WSJT non si fa: le DXpedition in FT8 usano Fox/Hound (la trasmissione resta nella banda passante,
 * sopra i 1000 Hz) e in quei modi lo split della radio lo comanda Aurora con l'XIT delle Impostazioni.
 */
export interface SpotSplit {
  kind: 'up' | 'down' | 'qsx';
  /** Spostamento della trasmissione rispetto allo spot, in kHz (negativo = sotto). */
  offsetKhz: number;
  /** La frequenza di trasmissione in kHz. */
  txKhz: number;
  /** Testo breve per l'etichetta della riga: «UP 5», «DN 2», «QSX 14.030». */
  label: string;
}

/** Spostamento massimo accettato come «UP N» (kHz): oltre e' un altro numero (rapporto, frequenza, ora). */
export const MAX_OFFSET_KHZ = 50;
/** Distanza massima di una QSX dallo spot (kHz), come nel core. */
const MAX_QSX_KHZ = 500;

/** I modi in cui lo Split DX non si usa (vedi sopra). */
const WSJT_MODES = /^(FT8|FT4|FST4W?|JT65|JT9|JT4|Q65|MSK144|WSPR)/i;

const NUM = String.raw`\d{1,5}(?:[.,]\d{1,3})?`;
const RANGE = String.raw`(${NUM})(?:\s*[-/]\s*(${NUM}))?`;
const UNIT = String.raw`(?:\s*K(?:HZ)?)?`;
const WORD = String.raw`(UP|DN|DWN|DOWN)`;
/** «UP 5», «UP5», «up 1-3», «DN 2», «UP 5KHZ», «UP +5». */
const AFTER = new RegExp(String.raw`(?<![A-Z0-9])${WORD}\s*\+?\s*${RANGE}${UNIT}(?![A-Z0-9])`, 'gi');
/** «5 UP», «5UP», «10up», «1-3 UP», «2 DOWN». */
const BEFORE = new RegExp(String.raw`(?<![A-Z0-9.,])${RANGE}${UNIT}\s*${WORD}(?![A-Z0-9])`, 'gi');
/** «UP» o «DN» da soli. */
const BARE = new RegExp(String.raw`(?<![A-Z0-9])${WORD}(?![A-Z0-9])`, 'i');
const QSX = new RegExp(String.raw`(?<![A-Z0-9])QSX\s*:?\s*(${NUM})(?![0-9]|[.,][0-9])`, 'i');

function num(text: string): number {
  return Number(text.replace(',', '.'));
}

/** 7.5 -> «7.5», 2 -> «2». */
function short(khz: number): string {
  return String(Math.round(khz * 10) / 10);
}

function sameBand(aKhz: number, bKhz: number): boolean {
  const band = bandOf(aKhz * 1000);
  return band !== null && band === bandOf(bKhz * 1000);
}

/** Il numero come frequenza assoluta (kHz) nella banda dello spot: «14030», «14.030», «14030.5». `null` se non lo e'. */
function absolute(raw: string, spotKhz: number): number | null {
  const v = num(raw);
  const candidates = v >= 1000 ? [v] : /[.,]\d{3}$/.test(raw) ? [v * 1000] : [];
  return candidates.find((c) => c !== spotKhz && sameBand(c, spotKhz) && Math.abs(c - spotKhz) <= MAX_QSX_KHZ) ?? null;
}

/** «QSX 14.030», «QSX 14030», «QSX 030»: la frequenza di trasmissione in kHz, o `null`. */
function qsxKhz(raw: string, spotKhz: number): number | null {
  const whole = absolute(raw, spotKhz);
  if (whole !== null) return whole;
  const v = num(raw);
  // «14.030» o «14030» fuori dalla banda dello spot: non si prova a leggerli come ultime cifre
  if (v >= 1000 || /[.,]\d{3}$/.test(raw)) return null;
  // solo le ultime cifre: dentro lo stesso MHz dello spot o in quello accanto, la piu' vicina
  const base = Math.floor(spotKhz / 1000) * 1000;
  const best = [base - 1000 + v, base + v, base + 1000 + v].reduce((a, b) => (Math.abs(b - spotKhz) < Math.abs(a - spotKhz) ? b : a));
  return best !== spotKhz && sameBand(best, spotKhz) && Math.abs(best - spotKhz) <= MAX_QSX_KHZ ? best : null;
}

function round3(khz: number): number {
  return Math.round(khz * 1000) / 1000;
}

function qsxSplit(txKhz: number, spotKhz: number): SpotSplit {
  return { kind: 'qsx', offsetKhz: round3(txKhz - spotKhz), txKhz: round3(txKhz), label: `QSX ${(txKhz / 1000).toFixed(3)}` };
}

/** «UP»/«DN» con il numero (o l'intervallo) trovato: spostamento, frequenza dove ascolta, o niente se il numero non e' uno spostamento. */
function fromMatch(word: string, first: string, second: string | undefined, spotKhz: number): SpotSplit | null {
  if (second === undefined) {
    const at = absolute(first, spotKhz);
    if (at !== null) return qsxSplit(at, spotKhz);
  }
  const a = num(first);
  const b = second === undefined ? a : num(second);
  const khz = (Math.min(a, b) + Math.max(a, b)) / 2;
  if (!(khz > 0) || Math.max(a, b) > MAX_OFFSET_KHZ) return null;
  const down = !word.toUpperCase().startsWith('U');
  const offset = down ? -khz : khz;
  // l'etichetta ripete l'intervallo del commento; la trasmissione va al centro (il suggerimento della riga lo dice)
  const amount = a === b ? short(a) : `${short(Math.min(a, b))}-${short(Math.max(a, b))}`;
  return { kind: down ? 'down' : 'up', offsetKhz: round3(offset), txKhz: round3(spotKhz + offset), label: `${down ? 'DN' : 'UP'} ${amount}` };
}

/**
 * Lo Split DX scritto nel commento dello spot, o `null` (niente split, numero che non e' uno spostamento, spot FT8/FT4). `spotKhz` e' la
 * frequenza dello spot in kHz, `mode` il modo dello spot (se noto).
 */
export function parseSpotSplit(comment: string | null | undefined, spotKhz: number, mode?: string | null): SpotSplit | null {
  const text = (comment ?? '').trim();
  if (text === '' || !(spotKhz > 0) || (mode && WSJT_MODES.test(mode.trim()))) return null;
  const qsx = QSX.exec(text);
  if (qsx?.[1]) {
    const tx = qsxKhz(qsx[1], spotKhz);
    if (tx !== null) return qsxSplit(tx, spotKhz);
  }
  // prima «UP 5», poi «5 UP»: vale il primo che e' davvero uno spostamento (un locatore come «DN70» non lo e')
  const found = [
    ...Array.from(text.matchAll(AFTER), (m) => (m[1] && m[2] ? fromMatch(m[1], m[2], m[3], spotKhz) : null)),
    ...Array.from(text.matchAll(BEFORE), (m) => (m[3] && m[1] ? fromMatch(m[3], m[1], m[2], spotKhz) : null)),
  ];
  const valid = found.find((f) => f !== null);
  if (valid) return valid;
  // un numero accanto a UP/DN che non e' uno spostamento (oltre 50 kHz, «599 UP»): non si indovina
  if (found.length > 0) return null;
  const bare = BARE.exec(text);
  if (bare?.[1]) {
    const down = !bare[1].toUpperCase().startsWith('U');
    return { kind: down ? 'down' : 'up', offsetKhz: down ? -1 : 1, txKhz: round3(spotKhz + (down ? -1 : 1)), label: down ? 'DN 1' : 'UP 1' };
  }
  return null;
}

/** L'interruttore «Split DX» (Impostazioni > DX Cluster), acceso di fabbrica; vale per la pagina Stazione e per la PSK · RTTY · CW. */
const KEY = 'aurora.cluster.splitdx';
const listeners = new Set<(on: boolean) => void>();

export function readSplitDxEnabled(): boolean {
  try {
    return browserStorage()?.getItem(KEY) !== 'off';
  } catch {
    return true;
  }
}

export function setSplitDxEnabled(on: boolean): void {
  try {
    browserStorage()?.setItem(KEY, on ? 'on' : 'off');
  } catch {
    // senza archivio del browser vale solo per questa sessione
  }
  listeners.forEach((l) => l(on));
}

/** L'interruttore di adesso, che si aggiorna quando cambia (anche da un'altra finestra). */
export function useSplitDxEnabled(): boolean {
  const [on, setOn] = useState(readSplitDxEnabled);
  useEffect(() => {
    const sync = () => setOn(readSplitDxEnabled());
    listeners.add(setOn);
    window.addEventListener('storage', sync);
    return () => {
      listeners.delete(setOn);
      window.removeEventListener('storage', sync);
    };
  }, []);
  return on;
}
