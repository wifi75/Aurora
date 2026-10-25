import logoSvg from '../assets/logo.svg?raw';

/** Il logo di Aurora: la cascata con le tracce FT8. Con `animated` la cascata scorre. */
export function Logo({ size = 32, animated = false, className = '' }: { size?: number; animated?: boolean; className?: string }) {
  return (
    <span
      className={`logo${animated ? ' logo--animated' : ''}${className ? ` ${className}` : ''}`}
      style={{ width: size, height: size }}
      role="img"
      aria-label="Aurora"
      dangerouslySetInnerHTML={{ __html: logoSvg }}
    />
  );
}
