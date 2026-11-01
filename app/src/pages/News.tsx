import { useQuery } from '@tanstack/react-query';
import { getHealth } from '../api/client';
import { useLang, useT, type Key } from '../i18n';
import { pick } from '../i18n/core';
import { NOVITA, type Novita, type NovitaKind } from '../lib/novita';

const ICON: Record<NovitaKind, string> = { new: '✦', changed: '↻', fixed: '✓' };

/** Le voci raggruppate per versione, dalla piu' recente (l'ordine dell'elenco). */
export function groupByVersion(items: readonly Novita[]): Array<{ version: string; released: string; items: Novita[] }> {
  const out: Array<{ version: string; released: string; items: Novita[] }> = [];
  for (const n of items) {
    const last = out[out.length - 1];
    if (last && last.version === n.version) last.items.push(n);
    else out.push({ version: n.version, released: n.released, items: [n] });
  }
  return out;
}

/** «domenica 1 novembre 2026, 08:42» nell'ora del dispositivo e nella lingua scelta. */
export function releaseLabel(iso: string, lang: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(lang, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}

/** Novita': le ultime 20 funzioni aggiunte, modificate o corrette, a gruppi per versione, con data e ora del rilascio. */
export function News() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const health = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  const groups = groupByVersion(NOVITA);
  return (
    <div className="news">
      <header className="news-head">
        <h1>{t('news.title')}</h1>
        <p className="muted">{t('news.intro', { n: NOVITA.length })}</p>
        <ul className="news-legend" aria-label={t('news.legend')}>
          {(['new', 'changed', 'fixed'] as const).map((k) => (
            <li key={k} className={`news-kind news-kind--${k}`}>
              <span aria-hidden="true">{ICON[k]}</span> {t(`news.kind.${k}` as Key)}
            </li>
          ))}
        </ul>
      </header>
      <ol className="news-timeline">
        {groups.map((g) => (
          <li key={g.version} className="news-release">
            <div className="news-release-head">
              <span className="news-version">v{g.version}</span>
              {health.data?.version === g.version ? <span className="news-current">{t('news.current')}</span> : null}
              <time className="news-date" dateTime={g.released} title={g.released}>
                {t('news.released', { when: releaseLabel(g.released, lang) })}
              </time>
            </div>
            <ul className="news-items">
              {g.items.map((n) => (
                <li key={pick(n.title, 'it')} className={`news-card news-card--${n.kind}`}>
                  <span className={`news-badge news-kind--${n.kind}`}>
                    <span aria-hidden="true">{ICON[n.kind]}</span> {t(`news.kind.${n.kind}` as Key)}
                  </span>
                  <h2>{pick(n.title, lang)}</h2>
                  <p>{pick(n.text, lang)}</p>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ol>
    </div>
  );
}
