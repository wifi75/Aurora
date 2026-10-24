import { useEffect, useState } from 'react';
import { useT } from '../i18n';
import { applyTheme, browserStorage, readTheme, saveTheme, type Theme } from '../lib/theme';

/** Interruttore Scuro/Chiaro nella testata. */
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
      className="theme-toggle"
      title={t('tip.theme')}
      onClick={() => {
        saveTheme(browserStorage(), next);
        setTheme(next);
      }}
    >
      {next === 'light' ? t('theme.light') : t('theme.dark')}
    </button>
  );
}
