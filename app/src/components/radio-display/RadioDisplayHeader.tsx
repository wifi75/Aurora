import { useEffect, useId, useRef, useState } from 'react';
import { useT } from '../../i18n';
import { usePhone } from '../../lib/phone';
import { displayModel, useRadioDisplay } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';
import { RadioDisplay } from './RadioDisplay';

const OPEN_DELAY_MS = 250;
const CLOSE_DELAY_MS = 180;

/**
 * Il display in testata: sul PC e' piccolo, con un clic (o passandoci sopra) si apre la versione ingrandita con tutti gli strumenti.
 * Sul telefono resta la versione compatta, senza ingrandimento. Con "Nascondi il display" non c'e' affatto.
 */
export function RadioDisplayHeader() {
  const t = useT();
  const phone = usePhone();
  const pref = useRadioDisplay((s) => s.pref);
  const status = useStation((s) => s.status);
  const [open, setOpen] = useState(false);
  const [pinned, setPinned] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const root = useRef<HTMLDivElement>(null);
  const popoverId = useId();

  const later = (value: boolean, ms: number) => {
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setOpen(value), ms);
  };
  useEffect(() => () => clearTimeout(timer.current), []);

  // Esc chiude; un clic fuori chiude anche quando e' aperto con un clic
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      setOpen(false);
      setPinned(false);
    };
    const onDown = (e: PointerEvent) => {
      if (root.current && e.target instanceof Node && !root.current.contains(e.target)) {
        setOpen(false);
        setPinned(false);
      }
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onDown);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onDown);
    };
  }, [open]);

  if (pref.hidden) return null;
  const model = displayModel(status);

  if (phone) {
    return (
      <div className="rd-header rd-header--phone">
        <RadioDisplay model={model} style={pref.style} size="mini" />
      </div>
    );
  }

  return (
    <div
      className="rd-header"
      ref={root}
      onMouseEnter={() => {
        if (!pinned) later(true, OPEN_DELAY_MS);
      }}
      onMouseLeave={() => {
        if (!pinned) later(false, CLOSE_DELAY_MS);
      }}
    >
      <button
        type="button"
        className="rd-trigger"
        aria-expanded={open}
        aria-controls={open ? popoverId : undefined}
        title={t('rd.open')}
        onClick={() => {
          clearTimeout(timer.current);
          const next = !(open && pinned);
          setPinned(next);
          setOpen(next);
        }}
      >
        <RadioDisplay model={model} style={pref.style} size="mini" />
      </button>
      {open ? (
        <div className="rd-popover" id={popoverId} role="group" aria-label={t('rd.popover')}>
          <RadioDisplay model={model} style={pref.style} size="full" />
        </div>
      ) : null}
    </div>
  );
}
