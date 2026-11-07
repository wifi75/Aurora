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
      className={`agent-card${leaving ? ' agent-card--leaving' : ''}${notice.update ? ' agent-card--update' : ''}`}
      role="status"
      onMouseEnter={() => (hover.current = true)}
      onMouseLeave={() => (hover.current = false)}
    >
      <div className="agent-card-band">
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
          <button type="button" className="agent-card-btn" onClick={leave}>
            {notice.secondary}
          </button>
          <button type="button" className="agent-card-btn agent-card-btn--main" onClick={leave}>
            {notice.primary}
          </button>
        </div>
      </div>
      <i className="agent-card-time" style={{ width: `${(left / (seconds * 1000)) * 100}%` }} />
    </div>
  );
}
