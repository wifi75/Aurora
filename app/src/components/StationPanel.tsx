import type { StationStatus } from '../api/schemas';
import { bandOf, formatMhz } from '../lib/format';

interface Props {
  status: StationStatus | null;
}

function txLabel(s: StationStatus): { text: string; hot: boolean } {
  if (s.transmitting) return { text: 'In trasmissione', hot: true };
  if (s.tx_enabled) return { text: 'Abilitata', hot: false };
  return { text: 'Spenta', hot: false };
}

/** Scheda "Stazione": i dati che JTDX comunica, in forma leggibile. */
export function StationPanel({ status }: Props) {
  if (!status) {
    return <p className="muted station-empty">In attesa di JTDX…</p>;
  }
  const tx = txLabel(status);
  return (
    <dl className="facts">
      <div>
        <dt>Banda</dt>
        <dd className="data">{bandOf(status.dial_hz) ?? 'Banda non nota'}</dd>
      </div>
      <div>
        <dt>Frequenza</dt>
        <dd className="data">{status.dial_hz > 0 ? formatMhz(status.dial_hz) : '—'}</dd>
      </div>
      <div>
        <dt>Modo</dt>
        <dd className="data">{status.mode}</dd>
      </div>
      <div>
        <dt>Stazione</dt>
        <dd className="data">{`${status.de_call} ${status.de_grid}`.trim()}</dd>
      </div>
      <div>
        <dt>Ricezione</dt>
        <dd>{status.decoding ? 'In ricezione' : 'In attesa del prossimo ciclo'}</dd>
      </div>
      <div>
        <dt>Trasmissione</dt>
        <dd className={tx.hot ? 'tx-hot' : undefined}>{tx.text}</dd>
      </div>
    </dl>
  );
}
