import { useEffect, useState } from 'react';
import { deleteWefaxImage, wefaxImageUrl, type WefaxImageEntry } from '../../api/client';
import { useT } from '../../i18n';
import { askConfirm } from '../ConfirmHost';

function when(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}Z`;
}

/** La cronologia delle cartine ricevute: miniature (le piu' nuove prima), apri, scarica il PNG, cancella. */
export function History({ images, onChanged }: { images: WefaxImageEntry[]; onChanged: () => void }) {
  const t = useT();
  const [open, setOpen] = useState<WefaxImageEntry | null>(null);
  const sorted = [...images].sort((a, b) => b.time_ms - a.time_ms);

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
      <ul className="wefax-thumbs">
        {sorted.map((img) => (
          <li key={img.id} className="wefax-thumb">
            {/* i pulsantini stanno sopra la miniatura: apri, scarica, cancella */}
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
            <button type="button" className="wefax-thumb-open" onClick={() => setOpen(img)} title={t('wefax.hist.open')} aria-label={`${t('wefax.hist.open')} ${when(img.time_ms)}`}>
              <img src={wefaxImageUrl(img.id)} alt="" loading="lazy" />
              {!img.complete ? <span className="sstv-thumb-tag">{t('wefax.hist.partial')}</span> : null}
            </button>
            {/* solo la data */}
            <span className="wefax-thumb-meta">
              <b>{when(img.time_ms)}</b>
            </span>
          </li>
        ))}
      </ul>
      {open ? <Lightbox entry={open} onClose={() => setOpen(null)} onDelete={() => void remove(open)} /> : null}
    </section>
  );
}

/** La cartina aperta: adattata alla larghezza della finestra oppure a pixel reali (si scorre nel riquadro). */
function Lightbox({ entry, onClose, onDelete }: { entry: WefaxImageEntry; onClose: () => void; onDelete: () => void }) {
  const t = useT();
  const [actual, setActual] = useState(false);
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
          <img src={wefaxImageUrl(entry.id)} alt="" width={entry.width} height={entry.height} className={actual ? 'wefax-lb-img wefax-lb-img--actual' : 'wefax-lb-img'} />
        </div>
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
            <a className="glass sstv-btn sstv-btn--primary" href={wefaxImageUrl(entry.id)} download={entry.file}>
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
