import { useCallback, useEffect, useRef, useState } from 'react';
import { deleteWefaxImage, postWefaxRotate, postWefaxSlant, wefaxImageUrl, type WefaxImageEntry, type WefaxSlantPoints } from '../../api/client';
import { useT } from '../../i18n';
import { askConfirm } from '../ConfirmHost';
import { SlantOverlay } from './SlantOverlay';
import '../../styles/wefax-ruota.css';

function when(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}Z`;
}

/** La data corta per la miniatura (UTC): «21/11 09:30». */
function short(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getUTCDate())}/${pad(d.getUTCMonth() + 1)} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;
}

/** La cronologia delle cartine ricevute: miniature (le piu' nuove prima), apri, scarica il PNG, cancella. */
export function History({ images, onChanged }: { images: WefaxImageEntry[]; onChanged: () => void }) {
  const t = useT();
  const [open, setOpen] = useState<WefaxImageEntry | null>(null);
  // una cartina sostituita (ruotata) ha lo stesso indirizzo: il numero fa ricaricare la miniatura
  const [bumps, setBumps] = useState<Record<string, number>>({});
  const thumb = (id: string) => `${wefaxImageUrl(id)}${bumps[id] ? `?v=${bumps[id]}` : ''}`;
  const sorted = [...images].sort((a, b) => b.time_ms - a.time_ms);
  // le miniature sono quadrate in una riga sola che scorre: due pulsanti ‹ › portano a sinistra e a destra di una pagina alla volta
  const strip = useRef<HTMLUListElement | null>(null);
  const [edge, setEdge] = useState({ left: false, right: false });
  const measure = useCallback(() => {
    const el = strip.current;
    if (!el) return;
    setEdge({ left: el.scrollLeft > 2, right: el.scrollLeft + el.clientWidth < el.scrollWidth - 2 });
  }, []);
  useEffect(() => {
    measure();
    const el = strip.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [measure, sorted.length]);
  const slide = (dir: -1 | 1) => {
    const el = strip.current;
    if (el) el.scrollBy({ left: dir * Math.max(80, el.clientWidth * 0.85), behavior: 'smooth' });
  };

  const remove = async (img: WefaxImageEntry) => {
    if (!(await askConfirm(t('wefax.hist.confirm')))) return;
    await deleteWefaxImage(img.id).catch(() => undefined);
    setOpen((o) => (o?.id === img.id ? null : o));
    onChanged();
  };

  return (
    <section className="sstv-history wefax-history" aria-label={t('wefax.hist.title')}>
      <div className="sstv-hist-head">
        {t('wefax.hist.title')}
        <span className="sstv-count">{sorted.length}</span>
      </div>
      {sorted.length === 0 ? <p className="sstv-hint">{t('wefax.hist.empty')}</p> : null}
      <div className="wefax-thumbs-wrap">
      <button type="button" className="wefax-thumbs-arrow wefax-thumbs-arrow--prev" onClick={() => slide(-1)} disabled={!edge.left} title={t('wefax.hist.prev')} aria-label={t('wefax.hist.prev')}>
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
          <path d="M15 5l-7 7 7 7" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
      <ul className="wefax-thumbs" ref={strip} onScroll={measure}>
        {sorted.map((img) => (
          <li key={img.id} className="wefax-thumb" title={when(img.time_ms)}>
            {/* la cartina intera; un clic la apre in grande */}
            <button type="button" className="wefax-thumb-open" onClick={() => setOpen(img)} title={t('wefax.hist.open')} aria-label={`${t('wefax.hist.open')} ${when(img.time_ms)}`}>
              <img src={thumb(img.id)} alt="" loading="lazy" />
              {!img.complete ? <span className="sstv-thumb-tag">{t('wefax.hist.partial')}</span> : null}
            </button>
            {/* in alto a destra, sopra la miniatura: apri, scarica, cancella */}
            <span className="wefax-thumb-tools">
              <button type="button" className="glass sstv-btn wefax-thumb-icon" onClick={() => setOpen(img)} title={t('wefax.hist.open')} aria-label={`${t('wefax.hist.open')} ${when(img.time_ms)}`}>
                ⤢
              </button>
              <a className="glass sstv-btn wefax-thumb-icon" href={wefaxImageUrl(img.id)} download={img.file} title={t('wefax.hist.download')} aria-label={t('wefax.hist.download')}>
                ⬇
              </a>
              <button type="button" className="glass sstv-btn sstv-btn--danger wefax-thumb-icon" onClick={() => void remove(img)} title={t('wefax.hist.delete')} aria-label={t('wefax.hist.delete')}>
                ✕
              </button>
            </span>
            {/* in basso a sinistra: la data */}
            <span className="wefax-thumb-date">{short(img.time_ms)}</span>
          </li>
        ))}
      </ul>
      <button type="button" className="wefax-thumbs-arrow wefax-thumbs-arrow--next" onClick={() => slide(1)} disabled={!edge.right} title={t('wefax.hist.next')} aria-label={t('wefax.hist.next')}>
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
          <path d="M9 5l7 7-7 7" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
      </div>
      {open ? (
        <Lightbox
          entry={open}
          onClose={() => setOpen(null)}
          onDelete={() => void remove(open)}
          onFixed={(e) => {
            // raddrizzata: la nuova copia (o la stessa cartina sostituita) si vede subito, con l'elenco aggiornato
            setOpen(e);
            setBumps((b) => ({ ...b, [e.id]: (b[e.id] ?? 0) + 1 }));
            onChanged();
          }}
        />
      ) : null}
    </section>
  );
}

/** La cartina aperta: adattata alla larghezza della finestra oppure a pixel reali (si scorre nel riquadro). */
function Lightbox({ entry, onClose, onDelete, onFixed }: { entry: WefaxImageEntry; onClose: () => void; onDelete: () => void; onFixed: (e: WefaxImageEntry) => void }) {
  const t = useT();
  const [actual, setActual] = useState(false);
  // misura dell'inclinazione a due clic sulla cartina salvata: si raddrizza e si salva come nuova copia (o si sostituisce, dopo conferma)
  const [measuring, setMeasuring] = useState(false);
  const [replace, setReplace] = useState(false);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  // una cartina sostituita ha lo stesso indirizzo: il numero fa ricaricare l'immagine
  const [version, setVersion] = useState(0);
  const src = `${wefaxImageUrl(entry.id)}${version > 0 ? `?v=${version}` : ''}`;
  // ruota di 90 gradi e salva subito, al posto dell'originale (nessuna conferma: ogni clic gira ancora di un quarto)
  const rotate = async (dir: 'left' | 'right') => {
    setMeasuring(false);
    setBusy(true);
    try {
      const e = await postWefaxRotate(entry.id, dir);
      setNote(null);
      setVersion((v) => v + 1);
      onFixed(e);
    } catch {
      setNote({ ok: false, text: t('wefax.rotate.error') });
    } finally {
      setBusy(false);
    }
  };
  const measure = async (p: WefaxSlantPoints) => {
    setMeasuring(false);
    if (replace && !(await askConfirm(t('wefax.slant.replaceAsk')))) return;
    setBusy(true);
    try {
      const r = await postWefaxSlant({ points: p, id: entry.id, replace });
      setNote({ ok: true, text: t(replace ? 'wefax.slant.replaced' : 'wefax.slant.copied', { ppm: String(Math.round(r.ppm)) }) });
      if (r.image) {
        setVersion((v) => v + 1);
        onFixed(r.image);
      }
    } catch {
      setNote({ ok: false, text: t('wefax.slant.error') });
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [onClose]);
  return (
    <div className="sstv-lightbox wefax-lightbox" role="dialog" aria-modal="true" aria-label={when(entry.time_ms)} onClick={onClose}>
      <figure onClick={(e) => e.stopPropagation()}>
        <div className={`wefax-lb-scroll${actual ? ' wefax-lb-scroll--actual' : ''}`}>
          <div className="wefax-lb-sheet">
            <img src={src} alt="" width={entry.width} height={entry.height} className={actual ? 'wefax-lb-img wefax-lb-img--actual' : 'wefax-lb-img'} />
            {measuring ? <SlantOverlay width={entry.width} height={entry.height} onMeasure={(p) => void measure(p)} onCancel={() => setMeasuring(false)} /> : null}
          </div>
        </div>
        {note ? (
          <p className={`wefax-slant-note${note.ok ? '' : ' wefax-slant-note--bad'}`} role="status">
            {note.text}
          </p>
        ) : null}
        <figcaption>
          <span>
            <b>{when(entry.time_ms)}</b> · IOC {entry.ioc} · {entry.width}×{entry.height}
          </span>
          <span className="sstv-lightbox-actions">
            <span className="sstv-views" role="group">
              <button type="button" className={actual ? '' : 'on'} aria-pressed={!actual} onClick={() => setActual(false)} title={t('wefax.view.fitHint')}>
                {t('wefax.view.fit')}
              </button>
              <button type="button" className={actual ? 'on' : ''} aria-pressed={actual} onClick={() => setActual(true)} title={t('wefax.view.actualHint')}>
                {t('wefax.view.actual')}
              </button>
            </span>
            <span className="wefax-rotate" role="group" aria-label={t('wefax.rotate.group')}>
              <button type="button" className="glass sstv-btn wefax-rotate-btn" disabled={busy} title={t('wefax.rotate.left')} aria-label={t('wefax.rotate.left')} onClick={() => void rotate('left')}>
                <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                  <path d="M9 4L5 8l4 4M5 8h8a6 6 0 1 1-6 6" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </button>
              <button type="button" className="glass sstv-btn wefax-rotate-btn" disabled={busy} title={t('wefax.rotate.right')} aria-label={t('wefax.rotate.right')} onClick={() => void rotate('right')}>
                <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                  <path d="M15 4l4 4-4 4M19 8h-8a6 6 0 1 0 6 6" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </button>
            </span>
            <button type="button" className={`glass sstv-btn${measuring ? ' sstv-btn--primary' : ''}`} disabled={busy} aria-pressed={measuring} title={t('wefax.slant.measureTip')} onClick={() => setMeasuring((m) => !m)}>
              {t('wefax.slant.measure')}
            </button>
            <label className="sstv-check" title={t('wefax.slant.replaceTip')}>
              <input type="checkbox" checked={replace} onChange={(e) => setReplace(e.target.checked)} />
              {t('wefax.slant.replace')}
            </label>
            <a className="glass sstv-btn sstv-btn--primary" href={src} download={entry.file}>
              {t('wefax.hist.download')}
            </a>
            <button type="button" className="glass sstv-btn sstv-btn--danger" onClick={onDelete}>
              {t('wefax.hist.delete')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={onClose}>
              {t('wefax.hist.close')}
            </button>
          </span>
        </figcaption>
      </figure>
    </div>
  );
}
