import type { WaterfallSize } from '../lib/waterfallSize';

const OPTIONS: ReadonlyArray<{ value: WaterfallSize; label: string }> = [
  { value: 'large', label: 'Grande' },
  { value: 'small', label: 'Ridotta' },
  { value: 'hidden', label: 'Nascosta' },
];

interface Props {
  value: WaterfallSize;
  onChange(size: WaterfallSize): void;
}

/** Tre pulsanti per far occupare alla cascata tutto lo spazio, una fascia sottile o niente. */
export function WaterfallSizeSwitch({ value, onChange }: Props) {
  return (
    <div className="size-switch" role="group" aria-label="Dimensione della cascata">
      {OPTIONS.map((o) => (
        <button key={o.value} type="button" aria-pressed={value === o.value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}
