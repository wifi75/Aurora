import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getOnline, putOnline, type OnlineSettings } from '../api/client';
import { useT, type Key } from '../i18n';
import { SPECIAL_TAGS, type SpecialTag } from '../lib/specialCq';
import { anySpecialSound, useSpecialCq, type SpecialCqPrefs } from '../lib/specialCqPrefs';
import { useSoundAssign } from '../lib/soundAssign';
import { Check, Help } from './common';
import { Cells, Group, Notes, Page } from './layout';

/**
 * Pagina «CQ speciali» (Modi › FT8 · FT4): riconosce «CQ POTA», «CQ SOTA», «CQ IOTA», «CQ WWFF» e «CQ QRP» (richiesta #115).
 * Tutto spento di fabbrica; le scelte valgono per questo dispositivo, come gli altri avvisi, e si applicano subito.
 */
export function SpecialCqTab() {
  const t = useT();
  const prefs = useSpecialCq((s) => s.prefs);
  const save = useSpecialCq((s) => s.set);
  // la ricerca del parco la fa il core: l'interruttore sta con gli altri servizi online (spento di fabbrica)
  const client = useQueryClient();
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const putPota = useMutation({
    mutationFn: (settings: OnlineSettings) => putOnline(settings),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['online'] });
      void client.invalidateQueries({ queryKey: ['online-pota'] });
    },
  });
  const setPota = (pota: boolean) => {
    if (online.data) putPota.mutate({ ...online.data, pota });
  };
  const change = (next: SpecialCqPrefs) => {
    save(next);
    // spegnendo l'interruttore generale si ferma anche la lettura degli spot POTA
    if (!next.enabled && online.data?.pota === true) setPota(false);
    // un avviso acceso deve suonare: se l'evento non ha ancora un suono, gli si da' quello di fabbrica (il telegrafo «CQ»)
    if (anySpecialSound(next)) useSoundAssign.getState().ensure(['special_cq']);
  };
  const flag = (kind: 'show' | 'sound', tag: SpecialTag, value: boolean) => change({ ...prefs, [kind]: { ...prefs[kind], [tag]: value } });

  return (
    <Page>
      <Group title={t('scq.title')} span={2}>
        <Cells min={16}>
          <Check label={t('scq.enabled')} hint={t('tip.scq.enabled')} checked={prefs.enabled} onChange={(enabled) => change({ ...prefs, enabled })} />
        </Cells>
      </Group>

      <Group title={t('scq.grpShow')}>
        <Cells min={10}>
          {SPECIAL_TAGS.map((tag) => (
            <Check key={tag} label={tag} hint={t(`scq.desc.${tag}` as Key)} checked={prefs.show[tag]} onChange={(v) => flag('show', tag, v)} />
          ))}
        </Cells>
      </Group>

      <Group title={t('scq.grpSound')}>
        <Cells min={10}>
          {SPECIAL_TAGS.map((tag) => (
            <Check key={tag} label={tag} hint={t('tip.scq.sound')} checked={prefs.sound[tag]} onChange={(v) => flag('sound', tag, v)} />
          ))}
        </Cells>
      </Group>

      <Group title="POTA">
        <Cells min={16}>
          <Check
            label={t('scq.pota')}
            hint={t('tip.scq.pota')}
            checked={prefs.enabled && online.data?.pota === true}
            onChange={(v) => setPota(v && prefs.enabled)}
          />
        </Cells>
      </Group>

      <Notes>
        <Help>{t('tip.scq.enabled')}</Help>
      </Notes>
    </Page>
  );
}
