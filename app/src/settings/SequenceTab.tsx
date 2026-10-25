import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getAutomation, putAutomation } from '../api/client';
import { AutomationRulesForm } from '../components/AutomationRulesForm';
import { useT } from '../i18n';
import { EngineValues } from './EngineValues';
import { NativeSequence } from './JtdxTabs';
import { useIsNative } from './NativeSettings';
import { Help } from './common';
import { Actions, Group, Notes, Page } from './layout';

/** Scheda Sequenza: le regole con cui Aurora risponde da sola ai CQ (le stesse della pagina Automazioni). */
export function SequenceTab() {
  const t = useT();
  const client = useQueryClient();
  const native = useIsNative();
  const state = useQuery({ queryKey: ['automation'], queryFn: getAutomation, retry: false });
  const save = useMutation({
    mutationFn: (rules: Parameters<typeof putAutomation>[0]['rules']) => putAutomation({ rules }),
    onSuccess: (next) => client.setQueryData(['automation'], next),
  });
  const rules = (
    <Group title={native ? t('seq2.rules') : t('tab.sequence')} span={2}>
      {state.isError ? (
        <p className="notice">{t('auto.unavailable')}</p>
      ) : !state.data ? (
        <p className="muted">{t('set.loading')}</p>
      ) : (
        <div className="sx-rules">
          <AutomationRulesForm rules={state.data.rules} onSave={(r) => save.mutate(r)} error={save.isError} />
        </div>
      )}
      <Actions>
        <a className="glass glass--link" href="#/automazioni">
          {t('seq.link')}
        </a>
      </Actions>
    </Group>
  );
  if (native) {
    return (
      <NativeSequence
        extra={
          <>
            {rules}
            <Notes>
              <Help>{t('seq.intro')}</Help>
            </Notes>
          </>
        }
      />
    );
  }
  return (
    <Page>
      {rules}
      <Group title={t('jx.title')}>
        <EngineValues group="sequence" />
      </Group>
      <Notes span={3}>
        <Help>{t('seq.intro')}</Help>
      </Notes>
    </Page>
  );
}
