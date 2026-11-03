import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, getDiagnostics, prepareSupportBundle, revealSupportBundle, type SupportBundle } from '../api/client';
import { useLang, useT } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { ReportsBox } from './ReportsBox';

/** Un pacchetto fatto da meno di cosi' si riusa per «Manda per email» invece di rifarlo. */
const FRESH_MS = 5 * 60_000;

/** Il messaggio di posta gia' compilato per chi aiuta. */
export function supportMailto(to: string, subject: string, body: string): string {
  return `mailto:${encodeURIComponent(to.trim())}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
}

/** Lo stesso messaggio in Gmail sul web (si apre in Chrome): niente programma di posta da configurare. */
export function supportGmail(to: string, subject: string, body: string): string {
  return `https://mail.google.com/mail/?view=cm&fs=1&to=${encodeURIComponent(to.trim())}&su=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
}

/** Scheda «Assistenza» delle Impostazioni (pagina a se'). */
export function SupportTab() {
  return (
    <div className="sx-page">
      <SupportBox />
      <ReportsBox />
    </div>
  );
}

/** Riquadro «Assistenza»: prepara lo zip per chi aiuta e, se si vuole, apre la posta gia' scritta. */
export function SupportBox() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const email = usePrefs((s) => s.prefs.supportEmail);
  const setPrefs = usePrefs((s) => s.set);
  const diagnostics = useQuery({ queryKey: ['diagnostics'], queryFn: getDiagnostics, retry: false });
  const [busy, setBusy] = useState(false);
  const [made, setMade] = useState<{ bundle: SupportBundle; at: number } | null>(null);
  const [failure, setFailure] = useState('');
  const available = diagnostics.data?.support_bundle === true;
  const emailOk = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim());

  const prepare = async (): Promise<SupportBundle | null> => {
    setBusy(true);
    setFailure('');
    try {
      const bundle = await prepareSupportBundle(lang);
      setMade({ bundle, at: Date.now() });
      return bundle;
    } catch (error) {
      setFailure(t('sup.failed', { message: error instanceof ApiError ? error.code : '—' }));
      return null;
    } finally {
      setBusy(false);
    }
  };

  const reveal = () => {
    setFailure('');
    revealSupportBundle().catch((error: unknown) => setFailure(t('sup.failed', { message: error instanceof ApiError ? error.code : '—' })));
  };

  const sendMail = async () => {
    let bundle = made && Date.now() - made.at < FRESH_MS ? made.bundle : null;
    if (bundle) reveal();
    else bundle = await prepare();
    if (!bundle) return;
    const vars = { version: bundle.version || '—', call: bundle.call || '—', radio: bundle.radio || '—', ptt: bundle.ptt || '—', name: bundle.name };
    // la posta si apre in Gmail nel browser (Chrome nel programma Windows): niente programma di posta da configurare
    window.open(supportGmail(email, t('sup.mailSubject', vars), t('sup.mailBody', vars)), '_blank');
  };

  return (
    <section className="about-fact about-support">
      <h3>{t('sup.title')}</h3>
      <p>{t('sup.intro')}</p>
      {available ? (
        <>
          <div className="about-support-actions">
            <button type="button" className="glass about-support-main" onClick={() => void prepare()} disabled={busy} title={t('tip.sup.prepare')}>
              {busy ? t('sup.working') : t('sup.prepare')}
            </button>
            {made ? (
              <button type="button" className="glass" onClick={reveal} title={t('tip.sup.openFolder')}>
                {t('sup.openFolder')}
              </button>
            ) : null}
          </div>
          {made ? <p className="about-support-ready">{t('sup.ready', { name: made.bundle.name })}</p> : null}
          <div className="about-support-actions">
            <label className="about-support-email" title={t('tip.sup.email')}>
              <span>{t('sup.email')}</span>
              <input type="email" autoComplete="off" spellCheck={false} value={email} onChange={(e) => setPrefs({ supportEmail: e.target.value })} />
            </label>
            <button type="button" className="glass" onClick={() => void sendMail()} disabled={busy || !emailOk} title={emailOk ? t('tip.sup.sendMail') : t('sup.needEmail')}>
              {t('sup.sendMail')}
            </button>
          </div>
          <p className="about-support-note">{t('sup.mailHint')}</p>
        </>
      ) : (
        <p className="about-support-note">{t('sup.onlyPc')}</p>
      )}
      {failure ? <p className="about-support-alert" role="alert">{failure}</p> : null}
      <p className="about-support-note">{t('sup.privacy')}</p>
    </section>
  );
}
