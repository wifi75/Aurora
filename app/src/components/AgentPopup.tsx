import { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Logo } from './Logo';

/** Un avviso dell'agente nel tray, nello stile «cartolina»: banda con il logo, titolo grande, testo e due pulsanti. */
export interface AgentNotice {
  kind: string;
  title: string;
  text: string;
  primary: string;
  secondary: string;
  update?: boolean;
  /** Tema grafico dei messaggi dell'autore: natale, anno, pasqua, aggiorna, servizio (vuoto = normale). */
  theme?: string;
  /** Il pulsante principale apre questo indirizzo (nel browser). */
  link?: string;
  /** Cartolina di festa (auguri): colori caldi. */
  festive?: boolean;
}

/**
 * Prototipo: mostra la cartolina in basso a destra dentro la finestra di Aurora, come apparira' sopra l'orologio di Windows.
 * Si chiude da sola dopo `seconds` (ferma con il mouse sopra), con un pulsante o con la ×.
 */
export function showAgentPopup(notice: AgentNotice, seconds: number): void {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  let done = false;
  const close = () => {
    if (done) return;
    done = true;
    root.unmount();
    host.remove();
  };
  root.render(<AgentPopup notice={notice} seconds={seconds} onClose={close} />);
  chime(notice.update === true);
}

/**
 * Il suono di ogni cartolina: un accordo luminoso che sale (quattro note morbide, con una piccola eco), pieno abbastanza da farsi
 * notare. Per gli aggiornamenti una nota in piu' in cima. Se l'audio e' bloccato la cartolina compare lo stesso, muta.
 */
function chime(update: boolean): void {
  try {
    const Ctx = window.AudioContext;
    if (!Ctx) return;
    const ctx = new Ctx();
    if (ctx.state === 'suspended') void ctx.resume();
    const out = ctx.createGain();
    out.gain.value = 0.9;
    // eco leggera: il suono resta nell'aria un attimo
    const echo = ctx.createDelay();
    echo.delayTime.value = 0.16;
    const feedback = ctx.createGain();
    feedback.gain.value = 0.28;
    echo.connect(feedback).connect(echo);
    echo.connect(out);
    out.connect(ctx.destination);
    const notes = update ? [523.25, 659.25, 783.99, 1046.5, 1318.5] : [523.25, 659.25, 783.99, 1046.5];
    const start = ctx.currentTime + 0.04;
    notes.forEach((f, i) => {
      const at = start + i * 0.085;
      for (const [type, mult, level] of [['triangle', 1, 0.16], ['sine', 2, 0.05]] as const) {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = type;
        osc.frequency.value = f * mult;
        gain.gain.setValueAtTime(0.0001, at);
        gain.gain.exponentialRampToValueAtTime(level, at + 0.015);
        gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.9);
        osc.connect(gain);
        gain.connect(out);
        gain.connect(echo);
        osc.start(at);
        osc.stop(at + 0.95);
      }
    });
    window.setTimeout(() => void ctx.close(), 2500);
  } catch {
    // audio non disponibile: la cartolina resta muta
  }
}

function AgentPopup({ notice, seconds, onClose }: { notice: AgentNotice; seconds: number; onClose(): void }) {
  const [leaving, setLeaving] = useState(false);
  const [left, setLeft] = useState(seconds * 1000);
  const hover = useRef(false);

  useEffect(() => {
    const step = 100;
    const timer = window.setInterval(() => {
      if (hover.current) return;
      setLeft((ms) => Math.max(0, ms - step));
    }, step);
    return () => window.clearInterval(timer);
  }, []);
  useEffect(() => {
    if (left > 0 || leaving) return;
    setLeaving(true);
  }, [left, leaving]);
  useEffect(() => {
    if (!leaving) return;
    const timer = window.setTimeout(onClose, 260);
    return () => window.clearTimeout(timer);
  }, [leaving, onClose]);

  const leave = () => setLeaving(true);
  return (
    <div
      className={`agent-card${leaving ? ' agent-card--leaving' : ''}${notice.update ? ' agent-card--update' : ''}${notice.festive ? ' agent-card--festa' : ''}${notice.theme ? ` agent-card--t-${notice.theme}` : ''}`}
      role="status"
      onMouseEnter={() => (hover.current = true)}
      onMouseLeave={() => (hover.current = false)}
    >
      <div className="agent-card-band">
        {notice.theme ? <i className="agent-card-deco" aria-hidden="true" /> : null}
        <Logo size={46} animated className="agent-card-logo" />
        <div className="agent-card-brand">
          <b>AURORA</b>
          <span>{notice.kind}</span>
        </div>
        <button type="button" className="agent-card-x" aria-label="×" onClick={leave}>
          ×
        </button>
      </div>
      <div className="agent-card-body">
        <h3>{notice.title}</h3>
        <p>{notice.text}</p>
        <div className="agent-card-actions">
          {notice.secondary ? (
            <button type="button" className="agent-card-btn" onClick={leave}>
              {notice.secondary}
            </button>
          ) : null}
          <button
            type="button"
            className="agent-card-btn agent-card-btn--main"
            onClick={() => {
              if (notice.link) window.open(notice.link, '_blank', 'noopener');
              leave();
            }}
          >
            {notice.primary}
          </button>
        </div>
      </div>
      <i className="agent-card-time" style={{ width: `${(left / (seconds * 1000)) * 100}%` }} />
    </div>
  );
}
