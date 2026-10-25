import { useT } from '../i18n';
import { CatGroups, CatNotes } from './CatTab';
import { EngineValues } from './EngineValues';
import { Group, Cells, Page, Unavailable } from './layout';
import { InfoTab, Row } from './common';
import { useIsNative } from './NativeSettings';

/** «Audio TCI» e' l'unica voce della radio che Aurora non ha: si vede spenta, con il motivo, nel riquadro delle voci non disponibili. */
function TciUnavailable() {
  const t = useT();
  const label = t('jx.TCIAudio');
  return (
    <Unavailable count={1}>
      <div className="jt-row jt-row--off" title={t('radio.tciOff')}>
        <input type="checkbox" disabled aria-label={label} />
        <span>{label}</span>
        <span className="muted jt-reason">{t('radio.tciOff')}</span>
      </div>
    </Unavailable>
  );
}

/** Scheda Radio: un'unica scheda con tutto il collegamento (modello, porta, PTT, split, lettura, profili, prova, condivisione, OmniRig). */
export function RadioTab() {
  const t = useT();
  const native = useIsNative();
  return (
    <Page>
      <CatGroups />
      <InfoTab
        render={(info) => (
          <Group title={t('radio.sharing')}>
            <Cells min={12}>
              <Row name={t('radio.control')}>{info.rig_control ? t('radio.on') : t('radio.off')}</Row>
            </Cells>
            {native ? null : <EngineValues group="radio" />}
          </Group>
        )}
      />
      {native ? <TciUnavailable /> : null}
      <CatNotes />
    </Page>
  );
}
