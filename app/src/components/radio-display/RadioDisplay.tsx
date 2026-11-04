import { useT } from '../../i18n';
import type { DisplayModel, RadioDisplayStyle } from '../../lib/radioDisplay';
import { AmberDisplay } from './AmberDisplay';
import { AnalogDisplay } from './AnalogDisplay';
import { ColorDisplay } from './ColorDisplay';
import { describeDisplay } from './describe';
import { FluorescentDisplay } from './FluorescentDisplay';
import { LedDisplay } from './LedDisplay';
import { MiniDisplay } from './MiniDisplay';
import { DEMO_FACE_EXTRAS, Ft2000Display, Ic7610Display, type FaceExtras } from './RigFaceDisplays';
import { IcBlackSquare, IcBlueDisplay, IcBlueSquare, IcDualDisplay, LedLineDisplay, YaesuSquare } from './ModernDisplays';

interface Props {
  model: DisplayModel;
  style: RadioDisplayStyle;
  /** `mini`: testata e telefono. `full`: tutti gli strumenti (anteprime e ingrandimento). */
  size?: 'mini' | 'full';
  /** Dati in piu' della radio per gli stili a pannello unico; senza, quelli di prova (anteprime). */
  face?: FaceExtras;
}

/** Il display della radio in SVG: si ridisegna con `viewBox` fisso e riempie la larghezza che trova. */
export function RadioDisplay({ model, style, size = 'full', face = DEMO_FACE_EXTRAS }: Props) {
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
    case 'ft2000':
      return <Ft2000Display {...common} data={face} />;
    case 'ic7610':
      return <Ic7610Display {...common} data={face} />;
    case 'icdual':
      return <div className="rd-modern-slot"><IcDualDisplay {...common} data={face} /></div>;
    case 'icblue':
      return <div className="rd-modern-slot"><IcBlueDisplay {...common} data={face} /></div>;
    case 'ledline':
      return <div className="rd-modern-slot"><LedLineDisplay {...common} data={face} /></div>;
    case 'icblueq':
      return <div className="rd-square-preview"><IcBlueSquare {...common} data={face} /></div>;
    case 'icblackq':
      return <div className="rd-square-preview"><IcBlackSquare {...common} data={face} /></div>;
    case 'yaesuq':
      return <div className="rd-square-preview"><YaesuSquare {...common} data={face} /></div>;
  }
}
