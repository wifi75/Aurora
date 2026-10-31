import { useQuery } from '@tanstack/react-query';
import { getClock, type ClockState } from '../api/client';
import { useT, type T } from '../i18n';
import { clockInTolerance } from '../lib/clock';

/** Oltre questo tempo senza una verifica riuscita l'orologio si segnala anche se l'ultima misura era buona. */
export const CLOCK_STALE_MS = 24 * 60 * 60_000;

/** Il testo dell'avviso, o `null` se l'orologio va bene (niente da mostrare). */
export function clockAlert(state: ClockState | undefined, now: number, t: T): string | null {
  if (!state) return null;
  const warn = state.config.warn_ms;
  const offset = state.last?.offset_ms ?? null;
  if (offset !== null && !clockInTolerance(offset, warn)) return t('clock.alert.off', { ms: Math.round(Math.abs(offset)) });
  const good = state.last_good_ms ?? null;
  if (good === null) {
    // finche' il primo controllo non c'e' stato non si allarma; se c'e' stato ed e' fallito, si'
    return state.last && state.last.offset_ms === null ? t('clock.alert.never') : null;
  }
  const age = now - good;
  if (age <= CLOCK_STALE_MS) return null;
  const hours = Math.floor(age / 3_600_000);
  const days = Math.floor(hours / 24);
  return t('clock.alert.stale', { days: days >= 2 ? t('clock.alert.days', { n: days }) : t('clock.alert.hours', { n: hours }) });
}

/**
 * Nella barra di stato non c'e' piu' niente finche' l'orologio va bene: compare un avviso solo se l'ora e' fuori tolleranza o non si e'
 * potuta verificare da piu' di un giorno (anche tra un avvio e l'altro). Un clic apre le impostazioni dell'orologio, con le prove NTP.
 */
export function ClockPill() {
  const t = useT();
  const clock = useQuery({ queryKey: ['clock'], queryFn: getClock, retry: false, refetchInterval: 60_000 });
  const text = clockAlert(clock.data, Date.now(), t);
  if (!text) return null;
  return (
    <a className="pill pill--warn clock-alert" href="#/impostazioni/clock" title={t('tip.clock.alert')}>
      ⚠ {text}
    </a>
  );
}
