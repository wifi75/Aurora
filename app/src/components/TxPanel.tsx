import type { DecodedMessage, StationStatus } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { useStation } from '../state/station';
import { OutputLevel } from './OutputLevel';
import { QsoSwitches } from './QsoSwitches';
import { activeTx, nextTx, txMessages } from '../lib/txMessages';

interface Props {
  /** La trasmissione e' aperta su questo PC (AURORA_ALLOW_TX=1). */
  txAllowed: boolean;
  /** Stato di JTDX: stazione DX, rapporto e messaggio in corso. */
  status?: StationStatus | null;
  /** Messaggi decodificati (i piu' recenti per primi): servono a capire a che punto e' il QSO. */
  decodes?: DecodedMessage[];
}

/** Stato del QSO: stazione DX, i sei messaggi Tx con la fase in corso e il livello audio di uscita. */
export function TxPanel({ txAllowed, status = null, decodes = [] }: Props) {
  const t = useT();
  const pendingDx = useStation((s) => s.pendingDx);
  const messages = status
    ? txMessages({ myCall: status.de_call, myGrid: status.de_grid, dxCall: status.dx_call ?? '', report: status.report ?? '' })
    : [];
  const active = activeTx(messages, status?.tx_message);
  const next = status && active < 0 ? nextTx(decodes, status.de_call, status.dx_call ?? '') : -1;

  return (
    <div className="tx-panel">
      <div className="tx-main tx-split">
        <section className="tx-left tx-block" aria-label={t('ctl.sec.dx')}>
      <h3 className="cmd-title">{t('ctl.sec.dx')}</h3>
      {status && (
        <>
          <div className="tx-dx">
            <strong className="data tx-dx-call">{status.dx_call || pendingDx || '—'}</strong>
            {status.dx_grid ? <span className="data muted">{status.dx_grid}</span> : null}
            <span className="data muted tx-freqs">
              <span>{`Tx ${status.tx_df ?? '—'} Hz`}</span> <span>{`Rx ${status.rx_df ?? '—'} Hz`}</span>
            </span>
          </div>
          <ol className="tx-list" aria-label={t('tx.list')} title={t('tip.tx.list')}>
            {messages.map((m, i) => (
              <li
                key={i}
                aria-current={i === active ? 'true' : undefined}
                data-next={i === next && m !== '' ? 'true' : undefined}
                className={m === '' ? 'tx-empty' : undefined}
              >
                <span className="tx-n">{i + 1}</span>
                <span className="data tx-msg">{m}</span>
                <span className="tx-step">{t(`tx.step.${i + 1}` as Key)}</span>
              </li>
            ))}
          </ol>
        </>
      )}
        </section>
        <QsoSwitches txAllowed={txAllowed} />
      </div>
      <OutputLevel txAllowed={txAllowed} />
    </div>
  );
}
