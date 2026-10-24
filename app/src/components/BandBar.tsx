import { useT } from '../i18n';
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
  const t = useT();
  const current = dialHz > 0 ? bandOf(dialHz) : null;
  return (
    <div className="band-bar" role="group" aria-label={t('band.group')} title={t('tip.band.bar')}>
      {bands.map((name) => (
        <button
          key={name}
          type="button"
          className={`glass${name === current ? ' glass--on' : ''}`}
          aria-pressed={name === current}
          aria-current={name === current ? 'true' : undefined}
          disabled={!onPick || locked}
          title={onPick ? (locked ? t('band.locked') : t('band.pick', { band: name })) : t('band.noRig')}
          onClick={() => onPick?.(name)}
        >
          {name.replace('m', '')}
        </button>
      ))}
    </div>
  );
}
