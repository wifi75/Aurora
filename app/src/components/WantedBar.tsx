import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { getWishlist, postFrequency } from '../api/client';
import { useT, type Key } from '../i18n';
import { formatKhz } from '../lib/format';
import { ringState, soundFor } from '../lib/soundAssign';
import { playSound } from '../lib/soundBank';
import { useStation } from '../state/station';
import { useWishlist } from '../state/wishlist';

/** Dopo quanto un avviso lasciato li' si toglie da solo. */
const AUTO_HIDE_MS = 90_000;

/**
 * La lista dei desiderati in uso (per evidenziare le righe in tutta l'applicazione) e l'avviso quando una stazione voluta compare:
 * una barra in alto, il suono se gli avvisi sonori sono accesi, e la notifica del browser se la si e' consentita.
 */
export function WantedBar() {
  const t = useT();
  const banner = useStation((s) => s.wantedBanner);
  const dismiss = useStation((s) => s.dismissWanted);
  const setList = useWishlist((s) => s.set);

  const wishlist = useQuery({ queryKey: ['wishlist'], queryFn: getWishlist, retry: false, refetchInterval: 30_000 });
  useEffect(() => {
    if (wishlist.data) setList(wishlist.data.config);
  }, [wishlist.data, setList]);

  const key = banner ? `${banner.time_ms}-${banner.call}` : '';
  useEffect(() => {
    if (!banner) return;
    const st = ringState(useStation.getState().status?.transmitting ?? false);
    const sound = soundFor('wanted', st);
    if (sound) playSound(sound, st.volume);
    try {
      if (typeof Notification !== 'undefined' && Notification.permission === 'granted') {
        new Notification(`${banner.call}`, { body: banner.text, tag: `wanted-${banner.call}` });
      }
    } catch {
      // le notifiche del browser non ci sono su questo indirizzo o dispositivo
    }
    const timer = setTimeout(dismiss, AUTO_HIDE_MS);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  if (!banner) return null;
  const details = [
    banner.band,
    banner.freq_hz ? `${formatKhz(banner.freq_hz / 1000)} kHz` : null,
    banner.snr !== null && banner.snr !== undefined ? `${banner.snr} dB` : null,
    banner.country,
  ].filter((x): x is string => !!x);
  const reason = t(`wish.reason.${banner.reason}` as Key);

  return (
    <div className="wanted-bar" role="alert">
      <span className="wanted-star" aria-hidden="true">
        ★
      </span>
      <strong className="data">{t('wish.banner', { call: banner.call, reason })}</strong>
      <span className="muted">{t(`wish.source.${banner.source}` as Key)}</span>
      {banner.text ? <span className="data wanted-text">{banner.text}</span> : null}
      {details.length > 0 ? <span className="muted">{details.join(' · ')}</span> : null}
      {banner.source === 'cluster' && banner.freq_hz ? (
        <button type="button" className="glass" onClick={() => void postFrequency(banner.freq_hz as number).catch(() => undefined)}>
          {t('wish.tune')}
        </button>
      ) : null}
      <button type="button" className="glass" onClick={dismiss}>
        {t('wish.close')}
      </button>
    </div>
  );
}
