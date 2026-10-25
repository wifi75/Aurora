import { useT } from '../i18n';
import { EngineValues } from './EngineValues';
import { Group, Notes, Page } from './layout';
import { Help } from './common';
import { useIsNative } from './NativeSettings';

/** Scheda Avanzate: i parametri del decodificatore stanno nel motore. */
export function AdvancedTab() {
  const t = useT();
  const native = useIsNative();
  return (
    <Page>
      <Group title={t('tab.advanced')} span={2}>
        <EngineValues group="advanced" />
      </Group>
      <Group title={t('adv.grpGuide')}>
        <a className="glass glass--link" href="#/guida">
          {t('adv.wiki')}
        </a>
      </Group>
      <Notes span={3}>
        <Help>{native ? t('ens.adv.help') : t('adv.help')}</Help>
      </Notes>
    </Page>
  );
}
