import type { CSSProperties, ReactNode } from 'react';
import type { DecodedMessage, DecodeTag } from '../api/schemas';
import { formatClock } from '../lib/format';
import { flagCode } from '../lib/flags';
import { distanceKm, gridCenter, gridOfMessage } from '../lib/geo';

const BADGES: Partial<Record<DecodeTag, { label: string; color: string }>> = {
  new_dxcc: { label: 'NUOVO DXCC', color: 'new-dxcc' },
  new_dxcc_band: { label: 'DXCC NUOVA BANDA', color: 'new-dxcc-band' },
  new_zone: { label: 'NUOVA ZONA', color: 'new-zone' },
  new_zone_band: { label: 'ZONA NUOVA BANDA', color: 'new-zone-band' },
  new_itu: { label: 'NUOVA ITU', color: 'new-itu' },
  new_itu_band: { label: 'ITU NUOVA BANDA', color: 'new-itu-band' },
  new_grid: { label: 'NUOVA GRIGLIA', color: 'new-grid' },
  new_grid_band: { label: 'GRIGLIA NUOVA BANDA', color: 'new-grid-band' },
  new_px: { label: 'NUOVO PREFISSO', color: 'new-px' },
  new_px_band: { label: 'PREFISSO NUOVA BANDA', color: 'new-px-band' },
  new_call: { label: 'NUOVA STAZIONE', color: 'new-call' },
  new_call_band: { label: 'STAZIONE NUOVA BANDA', color: 'new-call-band' },
  worked_call: { label: 'GIÀ LAVORATA', color: 'worked-call' },
  auto: { label: 'AUTO', color: 'auto' },
};

/** Colore della barra accanto al messaggio: quello dell'etichetta, o dei CQ. */
function accentOf(tag: DecodeTag | null | undefined): string | null {
  if (tag === 'cq') return 'cq';
  return tag ? (BADGES[tag]?.color ?? null) : null;
}

function namesCall(text: string, call: string): boolean {
  const wanted = call.trim().toUpperCase();
  return wanted !== '' && text.toUpperCase().split(/\s+/).includes(wanted);
}

/** Il testo del messaggio, con il nominativo chiamato in evidenza. */
function highlightCall(text: string, call: string): ReactNode {
  const wanted = call.trim().toUpperCase();
  if (wanted === '') return text;
  return text.split(/(\s+)/).map((part, i) =>
    part.toUpperCase() === wanted ? (
      <mark key={i} className="dx-call">
        {part}
      </mark>
    ) : (
      part
    ),
  );
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
  /** Stazione che si sta chiamando: la sua riga e il suo nominativo si evidenziano. */
  dxCall?: string | null;
  /** Testo dello stato vuoto (predefinito: invito all'ascolto). */
  emptyText?: string;
}

export function DecodeList({ decodes, onReply, myGrid, dxCall, emptyText = 'In ascolto: i messaggi compaiono qui' }: Props) {
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
            className={`decode-row${dxCall && namesCall(d.text, dxCall) ? ' decode-row--dx' : ''}`}
            style={accentOf(d.tag) ? ({ '--row-accent': `var(--tag-${accentOf(d.tag)})` } as CSSProperties) : undefined}
            onClick={() => onReply(d.id)}
          >
            <span className="num muted">{formatClock(d.time_ms)}</span>
            <span className="num">{d.snr}</span>
            <span className="num muted">{d.delta_time.toFixed(1)}</span>
            <span className="num muted">{d.freq_hz}</span>
            <span className="decode-text">
              <span className="data">{dxCall ? highlightCall(d.text, dxCall) : d.text}</span>
            </span>
            <span className="decode-where">
              {flagCode(d.country) ? <span className={`fi fi-${flagCode(d.country)} decode-flag`} aria-hidden="true" /> : null}
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
