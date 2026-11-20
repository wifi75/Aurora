import { useCallback, useEffect, useRef, useState } from 'react';
import type { SatPosition } from '../api/client';
import { alertViews, applyMarks, dueAlerts, loadAlertSettings, remember, rotorNotice, saveAlertSettings, type AlertKind, type AlertMark, type AlertPass, type AlertRecord, type AlertView, type LiveSat, type RotorNotice, type SatAlertSettings, type SoundChoice } from './satAlerts';
import { audioBlocked, playAlert, previewAlert } from './satAlertSound';

interface Args {
  passes: readonly AlertPass[];
  positions: readonly SatPosition[];
  tracked: readonly number[];
  selected: number | null;
  minEl: number;
  /** L'ora UTC del core (secondi) all'ultimo aggiornamento della pagina. */
  serverNow: number;
  /** Rotore acceso nelle impostazioni dei satelliti (per l'avviso di spostamento). */
  rotorOn: boolean;
}

const TICK_MS = 500;

const soundFor = (s: SatAlertSettings, kind: AlertKind): SoundChoice => (kind === 'pre' ? s.soundPre : kind === 'aos' ? s.soundAos : kind === 'los' ? s.soundLos : s.soundRotor);

/**
 * Gli avvisi della pagina Satelliti: tiene le impostazioni (salvate nel browser), controlla due volte al secondo se c'e' un
 * preavviso, un AOS o una LOS da dare, li suona e dice alla mappa cosa disegnare. L'orologio e' quello del core,
 * interpolato fra un aggiornamento e l'altro.
 */
export function useSatAlerts(args: Args) {
  const [settings, setSettings] = useState<SatAlertSettings>(() => loadAlertSettings());
  const [views, setViews] = useState<AlertView[]>([]);
  const [blocked, setBlocked] = useState(false);
  const [rotor, setRotor] = useState<RotorNotice | null>(null);
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
      const common = { passes: a.passes, now, settings: s, selected: a.selected, tracked: a.tracked, live, minEl: a.minEl, log: st.log, rotorOn: a.rotorOn };
      const events = dueAlerts({ ...common, prevNow: st.prevNow, prevLive: st.prevLive });
      for (const ev of events) {
        const choice = soundFor(s, ev.kind);
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
      const rn = rotorNotice(common);
      setRotor((cur) => (cur && rn && cur.norad === rn.norad && cur.az === rn.az && Math.round(cur.secondsToAos) === Math.round(rn.secondsToAos) ? cur : rn));
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

  /** Ascolta il suono scelto per l'evento, o `choice` se lo si sta ancora scegliendo. */
  const test = useCallback((kind: AlertKind, choice?: SoundChoice) => {
    const s = settingsRef.current;
    previewAlert(kind, choice ?? soundFor(s, kind), s.volume);
  }, []);

  return { settings, update, views, blocked, test, rotor };
}
