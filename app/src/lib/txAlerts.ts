import type { TxAlert } from '../api/schemas';

/** Il PTT non spento ha il suo avviso rosso fisso (`PttStuckBanner`): non si ripete negli avvisi della trasmissione. */
const OWN_BANNER = new Set(['ptt_stuck', 'ptt_abandoned']);

/** Gli avvisi da mostrare nella barra: tutti tranne quelli del PTT; prima i rossi (ROS alto, trasmissione fermata), poi gli altri, nell'ordine del motore. */
export function visibleTxAlerts(alerts: readonly TxAlert[] | null | undefined): TxAlert[] {
  const list = (alerts ?? []).filter((a) => !OWN_BANNER.has(a.kind));
  return [...list.filter((a) => txAlertTone(a) === 'danger'), ...list.filter((a) => txAlertTone(a) !== 'danger')];
}

/** Rosso per il ROS alto e per la trasmissione fermata, ambra per gli altri. */
export function txAlertTone(a: TxAlert): 'danger' | 'warn' {
  return a.kind === 'high_swr' || a.kind === 'tx_stopped' ? 'danger' : 'warn';
}
