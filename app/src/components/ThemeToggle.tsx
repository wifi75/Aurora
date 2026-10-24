import { useEffect, useState } from 'react';
import { applyTheme, browserStorage, readTheme, saveTheme, type Theme } from '../lib/theme';

/** Interruttore Scuro/Chiaro nella testata. */
export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(() => readTheme(browserStorage()));

  useEffect(() => {
    applyTheme(document.documentElement, theme);
  }, [theme]);

  const next: Theme = theme === 'dark' ? 'light' : 'dark';
  return (
    <button
      type="button"
      className="theme-toggle"
      onClick={() => {
        saveTheme(browserStorage(), next);
        setTheme(next);
      }}
    >
      {next === 'light' ? 'Tema chiaro' : 'Tema scuro'}
    </button>
  );
}
