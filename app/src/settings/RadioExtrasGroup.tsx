import { useT, type Key } from '../i18n';
import { RADIO_EXTRA_KEYS, useRadioExtras } from '../lib/radioExtras';
import { Check, Help } from './common';
import { Cells, Group } from './layout';

/** Display esteso: ogni voce si accende o si spegne, con la sua spiegazione. */
export function RadioExtrasGroup() {
  const t = useT();
  const prefs = useRadioExtras((s) => s.prefs);
  return (
    <Group title={t('rx.grp')} span={3}>
      <Cells min={16}>
        {RADIO_EXTRA_KEYS.map((k) => (
          <Check key={k} label={t(`rx.${k}` as Key)} hint={t(`tip.rx.${k}` as Key)} checked={prefs[k]} onChange={(v) => useRadioExtras.getState().set({ [k]: v })} />
        ))}
      </Cells>
      <Help>{t('rx.note')}</Help>
    </Group>
  );
}
