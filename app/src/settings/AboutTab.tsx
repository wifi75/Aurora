import { useQuery } from '@tanstack/react-query';
import { getHealth } from '../api/client';
import { Logo } from '../components/Logo';
import { useLang, useT } from '../i18n';
import { pick } from '../i18n/core';
import { NOVITA } from '../lib/novita';
import { openInMain } from '../lib/sdr';

const REPO = 'https://github.com/wifi75/Aurora';
const SITE = 'https://aurora.iu3wjq.com/';
/** Il pulsante Donate dei README e di `.github/FUNDING.yml`. */
export const DONATE_URL = 'https://www.paypal.com/donate/?hosted_button_id=MY55Q6GDQEH8L';
/** Quante novita' si mostrano nella pagina iniziale delle Impostazioni. */
const NEWS_SHOWN = 3;

/**
 * La pagina di presentazione, prima pagina delle Impostazioni: logo, versione dal core, dedica, autore, licenza, ringraziamenti, le ultime
 * novita', il riquadro per sostenere Aurora (PayPal) e i collegamenti.
 */
export function AboutTab() {
  const t = useT();
  const lang = useLang((s) => s.lang);
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
      {/* la dedica dell'autore: Aurora porta il nome di sua figlia Aurora */}
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
      <section className="about-news" aria-label={t('about.news')}>
        <h3>{t('about.news')}</h3>
        <ul>
          {NOVITA.slice(0, NEWS_SHOWN).map((n) => (
            <li key={`${n.version}-${pick(n.title, 'en')}`}>
              <b>{pick(n.title, lang)}</b>
              <span className="about-news-text">{pick(n.text, lang)}</span>
            </li>
          ))}
        </ul>
        <a
          className="about-news-all"
          href="#/novita"
          onClick={(e) => {
            // dalla finestra a parte la pagina delle Novita' si apre nella finestra principale
            e.preventDefault();
            openInMain('#/novita');
          }}
        >
          {t('about.allNews')}
        </a>
      </section>
      {/* sostenere Aurora: facoltativo, detto con garbo; la stella su GitHub e' l'aiuto che non costa nulla */}
      <section className="about-donate" aria-label={t('about.donateTitle')}>
        <span className="about-donate-heart" aria-hidden="true">♥</span>
        <div className="about-donate-body">
          <h3>{t('about.donateTitle')}</h3>
          <p>{t('about.donateText')}</p>
          <div className="about-donate-actions">
            <a className="about-donate-btn" href={DONATE_URL} target="_blank" rel="noreferrer">
              <span aria-hidden="true">♥</span> {t('about.donateButton')}
            </a>
            <a className="glass about-link about-donate-star" href={REPO} target="_blank" rel="noreferrer">
              {t('about.star')}
            </a>
          </div>
        </div>
      </section>
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
