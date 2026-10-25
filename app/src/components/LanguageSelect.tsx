import { useEffect, useId, useRef, useState, type KeyboardEvent } from 'react';
import { LANGS, LANG_NAMES, useLang, useT, type Lang } from '../i18n';

/** Codice bandiera (libreria flag-icons) di ogni lingua: l'inglese e' la bandiera del Regno Unito. */
export const LANG_FLAGS: Record<Lang, string> = { it: 'it', en: 'gb', de: 'de', fr: 'fr', es: 'es', pt: 'pt' };

/**
 * Scelta della lingua nella testata: menu con le bandiere, sei lingue, il cambio vale subito per tutta l'interfaccia.
 * Tastiera: frecce (apre e sposta), Invio o spazio (apre e sceglie), Esc (chiude), Home e Fine.
 */
export function LanguageSelect() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const setLang = useLang((s) => s.setLang);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(() => LANGS.indexOf(lang));
  const root = useRef<HTMLDivElement>(null);
  const listId = useId();
  const optionId = (code: Lang) => `${listId}-${code}`;

  const openMenu = () => {
    setActive(Math.max(0, LANGS.indexOf(lang)));
    setOpen(true);
  };
  const choose = (code: Lang) => {
    setLang(code);
    setOpen(false);
  };

  // un clic fuori dal menu lo chiude
  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      if (root.current && e.target instanceof Node && !root.current.contains(e.target)) setOpen(false);
    };
    document.addEventListener('pointerdown', onDown);
    return () => document.removeEventListener('pointerdown', onDown);
  }, [open]);

  const onKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    const last = LANGS.length - 1;
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        if (!open) openMenu();
        else setActive((i) => Math.min(last, i + 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (!open) openMenu();
        else setActive((i) => Math.max(0, i - 1));
        break;
      case 'Home':
        if (open) {
          e.preventDefault();
          setActive(0);
        }
        break;
      case 'End':
        if (open) {
          e.preventDefault();
          setActive(last);
        }
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (!open) openMenu();
        else choose(LANGS[active] ?? lang);
        break;
      case 'Escape':
        if (open) {
          e.preventDefault();
          e.stopPropagation();
          setOpen(false);
        }
        break;
      case 'Tab':
        setOpen(false);
        break;
    }
  };

  return (
    <div className="lang-select lang-menu glass" ref={root} title={t('tip.lang')}>
      <button
        type="button"
        className="lang-trigger"
        role="combobox"
        aria-label={t('lang.label')}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        aria-activedescendant={open ? optionId(LANGS[active] ?? lang) : undefined}
        onClick={() => (open ? setOpen(false) : openMenu())}
        onKeyDown={onKeyDown}
      >
        <span className={`fi fi-${LANG_FLAGS[lang]} lang-flag`} aria-hidden="true" />
        <span className="lang-name">{LANG_NAMES[lang]}</span>
        <span className="lang-caret" aria-hidden="true" />
      </button>
      <ul className="lang-list" id={listId} role="listbox" aria-label={t('lang.label')} hidden={!open}>
        {LANGS.map((code, i) => (
          <li
            key={code}
            id={optionId(code)}
            role="option"
            aria-selected={code === lang}
            className={`lang-option${i === active ? ' lang-option--active' : ''}`}
            onPointerEnter={() => setActive(i)}
            onClick={() => choose(code)}
          >
            <span className={`fi fi-${LANG_FLAGS[code]} lang-flag`} aria-hidden="true" />
            <span className="lang-name">{LANG_NAMES[code]}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
