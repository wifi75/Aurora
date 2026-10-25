import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getOnline, putOnline, type OnlineSettings } from '../api/client';
import { useT } from '../i18n';
import { Check, Help } from './common';
import { Cells, Group, Message, Notes, Page } from './layout';

/** Scheda Servizi online: PSK Reporter e propagazione, spenti finche' non si accendono; ognuno dice cosa vede il sito. */
export function OnlineTab() {
  const t = useT();
  const client = useQueryClient();
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const [note, setNote] = useState('');
  const save = useMutation({
    mutationFn: (settings: OnlineSettings) => putOnline(settings),
    onSuccess: () => {
      setNote(t('onl.saved'));
      void client.invalidateQueries({ queryKey: ['online'] });
      void client.invalidateQueries({ queryKey: ['online-psk'] });
      void client.invalidateQueries({ queryKey: ['online-solar'] });
    },
    onError: () => setNote(t('onl.error')),
  });

  if (online.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!online.data) return <p className="muted">{t('set.loading')}</p>;
  const settings = online.data;

  return (
    <Page>
      <Group title="PSK Reporter">
        <Cells min={12}>
          <Check label={t('onl.psk')} checked={settings.psk} onChange={(psk) => save.mutate({ ...settings, psk })} />
        </Cells>
        <Help>{t('onl.psk.note')}</Help>
      </Group>
      <Group title={t('onl.grpSolar')}>
        <Cells min={12}>
          <Check label={t('onl.solar')} checked={settings.solar} onChange={(solar) => save.mutate({ ...settings, solar })} />
        </Cells>
        <Help>{t('onl.solar.note')}</Help>
      </Group>
      <Notes>
        <Help>{t('onl.intro')}</Help>
      </Notes>
      {note ? (
        <div className="sx-span-3">
          <Message>{note}</Message>
        </div>
      ) : null}
    </Page>
  );
}
