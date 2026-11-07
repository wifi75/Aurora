import { useEffect, useState } from 'react';
import { useT } from '../i18n';
import { applyTheme, browserStorage, readThemeChoice, resolveTheme, saveThemeChoice, type ThemeChoice } from '../lib/theme';

const ORDER: readonly ThemeChoice[] = ['dark', 'light', 'auto'];

/** Interruttore del tema nella testata, a icona: Scuro, Chiaro, Automatico (segue il tema del sistema e cambia con lui). */
export function ThemeToggle() {
  const t = useT();
  const [choice, setChoice] = useState<ThemeChoice>(() => readThemeChoice(browserStorage()));

  useEffect(() => {
    const apply = () => applyTheme(document.documentElement, resolveTheme(choice));
    apply();
    if (choice !== 'auto') return;
    let media: MediaQueryList | null = null;
    try {
      media = window.matchMedia('(prefers-color-scheme: light)');
    } catch {
      media = null;
    }
    media?.addEventListener('change', apply);
    return () => media?.removeEventListener('change', apply);
  }, [choice]);

  const next = ORDER[(ORDER.indexOf(choice) + 1) % ORDER.length]!;
  const label = (c: ThemeChoice) => (c === 'light' ? t('theme.light') : c === 'dark' ? t('theme.dark') : t('theme.auto'));
  return (
    <button
      type="button"
      className="theme-toggle theme-toggle--icon"
      aria-label={label(next)}
      title={`${t('theme.now')}: ${label(choice)} — ${label(next)} — ${t('tip.theme')}`}
      onClick={() => {
        saveThemeChoice(browserStorage(), next);
        setChoice(next);
      }}
    >
      {/* l'icona dice il tema scelto: luna = scuro, sole = chiaro, cerchio meta' pieno = automatico */}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        {choice === 'light' ? (
          <>
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
          </>
        ) : choice === 'dark' ? (
          <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
        ) : (
          <>
            <circle cx="12" cy="12" r="8" />
            <path d="M12 4a8 8 0 0 1 0 16z" fill="currentColor" />
          </>
        )}
      </svg>
    </button>
  );
}
