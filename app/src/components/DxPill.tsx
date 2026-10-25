import { useQuery } from '@tanstack/react-query';
import type { StationStatus } from '../api/schemas';
import { getDxcc } from '../api/client';
import { useT } from '../i18n';
import { countryOfCall } from '../lib/dxCountry';
import { flagCode } from '../lib/flags';
import { useStation } from '../state/station';

/** La stazione DX attuale (nominativo, locatore, bandiera e paese) nell'etichetta gialla: sta nel riquadro Trasmissione. Senza stazione non mostra niente. */
export function DxPill({ status }: { status: StationStatus | null }) {
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
  if (!dxCall) return null;
  return (
    <span className="pill data pill--dx" title={t('tip.status.dx')}>
      <span className="pill-dx-main">
        <span className="pill-dx-label">DX</span> <strong>{dxCall}</strong>
        {status?.dx_grid && !pendingDx ? <span className="muted"> {status.dx_grid}</span> : null}
      </span>
      {dxCountry ? (
        <span className="pill-dx-country">
          {dxFlag ? <span className={`fi fi-${dxFlag} pill-dx-flag`} aria-hidden="true" /> : null}
          {dxCountry}
        </span>
      ) : null}
    </span>
  );
}
