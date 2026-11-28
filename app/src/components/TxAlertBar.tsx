import { useT } from '../i18n';
import { visibleTxAlerts, txAlertTone } from '../lib/txAlerts';
import { useStation } from '../state/station';
import { alertText } from './TxPanel';

/**
 * Gli avvisi di sicurezza della trasmissione (ROS alto, trasmissione fermata, Fox/Hound, inibito...) in una barra a tutta larghezza sotto
 * i comandi in alto della pagina FT8: ben visibile e lampeggiante. Sparisce da sola, senza lasciare spazio, quando il motore li toglie.
 */
export function TxAlertBar() {
  const t = useT();
  const raw = useStation((s) => s.status?.tx_alerts);
  const list = visibleTxAlerts(raw);
  if (list.length === 0) return null;
  return (
    <div className="tx-alert-bar" role="alert" title={t('tip.tx.alert')}>
      {list.map((a) => (
        <p key={a.kind} className={`tx-alert-bar-row tx-alert-bar-row--${txAlertTone(a)}`}>
          <span className="tx-alert-bar-icon" aria-hidden="true">⚠</span>
          <span className="tx-alert-bar-text">{alertText(a, t)}</span>
        </p>
      ))}
    </div>
  );
}
