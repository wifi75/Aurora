import { useEffect, useMemo, useState } from 'react';
import { STRINGS, textOf, useLang, useT, type Key } from '../i18n';
import { pick, type Entry, type Lang } from '../i18n/core';
import { GUIDE, type GuideSection } from '../i18n/guide';

/** `[[chiave]]` diventa il testo vero dell'interfaccia (nome di una scheda, di un pulsante): la guida dice sempre cio' che si vede. */
export function guideText(entry: Entry, lang: Lang): string {
  return pick(entry, lang).replace(/\[\[([\w.]+)\]\]/g, (whole, key: string) => (key in STRINGS ? (textOf(lang, key as Key) ?? whole) : whole));
}

function sectionText(s: GuideSection, lang: Lang): string {
  return [s.title, s.text, s.tip, ...(s.steps ?? []), ...(s.fix ?? [])].filter((e): e is Entry => !!e).map((e) => guideText(e, lang)).join(' ');
}

/** Guida: capitoli con indice, ricerca e passi numerati (cosa premere e dove), nella lingua scelta. */
export function Wiki() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const [query, setQuery] = useState('');
  const needle = query.trim().toLowerCase();

  // `#/guida/<sezione>` (dalla scheda SDR delle Impostazioni): si scorre fino a quella sezione, anche se la Guida era gia' aperta
  useEffect(() => {
    const go = () => {
      const m = /^#\/guida\/([a-z0-9-]{1,30})$/.exec(location.hash);
      if (m) document.getElementById(`guida-sez-${m[1]}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    };
    const timer = window.setTimeout(go, 50);
    window.addEventListener('hashchange', go);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener('hashchange', go);
    };
  }, []);

  const chapters = useMemo(
    () =>
      GUIDE.map((c) => ({
        ...c,
        sections: needle === '' ? c.sections : c.sections.filter((s) => sectionText(s, lang).toLowerCase().includes(needle)),
      })).filter((c) => c.sections.length > 0),
    [lang, needle],
  );

  return (
    <div className="guide">
      <aside className="guide-index" aria-label={t('wiki.title')}>
        <input type="search" className="guide-search" title={t('tip.ui.search')} placeholder={t('guide.search')} aria-label={t('guide.search')} value={query} onChange={(e) => setQuery(e.target.value)} />
        <nav>
          <ol>
            {GUIDE.map((c) => (
              <li key={c.id}>
                <a href={`#/guida/${c.id}`} onClick={(e) => {
                  e.preventDefault();
                  document.getElementById(`guida-${c.id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
                }}>
                  {guideText(c.title, lang)}
                </a>
              </li>
            ))}
          </ol>
        </nav>
      </aside>

      <div className="guide-body">
        <h1 className="page-title">{t('wiki.title')}</h1>
        <p className="muted guide-intro">{t('wiki.intro')}</p>
        {chapters.length === 0 ? <p className="muted">{t('guide.none')}</p> : null}
        {chapters.map((c, ci) => (
          <section key={c.id} id={`guida-${c.id}`} className="guide-chapter">
            <h2 className="guide-chapter-title">
              <span className="guide-chapter-n">{GUIDE.findIndex((x) => x.id === c.id) + 1 || ci + 1}</span>
              {guideText(c.title, lang)}
            </h2>
            <div className="guide-sections">
              {c.sections.map((s) => (
                <article key={s.id} id={`guida-sez-${s.id}`} className="guide-section">
                  <h3>{guideText(s.title, lang)}</h3>
                  {s.text ? <p>{guideText(s.text, lang)}</p> : null}
                  {s.steps ? (
                    <ol className="guide-steps">
                      {s.steps.map((step, i) => (
                        <li key={i}>{guideText(step, lang)}</li>
                      ))}
                    </ol>
                  ) : null}
                  {s.tip ? <p className="guide-tip">{guideText(s.tip, lang)}</p> : null}
                  {s.fix ? (
                    <div className="guide-fix">
                      <h4>{t('guide.fix')}</h4>
                      <ul>
                        {s.fix.map((f, i) => (
                          <li key={i}>{guideText(f, lang)}</li>
                        ))}
                      </ul>
                    </div>
                  ) : null}
                </article>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
