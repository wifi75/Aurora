import type { StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';
import { useT } from '../i18n';
import { ClockPill } from './ClockPill';
import { useQuery } from '@tanstack/react-query';
import { getDxcc } from '../api/client';
import { countryOfCall } from '../lib/dxCountry';
import { flagCode } from '../lib/flags';
import { formatDial } from '../lib/format';
import { useStation } from '../state/station';

interface Props {
  status: StationStatus | null;
  connection: ConnectionState;
  /** La frequenza e' gia' sul display della radio: la pastiglia si toglie per non ripeterla. */
  hideDial?: boolean;
}

export function StatusBar({ status, connection, hideDial = false }: Props) {
  const t = useT();
  const decodes = useStation((s) => s.decodes);
  const pendingDx = useStation((s) => s.pendingDx);
  // la stazione scelta col doppio clic compare subito, prima che il motore la riporti
  const dxCall = pendingDx || status?.dx_call || '';
  const heard = countryOfCall(decodes, dxCall);
  // la stazione scelta puo' non essere tra i messaggi recenti: allora il paese si cerca nel file dei prefissi
  const looked = useQuery({ queryKey: ['dxcc', dxCall], queryFn: () => getDxcc(dxCall), enabled: !heard && dxCall !== '', staleTime: Infinity, retry: false });
  const dxCountry = heard ?? looked.data?.country ?? null;
  const dxFlag = flagCode(dxCountry);
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
          {dxCall ? (
            <span className="pill data pill--dx" title={t('tip.status.dx')}>
              <span className="pill-dx-main">
                <span className="pill-dx-label">DX</span> <strong>{dxCall}</strong>
                {status.dx_grid && !pendingDx ? <span className="muted"> {status.dx_grid}</span> : null}
              </span>
              {dxCountry ? (
                <span className="pill-dx-country">
                  {dxFlag ? <span className={`fi fi-${dxFlag} pill-dx-flag`} aria-hidden="true" /> : null}
                  {dxCountry}
                </span>
              ) : null}
            </span>
          ) : null}
        </>
      )}
    </div>
  );
}
