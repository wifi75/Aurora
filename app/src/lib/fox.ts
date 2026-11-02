/**
 * Riconoscimento delle stazioni in modo Fox/Hound di WSJT-X, come `fox_sender` del motore: il Fox manda il messaggio DXpedition a due Hound
 * `K1ABC RR73; W9XYZ <KH1/KH7Z> -08` (`i3=0 n3=1`). Le risposte multiple normali (MSHV) sono messaggi standard separati e non contano.
 */
export function foxSender(text: string): string | null {
  const [first, after] = text.trim().toUpperCase().split('; ');
  if (first === undefined || after === undefined) return null;
  const a = first.split(/\s+/);
  const b = after.split(/\s+/);
  if (a.length !== 2 || a[1] !== 'RR73' || b.length !== 3) return null;
  const fox = b[1]!.replace(/^</, '').replace(/>$/, '');
  return fox === '' || fox === '...' ? null : fox;
}

/** Una Fox vera trasmette sotto i 1000 Hz (gli Hound sopra): un messaggio doppio piu' in alto e' una risposta multipla di una stazione normale. */
export const FOX_MAX_HZ = 1000;

/** I nominativi che nei messaggi ricevuti trasmettono come Fox (messaggio DXpedition sotto i 1000 Hz, se la frequenza e' nota). */
export function foxCalls(items: Array<string | { text: string; freq_hz?: number }>): Set<string> {
  const out = new Set<string>();
  for (const item of items) {
    const text = typeof item === 'string' ? item : item.text;
    const hz = typeof item === 'string' ? undefined : item.freq_hz;
    if (hz !== undefined && hz >= FOX_MAX_HZ) continue;
    const f = foxSender(text);
    if (f) out.add(f);
  }
  return out;
}

/** Chi trasmette una riga: il Fox di una riga DXpedition, chi chiama CQ, il secondo nominativo di un messaggio diretto. */
export function lineSender(text: string): string | null {
  const fox = foxSender(text);
  if (fox) return fox;
  const w = text.trim().toUpperCase().split(/\s+/);
  if (w[0] === 'CQ') return (w.length > 2 && w[1]!.length <= 4 && !/\d/.test(w[1]!) ? w[2] : w[1])?.replace(/[<>]/g, '') ?? null;
  return w.length >= 2 ? w[1]!.replace(/[<>]/g, '') : null;
}
