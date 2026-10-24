import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

export interface PickOption {
  value: string;
  label: string;
}

interface Props {
  /** Nome del filtro (anche nome accessibile del pulsante). */
  label: string;
  /** Testo della voce che azzera la scelta ("Tutte"). */
  allLabel: string;
  options: readonly PickOption[];
  selected: readonly string[];
  onChange(next: string[]): void;
}

/** Un menu a tendina con caselle: si possono scegliere piu' voci insieme; nessuna scelta = tutte. */
export function MultiPick({
  label,
  allLabel,
  options,
  selected,
  onChange,
}: Props) {
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState({ top: 0, left: 0 });
  const root = useRef<HTMLDivElement>(null);
  const menu = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const away = (e: Event) => {
      const inside =
        root.current?.contains(e.target as Node) ||
        menu.current?.contains(e.target as Node);
      if (!inside) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", away);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("pointerdown", away);
      document.removeEventListener("keydown", esc);
    };
  }, [open]);

  const chosen = options.filter((o) => selected.includes(o.value));
  const summary =
    chosen.length === 0
      ? allLabel
      : chosen.length <= 2
        ? chosen.map((o) => o.label).join(", ")
        : `${chosen.length}`;
  const toggle = (value: string) =>
    onChange(
      selected.includes(value)
        ? selected.filter((v) => v !== value)
        : [...selected, value],
    );

  return (
    <div className="mpick" ref={root}>
      <button
        type="button"
        className="mpick-button"
        aria-haspopup="true"
        aria-expanded={open}
        aria-label={label}
        onClick={(e) => {
          // il menu e' fisso rispetto alla finestra: cosi' non lo taglia il bordo del pannello
          const r = e.currentTarget.getBoundingClientRect();
          setPos({ top: r.bottom + 4, left: r.left });
          setOpen(!open);
        }}
      >
        <span className="mpick-label">{label}:</span> {summary}
        <span className="mpick-caret" aria-hidden="true">
          ▾
        </span>
      </button>
      {open
        ? createPortal(
            // fuori dal pannello: il suo vetro sfocato (backdrop-filter) farebbe da riferimento al posizionamento fisso
            <div
              className="mpick-menu"
              role="group"
              aria-label={label}
              ref={menu}
              style={{ top: pos.top, left: pos.left }}
            >
              <label className="mpick-item mpick-item--all">
                <input
                  type="checkbox"
                  checked={selected.length === 0}
                  onChange={() => onChange([])}
                />
                {allLabel}
              </label>
              {options.map((o) => (
                <label key={o.value} className="mpick-item">
                  <input
                    type="checkbox"
                    checked={selected.includes(o.value)}
                    onChange={() => toggle(o.value)}
                  />
                  {o.label}
                </label>
              ))}
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}
