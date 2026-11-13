import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, postSplitOff } from '../../api/client';
import { useT } from '../../i18n';
import { toast } from '../../lib/toast';

/**
 * «Spegni split» accanto al display della radio: compare solo con la radio in split (VFO B in trasmissione) e la riporta in simplex,
 * qualunque cosa l'avesse accesa (uno spot «UP», lo split di FT8, la radio o un altro programma). Richiesta del proprietario: deve
 * esserci sempre un modo veloce per spegnerlo.
 */
export function SplitOffButton({ split }: { split: boolean | null | undefined }) {
  const t = useT();
  const client = useQueryClient();
  const [busy, setBusy] = useState(false);
  if (split !== true) return null;
  const off = async () => {
    setBusy(true);
    try {
      await postSplitOff();
      void client.invalidateQueries({ queryKey: ['cat'] });
      void client.invalidateQueries({ queryKey: ['split-dx'] });
    } catch (e) {
      toast(e instanceof ApiError ? (e.message ?? e.code) : String(e), 'error');
    } finally {
      setBusy(false);
    }
  };
  return (
    <button type="button" className="glass rd-split-off" title={t('rd.splitOff.tip')} disabled={busy} onClick={() => void off()}>
      {t('rd.splitOff')}
    </button>
  );
}
