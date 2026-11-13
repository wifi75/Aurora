import { useT } from '../i18n';
import { LOG_SERVICE_GROUPS } from '../lib/engineSettingsLayout';
import { Log4omDbSection } from './Log4omDbSection';
import { NewLogBox } from '../components/LogTransfer';
import { NativeGroupsPage, useIsNative } from './NativeSettings';

/** Scheda «Servizi di log»: eQSL, Wavelog/Cloudlog, ClubLog, QRZ.com e LoTW, un riquadro per servizio (interruttore, campi, «Prova», ultimo invio). */
export function LogServicesTab() {
  const t = useT();
  const native = useIsNative();
  if (native === undefined) return <p className="muted">{t('set.loading')}</p>;
  return (
    <>
      {/* «Nuovo log vuoto» qui, con il log e i suoi servizi (prima era nella pagina File ADIF) */}
      <section className="sw-box" aria-label={t('newlog.button')}>
        <h3>{t('newlog.button')}</h3>
        <NewLogBox />
      </section>
      <Log4omDbSection />
      {native ? <NativeGroupsPage groups={LOG_SERVICE_GROUPS} label={t('tab.logservices')} layout="rows" /> : <p className="muted">{t('ens.unavailable')}</p>}
    </>
  );
}
