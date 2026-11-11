import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ApiError, getDxcc, getRotator, rotatorGoto, rotatorStop, type RotatorStatus } from '../../api/client';
import { useT } from '../../i18n';
import { bearingDeg, gridCenter, type Point } from '../../lib/geo';
import { compassPoint, longPath, normalizeAz, nudge } from '../../lib/rotator';
import { useStatusPick } from '../../state/station';
import { Rose } from '../../pages/RotatorPage';
import '../../styles/rotator.css';

interface Props {
  /** Il nominativo e il locatore del QSO in corso: da li' si calcola la direzione del DX. */
  call: string;
  grid: string;
}

const fmt = (az: number | null | undefined) => (az === null || az === undefined ? '—' : `${Math.round(normalizeAz(az))}°`);

/**
 * Il blocco quadrato del rotore nelle pagine PSK, RTTY e CW: la rosa dei venti con l'ago (un clic la porta li'), la direzione del DX del
 * QSO in corso per il percorso corto e lungo, i pulsanti per girare di 10 gradi e lo STOP.
 */
export function RotorBlock({ call, grid }: Props) {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['rotator'], queryFn: getRotator, retry: false, refetchInterval: 1000 });
  const station = useStatusPick(['de_grid']);
  const status = snap.data?.status;
  const connected = status?.connected === true;
  const cleanCall = call.trim().toUpperCase();

  // la direzione del DX: dal suo locatore, altrimenti dal centro del suo paese
  const dxcc = useQuery({ queryKey: ['dxcc', cleanCall], queryFn: () => getDxcc(cleanCall), enabled: cleanCall.length >= 3 && grid.trim() === '', staleTime: 10 * 60_000, retry: false });
  const from = station?.de_grid ? gridCenter(station.de_grid) : null;
  const to: Point | null = gridCenter(grid) ?? (typeof dxcc.data?.lat === 'number' && typeof dxcc.data.lon === 'number' ? { lat: dxcc.data.lat, lon: dxcc.data.lon } : null);
  const sp = from && to ? bearingDeg(from, to) : null;
  const lp = sp === null ? null : longPath(sp);

  const refresh = (s: RotatorStatus) => client.setQueryData(['rotator'], (old: typeof snap.data) => (old ? { ...old, status: s } : old));
  const go = useMutation({ mutationFn: (az: number) => rotatorGoto(normalizeAz(az)), onSuccess: refresh });
  const stop = useMutation({ mutationFn: rotatorStop, onSuccess: refresh });
  const base = status?.target ?? status?.az ?? 0;
  const problem = go.error ?? stop.error;
  const problemText = problem instanceof ApiError && problem.code === 'not_connected' ? t('rot.err.notConnected') : problem ? t('rot.err.failed') : '';

  return (
    <div className="digi-rotor">
      {!snap.data?.config.enabled ? (
        <p className="muted digi-rotor-note">
          {t('rot.offText')} <a href="#/impostazioni/rotator">{t('rot.openSettings')}</a>
        </p>
      ) : null}
      <div className="digi-rotor-square">
        <Rose status={status} sp={sp} lp={lp} dxCall={cleanCall} disabled={!connected} onPick={(az) => go.mutate(az)} label={t('rot.rose')} />
      </div>
      <div className="digi-rotor-read">
        <span>
          {t('rot.heading')} <b>{fmt(status?.az)}</b> {status?.az === null || status?.az === undefined ? '' : compassPoint(status.az)}
        </span>
        <span>
          {t('rot.target')} <b>{fmt(status?.target)}</b>
        </span>
      </div>
      <div className="digi-rotor-buttons">
        <button type="button" className="glass digi-rotor-btn" disabled={!connected || sp === null} onClick={() => sp !== null && go.mutate(sp)} title={t('rot.sp')}>
          {t('rot.sp')} {sp !== null ? `${sp}°` : ''}
        </button>
        <button type="button" className="glass digi-rotor-btn" disabled={!connected || lp === null} onClick={() => lp !== null && go.mutate(lp)} title={t('rot.lp')}>
          {t('rot.lp')} {lp !== null ? `${lp}°` : ''}
        </button>
        <button type="button" className="glass digi-rotor-btn" disabled={!connected} onClick={() => go.mutate(nudge(base, -10))}>
          −10°
        </button>
        <button type="button" className="glass digi-rotor-btn" disabled={!connected} onClick={() => go.mutate(nudge(base, 10))}>
          +10°
        </button>
        <button type="button" className="digi-rotor-btn digi-rotor-stop" disabled={!connected} onClick={() => stop.mutate()}>
          {t('rot.stop')}
        </button>
      </div>
      {problemText ? <p className="notice digi-rotor-note" role="alert">{problemText}</p> : null}
      <p className="muted digi-rotor-note">{t('rot.clickHint')}</p>
    </div>
  );
}
