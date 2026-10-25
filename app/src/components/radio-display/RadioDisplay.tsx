import { useT } from '../../i18n';
import type { DisplayModel, RadioDisplayStyle } from '../../lib/radioDisplay';
import { AmberDisplay } from './AmberDisplay';
import { AnalogDisplay } from './AnalogDisplay';
import { ColorDisplay } from './ColorDisplay';
import { describeDisplay } from './describe';
import { FluorescentDisplay } from './FluorescentDisplay';
import { LedDisplay } from './LedDisplay';
import { MiniDisplay } from './MiniDisplay';

interface Props {
  model: DisplayModel;
  style: RadioDisplayStyle;
  /** `mini`: testata e telefono. `full`: tutti gli strumenti (anteprime e ingrandimento). */
  size?: 'mini' | 'full';
}

/** Il display della radio in SVG: si ridisegna con `viewBox` fisso e riempie la larghezza che trova. */
export function RadioDisplay({ model, style, size = 'full' }: Props) {
  const t = useT();
  const common = { model, noRadio: t('rd.noRadio'), ariaLabel: describeDisplay(model, t) };
  if (size === 'mini') return <MiniDisplay {...common} style={style} />;
  switch (style) {
    case 'fluorescent':
      return <FluorescentDisplay {...common} />;
    case 'analog':
      return <AnalogDisplay {...common} />;
    case 'color':
      return <ColorDisplay {...common} />;
    case 'amber':
      return <AmberDisplay {...common} />;
    case 'led':
      return <LedDisplay {...common} />;
  }
}
