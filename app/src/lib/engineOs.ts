import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { getHealth } from '../api/client';
import { useEngineOs } from '../i18n';

/** Tiene aggiornato il sistema su cui gira il motore (`/health`, campo `os`): lo usa la traduzione per i testi che nominano Windows. */
export function useEngineOsSync(): void {
  const { data } = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  const os = data?.os;
  useEffect(() => {
    if (os) useEngineOs.getState().setOs(os);
  }, [os]);
}
