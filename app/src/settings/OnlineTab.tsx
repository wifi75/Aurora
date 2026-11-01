import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getOnline, putOnline, type OnlineSettings } from '../api/client';
import { useT } from '../i18n';
import { Check, Help } from './common';
import { Cells, Field, Group, Message, Notes, Page } from './layout';
import { usePrefs } from '../lib/prefs';

/** Scheda Servizi online: PSK Reporter e propagazione, spenti finche' non si accendono; ognuno dice cosa vede il sito. */
export function OnlineTab() {
  const t = useT();
  const client = useQueryClient();
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const [note, setNote] = useState('');
  const prefs = usePrefs((s) => s.prefs);
  const save = useMutation({
    mutationFn: (settings: OnlineSettings) => putOnline(settings),
    onSuccess: () => {
      setNote(t('onl.saved'));
      void client.invalidateQueries({ queryKey: ['online'] });
      void client.invalidateQueries({ queryKey: ['online-psk'] });
      void client.invalidateQueries({ queryKey: ['online-solar'] });
      void client.invalidateQueries({ queryKey: ['online-dxped'] });
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
          <Check label={t('onl.psk')} hint={t('tip.onl.psk')} checked={settings.psk} onChange={(psk) => save.mutate({ ...settings, psk })} />
        </Cells>
        <Help>{t('onl.psk.note')}</Help>
      </Group>
      <Group title={t('onl.grpSolar')}>
        <Cells min={12}>
          <Check label={t('onl.solar')} hint={t('tip.onl.solar')} checked={settings.solar} onChange={(solar) => save.mutate({ ...settings, solar })} />
        </Cells>
        <Help>{t('onl.solar.note')}</Help>
      </Group>
      <Group title={t('onl.grpDxped')}>
        <Cells min={12}>
          <Check label={t('onl.dxped')} hint={t('tip.onl.dxped')} checked={settings.dxped === true} onChange={(dxped) => save.mutate({ ...settings, dxped })} />
          <Field label={t('onl.dxpedDays')} hint={t('tip.onl.dxpedDays')}>
            <input
              type="number"
              className="set-number"
              min={0}
              max={90}
              value={prefs.dxpedDays}
              onChange={(e) => {
                const n = Math.round(Number(e.target.value));
                if (Number.isFinite(n)) usePrefs.getState().set({ dxpedDays: Math.min(90, Math.max(0, n)) });
              }}
            />
          </Field>
        </Cells>
        <Help>{t('onl.dxped.note')}</Help>
      </Group>
      <Notes>        <Help>{t('onl.intro')}</Help>
      </Notes>
      {note ? (
        <div className="sx-span-3">
          <Message>{note}</Message>
        </div>
      ) : null}
    </Page>
  );
}
