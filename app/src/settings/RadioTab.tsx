import { useT } from '../i18n';
import { CatGroups, CatNotes } from './CatTab';
import { RadioDisplayPicker } from './RadioDisplayPicker';
import { RadioExtrasGroup } from './RadioExtrasGroup';
import { EngineValues } from './EngineValues';
import { Group, Cells, Page } from './layout';
import { InfoTab, Row } from './common';
import { useIsNative } from './NativeSettings';

/** Scheda Radio: un'unica scheda con tutto il collegamento (modello, porta, PTT, split, lettura, profili, prova, condivisione, OmniRig). */
export function RadioTab() {
  const t = useT();
  const native = useIsNative();
  return (
    <Page>
      <CatGroups />
      <Group title={t('rd.title')} span={3}>
        <RadioDisplayPicker />
      </Group>
      <RadioExtrasGroup />
      <InfoTab
        render={(info) => (
          <Group title={t('radio.sharing')}>
            <Cells min={12}>
              <Row name={t('radio.control')} hint={t('tip.radio.control')}>{info.rig_control ? t('radio.on') : t('radio.off')}</Row>
            </Cells>
            {native ? null : <EngineValues group="radio" />}
          </Group>
        )}
      />
      <CatNotes />
    </Page>
  );
}
