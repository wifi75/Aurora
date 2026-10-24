import { BAND_NAMES, bandOf } from '../lib/format';

interface Props {
  dialHz: number;
  /** Cambia banda; senza, la barra mostra solo la banda in uso. */
  onPick?(band: string): void;
  /** Bande da mostrare, in ordine (predefinito: tutte). */
  bands?: readonly string[];
  /** Bloccata (per esempio in trasmissione). */
  locked?: boolean;
}

/** Barra delle bande come in JTDX: la banda in uso e' accesa; con il comando radio si cambia banda con un clic. */
export function BandBar({ dialHz, onPick, bands = BAND_NAMES, locked = false }: Props) {
  const current = dialHz > 0 ? bandOf(dialHz) : null;
  return (
    <div className="band-bar" role="group" aria-label="Bande">
      {bands.map((name) => (
        <button
          key={name}
          type="button"
          className={`glass${name === current ? ' glass--on' : ''}`}
          aria-pressed={name === current}
          aria-current={name === current ? 'true' : undefined}
          disabled={!onPick || locked}
          title={onPick ? (locked ? 'Non si cambia banda in trasmissione' : `Vai su ${name}`) : 'Comando radio non attivo (AURORA_RIG=omnirig)'}
          onClick={() => onPick?.(name)}
        >
          {name.replace('m', '')}
        </button>
      ))}
    </div>
  );
}
