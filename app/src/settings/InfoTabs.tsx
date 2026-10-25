import { useT } from '../i18n';
import { EngineValues } from './EngineValues';
import { AudioDevicesPanel } from './AudioDevicesPanel';
import { Help, InfoTab, Row } from './common';
import { NativeReports, useIsNative } from './NativeSettings';

/** Scheda Audio: con il motore proprio ingresso, uscita, canali e file audio si scelgono qui; con JTDX mostra da dove arriva la cascata. */
export function AudioTab() {
  const t = useT();
  const native = useIsNative();
  if (native) {
    return (
      <>
        <AudioDevicesPanel />
        <Help>{t('ens.audio.help')}</Help>
      </>
    );
  }
  return (
    <InfoTab
      render={(info) => (
        <>
          <Row name={t('audio.input')}>{info.audio_device ?? t('audio.none')}</Row>
          <Help>{t('audio.help')}</Help>
          <EngineValues group="audio" />
        </>
      )}
    />
  );
}

/** Scheda Segnalazioni: a chi vanno i QSO e quali file dati sono caricati. */
export function ReportsTab() {
  const t = useT();
  const native = useIsNative();
  if (native === undefined) return <p className="muted">{t('set.loading')}</p>;
  if (native) return <NativeReportsTab />;
  return (
    <InfoTab
      render={(info) => (
        <>
          <h3 className="set-sub">{t('rep.export')}</h3>
          {info.export_udp.length === 0 && !info.export_file ? <p className="muted">{t('rep.none')}</p> : null}
          {info.export_udp.map((a) => (
            <Row key={a} name={t('rep.udp')}>
              {a}
            </Row>
          ))}
          {info.export_file ? <Row name={t('rep.file')}>{info.export_file}</Row> : null}
          <Help>{t('rep.help')}</Help>
          <h3 className="set-sub">UDP</h3>
          <Row name={t('rep.udpServer')}>{info.udp_listen}</Row>
          {info.udp_forward.map((a) => (
            <Row key={a} name={t('rep.forward')}>
              {a}
            </Row>
          ))}
          <Row name={t('rep.cty')}>{info.cty_loaded > 0 ? t('rep.countries', { n: info.cty_loaded }) : t('rep.notLoaded')}</Row>
          <Row name={t('rep.log')}>{info.worked_qsos > 0 ? t('rep.qsos', { n: info.worked_qsos }) : t('rep.notRead')}</Row>
          <EngineValues group="reports" />
        </>
      )}
    />
  );
}

/** Scheda Segnalazioni con il motore proprio: le impostazioni di Aurora come nella schermata di JTDX; sotto, solo informazioni. */
function NativeReportsTab() {
  const t = useT();
  return (
    <>
      <NativeReports />
      <InfoTab
        render={(info) => (
          <>
            <h3 className="set-sub">{t('ens.info')}</h3>
            {info.export_file ? <Row name={t('rep.file')}>{info.export_file}</Row> : null}
            <Row name={t('rep.cty')}>{info.cty_loaded > 0 ? t('rep.countries', { n: info.cty_loaded }) : t('rep.notLoaded')}</Row>
            <Row name={t('rep.log')}>{info.worked_qsos > 0 ? t('rep.qsos', { n: info.worked_qsos }) : t('rep.notRead')}</Row>
          </>
        )}
      />
    </>
  );
}
