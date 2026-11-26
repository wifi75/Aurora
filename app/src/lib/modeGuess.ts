/**
 * Riconoscimento del modo di un segnale della cascata: solo un SUGGERIMENTO, mai un cambio di modo.
 *
 * Si guardano soltanto le righe di spettro che il client riceve gia' (byte = dB da `DB_MIN` a passo `DB_STEP`, canali da 0,73 Hz). Ogni
 * riga e' la FFT di una finestra di 1,365 s: la manipolazione del CW e la velocita' dei simboli non si vedono, si vedono la forma in
 * frequenza (larghezza occupata, numero e distanza delle righe) e l'andamento lento nel tempo (segnale continuo o a pause, livello
 * fermo o che varia). Quando le misure non bastano si dice «non riconosciuto» o si danno piu' candidati.
 */
import { DB_MIN, DB_STEP } from '../render/jtdxRow';
import type { SpectrumRow } from './digiSpectrum';

export type GuessKind = 'cw' | 'carrier' | 'rtty' | 'psk31' | 'psk63' | 'ft8' | 'ft4' | 'wide';

export interface ModeGuess {
  kind: GuessKind;
  /** `likely`: le misure lo indicano bene; `possible`: e' una delle spiegazioni. */
  confidence: 'likely' | 'possible';
  /** Centro del segnale (Hz audio). */
  centerHz: number;
  /** Per l'RTTY: distanza fra i due toni (Hz). */
  shiftHz?: number;
}

export interface ModeGuessResult {
  /** Dal piu' probabile; vuoto = non riconosciuto (o nessun segnale netto sotto il mouse). */
  guesses: ModeGuess[];
  /** Larghezza misurata a -10 dB dal picco (Hz); 0 se non c'e' segnale. */
  widthHz: number;
  /** Quanto il picco sta sopra il fondo (dB); 0 se non c'e' segnale. */
  peakDb: number;
}

/** Quanto deve stare sopra il fondo il picco vicino al mouse per parlare di un segnale (dB). */
const SIGNAL_DB = 8;
/** Distanza massima del picco dal punto cliccato (Hz). */
const REACH_HZ = 60;
/** Zona per il fondo (Hz per parte). */
const ZONE_HZ = 1500;
/** Spostamenti RTTY comuni (Hz). */
const RTTY_SHIFTS = [85, 170, 200, 425, 450, 850];
/** Storia minima perche' si possano giudicare le pause (s). */
const MIN_HISTORY_S = 12;

const toPow = (b: number) => 10 ** ((DB_MIN + b * DB_STEP) / 10);
const toDb = (p: number) => 10 * Math.log10(Math.max(p, 1e-30));

/** Il quantile `q` (0-1) dei valori. */
function quantile(v: number[], q: number): number {
  const s = [...v].sort((a, b) => a - b);
  return s[Math.floor(s.length * q)] ?? 0;
}

function correlation(a: number[], b: number[]): number | null {
  const n = Math.min(a.length, b.length);
  if (n < 4) return null;
  const ma = a.reduce((x, y) => x + y, 0) / n;
  const mb = b.reduce((x, y) => x + y, 0) / n;
  let sab = 0;
  let saa = 0;
  let sbb = 0;
  for (let i = 0; i < n; i++) {
    const da = (a[i] ?? 0) - ma;
    const db = (b[i] ?? 0) - mb;
    sab += da * db;
    saa += da * da;
    sbb += db * db;
  }
  // livelli quasi fermi (deviazione sotto 1 dB): la correlazione non dice niente
  if (Math.sqrt(saa / n) < 1 || Math.sqrt(sbb / n) < 1) return null;
  return sab / Math.sqrt(saa * sbb);
}

function std(v: number[]): number {
  if (v.length === 0) return 0;
  const m = v.reduce((x, y) => x + y, 0) / v.length;
  return Math.sqrt(v.reduce((x, y) => x + (y - m) ** 2, 0) / v.length);
}

/**
 * Il modo piu' probabile del segnale vicino a `hz`, dalle righe recenti (`rows`, dalla piu' vecchia; `secondsPerRow` fra una riga e la
 * successiva). Funzione pura: non cambia niente.
 */
export function guessMode(rows: SpectrumRow[], hz: number, secondsPerRow: number): ModeGuessResult {
  const none: ModeGuessResult = { guesses: [], widthHz: 0, peakDb: 0 };
  const last = rows[rows.length - 1];
  if (!last || last.bins.length < 16) return none;
  const n = last.bins.length;
  const use = rows.filter((r) => r.bins.length === n && r.fMinHz === last.fMinHz && r.fMaxHz === last.fMaxHz);
  const w = (last.fMaxHz - last.fMinHz) / n;
  const idx = (f: number) => Math.round((f - last.fMinHz) / w - 0.5);
  const hzOf = (i: number) => last.fMinHz + (i + 0.5) * w;
  const lo = Math.max(0, idx(hz - ZONE_HZ));
  const hi = Math.min(n - 1, idx(hz + ZONE_HZ));
  if (hi - lo < 16) return none;

  // spettro medio (in potenza) e fondo: il 20% piu' basso della zona (la mediana sale quando un segnale largo occupa meta' della zona)
  const spec = new Float64Array(n);
  for (const r of use) for (let i = lo; i <= hi; i++) spec[i] = (spec[i] ?? 0) + toPow(r.bins[i] ?? 0);
  const db = new Float64Array(n);
  for (let i = lo; i <= hi; i++) db[i] = toDb((spec[i] ?? 0) / use.length);
  const zone: number[] = [];
  for (let i = lo; i <= hi; i++) zone.push(db[i] ?? 0);
  const floor = quantile(zone, 0.2);
  const ex = (i: number) => (i < lo || i > hi ? 0 : (db[i] ?? 0) - floor);

  // il picco vicino al mouse
  let pk = -1;
  for (let i = Math.max(lo, idx(hz - REACH_HZ)); i <= Math.min(hi, idx(hz + REACH_HZ)); i++) if (pk < 0 || ex(i) > ex(pk)) pk = i;
  if (pk < 0 || ex(pk) < SIGNAL_DB) return none;
  const pkEx = ex(pk);

  // la macchia a -level dB dal picco di `at` (buchi fino a `gap` canali)
  const blob = (at: number, level: number, gap: number) => {
    const thr = Math.max(ex(at) - level, 4);
    const grow = (dir: 1 | -1) => {
      let end = at;
      let miss = 0;
      for (let i = at + dir; i >= lo && i <= hi; i += dir) {
        if (ex(i) >= thr) {
          end = i;
          miss = 0;
        } else if (++miss > gap) break;
      }
      return end;
    };
    return { a: grow(-1), b: grow(1) };
  };
  const width = (b: { a: number; b: number }) => (b.b - b.a + 1) * w;
  const isPeak = (i: number) => {
    const r = Math.max(1, Math.round(3 / w));
    for (let j = i - r; j <= i + r; j++) if (j !== i && ex(j) > ex(i)) return false;
    return true;
  };
  // il punto piu' alto entro `tol` Hz da `f`
  const bestNear = (f: number, tol: number) => {
    let b = -1;
    for (let i = Math.max(lo, idx(f - tol)); i <= Math.min(hi, idx(f + tol)); i++) if (b < 0 || ex(i) > ex(b)) b = i;
    return b;
  };
  // livello (dB sopra il fondo) di una banda in ogni riga
  const levels = (a: number, b: number) =>
    use.map((r) => {
      let p = 0;
      for (let i = a; i <= b; i++) p += toPow(r.bins[i] ?? 0);
      return toDb(p / (b - a + 1)) - floor;
    });
  const historyS = use.length * secondsPerRow;
  // a pause: dopo un tratto acceso, un tratto spento di almeno 1 s (servono abbastanza righe)
  const intermittent = (lv: number[]) => {
    if (historyS < MIN_HISTORY_S) return null;
    const need = Math.max(1, Math.round(1 / secondsPerRow));
    let seenOn = false;
    let off = 0;
    for (const v of lv) {
      if (v >= 4) {
        if (seenOn && off >= need) return true;
        seenOn = true;
        off = 0;
      } else if (seenOn) off++;
    }
    return seenOn && off >= need;
  };
  const one = (b: { a: number; b: number }) => Math.round(hzOf((b.a + b.b) / 2) * 10) / 10;
  const main = blob(pk, 10, 2);
  const mainW = width(main);
  const result = (guesses: ModeGuess[]): ModeGuessResult => ({ guesses, widthHz: Math.round(mainW), peakDb: Math.round(pkEx) });

  // due righe strette alla stessa altezza: PSK (inversioni a ±mezzo baud) o RTTY (mark e space). Due stazioni CW vicine danno righe con
  // livelli che non vanno insieme nel tempo: con la correlazione bassa la coppia non vale.
  const pair = (dist: number, tol: number, maxDiff: number) => {
    if (mainW > 60) return null;
    let best: { q: number; corr: number | null } | null = null;
    for (const sign of [-1, 1]) {
      const q = bestNear(hzOf(pk) + sign * dist, tol);
      if (q < 0 || !isPeak(q) || ex(q) < SIGNAL_DB || pkEx - ex(q) > maxDiff || width(blob(q, 6, 1)) > 60) continue;
      const [a, b] = q < pk ? [q, pk] : [pk, q];
      // in mezzo il segnale scende (il PSK in pausa e l'RTTY hanno un buco fra le righe)
      let mid = Infinity;
      for (let i = a + Math.max(1, Math.round(3 / w)); i <= b - Math.max(1, Math.round(3 / w)); i++) mid = Math.min(mid, ex(i));
      if (!(mid < Math.min(ex(a), ex(b)) - 6)) continue;
      const r = Math.max(1, Math.round(2 / w));
      const corr = correlation(levels(pk - r, pk + r), levels(q - r, q + r));
      if (corr !== null && corr < 0.3) continue;
      if (!best || ex(q) > ex(best.q)) best = { q, corr };
    }
    return best;
  };
  for (const [kind, dist] of [['psk31', 31.25], ['psk63', 62.5]] as const) {
    const p = pair(dist, Math.max(2, 2 * w), 6);
    if (p) {
      const centerHz = Math.round(((hzOf(pk) + hzOf(p.q)) / 2) * 10) / 10;
      return result([{ kind, confidence: p.corr !== null && p.corr > 0.6 ? 'likely' : 'possible', centerHz }]);
    }
  }
  for (const s of RTTY_SHIFTS) {
    const p = pair(s, Math.max(4, s * 0.05), 10);
    if (p) {
      const centerHz = Math.round(((hzOf(pk) + hzOf(p.q)) / 2) * 10) / 10;
      const shiftHz = Math.round(Math.abs(hzOf(p.q) - hzOf(pk)));
      return result([{ kind: 'rtty', confidence: p.corr !== null && p.corr > 0.6 ? 'likely' : 'possible', centerHz, shiftHz }]);
    }
  }

  const centerHz = one(main);
  const lv = levels(main.a, main.b);
  const pauses = intermittent(lv);
  if (mainW <= 15) {
    // una riga stretta: CW se il livello si muove (parole e pause) o va a pause, altrimenti una portante ferma
    const on = lv.filter((v) => v >= 4);
    const moving = pauses === true || std(on) >= 2;
    return result(moving ? [{ kind: 'cw', confidence: 'likely', centerHz }] : [{ kind: 'carrier', confidence: 'likely', centerHz }, { kind: 'cw', confidence: 'possible', centerHz }]);
  }
  if (mainW >= 25 && mainW <= 70) {
    // un blocco di circa 50 Hz: FT8 se va a pause (i periodi di 15 s), altrimenti puo' essere anche un PSK31 che scrive
    return result(
      pauses === true
        ? [{ kind: 'ft8', confidence: 'likely', centerHz }, { kind: 'psk31', confidence: 'possible', centerHz }]
        : [{ kind: 'ft8', confidence: 'possible', centerHz }, { kind: 'psk31', confidence: 'possible', centerHz }],
    );
  }
  if (mainW > 70 && mainW <= 120) {
    return result(
      pauses === true
        ? [{ kind: 'ft4', confidence: 'likely', centerHz }, { kind: 'psk63', confidence: 'possible', centerHz }]
        : [{ kind: 'ft4', confidence: 'possible', centerHz }, { kind: 'psk63', confidence: 'possible', centerHz }],
    );
  }
  // piu' largo: voce SSB, un modo digitale largo (Olivia, MFSK...) o piu' segnali vicini; il fondo non si misura bene quando il segnale
  // occupa quasi tutta la banda audio, quindi non si va oltre «largo»
  if (mainW > 120) return result([{ kind: 'wide', confidence: 'possible', centerHz }]);
  // fra 15 e 25 Hz: non si sa
  return result([]);
}
