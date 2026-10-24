import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import {
  ApiError,
  getAccess,
  getDevices,
  getPairingSettings,
  putAccess,
  putPairingSettings,
  revokeDevice,
  startPairing,
  type PairingOffer,
} from '../api/client';
import { useT, type T } from '../i18n';
import { Check, Help } from './common';

function errorText(error: unknown, t: T): string {
  return error instanceof ApiError && error.code === 'local_only' ? t('acc.pcOnly') : t('set.saveFailed');
}

/** Scheda Accesso: accesso senza codice per le prove, durata dei codici, nuovo codice con QR, dispositivi. */
export function AccessTab() {
  const t = useT();
  const client = useQueryClient();

  const access = useQuery({ queryKey: ['access'], queryFn: getAccess, retry: false });
  const toggle = useMutation({
    mutationFn: (open: boolean) => putAccess(open),
    onSuccess: () => void client.invalidateQueries({ queryKey: ['access'] }),
  });

  const pairing = useQuery({ queryKey: ['pairing-settings'], queryFn: getPairingSettings, retry: false });
  const [minutes, setMinutes] = useState<string | null>(null);
  const saveTtl = useMutation({
    mutationFn: (secs: number) => putPairingSettings(secs),
    onSuccess: () => {
      setMinutes(null);
      void client.invalidateQueries({ queryKey: ['pairing-settings'] });
    },
  });

  const [offer, setOffer] = useState<PairingOffer | null>(null);
  const show = useMutation({ mutationFn: () => startPairing(window.location.origin), onSuccess: setOffer });

  const devices = useQuery({ queryKey: ['devices'], queryFn: getDevices, retry: false });
  const revoke = useMutation({
    mutationFn: (id: number) => revokeDevice(id),
    onSuccess: () => void client.invalidateQueries({ queryKey: ['devices'] }),
  });

  const shownMinutes = minutes ?? (pairing.data ? String(Math.round(pairing.data.code_ttl_secs / 60)) : '');
  const secs = Math.round(Number(shownMinutes) * 60);
  const inRange = pairing.data ? secs >= pairing.data.min_secs && secs <= pairing.data.max_secs : false;

  return (
    <>
      <h3 className="set-sub">{t('acc.open')}</h3>
      <Check label={t('acc.open')} hint={t('tip.acc.open')} checked={access.data?.open ?? false} onChange={(v) => toggle.mutate(v)} />
      <Help>{t('acc.openWarn')}</Help>
      {toggle.isError ? (
        <p className="live-note" role="alert">
          {errorText(toggle.error, t)}
        </p>
      ) : null}

      <h3 className="set-sub">{t('acc.ttl')}</h3>
      <div className="tx-row" title={t('tip.acc.ttl')}>
        <input
          type="number"
          className="ttl-input"
          aria-label={t('acc.ttl')}
          min={1}
          max={60}
          value={shownMinutes}
          onChange={(e) => setMinutes(e.target.value)}
        />
        <button type="button" className="glass glass--on" disabled={minutes === null || !inRange} onClick={() => saveTtl.mutate(secs)}>
          {t('set.save')}
        </button>
      </div>
      {saveTtl.isSuccess && minutes === null ? <p className="live-note">{t('acc.ttlSaved')}</p> : null}
      {saveTtl.isError ? (
        <p className="live-note" role="alert">
          {errorText(saveTtl.error, t)}
        </p>
      ) : null}

      <h3 className="set-sub">{t('acc.showCode')}</h3>
      <button type="button" className="glass" title={t('tip.acc.showCode')} onClick={() => show.mutate()}>
        {t('acc.showCode')}
      </button>
      {show.isError ? (
        <p className="live-note" role="alert">
          {errorText(show.error, t)}
        </p>
      ) : null}
      {offer ? (
        <div className="offer">
          <p>
            {t('acc.code')}: <strong className="data offer-code">{offer.code}</strong>
          </p>
          <p className="muted">{t('acc.validFor', { n: Math.round(offer.expires_in / 60) })}</p>
          <p>
            {t('acc.link')}: <span className="data">{`${window.location.origin}/#/pair/${offer.code}`}</span>
          </p>
          <img className="offer-qr" alt="QR" src={`data:image/svg+xml;utf8,${encodeURIComponent(offer.qr_svg)}`} />
        </div>
      ) : null}

      <h3 className="set-sub">{t('acc.devices')}</h3>
      {devices.data && devices.data.length === 0 ? <p className="muted">{t('acc.noDevices')}</p> : null}
      <ul className="macro-list">
        {(devices.data ?? []).map((d) => (
          <li key={d.id}>
            <span className="data">{d.name}</span>
            <button type="button" className="glass" title={t('tip.acc.revoke')} aria-label={`${t('acc.revoke')} ${d.name}`} onClick={() => revoke.mutate(d.id)}>
              {t('acc.revoke')}
            </button>
          </li>
        ))}
      </ul>
    </>
  );
}
