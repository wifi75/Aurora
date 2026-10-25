import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getAutomation, putAutomation } from '../api/client';
import { AutomationRulesForm } from '../components/AutomationRulesForm';
import { useT } from '../i18n';
import { EngineValues } from './EngineValues';
import { NativeSequence } from './JtdxTabs';
import { useIsNative } from './NativeSettings';
import { Help } from './common';

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
  if (state.isError) return <p className="notice">{t('auto.unavailable')}</p>;
  if (!state.data) return <p className="muted">{t('set.loading')}</p>;
  return (
    <>
      {native ? <NativeSequence /> : null}
      {native ? <h3 className="set-sub">{t('seq2.rules')}</h3> : null}
      <Help>{t('seq.intro')}</Help>
      <AutomationRulesForm rules={state.data.rules} onSave={(rules) => save.mutate(rules)} error={save.isError} />
      {native ? null : <EngineValues group="sequence" />}
      <a className="glass glass--link" href="#/automazioni">
        {t('seq.link')}
      </a>
    </>
  );
}
