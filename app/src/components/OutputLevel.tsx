import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getOutputLevel, putOutputLevel } from '../api/client';
import { useT } from '../i18n';
import { attenuationFromPercent, levelPercent } from '../lib/outputLevel';
import { replyErrorMessage } from '../lib/replyError';
import { VerticalSlider } from './VerticalSlider';

interface Props {
  /** Il livello si regola sempre, come nel motore; la proprieta' resta perche' il pannello la passa a tutti i suoi blocchi. */
  txAllowed?: boolean;
}

/** Il cursore "Pwr" del motore: livello audio di uscita. Si invia al rilascio, non a ogni passo. */
export function OutputLevel(_: Props) {
  const t = useT();
  const client = useQueryClient();
  const level = useQuery({ queryKey: ['output-level'], queryFn: getOutputLevel, retry: false, refetchInterval: (q) => (q.state.status === 'error' ? 3000 : 15_000) });
  const [pct, setPct] = useState<number | null>(null);
  const [note, setNote] = useState('');

  // il valore del motore vince quando non si sta trascinando
  useEffect(() => {
    if (level.data && pct === null) setPct(levelPercent(level.data.attenuation_db));
  }, [level.data, pct]);

  if (level.isError) return <p className="muted out-level-off" title={t('out.unavailable')}>Pwr –</p>;
  if (pct === null) return null;

  const commit = async (value: number) => {
    try {
      await putOutputLevel(attenuationFromPercent(value));
      await client.invalidateQueries({ queryKey: ['output-level'] });
      setNote(t('out.saved'));
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  return (
    <div className="out-level" title={t('tip.out')}>
      <span className="out-level-title">Pwr</span>
      {/* come nel motore il cursore e' l'attenuazione: in basso 0 dB (potenza massima), in alto 45 dB (minima) */}
      <VerticalSlider
        value={100 - pct}
        label={t('out.label')}
        onChange={(v) => setPct(100 - v)}
        onCommit={(v) => void commit(100 - v)}
      />
      <span className="data muted out-level-value" title={t('out.value', { pct, db: attenuationFromPercent(pct).toFixed(1) })}>{pct}%</span>
      {note ? <span className="live-note" role="status">{note}</span> : null}
    </div>
  );
}
