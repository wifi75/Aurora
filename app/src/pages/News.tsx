import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
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

/** «4 nov 2026, 18:08»: corta, per la riga. */
function shortLabel(iso: string, lang: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(lang, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}

/** Novita': le ultime voci, una riga per versione (tipo, versione, titolo, data); il testo si apre con un clic. Filtri per tipo. */
export function News() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const health = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  const [shown, setShown] = useState<Record<NovitaKind, boolean>>({ new: true, changed: true, fixed: true });
  const items = NOVITA.filter((n) => shown[n.kind]);
  return (
    <div className="news news--compact">
      <header className="news-head">
        <h1>{t('news.title')}</h1>
        <p className="muted">{t('news.intro', { n: NOVITA.length })}</p>
        <div className="news-legend" role="group" aria-label={t('news.legend')}>
          {(['new', 'changed', 'fixed'] as const).map((k) => (
            <button
              key={k}
              type="button"
              aria-pressed={shown[k]}
              className={`news-kind news-kind--${k}${shown[k] ? '' : ' news-kind--off'}`}
              onClick={() => setShown((v) => ({ ...v, [k]: !v[k] }))}
            >
              <span aria-hidden="true">{ICON[k]}</span> {t(`news.kind.${k}` as Key)}
            </button>
          ))}
        </div>
      </header>
      <ul className="news-rows">
        {items.map((n) => {
          const current = health.data?.version === n.version;
          return (
            <li key={`${n.version}-${pick(n.title, 'it')}`}>
              <details className={`news-row news-row--${n.kind}`} open={current}>
                <summary>
                  <span className={`news-badge news-kind--${n.kind}`} title={t(`news.kind.${n.kind}` as Key)}>
                    <span aria-hidden="true">{ICON[n.kind]}</span>
                  </span>
                  <span className="news-version">v{n.version}</span>
                  {current ? <span className="news-current">{t('news.current')}</span> : null}
                  <span className="news-title">{pick(n.title, lang)}</span>
                  <time className="news-date" dateTime={n.released} title={releaseLabel(n.released, lang)}>
                    {shortLabel(n.released, lang)}
                  </time>
                </summary>
                <p>{pick(n.text, lang)}</p>
              </details>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
