import { useEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { getCallsign, type CallsignInfo } from '../api/client';
import { flagCode } from '../lib/flags';
import { useT } from '../i18n';
import { lotwDateToShow, useLotwView } from '../lib/lotw';
import { bearingDeg, distanceKm, gridCenter } from '../lib/geo';
import { formatDistance, usePrefs } from '../lib/prefs';
import { useStatusPick } from '../state/station';

/**
 * Le ricerche gia' fatte (o in corso) e fino a quando valgono (`until`, ms): lo stesso nominativo non si chiede due volte finche' la
 * risposta vale. Una risposta con l'ultimo invio a LoTW vale finche' la pagina resta aperta; senza, si rinfresca dopo qualche minuto
 * (l'elenco LoTW dell'ARRL il motore lo scarica dopo l'avvio); una ricerca fallita si ritenta poco dopo.
 */
const cache = new Map<string, { p: Promise<CallsignInfo | null>; until: number }>();
/** Le risposte gia' arrivate (mai gli errori): il riquadro di questi nominativi si apre subito, senza attesa. */
const ready = new Map<string, CallsignInfo>();
/** Una risposta senza `lotw_last_upload` si richiede dopo questo tempo. */
const REFRESH_NO_LOTW_MS = 5 * 60_000;
/** Una ricerca fallita non si conserva: si ritenta dopo questa pausa (non subito, per non tempestare il motore col precaricamento). */
const RETRY_FAILED_MS = 30_000;
/**
 * Quanti nominativi si ricordano al massimo: con la pagina aperta per giorni il precaricamento ne cerca migliaia e la memoria cresceva
 * senza fine. Oltre, si dimenticano i meno recenti (se ripassano sotto il mouse si richiedono).
 */
const MAX_CACHED = 3000;

/** Tiene `cache` e `ready` entro `MAX_CACHED`, togliendo i nominativi cercati per primi. */
function trimCache(): void {
  for (const key of cache.keys()) {
    if (cache.size <= MAX_CACHED) break;
    cache.delete(key);
    ready.delete(key);
  }
  for (const key of ready.keys()) {
    if (ready.size <= MAX_CACHED) break;
    ready.delete(key);
  }
}

/** La ricerca di questo nominativo e' in corso o la sua risposta vale ancora. */
function fresh(key: string): boolean {
  const entry = cache.get(key);
  return entry !== undefined && Date.now() < entry.until;
}

/** Chi vuole sapere quando arriva una risposta (lo stato USA per «nuovo stato»). */
export const readyListeners = new Set<() => void>();

/** La risposta gia' arrivata per un nominativo, senza attendere (undefined = non ancora cercato). */
export function knownCallsign(call: string): CallsignInfo | null | undefined {
  return ready.get(call.toUpperCase());
}

/** Solo per le prove. */
export function clearCallsignCache(): void {
  cache.clear();
  ready.clear();
}

/** Solo per le prove: quanti nominativi si ricordano. */
export function callsignCacheSize(): { cache: number; ready: number } {
  return { cache: cache.size, ready: ready.size };
}

export function lookupCallsign(call: string): Promise<CallsignInfo | null> {
  const key = call.toUpperCase();
  const known = cache.get(key);
  if (known && Date.now() < known.until) return known.p;
  // in corso: vale finche' non arriva la risposta
  const entry: { p: Promise<CallsignInfo | null>; until: number } = { p: Promise.resolve(null), until: Number.POSITIVE_INFINITY };
  entry.p = getCallsign(key)
    .catch(() => null)
    .then((v) => {
      if (!v) {
        // l'errore non resta: si ritenta fra poco, e intanto vale la risposta buona di prima (se c'era)
        entry.until = Date.now() + RETRY_FAILED_MS;
        return ready.get(key) ?? null;
      }
      entry.until = v.lotw_last_upload ? Number.POSITIVE_INFINITY : Date.now() + REFRESH_NO_LOTW_MS;
      // dimenticata mentre era in corso (oltre `MAX_CACHED`): la risposta vale per chi l'aspettava ma non si conserva
      if (cache.get(key) === entry) ready.set(key, v);
      for (const f of readyListeners) f();
      return v;
    });
  // in fondo all'elenco: il piu' recente e' l'ultimo a essere dimenticato
  cache.delete(key);
  cache.set(key, entry);
  trimCache();
  return entry.p;
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
    if (key !== '' && !fresh(key) && !queue.includes(key)) queue.push(key);
  }
  if (draining) return;
  draining = true;
  const next = () => {
    const call = queue.shift();
    if (!call) {
      draining = false;
      return;
    }
    if (fresh(call)) {
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
export function CallsignTip({ call, lotwDate, grid, children }: { call: string; /** Ultimo invio a LoTW dall'elenco pubblico dell'ARRL (gia' filtrato dalle preferenze), se la stazione c'e'. */ lotwDate?: string | null; /** Il locatore della stazione, se si conosce gia' (dal messaggio); se manca si usa quello della ricerca. */ grid?: string | null; children: ReactNode }) {
  if (call.trim() === '') return <>{children}</>;
  return <CallsignTipInner call={call} lotwDate={lotwDate ?? null} grid={grid ?? null}>{children}</CallsignTipInner>;
}

function CallsignTipInner({ call, lotwDate, grid, children }: { call: string; lotwDate: string | null; grid: string | null; children: ReactNode }) {
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
    // gia' precaricato: si mostra subito (e, se la risposta e' da rinfrescare, la nuova la sostituisce appena arriva)
    const key = call.toUpperCase();
    const known = ready.get(key);
    if (known !== undefined) {
      setPos(target);
      setInfo(known);
      setOpen(true);
      if (!fresh(key)) {
        void lookupCallsign(call).then((v) => {
          if (alive.current && v) setInfo(v);
        });
      }
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
  return (
    <span className="calltip-anchor" onMouseEnter={enter} onMouseLeave={leave} onFocus={() => undefined}>
      {children}
      {/* nel corpo della pagina: dentro gli elenchi (cluster, Attivita' di banda) il riquadro finiva sotto le righe e si leggeva in filigrana */}
      {open ? createPortal(<TipBox call={call} info={info} lotwDate={lotwDate} grid={grid} pos={pos} />, document.body) : null}
    </span>
  );
}

/** Il riquadro aperto: montato solo quando si vede, cosi' i nominativi chiusi (centinaia nelle tabelle) non seguono le preferenze LoTW. */
function TipBox({ call, info, lotwDate, grid, pos }: { call: string; info: CallsignInfo | null | undefined; lotwDate: string | null; grid: string | null; pos: { x: number; y: number } }) {
  const t = useT();
  const myGrid = useStatusPick(['de_grid'])?.de_grid ?? '';
  const unit = usePrefs((s) => s.prefs.distanceUnit);
  // direzione per l'antenna: sempre, con il locatore del messaggio o, se manca, quello della ricerca e il nostro
  const mine = myGrid ? gridCenter(myGrid) : null;
  const theirs = grid || info?.person?.grid ? gridCenter((grid || info?.person?.grid) ?? '') : null;
  const heading = mine && theirs ? { az: Math.round(bearingDeg(mine, theirs)) % 360, km: distanceKm(mine, theirs) } : null;
  const country = info?.dxcc?.country ?? info?.person?.country ?? null;
  const code = flagCode(country);
  const person = info?.person;
  // LoTW dall'elenco dell'ARRL: quello della decodifica o, se manca (DX Cluster), quello che il motore da' con la ricerca del nominativo,
  // filtrato dalle stesse preferenze (mesi) della finestra Rx
  const lotwView = useLotwView((s) => s.view);
  const arrlDate = lotwDate ?? lotwDateToShow(info?.lotw_last_upload ?? null, lotwView, new Date());
  const fullName = person ? [person.fname, person.name].filter(Boolean).join(' ') : '';
  return (
    <span className="calltip" role="tooltip" style={{ left: pos.x, top: pos.y }} data-testid="calltip">
      <strong className="calltip-call data">{call.toUpperCase()}</strong>
      {info === undefined ? (
        <span className="muted">{t('calltip.loading')}</span>
      ) : !country && !person ? (
        arrlDate ? <QslBadges person={{ lotw: true }} long /> : <span className="muted">{t('calltip.unknown')}</span>
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
          {heading ? <span className="calltip-bearing data">{t('calltip.bearing', { az: heading.az, dist: formatDistance(heading.km, unit) })}</span> : null}
          {/* LoTW una volta sola: dal servizio (QRZ.com/HamQTH) o, se la stazione non l'ha indicato li', dall'elenco pubblico dell'ARRL */}
          {person || arrlDate ? <QslBadges person={{ ...(person ?? {}), lotw: person?.lotw === true || arrlDate !== null }} long /> : null}
        </>
      )}
    </span>
  );
}

type Person = NonNullable<CallsignInfo['person']>;

/**
 * Come la stazione conferma i QSO (da QRZ.com o HamQTH): LoTW, eQSL, QSL diretta e il QSL manager. Acceso = si', grigio = no, assente = il
 * servizio non lo dice. LoTW compare solo se la stazione lo usa (mai grigio). Corto (L E D) nei Comandi, per esteso nel riquadro del nominativo.
 */
export function QslBadges({ person, long = false }: { person: Partial<Person>; long?: boolean }) {
  const t = useT();
  const items = [
    { key: 'L', label: t('qsl.lotw'), on: person.lotw === true ? true : undefined },
    { key: 'E', label: t('qsl.eqsl'), on: person.eqsl },
    { key: 'D', label: t('qsl.direct'), on: person.direct },
    { key: 'B', label: t('qsl.bureau'), on: person.bureau },
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

/**
 * Un testo intero al passaggio del mouse, nel riquadro con lo stile di quello del nominativo (nero nei temi scuri, bianco nel tema chiaro):
 * per le celle troppo strette, come i Commenti del DX Cluster. Mostra solo quel testo e copre tutta la cella.
 */
export function TextTip({ text, children }: { text: string; children: ReactNode }) {
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );
  if (text.trim() === '') return <>{children}</>;
  const enter = (e: React.MouseEvent<HTMLElement>) => {
    // sempre, su tutta la cella: nelle tabelle la misura del testo tagliato non e' affidabile e il riquadro non compariva
    const box = e.currentTarget.closest('td') ?? e.currentTarget;
    const r = box.getBoundingClientRect();
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setPos({ x: r.left, y: r.bottom + 4 }), TIP_DELAY_MS);
  };
  const leave = () => {
    if (timer.current) clearTimeout(timer.current);
    setPos(null);
  };
  return (
    <span className="texttip-anchor" onMouseEnter={enter} onMouseLeave={leave}>
      {children}
      {pos
        ? createPortal(
            <span className="calltip calltip--text" role="tooltip" style={{ left: pos.x, top: pos.y }}>
              {text}
            </span>,
            document.body,
          )
        : null}
    </span>
  );
}
