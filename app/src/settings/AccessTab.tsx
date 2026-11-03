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
import { Actions, Cell, Cells, Group, Message, Page } from './layout';
import { useAutosave } from './useAutosave';

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
  // la durata dei codici si salva da sola quando e' nei limiti
  useAutosave(minutes !== null && inRange ? secs : null, minutes !== null, (v) => saveTtl.mutate(v), (v) => saveTtl.mutate(v), 900);

  return (
    <Page>
      <Group title={t('acc.open')}>
        <Cells min={12}>
          <Check label={t('acc.open')} hint={t('tip.acc.open')} checked={access.data?.open ?? false} onChange={(v) => toggle.mutate(v)} />
        </Cells>
        <Help>{t('acc.openWarn')}</Help>
        {toggle.isError ? <Message kind="alert">{errorText(toggle.error, t)}</Message> : null}
      </Group>

      <Group title={t('acc.home')}>
        {access.data?.mdns ? (
          <p title={t('tip.acc.home')}>
            <strong className="data">{access.data.mdns}</strong>
          </p>
        ) : (
          <Message kind="muted">{t('acc.noHome')}</Message>
        )}
        <Help>{t('tip.acc.home')}</Help>
      </Group>

      <Group title={t('acc.ttl')}>
        <Cells min={12}>
          <Cell label={t('acc.ttl')} hint={t('tip.acc.ttl')}>
            <input
              type="number"
              className="ttl-input"
              aria-label={t('acc.ttl')}
              min={1}
              max={60}
              value={shownMinutes}
              onChange={(e) => setMinutes(e.target.value)}
            />
          </Cell>
        </Cells>
        <Actions>
          <button title={t('tip.ui.save')} type="button" className="glass glass--on" disabled={minutes === null || !inRange} onClick={() => saveTtl.mutate(secs)}>
            {t('set.save')}
          </button>
        </Actions>
        {saveTtl.isSuccess && minutes === null ? <Message>{t('acc.ttlSaved')}</Message> : null}
        {saveTtl.isError ? <Message kind="alert">{errorText(saveTtl.error, t)}</Message> : null}
      </Group>

      <Group title={t('acc.showCode')} rows={2}>
        <Actions>
          <button type="button" className="glass" title={t('tip.acc.showCode')} onClick={() => show.mutate()}>
            {t('acc.showCode')}
          </button>
        </Actions>
        {show.isError ? <Message kind="alert">{errorText(show.error, t)}</Message> : null}
        {offer ? (
          <div className="offer">
            <p>
              {t('acc.code')}: <strong className="data offer-code">{offer.code}</strong>
            </p>
            <p className="muted">{t('acc.validFor', { n: Math.round(offer.expires_in / 60) })}</p>
            <img className="offer-qr" alt="QR" src={`data:image/svg+xml;utf8,${encodeURIComponent(offer.qr_svg)}`} />
            <p className="muted">{t('acc.qrHint')}</p>
            <p>
              {t('acc.link')}: <span className="data">{offer.link ?? `${window.location.origin}/#/pair/${offer.code}`}</span>
            </p>
            {offer.addresses && offer.addresses.length > 0 ? (
              <p>
                {t('acc.addresses')}: <span className="data">{offer.addresses.join(' · ')}</span>
              </p>
            ) : null}
          </div>
        ) : null}
      </Group>

      <Group title={t('acc.preview')} span={2}>
        <LivePreviews />
      </Group>

      <Group title={t('acc.devices')} span={2}>
        {devices.data && devices.data.length === 0 ? <Message kind="muted">{t('acc.noDevices')}</Message> : null}
        <ul className="sx-list">
          {(devices.data ?? []).map((d) => (
            <li key={d.id}>
              <span className="data">{d.name}</span>
              <button type="button" className="glass" title={t('tip.acc.revoke')} aria-label={`${t('acc.revoke')} ${d.name}`} onClick={() => revoke.mutate(d.id)}>
                {t('acc.revoke')}
              </button>
            </li>
          ))}
        </ul>
      </Group>
    </Page>
  );
}

/** Una cornice a forma di telefono o tablet con dentro la stessa Aurora (stessa origine), rimpicciolita. */
function Device({ kind, width, height, scale, label }: { kind: 'phone' | 'tablet'; width: number; height: number; scale: number; label: string }) {
  const vista = kind === 'phone' ? 'telefono' : 'tablet';
  return (
    <figure className={`live-device live-device--${kind}`}>
      <div className="live-device-frame" style={{ width: width * scale, height: height * scale }}>
        <iframe
          title={label}
          src={`/?vista=${vista}#/`}
          loading="lazy"
          style={{ width, height, transform: `scale(${scale})`, transformOrigin: '0 0' }}
        />
      </div>
      <figcaption className="muted">{`${label} · ${width}×${height}`}</figcaption>
    </figure>
  );
}

/** Le anteprime dal vivo: l'app che gira adesso, con i dati veri, come la vedono il telefono e il tablet in orizzontale. */
function LivePreviews() {
  const t = useT();
  return (
    <div className="live-previews" title={t('tip.acc.preview')}>
      <p className="live-previews-badge">{t('acc.preview')}</p>
      <div className="live-previews-row">
        <Device kind="phone" width={390} height={844} scale={0.5} label={t('acc.previewPhone')} />
        <Device kind="tablet" width={1180} height={820} scale={0.42} label={t('acc.previewTablet')} />
      </div>
    </div>
  );
}
