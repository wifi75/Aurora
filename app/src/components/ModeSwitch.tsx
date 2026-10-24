const MODES = ['FT8', 'FT4'] as const;

interface Props {
  /** Modo in uso, come lo riporta JTDX. */
  mode: string;
  /** Cambia modo; senza, il selettore mostra solo il modo in uso. */
  onPick?(mode: string): void;
  /** Bloccato (per esempio in trasmissione). */
  locked?: boolean;
}

/** Selettore FT8 / FT4 come in JTDX; il modo in uso e' acceso. */
export function ModeSwitch({ mode, onPick, locked = false }: Props) {
  return (
    <div className="mode-switch" role="group" aria-label="Modo">
      {MODES.map((m) => (
        <button
          key={m}
          type="button"
          className={`glass${m === mode ? ' glass--on' : ''}`}
          aria-pressed={m === mode}
          aria-current={m === mode ? 'true' : undefined}
          disabled={!onPick || locked}
          title={onPick ? (locked ? 'Non si cambia modo in trasmissione' : `Passa a ${m}`) : 'Comando radio non attivo (AURORA_RIG=omnirig)'}
          onClick={() => {
            if (m !== mode) onPick?.(m);
          }}
        >
          {m}
        </button>
      ))}
    </div>
  );
}
