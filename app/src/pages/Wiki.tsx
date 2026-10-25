import { useT } from '../i18n';
import { WIKI_TOPICS } from '../i18n/strings/wiki';

/** Guida in-app: spiegazioni brevi, una per argomento, nella lingua scelta. */
export function Wiki() {
  const t = useT();
  return (
    <div className="settings">
      <h1 className="page-title">{t('wiki.title')}</h1>
      <p className="muted">{t('wiki.intro')}</p>
      {WIKI_TOPICS.map((id) => (
        <section key={id} id={`guida-${id}`} className="panel settings-block">
          <h2 className="panel-title">{t(`wiki.${id}.t`)}</h2>
          <p style={{ whiteSpace: 'pre-line' }}>{t(`wiki.${id}.b`)}</p>
        </section>
      ))}
    </div>
  );
}
