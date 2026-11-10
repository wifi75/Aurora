import { useEffect, useRef, useState } from 'react';
import { getSstvCurrentBlob, sstvForce, sstvRx, type SstvSnapshot } from '../../api/client';
import { useT } from '../../i18n';
import { base64ToBytes, rgbIntoRgba } from '../../lib/sstv';
import { onSstv } from '../../lib/sstvBus';

/**
 * Ricezione: l'immagine in arrivo si riempie riga per riga su un canvas alla risoluzione vera del modo, ingrandito senza sfocare
 * (pixel netti; «ingrandimento morbido» a scelta). Accanto: modo, righe, sincronismo, livello, sintonia, comandi.
 */
export function ReceiveView({ snap, onChanged, onSaved }: { snap: SstvSnapshot; onChanged: () => void; onSaved: () => void }) {
  const t = useT();
  const canvas = useRef<HTMLCanvasElement>(null);
  const image = useRef<ImageData | null>(null);
  const [mode, setMode] = useState<string | null>(snap.rx.mode);
  const [rows, setRows] = useState(snap.rx.rows);
  const [size, setSize] = useState({ w: snap.rx.width || 320, h: snap.rx.height || 256 });
  const [smooth, setSmooth] = useState(false);
  const [note, setNote] = useState<'saved' | 'lost' | null>(null);
  const [forceMode, setForceMode] = useState(snap.settings.lock_mode ?? snap.settings.default_mode);
  const rx = snap.rx;

  // all'apertura si riprende l'immagine in corso (o l'ultima finita) dal core
  useEffect(() => {
    let gone = false;
    getSstvCurrentBlob()
      .then((blob) => createImageBitmap(blob))
      .then((bmp) => {
        if (gone || !canvas.current) return;
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

  useEffect(
    () =>
      onSstv((ev) => {
        const c = canvas.current;
        if (!c) return;
        if (ev.kind === 'rx_start' && ev.width && ev.height) {
          c.width = ev.width;
          c.height = ev.height;
          setSize({ w: ev.width, h: ev.height });
          setMode(ev.mode ?? null);
          setRows(0);
          setNote(null);
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
        } else if (ev.kind === 'rx_done') {
          setNote(ev.image_id ? 'saved' : null);
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

  const modeName = snap.modes.find((m) => m.id === mode)?.name ?? mode ?? '—';
  const total = size.h;
  const syncPct = Math.round((rx.sync || 0) * 100);
  const level = Math.max(0, Math.min(100, ((rx.level_db + 60) / 60) * 100));
  const act = (fn: () => Promise<void>) => () => {
    fn().then(onChanged, onChanged);
  };

  return (
    <section className="sstv-rx">
      <div className="sstv-stage">
        <div className="sstv-screen" style={{ aspectRatio: `${size.w} / ${size.h}` }}>
          <canvas ref={canvas} width={size.w} height={size.h} className={smooth ? 'sstv-canvas' : 'sstv-canvas sstv-canvas--crisp'} aria-label={t('sstv.view.rx')} />
          {rx.receiving ? <div className="sstv-scanline" style={{ top: `${(rows / Math.max(1, total)) * 100}%` }} aria-hidden="true" /> : null}
          {!rx.receiving && rows === 0 ? <p className="sstv-screen-msg">{rx.listening ? t('sstv.rx.waiting') : t('sstv.rx.offHint')}</p> : null}
        </div>
        {note ? (
          <p className={`sstv-note sstv-note--${note}`} role="status">
            {note === 'saved' ? t('sstv.rx.saved') : t('sstv.rx.lost')}
          </p>
        ) : null}
      </div>

      <aside className="sstv-side">
        <dl className="sstv-readouts">
          <div>
            <dt>{t('sstv.rx.mode')}</dt>
            <dd className="sstv-big">{modeName}</dd>
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
                <span style={{ width: `${level}%` }} />
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
          {rx.listening ? (
            <button type="button" className="glass sstv-btn" onClick={act(() => sstvRx('stop'))}>
              {t('sstv.rx.stop')}
            </button>
          ) : (
            <button type="button" className="glass sstv-btn sstv-btn--primary" onClick={act(() => sstvRx('start'))}>
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
          <button type="button" className="glass sstv-btn" title={t('sstv.rx.forceHint')} onClick={act(() => sstvForce(forceMode))}>
            {t('sstv.rx.force')}
          </button>
          <p className="sstv-hint">{t('sstv.rx.forceHint')}</p>
        </div>

        <label className="sstv-check">
          <input type="checkbox" checked={smooth} onChange={(e) => setSmooth(e.target.checked)} />
          {t('sstv.rx.smooth')}
        </label>
      </aside>
    </section>
  );
}
