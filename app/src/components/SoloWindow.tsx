import { useEffect, useRef, type PointerEvent as ReactPointerEvent } from 'react';
import { useT, type Key } from '../i18n';
import { attach, groupFromUrl, markClosed, restoreGroup, updateGroup, useGroups, type ItemId } from '../lib/detached';
import { Live } from '../pages/Live';
import { MapPage } from '../pages/MapPage';
import { PropagationPage } from '../pages/PropagationPage';
import '../styles/solo.css';
import { freeCell, GridBoard, type Cell } from './GridBoard';

/** Il titolo di un elemento (gli stessi nomi dei pannelli e delle pagine). */
export const ITEM_TITLE: Record<ItemId, Key> = {
  waterfall: 'panel.waterfall',
  station: 'panel.station',
  tx: 'panel.tx',
  cq: 'panel.cq',
  all: 'panel.all',
  log: 'panel.log',
  cluster: 'panel.cluster',
  map: 'nav.map',
  propagation: 'nav.propagation',
};

function Item({ item }: { item: ItemId }) {
  if (item === 'map') return <MapPage solo />;
  if (item === 'propagation') return <PropagationPage solo />;
  return <Live soloPanel={item} />;
}

/**
 * Una finestra a parte (da portare sul secondo schermo): uno o piu' elementi affiancati o uno sopra l'altro, con un divisore fra l'uno e
 * l'altro. Ogni elemento ha una barretta sottile con il titolo e un pulsantino per rimetterlo nella finestra principale.
 */
export function SoloWindow({ id }: { id: string }) {
  const t = useT();
  const groups = useGroups();
  const group = groups.find((g) => g.id === id);
  const box = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    document.title = group ? `Aurora - ${group.items.map((i) => t(ITEM_TITLE[i])).join(' · ')}` : 'Aurora';
  }, [group, t]);
  // chiusa con la X: gli elementi tornano nella finestra principale
  useEffect(() => {
    // aperta (anche dopo un ricaricamento): i suoi elementi non si vedono nella finestra principale
    updateGroup(id, { open: true });
    const onClose = () => markClosed(id);
    window.addEventListener('beforeunload', onClose);
    return () => window.removeEventListener('beforeunload', onClose);
  }, [id]);
  // tutti gli elementi rimessi nella finestra principale: questa finestra non serve piu'
  // chiusa solo se il gruppo c'era e poi e' sparito (tutto rimesso nella principale); appena aperta, se l'archivio non lo ha ancora,
  // lo si ricostruisce dall'indirizzo invece di chiudersi (prima la finestra lampeggiava e spariva)
  const seen = useRef(false);
  if (group) seen.current = true;
  useEffect(() => {
    if (group) return;
    if (!seen.current) {
      const fromUrl = groupFromUrl(id);
      if (fromUrl) {
        restoreGroup(fromUrl);
        return;
      }
    }
    window.close();
    // nel programma per Windows window.close() non basta: il guscio chiude la finestra quando la pagina chiede questo indirizzo
    location.href = `${location.origin}/__chiudi-finestra`;
  }, [group, id]);
  if (!group) return null;

  const startDivider = (at: number) => (e: ReactPointerEvent<HTMLDivElement>) => {
    const rect = box.current?.getBoundingClientRect();
    if (!rect) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const handle = e.currentTarget;
    const sizes = [...group.sizes];
    const before = sizes.slice(0, at).reduce((a, b) => a + b, 0);
    const pair = (sizes[at] ?? 0) + (sizes[at + 1] ?? 0);
    const move = (ev: PointerEvent) => {
      const pos = group.dir === 'row' ? (ev.clientX - rect.left) / rect.width : (ev.clientY - rect.top) / rect.height;
      const first = Math.min(pair - 0.08, Math.max(0.08, pos - before));
      const next = [...sizes];
      next[at] = first;
      next[at + 1] = pair - first;
      updateGroup(id, { sizes: next });
    };
    const stop = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
  };

  // dashboard a blocchi: ogni elemento e' un blocco da trascinare dalla barretta e ridimensionare dall'angolo
  if (group.mode === 'grid') {
    const cells: Record<string, Cell> = {};
    for (const item of group.items) cells[item] = group.cells?.[item] ?? freeCell(cells);
    return (
      <div className="solo solo--grid">
        <GridBoard
          cells={cells}
          onChange={(next) => updateGroup(id, { cells: next })}
          render={(item, grip) => (
            <div className="solo-slot solo-slot--block">
              <div className="solo-bar solo-bar--grip" onPointerDown={grip.onPointerDown} title={t('win.drag')}>
                <span className="solo-title">{t(ITEM_TITLE[item as ItemId])}</span>
                <button type="button" className="solo-btn" title={t('win.attach')} aria-label={t('win.attach')} onPointerDown={(e) => e.stopPropagation()} onClick={() => attach(item as ItemId)}>
                  ⤓
                </button>
              </div>
              <div className="solo-body">
                <Item item={item as ItemId} />
              </div>
            </div>
          )}
        />
      </div>
    );
  }

  return (
    <div ref={box} className={`solo solo--${group.dir}`}>
      {group.items.map((item, at) => (
        <div key={item} className="solo-slot" style={{ flexGrow: group.sizes[at] ?? 1 }}>
          <div className="solo-bar">
            <span className="solo-title">{t(ITEM_TITLE[item])}</span>
            {at === 0 && group.items.length > 1 ? (
              <button type="button" className="solo-btn" title={t('win.dir')} aria-label={t('win.dir')} onClick={() => updateGroup(id, { dir: group.dir === 'row' ? 'col' : 'row' })}>
                {group.dir === 'row' ? '⇅' : '⇆'}
              </button>
            ) : null}
            <button type="button" className="solo-btn" title={t('win.attach')} aria-label={t('win.attach')} onClick={() => attach(item)}>
              ⤓
            </button>
          </div>
          <div className="solo-body">
            <Item item={item} />
          </div>
          {at < group.items.length - 1 ? <div className="solo-divider" role="separator" aria-orientation={group.dir === 'row' ? 'vertical' : 'horizontal'} onPointerDown={startDivider(at)} /> : null}
        </div>
      ))}
    </div>
  );
}
