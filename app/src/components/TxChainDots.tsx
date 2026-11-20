import { useT, type Key } from '../i18n';
import { chainDots, type DotState } from '../lib/txChain';
import { useStation } from '../state/station';

const LABEL = { mode: 'tx.chain.mode', audio: 'tx.chain.audio', ptt: 'tx.chain.ptt' } as const;
const TIP: Record<'mode' | 'audio' | 'ptt', Record<DotState, Key>> = {
  mode: { ok: 'tx.chain.mode.ok', bad: 'tx.chain.mode.bad', unknown: 'tx.chain.mode.unknown' },
  audio: { ok: 'tx.chain.audio.ok', bad: 'tx.chain.audio.bad', unknown: 'tx.chain.audio.unknown' },
  ptt: { ok: 'tx.chain.ptt.ok', bad: 'tx.chain.ptt.bad', unknown: 'tx.chain.ptt.unknown' },
};

/**
 * Tre pallini nella scheda Trasmissione: come e' andata l'ultima trasmissione (modo radio, audio, PTT). Verde = a posto, rosso = problema,
 * grigio = la radio non lo dice. La causa e' nel suggerimento; nessuna finestra si apre. Non compare prima della prima trasmissione.
 */
export function TxChainDots() {
  const t = useT();
  const chain = useStation((s) => s.status?.tx_chain);
  const dots = chainDots(chain);
  if (dots.length === 0) return null;
  return (
    <span className="tx-chain" role="group" aria-label={t('tx.chain.title')}>
      {dots.map((d) => {
        const tip = `${t(LABEL[d.key])}: ${t(TIP[d.key][d.state])}${d.value ? ` (${d.value})` : ''}`;
        return <span key={d.key} className="tx-chain-dot" data-state={d.state} role="img" aria-label={tip} title={tip} />;
      })}
    </span>
  );
}
