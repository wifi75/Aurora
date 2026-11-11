import { useEffect, useState } from 'react';
import { deleteSstvImage, deleteSstvImages, getSstvImageBlob, getSstvImageCleanBlob, sstvImageCleanUrl, sstvImageUrl, type SstvImageEntry, type SstvModeInfo } from '../../api/client';
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
  // selezione per cancellare: una colonna alla volta, piu' immagini insieme
  const [selecting, setSelecting] = useState<'rx' | 'tx' | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const toggle = (id: string) =>
    setPicked((p) => {
      const n = new Set(p);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });
  const stopSelecting = () => {
    setSelecting(null);
    setPicked(new Set());
  };
  const removePicked = async () => {
    if (picked.size === 0 || !window.confirm(t('sstv.sel.confirmImages', { n: picked.size }))) return;
    await deleteSstvImages([...picked]);
    stopSelecting();
    onChanged();
  };
  const count = (d: 'rx' | 'tx') => images.filter((i) => (d === 'tx' ? i.direction === 'tx' : i.direction !== 'tx')).length;
  const modeName = (id: string) => modes.find((m) => m.id === id)?.name ?? id;

  const strip = (d: 'rx' | 'tx') => {
    const items = images.filter((i) => (d === 'tx' ? i.direction === 'tx' : i.direction !== 'tx'));
    return (
      <ul className="sstv-strip">
        {items.map((img) => (
          <li key={img.id}>
            <button
              type="button"
              className={`sstv-thumb${selecting === d ? ' sstv-thumb--select' : ''}${picked.has(img.id) ? ' sstv-thumb--picked' : ''}`}
              onClick={() => (selecting === d ? toggle(img.id) : setOpen(img))}
              aria-pressed={selecting === d ? picked.has(img.id) : undefined}
              title={`${modeName(img.mode)} · ${when(img.time_ms)}`}
            >
              {selecting === d ? <span className="sstv-tick" aria-hidden="true">{picked.has(img.id) ? '✓' : ''}</span> : null}
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
        {Array.from({ length: Math.max(0, 6 - items.length) }, (_, i) => (
          <li key={`vuoto${i}`} className="sstv-slot" aria-hidden="true" />
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
            <span className="sstv-hist-tools">
              {selecting === d ? (
                <>
                  <button type="button" className="glass sstv-btn" onClick={() => setPicked(new Set(images.filter((i) => (d === 'tx' ? i.direction === 'tx' : i.direction !== 'tx')).map((i) => i.id)))}>
                    {t('sstv.sel.all')}
                  </button>
                  <button type="button" className="glass sstv-btn sstv-btn--danger" disabled={picked.size === 0} onClick={() => void removePicked()}>
                    {t('sstv.sel.delete', { n: picked.size })}
                  </button>
                  <button type="button" className="glass sstv-btn" onClick={stopSelecting}>
                    {t('sstv.sel.done')}
                  </button>
                </>
              ) : (
                <button type="button" className="glass sstv-btn" disabled={count(d) === 0} onClick={() => { setSelecting(d); setPicked(new Set()); }}>
                  {t('sstv.sel.start')}
                </button>
              )}
            </span>
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
  // la pulizia dal rumore la decide l'operatore: «Originale» o «Pulita»; l'originale resta sempre com'e'
  const [clean, setClean] = useState(false);
  const [order, setOrder] = useState(3);
  const [level, setLevel] = useState(0.4);
  const [applied, setApplied] = useState({ order: 3, level: 0.4 });
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const id = window.setTimeout(() => setApplied({ order, level }), 350);
    return () => window.clearTimeout(id);
  }, [order, level]);
  const shown = clean ? sstvImageCleanUrl(entry.id, applied.order, applied.level) : sstvImageUrl(entry.id);
  useEffect(() => {
    if (clean) setBusy(true);
  }, [clean, shown]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [onClose]);

  const save = async () => {
    const blob = clean ? await getSstvImageCleanBlob(entry.id, applied.order, applied.level) : await getSstvImageBlob(entry.id);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = clean ? entry.file.replace(/(\.[^.]*)?$/, '-pulita$1') : entry.file;
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
        <img src={shown} alt="" className={`${smooth ? '' : 'sstv-crisp'}${busy && clean ? ' sstv-busy' : ''}`} width={entry.width} height={entry.height} onLoad={() => setBusy(false)} onError={() => setBusy(false)} />
        <div className="sstv-clean">
          <span className="sstv-views" role="group" aria-label={t('sstv.clean.title')}>
            <button type="button" className={clean ? '' : 'on'} onClick={() => setClean(false)}>
              {t('sstv.clean.original')}
            </button>
            <button type="button" className={clean ? 'on' : ''} onClick={() => setClean(true)} title={t('sstv.clean.hint')}>
              {t('sstv.clean.cleaned')}
            </button>
          </span>
          {clean ? (
            <>
              <label className="sstv-slider">
                <span>{t('sstv.clean.order')} {order}</span>
                <input type="range" min={1} max={6} step={1} value={order} onChange={(e) => setOrder(Number(e.target.value))} />
              </label>
              <label className="sstv-slider">
                <span>{t('sstv.clean.smooth')} {level.toFixed(2)}</span>
                <input type="range" min={0.1} max={1} step={0.05} value={level} onChange={(e) => setLevel(Number(e.target.value))} />
              </label>
              <span className="sstv-hint">{busy ? t('sstv.clean.working') : t('sstv.clean.hint')}</span>
            </>
          ) : null}
        </div>
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
