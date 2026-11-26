import { useT } from '../../../i18n';
import type { TextContestScore } from '../../../api/contestTesto';
import { contestBandColor } from '../../../lib/contestBandColors';
import { multGridOf } from '../../../lib/contestMultGrid';
import { multName } from '../ContestStatsDialog';

interface Props {
  score: TextContestScore;
  /** Il tipo di moltiplicatore della scheda (`dom`, `dxcc`, `cq`...). */
  tipo: string;
  /** I moltiplicatori che il nominativo scritto porterebbe di nuovo (`tipo:valore`): la casella pulsa. */
  pending: ReadonlySet<string>;
}

/** Il titolo della scheda: nome del tipo e «lavorati su totali». */
export function MultsTitle({ score, tipo }: { score: TextContestScore; tipo: string }) {
  const t = useT();
  const g = multGridOf(score, tipo);
  return (
    <>
      {multName(t, tipo)} <span className="ctw-count">{g.total !== null ? t('ssbc.multOf', { n: g.worked, tot: g.total }) : g.worked}</span>
    </>
  );
}

/**
 * Una scheda per tipo di moltiplicatore: una casella compatta per ogni valore (sigla), con i punti colorati delle bande dove e' stato lavorato; il valore
 * ancora da lavorare e' in tono spento e l'ultimo nuovo e' evidenziato. L'elenco completo dei valori lo da' il core per i tipi a elenco chiuso (altrimenti
 * ci sono solo i lavorati). La griglia scorre dentro la scheda se i valori sono molti.
 */
export function MultsCard({ score, tipo, pending }: Props) {
  const t = useT();
  const g = multGridOf(score, tipo, pending);
  if (g.cells.length === 0) return <p className="ssbc-muted">{t('ssbc.multsEmpty')}</p>;
  return (
    <div className="ctw-mgrid" role="list" aria-label={multName(t, tipo)}>
      {g.cells.map((c) => (
        <span
          key={c.value}
          role="listitem"
          className={`ctw-mcell${c.worked ? ' on' : ''}${c.fresh ? ' new' : ''}${c.pending ? ' pend' : ''}`}
          title={(c.worked ? (c.bands.length > 0 ? t('ctw.multTip', { value: c.value, bands: c.bands.join(' ') }) : t('ctw.multTipOnce', { value: c.value })) : t('ctw.multTipNo', { value: c.value })) + (c.modes.length > 0 ? ` (${c.modes.join(' ')})` : '')}
        >
          <b>{c.value}</b>
          {g.perBand ? (
            <i className="ctw-dots" aria-hidden="true">
              {c.bands.map((b) => (
                <u key={b} style={{ background: contestBandColor(b) }} />
              ))}
            </i>
          ) : null}
        </span>
      ))}
    </div>
  );
}
