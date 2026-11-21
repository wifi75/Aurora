import { useQuery } from '@tanstack/react-query';
import { getCat } from '../../api/client';
import { displayModel, formatRadioFrequency, hasFrequency, mainMeter } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';

const LEDS = 14;
/** Dal segmento 10 in su si e' oltre S9. */
const S9_LED = 9;

/** «PKTUSB» / «USB» → «USB»; «LSB», «CW»...: il modo della radio senza il prefisso dei modi dati. */
export function radioModeLabel(mode: string | null | undefined): string {
  const m = (mode ?? '').toUpperCase().replace(/^PKT/, '');
  return m === '' ? '—' : m;
}

/**
 * Il display per la sola ricezione (pagina WEFAX): frequenza, modo della radio (USB o LSB) e il segnale con la barra di led dell'S-meter.
 * Nient'altro: niente potenza, ROS, TX o TUNE (qui non si trasmette mai). Spettro e cascata stanno sopra il visore
 * (`WefaxScope`).
 */
export function WefaxMini() {
  const status = useStation((s) => s.status);
  const model = displayModel(status);
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 });
  const rigMode = cat.data?.status.rig?.mode;
  const meter = mainMeter({ ...model, transmitting: false });
  const lit = Math.round((meter.fraction ?? 0) * LEDS);
  const freq = hasFrequency(model.dialHz) ? formatRadioFrequency(model.dialHz) : '--.---.--';
  return (
    <div className="wefax-mini">
      <div className="wefax-rx" role="group" aria-label="RX">
        <span className="wefax-rx-freq data">{freq}</span>
        <span className="wefax-rx-mode">{radioModeLabel(rigMode)}</span>
        <span className="wefax-rx-leds" role="img" aria-label={`S ${meter.label ?? '—'}`} title={`S ${meter.label ?? '—'}`}>
          {Array.from({ length: LEDS }, (_, i) => (
            <i key={i} className={i < lit ? (i >= S9_LED ? 'on over' : 'on') : ''} />
          ))}
        </span>
      </div>
    </div>
  );
}
