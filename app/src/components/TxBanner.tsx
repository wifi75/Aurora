import type { StationStatus } from '../api/schemas';
import { useT } from '../i18n';

interface Props {
  status: StationStatus | null;
}

/**
 * Linguetta piccola a destra, come sui ricetrasmettitori: rosso lampeggiante = trasmissione in onda,
 * verde fisso = ascolto. In trasmissione riporta il messaggio che parte.
 */
export function TxBanner({ status }: Props) {
  const t = useT();
  if (!status) return null;
  if (status.transmitting) {
    return (
      <div className="onair-tab" role="alert" data-state="tx" title={t('tip.onair')}>
        <span className="onair-dot" aria-hidden="true" />
        <strong>{t('onair.tx')}</strong>
        {status.tx_message ? <span className="data onair-msg">{status.tx_message}</span> : null}
        {status.tx_df ? <span className="data">{status.tx_df} Hz</span> : null}
      </div>
    );
  }
  return (
    <div className="onair-tab" role="status" data-state="rx" title={t('tip.onair')}>
      <span className="onair-dot" aria-hidden="true" />
      <strong>{t('onair.rx')}</strong>
      {status.tx_enabled ? <span className="onair-armed">{t('onair.armed')}</span> : null}
    </div>
  );
}
