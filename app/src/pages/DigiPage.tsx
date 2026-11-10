import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { addLogQso, ApiError, digiAbort, digiRx, digiTransmit, getDigi, putDigi, type DigiMacro, type DigiSettings, type DigiSnapshot } from '../api/client';
import { ControlBar } from '../components/digi/ControlBar';
import { MacroEditor, type MacroScope } from '../components/digi/MacroEditor';
import { useT } from '../i18n';
import { appendDigiTx, clearDigiText, getDigiText, onDigi, onDigiText } from '../lib/digiBus';
import { bandOf } from '../lib/format';
import { effectiveMacros, macroNameForKey, runMacro, type Macro, type MacroAction, type MacroContext } from '../lib/macroLang';
import { formatDuration } from '../lib/sstv';
import { browserStorage } from '../lib/theme';
import { toast } from '../lib/toast';
import { useStation } from '../state/station';
import '../styles/digi.css';

const FIELDS_KEY = 'aurora.digi.campi';
const BAR_KEY = 'aurora.digi.barraEstesa';
const RTTY_SHIFTS = [85, 170, 200, 425, 450, 850];
const RTTY_BAUDS = [45, 45.45, 50, 75, 100];
const MODE_NAMES: Record<DigiSettings['mode'], string> = { psk31: 'PSK31', psk63: 'PSK63', rtty: 'RTTY' };
/** Secondi fra un CQ automatico e il successivo. */
const AUTOCQ_DELAY_S = 8;

/** I dati della stazione che non cambiano da un QSO all'altro: restano salvati. */
interface Personal {
  myname: string;
  myqth: string;
  rsts: string;
  rstr: string;
}

/** I dati del QSO in corso. */
interface Qso {
  call: string;
  name: string;
  qth: string;
  wwloc: string;
  notes: string;
}

const EMPTY_QSO: Qso = { call: '', name: '', qth: '', wwloc: '', notes: '' };

function readPersonal(): Personal {
  try {
    const raw = browserStorage()?.getItem(FIELDS_KEY);
    if (raw) {
      const p = JSON.parse(raw) as Partial<Personal>;
      return { myname: p.myname ?? '', myqth: p.myqth ?? '', rsts: p.rsts ?? '599', rstr: p.rstr ?? '599' };
    }
  } catch {
    // senza archivio si riparte dai valori di base
  }
  return { myname: '', myqth: '', rsts: '599', rstr: '599' };
}

function readExtended(): boolean {
  try {
    return browserStorage()?.getItem(BAR_KEY) === '1';
  } catch {
    return false;
  }
}

const toMacro = (m: DigiMacro): Macro => ({ name: m.name, label: m.label, text: m.text, color: m.color });

/** Il modo del programma per il nome che si usa nelle macro (BPSK31, RTTY...), o `null` se non lo conosce. */
function modeFromName(name: string): DigiSettings['mode'] | null {
  switch (name.toUpperCase()) {
    case 'PSK31':
    case 'BPSK31': return 'psk31';
    case 'PSK63':
    case 'BPSK63': return 'psk63';
    case 'RTTY': return 'rtty';
    default: return null;
  }
}

/** La pagina dei modi testuali: PSK31, PSK63 e RTTY. Barra dei comandi, ricezione del testo, trasmissione e dati del QSO. */
export function DigiPage() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['digi'], queryFn: getDigi, retry: false, refetchInterval: 1000 });
  const data = snap.data;
  const refresh = useCallback(() => void client.invalidateQueries({ queryKey: ['digi'] }), [client]);

  // un evento di stato dal flusso aggiorna subito la pagina
  useEffect(() => onDigi((ev) => (ev.kind === 'rx_status' || ev.kind === 'tx' ? refresh() : undefined)), [refresh]);

  return (
    <div className="digi-page">
      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? <DigiBody snap={data} onChanged={refresh} /> : null}
    </div>
  );
}

function DigiBody({ snap, onChanged }: { snap: DigiSnapshot; onChanged: () => void }) {
  const t = useT();
  const client = useQueryClient();
  const status = useStation((s) => s.status);
  const text = useSyncExternalStore(onDigiText, getDigiText);
  const box = useRef<HTMLPreElement>(null);
  const txBox = useRef<HTMLTextAreaElement>(null);
  const [tx, setTx] = useState('');
  const [qso, setQso] = useState<Qso>(EMPTY_QSO);
  const [personal, setPersonal] = useState<Personal>(readPersonal);
  const [startAt, setStartAt] = useState<Date | null>(null);
  const [extended, setExtended] = useState(readExtended);
  const [problem, setProblem] = useState('');
  const [editing, setEditing] = useState<string | null>(null);
  const [autoCq, setAutoCq] = useState(false);
  const [rxFreq, setRxFreq] = useState(snap.settings.rx_freq_hz);
  const [txFreq, setTxFreq] = useState(snap.settings.tx_freq_hz);
  const s = snap.settings;
  const sending = snap.tx.state === 'sending';
  const badge = sending ? 'sending' : snap.rx.listening ? 'listening' : 'off';
  const level = Math.max(0, Math.min(100, ((snap.rx.level_db + 60) / 60) * 100));
  const rtty = s.mode === 'rtty';

  const macros = useMemo(
    () => effectiveMacros(s.macros.map(toMacro), s.mode_macros[s.mode]?.map(toMacro)),
    [s.macros, s.mode_macros, s.mode],
  );

  // l'ultimo stato, per i comandi dei tasti e del CQ automatico (che non devono usare valori vecchi)
  const live = useRef({ snap, tx, qso, personal, startAt, status, macros });
  live.current = { snap, tx, qso, personal, startAt, status, macros };

  useEffect(() => {
    const el = box.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [text]);

  useEffect(() => setRxFreq(s.rx_freq_hz), [s.rx_freq_hz]);
  useEffect(() => setTxFreq(s.tx_freq_hz), [s.tx_freq_hz]);

  const save = useCallback(
    (patch: Partial<DigiSettings>) => {
      void putDigi({ ...live.current.snap.settings, ...patch }).then(
        (next) => {
          client.setQueryData(['digi'], next);
          onChanged();
        },
        () => setProblem(t('digi.saveError')),
      );
    },
    [client, onChanged, t],
  );

  const setPersonalField = (patch: Partial<Personal>) => {
    const next = { ...personal, ...patch };
    setPersonal(next);
    try {
      browserStorage()?.setItem(FIELDS_KEY, JSON.stringify(next));
    } catch {
      // senza archivio i dati restano finche' la pagina e' aperta
    }
  };

  const send = useCallback(
    (body: string) => {
      const message = body.trim();
      if (message === '') return;
      setProblem('');
      digiTransmit(message).then(
        () => {
          appendDigiTx(message);
          setTx('');
          onChanged();
        },
        (e: unknown) => {
          const code = e instanceof ApiError ? e.code : '';
          setProblem(code === 'tx_disabled' ? t('digi.tx.disabled') : code === 'busy' ? t('digi.tx.busy') : code === 'no_transmitter' ? t('digi.tx.notReady') : t('digi.tx.error'));
        },
      );
    },
    [onChanged, t],
  );

  const context = (): MacroContext => {
    const l = live.current;
    const dial = l.status?.dial_hz ?? 0;
    return {
      mycall: l.status?.de_call ?? '',
      myname: l.personal.myname,
      myqth: l.personal.myqth,
      mywwloc: l.status?.de_grid ?? '',
      call: l.qso.call,
      name: l.qso.name,
      qth: l.qso.qth,
      wwloc: l.qso.wwloc,
      rstr: l.personal.rstr,
      rsts: l.personal.rsts,
      notes: l.qso.notes,
      nrr: '',
      nrs: '',
      startTime: l.startAt,
      now: new Date(),
      dialHz: dial,
      band: dial > 0 ? (bandOf(dial) ?? '') : '',
      modeName: MODE_NAMES[l.snap.settings.mode],
      qsoNumber: 1,
    };
  };

  const saveQso = (when: Date) => {
    const l = live.current;
    const call = l.qso.call.trim().toUpperCase();
    if (call === '') {
      toast(t('digi.run.noCall'), 'error');
      return;
    }
    const dial = l.status?.dial_hz ?? 0;
    const at = l.startAt ?? when;
    void addLogQso({
      call,
      date: at.toISOString().slice(0, 10),
      time: at.toISOString().slice(11, 16),
      band: dial > 0 ? (bandOf(dial) ?? '') : '',
      freq_hz: dial,
      mode: MODE_NAMES[l.snap.settings.mode],
      rst_sent: l.personal.rsts,
      rst_rcvd: l.personal.rstr,
      grid: l.qso.wwloc,
      name: l.qso.name,
      comment: l.qso.notes,
    }).then(
      () => toast(t('digi.run.saved', { call }), 'ok'),
      () => toast(t('digi.tx.error'), 'error'),
    );
  };

  const beep = () => {
    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      osc.frequency.value = 880;
      osc.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.18);
      window.setTimeout(() => void ctx.close(), 400);
    } catch {
      // senza audio nel browser il comando non fa niente
    }
  };

  /** Una macro: i comandi di programma nell'ordine, il testo in trasmissione o nella casella, poi il salvataggio e la pulizia. */
  const execute = (script: string) => {
    const l = live.current;
    const result = runMacro(script, context());
    let toTx = false;
    let rxClear = false;
    let clearTx = false;
    let wantSave = false;
    const apply = (a: MacroAction) => {
      switch (a.type) {
        case 'tx':
        case 'txToggle': toTx = true; break;
        case 'rx': rxClear = rxClear || a.clear; break;
        case 'clearTx': clearTx = true; break;
        case 'clearRx': clearDigiText(); break;
        case 'clearWindow': clearDigiText(); clearTx = true; break;
        case 'clearQso': setQso(EMPTY_QSO); setStartAt(null); break;
        case 'fixStartTime': setStartAt(new Date()); break;
        case 'saveQso': wantSave = true; break;
        case 'mode': {
          const m = modeFromName(a.mode);
          if (m) save({ mode: m });
          else toast(t('digi.run.modeUnsupported', { mode: a.mode }), 'info');
          break;
        }
        case 'baud': {
          const cur = l.snap.settings.mode;
          if (cur !== 'rtty' && Math.abs(a.baud - 31.25) < 0.5) save({ mode: 'psk31' });
          else if (cur !== 'rtty' && Math.abs(a.baud - 62.5) < 0.5) save({ mode: 'psk63' });
          else if (cur === 'rtty' && a.baud >= 30 && a.baud <= 300) save({ rtty_baud: a.baud });
          else toast(t('digi.run.baudUnsupported', { baud: a.baud }), 'info');
          break;
        }
        case 'shift': save({ rtty_shift_hz: a.hz }); break;
        case 'audioFq': {
          const hz = Math.max(200, Math.min(3500, a.relative ? l.snap.settings.rx_freq_hz + a.hz : a.hz));
          save({ rx_freq_hz: hz });
          break;
        }
        case 'cursor': if (a.target === 'TX') txBox.current?.focus(); break;
        case 'beep': beep(); break;
        case 'autoCq': setAutoCq((on) => !on); break;
        case 'txe':
        case 'rxe': break;
        case 'unknown': toast(t('digi.run.unknown', { tag: a.tag }), 'info'); break;
        default: toast(t('digi.run.unknown', { tag: a.type.toUpperCase() }), 'info');
      }
    };
    result.actions.forEach(apply);
    const body = result.text.replace(/^[\r\n\t ]+|[\r\n\t ]+$/g, '');
    if (toTx) {
      // un comando <TX> da solo trasmette quello che c'e' nella casella
      send(body !== '' ? body : l.tx);
    } else if (body !== '') {
      setTx((v) => (clearTx || v === '' ? body : `${v} ${body}`));
    } else if (clearTx) {
      setTx('');
    }
    if (clearTx && toTx) setTx('');
    if (wantSave) saveQso(new Date());
    if (rxClear && !toTx) setTx('');
  };

  const runMacroByName = (name: string): boolean => {
    const m = live.current.macros.get(name);
    if (!m || (m.label.trim() === '' && m.text.trim() === '')) return false;
    execute(m.text);
    return true;
  };

  // i tasti F1-F12 con Ctrl e Shift, il «+» del tastierino (macro Gray+) e Esc che ferma la trasmissione
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (editing !== null) return;
      if (e.key === 'Escape' && live.current.snap.tx.state === 'sending') {
        void digiAbort().then(onChanged, onChanged);
        return;
      }
      const name = e.code === 'NumpadAdd' ? 'Gray+' : macroNameForKey(e);
      if (name && runMacroByName(name)) e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  // CQ automatico: ripete la macro «CQ» finche' non si spegne, con una pausa fra una chiamata e l'altra
  useEffect(() => {
    if (!autoCq || sending || !snap.can_transmit) return;
    const m = live.current.macros.get('AutoCqText') ?? live.current.macros.get('F2');
    if (!m || m.text.trim() === '') {
      toast(t('digi.run.autoCqNoMacro'), 'info');
      setAutoCq(false);
      return;
    }
    const timer = window.setTimeout(() => execute(m.text), AUTOCQ_DELAY_S * 1000);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoCq, sending, snap.can_transmit, snap.tx.state]);

  useEffect(() => {
    toast(autoCq ? t('digi.run.autoCqOn', { s: AUTOCQ_DELAY_S }) : t('digi.run.autoCqOff'), 'info');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoCq]);

  const toggleExtended = () => {
    const next = !extended;
    setExtended(next);
    try {
      browserStorage()?.setItem(BAR_KEY, next ? '1' : '0');
    } catch {
      // senza archivio la scelta vale finche' la pagina e' aperta
    }
  };

  const editedMacro: Macro | null = useMemo(() => {
    if (editing === null) return null;
    return macros.get(editing) ?? { name: editing, label: '', text: '', color: null };
  }, [editing, macros]);
  const editedScope: MacroScope = editing !== null && (s.mode_macros[s.mode] ?? []).some((m) => m.name === editing) ? 'mode' : 'all';

  const saveMacro = (m: Macro, where: MacroScope) => {
    const original = editing ?? m.name;
    const blank = m.label.trim() === '' && m.text.trim() === '';
    const strip = (list: DigiMacro[], name: string) => list.filter((x) => x.name !== name);
    const entry: DigiMacro = { name: m.name, label: m.label, text: m.text, color: m.color };
    let all = [...s.macros];
    const modeSets = { ...s.mode_macros };
    let forMode = [...(modeSets[s.mode] ?? [])];
    // il tasto di partenza si libera dall'insieme che si sta modificando se e' stato cambiato
    if (original !== m.name) {
      all = strip(all, original);
      forMode = strip(forMode, original);
    }
    if (where === 'all') {
      all = strip(all, m.name);
      forMode = strip(forMode, m.name);
      if (!blank) all.push(entry);
    } else {
      forMode = strip(forMode, m.name);
      // vuota per questo modo = il tasto resta libero anche se la macro di base esiste
      if (!blank || all.some((x) => x.name === m.name)) forMode.push(blank ? { name: m.name, label: '', text: '', color: null } : entry);
    }
    modeSets[s.mode] = forMode;
    save({ macros: all, mode_macros: modeSets });
    setEditing(null);
  };

  return (
    <>
      <header className="digi-head">
        <h1>{t('digi.title')}</h1>
        <span className={`sstv-badge sstv-badge--${badge}`} role="status">
          <span className="sstv-dot" aria-hidden="true" />
          {t(`digi.state.${badge}`)}
        </span>
        <div className="sstv-views" role="tablist" aria-label={t('digi.mode')}>
          {snap.modes.map((m) => (
            <button key={m.id} type="button" role="tab" aria-selected={s.mode === m.id} className={s.mode === m.id ? 'on' : ''} onClick={() => (s.mode === m.id ? undefined : save({ mode: m.id as DigiSettings['mode'] }))}>
              {m.name}
            </button>
          ))}
        </div>
      </header>

      <ControlBar macros={macros} extended={extended} onToggleExtended={toggleExtended} onRun={(m) => execute(m.text)} onEdit={setEditing} />

      <section className="digi-qso" aria-label={t('digi.f.call')}>
        <label>
          <span>{t('digi.f.call')}</span>
          <input value={qso.call} maxLength={16} className="digi-call" onChange={(e) => setQso({ ...qso, call: e.target.value.toUpperCase() })} />
        </label>
        <label>
          <span>{t('digi.f.name')}</span>
          <input value={qso.name} maxLength={30} onChange={(e) => setQso({ ...qso, name: e.target.value })} />
        </label>
        <label>
          <span>{t('digi.f.qth')}</span>
          <input value={qso.qth} maxLength={40} onChange={(e) => setQso({ ...qso, qth: e.target.value })} />
        </label>
        <label>
          <span>{t('digi.f.rsts')}</span>
          <input value={personal.rsts} maxLength={4} className="digi-rst" onChange={(e) => setPersonalField({ rsts: e.target.value })} />
        </label>
        <label>
          <span>{t('digi.f.rstr')}</span>
          <input value={personal.rstr} maxLength={4} className="digi-rst" onChange={(e) => setPersonalField({ rstr: e.target.value })} />
        </label>
        <label>
          <span>{t('digi.f.notes')}</span>
          <input value={qso.notes} maxLength={80} onChange={(e) => setQso({ ...qso, notes: e.target.value })} />
        </label>
      </section>

      <div className="digi-grid">
        <section className="digi-rx" aria-label={t('digi.rx.title')}>
          <div className="digi-bar">
            <strong>{t('digi.rx.title')}</strong>
            <span className="sstv-meter digi-level" title={`${t('digi.level')}: ${Math.round(snap.rx.level_db)} dB`}>
              <span style={{ width: `${level}%` }} />
            </span>
            <button type="button" className="glass sstv-btn" onClick={() => void (snap.rx.listening ? digiRx('stop') : digiRx('start')).then(onChanged, onChanged)}>
              {snap.rx.listening ? t('digi.rx.stop') : t('digi.rx.start')}
            </button>
            <button type="button" className="glass sstv-btn" onClick={() => clearDigiText()}>
              {t('digi.rx.clear')}
            </button>
          </div>
          <pre className="digi-text" ref={box} aria-live="off">
            {text === '' ? <span className="digi-empty">{t('digi.rx.empty')}</span> : text}
          </pre>
        </section>

        <aside className="digi-side">
          <div className="digi-freq">
            <label>
              <span>{t('digi.freq.rx')}</span>
              <input
                type="number"
                min={200}
                max={3500}
                step={1}
                value={rxFreq}
                onChange={(e) => setRxFreq(Number(e.target.value))}
                onBlur={() => (rxFreq !== s.rx_freq_hz && rxFreq >= 200 && rxFreq <= 3500 ? save({ rx_freq_hz: rxFreq }) : setRxFreq(s.rx_freq_hz))}
                onKeyDown={(e) => (e.key === 'Enter' ? e.currentTarget.blur() : undefined)}
              />
            </label>
            <input
              type="range"
              min={200}
              max={3500}
              step={1}
              value={rxFreq}
              aria-label={t('digi.freq.rx')}
              onChange={(e) => setRxFreq(Number(e.target.value))}
              onPointerUp={() => (rxFreq !== s.rx_freq_hz ? save({ rx_freq_hz: rxFreq }) : undefined)}
              onKeyUp={() => (rxFreq !== s.rx_freq_hz ? save({ rx_freq_hz: rxFreq }) : undefined)}
            />
            <p className="sstv-hint">{t('digi.freq.hint')}</p>
            <label className="sstv-check">
              <input type="checkbox" checked={s.tx_follow_rx} onChange={(e) => save({ tx_follow_rx: e.target.checked })} />
              {t('digi.freq.follow')}
            </label>
            {s.tx_follow_rx ? null : (
              <label>
                <span>{t('digi.freq.tx')}</span>
                <input
                  type="number"
                  min={200}
                  max={3500}
                  step={1}
                  value={txFreq}
                  onChange={(e) => setTxFreq(Number(e.target.value))}
                  onBlur={() => (txFreq !== s.tx_freq_hz && txFreq >= 200 && txFreq <= 3500 ? save({ tx_freq_hz: txFreq }) : setTxFreq(s.tx_freq_hz))}
                  onKeyDown={(e) => (e.key === 'Enter' ? e.currentTarget.blur() : undefined)}
                />
              </label>
            )}
            {rtty ? (
              <div className="digi-two">
                <label>
                  <span>{t('digi.rtty.shift')}</span>
                  <select value={s.rtty_shift_hz} onChange={(e) => save({ rtty_shift_hz: Number(e.target.value) })}>
                    {(RTTY_SHIFTS.includes(s.rtty_shift_hz) ? RTTY_SHIFTS : [...RTTY_SHIFTS, s.rtty_shift_hz].sort((a, b) => a - b)).map((v) => (
                      <option key={v} value={v}>
                        {v}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>{t('digi.rtty.baud')}</span>
                  <select value={s.rtty_baud} onChange={(e) => save({ rtty_baud: Number(e.target.value) })}>
                    {(RTTY_BAUDS.includes(s.rtty_baud) ? RTTY_BAUDS : [...RTTY_BAUDS, s.rtty_baud].sort((a, b) => a - b)).map((v) => (
                      <option key={v} value={v}>
                        {v}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            ) : null}
            <div className="digi-two">
              <label>
                <span>{t('digi.f.myname')}</span>
                <input value={personal.myname} maxLength={30} onChange={(e) => setPersonalField({ myname: e.target.value })} />
              </label>
              <label>
                <span>{t('digi.f.myqth')}</span>
                <input value={personal.myqth} maxLength={40} onChange={(e) => setPersonalField({ myqth: e.target.value })} />
              </label>
            </div>
            <label className="sstv-check">
              <input type="checkbox" checked={s.auto_receive} onChange={(e) => save({ auto_receive: e.target.checked })} />
              {t('digi.autoReceive')}
            </label>
          </div>
        </aside>
      </div>

      <section className="digi-tx" aria-label={t('digi.tx.title')}>
        <div className="digi-bar">
          <strong>{t('digi.tx.title')}</strong>
        </div>
        <textarea
          ref={txBox}
          className="digi-box"
          rows={4}
          value={tx}
          placeholder={t('digi.tx.placeholder')}
          onChange={(e) => setTx(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && !sending) send(tx);
          }}
        />
        <div className="sstv-sendbar">
          {sending ? (
            <>
              <div className="sstv-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(snap.tx.progress * 100)}>
                <span style={{ width: `${snap.tx.progress * 100}%` }} />
                <em>{t('digi.tx.progress', { done: formatDuration(snap.tx.elapsed_s), total: formatDuration(snap.tx.total_s) })}</em>
              </div>
              <button type="button" className="glass sstv-btn sstv-btn--danger sstv-btn--big" onClick={() => void digiAbort().then(onChanged, onChanged)}>
                {t('digi.tx.stop')}
              </button>
            </>
          ) : (
            <>
              <span className="sstv-sendinfo">
                {snap.tx.state === 'done' ? <span className="sstv-ok">{t('digi.tx.done')}</span> : null}
                {snap.tx.state === 'aborted' ? <span className="sstv-warn">{t('digi.tx.aborted')}</span> : null}
                {snap.tx.state === 'error' ? <span className="sstv-bad">{snap.tx.message ?? t('digi.tx.error')}</span> : null}
              </span>
              <button type="button" className="glass sstv-btn sstv-btn--primary sstv-btn--big" disabled={!snap.can_transmit || tx.trim() === ''} onClick={() => send(tx)}>
                {t('digi.tx.send')}
              </button>
            </>
          )}
        </div>
        {!snap.can_transmit ? <p className="notice">{t('digi.tx.notReady')}</p> : null}
        {problem ? (
          <p className="notice sstv-bad" role="alert">
            {problem}
          </p>
        ) : null}
        <p className="sstv-hint">{t('digi.macros.hint')}</p>
      </section>

      {editing !== null && editedMacro ? (
        <MacroEditor key={editing} macro={editedMacro} modeName={MODE_NAMES[s.mode]} scope={editedScope} onSave={saveMacro} onCancel={() => setEditing(null)} />
      ) : null}
    </>
  );
}
