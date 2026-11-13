import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { STRINGS, textOf, useLang, useT, type Key } from '../i18n';
import { pick, type Entry, type Lang } from '../i18n/core';
import { GUIDE, type GuideSection } from '../i18n/guide';

/** `[[chiave]]` diventa il testo vero dell'interfaccia (nome di una scheda, di un pulsante): la guida dice sempre cio' che si vede. */
/** Il testo con le parole cercate evidenziate (`<mark>`), senza badare a maiuscole e minuscole; senza ricerca il testo com'e'. */
export function highlight(text: string, needle: string): ReactNode {
  if (needle === '') return text;
  const lower = text.toLowerCase();
  const out: ReactNode[] = [];
  let from = 0;
  let at = lower.indexOf(needle);
  while (at >= 0) {
    if (at > from) out.push(text.slice(from, at));
    out.push(<mark key={at} className="guide-hit">{text.slice(at, at + needle.length)}</mark>);
    from = at + needle.length;
    at = lower.indexOf(needle, from);
  }
  if (from < text.length) out.push(text.slice(from));
  return out;
}

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
  // come la ricerca di un lettore PDF: quante volte la parola compare, quale si sta guardando, ▲▼ (e Invio / Maiusc+Invio) per saltare
  const [hits, setHits] = useState(0);
  const [cur, setCur] = useState(0);
  const goHit = (to: number) => {
    const marks = Array.from(document.querySelectorAll<HTMLElement>('mark.guide-hit'));
    if (marks.length === 0) return;
    const i = ((to % marks.length) + marks.length) % marks.length;
    marks.forEach((m, k) => m.classList.toggle('guide-hit--on', k === i));
    marks[i]?.scrollIntoView?.({ behavior: 'smooth', block: 'center' });
    setCur(i);
  };
  useEffect(() => {
    const n = document.querySelectorAll('mark.guide-hit').length;
    setHits(n);
    setCur(0);
    if (n > 0) {
      const first = document.querySelector<HTMLElement>('mark.guide-hit');
      first?.classList.add('guide-hit--on');
      first?.scrollIntoView?.({ block: 'center' });
    }
  }, [needle, lang]);

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
        <input type="search" className="guide-search" title={t('tip.ui.search')} placeholder={t('guide.search')} aria-label={t('guide.search')} value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              goHit(e.shiftKey ? cur - 1 : cur + 1);
            }
          }} />
        {needle !== '' ? (
          <div className="guide-hits" role="status">
            <span className="data">{hits > 0 ? t('guide.hits', { n: cur + 1, total: hits }) : t('guide.noHits')}</span>
            <button type="button" className="glass chip" title={t('guide.prevHit')} aria-label={t('guide.prevHit')} disabled={hits === 0} onClick={() => goHit(cur - 1)}>▲</button>
            <button type="button" className="glass chip" title={t('guide.nextHit')} aria-label={t('guide.nextHit')} disabled={hits === 0} onClick={() => goHit(cur + 1)}>▼</button>
          </div>
        ) : null}
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
                  <h3>{highlight(guideText(s.title, lang), needle)}</h3>
                  {s.text ? <p>{highlight(guideText(s.text, lang), needle)}</p> : null}
                  {s.steps ? (
                    <ol className="guide-steps">
                      {s.steps.map((step, i) => (
                        <li key={i}>{highlight(guideText(step, lang), needle)}</li>
                      ))}
                    </ol>
                  ) : null}
                  {s.tip ? <p className="guide-tip">{highlight(guideText(s.tip, lang), needle)}</p> : null}
                  {s.fix ? (
                    <div className="guide-fix">
                      <h4>{t('guide.fix')}</h4>
                      <ul>
                        {s.fix.map((f, i) => (
                          <li key={i}>{highlight(guideText(f, lang), needle)}</li>
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
