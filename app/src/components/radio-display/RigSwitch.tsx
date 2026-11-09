import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getCat, putCat } from '../../api/client';
import { useT } from '../../i18n';
import type { RadioDisplayStyle } from '../../lib/radioDisplay';

/**
 * Con OmniRig: il blocchetto «RIG 1 / RIG 2» a sinistra del display, nei colori dello stile del display scelto (richiesta del proprietario,
 * 2026-11-09, per chi ha due radio in OmniRig). Un clic passa all'altra radio. Senza OmniRig non c'e'.
 */
export function RigSwitch({ style }: { style: RadioDisplayStyle }) {
  const t = useT();
  const qc = useQueryClient();
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 });
  const config = cat.data?.config;
  if (!config || config.backend !== 'omnirig') return null;
  const rig = config.omnirig_rig === 2 ? 2 : 1;
  const choose = (n: 1 | 2) => {
    if (n === rig) return;
    void putCat({ ...config, omnirig_rig: n })
      .then(() => qc.invalidateQueries({ queryKey: ['cat'] }))
      .catch(() => undefined);
  };
  return (
    <div className={`rd-rigsw rd-rigsw--${style}`} role="radiogroup" aria-label={t('cat.omnirig.rig')} title={t('tip.cat.omnirig.rig')}>
      <span className="rd-rigsw-label">RIG</span>
      {([1, 2] as const).map((n) => (
        <button key={n} type="button" role="radio" aria-checked={rig === n} className={rig === n ? 'rd-rigsw-btn rd-rigsw-btn--on' : 'rd-rigsw-btn'} onClick={() => choose(n)}>
          {n}
        </button>
      ))}
    </div>
  );
}
