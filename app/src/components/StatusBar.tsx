import type { StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';
import { useT } from '../i18n';
import { ClockPill } from './ClockPill';
import { useQuery } from '@tanstack/react-query';
import { getDxcc } from '../api/client';
import { countryOfCall } from '../lib/dxCountry';
import { flagCode } from '../lib/flags';
import { formatMhz } from '../lib/format';
import { useStation } from '../state/station';

interface Props {
  status: StationStatus | null;
  connection: ConnectionState;
}

export function StatusBar({ status, connection }: Props) {
  const t = useT();
  const decodes = useStation((s) => s.decodes);
  const heard = countryOfCall(decodes, status?.dx_call);
  // la stazione scelta puo' non essere tra i messaggi recenti: allora il paese si cerca nel file dei prefissi
  const looked = useQuery({ queryKey: ['dxcc', status?.dx_call ?? ''], queryFn: () => getDxcc(status?.dx_call ?? ''), enabled: !heard && !!status?.dx_call, staleTime: Infinity, retry: false });
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
          <span className="pill data" title={t('tip.status.freq')}>
            {formatMhz(status.dial_hz)}
          </span>
          <ClockPill />
          {status.dx_call ? (
            <span className="pill data pill--dx" title={t('tip.status.dx')}>
              <span className="pill-dx-main">
                <span className="pill-dx-label">DX</span> <strong>{status.dx_call}</strong>
                {status.dx_grid ? <span className="muted"> {status.dx_grid}</span> : null}
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
