import type { StationStatus } from '../api/schemas';
import type { ConnectionState } from '../api/stream';
import { useT } from '../i18n';
import { ClockPill } from './ClockPill';
import { countryOfCall } from '../lib/dxCountry';
import { flagCode } from '../lib/flags';
import { bandOf, formatMhz } from '../lib/format';
import { useStation } from '../state/station';

interface Props {
  status: StationStatus | null;
  connection: ConnectionState;
}

export function StatusBar({ status, connection }: Props) {
  const t = useT();
  const decodes = useStation((s) => s.decodes);
  const dxCountry = countryOfCall(decodes, status?.dx_call);
  const dxFlag = flagCode(dxCountry);
  const band = status ? bandOf(status.dial_hz) : null;
  const state = !status ? '—' : status.transmitting ? t('status.tx') : status.decoding ? t('status.rx') : t('status.idle');
  const label = { connecting: t('conn.connecting'), open: t('conn.open'), closed: t('conn.closed') }[connection];
  return (
    <div className="status-bar" role="status">
      <span className={`pill pill--link pill--${connection}`} title={t('tip.conn')}>
        {label}
      </span>
      {status && (
        <>
          {band && (
            <span className="pill data" title={t('tip.status.band')}>
              {band}
            </span>
          )}
          <span className="pill data" title={t('tip.status.freq')}>
            {formatMhz(status.dial_hz)}
          </span>
          <span className="pill data" title={t('tip.status.mode')}>
            {status.mode}
          </span>
          <span className={`pill data ${status.transmitting ? 'pill--tx' : 'pill--rx'}`} title={t('tip.status.state')}>
            {state}
          </span>
          <ClockPill />
          <span className="pill data muted" title={t('tip.status.me')}>
            {status.de_call} {status.de_grid}
          </span>
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
