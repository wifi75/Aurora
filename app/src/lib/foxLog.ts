import type { FoxLogEntry } from '../api/schemas';

/** Data e ora UTC di un QSO: `AAAAMMGG`, `HHMMSS` e `HHMM`. */
function utc(ms: number) {
  const d = new Date(ms);
  const p = (n: number, w = 2) => String(n).padStart(w, '0');
  return {
    date: `${d.getUTCFullYear()}${p(d.getUTCMonth() + 1)}${p(d.getUTCDate())}`,
    dateDash: `${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())}`,
    time: `${p(d.getUTCHours())}${p(d.getUTCMinutes())}${p(d.getUTCSeconds())}`,
    hhmm: `${p(d.getUTCHours())}${p(d.getUTCMinutes())}`,
  };
}

const field = (name: string, value: string) => (value ? `<${name}:${new TextEncoder().encode(value).length}>${value} ` : '');

/** Il Fox log come file ADIF 3 (un record per QSO, come `FoxLogWindow` di WSJT-X). */
export function foxLogAdif(log: FoxLogEntry[], myCall: string, myGrid: string): string {
  const head = `Aurora Fox log\n<adif_ver:5>3.1.4 <programid:6>Aurora <eoh>\n`;
  const rows = log.map((q) => {
    const t = utc(q.time_ms);
    return (
      field('call', q.call) +
      field('gridsquare', q.grid) +
      field('mode', q.mode) +
      field('rst_sent', q.sent) +
      field('rst_rcvd', q.rcvd) +
      field('qso_date', t.date) +
      field('time_on', t.time) +
      field('qso_date_off', t.date) +
      field('time_off', t.time) +
      field('band', q.band.toLowerCase()) +
      field('freq', (q.freq_hz / 1e6).toFixed(6)) +
      field('station_callsign', myCall) +
      field('my_gridsquare', myGrid) +
      '<eor>'
    );
  });
  return `${head}${rows.join('\n')}\n`;
}

/** Il Fox log in Cabrillo 3.0 (righe QSO con frequenza in kHz, modo DG, rapporti come scambio). */
export function foxLogCabrillo(log: FoxLogEntry[], myCall: string): string {
  const lines = ['START-OF-LOG: 3.0', `CALLSIGN: ${myCall}`, 'CONTEST: DXPEDITION', 'CATEGORY-MODE: DIGI', 'CREATED-BY: Aurora'];
  for (const q of log) {
    const t = utc(q.time_ms);
    const khz = String(Math.round(q.freq_hz / 1000)).padStart(5, ' ');
    lines.push(`QSO: ${khz} DG ${t.dateDash} ${t.hhmm} ${myCall.padEnd(13)} ${q.sent.padEnd(4)} ${q.call.padEnd(13)} ${q.rcvd.padEnd(4)}`.trimEnd());
  }
  lines.push('END-OF-LOG:');
  return `${lines.join('\r\n')}\r\n`;
}

/** Salva un testo come file scaricato dal browser. */
export function saveText(name: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' }));
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** QSO all'ora (sugli ultimi `n`, dal primo all'ultimo) e quanti nell'ultima ora prima di `now`. */
export function foxRates(log: FoxLogEntry[], now: number, n = 10): { rate: number; lastHour: number; buckets: number[] } {
  const last = log.slice(-n);
  let rate = 0;
  const first = last.at(0);
  const latest = last.at(-1);
  if (first && latest && last.length >= 2) {
    const span = latest.time_ms - first.time_ms;
    rate = span > 0 ? Math.round(((last.length - 1) * 3_600_000) / span) : 0;
  }
  const lastHour = log.filter((q) => now - q.time_ms <= 3_600_000).length;
  // dodici barre da 5 minuti per l'ultima ora, la piu' recente a destra
  const buckets = Array.from({ length: 12 }, () => 0);
  for (const q of log) {
    const age = now - q.time_ms;
    const i = 11 - Math.floor(age / 300_000);
    if (age >= 0 && age < 3_600_000) buckets[i] = (buckets[i] ?? 0) + 1;
  }
  return { rate, lastHour, buckets };
}

/** L'ordine dei continenti nei numeri della Fox (sigle di `cty.dat`). */
const CONTINENTS = ['EU', 'NA', 'AS', 'SA', 'AF', 'OC', 'AN'];

/** Quanti QSO del Fox log per continente, nell'ordine fisso; i continenti senza QSO non ci sono, quelli ignoti finiscono sotto «?». */
export function foxContinents(log: FoxLogEntry[]): [string, number][] {
  const count = new Map<string, number>();
  for (const q of log) {
    const c = q.continent && CONTINENTS.includes(q.continent) ? q.continent : '?';
    count.set(c, (count.get(c) ?? 0) + 1);
  }
  return [...CONTINENTS, '?'].flatMap((c): [string, number][] => {
    const n = count.get(c);
    return n ? [[c, n]] : [];
  });
}
