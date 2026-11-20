import { useCallback, useEffect, useState } from 'react';
import { ALERT_STORAGE_KEY, loadAlertSettings, saveAlertSettings } from './satAlerts';
import { ALERT_SETTINGS_EVENT } from './useSatAlerts';

/**
 * L'opzione «etichetta e suono dei satelliti in ogni pagina» (spenta di fabbrica). Sta con le altre scelte degli avvisi dei satelliti:
 * la si cambia dalle Impostazioni o dalla campanella della pagina Satelliti, e tutte le copie si aggiornano.
 */
export function useBarLabel(): [boolean, (on: boolean) => void] {
  const [on, setOn] = useState(() => loadAlertSettings().barLabel);
  useEffect(() => {
    const reload = () => setOn(loadAlertSettings().barLabel);
    const onStorage = (e: StorageEvent) => {
      if (e.key === null || e.key === ALERT_STORAGE_KEY) reload();
    };
    window.addEventListener(ALERT_SETTINGS_EVENT, reload);
    window.addEventListener('storage', onStorage);
    return () => {
      window.removeEventListener(ALERT_SETTINGS_EVENT, reload);
      window.removeEventListener('storage', onStorage);
    };
  }, []);
  const set = useCallback((value: boolean) => {
    saveAlertSettings({ ...loadAlertSettings(), barLabel: value });
    setOn(value);
    window.dispatchEvent(new Event(ALERT_SETTINGS_EVENT));
  }, []);
  return [on, set];
}
