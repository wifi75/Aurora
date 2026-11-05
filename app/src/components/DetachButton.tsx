import { useEffect, useRef, useState } from 'react';
import { useT } from '../i18n';
import { DASHBOARD, detach, useGroups, type ItemId } from '../lib/detached';
import { REMOTE_HOST } from '../lib/phone';
import { ITEM_TITLE } from './SoloWindow';

const ICON = (
  <svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true">
    <rect x="1.5" y="4.5" width="9" height="9" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.3" />
    <path d="M6 1.5h8.5V10M14.5 1.5 8 8" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

/**
 * Pulsantino discreto per portare un pannello o una pagina in una finestra a parte (secondo schermo). Se ci sono gia' finestre a parte,
 * un menu piccolo chiede se aprirne una nuova o affiancarlo a una di quelle. Solo sul PC dove gira Aurora: sul telefono non c'e'.
 */
export function DetachButton({ item, className = 'detach-btn' }: { item: ItemId; className?: string }) {
  const t = useT();
  const groups = useGroups();
  const [menu, setMenu] = useState(false);
  const host = useRef<HTMLSpanElement | null>(null);
  useEffect(() => {
    if (!menu) return;
    const close = (e: PointerEvent) => {
      if (!host.current?.contains(e.target as Node)) setMenu(false);
    };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, [menu]);
  if (REMOTE_HOST) return null;
  const others = groups.filter((g) => g.open && !g.items.includes(item) && g.id !== DASHBOARD);
  const dash = groups.find((g) => g.id === DASHBOARD && g.open);
  const go = (into?: string) => {
    setMenu(false);
    detach(item, into);
    // una pagina intera staccata: la finestra principale torna alla Stazione
    if (item === 'map' || item === 'propagation') location.hash = '#/';
  };
  return (
    <span className="dock-detach-host" ref={host} onPointerDown={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
      <button type="button" className={className} title={t('win.detach')} aria-label={t('win.detach')} onClick={() => setMenu(!menu)}>
        {ICON}
      </button>
      {menu ? (
        <span className="dock-detach-menu" role="menu">
          <button type="button" role="menuitem" onClick={() => go()}>
            {t('win.new')}
          </button>
          {dash?.items.includes(item) ? null : (
            <button type="button" role="menuitem" onClick={() => go(DASHBOARD)}>
              {t('win.dash')}
            </button>
          )}
          {others.map((g) => (
            <button key={g.id} type="button" role="menuitem" onClick={() => go(g.id)}>
              {t('win.beside', { what: g.items.map((i) => t(ITEM_TITLE[i])).join(' · ') })}
            </button>
          ))}
        </span>
      ) : null}
    </span>
  );
}
