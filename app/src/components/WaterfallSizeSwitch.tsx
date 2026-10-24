import { useT, type Key } from '../i18n';
import type { WaterfallSize } from '../lib/waterfallSize';

const OPTIONS: ReadonlyArray<{ value: WaterfallSize; label: Key }> = [
  { value: 'large', label: 'size.large' },
  { value: 'small', label: 'size.small' },
  { value: 'hidden', label: 'size.hidden' },
];

interface Props {
  value: WaterfallSize;
  onChange(size: WaterfallSize): void;
}

/** Tre pulsanti per far occupare alla cascata tutto lo spazio, una fascia sottile o niente. */
export function WaterfallSizeSwitch({ value, onChange }: Props) {
  const t = useT();
  return (
    <div className="size-switch" role="group" aria-label={t('size.group')} title={t('tip.size')}>
      {OPTIONS.map((o) => (
        <button key={o.value} type="button" aria-pressed={value === o.value} onClick={() => onChange(o.value)}>
          {t(o.label)}
        </button>
      ))}
    </div>
  );
}
