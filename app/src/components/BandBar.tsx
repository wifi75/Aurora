import { BAND_NAMES, bandOf } from '../lib/format';

interface Props {
  dialHz: number;
}

/** Barra delle bande come in JTDX: la banda in uso e' accesa. Il cambio banda arriva con il comando radio. */
export function BandBar({ dialHz }: Props) {
  const current = dialHz > 0 ? bandOf(dialHz) : null;
  return (
    <div className="band-bar" role="group" aria-label="Bande">
      {BAND_NAMES.map((name) => (
        <button
          key={name}
          type="button"
          className={`glass${name === current ? ' glass--on' : ''}`}
          aria-pressed={name === current}
          disabled
          title="Il cambio banda arriva con il comando radio"
        >
          {name.replace('m', '')}
        </button>
      ))}
    </div>
  );
}
