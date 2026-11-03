import { useQuery } from '@tanstack/react-query';
import { getHealth } from '../api/client';
import { Logo } from '../components/Logo';
import { useT } from '../i18n';
import { SupportBox } from './SupportBox';

const REPO = 'https://github.com/wifi75/Aurora';
const SITE = 'https://wifi75.github.io/Aurora/';

/** La pagina di presentazione: logo, versione dal core, autore, licenza, collegamenti e ringraziamenti. */
export function AboutTab() {
  const t = useT();
  const { data } = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  const links: Array<[string, string]> = [
    [t('about.site'), SITE],
    [t('about.repo'), REPO],
    [t('about.releases'), `${REPO}/releases`],
    [t('about.issues'), `${REPO}/issues`],
    [t('about.qrz'), 'https://www.qrz.com/db/IU3WJQ'],
  ];
  return (
    <div className="about">
      <header className="about-hero">
        <Logo size={96} animated className="about-logo" />
        <div className="about-title">
          <h2 className="about-name">AURORA</h2>
          <p className="about-tagline">{t('about.tagline')}</p>
          <p className="about-version">
            {t('about.version')} <strong className="data about-version-n">{data?.version ?? '—'}</strong>
          </p>
        </div>
      </header>
      {/* la dedica dell'autore: Aurora porta il nome di sua figlia Aura */}
      <figure className="about-dedica">
        <span className="about-dedica-cuore" aria-hidden="true">♥</span>
        <blockquote>{t('about.dedica')}</blockquote>
        <figcaption>{t('about.dedicaFirma')}</figcaption>
      </figure>
      <div className="about-facts">
        <section className="about-fact">
          <h3>{t('about.author')}</h3>
          <p>
            Tiziano Cassone · <strong className="data">IU3WJQ</strong>
          </p>
        </section>
        <section className="about-fact">
          <h3>{t('about.license')}</h3>
          <p>
            © 2026 Tiziano Cassone (IU3WJQ). <strong>GPL-3.0</strong> — {t('about.licenseText')} {t('about.warranty')}
          </p>
        </section>
        <section className="about-fact">
          <h3>{t('about.derived')}</h3>
          <p>{t('about.derivedText')}</p>
        </section>
        <section className="about-fact">
          <h3>{t('about.thanks')}</h3>
          <p>{t('about.thanksText')}</p>
        </section>
      </div>
      <SupportBox />
      <nav className="about-links" aria-label={t('about.links')}>
        {links.map(([label, href]) => (
          <a key={href} className="glass about-link" href={href} target="_blank" rel="noreferrer">
            {label} ↗
          </a>
        ))}
      </nav>
    </div>
  );
}
