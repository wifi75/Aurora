import { useT } from '../i18n';
import { EngineValues } from './EngineValues';
import { NativeSequence, NativeTxOptions } from './JtdxTabs';
import { EngineGroups, useIsNative } from './NativeSettings';
import { QSO_LOG_GROUPS } from '../lib/engineSettingsLayout';
import { Help } from './common';
import { Group, Notes, Page } from './layout';

/** Scheda Sequenza: come si svolge il QSO e come si registra. Le regole delle risposte automatiche stanno nella scheda Automazioni. */
export function SequenceTab() {
  const t = useT();
  const native = useIsNative();
  if (native) {
    return (
      <NativeSequence
        extra={
          <>
            <EngineGroups groups={QSO_LOG_GROUPS} />
            <NativeTxOptions />
          </>
        }
        notes={<Help>{t('seq.intro')}</Help>}
      />
    );
  }
  return (
    <Page>
      <Group title={t('jx.title')}>
        <EngineValues group="sequence" />
      </Group>
      <Notes span={3}>
        <Help>{t('seq.intro')}</Help>
      </Notes>
    </Page>
  );
}
