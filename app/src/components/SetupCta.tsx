import '../styles/first-run.css';
import { useT } from '../i18n';
import type { Key } from '../i18n';

/** Un riquadro unico «clicca per configurare» al posto di scritte sparse quando manca qualcosa (radio, log). */
export function SetupCta({ href, title, action, className = '' }: { href: string; title: Key; action: Key; className?: string }) {
  const t = useT();
  return (
    <a className={`setup-cta ${className}`} href={href}>
      <span className="setup-cta__icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="18" height="18">
          <path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Zm8 3.5-2.1-.6a6 6 0 0 0-.6-1.5l1-1.9-1.4-1.4-1.9 1a6 6 0 0 0-1.5-.6L12.9 4h-1.8l-.6 2.1a6 6 0 0 0-1.5.6l-1.9-1-1.4 1.4 1 1.9a6 6 0 0 0-.6 1.5L4 11.1v1.8l2.1.6c.1.5.3 1 .6 1.5l-1 1.9 1.4 1.4 1.9-1c.5.3 1 .5 1.5.6l.6 2.1h1.8l.6-2.1c.5-.1 1-.3 1.5-.6l1.9 1 1.4-1.4-1-1.9c.3-.5.5-1 .6-1.5l2.1-.6Z" fill="currentColor" />
        </svg>
      </span>
      <span className="setup-cta__text">
        <strong>{t(title)}</strong>
        <span>{t(action)}</span>
      </span>
    </a>
  );
}
