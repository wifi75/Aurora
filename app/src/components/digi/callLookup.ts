import { useCallback, useRef } from 'react';
import { getCallsign, type CallsignInfo } from '../../api/client';
import type { Qso } from './DigiLog';

/** Una parola che puo' essere un nominativo: lettere, cifre e «/», almeno una cifra e una lettera (come nel log SSTV). */
export function isCallsign(word: string): boolean {
  const c = word.trim().toUpperCase();
  return /^[A-Z0-9/]{3,15}$/.test(c) && /\d/.test(c) && /[A-Z]/.test(c);
}

type Filled = Pick<Qso, 'name' | 'qth' | 'wwloc'> & { country: string };

/** Dalla risposta della ricerca: nome (quello con cui la stazione si fa chiamare, se c'e'), QTH (citta'), locatore e paese (cty.dat). */
export function filledFrom(info: CallsignInfo): Filled {
  const p = info.person as (CallsignInfo['person'] & { grid?: string }) | null;
  return {
    name: (p?.fname || p?.name || '').trim(),
    qth: (p?.city ?? '').trim(),
    wwloc: (p?.grid ?? '').trim().toUpperCase(),
    country: (info.dxcc?.country ?? p?.country ?? '').trim(),
  };
}

/**
 * Il nominativo preso dalla finestra di ricezione (doppio clic o menu del tasto destro) va nel QSO in corso e parte la ricerca gia'
 * configurata in Aurora (QRZ.com o HamQTH, `/callsign/{call}`): nome, QTH, locatore e paese vanno nei campi vuoti o in quelli riempiti da
 * una ricerca di prima. Quello scritto a mano dall'operatore non si tocca.
 */
export function useCallLookup(setQso: (f: (q: Qso) => Qso) => void): (call: string) => void {
  const auto = useRef<Partial<Filled>>({});
  return useCallback(
    (word: string) => {
      const call = word.trim().toUpperCase();
      if (call === '') return;
      setQso((q) => ({ ...q, call }));
      if (!isCallsign(call)) return;
      void getCallsign(call)
        .then((res) => {
          const found = filledFrom(res);
          setQso((q) => {
            if (q.call.trim().toUpperCase() !== call) return q;
            const next = { ...q };
            for (const k of ['name', 'qth', 'wwloc', 'country'] as const) {
              const now = next[k] ?? '';
              const mine = now !== '' && auto.current[k] !== now;
              if (!mine) {
                next[k] = found[k];
                auto.current[k] = found[k];
              }
            }
            return next;
          });
        })
        .catch(() => undefined);
    },
    [setQso],
  );
}
