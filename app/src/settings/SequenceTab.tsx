import { useT } from '../i18n';
import { EngineValues } from './EngineValues';
import { NativeSequence, NativeTxOptions } from './JtdxTabs';
import { EngineGroups, useIsNative } from './NativeSettings';
import { QSO_LOG_GROUPS } from '../lib/engineSettingsLayout';
import { Help } from './common';
import { Group, Notes, Page } from './layout';

/**
 * Scheda Sequenza: come si svolge il QSO e come si registra. Le regole delle risposte automatiche stanno nella scheda Automazioni.
 * `part="fox"` mostra solo la verifica del Fox (Hound), che sta in Modi › Contest · Fox/Hound; la Sequenza normale la lascia fuori.
 */
export function SequenceTab({ part = 'all' }: { part?: 'all' | 'qso' | 'fox' } = {}) {
  const t = useT();
  const native = useIsNative();
  if (part === 'fox') {
    return native ? (
      <Page>
        <NativeTxOptions part="fox" />
      </Page>
    ) : null;
  }
  if (native) {
    return (
      <NativeSequence
        extra={
          <>
            <EngineGroups groups={QSO_LOG_GROUPS} />
            <NativeTxOptions part={part === 'qso' ? 'nofox' : 'all'} />
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
