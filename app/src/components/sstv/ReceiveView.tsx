import { useEffect, useRef, useState, type ReactNode } from 'react';
import { getSstvCurrentBlob, sstvCurrentCleanUrl, sstvForce, sstvRx, type SstvSnapshot } from '../../api/client';
import { useT } from '../../i18n';
import { usePrefs } from '../../lib/prefs';
import { base64ToBytes, rgbIntoRgba } from '../../lib/sstv';
import { onSstv } from '../../lib/sstvBus';

/** Ogni quanto si ripulisce l'immagine mentre arriva (con «Pulita» scelta): la pulizia costa, non si rifa' a ogni riga. */
const CLEAN_REFRESH_MS = 4000;

/**
 * Ricezione: l'immagine in arrivo si riempie riga per riga su un canvas alla risoluzione vera del modo, ingrandito senza sfocare
 * (pixel netti; «ingrandimento morbido» a scelta nelle Impostazioni SSTV). Accanto: modo, righe, sincronismo, livello, sintonia, comandi.
 * `spectrum` sta sotto il visore, nella stessa colonna (stessi bordi); `log` e' una terza colonna a destra della scheda del modo.
 * Sopra l'immagine il nominativo letto dall'FSK ID; sotto, «Originale / Pulita» per vedere l'immagine pulita dal rumore.
 * La ricezione vive nel servizio del motore: il componente puo' smontarsi e rimontarsi (cambio di pagina) e riprende dall'immagine in corso.
 */
export function ReceiveView({
  snap,
  onChanged,
  onSaved,
  onTransmit,
  onListen,
  spectrum,
  log,
}: {
  snap: SstvSnapshot;
  onChanged: () => void;
  onSaved: () => void;
  onTransmit: () => void;
  /** L'operatore accende o spegne l'ascolto (la pagina lo tiene acceso finche' non lo spegne lui). */
  onListen?: (on: boolean) => void;
  spectrum?: ReactNode;
  log?: ReactNode;
}) {
  const t = useT();
  const canvas = useRef<HTMLCanvasElement>(null);
  const image = useRef<ImageData | null>(null);
  // un'immagine nuova arrivata prima della risposta di `rx/current`: quella risposta (vecchia) non deve coprirla
  const started = useRef(false);
  const [mode, setMode] = useState<string | null>(snap.rx.mode);
  const [rows, setRows] = useState(snap.rx.rows);
  const [size, setSize] = useState({ w: snap.rx.width || 320, h: snap.rx.height || 256 });
  const smooth = usePrefs((s) => s.prefs.sstvSmooth);
  const [note, setNote] = useState<'saved' | 'lost' | null>(null);
  const [forceMode, setForceMode] = useState(snap.settings.lock_mode ?? snap.settings.default_mode);
  const [fskCall, setFskCall] = useState<string | null>(snap.rx.last_call ?? null);
  const [source, setSource] = useState<string | null>(snap.rx.source ?? null);
  // pulizia dal rumore dell'immagine in arrivo o appena ricevuta: l'originale (e quella salvata) non cambia
  const [clean, setClean] = useState(false);
  const [order, setOrder] = useState(3);
  const [level, setLevel] = useState(0.4);
  const [applied, setApplied] = useState({ order: 3, level: 0.4 });
  const [version, setVersion] = useState(0);
  const [cleanBusy, setCleanBusy] = useState(false);
  const lastClean = useRef(0);
  const rx = snap.rx;

  // all'apertura si riprende l'immagine in corso (o l'ultima finita) dal core
  useEffect(() => {
    let gone = false;
    getSstvCurrentBlob()
      .then((blob) => createImageBitmap(blob))
      .then((bmp) => {
        if (gone || started.current || !canvas.current) return;
        const c = canvas.current;
        c.width = bmp.width;
        c.height = bmp.height;
        setSize({ w: bmp.width, h: bmp.height });
        const ctx = c.getContext('2d');
        if (!ctx) return;
        ctx.drawImage(bmp, 0, 0);
        image.current = ctx.getImageData(0, 0, bmp.width, bmp.height);
      })
      .catch(() => undefined);
    return () => {
      gone = true;
    };
  }, []);

  useEffect(() => {
    const id = window.setTimeout(() => setApplied({ order, level }), 350);
    return () => window.clearTimeout(id);
  }, [order, level]);

  useEffect(
    () =>
      onSstv((ev) => {
        if (ev.kind === 'rx_fskid' && ev.call) {
          setFskCall(ev.call);
          return;
        }
        const c = canvas.current;
        if (!c) return;
        if (ev.kind === 'rx_start' && ev.width && ev.height) {
          started.current = true;
          c.width = ev.width;
          c.height = ev.height;
          setSize({ w: ev.width, h: ev.height });
          setMode(ev.mode ?? null);
          setRows(0);
          setNote(null);
          setFskCall(null);
          setSource(ev.message ?? null);
          const ctx = c.getContext('2d');
          if (ctx) {
            ctx.fillStyle = '#000';
            ctx.fillRect(0, 0, ev.width, ev.height);
            image.current = ctx.getImageData(0, 0, ev.width, ev.height);
          }
        } else if (ev.kind === 'rx_rows' && ev.rgb && ev.width && ev.first_row !== undefined) {
          const ctx = c.getContext('2d');
          if (!ctx) return;
          // l'immagine finita puo' avere meno righe (ricezione interrotta): si adatta il canvas
          if (!image.current || image.current.width !== ev.width || (ev.height && image.current.height !== ev.height)) {
            c.width = ev.width;
            c.height = ev.height ?? c.height;
            setSize({ w: c.width, h: c.height });
            image.current = ctx.getImageData(0, 0, c.width, c.height);
          }
          rgbIntoRgba(base64ToBytes(ev.rgb), image.current.data, ev.width, ev.first_row);
          ctx.putImageData(image.current, 0, 0);
          setRows((r) => Math.max(r, ev.first_row! + (ev.rows ?? 0)));
          if (ev.mode) setMode(ev.mode);
          if (Date.now() - lastClean.current > CLEAN_REFRESH_MS) {
            lastClean.current = Date.now();
            setVersion((v) => v + 1);
          }
        } else if (ev.kind === 'rx_done') {
          setNote(ev.image_id ? 'saved' : null);
          if (ev.call) setFskCall(ev.call);
          setVersion((v) => v + 1);
          onSaved();
          onChanged();
        } else if (ev.kind === 'rx_lost') {
          setNote('lost');
          onChanged();
        } else if (ev.kind === 'rx_start') {
          onChanged();
        }
      }),
    [onChanged, onSaved],
  );

  const cleanSrc = sstvCurrentCleanUrl(applied.order, applied.level, version);
  useEffect(() => {
    if (clean) setCleanBusy(true);
  }, [clean, cleanSrc]);

  const modeName = snap.modes.find((m) => m.id === mode)?.name ?? mode ?? '—';
  const total = size.h;
  const syncPct = Math.round((rx.sync || 0) * 100);
  const level01 = Math.max(0, Math.min(100, ((rx.level_db + 60) / 60) * 100));
  const hasImage = rows > 0 || image.current !== null;
  const act = (fn: () => Promise<void>) => () => {
    fn().then(onChanged, onChanged);
  };
  const sourceText = source === 'sync' ? t('sstv.rx.bySync') : source === 'forced' ? t('sstv.rx.byHand') : source === 'vis' ? t('sstv.rx.byVis') : null;

  return (
    <section className={log ? 'sstv-rx sstv-rx--log' : 'sstv-rx'}>
      <div className="sstv-stage">
        <div className="sstv-screen" style={{ aspectRatio: `${size.w} / ${size.h}` }}>
          <canvas ref={canvas} width={size.w} height={size.h} className={smooth ? 'sstv-canvas' : 'sstv-canvas sstv-canvas--crisp'} aria-label={t('sstv.view.rx')} />
          {clean && hasImage ? (
            <img
              src={cleanSrc}
              alt={t('sstv.clean.cleaned')}
              className={`sstv-canvas sstv-canvas--clean${smooth ? '' : ' sstv-canvas--crisp'}${cleanBusy ? ' sstv-busy' : ''}`}
              onLoad={() => setCleanBusy(false)}
              onError={() => setCleanBusy(false)}
            />
          ) : null}
          {rx.receiving ? <div className="sstv-scanline" style={{ top: `${(rows / Math.max(1, total)) * 100}%` }} aria-hidden="true" /> : null}
          {!rx.receiving && rows === 0 ? <p className="sstv-screen-msg">{rx.listening ? t('sstv.rx.waiting') : t('sstv.rx.offHint')}</p> : null}
          {/* il nominativo dell'FSK ID, accanto all'immagine come nell'MMSSTV */}
          {fskCall ? (
            <span className="sstv-fskid" title={t('sstv.rx.fskIdHint')}>
              {fskCall}
            </span>
          ) : null}
        </div>
        <div className="sstv-rxbar">
          <span className="sstv-views" role="group" aria-label={t('sstv.clean.title')}>
            <button type="button" className={clean ? '' : 'on'} aria-pressed={!clean} onClick={() => setClean(false)}>
              {t('sstv.clean.original')}
            </button>
            <button type="button" className={clean ? 'on' : ''} aria-pressed={clean} disabled={!hasImage} onClick={() => setClean(true)} title={t('sstv.clean.hint')}>
              {t('sstv.clean.cleaned')}
            </button>
          </span>
          {clean ? (
            <>
              <label className="sstv-slider">
                <span>
                  {t('sstv.clean.order')} {order}
                </span>
                <input type="range" min={1} max={6} step={1} value={order} onChange={(e) => setOrder(Number(e.target.value))} />
              </label>
              <label className="sstv-slider">
                <span>
                  {t('sstv.clean.smooth')} {level.toFixed(2)}
                </span>
                <input type="range" min={0.1} max={1} step={0.05} value={level} onChange={(e) => setLevel(Number(e.target.value))} />
              </label>
            </>
          ) : null}
          {note ? (
            <span className={`sstv-note sstv-note--${note}`} role="status">
              {note === 'saved' ? t('sstv.rx.saved') : t('sstv.rx.lost')}
            </span>
          ) : null}
        </div>
        {spectrum}
      </div>

      <aside className="sstv-side">
        <dl className="sstv-readouts">
          <div>
            <dt>{t('sstv.rx.mode')}</dt>
            <dd className="sstv-big">{modeName}</dd>
            {sourceText && mode ? <dd className="sstv-source">{sourceText}</dd> : null}
          </div>
          <div>
            <dt>{t('sstv.rx.fskId')}</dt>
            <dd>
              <b className="sstv-fskid-text">{fskCall ?? '—'}</b>
            </dd>
          </div>
          <div>
            <dt>{t('sstv.rx.lines', { n: rows, total })}</dt>
            <dd>
              <span className="sstv-meter">
                <span style={{ width: `${(rows / Math.max(1, total)) * 100}%` }} />
              </span>
            </dd>
          </div>
          <div>
            <dt>{t('sstv.rx.sync')}</dt>
            <dd>
              <span className={`sstv-meter sstv-meter--${syncPct > 80 ? 'ok' : syncPct > 40 ? 'warn' : 'bad'}`}>
                <span style={{ width: `${syncPct}%` }} />
              </span>
              <b>{rx.receiving || rows > 0 ? `${syncPct} %` : '—'}</b>
            </dd>
          </div>
          <div>
            <dt>{t('sstv.rx.level')}</dt>
            <dd>
              <span className="sstv-meter">
                <span style={{ width: `${level01}%` }} />
              </span>
              <b>{rx.listening ? `${Math.round(rx.level_db)} dB` : '—'}</b>
            </dd>
          </div>
          <div>
            <dt>{t('sstv.rx.offset')}</dt>
            <dd>
              <b>{rx.receiving || rows > 0 ? `${rx.offset_hz >= 0 ? '+' : ''}${Math.round(rx.offset_hz)} Hz` : '—'}</b>
            </dd>
          </div>
        </dl>

        <div className="sstv-actions">
          {/* TX: trasmette l'immagine preparata nella pagina Trasmetti e porta li' */}
          <button
            type="button"
            className="glass sstv-btn sstv-btn--primary sstv-btn--tx"
            disabled={!snap.can_transmit || rx.receiving || snap.tx.state === 'sending'}
            title={rx.receiving ? t('sstv.rx.txWait') : t('sstv.rx.txHint')}
            onClick={onTransmit}
          >
            {t('sstv.rx.tx')}
          </button>
          {rx.listening ? (
            <button
              type="button"
              className="glass sstv-btn"
              onClick={act(() => {
                onListen?.(false);
                return sstvRx('stop');
              })}
            >
              {t('sstv.rx.stop')}
            </button>
          ) : (
            <button
              type="button"
              className="glass sstv-btn sstv-btn--primary"
              onClick={act(() => {
                onListen?.(true);
                return sstvRx('start');
              })}
            >
              {t('sstv.rx.start')}
            </button>
          )}
          <button type="button" className="glass sstv-btn" disabled={!rx.receiving} onClick={act(() => sstvRx('cancel'))}>
            {t('sstv.rx.cancel')}
          </button>
        </div>

        <div className="sstv-force">
          <label>
            <span>{t('sstv.rx.mode')}</span>
            <select value={forceMode} onChange={(e) => setForceMode(e.target.value)}>
              {snap.modes.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            className="glass sstv-btn"
            title={t('sstv.rx.forceHint')}
            onClick={act(() => {
              onListen?.(true);
              return sstvForce(forceMode);
            })}
          >
            {t('sstv.rx.force')}
          </button>
          <p className="sstv-hint">{t('sstv.rx.forceHint')}</p>
        </div>
      </aside>
      {log}
    </section>
  );
}
