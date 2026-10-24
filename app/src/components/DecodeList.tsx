import type { CSSProperties } from 'react';
import type { DecodedMessage, DecodeTag } from '../api/schemas';
import { formatClock } from '../lib/format';
import { distanceKm, gridCenter, gridOfMessage } from '../lib/geo';

const BADGES: Partial<Record<DecodeTag, { label: string; color: string }>> = {
  new_dxcc: { label: 'NUOVO DXCC', color: 'new-dxcc' },
  new_dxcc_band: { label: 'DXCC NUOVA BANDA', color: 'new-dxcc-band' },
  new_zone: { label: 'NUOVA ZONA', color: 'new-zone' },
  new_grid: { label: 'NUOVA GRIGLIA', color: 'new-grid' },
  auto: { label: 'AUTO', color: 'auto' },
};

/** Colore della barra accanto al messaggio: l'etichetta se c'e', altrimenti quello dei CQ. */
function accentOf(tag: DecodeTag | null | undefined): string | null {
  if (tag === 'cq') return 'cq';
  return tag ? (BADGES[tag]?.color ?? null) : null;
}

function distanceOf(mine: { lat: number; lon: number }, text: string): number | null {
  const grid = gridOfMessage(text);
  const theirs = grid ? gridCenter(grid) : null;
  return theirs ? distanceKm(mine, theirs) : null;
}

interface Props {
  decodes: DecodedMessage[];
  onReply(decodeId: number): void;
  /** Il proprio locatore, per mostrare la distanza di ogni stazione. */
  myGrid?: string;
  /** Testo dello stato vuoto (predefinito: invito all'ascolto). */
  emptyText?: string;
}

export function DecodeList({ decodes, onReply, myGrid, emptyText = 'In ascolto: i messaggi compaiono qui' }: Props) {
  const mine = myGrid ? gridCenter(myGrid) : null;
  if (decodes.length === 0) {
    return (
      <section className="decode-list decode-list--empty" aria-label="Messaggi decodificati">
        <p>{emptyText}</p>
      </section>
    );
  }
  return (
    <section className="decode-list" aria-label="Messaggi decodificati">
      {decodes.map((d) => {
        const badge = d.tag ? BADGES[d.tag] : undefined;
        return (
          <button
            type="button"
            key={d.id}
            className="decode-row"
            style={accentOf(d.tag) ? ({ '--row-accent': `var(--tag-${accentOf(d.tag)})` } as CSSProperties) : undefined}
            onClick={() => onReply(d.id)}
          >
            <span className="num muted">{formatClock(d.time_ms)}</span>
            <span className="num">{d.snr}</span>
            <span className="num muted">{d.delta_time.toFixed(1)}</span>
            <span className="num muted">{d.freq_hz}</span>
            <span className="decode-text">
              <span className="data">{d.text}</span>
            </span>
            <span className="decode-where">
              {d.country ? <span className="decode-country">{d.country}</span> : null}
              {mine && distanceOf(mine, d.text) !== null ? <span className="num muted"> [{distanceOf(mine, d.text)} km]</span> : null}
            </span>
            {badge ? (
              <span
                className="badge badge--tag"
                style={{ background: `var(--tag-${badge.color})`, color: `var(--tag-${badge.color}-ink)` }}
              >
                {badge.label}
              </span>
            ) : (
              <span />
            )}
          </button>
        );
      })}
    </section>
  );
}
