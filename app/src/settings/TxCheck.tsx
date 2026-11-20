import { useMutation } from '@tanstack/react-query';
import { getAudioDevices, getCat, getOutputLevel } from '../api/client';
import { useT, type Key } from '../i18n';
import { evaluateTxCheck, type CheckItem } from '../lib/txCheck';
import { Group } from './layout';

const LABEL: Record<CheckItem['key'], Key> = { cat: 'txcheck.l.cat', mode: 'tx.chain.mode', ptt: 'tx.chain.ptt', audio: 'tx.chain.audio', level: 'txcheck.l.level' };

/**
 * «Prova senza trasmettere» (Impostazioni › Radio): guarda CAT, modo radio, PTT, uscita audio e livello con quello che gia' si sa, senza
 * dare il PTT e senza mandare audio (con il VOX farebbe trasmettere).
 */
export function TxCheck() {
  const t = useT();
  const run = useMutation({
    mutationFn: async () => {
      const [cat, audio, level] = await Promise.all([getCat().catch(() => null), getAudioDevices().catch(() => null), getOutputLevel().catch(() => null)]);
      return evaluateTxCheck({ cat, audio, level });
    },
  });
  return (
    <Group title={t('txcheck.title')}>
      <p className="notice">{t('txcheck.hint')}</p>
      <p>
        <button type="button" className="glass chip" onClick={() => run.mutate()} disabled={run.isPending}>
          {t('txcheck.run')}
        </button>
      </p>
      {run.data ? (
        <ul className="txcheck-list" aria-live="polite">
          {run.data.map((i) => (
            <li key={i.key} className="txcheck-item" data-state={i.state}>
              <span className="tx-chain-dot" data-state={i.state} aria-hidden="true" />
              <b>{t(LABEL[i.key])}</b>: {t(i.text as Key)}
              {i.value ? ` (${i.value})` : ''}
            </li>
          ))}
        </ul>
      ) : null}
    </Group>
  );
}
