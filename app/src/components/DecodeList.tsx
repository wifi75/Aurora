import { useState, type CSSProperties, type ReactNode } from 'react';
import type { DecodedMessage, DecodeTag } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { flagCode } from '../lib/flags';
import { formatClock } from '../lib/format';
import { distanceKm, gridCenter, gridOfMessage } from '../lib/geo';
import { useNotify } from '../lib/notify';
import { isCallToMe, isClosingMessage } from '../lib/mycall';
import { formatDistance, usePrefs } from '../lib/prefs';

/** Etichetta e colore di ogni tipo di messaggio; il CQ semplice ha solo il colore. */
const BADGES: Partial<Record<DecodeTag, { label: Key; color: string }>> = {
  new_dxcc: { label: 'tag.new_dxcc', color: 'new-dxcc' },
  new_dxcc_band: { label: 'tag.new_dxcc_band', color: 'new-dxcc-band' },
  new_zone: { label: 'tag.new_zone', color: 'new-zone' },
  new_zone_band: { label: 'tag.new_zone_band', color: 'new-zone-band' },
  new_itu: { label: 'tag.new_itu', color: 'new-itu' },
  new_itu_band: { label: 'tag.new_itu_band', color: 'new-itu-band' },
  new_grid: { label: 'tag.new_grid', color: 'new-grid' },
  new_grid_band: { label: 'tag.new_grid_band', color: 'new-grid-band' },
  new_px: { label: 'tag.new_px', color: 'new-px' },
  new_px_band: { label: 'tag.new_px_band', color: 'new-px-band' },
  new_call: { label: 'tag.new_call', color: 'new-call' },
  new_call_band: { label: 'tag.new_call_band', color: 'new-call-band' },
  worked_call: { label: 'tag.worked_call', color: 'worked-call' },
  auto: { label: 'tag.auto', color: 'auto' },
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
  /** Il proprio nominativo: i messaggi rivolti a me si evidenziano forte, come nel vecchio programma. */
  myCall?: string | null;
  /** Stazione che si sta chiamando: la sua riga e il suo nominativo si evidenziano. */
  dxCall?: string | null;
  /** Testo dello stato vuoto (predefinito: invito all'ascolto). */
  emptyText?: string;
}

export function DecodeList({ decodes, onReply, myGrid, myCall, dxCall, emptyText }: Props) {
  const t = useT();
  const prefs = usePrefs((s) => s.prefs);
  const notify = useNotify((s) => s.notify);
  const mine = myGrid ? gridCenter(myGrid) : null;
  // un clic seleziona la riga; trasmettere richiede il doppio clic (come nel motore)
  const [selected, setSelected] = useState<number | null>(null);
  if (decodes.length === 0) {
    return (
      <section className="decode-list decode-list--empty" aria-label={t('decode.aria')}>
        <p>{emptyText ?? t('decode.empty')}</p>
      </section>
    );
  }
  return (
    <section className={`decode-list${prefs.showDt ? '' : ' decode-list--no-dt'}`} aria-label={t('decode.aria')} title={t('tip.panel.all')}>
      {decodes.map((d) => {
        const own = d.id < 0;
        // un tipo con l'evidenziazione spenta si mostra come un messaggio qualunque
        const lit = !own && d.tag && notify.highlight[d.tag] ? d.tag : null;
        const badge = lit ? BADGES[lit] : undefined;
        const accent = accentOf(lit);
        const km = prefs.showDistance && mine ? distanceOf(mine, d.text) : null;
        const code = prefs.showFlag ? flagCode(d.country) : null;
        const toMe = !own && isCallToMe(d.text, myCall);
        return (
          <button
            type="button"
            key={d.id}
            title={own ? t('tip.row.tx') : toMe ? t('tip.row.me') : undefined}
            className={`decode-row${own ? ' decode-row--tx' : ''}${toMe ? ' decode-row--me' : ''}${dxCall && namesCall(d.text, dxCall) ? ' decode-row--dx' : ''}${selected === d.id ? ' decode-row--selected' : ''}`}
            style={accent ? ({ '--row-accent': `var(--tag-${accent})` } as CSSProperties) : undefined}
            onClick={() => setSelected(d.id)}
            onDoubleClick={() => (own || !prefs.doubleClickReply ? setSelected(d.id) : onReply(d.id))}
          >
            <span className="c-time num muted">{formatClock(d.time_ms)}</span>
            <span className="c-snr num">{own ? 'Tx' : d.snr}</span>
            {prefs.showDt ? <span className="c-dt num muted">{own ? '' : d.delta_time.toFixed(1)}</span> : null}
            <span className="c-freq num muted">{d.freq_hz}</span>
            <span className="decode-text">
              <span className="data">{dxCall ? highlightCall(d.text, dxCall) : d.text}</span>
            </span>
            <span className="decode-where">
              <span className="w-flag">{code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}</span>
              <span className="w-country">{prefs.showCountry && d.country ? <span className="decode-country">{d.country}</span> : null}</span>
              <span className="w-km num muted">{km !== null ? formatDistance(km, prefs.distanceUnit) : ''}</span>
            </span>
            {badge ? (
              <span className="badge badge--tag badge-cell" title={t(`kind.${lit}` as Key)} style={{ background: `var(--tag-${badge.color})`, color: `var(--tag-${badge.color}-ink)` }}>
                {t(badge.label)}
              </span>
            ) : (
              <span className="badge-cell" />
            )}
            {toMe && !isClosingMessage(d.text) ? <span className="me-mark" aria-hidden="true" /> : null}
          </button>
        );
      })}
    </section>
  );
}
