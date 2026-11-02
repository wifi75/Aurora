import { useEffect, useRef, useState, type ReactNode } from 'react';
import { getCallsign, type CallsignInfo } from '../api/client';
import { flagCode } from '../lib/flags';
import { useT } from '../i18n';

/** Le ricerche gia' fatte (o in corso): lo stesso nominativo non si chiede due volte finche' la pagina resta aperta. */
const cache = new Map<string, Promise<CallsignInfo | null>>();

/** Solo per le prove. */
export function clearCallsignCache(): void {
  cache.clear();
}

export function lookupCallsign(call: string): Promise<CallsignInfo | null> {
  const key = call.toUpperCase();
  let p = cache.get(key);
  if (!p) {
    p = getCallsign(key).catch(() => null);
    cache.set(key, p);
  }
  return p;
}

/** Attesa prima della ricerca: passando veloci sulla tabella non parte niente. */
export const TIP_DELAY_MS = 350;

/**
 * Un nominativo con il suo riquadro al passaggio del mouse: bandiera, paese e continente (da `cty.dat`) e, se le ricerche sono accese,
 * nome e citta' da QRZ.com o HamQTH. La ricerca parte solo dopo un breve ritardo e si ricorda.
 */
export function CallsignTip({ call, children }: { call: string; children: ReactNode }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [info, setInfo] = useState<CallsignInfo | null | undefined>(undefined);
  const [pos, setPos] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const alive = useRef(true);
  useEffect(
    () => () => {
      alive.current = false;
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );
  const enter = (e: React.MouseEvent<HTMLElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    const target = { x: r.left, y: r.bottom + 4 };
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      setPos(target);
      setOpen(true);
      void lookupCallsign(call).then((v) => {
        if (alive.current) setInfo(v);
      });
    }, TIP_DELAY_MS);
  };
  const leave = () => {
    if (timer.current) clearTimeout(timer.current);
    setOpen(false);
  };
  const country = info?.dxcc?.country ?? info?.person?.country ?? null;
  const code = flagCode(country);
  const person = info?.person;
  const fullName = person ? [person.fname, person.name].filter(Boolean).join(' ') : '';
  return (
    <span className="calltip-anchor" onMouseEnter={enter} onMouseLeave={leave} onFocus={() => undefined}>
      {children}
      {open ? (
        <span className="calltip" role="tooltip" style={{ left: pos.x, top: pos.y }} data-testid="calltip">
          <strong className="calltip-call data">{call.toUpperCase()}</strong>
          {info === undefined ? (
            <span className="muted">{t('calltip.loading')}</span>
          ) : !country && !person ? (
            <span className="muted">{t('calltip.unknown')}</span>
          ) : (
            <>
              {country ? (
                <span className="calltip-country">
                  {code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null} {country}
                  {info?.dxcc?.continent ? <span className="muted"> · {info.dxcc.continent}</span> : null}
                </span>
              ) : null}
              {fullName ? <span className="calltip-name">{fullName}</span> : null}
              {person?.city ? <span className="calltip-city">{person.city}</span> : null}
            </>
          )}
        </span>
      ) : null}
    </span>
  );
}
