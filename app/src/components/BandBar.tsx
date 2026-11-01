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
  /** Banda appena scelta: si accende subito, prima che la radio e il motore la confermino. */
  pendingBand?: string | null;
  /** Tasto destro: la frequenza successiva dell'elenco del Band Hopping. */
  onHop?(): void;
}

/** Barra delle bande come nel motore: la banda in uso e' accesa; con il comando radio si cambia banda con un clic. */
export function BandBar({ dialHz, onPick, bands = BAND_NAMES, locked = false, pendingBand = null, onHop }: Props) {
  const t = useT();
  const current = pendingBand ?? (dialHz > 0 ? bandOf(dialHz) : null);
  return (
    <div className="band-bar" role="group" aria-label={t('band.group')} title={onHop ? `${t('tip.band.bar')} ${t('tip.band.hop')}` : t('tip.band.bar')}
      onContextMenu={(e) => {
        if (!onHop || locked) return;
        e.preventDefault();
        onHop();
      }}
    >
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
