import { useCallback, useEffect, useRef, useState } from 'react';
import type { SatPosition } from '../api/client';
import { alertViews, applyMarks, dueAlerts, loadAlertSettings, remember, saveAlertSettings, type AlertKind, type AlertMark, type AlertPass, type AlertRecord, type AlertView, type LiveSat, type SatAlertSettings } from './satAlerts';
import { audioBlocked, playAlert, previewAlert } from './satAlertSound';

interface Args {
  passes: readonly AlertPass[];
  positions: readonly SatPosition[];
  tracked: readonly number[];
  selected: number | null;
  minEl: number;
  /** L'ora UTC del core (secondi) all'ultimo aggiornamento della pagina. */
  serverNow: number;
}

const TICK_MS = 500;

/**
 * Gli avvisi della pagina Satelliti: tiene le impostazioni (salvate nel browser), controlla due volte al secondo se c'e' un
 * preavviso, un AOS o una LOS da dare, li suona e dice alla mappa cosa disegnare. L'orologio e' quello del core,
 * interpolato fra un aggiornamento e l'altro.
 */
export function useSatAlerts(args: Args) {
  const [settings, setSettings] = useState<SatAlertSettings>(() => loadAlertSettings());
  const [views, setViews] = useState<AlertView[]>([]);
  const [blocked, setBlocked] = useState(false);
  const latest = useRef(args);
  latest.current = args;
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const clock = useRef({ base: args.serverNow, at: Date.now() });
  const state = useRef<{ log: AlertRecord[]; marks: Map<number, AlertMark>; prevLive: Map<number, LiveSat>; prevNow: number | null; shown: string }>({ log: [], marks: new Map(), prevLive: new Map(), prevNow: null, shown: '' });

  useEffect(() => {
    clock.current = { base: args.serverNow, at: Date.now() };
  }, [args.serverNow]);

  useEffect(() => {
    const tick = () => {
      const a = latest.current;
      const s = settingsRef.current;
      const st = state.current;
      const now = clock.current.base + (Date.now() - clock.current.at) / 1000;
      const live = new Map<number, LiveSat>(a.positions.map((p) => [p.norad, { visible: p.visible, name: p.name }]));
      const common = { passes: a.passes, now, settings: s, selected: a.selected, tracked: a.tracked, live, minEl: a.minEl, log: st.log };
      const events = dueAlerts({ ...common, prevNow: st.prevNow, prevLive: st.prevLive });
      for (const ev of events) {
        const choice = ev.kind === 'pre' ? s.soundPre : ev.kind === 'aos' ? s.soundAos : s.soundLos;
        playAlert(ev.kind, choice, s.volume);
      }
      st.log = remember(st.log, events, now);
      st.marks = applyMarks(st.marks, events, now);
      st.prevLive = live;
      st.prevNow = now;
      const next = alertViews({ ...common, log: st.log, marks: st.marks });
      const key = JSON.stringify(next);
      if (key !== st.shown) {
        st.shown = key;
        setViews(next);
      }
      setBlocked(s.enabled && audioBlocked());
    };
    tick();
    const id = window.setInterval(tick, TICK_MS);
    return () => window.clearInterval(id);
  }, []);

  const update = useCallback((patch: Partial<SatAlertSettings>) => {
    setSettings((cur) => {
      const next = { ...cur, ...patch };
      saveAlertSettings(next);
      return next;
    });
  }, []);

  const test = useCallback((kind: AlertKind) => {
    const s = settingsRef.current;
    previewAlert(kind, kind === 'pre' ? s.soundPre : kind === 'aos' ? s.soundAos : s.soundLos, s.volume);
  }, []);

  return { settings, update, views, blocked, test };
}
