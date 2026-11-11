import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { getFrequencies, type FrequencyEntry } from '../api/client';
import { useT } from '../i18n';
import { bandOf } from '../lib/format';

interface Props {
  dialHz: number;
  mode: string;
  /** Va sulla frequenza scelta; senza, il selettore non si mostra. */
  onPick?(hz: number): void;
  locked?: boolean;
  /** Solo queste bande (quelle scelte per la barra); senza, tutte. */
  bands?: readonly string[];
  /** Band Hopping: col tasto destro sul pulsante si passa alla frequenza successiva del giro (come facevano i pulsanti delle bande). */
  onHop?(): void;
  /** Righe proprie al posto della tabella del motore (piano di banda dei modi testuali e dell'SSTV). */
  rows?: readonly FrequencyEntry[];
}

/** Frequenza in MHz con tre decimali (quattro se serve: 7,0475). */
const mhz = (hz: number) => (hz / 1e6).toFixed(hz % 1000 === 0 ? 3 : 4);

/**
 * Come in WSJT-X, tutte le frequenze del modo in uso dalla tabella delle Frequenze: un pulsante con la frequenza attuale apre un pannello
 * con una riga per banda; la frequenza standard e' piena, le alternative (anche DXpedition) piu' tenui.
 */
export function FreqPicker({ dialHz, mode, onPick, locked = false, bands: shown, onHop, rows: own }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const table = useQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, staleTime: 60_000, retry: false });

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', esc);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', esc);
    };
  }, [open]);

  // la trasmissione comincia con il menu aperto: si chiude
  useEffect(() => {
    if (locked) setOpen(false);
  }, [locked]);

  const m = mode.toUpperCase().split('+')[0];
  const rows = (own ?? table.data ?? []).filter((r) => r.mode.toUpperCase() === m && (!shown || shown.includes(r.band))).sort((a, b) => a.hz - b.hz);
  if (!onPick || rows.length === 0) return null;
  const bands = [...new Set(rows.map((r) => r.band))];
  const current = bandOf(dialHz);

  return (
    <div className="freq-picker" ref={box}>
      <button
        type="button"
        className="glass freq-picker-btn fp-chip"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t('band.freqs')}
        title={onHop ? `${t('tip.band.freqs')} ${t('tip.band.hop')}` : t('tip.band.freqs')}
        // in trasmissione la frequenza non si cambia, ma il pulsante non si spegne di colore (scelta del proprietario): si ignora solo il clic
        aria-disabled={locked || undefined}
        onClick={() => {
          if (!locked) setOpen((o) => !o);
        }}
        onContextMenu={(e) => {
          if (!onHop || locked) return;
          e.preventDefault();
          setOpen(false);
          onHop();
        }}
      >
        {current ? <span className="freq-picker-btn-band">{current}</span> : null}
        <span className="freq-picker-btn-mhz">{dialHz > 0 ? mhz(dialHz) : '—'}</span>
        <span className="freq-picker-btn-unit">MHz</span>
        <span className="mode-switch-caret" aria-hidden="true">
          ▾
        </span>
      </button>
      {open && (
        <div className="freq-picker-menu glass" role="menu" aria-label={t('band.freqs')}>
          <div className="freq-picker-head">
            <span className="freq-picker-title">
              {t('band.freqs')} · {m}
            </span>
            <span className="freq-picker-legend">
              <i className="freq-picker-dot" aria-hidden="true" /> {t('band.main')}
            </span>
          </div>
          {bands.map((b) => (
            <div key={b} className={`freq-picker-row${b === current ? ' is-current' : ''}`}>
              {(() => {
                // il nome della banda porta sulla sua frequenza principale (la prima «principale», altrimenti la prima della riga)
                const main = rows.find((r) => r.band === b && r.preferred) ?? rows.find((r) => r.band === b);
                return (
                  <button
                    type="button"
                    className="freq-picker-band"
                    title={t('band.tipMain')}
                    onClick={() => {
                      setOpen(false);
                      if (!locked && main && main.hz !== dialHz) onPick(main.hz);
                    }}
                  >
                    {b}
                  </button>
                );
              })()}
              <span className="freq-picker-list">
                {rows
                  .filter((r) => r.band === b)
                  .map((r) => (
                    <button
                      key={`${r.hz}-${r.region}`}
                      type="button"
                      role="menuitemradio"
                      aria-checked={r.hz === dialHz}
                      className={`fp-chip ${r.preferred ? 'is-main' : 'is-alt'}${r.hz === dialHz ? ' is-on' : ''}`}
                      title={`${r.preferred ? t('band.tipMain') : t('band.tipAlt')}${r.region !== 'ALL' ? ` · ${t('band.tipRegion', { r: r.region.slice(1) })}` : ''}`}
                      onClick={() => {
                        setOpen(false);
                        if (!locked && r.hz !== dialHz) onPick(r.hz);
                      }}
                    >
                      {mhz(r.hz)}
                      {r.region !== 'ALL' ? <sup>{r.region}</sup> : null}
                    </button>
                  ))}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
