import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getInputLevel, getOutputLevel, postControlValue, putInputLevel, putOutputLevel } from '../api/client';
import { useT } from '../i18n';
import { isWindowsOs, useEngineOs } from '../i18n/os';
import { useControlValues } from '../state/controlValues';
import { attenuationFromPercent, levelPercent } from '../lib/outputLevel';

/**
 * Il cursore del volume di Windows del dispositivo scelto, accanto alla sua scelta: per l'ingresso il volume di registrazione (quello del
 * controllo audio di Windows), per l'uscita il livello Lvl TX, che con l'opzione «volume di Windows» Aurora porta sul volume della scheda. Si invia al rilascio.
 */
export function DeviceVolume({ kind }: { kind: 'input' | 'output' }) {
  const t = useT();
  const client = useQueryClient();
  const windows = isWindowsOs(useEngineOs((v) => v.os));
  const levelInWindows = (useControlValues((v) => v.values.tx_level_windows) ?? 0) === 1;
  const input = useQuery({ queryKey: ['input-level'], queryFn: getInputLevel, retry: false, enabled: kind === 'input', refetchInterval: 3000 });
  const output = useQuery({ queryKey: ['output-level'], queryFn: getOutputLevel, retry: false, enabled: kind === 'output', refetchInterval: 3000 });
  const served = kind === 'input' ? input.data?.volume_pct : output.data ? levelPercent(output.data.attenuation_db) : undefined;
  const [pct, setPct] = useState<number | null>(null);
  const [moving, setMoving] = useState(false);
  useEffect(() => {
    if (!moving && served !== undefined) setPct(Math.round(served));
  }, [served, moving]);
  const failed = kind === 'input' ? input.isError : output.isError;
  if (failed) return <p className="muted dev-vol-off">{t('aud.vol.off')}</p>;
  if (pct === null) return null;
  const commit = async (value: number) => {
    setMoving(false);
    if (kind === 'input') {
      await putInputLevel({ volume_pct: value }).catch(() => undefined);
      await client.invalidateQueries({ queryKey: ['input-level'] });
    } else {
      await putOutputLevel(attenuationFromPercent(value)).catch(() => undefined);
      await client.invalidateQueries({ queryKey: ['output-level'] });
    }
  };
  const id = `dev-vol-${kind}`;
  // «Lvl TX» come volume di Windows o come attenuazione del segnale (di fabbrica: attenuazione, Aurora non tocca il volume di Windows)
  const modeToggle = kind === 'output' && windows ? (
    <label className="dev-vol-mode" title={t('tip.out.mode')}>
      <input type="checkbox" checked={levelInWindows} onChange={(e) => void postControlValue('tx_level_windows', e.target.checked ? 1 : 0).catch(() => undefined)} /> {t('out.mode')}
    </label>
  ) : null;
  return (
    <>
    <div className="dev-vol">
      <label htmlFor={id}>{t(kind === 'input' ? 'aud.vol.in' : 'aud.vol.out')}</label>
      <input
        id={id}
        type="range"
        min={0}
        max={100}
        step={1}
        value={pct}
        title={t(kind === 'input' ? 'tip.aud.vol.in' : 'tip.aud.vol.out')}
        onChange={(e) => {
          setMoving(true);
          setPct(Number(e.target.value));
        }}
        onPointerUp={(e) => void commit(Number((e.target as HTMLInputElement).value))}
        onKeyUp={(e) => void commit(Number((e.target as HTMLInputElement).value))}
      />
      <span className="data dev-vol-val">{pct}%</span>
      {kind === 'input' && input.data?.auto ? <span className="muted dev-vol-auto">AUTO</span> : null}
    </div>
    {modeToggle}
    </>
  );
}
