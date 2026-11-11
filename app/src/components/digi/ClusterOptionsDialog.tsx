import { useT } from '../../i18n';
import { CLUSTER_COLUMNS, EXPIRE_CHOICES, type ClusterColumn, type ClusterOptions } from '../../lib/clusterOptions';

interface Props {
  options: ClusterOptions;
  onChange(patch: Partial<ClusterOptions>): void;
  onClose(): void;
}

const LABEL: Record<ClusterColumn, 'cluster.col.by' | 'cluster.col.freq' | 'cluster.col.call' | 'cluster.col.country' | 'cluster.col.mode' | 'cluster.col.comment' | 'cluster.col.time'> = {
  de: 'cluster.col.by',
  freq: 'cluster.col.freq',
  dx: 'cluster.col.call',
  country: 'cluster.col.country',
  mode: 'cluster.col.mode',
  comment: 'cluster.col.comment',
  time: 'cluster.col.time',
};

/** La rotella del DX Cluster nelle pagine PSK, RTTY e CW: come si sceglie uno spot, rotore, scadenza degli spot e colonne. */
export function ClusterOptionsDialog({ options, onChange, onClose }: Props) {
  const t = useT();
  const toggleColumn = (c: ClusterColumn, show: boolean) => {
    const hidden = show ? options.hidden.filter((x) => x !== c) : [...options.hidden, c];
    // almeno una colonna resta: senza il DX lo spot non servirebbe a niente
    if (CLUSTER_COLUMNS.every((x) => hidden.includes(x))) return;
    onChange({ hidden });
  };

  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('cop.title')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <div className="digi-dialog cop-dialog">
        <header className="cop-head">
          <strong>{t('cop.title')}</strong>
          <button type="button" className="digi-tool" onClick={onClose} aria-label={t('sat.set.close')}>
            ✕
          </button>
        </header>

        <fieldset>
          <legend>{t('cop.choose')}</legend>
          <label className="cop-radio">
            <input type="radio" name="cop-click" checked={options.click === 'double'} onChange={() => onChange({ click: 'double' })} />
            <span>{t('cop.double')}</span>
          </label>
          <label className="cop-radio">
            <input type="radio" name="cop-click" checked={options.click === 'single'} onChange={() => onChange({ click: 'single' })} />
            <span>{t('cop.single')}</span>
          </label>
          <label className="cop-radio">
            <input type="checkbox" checked={options.rotor} onChange={(e) => onChange({ rotor: e.target.checked })} />
            <span>{t('cop.rotor')}</span>
          </label>
          <p className="cop-hint">{t('cop.rotorHint')}</p>
        </fieldset>

        <fieldset>
          <legend>{t('cop.spots')}</legend>
          <label className="cop-row">
            <span>{t('cop.expire')}</span>
            <select value={options.expireMin} onChange={(e) => onChange({ expireMin: Number(e.target.value) })}>
              {[...new Set([...EXPIRE_CHOICES, options.expireMin])]
                .sort((a, b) => a - b)
                .map((m) => (
                  <option key={m} value={m}>
                    {m === 0 ? t('cop.never') : t('cop.minutes', { n: m })}
                  </option>
                ))}
            </select>
          </label>
        </fieldset>

        <fieldset>
          <legend>{t('cop.columns')}</legend>
          <div className="cop-cols">
            {CLUSTER_COLUMNS.map((c) => (
              <label key={c} className="cop-radio">
                <input type="checkbox" checked={!options.hidden.includes(c)} onChange={(e) => toggleColumn(c, e.target.checked)} />
                <span>{t(LABEL[c])}</span>
              </label>
            ))}
          </div>
        </fieldset>
      </div>
    </div>
  );
}
