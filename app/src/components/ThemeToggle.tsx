import { useEffect, useState } from 'react';
import { useT } from '../i18n';
import { applyTheme, browserStorage, readTheme, saveTheme, type Theme } from '../lib/theme';

/** Interruttore Scuro/Chiaro nella testata, a icona (sole o luna). */
export function ThemeToggle() {
  const t = useT();
  const [theme, setTheme] = useState<Theme>(() => readTheme(browserStorage()));

  useEffect(() => {
    applyTheme(document.documentElement, theme);
  }, [theme]);

  const next: Theme = theme === 'dark' ? 'light' : 'dark';
  return (
    <button
      type="button"
      className="theme-toggle theme-toggle--icon"
      aria-label={next === 'light' ? t('theme.light') : t('theme.dark')}
      title={`${next === 'light' ? t('theme.light') : t('theme.dark')} — ${t('tip.theme')}`}
      onClick={() => {
        saveTheme(browserStorage(), next);
        setTheme(next);
      }}
    >
      {/* sole per passare al chiaro, luna per passare allo scuro */}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        {next === 'light' ? (
          <>
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
          </>
        ) : (
          <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
        )}
      </svg>
    </button>
  );
}
