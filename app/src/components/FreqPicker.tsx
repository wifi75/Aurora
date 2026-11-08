import { useQuery } from '@tanstack/react-query';
import { getFrequencies } from '../api/client';
import { useT } from '../i18n';
import { bandOf } from '../lib/format';

interface Props {
  dialHz: number;
  mode: string;
  /** Va sulla frequenza scelta; senza, il menu non si mostra. */
  onPick?(hz: number): void;
  locked?: boolean;
}

/** Le frequenze della banda in uso per il modo in uso (standard, alternative, DXpedition) dalla tabella delle Frequenze: se ne sceglie una. */
export function FreqPicker({ dialHz, mode, onPick, locked = false }: Props) {
  const t = useT();
  const table = useQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, staleTime: 60_000, retry: false });
  const band = dialHz > 0 ? bandOf(dialHz) : null;
  const m = mode.toUpperCase().split('+')[0];
  const rows = (table.data ?? []).filter((r) => r.band === band && r.mode.toUpperCase() === m).sort((a, b) => a.hz - b.hz);
  if (!onPick || rows.length < 2) return null;
  const current = rows.find((r) => r.hz === dialHz);
  return (
    <select
      className="freq-picker"
      aria-label={t('band.freqs')}
      title={t('tip.band.freqs')}
      disabled={locked}
      value={current ? String(current.hz) : ''}
      onChange={(e) => {
        const hz = Number(e.target.value);
        if (hz > 0) onPick(hz);
      }}
    >
      {current ? null : <option value="">{(dialHz / 1e6).toFixed(3)}</option>}
      {rows.map((r) => (
        <option key={`${r.hz}-${r.region}`} value={r.hz}>
          {(r.hz / 1e6).toFixed(r.hz % 1000 === 0 ? 3 : 4)}
          {r.preferred ? '' : ` · ${t('band.alt')}`}
          {r.region !== 'ALL' ? ` (${r.region})` : ''}
        </option>
      ))}
    </select>
  );
}
