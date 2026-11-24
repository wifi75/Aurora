import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getAudioDevices, putAudioSettings, type AudioDevice, type AudioDevices } from '../../api/client';
import { useT } from '../../i18n';
import { coreText } from '../../lib/coreText';
import { toast } from '../../lib/toast';
import type { MenuItem } from './MenuBar';

/** Il dispositivo scelto per l'ingresso (vuoto o «predefinito del sistema» valgono lo stesso). */
export function isInputChosen(d: AudioDevices, name: string): boolean {
  const norm = (v: string) => (v.trim() === '' || v.trim().toLowerCase() === d.system_default.toLowerCase() ? '' : v.trim().toLowerCase());
  return norm(d.values.input) === norm(name);
}

/**
 * La voce «Ingresso audio» del menu Configura: un sottomenu con i dispositivi di ingresso rilevati (come nelle Impostazioni › Audio) per
 * cambiare in un clic, per esempio fra l'audio della radio e un ricevitore SDR in rete (richiesta di un utente, #95). Segna quello in uso.
 */
export function useAudioInputMenu(): MenuItem {
  const t = useT();
  const client = useQueryClient();
  const devices = useQuery({ queryKey: ['audio-devices'], queryFn: getAudioDevices, retry: false, staleTime: 5_000 });
  const d = devices.data;
  const pick = (name: string) => {
    void putAudioSettings({ input: name })
      .then((r) => {
        toast(r.input.status === 'failed' ? (r.input.message ? coreText(r.input.message, t) : t('digi.mi.audioInputFailed')) : t('digi.mi.audioInputDone', { name: r.input.in_use ?? name }), r.input.status === 'failed' ? 'error' : 'ok');
        void client.invalidateQueries({ queryKey: ['audio-devices'] });
      })
      .catch(() => toast(t('digi.mi.audioInputFailed'), 'error'));
  };
  if (!d) return { label: t('digi.mi.audioInput'), disabled: true };
  const defaultName = d.inputs.find((x: AudioDevice) => x.is_default)?.name;
  return {
    label: t('digi.mi.audioInput'),
    items: [
      { label: defaultName ? t('aud.default', { name: defaultName }) : t('aud.defaultPlain'), checked: isInputChosen(d, d.system_default), onSelect: () => pick(d.system_default) },
      ...d.inputs.map((x: AudioDevice): MenuItem => ({ label: x.name, checked: isInputChosen(d, x.name), onSelect: () => pick(x.name) })),
    ],
  };
}
