import type { StationStatus } from '../api/schemas';

interface Props {
  status: StationStatus | null;
}

/**
 * Linguetta piccola a destra, come sui ricetrasmettitori: rosso lampeggiante = trasmissione in onda,
 * verde fisso = ascolto. In trasmissione riporta il messaggio che parte.
 */
export function TxBanner({ status }: Props) {
  if (!status) return null;
  if (status.transmitting) {
    return (
      <div className="onair-tab" role="alert" data-state="tx">
        <span className="onair-dot" aria-hidden="true" />
        <strong>IN TRASMISSIONE</strong>
        {status.tx_message ? <span className="data onair-msg">{status.tx_message}</span> : null}
        {status.tx_df ? <span className="data">{status.tx_df} Hz</span> : null}
      </div>
    );
  }
  return (
    <div className="onair-tab" role="status" data-state="rx">
      <span className="onair-dot" aria-hidden="true" />
      <strong>IN ASCOLTO</strong>
      {status.tx_enabled ? <span className="onair-armed">Tx abilitato</span> : null}
    </div>
  );
}
