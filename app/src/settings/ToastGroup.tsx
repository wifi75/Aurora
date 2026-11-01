import { useT, type Key } from '../i18n';
import { toast } from '../lib/toast';
import { TOAST_KINDS, TOAST_POSITIONS, TOAST_SIZES, useToastPrefs, type ToastKindKey, type ToastPosition, type ToastSize } from '../lib/toastPrefs';
import { Check } from './common';
import { Actions, Cells, Field, Group } from './layout';

/** Personalizzazione dei messaggi toast: quali tipi, quanto restano, dove compaiono e quanto sono grandi. */
export function ToastGroup() {
  const t = useT();
  const prefs = useToastPrefs((s) => s.prefs);
  const set = useToastPrefs.getState().set;
  return (
    <Group title={t('toast.grp')} span={2}>
      <Cells min={14}>
        {TOAST_KINDS.map((k: ToastKindKey) => (
          <Check key={k} label={t(`toast.show.${k}` as Key)} hint={t('tip.toast.show')} checked={prefs.show[k]} onChange={(v) => set({ show: { ...prefs.show, [k]: v } })} />
        ))}
        {TOAST_KINDS.map((k: ToastKindKey) => (
          <Field key={`s${k}`} label={t(`toast.secs.${k}` as Key)} hint={t('tip.toast.secs')}>
            <input type="number" min={1} max={30} step={0.5} value={prefs.seconds[k]} onChange={(e) => set({ seconds: { ...prefs.seconds, [k]: Math.min(30, Math.max(1, Number(e.target.value) || 1)) } })} />
          </Field>
        ))}
        <Field label={t('toast.position')} hint={t('tip.toast.position')}>
          <select value={prefs.position} onChange={(e) => set({ position: e.target.value as ToastPosition })}>
            {TOAST_POSITIONS.map((p) => (
              <option key={p} value={p}>
                {t(`toast.pos.${p}` as Key)}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t('toast.size')} hint={t('tip.toast.size')}>
          <select value={prefs.size} onChange={(e) => set({ size: e.target.value as ToastSize })}>
            {TOAST_SIZES.map((s) => (
              <option key={s} value={s}>
                {t(`toast.sz.${s}` as Key)}
              </option>
            ))}
          </select>
        </Field>
      </Cells>
      <Actions>
        <button
          type="button"
          className="glass"
          onClick={() => {
            toast(t('toast.test.ok'), 'ok');
            toast(t('toast.test.info'), 'info');
            toast(t('toast.test.error'), 'error');
          }}
        >
          {t('toast.test')}
        </button>
        <button type="button" className="glass" onClick={() => useToastPrefs.getState().reset()}>
          {t('set.reset')}
        </button>
      </Actions>
    </Group>
  );
}
