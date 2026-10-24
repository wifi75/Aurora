import type { StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';
import { bandOf, formatMhz } from '../lib/format';

const CONNECTION_LABEL: Record<ConnectionState, string> = {
  connecting: 'Connessione…',
  open: 'In linea',
  closed: 'Scollegato',
};

interface Props {
  status: StationStatus | null;
  connection: ConnectionState;
}

export function StatusBar({ status, connection }: Props) {
  const band = status ? bandOf(status.dial_hz) : null;
  const state = !status ? '—' : status.transmitting ? 'TX' : status.decoding ? 'RX' : 'Fermo';
  return (
    <div className="status-bar" role="status">
      <span className={`pill pill--link pill--${connection}`}>{CONNECTION_LABEL[connection]}</span>
      {status && (
        <>
          {band && <span className="pill data">{band}</span>}
          <span className="pill data">{formatMhz(status.dial_hz)}</span>
          <span className="pill data">{status.mode}</span>
          <span className={`pill data ${status.transmitting ? 'pill--tx' : 'pill--rx'}`}>{state}</span>
          <span className="pill data muted">
            {status.de_call} {status.de_grid}
          </span>
        </>
      )}
    </div>
  );
}
