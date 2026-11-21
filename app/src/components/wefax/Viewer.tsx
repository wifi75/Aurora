import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { getWefaxCurrentBlob } from '../../api/client';
import { useT } from '../../i18n';
import { decodeGrayRows, fitScale, followScrollTop, stepZoom } from '../../lib/wefax';
import { onWefax } from '../../lib/wefaxBus';

/** Le righe del canvas si allargano a blocchi di questa misura: cambiare l'altezza del canvas lo svuota, quindi si fa di rado. */
const CHUNK_ROWS = 512;

type ZoomMode = 'fit' | number;

/**
 * Il visore della cartina: un canvas alla risoluzione vera (1809 px con IOC 576, 904 con 288), le righe nuove si disegnano in coda
 * con `putImageData` (mai l'immagine intera a ogni riga). Si mostra adattato alla larghezza (predefinito, nessuno scorrimento
 * orizzontale), a pixel reali 1:1 con scorrimento nel riquadro, o con lo zoom a gradini. Durante la ricezione segue l'ultima riga
 * (si disattiva). Il riquadro occupa al massimo l'altezza dello schermo; la ricezione vive nel motore e riprende al rientro.
 */
export function Viewer({ snapRows, listening, receiving, onSaved, onChanged }: { snapRows: number; listening: boolean; receiving: boolean; onSaved: () => void; onChanged: () => void }) {
  // lo spazio sulla riga in alto della pagina dove vanno i comandi dello zoom (se la pagina lo offre)
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  useEffect(() => {
    setSlot(document.getElementById('wefax-toolbar-slot'));
  }, []);
  const t = useT();
  const canvas = useRef<HTMLCanvasElement>(null);
  const box = useRef<HTMLDivElement>(null);
  // una cartina nuova arrivata prima della risposta di `rx/current`: quella risposta (vecchia) non deve coprirla
  const started = useRef(false);
  const [size, setSize] = useState({ w: 1809, h: CHUNK_ROWS });
  const [rows, setRows] = useState(0);
  // la cartina si vede uno a uno (pixel reali); i pulsanti − e + cambiano la scala, niente «Adatta»
  const [zoom, setZoom] = useState<ZoomMode>(1);
  const [follow, setFollow] = useState(true);
  const [avail, setAvail] = useState(0);
  const [note, setNote] = useState<'saved' | 'lost' | null>(null);

  // la larghezza disponibile segue il riquadro (finestra ridimensionata, secondo schermo, telefono)
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => setAvail(el.clientWidth);
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // all'apertura si riprende la cartina in corso (o l'ultima finita) dal core
  useEffect(() => {
    let gone = false;
    getWefaxCurrentBlob()
      .then((blob) => createImageBitmap(blob))
      .then((bmp) => {
        if (gone || started.current || !canvas.current) return;
        const c = canvas.current;
        const ctx = c.getContext('2d');
        if (!ctx) return;
        c.width = bmp.width;
        c.height = bmp.height;
        ctx.drawImage(bmp, 0, 0);
        setSize({ w: bmp.width, h: bmp.height });
        setRows(bmp.height);
      })
      .catch(() => undefined);
    return () => {
      gone = true;
    };
  }, []);

  useEffect(
    () =>
      onWefax((ev) => {
        const c = canvas.current;
        if (ev.kind === 'rx_done') {
          setNote(ev.image_id ? 'saved' : null);
          onSaved();
          onChanged();
          return;
        }
        if (ev.kind === 'ioc_auto') {
          // il tono d'inizio era dell'altro IOC: l'IOC e le colonne della lettura cambiano
          onChanged();
          return;
        }
        if (ev.kind === 'rx_lost') {
          setNote('lost');
          onChanged();
          return;
        }
        if (!c) return;
        if (ev.kind === 'rx_start' && ev.width) {
          started.current = true;
          c.width = ev.width;
          c.height = CHUNK_ROWS;
          setSize({ w: ev.width, h: CHUNK_ROWS });
          setRows(0);
          setNote(null);
          onChanged();
        } else if (ev.kind === 'rx_rows' && ev.gray && ev.width && ev.first_row !== undefined) {
          const ctx = c.getContext('2d');
          if (!ctx) return;
          started.current = true;
          const { rgba, rows: n } = decodeGrayRows(ev.gray, ev.width);
          if (n === 0) return;
          const need = ev.first_row + n;
          if (c.width !== ev.width || c.height < need) {
            // il canvas si allarga a blocchi: il disegno gia' fatto si riporta nel canvas nuovo
            const keep = document.createElement('canvas');
            keep.width = c.width;
            keep.height = c.height;
            keep.getContext('2d')?.drawImage(c, 0, 0);
            const oldW = c.width;
            const oldH = c.height;
            c.width = ev.width;
            c.height = Math.max(need, Math.ceil(need / CHUNK_ROWS) * CHUNK_ROWS, ev.height ?? 0);
            if (oldW === ev.width && oldH > 0) ctx.drawImage(keep, 0, 0);
            setSize({ w: c.width, h: c.height });
          }
          ctx.putImageData(new ImageData(rgba, ev.width, n), 0, ev.first_row);
          setRows((r) => (ev.first_row === 0 ? Math.max(n, r) : Math.max(r, need)));
        }
      }),
    [onSaved, onChanged],
  );

  const shownRows = Math.max(rows, receiving ? snapRows : 0);
  const scale = zoom === 'fit' ? fitScale(avail, size.w) : zoom;
  const pct = Math.round(scale * 100);

  // segue l'ultima riga attiva: dopo ogni aggiornamento delle righe il riquadro scorre in fondo
  useEffect(() => {
    const el = box.current;
    if (!el || !follow || !receiving) return;
    el.scrollTop = followScrollTop(shownRows, scale, el.clientHeight);
  }, [shownRows, scale, follow, receiving]);

  const pixelated = scale >= 1;
  // i comandi dello zoom stanno sulla riga in alto della pagina (nello spazio `wefax-toolbar-slot`), se c'e'; altrimenti sopra il visore
  const toolbar = (
    <div className="wefax-toolbar" role="toolbar" aria-label={t('wefax.view.label')}>
        <button type="button" className="glass sstv-btn" onClick={() => setZoom(stepZoom(scale, -1))} title={t('wefax.view.zoomOut')} aria-label={t('wefax.view.zoomOut')}>
          −
        </button>
        <output className="wefax-zoom" aria-live="off">
          {pct} %
        </output>
        <button type="button" className="glass sstv-btn" onClick={() => setZoom(stepZoom(scale, 1))} title={t('wefax.view.zoomIn')} aria-label={t('wefax.view.zoomIn')}>
          +
        </button>
        <label className="sstv-check wefax-follow">
          <input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} />
          {t('wefax.view.follow')}
        </label>
        {note ? (
          <span className={`sstv-note sstv-note--${note}`} role="status">
            {note === 'saved' ? t('wefax.note.saved') : t('wefax.note.lost')}
          </span>
        ) : null}
      </div>
  );
  return (
    <div className="wefax-viewer">
      {slot ? createPortal(toolbar, slot) : toolbar}
      <div className="wefax-box" ref={box} data-testid="wefax-box">
        <div className="wefax-sheet" style={{ width: Math.round(size.w * scale), height: Math.max(1, Math.round(shownRows * scale)) }}>
          <canvas
            ref={canvas}
            width={size.w}
            height={size.h}
            className={`wefax-canvas${pixelated ? ' wefax-canvas--crisp' : ''}`}
            style={{ width: Math.round(size.w * scale), height: Math.round(size.h * scale) }}
            aria-label={t('wefax.view.label')}
          />
        </div>
        {shownRows === 0 ? <p className="wefax-msg">{listening ? t('wefax.rx.waiting') : t('wefax.rx.offHint')}</p> : null}
      </div>
    </div>
  );
}
