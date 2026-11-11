import { useEffect, useState } from 'react';
import { deleteSstvImage, getSstvImageBlob, sstvImageUrl, type SstvImageEntry, type SstvModeInfo } from '../../api/client';
import { useT } from '../../i18n';

function when(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}Z`;
}

/** Le due cronologie sotto l'area principale: ricevute (ultime 200) e trasmesse (ultime 20), in schede. */
export function History({ images, modes, onChanged, onReuse }: { images: SstvImageEntry[]; modes: SstvModeInfo[]; onChanged: () => void; onReuse: (entry: SstvImageEntry) => void }) {
  const t = useT();
  const [open, setOpen] = useState<SstvImageEntry | null>(null);
  const count = (d: 'rx' | 'tx') => images.filter((i) => (d === 'tx' ? i.direction === 'tx' : i.direction !== 'tx')).length;
  const modeName = (id: string) => modes.find((m) => m.id === id)?.name ?? id;

  const strip = (d: 'rx' | 'tx') => {
    const items = images.filter((i) => (d === 'tx' ? i.direction === 'tx' : i.direction !== 'tx'));
    return items.length === 0 ? (
      <p className="sstv-hint sstv-empty">{t('sstv.hist.empty')}</p>
    ) : (
      <ul className="sstv-strip">
        {items.map((img) => (
          <li key={img.id}>
            <button type="button" className="sstv-thumb" onClick={() => setOpen(img)} title={`${modeName(img.mode)} · ${when(img.time_ms)}`}>
              <img src={sstvImageUrl(img.id)} alt="" loading="lazy" width={img.width} height={img.height} />
              <span className="sstv-thumb-meta">
                <b>{img.call ?? modeName(img.mode)}</b>
                <span>
                  {img.call ? `${modeName(img.mode)} · ` : ''}
                  {when(img.time_ms)}
                </span>
              </span>
              {!img.complete ? <span className="sstv-thumb-tag">{t('sstv.img.partial')}</span> : null}
            </button>
          </li>
        ))}
      </ul>
    );
  };

  return (
    <section className="sstv-history sstv-history--both">
      {(['rx', 'tx'] as const).map((d) => (
        <div key={d} className="sstv-hist-col">
          <div className="sstv-hist-head">
            {d === 'rx' ? t('sstv.hist.received') : t('sstv.hist.sent')}
            <span className="sstv-count">{count(d)}</span>
          </div>
          {strip(d)}
        </div>
      ))}
      {open ? (
        <Lightbox
          entry={open}
          modeName={modeName(open.mode)}
          when={when(open.time_ms)}
          onClose={() => setOpen(null)}
          onDeleted={() => {
            setOpen(null);
            onChanged();
          }}
          onReuse={() => {
            onReuse(open);
            setOpen(null);
          }}
        />
      ) : null}
    </section>
  );
}

function Lightbox({ entry, modeName, when, onClose, onDeleted, onReuse }: { entry: SstvImageEntry; modeName: string; when: string; onClose: () => void; onDeleted: () => void; onReuse: () => void }) {
  const t = useT();
  const [smooth, setSmooth] = useState(false);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [onClose]);

  const save = async () => {
    const blob = await getSstvImageBlob(entry.id);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = entry.file;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 2000);
  };
  const remove = async () => {
    if (!window.confirm(t('sstv.img.deleteConfirm'))) return;
    await deleteSstvImage(entry.id);
    onDeleted();
  };

  return (
    <div className="sstv-lightbox" role="dialog" aria-modal="true" aria-label={modeName} onClick={onClose}>
      <figure onClick={(e) => e.stopPropagation()}>
        <img src={sstvImageUrl(entry.id)} alt="" className={smooth ? '' : 'sstv-crisp'} width={entry.width} height={entry.height} />
        <figcaption>
          <span>
            <b>{entry.call ?? modeName}</b> {entry.call ? `· ${modeName} ` : ''}· {entry.width}×{entry.height} · {when}
            {entry.sync_quality !== undefined ? ` · ${t('sstv.rx.sync')} ${Math.round(entry.sync_quality * 100)} %` : ''}
          </span>
          <span className="sstv-lightbox-actions">
            <label className="sstv-check">
              <input type="checkbox" checked={smooth} onChange={(e) => setSmooth(e.target.checked)} />
              {t('sstv.rx.smooth')}
            </label>
            <button type="button" className="glass sstv-btn sstv-btn--primary" onClick={onReuse}>
              {entry.direction === 'tx' ? t('sstv.img.useBg') : t('sstv.img.reply')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={() => void save()}>
              {t('sstv.img.save')}
            </button>
            <button type="button" className="glass sstv-btn sstv-btn--danger" onClick={() => void remove()}>
              {t('sstv.img.delete')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={onClose}>
              {t('sstv.img.close')}
            </button>
          </span>
        </figcaption>
      </figure>
    </div>
  );
}
