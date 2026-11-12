import { useEffect, useRef, useState } from 'react';

export type MenuItem =
  | 'sep'
  | {
      label: string;
      checked?: boolean;
      disabled?: boolean;
      /** Il tasto rapido da mostrare a destra (solo scritto). */
      shortcut?: string;
      onSelect?(): void;
      /** Un sottomenu. */
      items?: MenuItem[];
    };

export interface MenuDef {
  label: string;
  items: MenuItem[];
}

/** La barra dei menu della finestra (File, Edit, Mode, Options, View, Configure, Help). */
export function MenuBar({ menus }: { menus: MenuDef[] }) {
  const [open, setOpen] = useState<number | null>(null);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open === null) return;
    const close = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(null);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(null);
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', esc);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', esc);
    };
  }, [open]);

  return (
    <div className="digi-menubar" role="menubar" ref={box}>
      {menus.map((m, i) => (
        <div key={m.label} className="digi-menu">
          <button
            type="button"
            role="menuitem"
            aria-haspopup="menu"
            aria-expanded={open === i}
            className={open === i ? 'on' : ''}
            onClick={() => setOpen(open === i ? null : i)}
            onMouseEnter={() => (open !== null ? setOpen(i) : undefined)}
          >
            {m.label}
          </button>
          {open === i ? <MenuList items={m.items} onDone={() => setOpen(null)} /> : null}
        </div>
      ))}
    </div>
  );
}

export function MenuList({ items, onDone }: { items: MenuItem[]; onDone(): void }) {
  const [sub, setSub] = useState<number | null>(null);
  return (
    <ul className="digi-menu-list" role="menu">
      {items.map((it, i) =>
        it === 'sep' ? (
          <li key={`sep${i}`} className="digi-menu-sep" role="separator" />
        ) : (
          <li key={`${it.label}${i}`} onMouseEnter={() => setSub(it.items ? i : null)}>
            <button
              type="button"
              role="menuitem"
              disabled={it.disabled}
              aria-checked={it.checked === undefined ? undefined : it.checked}
              className={it.items ? 'has-sub' : ''}
              onClick={() => {
                if (it.items) return;
                it.onSelect?.();
                onDone();
              }}
            >
              <span className="digi-menu-check" aria-hidden="true">
                {it.checked ? '✓' : ''}
              </span>
              <span className="digi-menu-label">{it.label}</span>
              {it.shortcut ? <span className="digi-menu-key">{it.shortcut}</span> : null}
              {it.items ? <span className="digi-menu-arrow">▸</span> : null}
            </button>
            {it.items && sub === i ? (
              <div className="digi-menu-sub">
                <MenuList items={it.items} onDone={onDone} />
              </div>
            ) : null}
          </li>
        ),
      )}
    </ul>
  );
}
