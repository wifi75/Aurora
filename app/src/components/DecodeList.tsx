import { foxCalls, lineSender } from '../lib/fox';
import { Fragment, useEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { getDxcc } from '../api/client';
import type { DecodedMessage, DecodeTag } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { flagCode } from '../lib/flags';
import { formatClock } from '../lib/format';
import { bearingDeg, distanceKm, gridCenter, gridOfMessage } from '../lib/geo';
import { isCq, isFilterHighlight, useFilters } from '../lib/filters';
import { useNotifyView, workedLook } from '../lib/notifyView';
import { useNotify } from '../lib/notify';
import { isAnswerToMyCall, isOngoingDxReply, transmittedTo } from '../lib/answered';
import { isCallToMe, isClosingMessage } from '../lib/mycall';
import { useStation, type CallHighlight } from '../state/station';
import { senderOf, wishReason } from '../lib/wishlist';
import { useWishlist } from '../state/wishlist';
import { formatDistance, usePrefs } from '../lib/prefs';

/** Etichetta e colore di ogni tipo di messaggio; il CQ semplice ha solo il colore. */
const BADGES: Partial<Record<DecodeTag, { label: Key; color: string }>> = {
  new_dxcc: { label: 'tag.new_dxcc', color: 'new-dxcc' },
  new_dxcc_band: { label: 'tag.new_dxcc_band', color: 'new-dxcc-band' },
  new_zone: { label: 'tag.new_zone', color: 'new-zone' },
  new_zone_band: { label: 'tag.new_zone_band', color: 'new-zone-band' },
  new_itu: { label: 'tag.new_itu', color: 'new-itu' },
  new_itu_band: { label: 'tag.new_itu_band', color: 'new-itu-band' },
  new_grid: { label: 'tag.new_grid', color: 'new-grid' },
  new_grid_band: { label: 'tag.new_grid_band', color: 'new-grid-band' },
  new_px: { label: 'tag.new_px', color: 'new-px' },
  new_px_band: { label: 'tag.new_px_band', color: 'new-px-band' },
  new_call: { label: 'tag.new_call', color: 'new-call' },
  new_call_band: { label: 'tag.new_call_band', color: 'new-call-band' },
  worked_call: { label: 'tag.worked_call', color: 'worked-call' },
  auto: { label: 'tag.auto', color: 'auto' },
};

/** Colore della barra accanto al messaggio: quello dell'etichetta, o dei CQ. */
function accentOf(tag: DecodeTag | null | undefined): string | null {
  if (tag === 'cq') return 'cq';
  return tag ? (BADGES[tag]?.color ?? null) : null;
}

function namesCall(text: string, call: string): boolean {
  const wanted = call.trim().toUpperCase();
  return wanted !== '' && text.toUpperCase().split(/\s+/).includes(wanted);
}

/** I nominativi nelle parole di un messaggio, senza le parentesi angolari dei nominativi composti (`<PJ4/K1ABC>`). */
function callWords(text: string): string[] {
  return text.toUpperCase().split(/\s+/).map((w) => w.replace(/^<|>$/g, ''));
}

/** Il colore chiesto via UDP per una riga: il primo nominativo evidenziato nel messaggio; con «solo l'ultima» solo la riga piu' recente che lo nomina. */
export function highlightOf(text: string, id: number, highlights: Record<string, CallHighlight>, newest: Map<string, number>): CallHighlight | null {
  for (const word of callWords(text)) {
    const h = highlights[word];
    if (!h) continue;
    if (h.lastOnly && newest.get(word) !== id) continue;
    return h;
  }
  return null;
}

/** Il testo del messaggio, con il nominativo chiamato e il suo locatore (se si evidenziano) in rilievo. */
/** Il tooltip della riga: lo stato della stazione e, sotto, il suggerimento del tasto destro. */
function rowTitle(state: string | undefined, hint: string | undefined): string | undefined {
  if (state && hint) return `${state}\n${hint}`;
  return state ?? hint;
}

function highlightCall(text: string, call: string, grid: string, answered?: { call: string; title: string }): ReactNode {
  const wanted = call.trim().toUpperCase();
  const answeredCall = answered?.call.trim().toUpperCase() ?? '';
  const wantedGrid = grid.trim().toUpperCase().slice(0, 4);
  if (wanted === '' && wantedGrid.length < 4 && answeredCall === '') return text;
  return text.split(/(\s+)/).map((part, i) => {
    const up = part.toUpperCase();
    // chi mi ha risposto: etichetta viola con il suo tooltip (vale anche se non e' la stazione DX in corso)
    if (answeredCall !== '' && up === answeredCall) {
      return (
        <mark key={i} className="dx-call dx-call--answered" title={answered?.title}>
          {part}
        </mark>
      );
    }
    if (wanted !== '' && up === wanted) {
      return (
        <mark key={i} className="dx-call">
          {part}
        </mark>
      );
    }
    if (wantedGrid.length === 4 && up.slice(0, 4) === wantedGrid && /^[A-R]{2}\d{2}([A-X]{2})?$/.test(up)) {
      return (
        <mark key={i} className="dx-call dx-grid">
          {part}
        </mark>
      );
    }
    return part;
  });
}

/**
 * Il locatore di ogni stazione visto in un messaggio precedente: «JA2ORW S21BDK +07» non ha il locatore, ma S21BDK l'aveva mandato
 * nel suo CQ. Si ricorda per tutta la sessione (al massimo 5000 stazioni, le piu' vecchie escono).
 */
const KNOWN_GRIDS = new Map<string, string>();
/** I nominativi gia' chiesti al file dei prefissi (anche se senza risposta): non si richiedono. */
const DXCC_ASKED = new Set<string>();
const KNOWN_MAX = 5000;

export function rememberGrid(call: string, grid: string): void {
  const key = call.toUpperCase();
  KNOWN_GRIDS.delete(key);
  KNOWN_GRIDS.set(key, grid);
  if (KNOWN_GRIDS.size > KNOWN_MAX) {
    const oldest = KNOWN_GRIDS.keys().next().value;
    if (oldest !== undefined) KNOWN_GRIDS.delete(oldest);
  }
}

/** Il locatore della stazione che manda il messaggio: quello nel messaggio o, se manca, quello gia' visto. */
export function gridOfSender(text: string): string | null {
  const call = senderOf(text)?.call;
  const grid = gridOfMessage(text);
  if (grid) {
    if (call) rememberGrid(call, grid);
    return grid;
  }
  return call ? (KNOWN_GRIDS.get(call.toUpperCase()) ?? null) : null;
}

function distanceOf(mine: { lat: number; lon: number }, text: string): number | null {
  const grid = gridOfSender(text);
  const theirs = grid ? gridCenter(grid) : null;
  return theirs ? distanceKm(mine, theirs) : null;
}

/** L'azimut verso la stazione, in gradi, se si conosce il suo locatore. */
function bearingOf(mine: { lat: number; lon: number }, text: string): number | null {
  const grid = gridOfSender(text);
  const theirs = grid ? gridCenter(grid) : null;
  return theirs ? Math.round(bearingDeg(mine, theirs)) % 360 : null;
}

interface Props {
  decodes: DecodedMessage[];
  onReply(decodeId: number): void;
  /** Il proprio locatore, per mostrare la distanza di ogni stazione. */
  myGrid?: string;
  /** Il proprio nominativo: i messaggi rivolti a me si evidenziano forte, come nel vecchio programma. */
  myCall?: string | null;
  /** Stazione che si sta chiamando: la sua riga e il suo nominativo si evidenziano. */
  dxCall?: string | null;
  /** Il locatore di quella stazione: si evidenzia nei messaggi se la scheda Notifiche lo chiede. */
  dxGrid?: string | null;
  /** Testo dello stato vuoto (predefinito: invito all'ascolto). */
  emptyText?: string;
  /** Tasto destro su una stazione: la mette in coda per il prossimo QSO. */
  onQueue?: (id: number) => void;
  /** Telefono: un tocco sulla riga risponde subito (lo stesso del doppio clic sul PC). */
  tapReply?: boolean;
  /** Telefono: il nome del paese anche come suggerimento (la colonna e' stretta). Bandiera e paese seguono comunque le preferenze. */
  forceFlag?: boolean;
}

export function DecodeList({ decodes, onReply, onQueue, myGrid, myCall, dxCall, dxGrid, emptyText, tapReply = false, forceFlag = false }: Props) {
  // telefono: un tocco chiama la stazione; un secondo tocco sulla stessa riga subito dopo (o un doppio tocco) non richiama,
  // altrimenti la sequenza ripartiva a ogni richiesta e la trasmissione non partiva
  const lastTap = useRef<{ id: number; at: number } | null>(null);
  const tapOnce = (id: number) => {
    const now = Date.now();
    if (lastTap.current && lastTap.current.id === id && now - lastTap.current.at < 2000) return;
    lastTap.current = { id, at: now };
    setSelected(id);
    onReply(id);
  };
  const t = useT();
  // in JT65 e JT9 si decodifica a fine minuto: lo si dice invece di lasciare una pagina vuota che sembra rotta
  const mode = useStation((st) => st.status?.mode ?? '');
  const prefs = usePrefs((s) => s.prefs);
  const notify = useNotify((s) => s.notify);
  const wishlist = useWishlist((s) => s.list);
  const view = useNotifyView((s) => s.view);
  // nominativi, prefissi e paesi da evidenziare (Impostazioni → Filtri)
  const filters = useFilters((s) => s.filters);
  const look = workedLook(view);
  // i nominativi a cui ho trasmesso da poco: un loro messaggio a me e' una risposta alla mia chiamata
  const txLog = useStation((st) => st.txLog);
  const sentTo = transmittedTo(txLog);
  // le stazioni in coda per il prossimo QSO
  const queueCalls = (useStation((st) => st.status?.queue) ?? []).map((c) => c.toUpperCase());
  // la sigla «in coda» va solo sulla riga piu' recente di ogni stazione in coda, non su tutte le sue righe
  const queueRow = new Map<string, number>();
  for (const d of decodes) {
    const call = d.id >= 0 ? senderOf(d.text)?.call.toUpperCase() : undefined;
    if (call && queueCalls.includes(call) && !queueRow.has(call)) queueRow.set(call, d.id);
  }
  // i colori chiesti dai programmi collegati via UDP: per «solo l'ultima» serve la riga piu' recente di ogni nominativo (l'elenco e' dal piu' recente)
  const highlights = useStation((st) => st.highlights);
  const newestRow = new Map<string, number>();
  if (Object.keys(highlights).length > 0) {
    for (const d of decodes) {
      if (d.id < 0) continue;
      for (const word of callWords(d.text)) if (highlights[word] && !newestRow.has(word)) newestRow.set(word, d.id);
    }
  }
  // i locatori di tutte le righe prima di disegnarle: la riga piu' recente senza locatore usa quello di un CQ piu' vecchio
  for (const d of decodes) if (d.id >= 0) gridOfSender(d.text);
  // il paese di ogni stazione sentita: le nostre righe Tx mostrano quello della stazione chiamata, non il nostro
  const countryOfCall = new Map<string, string>();
  for (const d of decodes) {
    const c = d.id >= 0 && d.country ? senderOf(d.text)?.call.toUpperCase() : undefined;
    if (c && d.country && !countryOfCall.has(c)) countryOfCall.set(c, d.country);
  }
  // le stazioni chiamate che non sono nell'elenco: il paese si chiede al file dei prefissi del core (una volta per nominativo)
  const [lookedUp, setLookedUp] = useState<Record<string, string>>({});
  const missing = [...new Set(decodes.filter((d) => d.id < 0).map((d) => (d.text.split(/\s+/)[0] ?? '').replace(/[<>]/g, '').toUpperCase()))].filter(
    (c) => c.length >= 3 && !countryOfCall.has(c) && !(c in lookedUp) && !DXCC_ASKED.has(c),
  );
  useEffect(() => {
    for (const call of missing) {
      DXCC_ASKED.add(call);
      getDxcc(call)
        .then((r) => setLookedUp((old) => ({ ...old, [call]: r.country })))
        .catch(() => undefined);
    }
  }, [missing.join(' ')]);
  for (const [c, country] of Object.entries(lookedUp)) if (!countryOfCall.has(c)) countryOfCall.set(c, country);
  const mine = myGrid ? gridCenter(myGrid) : null;
  // un clic seleziona la riga; trasmettere richiede il doppio clic (come nel motore)
  const [selected, setSelected] = useState<number | null>(null);
  // il periodo della riga mostrata prima: serve alla «riga vuota tra i periodi»
  let previousTime: number | null = null;
  if (decodes.length === 0) {
    return (
      <section className="decode-list decode-list--empty" aria-label={t('decode.aria')}>
        <p>{emptyText ?? (mode.toUpperCase().startsWith('JT') ? t('decode.emptyJt', { mode }) : t('decode.empty'))}</p>
      </section>
    );
  }
  const foxSet = foxCalls(decodes);
  return (
    <section className={`decode-list${prefs.showDt ? '' : ' decode-list--no-dt'}${view.invert ? ' decode-list--invert' : ''}`} aria-label={t('decode.aria')} title={t('tip.panel.all')}>
      {decodes.map((d) => {
        // le stazioni che trasmettono in Fox/Hound di WSJT-X (righe «A RR73; B <FOX> -08»): etichetta FOX e sfondo proprio su tutte le loro righe
        const fox = d.id >= 0 && foxSet.has(lineSender(d.text) ?? '');
        const own = d.id < 0;
        // «Visualizza i tuoi messaggi Tx nella finestra Rx» spento: le righe Tx non si vedono
        if (own && !prefs.showTx) return null;
        const toMe = !own && isCallToMe(d.text, myCall);
        // un messaggio a me di una stazione a cui avevo trasmesso da poco e' la sua RISPOSTA (linguetta verde); altrimenti mi sta chiamando per prima (rossa)
        const talker = own ? null : senderOf(d.text);
        // risposta alla mia chiamata: gli avevo trasmesso da poco, oppure e' la stazione che sto lavorando adesso e le avevo gia' trasmesso
        // (il suo primissimo messaggio, prima di averle risposto, resta rosso: mi ha chiamato lei per prima)
        const answered =
          toMe && (isAnswerToMyCall(d.text, d.time_ms, sentTo) || (!!dxCall && !!talker && talker.call.toUpperCase() === dxCall.trim().toUpperCase() && isOngoingDxReply(d.text, d.time_ms, sentTo)));
        // il posto in coda (1 = la prossima): la sigla sta nella colonna delle etichette o accanto al paese, mai sopra il testo del messaggio
        const queuePos = talker ? queueCalls.indexOf(talker.call.toUpperCase()) + 1 : 0;
        const inQueue = queuePos > 0 && queueRow.get(talker?.call.toUpperCase() ?? '') === d.id;
        // un pallino col posto in coda prima del testo: si vede sempre, anche quando la colonna delle etichette ne ha gia' una
        const queueMark = inQueue ? (
          <span className="queue-dot" title={t('tip.row.queuedPos', { n: queuePos })} aria-label={t('tip.row.queuedPos', { n: queuePos })}>
            {queuePos}
          </span>
        ) : null;
        // una stazione della lista dei desiderati: si evidenzia anche se l'avviso e' gia' scattato
        const sender = own || !wishlist.enabled ? null : senderOf(d.text);
        const wanted = sender !== null && wishReason(wishlist, sender.call, sender.grid, d.country) !== null;
        // «gia' lavorato» come nel motore: barrato (o sottolineato, o colorato); «non mostrare» lo nasconde, salvo i messaggi a me e i desiderati
        const worked = !own && d.tag === 'worked_call';
        const picked = !own && isFilterHighlight(d, filters);
        // «non mostrare» nasconde i CQ delle stazioni gia' collegate, non i loro scambi con altri
        if (worked && view.workedDontShow && !toMe && !wanted && /^CQ\s/i.test(d.text.trim())) return null;
        // un tipo con l'evidenziazione spenta si mostra come un messaggio qualunque; un messaggio tra altre stazioni senza «altri messaggi predefiniti» non ha marcatori
        const between = !own && !toMe && !isCq(d.text);
        const lit = !own && d.tag && notify.highlight[d.tag] && !(between && !view.otherMarker) ? d.tag : null;
        const badge = lit ? BADGES[lit] : undefined;
        // il verde dei gia' lavorati compare solo con «Evidenzia gia' lavorati»; di base il messaggio e' barrato e senza colore
        const quiet = worked && !look.color;
        const accent = quiet ? null : accentOf(lit);
        // riga nostra: la stazione e' quella a cui trasmettiamo (il primo nominativo, anche tra < >); il locatore nel messaggio e' il nostro e non conta
        const target = own ? (d.text.split(/\s+/)[0] ?? '').replace(/[<>]/g, '').toUpperCase() : '';
        const targetGrid = own ? (KNOWN_GRIDS.get(target) ?? KNOWN_GRIDS.get(target.split('/').sort((a, b) => b.length - a.length)[0] ?? '') ?? null) : null;
        const country = own ? (countryOfCall.get(target) ?? null) : d.country;
        const theirs = own ? (targetGrid ? gridCenter(targetGrid) : null) : null;
        const km = prefs.showDistance && mine ? (own ? (theirs ? distanceKm(mine, theirs) : null) : distanceOf(mine, d.text)) : null;
        const azimuth = prefs.showAzimuth && mine ? (own ? (theirs ? Math.round(bearingDeg(mine, theirs)) % 360 : null) : bearingOf(mine, d.text)) : null;
        const code = prefs.showFlag ? flagCode(country) : null;
        const gap = prefs.insertBlank && previousTime !== null && previousTime !== d.time_ms;
        previousTime = d.time_ms;
        // «Evidenzia i messaggi con il nominativo DX»: spento, la stazione chiamata non si distingue dagli altri messaggi
        const named = view.highlightDxCall && dxCall && namesCall(d.text, dxCall);
        // «sfondo diverso per il mio nominativo nel messaggio»: ciano o giallo, come nel motore
        const myBackground = toMe && view.differentBackground ? (view.diffYellow ? ' decode-row--me-yellow' : ' decode-row--me-cyan') : '';
        const workedClass = worked ? ` decode-row--worked${look.strike ? ' decode-row--strike' : ''}${look.underline ? ' decode-row--underline' : ''}${look.color ? ' decode-row--worked-color' : ''}` : '';
        // il colore chiesto da un programma collegato (Highlight Callsign) vince sui colori normali
        const hl = own ? null : highlightOf(d.text, d.id, highlights, newestRow);
        const hlClass = hl ? `${hl.background ? ' decode-row--hl-bg' : ''}${hl.foreground ? ' decode-row--hl-fg' : ''}` : '';
        const rowStyle: Record<string, string> = {};
        if (accent) rowStyle['--row-accent'] = `var(--tag-${accent})`;
        if (hl?.background) rowStyle['--hl-bg'] = hl.background;
        if (hl?.foreground) rowStyle['--hl-fg'] = hl.foreground;
        return (
          <Fragment key={d.id}>
          {gap ? <div className="decode-gap" role="separator" /> : null}
          <button
            type="button"
            title={rowTitle(own ? t('tip.row.tx') : inQueue ? t('tip.row.queued') : answered ? t('tip.row.answered') : toMe ? t('tip.row.me') : wanted ? t('tip.row.wanted') : picked ? t('tip.row.picked') : worked ? t('tip.row.worked') : undefined, !own && !!talker && !!onQueue ? t('tip.row.queue') : undefined)}
            className={`decode-row${own ? ' decode-row--tx' : ''}${toMe ? ' decode-row--me' : ''}${answered ? ' decode-row--answered' : ''}${wanted ? ' decode-row--wanted' : ''}${picked ? ' decode-row--picked' : ''}${workedClass}${myBackground}${named ? ' decode-row--dx' : ''}${selected === d.id ? ' decode-row--selected' : ''}${inQueue ? ' decode-row--queued' : ''}${fox ? ' decode-row--fox' : ''}${hlClass}`}
            style={Object.keys(rowStyle).length > 0 ? (rowStyle as CSSProperties) : undefined}
            onClick={() => (tapReply && !own ? tapOnce(d.id) : setSelected(d.id))}
            onDoubleClick={() => (tapReply ? undefined : own || !prefs.doubleClickReply ? setSelected(d.id) : onReply(d.id))}
            onContextMenu={(e) => {
              // tasto destro: in coda (non sulle mie righe Tx ne' sui messaggi senza una stazione da chiamare)
              if (own || !talker || !onQueue) return;
              e.preventDefault();
              setSelected(d.id);
              onQueue(d.id);
            }}
          >
            <span className="c-time num muted">{formatClock(d.time_ms)}</span>
            <span className="c-snr num">{own ? 'Tx' : d.snr}</span>
            {prefs.showDt ? <span className="c-dt num muted">{own ? '' : d.delta_time.toFixed(1)}</span> : null}
            <span className="c-freq num muted">{d.freq_hz}</span>
            <span className="decode-text">
              {queueMark}
              <span className="data">{view.highlightDxCall || view.highlightDxGrid || answered ? highlightCall(d.text, view.highlightDxCall ? (dxCall ?? '') : '', view.highlightDxGrid ? (dxGrid ?? '') : '', answered && talker ? { call: talker.call, title: t('tip.row.answeredCall') } : undefined) : d.text}</span>
            </span>
            <span className="decode-where">
              <span className="w-flag">{code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}</span>
              <span className="w-country">
                {prefs.showCountry && country ? <span className="decode-country" title={forceFlag ? country : undefined}>{country}</span> : null}
              </span>
              <span className="w-km num muted">
                {km !== null ? formatDistance(km, prefs.distanceUnit) : ''}
                {azimuth !== null ? `${km !== null ? ' ' : ''}${azimuth}°` : ''}
              </span>
            </span>
            {fox ? (
              <span className="badge badge--tag badge-cell badge--fox" title={t('tip.row.fox')}>
                FOX
              </span>
            ) : badge ? (
              <span
                className={`badge badge--tag badge-cell${quiet ? ' badge--quiet' : ''}`}
                title={t(`kind.${lit}` as Key)}
                style={quiet ? undefined : { background: `var(--tag-${badge.color})`, color: `var(--tag-${badge.color}-ink)` }}
              >
                {t(badge.label)}
              </span>
            ) : (
              <span className="badge-cell" />
            )}
            {toMe && view.redMarker && !isClosingMessage(d.text) ? (
              <span className={`me-mark${answered ? ' me-mark--answer' : view.blueMarker && dxCall && namesCall(d.text, dxCall) ? ' me-mark--blue' : ''}`} aria-hidden="true" />
            ) : null}
          </button>
          </Fragment>
        );
      })}
    </section>
  );
}
