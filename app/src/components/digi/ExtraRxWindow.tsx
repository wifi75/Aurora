import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { useT } from "../../i18n";
import { getExtraText, onExtraText } from "../../lib/digiBus";
import "../../styles/digi-extra-rx.css";

interface Props {
  id: number;
  freqHz: number;
  modeName: string;
  /** Posizione di partenza (px dalla finestra), le finestre si mettono in scala. */
  index: number;
  onClose(): void;
  /** Doppio clic su una parola: la si prende (nominativo nel QSO). */
  onWord?(word: string): void;
}

/** Una finestra RX in piu': piccola, si trascina dal titolo, la X la chiude; mostra il testo del suo ricevitore. */
export function ExtraRxWindow({
  id,
  freqHz,
  modeName,
  index,
  onClose,
  onWord,
}: Props) {
  const t = useT();
  const text = useSyncExternalStore(onExtraText, () => getExtraText(id));
  const [pos, setPos] = useState<{ x: number; y: number }>(() => ({
    x: Math.max(16, window.innerWidth - 340 - index * 28),
    y: 140 + index * 34,
  }));
  const body = useRef<HTMLPreElement>(null);
  useEffect(() => {
    const el = body.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [text]);
  const drag = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const sx = e.clientX - pos.x;
    const sy = e.clientY - pos.y;
    const move = (m: PointerEvent) =>
      setPos({
        x: Math.min(window.innerWidth - 120, Math.max(0, m.clientX - sx)),
        y: Math.min(window.innerHeight - 40, Math.max(0, m.clientY - sy)),
      });
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  return (
    <section
      className="xrx"
      style={{ left: pos.x, top: pos.y }}
      aria-label={`RX (${id})`}
    >
      <header
        className="xrx-head"
        onPointerDown={drag}
        title={t("xrx.dragTip")}
      >
        <strong>RX ({id})</strong>
        <span>
          {modeName} · {Math.round(freqHz)} Hz
        </span>
        <button
          type="button"
          className="xrx-close"
          aria-label={t("xrx.close")}
          title={t("xrx.close")}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={onClose}
        >
          ×
        </button>
      </header>
      <pre
        ref={body}
        className="xrx-text"
        onDoubleClick={() => {
          const w =
            window.getSelection()?.toString().trim().toUpperCase() ?? "";
          if (w && onWord) onWord(w);
        }}
      >
        {text || <span className="xrx-empty">{t("xrx.empty")}</span>}
      </pre>
    </section>
  );
}
