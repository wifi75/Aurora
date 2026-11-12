import { useRef, type KeyboardEvent, type Ref } from 'react';
import type { TxProgress } from '../lib/digiTxLive';

/**
 * La finestra di trasmissione dei modi testuali. Mentre si trasmette, sotto il testo uno sfondo segna i caratteri gia' partiti via radio
 * (con una lineetta rossa al punto in cui e' arrivata la trasmissione) e sottolinea quelli in coda: lo sfondo e' una copia del testo con
 * le stesse misure, trasparente, che scorre con la casella.
 */
export function DigiTxBox({
  boxRef,
  className,
  value,
  progress,
  sending,
  label,
  placeholder,
  onText,
  onKeyDown,
}: {
  boxRef: Ref<HTMLTextAreaElement>;
  className: string;
  value: string;
  progress: TxProgress;
  sending: boolean;
  label: string;
  placeholder?: string;
  onText: (next: string) => void;
  onKeyDown: (e: KeyboardEvent<HTMLTextAreaElement>) => void;
}) {
  const back = useRef<HTMLDivElement>(null);
  const sent = Math.min(progress.sent, value.length);
  const queued = Math.max(sent, Math.min(progress.queued, value.length));
  const marked = sending || progress.queued > 0;
  return (
    <div className="digi-txstack">
      {marked ? (
        <div ref={back} className={`${className} digi-txback`} aria-hidden="true">
          <span className="digi-txsent">{value.slice(0, sent)}</span>
          <span className="digi-txpoint" />
          <span className="digi-txqueued">{value.slice(sent, queued)}</span>
          {value.slice(queued)}
          {'\n'}
        </div>
      ) : null}
      <textarea
        ref={boxRef}
        className={`${className} digi-txfront`}
        value={value}
        aria-label={label}
        placeholder={placeholder}
        onChange={(e) => onText(e.target.value)}
        onKeyDown={onKeyDown}
        onScroll={(e) => {
          if (back.current) back.current.scrollTop = e.currentTarget.scrollTop;
        }}
      />
    </div>
  );
}
