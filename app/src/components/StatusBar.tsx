import type { StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';
import { useT } from '../i18n';
import { ClockPill } from './ClockPill';
import { formatDial } from '../lib/format';

interface Props {
  status: StationStatus | null;
  connection: ConnectionState;
  /** La frequenza e' gia' sul display della radio: la pastiglia si toglie per non ripeterla. */
  hideDial?: boolean;
}

export function StatusBar({ status, connection, hideDial = false }: Props) {
  const t = useT();
  const label = { connecting: t('conn.connecting'), open: t('conn.open'), closed: t('conn.closed') }[connection];
  return (
    <div className="status-bar" role="status">
      <span className={`pill pill--link pill--${connection}`} title={t('tip.conn')}>
        {label}
      </span>
      {status && (
        <>
          {hideDial ? null : (
            <span className="pill data" title={t('tip.status.freq')}>
              {formatDial(status.dial_hz)}
            </span>
          )}
          <ClockPill />
        </>
      )}
    </div>
  );
}
