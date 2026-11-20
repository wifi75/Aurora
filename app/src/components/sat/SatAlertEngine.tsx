import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef } from 'react';
import { getSat, getSatPassesAll } from '../../api/client';
import { armAudio } from '../../lib/satAlertSound';
import { mergePasses, type AlertPass } from '../../lib/satAlerts';
import { rememberPasses, satLabel, satLabelStore, type KnownPass } from '../../lib/satLabel';
import { normalizeSatSelection } from '../../lib/satSelection';
import { useSatAlerts } from '../../lib/useSatAlerts';

const NO_POSITIONS: never[] = [];
const TICK_MS = 500;

/**
 * Motore invisibile degli avvisi dei satelliti, montato una volta sola alla radice dell'app (solo finestra principale del PC):
 * da qualunque pagina suona il preavviso e tiene aggiornata l'etichetta della barra in alto. Non legge le posizioni in tempo
 * reale (AOS e LOS suonano solo nella pagina Satelliti, come prima) e non guarda il rotore. Se non si segue nessun satellite
 * non chiede altro che le impostazioni, ogni 30 secondi.
 */
export function SatAlertEngine() {
  const snap = useQuery({ queryKey: ['sat'], queryFn: getSat, retry: false, refetchInterval: 30_000 });
  const data = snap.data;
  const tracked = data?.settings.tracked;
  const trackedKey = tracked?.join(',');
  const passesAll = useQuery({
    queryKey: ['sat-passes-all', 24, 5, trackedKey],
    queryFn: () => getSatPassesAll(24, 5),
    retry: false,
    refetchInterval: 60_000,
    enabled: !!data?.observer && (tracked?.length ?? 0) > 0,
  });

  const passes = useMemo<AlertPass[]>(() => mergePasses((passesAll.data ?? []).map((p) => ({ norad: p.norad, name: p.name, aos_unix: p.aos_unix, los_unix: p.los_unix, max_el_deg: p.max_el_deg, aos_az_deg: p.aos_az_deg }))), [passesAll.data]);
  const selected = data ? normalizeSatSelection(data.settings).selected : null;
  const minEl = data?.settings.min_el_deg ?? 5;

  const alerts = useSatAlerts({ passes, positions: NO_POSITIONS, tracked: tracked ?? [], selected, minEl, serverNow: data?.now.time_unix ?? Date.now() / 1000, rotorOn: false });

  // l'audio del browser parte solo dopo un gesto dell'utente: il primo clic o tasto, su qualunque pagina, lo sblocca
  useEffect(() => {
    const arm = () => armAudio();
    window.addEventListener('pointerdown', arm, { once: true });
    window.addEventListener('keydown', arm, { once: true });
    return () => {
      window.removeEventListener('pointerdown', arm);
      window.removeEventListener('keydown', arm);
    };
  }, []);

  const latest = useRef({ passes, tracked: tracked ?? [], selected, minEl, settings: alerts.settings, ready: !!data });
  latest.current = { passes, tracked: tracked ?? [], selected, minEl, settings: alerts.settings, ready: !!data };
  const known = useRef<KnownPass[]>([]);
  const clockNow = alerts.clockNow;

  useEffect(() => {
    const tick = () => {
      const a = latest.current;
      if (!a.ready) {
        satLabelStore.setLabel(null);
        return;
      }
      const now = clockNow();
      known.current = rememberPasses(known.current, a.passes, now);
      satLabelStore.setLabel(satLabel({ passes: a.passes, known: known.current, dismissed: satLabelStore.get().dismissed, now, settings: a.settings, selected: a.selected, tracked: a.tracked, minEl: a.minEl }));
    };
    tick();
    const id = window.setInterval(tick, TICK_MS);
    return () => {
      window.clearInterval(id);
      satLabelStore.setLabel(null);
    };
  }, [clockNow]);

  return null;
}
