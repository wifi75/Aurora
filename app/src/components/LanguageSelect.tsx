import { LANGS, LANG_NAMES, useLang, useT, type Lang } from '../i18n';

/** Scelta della lingua nella testata: sei lingue, il cambio vale subito per tutta l'interfaccia. */
export function LanguageSelect() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const setLang = useLang((s) => s.setLang);
  return (
    <label className="lang-select glass" title={t('tip.lang')}>
      <span className="visually-hidden">{t('lang.label')}</span>
      <select value={lang} onChange={(e) => setLang(e.target.value as Lang)} aria-label={t('lang.label')}>
        {LANGS.map((code) => (
          <option key={code} value={code}>
            {LANG_NAMES[code]}
          </option>
        ))}
      </select>
    </label>
  );
}
