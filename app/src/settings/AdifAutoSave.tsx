import { useT } from '../i18n';
import { Cells, Group, Message } from './layout';
import { SettingField, useDraft, useIsNative, useSettings } from './NativeSettings';

/** Il file che si scrive nella cartella scelta, con il separatore che l'utente ha usato. */
export function adifCopyPath(folder: string): string {
  const dir = folder.trim().replace(/[\/]+$/, '');
  if (!dir) return '';
  return `${dir}${dir.includes('/') && !dir.includes('\\') ? '/' : '\\'}aurora_log.adi`;
}

/** «Salvataggio ADIF automatico» (Impostazioni › Log, cluster e servizi › File): copia di `aurora_log.adi` in una cartella a scelta, a ogni QSO. */
export function AdifAutoSaveGroup() {
  const t = useT();
  const native = useIsNative();
  const settings = useSettings();
  const draft = useDraft();
  if (!native || settings.isError || !settings.data) return null;
  const doc = settings.data;
  const on = (draft.changes.AdifAutoSave ?? doc.values.AdifAutoSave ?? 'false') === 'true';
  const folder = draft.changes.AdifFolder ?? doc.values.AdifFolder ?? '';
  const path = adifCopyPath(folder);
  return (
    <Group title={t('adifauto.title')} span={3}>
      <Message kind="muted">{t('adifauto.dataFile')}</Message>
      <Cells min={18}>
        <SettingField row={{ key: 'AdifAutoSave', kind: 'bool' }} label={t('ens.k.AdifAutoSave')} doc={doc} draft={draft} />
        <SettingField row={{ key: 'AdifFolder', kind: 'text' }} label={t('ens.k.AdifFolder')} doc={doc} draft={draft} locked={!on} />
      </Cells>
      {path ? <Message kind="muted">{t('adifauto.result', { path })}</Message> : null}
      {on && !folder.trim() ? <Message kind="alert">{t('adifauto.pick')}</Message> : null}
      <Message kind="muted">{t('adifauto.help')}</Message>
    </Group>
  );
}
