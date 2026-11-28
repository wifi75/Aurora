import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { getPota } from '../api/client';
import { usePotaSpots } from '../lib/potaSpots';
import { useSpecialCq } from '../lib/specialCqPrefs';

/**
 * Porta gli spot POTA letti dal core nella memoria dell'interfaccia, dove la lista delle decodifiche li cerca per l'etichetta dei «CQ POTA».
 * Chiede al core solo con i CQ speciali accesi; il core legge api.pota.app solo se «Cerca il parco negli spot POTA» e' acceso.
 */
export function PotaFeeder() {
  const on = useSpecialCq((s) => s.prefs.enabled);
  const setSpots = usePotaSpots((s) => s.set);
  const pota = useQuery({ queryKey: ['online-pota'], queryFn: getPota, enabled: on, retry: false, refetchInterval: 60_000 });
  const spots = on && pota.data?.enabled ? pota.data.pota.spots : null;
  useEffect(() => {
    setSpots(spots ? Object.values(spots) : []);
  }, [spots, setSpots]);
  return null;
}
