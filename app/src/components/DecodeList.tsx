import type { DecodedMessage, DecodeTag } from '../api/schemas';
import { formatClock } from '../lib/format';

const BADGES: Partial<Record<DecodeTag, { label: string; kind: 'rare' | 'auto' }>> = {
  new_dxcc: { label: 'NUOVO DXCC', kind: 'rare' },
  new_zone: { label: 'NUOVA ZONA', kind: 'rare' },
  auto: { label: 'AUTO', kind: 'auto' },
};

interface Props {
  decodes: DecodedMessage[];
  onReply(decodeId: number): void;
}

export function DecodeList({ decodes, onReply }: Props) {
  if (decodes.length === 0) {
    return (
      <section className="decode-list decode-list--empty" aria-label="Messaggi decodificati">
        <p>In ascolto: i messaggi compaiono qui</p>
      </section>
    );
  }
  return (
    <section className="decode-list" aria-label="Messaggi decodificati">
      {decodes.map((d) => {
        const badge = d.tag ? BADGES[d.tag] : undefined;
        return (
          <button type="button" key={d.id} className="decode-row" onClick={() => onReply(d.id)}>
            <span className="num muted">{formatClock(d.time_ms)}</span>
            <span className="num">{d.snr}</span>
            <span className="num muted">{d.freq_hz}</span>
            <span className="data decode-text">{d.text}</span>
            {badge ? <span className={`badge badge--${badge.kind}`}>{badge.label}</span> : <span />}
          </button>
        );
      })}
    </section>
  );
}
