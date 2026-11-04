import { useEffect, useRef, useState, type ReactNode } from 'react';
import { getCallsign, type CallsignInfo } from '../api/client';
import { flagCode } from '../lib/flags';
import { useT } from '../i18n';

/** Le ricerche gia' fatte (o in corso): lo stesso nominativo non si chiede due volte finche' la pagina resta aperta. */
const cache = new Map<string, Promise<CallsignInfo | null>>();
/** Le risposte gia' arrivate: il riquadro di questi nominativi si apre subito, senza attesa. */
const ready = new Map<string, CallsignInfo | null>();

/** Solo per le prove. */
export function clearCallsignCache(): void {
  cache.clear();
  ready.clear();
}

export function lookupCallsign(call: string): Promise<CallsignInfo | null> {
  const key = call.toUpperCase();
  let p = cache.get(key);
  if (!p) {
    p = getCallsign(key)
      .catch(() => null)
      .then((v) => {
        ready.set(key, v);
        return v;
      });
    cache.set(key, p);
  }
  return p;
}

/** Nominativi in coda per il precaricamento: uno alla volta, senza ripetere quelli gia' cercati. */
const queue: string[] = [];
let draining = false;
/** Pausa fra due ricerche di precaricamento, per non tempestare QRZ.com o HamQTH. */
const PREFETCH_GAP_MS = 250;

/** Precarica in sottofondo le informazioni dei nominativi in vista: al passaggio del mouse il riquadro e' gia' pronto. */
export function prefetchCallsigns(calls: readonly string[]): void {
  for (const c of calls) {
    const key = c.trim().toUpperCase();
    if (key !== '' && !cache.has(key) && !queue.includes(key)) queue.push(key);
  }
  if (draining) return;
  draining = true;
  const next = () => {
    const call = queue.shift();
    if (!call) {
      draining = false;
      return;
    }
    if (cache.has(call)) {
      next();
      return;
    }
    void lookupCallsign(call).finally(() => setTimeout(next, PREFETCH_GAP_MS));
  };
  next();
}

/** Attesa prima della ricerca: passando veloci sulla tabella non parte niente. */
export const TIP_DELAY_MS = 350;

/**
 * Un nominativo con il suo riquadro al passaggio del mouse: bandiera, paese e continente (da `cty.dat`) e, se le ricerche sono accese,
 * nome e citta' da QRZ.com o HamQTH. La ricerca parte solo dopo un breve ritardo e si ricorda.
 */
export function CallsignTip({ call, children }: { call: string; children: ReactNode }) {
  if (call.trim() === '') return <>{children}</>;
  return <CallsignTipInner call={call}>{children}</CallsignTipInner>;
}

function CallsignTipInner({ call, children }: { call: string; children: ReactNode }) {
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
    // dentro una riga dell'elenco il contenitore non ha una sua scatola (display: contents): si misura il primo figlio
    const box = getComputedStyle(e.currentTarget).display === 'contents' ? (e.currentTarget.firstElementChild ?? e.currentTarget) : e.currentTarget;
    const r = box.getBoundingClientRect();
    const target = { x: r.left, y: r.bottom + 4 };
    if (timer.current) clearTimeout(timer.current);
    // gia' precaricato: si mostra subito
    const known = ready.get(call.toUpperCase());
    if (known !== undefined) {
      setPos(target);
      setInfo(known);
      setOpen(true);
      return;
    }
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
              {person ? <QslBadges person={person} long /> : null}
            </>
          )}
        </span>
      ) : null}
    </span>
  );
}

type Person = NonNullable<CallsignInfo['person']>;

/**
 * Come la stazione conferma i QSO (da QRZ.com o HamQTH): LoTW, eQSL, QSL diretta e il QSL manager. Acceso = si', grigio = no, assente = il
 * servizio non lo dice. Corto (L E D) nei Comandi, per esteso nel riquadro del nominativo.
 */
export function QslBadges({ person, long = false }: { person: Person; long?: boolean }) {
  const t = useT();
  const items = [
    { key: 'L', label: t('qsl.lotw'), on: person.lotw },
    { key: 'E', label: t('qsl.eqsl'), on: person.eqsl },
    { key: 'D', label: t('qsl.direct'), on: person.direct },
  ].filter((i) => i.on === true || i.on === false);
  const via = person.qsl_via?.trim() ?? '';
  if (items.length === 0 && via === '') return null;
  return (
    <span className={`qsl-badges${long ? ' qsl-badges--long' : ''}`}>
      {items.map((i) => (
        <span key={i.key} className={`qsl-badge${i.on ? ' qsl-badge--on' : ''}`} title={t(i.on ? 'qsl.yes' : 'qsl.no', { what: i.label })}>
          {long ? `${i.on ? '✔' : '✖'} ${i.label}` : i.key}
        </span>
      ))}
      {via !== '' ? (
        <span className="qsl-via data" title={t('qsl.via', { call: via })}>
          {long ? t('qsl.via', { call: via }) : `via ${via}`}
        </span>
      ) : null}
    </span>
  );
}

/** Le lettere L E D della stazione che si sta lavorando: si cerca da sola (con la memoria delle ricerche gia' fatte). */
export function DxQsl({ call }: { call: string }) {
  const [person, setPerson] = useState<Person | null>(null);
  useEffect(() => {
    let alive = true;
    setPerson(null);
    if (call.trim() === '') return;
    void lookupCallsign(call).then((v) => {
      if (alive) setPerson(v?.person ?? null);
    });
    return () => {
      alive = false;
    };
  }, [call]);
  return person ? <QslBadges person={person} /> : null;
}
