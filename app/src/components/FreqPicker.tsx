import { useQuery } from '@tanstack/react-query';
import { getFrequencies } from '../api/client';
import { useT } from '../i18n';

interface Props {
  dialHz: number;
  mode: string;
  /** Va sulla frequenza scelta; senza, il menu non si mostra. */
  onPick?(hz: number): void;
  locked?: boolean;
}

/** Come in WSJT-X: tendina con tutte le frequenze del modo in uso, divise per banda (standard, alternative, DXpedition), dalla tabella delle Frequenze. */
export function FreqPicker({ dialHz, mode, onPick, locked = false }: Props) {
  const t = useT();
  const table = useQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, staleTime: 60_000, retry: false });
  const m = mode.toUpperCase().split('+')[0];
  // come la tendina delle frequenze di WSJT-X: tutte le righe del modo in uso, banda per banda
  const rows = (table.data ?? []).filter((r) => r.mode.toUpperCase() === m).sort((a, b) => a.hz - b.hz);
  if (!onPick || rows.length === 0) return null;
  const bands = [...new Set(rows.map((r) => r.band))];
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
      {bands.map((b) => (
        <optgroup key={b} label={b}>
          {rows
            .filter((r) => r.band === b)
            .map((r) => (
              <option key={`${r.hz}-${r.region}`} value={r.hz}>
                {b} {(r.hz / 1e6).toFixed(r.hz % 1000 === 0 ? 3 : 4)} MHz
                {r.preferred ? '' : ` · ${t('band.alt')}`}
                {r.region !== 'ALL' ? ` (${r.region})` : ''}
              </option>
            ))}
        </optgroup>
      ))}
    </select>
  );
}
