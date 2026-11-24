import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { getOutputLevel, putOutputLevel } from '../api/client';
import { useT } from '../i18n';
import { attenuationFromPercent, levelPercent } from '../lib/outputLevel';
import { replyErrorMessage } from '../lib/replyError';
import { VerticalSlider } from './VerticalSlider';

interface Props {
  /** Il livello si regola sempre, come nel motore; la proprieta' resta perche' il pannello la passa a tutti i suoi blocchi. */
  txAllowed?: boolean;
  /** Cursore disteso in orizzontale (riga compatta). */
  horizontal?: boolean;
}

/** Dopo il rilascio, per quanto una lettura del motore non sposta il cursore (la richiesta di scrittura e la lettura periodica si possono incrociare). */
const SETTLE_MS = 2500;

/** Il cursore «Lvl TX» (prima «Pwr») del motore: livello audio di uscita. Si invia al rilascio, non a ogni passo. */
export function OutputLevel({ horizontal = false }: Props) {
  const t = useT();
  const client = useQueryClient();
  const level = useQuery({ queryKey: ['output-level'], queryFn: getOutputLevel, retry: false, refetchInterval: (q) => (q.state.status === 'error' ? 3000 : 2000) });
  const [pct, setPct] = useState<number | null>(null);
  const dragging = useRef(false);
  // quando si e' rilasciato il cursore l'ultima volta: per un attimo una risposta vecchia del motore non deve riportarlo indietro
  const settled = useRef(0);
  const [note, setNote] = useState('');
  // finita la finestra di assestamento si rilegge il valore del motore: una lettura identica non cambia il riferimento dei dati e senza questo il
  // cambio arrivato durante la finestra (cambio banda con memoria per banda) resterebbe scartato fino al cambio successivo
  const [resync, setResync] = useState(0);
  const resyncTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(resyncTimer.current), []);

  // il valore del motore vince quando non si sta trascinando: se cambia da solo (cambio banda con la memoria per banda) il cursore lo segue
  useEffect(() => {
    if (level.data && !dragging.current && Date.now() - settled.current > SETTLE_MS) setPct(levelPercent(level.data.attenuation_db));
  }, [level.data, resync]);

  if (level.isError) return <p className="muted out-level-off" title={t('out.unavailable')}>{t('out.short')} –</p>;
  if (pct === null) return null;

  const commit = async (value: number) => {
    settled.current = Date.now();
    try {
      await putOutputLevel(attenuationFromPercent(value));
      settled.current = Date.now();
      window.clearTimeout(resyncTimer.current);
      resyncTimer.current = window.setTimeout(() => setResync((n) => n + 1), SETTLE_MS + 100);
      await client.invalidateQueries({ queryKey: ['output-level'] });
      setNote(t('out.saved'));
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  return (
    <div className="out-level" title={t('tip.out')}>
      <span className="out-level-title">{t('out.short')}</span>
      {/* come in JTDX (attenuazione = (450 - valore) / 10 dB, verificato nel programma): in alto 0 dB (potenza massima), in basso 45 dB (minima) */}
      <VerticalSlider
        horizontal={horizontal}
        value={pct}
        label={t('out.label')}
        onChange={(v) => {
          dragging.current = true;
          setPct(v);
        }}
        onCommit={(v) => {
          dragging.current = false;
          void commit(v);
        }}
      />
      {/* come in WSJT-X: i dB di attenuazione si leggono subito sul cursore («Transmit drive -30.9 dB»); la percentuale resta nel suggerimento */}
      <span className="data muted out-level-value" title={t('out.value', { pct, db: attenuationFromPercent(pct).toFixed(1) })}>{dbText(attenuationFromPercent(pct))}</span>
      {note ? <span className="live-note" role="status">{note}</span> : null}
    </div>
  );
}

/** «−22.5 dB»; sotto il decimo di dB (anche lo zero) «0 dB», mai «−0.0». */
function dbText(attenuation: number): string {
  const v = attenuation.toFixed(1);
  return v === '0.0' ? '0 dB' : `−${v} dB`;
}
