import { memo, useEffect, useSyncExternalStore, type RefObject } from 'react';
import type React from 'react';
import { getDigiBase, getDigiText, getDigiWeak, onDigiText } from '../lib/digiBus';

/**
 * Il testo della finestra di ricezione dei modi testuali: le righe trasmesse (iniziano con «» ») sottolineate se richiesto, i caratteri
 * incerti (RTTY: probabilita' sotto 0,9, `weak` con '1' alla stessa posizione) in chiaro.
 * `base` e' la posizione assoluta del primo carattere (vedi `getDigiBase`): ogni riga ha per chiave la sua posizione assoluta, cosi' a ogni
 * carattere nuovo si ridisegna solo l'ultima riga e non tutte le centinaia di righe della finestra.
 */
export function DigiRxText({ text, weak, underlineSent, showSent = true, base = 0 }: { text: string; weak: string; underlineSent: boolean; /** Spento: le righe trasmesse non si mostrano (restano nella finestra di trasmissione). */ showSent?: boolean; base?: number }): React.ReactNode {
  let offset = 0;
  return text.split('\n').map((line, i, all) => {
    const start = offset;
    offset += line.length + 1;
    if (!showSent && line.startsWith('»')) return null;
    return <RxLine key={base + start} line={line} mask={weak.slice(start, start + line.length)} newline={i < all.length - 1} underline={underlineSent && line.startsWith('» ')} />;
  });
}

/** Una riga della finestra: si ridisegna solo se cambia il suo testo, i suoi caratteri incerti o il suo stile. */
const RxLine = memo(function RxLine({ line, mask, newline, underline }: { line: string; mask: string; newline: boolean; underline: boolean }) {
  const body = withWeak(line, mask);
  const nl = newline ? '\n' : '';
  return underline ? (
    <u className="digi-sent">
      {body}
      {nl}
    </u>
  ) : (
    <span>
      {body}
      {nl}
    </span>
  );
});

/**
 * La finestra di ricezione collegata al testo dei modi testuali: si iscrive da sola, cosi' a ogni carattere ricevuto si ridisegna solo lei e
 * non tutta la pagina; a ogni carattere nuovo `box` scorre in fondo.
 */
export function DigiRxLive({ box, underlineSent, showSent }: { box: RefObject<HTMLElement | null>; underlineSent: boolean; showSent: boolean }): React.ReactNode {
  const text = useSyncExternalStore(onDigiText, getDigiText);
  const weak = useSyncExternalStore(onDigiText, getDigiWeak);
  const base = useSyncExternalStore(onDigiText, getDigiBase);
  useEffect(() => {
    const el = box.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [text, box]);
  return text === '' ? null : <DigiRxText text={text} weak={weak} base={base} underlineSent={underlineSent} showSent={showSent} />;
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
