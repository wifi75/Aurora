import { useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import type { PanelId } from '../lib/layout';
import { PHONE_TABS, readPhoneTab, savePhoneTab } from '../lib/phone';
import { browserStorage } from '../lib/theme';

interface Props {
  titles: Record<PanelId, string>;
  /** Il contenuto di un pannello (lo stesso della versione a finestre). */
  render(id: PanelId): ReactNode;
  /** Sopra il pannello della scheda indicata, un blocco in piu' (i comandi di ricezione stanno con i Comandi). */
  extra?: Partial<Record<PanelId, ReactNode>>;
}

/** La pagina Stazione sul telefono: una scheda alla volta a tutta larghezza invece delle finestre affiancate. */
export function PhoneTabs({ titles, render, extra = {} }: Props) {
  const [active, setActive] = useState<PanelId>(() => readPhoneTab(browserStorage()));
  const strip = useRef<HTMLDivElement | null>(null);

  const pick = (id: PanelId) => {
    setActive(id);
    savePhoneTab(browserStorage(), id);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (step === 0) return;
    e.preventDefault();
    const next = PHONE_TABS[(index + step + PHONE_TABS.length) % PHONE_TABS.length];
    if (next) {
      pick(next);
      strip.current?.querySelector<HTMLButtonElement>(`#phone-tab-${next}`)?.focus();
    }
  };

  return (
    <div className="phone-tabs">
      <div className="phone-tablist" role="tablist" ref={strip}>
        {PHONE_TABS.map((id, i) => (
          <button
            key={id}
            id={`phone-tab-${id}`}
            type="button"
            role="tab"
            className={`glass${id === active ? ' glass--on' : ''}`}
            aria-selected={id === active}
            aria-controls={`phone-panel-${id}`}
            tabIndex={id === active ? 0 : -1}
            onClick={() => pick(id)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            {titles[id]}
          </button>
        ))}
      </div>
      <div className={`phone-panel phone-panel--${active}`} role="tabpanel" id={`phone-panel-${active}`} aria-labelledby={`phone-tab-${active}`}>
        {extra[active]}
        {render(active)}
      </div>
    </div>
  );
}
