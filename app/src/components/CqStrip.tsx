import { useEffect, useMemo, useRef, useState } from 'react';
import type { DecodedMessage, DecodeTag } from '../api/schemas';
import { useT } from '../i18n';
import { flagCode } from '../lib/flags';
import { senderOf } from '../lib/wishlist';
import { utcTime } from './NewDxAlert';

interface Props {
  decodes: DecodedMessage[];
  onReply(decodeId: number): void;
  onQueue?: (id: number) => void;
  emptyText?: string;
}

/** Il colore (variabile --tag-*) di ogni tipo di messaggio che si evidenzia. */
const ACCENT: Partial<Record<DecodeTag, string>> = {
  cq: 'cq',
  new_dxcc: 'new-dxcc',
  new_dxcc_band: 'new-dxcc-band',
  new_zone: 'new-zone',
  new_zone_band: 'new-zone-band',
  new_itu: 'new-itu',
  new_itu_band: 'new-itu-band',
  new_grid: 'new-grid',
  new_grid_band: 'new-grid-band',
  new_px: 'new-px',
  new_px_band: 'new-px-band',
  new_call: 'new-call',
  new_call_band: 'new-call-band',
  worked_call: 'worked-call',
};

/**
 * «CQ e novita'» in orizzontale: una stazione per casella, al massimo due righe; se le stazioni sono piu' di quelle che entrano la fascia
 * scorre da sola (avanti e indietro, si ferma col mouse sopra). Una stazione compare una volta sola, col suo messaggio piu' recente.
 * Doppio clic: risponde; tasto destro: in coda.
 */
/** Una stazione che non si sente piu' da tanto esce da sola dalla fascia (richiesta del proprietario, 2026-11-10). */
const SILENT_MS = 15 * 60_000;

/** Da quanti millisecondi e' arrivato un messaggio (ora in ms dalla mezzanotte UTC), anche a cavallo della mezzanotte. */
function ageMs(timeMs: number, now: number): number {
  return ((((now % 86_400_000) - timeMs) % 86_400_000) + 86_400_000) % 86_400_000;
}

export function CqStrip({ decodes, onReply, onQueue, emptyText }: Props) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);
  // l'elenco si ricontrolla ogni 30 s anche senza decodifiche nuove, cosi' chi tace da 15 minuti sparisce
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(id);
  }, []);
  const items = useMemo(() => {
    const seen = new Set<string>();
    const out: DecodedMessage[] = [];
    for (const d of decodes) {
      if (d.id < 0 || ageMs(d.time_ms, now) > SILENT_MS) continue;
      const key = senderOf(d.text)?.call.toUpperCase() ?? `#${d.id}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(d);
    }
    return out;
  }, [decodes, now]);

  // scorrimento automatico: solo se c'e' qualcosa fuori vista; avanti e indietro, ~28 px al secondo
  useEffect(() => {
    const el = box.current;
    if (!el || window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return;
    let raf = 0;
    let last = 0;
    let dir = 1;
    let pos = el.scrollLeft;
    let hold = 0;
    // 20 passi al secondo bastano per uno scorrimento lento e morbido (prima 60, sempre, anche a finestra nascosta)
    const tick = () => {
      raf = window.setTimeout(tick, 50);
      if (document.hidden) return;
      const now = performance.now();
      const dt = last ? Math.min(0.1, (now - last) / 1000) : 0;
      last = now;
      const max = el.scrollWidth - el.clientWidth;
      if (max <= 2 || el.matches(':hover')) {
        pos = el.scrollLeft;
        return;
      }
      if (hold > 0) {
        hold -= dt;
        return;
      }
      pos += dir * 28 * dt;
      if (pos >= max) {
        pos = max;
        dir = -1;
        hold = 1.5;
      } else if (pos <= 0) {
        pos = 0;
        dir = 1;
        hold = 1.5;
      }
      el.scrollLeft = pos;
    };
    raf = window.setTimeout(tick, 50);
    return () => window.clearTimeout(raf);
  }, []);

  if (items.length === 0) return <section className="cq-strip cq-strip--empty"><p>{emptyText ?? t('decode.noCq')}</p></section>;
  return (
    <div className="cq-strip" ref={box} aria-label={t('panel.cq')}>
      {items.map((d) => {
        const accent = d.tag ? ACCENT[d.tag] : undefined;
        const code = flagCode(d.country);
        const talker = senderOf(d.text);
        return (
          <button
            key={d.id}
            type="button"
            className={`cq-chip${d.tag && d.tag !== 'cq' ? ' cq-chip--new' : ''}${d.tag === 'worked_call' ? ' cq-chip--worked' : ''}${d.tag === 'new_dxcc' || d.tag === 'new_dxcc_band' ? ' cq-chip--flash' : ''}`}
            style={accent ? ({ '--row-accent': `var(--tag-${accent})` } as React.CSSProperties) : undefined}
            // solo nominativo, bandierina e paese (scelta del proprietario); il messaggio intero, il rapporto e l'ora nel suggerimento
            title={`${d.text} · ${d.snr} dB · ${utcTime(d.time_ms)} UTC${d.country ? ` · ${d.country}` : ''}`}
            onDoubleClick={() => onReply(d.id)}
            onContextMenu={(e) => {
              if (!talker || !onQueue) return;
              e.preventDefault();
              onQueue(d.id);
            }}
          >
            <span className="cq-chip-text">{talker?.call ?? d.text}</span>
            <span className="cq-chip-flag">{code ? <span className={`fi fi-${code}`} aria-hidden="true" /> : null}</span>
            {d.country ? <span className="cq-chip-country">{d.country}</span> : null}
          </button>
        );
      })}
    </div>
  );
}
