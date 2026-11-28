import { useT, type Key } from '../i18n';
import { SPECIAL_TAGS, type SpecialTag } from '../lib/specialCq';
import { useSpecialCq, type SpecialCqPrefs } from '../lib/specialCqPrefs';
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
  const change = (next: SpecialCqPrefs) => save(next);
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

      <Notes>
        <Help>{t('tip.scq.enabled')}</Help>
      </Notes>
    </Page>
  );
}
