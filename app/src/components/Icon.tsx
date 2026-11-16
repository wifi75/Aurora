import type { SVGProps } from 'react';
import './icon-btn.css';

export type IconName = 'gear' | 'refresh' | 'chevron-down' | 'bell' | 'speaker-off' | 'lock' | 'unlock' | 'layout';

/** Tracciati su griglia 24x24, tratto pieno `currentColor`: niente glifi di testo, quindi niente baseline diverse da font a font. */
const PATHS: Record<IconName, string[]> = {
  gear: [
    'M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z',
    'M19.4 13.5a7.7 7.7 0 0 0 0-3l2-1.6-2-3.4-2.4.9a7.6 7.6 0 0 0-2.6-1.5L14 2.5h-4l-.4 2.4A7.6 7.6 0 0 0 7 6.4l-2.4-.9-2 3.4 2 1.6a7.7 7.7 0 0 0 0 3l-2 1.6 2 3.4 2.4-.9a7.6 7.6 0 0 0 2.6 1.5l.4 2.4h4l.4-2.4a7.6 7.6 0 0 0 2.6-1.5l2.4.9 2-3.4-2-1.6Z',
  ],
  refresh: ['M20 11a8 8 0 0 0-14.3-4.5L4 8.5', 'M4 4v4.5h4.5', 'M4 13a8 8 0 0 0 14.3 4.5l1.7-2', 'M20 20v-4.5h-4.5'],
  'chevron-down': ['M6 9l6 6 6-6'],
  'speaker-off': ['M4 9.5h3.5L12 5.5v13l-4.5-4H4Z', 'M16 9.5l5 5', 'M21 9.5l-5 5'],
  lock: ['M7 11V8a5 5 0 0 1 10 0v3', 'M5.5 11h13v9.5h-13Z'],
  unlock: ['M7 11V8a5 5 0 0 1 9.6-1.8', 'M5.5 11h13v9.5h-13Z'],
  layout: ['M4 4h7v9H4Z', 'M13 4h7v5h-7Z', 'M13 11h7v9h-7Z', 'M4 15h7v5H4Z'],
  bell: ['M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 2h-15Z', 'M10 21a2 2 0 0 0 4 0'],
};

type Props = { name: IconName; size?: string | number; strokeWidth?: number } & Omit<SVGProps<SVGSVGElement>, 'name' | 'width' | 'height'>;

/** Icona vettoriale decorativa (`aria-hidden`): il nome accessibile resta sul pulsante che la contiene. */
export function Icon({ name, size = '1em', strokeWidth = 2.1, className, ...rest }: Props) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className ? `icon ${className}` : 'icon'}
      {...rest}
    >
      {PATHS[name].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}
