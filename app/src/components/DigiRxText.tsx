import type React from 'react';

/**
 * Il testo della finestra di ricezione dei modi testuali: le righe trasmesse (iniziano con «» ») sottolineate se richiesto, i caratteri
 * incerti (RTTY: probabilita' sotto 0,9, `weak` con '1' alla stessa posizione) in chiaro.
 */
export function DigiRxText({ text, weak, underlineSent }: { text: string; weak: string; underlineSent: boolean }): React.ReactNode {
  let offset = 0;
  return text.split('\n').map((line, i, all) => {
    const nl = i < all.length - 1 ? '\n' : '';
    const body = withWeak(line, weak.slice(offset, offset + line.length));
    offset += line.length + 1;
    return underlineSent && line.startsWith('» ') ? (
      <u key={i} className="digi-sent">
        {body}
        {nl}
      </u>
    ) : (
      <span key={i}>
        {body}
        {nl}
      </span>
    );
  });
}

/** Una riga divisa in tratti certi e incerti. */
function withWeak(line: string, mask: string): React.ReactNode {
  if (!mask.includes('1')) return line;
  const parts: React.ReactNode[] = [];
  let start = 0;
  for (let k = 1; k <= line.length; k++) {
    if (k === line.length || mask[k] !== mask[start]) {
      const piece = line.slice(start, k);
      parts.push(mask[start] === '1' ? <span key={start} className="digi-weak">{piece}</span> : piece);
      start = k;
    }
  }
  return parts;
}
