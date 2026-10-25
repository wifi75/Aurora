import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type CSSProperties, type ReactNode } from 'react';
import { getControls, postControl, type ControlId } from '../api/client';
import type { EngineSettings } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { applyColors, defaultColors, inkFor, kindKey, readColors, saveColors, type ColorKey, type TagColors } from '../lib/colors';
import { useNotify } from '../lib/notify';
import { SOUND_NAMES, playNamed, readAlerts, saveAlerts, webAudioContext, type AlertPrefs, type SoundName } from '../lib/alerts';
import { useNotifyView, workedLook, type AlertKey, type NotifyView } from '../lib/notifyView';
import { AlertSwitch } from '../components/AlertSwitch';
import { usePrefs, type Prefs } from '../lib/prefs';
import { browserStorage } from '../lib/theme';
import { LanguageSelect } from '../components/LanguageSelect';
import { ThemeToggle } from '../components/ThemeToggle';
import { useStation } from '../state/station';
import { ApplyBar, SettingField, useDraft, useSettings, type Draft } from './NativeSettings';
import { RadioDisplayPicker } from './RadioDisplayPicker';
import { Check, Help } from './common';

/** Il valore di una voce adesso: la modifica non ancora applicata, o quello salvato. */
function valueOf(doc: EngineSettings, draft: Draft, key: string): string {
  return draft.changes[key] ?? doc.values[key] ?? '';
}

function isOn(doc: EngineSettings, draft: Draft, key: string): boolean {
  return valueOf(doc, draft, key) === 'true';
}

/** Una casella di una voce del motore proprio: viva si modifica (con Applica), altrimenti e' spenta. */
function EngineBox({ k, label, doc, draft, locked = false }: { k: string; label: string; doc: EngineSettings; draft: Draft; locked?: boolean }) {
  const live = doc.items[k]?.state === 'live';
  const changed = draft.changes[k] !== undefined;
  return (
    <label className={`set-check${changed ? ' jx-row--changed' : ''}`}>
      <input
        type="checkbox"
        aria-label={label}
        data-setting={k}
        disabled={!live || locked}
        checked={live && isOn(doc, draft, k)}
        onChange={(e) => draft.set(k, e.target.checked ? 'true' : 'false', doc.values[k])}
      />
      <span>{label}</span>
    </label>
  );
}

/** Una voce che Aurora non ha: si vede spenta, con il motivo. */
function OffRow({ label, reason, kind = 'check', suffix }: { label: string; reason: Key; kind?: 'check' | 'select' | 'text' | 'button' | 'number'; suffix?: string }) {
  const t = useT();
  const why = t(reason);
  let control: ReactNode;
  if (kind === 'check') control = <input type="checkbox" disabled aria-label={label} />;
  else if (kind === 'select') control = <select disabled aria-label={label} />;
  else if (kind === 'number') control = <input type="number" disabled aria-label={label} className="set-number" />;
  else if (kind === 'text') control = <input type="text" disabled aria-label={label} />;
  else control = <button type="button" className="glass" disabled>{label}</button>;
  return (
    <div className="jt-row jt-row--off" title={why}>
      {control}
      {kind === 'button' ? null : <span>{label}</span>}
      {suffix ? <span className="muted">{suffix}</span> : null}
      <span className="muted jt-reason">{why}</span>
    </div>
  );
}

/** Una preferenza dello schermo (del browser, vale subito). */
function PrefCheck({ k, label }: { k: keyof Prefs & ('insertBlank' | 'showTx' | 'showAzimuth' | 'showCountry' | 'showDistance'); label: string }) {
  const prefs = usePrefs((s) => s.prefs);
  return <Check label={label} checked={prefs[k]} onChange={(v) => usePrefs.getState().set({ [k]: v })} />;
}

function useDoc() {
  const settings = useSettings();
  const draft = useDraft();
  return { settings, draft };
}

/* ---------------------------------------------------------------- Sequenza */

const SCENARIOS = [
  { on: 'SeqAnswerCQCount', value: 'SeqAnswerCQCounterValue', label: 'seq2.answerCq' },
  { on: 'SeqAnswerInCallCount', value: 'SeqAnswerInCallCounterValue', label: 'seq2.answerInCall' },
  { on: 'SeqSentRReportCount', value: 'SeqSentRReportCounterValue', label: 'seq2.sentRReport' },
  { on: 'SeqSentRR7373Count', value: 'SeqSentRR7373CounterValue', label: 'seq2.sentRR7373' },
] as const satisfies readonly { on: string; value: string; label: Key }[];

/** Un interruttore del pannello QSO (AutoSeq, 1 QSO, Salta Tx1, Risp B4): stesso stato e stesso comando del pannello, senza copie. */
function ControlBox({ id, label }: { id: ControlId; label: string }) {
  const client = useQueryClient();
  const controls = useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  const on = controls.data?.[id] === true;
  return (
    <label className="set-check">
      <input
        type="checkbox"
        aria-label={label}
        disabled={controls.isError || !controls.data}
        checked={on}
        onChange={async (e) => {
          const next = e.target.checked;
          try {
            await postControl(id, next);
            client.setQueryData(['controls'], (old: object | undefined) => ({ ...old, [id]: next }));
          } finally {
            await client.invalidateQueries({ queryKey: ['controls'] });
          }
        }}
      />
      <span>{label}</span>
    </label>
  );
}

/** La scheda Sequenza come nel motore: i quattro scenari brevi con il numero di volte, «ferma il Tx se...» e «operazione CQ rigorosa». */
export function NativeSequence() {
  const t = useT();
  const { settings, draft } = useDoc();
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  return (
    <section aria-label={t('seq2.title')}>
      <h3 className="set-sub">{t('seq2.title')}</h3>
      <fieldset className="jt-group">
        <legend>{t('seq2.switches')}</legend>
        <div className="jt-row">
          <ControlBox id="autoseq" label={t('seq2.autoseq')} />
          <ControlBox id="single_qso" label={t('seq2.single')} />
          <ControlBox id="skip_tx1" label={t('seq2.skipTx1')} />
          <ControlBox id="answer_b4" label={t('seq2.answerB4')} />
        </div>
        <div className="jt-row">
          <EngineBox k="CallPriorityCQ" label={t('seq2.prioCq')} doc={doc} draft={draft} />
          <EngineBox k="MaxDistance" label={t('seq2.maxDist')} doc={doc} draft={draft} />
        </div>
        <Help>{t('seq2.switchHelp')}</Help>
      </fieldset>
      <fieldset className="jt-group">
        <legend>{t('seq2.group')}</legend>
        {SCENARIOS.map((s) => (
          <div className="jt-row" key={s.on}>
            <EngineBox k={s.on} label={t(s.label)} doc={doc} draft={draft} />
            <input
              type="number"
              className="set-number"
              min={1}
              max={5}
              aria-label={`${t(s.label)} (${t('seq2.times')})`}
              disabled={doc.items[s.value]?.state !== 'live' || !isOn(doc, draft, s.on)}
              value={valueOf(doc, draft, s.value)}
              onChange={(e) => draft.set(s.value, e.target.value, doc.values[s.value])}
            />
            <span className="muted">{t('seq2.times')}</span>
          </div>
        ))}
      </fieldset>
      <div className="jt-row">
        <EngineBox k="SeqHaltTxReplyOther" label={t('seq2.haltReplyOther')} doc={doc} draft={draft} />
      </div>
      <div className="jt-row">
        <EngineBox k="StrictDirectionalCQ" label={t('seq2.strictCq')} doc={doc} draft={draft} />
      </div>
      <Help>{t('seq2.help')}</Help>
      <ApplyBar draft={draft} doc={doc} />
    </section>
  );
}

/* ---------------------------------------------------------------- Generali */

/** La scheda Generali come nel motore: stazione, filtro del log, schermo e funzionamento; le voci che Aurora non ha sono spente, con il motivo. */
export function NativeGeneral() {
  const t = useT();
  const { settings, draft } = useDoc();
  const status = useStation((s) => s.status);
  const prefs = usePrefs((s) => s.prefs);
  const set = usePrefs.getState().set;
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  const field = (key: string, kind: 'bool' | 'text' | 'num', label: Key, extra: { min?: number; max?: number; locked?: boolean } = {}) => (
    <SettingField key={key} row={{ key, kind, min: extra.min, max: extra.max }} label={t(label)} doc={doc} draft={draft} locked={extra.locked} />
  );
  return (
    <>
      <fieldset className="jt-group">
        <legend>{t('gen2.station')}</legend>
        <div className="jx-list">
          {field('MyCall', 'text', 'gen2.call')}
          {field('MyGrid', 'text', 'gen2.grid')}
        </div>
        <p className="muted settings-hint">
          {status?.de_call || '—'} · {status?.de_grid || '—'}
        </p>
        <OffRow label={t('gen2.region')} reason="ens.r.iaru_region" kind="select" suffix={t('gen2.regionAll')} />
        <OffRow label={t('gen2.type2')} reason="ens.r.type2_msgs" kind="select" suffix={t('gen2.type2Full')} />
      </fieldset>

      <fieldset className="jt-group">
        <legend>{t('gen2.logFilter')}</legend>
        <div className="jx-list">
          {field('callsignLogFiltering', 'bool', 'gen2.useCall')}
          {field('gridLogFiltering', 'bool', 'gen2.useGrid', { locked: !isOn(doc, draft, 'callsignLogFiltering') })}
          {field('timeFromLogFiltering', 'text', 'gen2.fromDate')}
        </div>
        <Help>{t('gen2.logHelp')}</Help>
      </fieldset>

      <fieldset className="jt-group">
        <legend>{t('gen2.screen')}</legend>
        <div className="set-checks">
          <PrefCheck k="insertBlank" label={t('gen2.blank')} />
          <Check label={t('gen.miles')} hint={t('tip.gen.miles')} checked={prefs.distanceUnit === 'mi'} onChange={(v) => set({ distanceUnit: v ? 'mi' : 'km' })} />
          <PrefCheck k="showTx" label={t('gen2.showTx')} />
          <PrefCheck k="showCountry" label={t('gen.showCountry')} />
          <PrefCheck k="showDistance" label={t('gen.showDistance')} />
          <PrefCheck k="showAzimuth" label={t('gen2.azimuth')} />
        </div>
        <OffRow label={t('gen2.scroll')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.hidePartial')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.prefix')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.align')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.fontApp')} reason="ens.r.screen_off" kind="button" />
        <OffRow label={t('gen2.fontRx')} reason="ens.r.screen_off" kind="button" />
        <h4 className="set-sub">{t('gen.theme')}</h4>
        <ThemeToggle />
      </fieldset>

      <fieldset className="jt-group">
        <legend>{t('gen2.operation')}</legend>
        <div className="jx-list">
          {field('MonitorOFF', 'bool', 'gen2.monitorOff')}
          {field('TxQSYAllowed', 'bool', 'gen2.qsy')}
          {field('TxWatchdogTimer', 'num', 'gen2.watchdog', { min: 0, max: 99 })}
        </div>
        <OffRow label={t('gen2.decode52')} reason="ens.r.decode52" />
        <OffRow label={t('gen2.lastFreq')} reason="ens.r.monitor_last" />
        <OffRow label={t('gen2.cwId')} reason="ens.r.cw_id" />
        <OffRow label={t('gen2.cwInt')} reason="ens.r.cw_id" kind="number" />
        <OffRow label={t('gen2.tune')} reason="ens.r.tune_timer" kind="number" />
      </fieldset>
      <ApplyBar draft={draft} doc={doc} />

      <h3 className="set-sub">{t('gen2.other')}</h3>
      <div className="set-checks">
        <Check label={t('gen.showFlag')} hint={t('tip.gen.flag')} checked={prefs.showFlag} onChange={(v) => set({ showFlag: v })} />
        <Check label={t('gen.showDt')} hint={t('tip.gen.dt')} checked={prefs.showDt} onChange={(v) => set({ showDt: v })} />
        <Check label={t('gen.escStop')} hint={t('tip.gen.escStop')} checked={prefs.escStop} onChange={(v) => set({ escStop: v })} />
        <Check label={t('gen.clearOnChange')} hint={t('tip.gen.clearOnChange')} checked={prefs.clearOnChange} onChange={(v) => set({ clearOnChange: v })} />
        <Check label={t('gen.dblReply')} hint={t('tip.gen.dblReply')} checked={prefs.doubleClickReply} onChange={(v) => set({ doubleClickReply: v })} />
      </div>
      <label className="set-check" title={t('tip.gen.log4omCount')}>
        <span>{t('gen.log4omCount')}</span>
        <input
          type="number"
          className="set-number"
          min={1}
          max={200}
          value={prefs.log4omCount}
          onChange={(e) => {
            const n = Math.round(Number(e.target.value));
            if (Number.isFinite(n)) set({ log4omCount: Math.min(200, Math.max(1, n)) });
          }}
        />
      </label>
      <button type="button" className="glass" onClick={() => usePrefs.getState().reset()}>
        {t('set.reset')}
      </button>
      <RadioDisplayPicker />
      <h3 className="set-sub">{t('gen.language')}</h3>
      <LanguageSelect />
    </>
  );
}

/* ---------------------------------------------------------------- Notifiche */

/** Le sei evidenziazioni di «nuovo», con le etichette di Aurora che coprono ognuna (in ordine di priorita' come nel motore). */
const NEW_ROWS = [
  { key: 'newCQZ', label: 'not2.cqz', kinds: ['new_zone', 'new_zone_band'], alert: ['alertCqz', 'alertCqzOb'] },
  { key: 'newITUZ', label: 'not2.itu', kinds: ['new_itu', 'new_itu_band'], alert: ['alertItu', 'alertItuOb'] },
  { key: 'newDXCC', label: 'not2.dxcc', kinds: ['new_dxcc', 'new_dxcc_band'], alert: ['alertDxcc', 'alertDxccOb'] },
  { key: 'newGrid', label: 'not2.grid', kinds: ['new_grid', 'new_grid_band'], alert: ['alertGrid', 'alertGridOb'] },
  { key: 'newPx', label: 'not2.px', kinds: ['new_px', 'new_px_band'], alert: ['alertPx', 'alertPxOb'] },
  { key: 'newCall', label: 'not2.call', kinds: ['new_call', 'new_call_band'], alert: null },
] as const satisfies readonly { key: string; label: Key; kinds: readonly ColorKey[]; alert: readonly [AlertKey, AlertKey] | null }[];

/** Una casella degli avvisi di «Play alert sound» (subito, nel browser). */
function AlertBox({ k, label, locked = false }: { k: AlertKey; label: string; locked?: boolean }) {
  const view = useNotifyView((s) => s.view);
  return (
    <label className="set-check">
      <input type="checkbox" aria-label={label} data-alert={k} disabled={locked} checked={view[k]} onChange={(e) => useNotifyView.getState().set({ [k]: e.target.checked })} />
      <span>{label}</span>
    </label>
  );
}

/** Avvisi sonori: accesi/spenti con il volume, il suono a scelta, «Prova» e «non suonare mentre trasmetti». */
function AlertOptions() {
  const t = useT();
  const [alerts, setAlerts] = useState<AlertPrefs>(() => readAlerts(browserStorage()));
  const change = (next: AlertPrefs) => {
    saveAlerts(browserStorage(), next);
    setAlerts(next);
  };
  return (
    <fieldset className="jt-group">
      <legend>{t('not2.playAlert')}</legend>
      <AlertSwitch value={alerts} onChange={change} />
      <div className="jt-row">
        <label className="set-check">
          <span>{t('not2.soundPick')}</span>
          <select aria-label={t('not2.soundPick')} value={alerts.sound} onChange={(e) => change({ ...alerts, sound: e.target.value as SoundName })}>
            {SOUND_NAMES.map((n) => (
              <option key={n} value={n}>
                {t(`not2.snd.${n}` as Key)}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="glass"
          onClick={() => {
            const ctx = webAudioContext();
            if (ctx) playNamed(alerts.sound, alerts.volume, ctx);
          }}
        >
          {t('not2.test')}
        </button>
        <Check label={t('not2.muteTx')} checked={alerts.muteWhileTx} onChange={(v) => change({ ...alerts, muteWhileTx: v })} />
      </div>
      <Help>{t('not2.alertHelp')}</Help>
    </fieldset>
  );
}

/** «Anche un suono»: i suoni dei due tipi di messaggio della riga (nuovo e nuovo su banda/modo), come gli avvisi di Aurora. */
function SoundBox({ kinds, label, disabled }: { kinds: readonly ColorKey[]; label: string; disabled?: boolean }) {
  const sound = useNotify((s) => s.notify.sound);
  return (
    <Check
      label={label}
      checked={!disabled && kinds.every((k) => sound[k])}
      onChange={(v) => {
        if (!disabled) for (const k of kinds) useNotify.getState().setFlag('sound', k, v);
      }}
    />
  );
}

/** Come si vede un tipo di messaggio nell'anteprima: sfondo e testo, scambiati con «Inverti». */
function sampleStyle(bg: string, view: NotifyView): CSSProperties {
  const ink = inkFor(bg);
  return view.invert ? { background: 'var(--panel)', color: bg, borderColor: bg } : { background: bg, color: ink };
}

/** Le righe dell'anteprima, nell'ordine della schermata del motore: prima i tipi di base, poi le novita' e in fondo «gia' lavorato». */
const PREVIEW_ROWS: readonly ColorKey[] = ['tx_msg', 'cq', 'my_call', 'standard_call', 'new_zone', 'new_zone_band', 'new_itu', 'new_itu_band', 'new_dxcc', 'new_dxcc_band', 'new_grid', 'new_grid_band', 'new_px', 'new_px_band', 'new_call', 'new_call_band', 'worked_call'];

export function NativeNotifications() {
  const t = useT();
  const { settings, draft } = useDoc();
  const view = useNotifyView((s) => s.view);
  const setView = useNotifyView.getState().set;
  const [colors, setColors] = useState<TagColors>(() => readColors(browserStorage()));
  const notify = useNotify((s) => s.notify);
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  const commit = (next: TagColors) => {
    setColors(next);
    saveColors(browserStorage(), next);
    applyColors(document.documentElement, next);
  };
  // «gia' lavorato» esiste solo se almeno una evidenziazione di «nuovo» e' accesa (come nel motore)
  const anyNew = NEW_ROWS.some((r) => isOn(doc, draft, r.key));
  const look = workedLook(view);
  return (
    <section aria-label={t('tab.notifications')}>
      <Help>{t('not2.help')}</Help>
      <h3 className="set-sub">{t('not2.intro')}</h3>
      <div className="jt-grid">
        {NEW_ROWS.map((r) => {
          const main = isOn(doc, draft, r.key);
          return (
            <div key={r.key} style={{ display: 'contents' }}>
              <EngineBox k={r.key} label={t(r.label)} doc={doc} draft={draft} />
              <EngineBox k={`${r.key}Band`} label={t('not2.perBand')} doc={doc} draft={draft} locked={!main} />
              <EngineBox k={`${r.key}BandMode`} label={t('not2.perMode')} doc={doc} draft={draft} locked={!main} />
              <SoundBox kinds={r.kinds} label={t('not2.alsoSound')} disabled={!main} />
              {r.alert ? <AlertBox k={r.alert[0]} label={t('not2.playAlert')} /> : <span />}
              {r.alert ? <AlertBox k={r.alert[1]} label={t('not2.perBand')} locked={!view[r.alert[0]]} /> : <span />}
            </div>
          );
        })}

        <Check label={t('not2.worked')} checked={anyNew && view.workedColor} onChange={(v) => setView({ workedColor: v })} />
        <label className="set-check">
          <input type="checkbox" disabled={!anyNew || view.workedUnderlined} checked={anyNew && view.workedStriked} onChange={(e) => setView({ workedStriked: e.target.checked })} />
          <span>{t('not2.strike')}</span>
        </label>
        <label className="set-check">
          <input type="checkbox" disabled={!anyNew || view.workedStriked} checked={anyNew && view.workedUnderlined} onChange={(e) => setView({ workedUnderlined: e.target.checked })} />
          <span>{t('not2.underline')}</span>
        </label>
        <Check label={t('not2.hide')} checked={anyNew && view.workedDontShow} onChange={(v) => setView({ workedDontShow: v })} />

        <Check label={t('not2.invert')} checked={view.invert} onChange={(v) => setView({ invert: v })} />
        <EngineBox k="newPotential" label={t('not2.potential')} doc={doc} draft={draft} />
        <SoundBox kinds={['my_call']} label={t('not2.soundMy')} />
      </div>
      <div className="jt-row">
        <Check label={t('not2.diffBg')} checked={view.differentBackground} onChange={(v) => setView({ differentBackground: v })} />
        <label className="set-check">
          <input type="radio" name="diff-bg" disabled={!view.differentBackground} checked={!view.diffYellow} onChange={() => setView({ diffYellow: false })} />
          <span>{t('not2.cyan')}</span>
        </label>
        <label className="set-check">
          <input type="radio" name="diff-bg" disabled={!view.differentBackground} checked={view.diffYellow} onChange={() => setView({ diffYellow: true })} />
          <span>{t('not2.yellow')}</span>
        </label>
      </div>
      <div className="jt-row">
        <Check label={t('not2.soundFirst')} checked={view.beepFirst} onChange={(v) => setView({ beepFirst: v })} />
        <Check label={t('not2.hlDxCall')} checked={view.highlightDxCall} onChange={(v) => setView({ highlightDxCall: v })} />
        <Check label={t('not2.hlDxGrid')} checked={view.highlightDxGrid} onChange={(v) => setView({ highlightDxGrid: v })} />
      </div>
      <div className="jt-row">
        <AlertBox k="alertMyCall" label={t('not2.alertMy')} />
        <AlertBox k="alertDxCall" label={t('not2.alertDx')} />
      </div>
      <AlertOptions />

      <fieldset className="jt-group">
        <legend>{t('not2.markers')}</legend>
        <div className="jt-row">
          <Check label={t('not2.markOther')} checked={view.otherMarker} onChange={(v) => setView({ otherMarker: v })} />
          <Check label={t('not2.markRed')} checked={view.redMarker} onChange={(v) => setView({ redMarker: v })} />
          <label className="set-check">
            <input type="checkbox" disabled={!view.redMarker} checked={view.redMarker && view.blueMarker} onChange={(e) => setView({ blueMarker: e.target.checked })} />
            <span>{t('not2.markBlue')}</span>
          </label>
          <EngineBox k="73RR73Marker" label={t('not2.mark7373')} doc={doc} draft={draft} />
        </div>
        <OffRow label={t('not2.markHint')} reason="ens.r.no_hint" />
      </fieldset>
      <ApplyBar draft={draft} doc={doc} />

      <h3 className="set-sub">{t('not2.preview')}</h3>
      <Help>{t('not2.previewHelp')}</Help>
      <div className="jt-preview">
        {PREVIEW_ROWS.map((key) => {
          const name = t(kindKey(key));
          const style = sampleStyle(colors[key], view);
          const worked = key === 'worked_call';
          // «gia' lavorato»: senza il suo colore e' un messaggio qualunque (sfondo neutro), con barra o sottolineatura
          const bg = worked && !look.color ? { background: 'var(--panel)', color: 'var(--text)' } : style;
          return (
            <div key={key} style={{ display: 'contents' }}>
              <span>{name}</span>
              <input type="color" aria-label={name} value={colors[key]} onChange={(e) => commit({ ...colors, [key]: e.target.value })} />
              <span
                className={`jt-sample${key.startsWith('new_') ? ' jt-sample--bold' : ''}${worked && look.strike ? ' jt-sample--strike' : ''}${worked && look.underline ? ' jt-sample--underline' : ''}`}
                style={bg}
              >
                K1ABC
              </span>
              <span className="muted jt-reason">{notify.sound[key] ? t('not.sound') : ''}</span>
            </div>
          );
        })}
      </div>
      <button
        type="button"
        className="glass"
        onClick={() => {
          commit(defaultColors());
          useNotify.getState().reset();
          useNotifyView.getState().reset();
        }}
      >
        {t('not.restoreColors')}
      </button>
    </section>
  );
}

