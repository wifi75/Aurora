import { useEffect, useMemo, useRef, useState } from 'react';
import type { DecodedMessage } from '../api/schemas';
import { useT } from '../i18n';
import { flagCode } from '../lib/flags';
import { useNotifyView } from '../lib/notifyView';
import { playAlert } from '../lib/voice';
import { useNewDxPos } from '../lib/newDxPos';
import { ringState, soundFor } from '../lib/soundAssign';
import { senderOf } from '../lib/wishlist';
import { useStation } from '../state/station';

/** Dopo quanto senza riceverla di nuovo la stazione esce dall'avviso. */
const KEEP_MS = 10 * 60_000;
/** Quante stazioni al massimo nell'avviso. */
const MAX = 4;

/** L'ora UTC del giorno (hh:mm:ss) di una decodifica. */
/** Le stazioni gia' mostrate e quelle chiuse: restano finche' Aurora e' aperto, anche cambiando pagina. */
const SEEN = new Map<string, number>();
const HIDDEN = new Set<string>();

export function utcTime(timeMs: number): string {
  const sec = Math.floor((((timeMs % 86_400_000) + 86_400_000) % 86_400_000) / 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${p(Math.floor(sec / 3600))}:${p(Math.floor((sec % 3600) / 60))}:${p(sec % 60)}`;
}

interface Hit {
  call: string;
  country: string | null;
  band: boolean;
  /** La decodifica piu' recente della stazione (per l'ora e per rispondere). */
  last: DecodedMessage;
  /** Quando e' entrata nell'avviso (ora del PC), per non risuonare. */
  since: number;
}

/**
 * Avviso in evidenza per un DXCC nuovo (o nuovo sulla banda): una scheda che lampeggia in alto a destra con bandiera, nominativo, paese e l'ora
 * dell'ultima ricezione, che si aggiorna a ogni nuova trasmissione della stazione; un suono suo quando la stazione compare. Doppio clic: si
 * risponde. Si accende e si spegne in Impostazioni > Notifiche.
 */
export function NewDxAlert({ onReply }: { onReply(decodeId: number): void }) {
  const t = useT();
  const decodes = useStation((s) => s.decodes);
  const transmitting = useStation((s) => s.status?.transmitting ?? false);
  const popNew = useNotifyView((s) => s.view.popNew);
  const pos = useNewDxPos();
  const popNewBand = useNotifyView((s) => s.view.popNewBand);
  // chiusa con la ×: non torna finche' Aurora resta aperta
  // una volta sola per stazione: chi e' gia' stato mostrato non ricompare tornando sulla pagina Stazione da un'altra pagina
  const [hidden, setHiddenState] = useState<ReadonlySet<string>>(() => new Set([...HIDDEN, ...SEEN.keys()]));
  const setHidden = (f: (x: ReadonlySet<string>) => ReadonlySet<string>) => setHiddenState((x) => {
    const next = f(x);
    next.forEach((c) => HIDDEN.add(c));
    return next;
  });
  const seen = useRef<Map<string, number>>(SEEN);

  const hits = useMemo(() => {
    if (!popNew && !popNewBand) return [] as Hit[];
    const wanted = new Map<string, Hit>();
    // le decodifiche sono dalla piu' recente: la prima di ogni stazione e' la sua ultima ricezione
    const lastOf = new Map<string, DecodedMessage>();
    for (const d of decodes) {
      if (d.id < 0) continue;
      const call = senderOf(d.text)?.call.toUpperCase();
      if (!call) continue;
      if (!lastOf.has(call)) lastOf.set(call, d);
      const band = d.tag === 'new_dxcc_band';
      if (!wanted.has(call) && ((d.tag === 'new_dxcc' && popNew) || (band && popNewBand))) {
        wanted.set(call, { call, country: d.country ?? null, band, last: d, since: seen.current.get(call) ?? Date.now() });
      }
    }
    const out: Hit[] = [];
    for (const hit of wanted.values()) {
      const last = lastOf.get(hit.call) ?? hit.last;
      out.push({ ...hit, last });
    }
    return out.sort((a, b) => b.last.time_ms - a.last.time_ms);
  }, [decodes, popNew, popNewBand]);

  // il suono solo quando una stazione compare (non a ogni sua nuova trasmissione); di nuovo solo dopo che e' uscita dall'avviso
  useEffect(() => {
    let fresh: Hit | null = null;
    for (const hit of hits) {
      if (!seen.current.has(hit.call)) {
        seen.current.set(hit.call, Date.now());
        fresh ??= hit;
      }
    }
    if (!fresh) return;
    const st = ringState(transmitting);
    const id = soundFor('new_dx_popup', st);
    if (id) playAlert(id, st.volume, { event: fresh.band ? 'new_dxcc_band' : 'new_dx_popup', call: fresh.call });
  }, [hits, transmitting]);

  // resta finche' la stazione si riceve: esce 10 minuti dopo la sua ultima ricezione (rispetto alla decodifica piu' recente)
  const newest = decodes.find((d) => d.id >= 0)?.time_ms ?? 0;
  const shown = hits.filter((h) => newest - h.last.time_ms < KEEP_MS && !hidden.has(h.call)).slice(0, MAX);
  if (shown.length === 0) return null;
  return (
    <div className={`newdx newdx--${pos}`} role="alert" aria-live="assertive">
      {shown.map((h) => {
        const code = flagCode(h.country ?? undefined);
        return (
          <div key={h.call} className={`newdx-card${h.band ? ' newdx-card--band' : ''}`} title={t('newdx.tip')} onDoubleClick={() => onReply(h.last.id)}>
            <span className="newdx-flag">{code ? <span className={`fi fi-${code}`} aria-hidden="true" /> : null}</span>
            <span className="newdx-body">
              <span className="newdx-kind">{t(h.band ? 'newdx.band' : 'newdx.dxcc')}</span>
              <span className="newdx-call">{h.call}</span>
              <span className="newdx-country">{h.country ?? '—'}</span>
              <span className="newdx-last">
                {t('newdx.last')} <b>{utcTime(h.last.time_ms)}</b> UTC · {h.last.snr} dB
              </span>
            </span>
            <button type="button" className="newdx-close" aria-label={t('newdx.close')} title={t('newdx.close')} onClick={() => setHidden((x) => new Set([...x, h.call]))}>
              ×
            </button>
          </div>
        );
      })}
    </div>
  );
}
