import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type CSSProperties, type ReactNode } from 'react';
import { getControls, postControl, type ControlId } from '../api/client';
import type { EngineSettings } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { applyColors, defaultColors, inkFor, kindKey, readColors, saveColors, type ColorKey, type TagColors } from '../lib/colors';
import { useNotify } from '../lib/notify';
import { useAlertPrefs } from '../lib/alerts';
import { eventOfAlert, useSoundAssign, type SoundEvent } from '../lib/soundAssign';
import { armAudio } from '../lib/soundBank';
import { SoundPicker, SoundsGroup, SoundsHelp } from './SoundControls';
import { useNotifyView, workedLook, type AlertKey, type NotifyView } from '../lib/notifyView';
import { AlertSwitch } from '../components/AlertSwitch';
import { usePrefs, type Prefs } from '../lib/prefs';
import { browserStorage } from '../lib/theme';
import { LanguageSelect } from '../components/LanguageSelect';
import { ThemeToggle } from '../components/ThemeToggle';
import { useStation } from '../state/station';
import type { SettingRow } from '../lib/engineSettingsLayout';
import { ApplyBar, SettingField, useDraft, useSettings, type Draft } from './NativeSettings';
import { RadioDisplayPicker } from './RadioDisplayPicker';
import { Check, Help } from './common';
import { Cell, Cells, Field, Group, Message, Notes, Page, Unavailable } from './layout';

/** Il valore di una voce adesso: la modifica non ancora applicata, o quello salvato. */
function valueOf(doc: EngineSettings, draft: Draft, key: string): string {
  return draft.changes[key] ?? doc.values[key] ?? '';
}

function isOn(doc: EngineSettings, draft: Draft, key: string): boolean {
  return valueOf(doc, draft, key) === 'true';
}

/** La casella nuda di una voce del motore proprio: viva si modifica (con Applica), altrimenti e' spenta. */
function EngineCheck({ k, label, doc, draft, locked = false }: { k: string; label: string; doc: EngineSettings; draft: Draft; locked?: boolean }) {
  const live = doc.items[k]?.state === 'live';
  return (
    <input
      type="checkbox"
      aria-label={label}
      data-setting={k}
      disabled={!live || locked}
      checked={live && isOn(doc, draft, k)}
      onChange={(e) => draft.set(k, e.target.checked ? 'true' : 'false', doc.values[k])}
    />
  );
}

/** Una cella con la casella di una voce del motore proprio. */
function EngineBox({ k, label, doc, draft, locked = false }: { k: string; label: string; doc: EngineSettings; draft: Draft; locked?: boolean }) {
  return (
    <Field label={label} changed={draft.changes[k] !== undefined}>
      <EngineCheck k={k} label={label} doc={doc} draft={draft} locked={locked} />
    </Field>
  );
}

/** Una voce che Aurora non ha: si vede spenta, con il motivo (va dentro un riquadro «Non disponibili»). */
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

/** Le scelte della regione IARU (i nomi di JTDX) e dei messaggi per i nominativi composti di tipo 2. */
const REGION_OPTIONS: SettingRow['options'] = [
  { value: 'ALL', label: 'gen2.regionAll' },
  { value: 'R1', label: 'gen2.region1' },
  { value: 'R2', label: 'gen2.region2' },
  { value: 'R3', label: 'gen2.region3' },
];
const TYPE2_OPTIONS: SettingRow['options'] = [
  { value: 'type_2_msg_1_full', label: 'gen2.type2Tx1' },
  { value: 'type_2_msg_3_full', label: 'gen2.type2Full' },
  { value: 'type_2_msg_5_only', label: 'gen2.type2Tx5' },
];

function useDoc() {
  const settings = useSettings();
  const draft = useDraft();
  return { settings, draft };
}

/** «Non ho potuto leggere le impostazioni» o «sto leggendo»: un messaggio a tutta larghezza. */
function Waiting({ error }: { error: boolean }) {
  const t = useT();
  return <p className={`sx-span-3 ${error ? 'notice' : 'muted'}`}>{error ? t('ens.unavailable') : t('set.loading')}</p>;
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
    <Field label={label}>
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
    </Field>
  );
}

/**
 * La scheda Sequenza come nel motore: i quattro scenari brevi con il numero di volte, «ferma il Tx se...» e «operazione CQ rigorosa».
 * `extra` sono altri riquadri (le regole di Aurora) che finiscono nella stessa griglia.
 */
export function NativeSequence({ extra }: { extra?: ReactNode }) {
  const t = useT();
  const { settings, draft } = useDoc();
  if (settings.isError || !settings.data) {
    return (
      <Page>
        <Waiting error={settings.isError} />
        {extra}
      </Page>
    );
  }
  const doc = settings.data;
  return (
    <Page>
      <Group title={t('seq2.switches')}>
        <Cells min={13}>
          <ControlBox id="autoseq" label={t('seq2.autoseq')} />
          <ControlBox id="single_qso" label={t('seq2.single')} />
          <ControlBox id="answer_all" label={t('seq2.answerAll')} />
          <ControlBox id="skip_tx1" label={t('seq2.skipTx1')} />
          <ControlBox id="answer_b4" label={t('seq2.answerB4')} />
          <EngineBox k="CallPriorityCQ" label={t('seq2.prioCq')} doc={doc} draft={draft} />
          <EngineBox k="MaxDistance" label={t('seq2.maxDist')} doc={doc} draft={draft} />
        </Cells>
      </Group>
      <Group title={t('seq2.group')}>
        <Cells min={20}>
          {SCENARIOS.map((s) => {
            const id = `seq-${s.on}`;
            return (
              <Cell
                key={s.on}
                label={<label htmlFor={id}>{t(s.label)}</label>}
                className={draft.changes[s.on] !== undefined || draft.changes[s.value] !== undefined ? 'sx-field--changed' : ''}
              >
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
                <EngineCheckWithId id={id} k={s.on} label={t(s.label)} doc={doc} draft={draft} />
              </Cell>
            );
          })}
        </Cells>
      </Group>
      <Group title={t('seq2.stop')}>
        <Cells min={13}>
          <EngineBox k="SeqHaltTxReplyOther" label={t('seq2.haltReplyOther')} doc={doc} draft={draft} />
          <EngineBox k="StrictDirectionalCQ" label={t('seq2.strictCq')} doc={doc} draft={draft} />
        </Cells>
      </Group>
      {extra}
      <Notes span={2}>
        <Help>{t('seq2.switchHelp')}</Help>
        <Help>{t('seq2.help')}</Help>
      </Notes>
      <ApplyBar draft={draft} doc={doc} />
    </Page>
  );
}

/** La casella di uno scenario, con un id per l'etichetta accanto. */
function EngineCheckWithId({ id, k, label, doc, draft }: { id: string; k: string; label: string; doc: EngineSettings; draft: Draft }) {
  const live = doc.items[k]?.state === 'live';
  return (
    <input
      id={id}
      type="checkbox"
      aria-label={label}
      data-setting={k}
      disabled={!live}
      checked={live && isOn(doc, draft, k)}
      onChange={(e) => draft.set(k, e.target.checked ? 'true' : 'false', doc.values[k])}
    />
  );
}

/* ---------------------------------------------------------------- Generali */

/** Il resto dello schermo (del browser): bandiera, DT, ESC, svuota al cambio, doppio clic, righe di Log4OM2, ripristino. */
function ScreenExtras() {
  const t = useT();
  const prefs = usePrefs((s) => s.prefs);
  const set = usePrefs.getState().set;
  return (
    <>
      <Check label={t('gen.showFlag')} hint={t('tip.gen.flag')} checked={prefs.showFlag} onChange={(v) => set({ showFlag: v })} />
      <Check label={t('gen.showDt')} hint={t('tip.gen.dt')} checked={prefs.showDt} onChange={(v) => set({ showDt: v })} />
      <Check label={t('gen.escStop')} hint={t('tip.gen.escStop')} checked={prefs.escStop} onChange={(v) => set({ escStop: v })} />
      <Check label={t('gen.clearOnChange')} hint={t('tip.gen.clearOnChange')} checked={prefs.clearOnChange} onChange={(v) => set({ clearOnChange: v })} />
      <Check label={t('gen.dblReply')} hint={t('tip.gen.dblReply')} checked={prefs.doubleClickReply} onChange={(v) => set({ doubleClickReply: v })} />
      <Field label={t('gen.log4omCount')} hint={t('tip.gen.log4omCount')}>
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
      </Field>
      <Cell>
        <button type="button" className="glass" onClick={() => usePrefs.getState().reset()}>
          {t('set.reset')}
        </button>
      </Cell>
    </>
  );
}

/** La scheda Generali come nel motore: stazione, filtro del log, schermo e funzionamento; le voci che Aurora non ha stanno nel riquadro «Non disponibili», con il motivo. */
export function NativeGeneral() {
  const t = useT();
  const { settings, draft } = useDoc();
  const status = useStation((s) => s.status);
  const prefs = usePrefs((s) => s.prefs);
  const set = usePrefs.getState().set;
  if (settings.isError || !settings.data) {
    return (
      <Page>
        <Waiting error={settings.isError} />
      </Page>
    );
  }
  const doc = settings.data;
  const field = (key: string, kind: 'bool' | 'text' | 'num' | 'choice', label: Key, extra: { min?: number; max?: number; locked?: boolean; options?: SettingRow['options'] } = {}) => (
    <SettingField key={key} row={{ key, kind, min: extra.min, max: extra.max, options: extra.options }} label={t(label)} doc={doc} draft={draft} locked={extra.locked} />
  );
  return (
    <Page>
      <Group title={t('gen2.station')}>
        <Cells min={18}>
          {field('MyCall', 'text', 'gen2.call')}
          {field('MyGrid', 'text', 'gen2.grid')}
        </Cells>
        <Message kind="muted">
          {status?.de_call || '—'} · {status?.de_grid || '—'}
        </Message>
      </Group>

      <Group title={t('gen2.logFilter')}>
        <Cells min={13}>
          {field('callsignLogFiltering', 'bool', 'gen2.useCall')}
          {field('gridLogFiltering', 'bool', 'gen2.useGrid', { locked: !isOn(doc, draft, 'callsignLogFiltering') })}
          {field('timeFromLogFiltering', 'text', 'gen2.fromDate')}
        </Cells>
        <Message kind="muted">{t('gen2.logHelp')}</Message>
      </Group>

      <Group title={t('gen2.operation')}>
        <Cells min={13}>
          {field('MonitorOFF', 'bool', 'gen2.monitorOff')}
          {field('TxQSYAllowed', 'bool', 'gen2.qsy')}
          {field('TxWatchdogTimer', 'num', 'gen2.watchdog', { min: 0, max: 99 })}
        </Cells>
      </Group>

      <Group title={t('gen2.iaru')}>
        <Cells min={18}>
          {field('Region', 'choice', 'gen2.region', { options: REGION_OPTIONS })}
          {field('Type2MsgGen', 'choice', 'gen2.type2', { options: TYPE2_OPTIONS })}
        </Cells>
      </Group>

      <Group title={t('gen2.screen')} span={2} rows={2}>
        <Cells min={15}>
          <PrefCheck k="insertBlank" label={t('gen2.blank')} />
          <Check label={t('gen.miles')} hint={t('tip.gen.miles')} checked={prefs.distanceUnit === 'mi'} onChange={(v) => set({ distanceUnit: v ? 'mi' : 'km' })} />
          <PrefCheck k="showTx" label={t('gen2.showTx')} />
          <PrefCheck k="showCountry" label={t('gen.showCountry')} />
          <PrefCheck k="showDistance" label={t('gen.showDistance')} />
          <PrefCheck k="showAzimuth" label={t('gen2.azimuth')} />
          <ScreenExtras />
        </Cells>
      </Group>

      <Group title={t('gen.language')}>
        <LanguageSelect />
      </Group>
      <Group title={t('gen.theme')}>
        <ThemeToggle />
      </Group>

      <Group title={t('rd.title')} span={3}>
        <RadioDisplayPicker />
      </Group>

      <Unavailable count={11}>
        <OffRow label={t('gen2.scroll')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.hidePartial')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.prefix')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.align')} reason="ens.r.screen_off" />
        <OffRow label={t('gen2.fontApp')} reason="ens.r.screen_off" kind="button" />
        <OffRow label={t('gen2.fontRx')} reason="ens.r.screen_off" kind="button" />
        <OffRow label={t('gen2.decode52')} reason="ens.r.decode52" />
        <OffRow label={t('gen2.lastFreq')} reason="ens.r.monitor_last" />
        <OffRow label={t('gen2.cwId')} reason="ens.r.cw_id" />
        <OffRow label={t('gen2.cwInt')} reason="ens.r.cw_id" kind="number" />
        <OffRow label={t('gen2.tune')} reason="ens.r.tune_timer" kind="number" />
      </Unavailable>
      <Notes>
        <Help>{t('gen2.iaruHelp')}</Help>
      </Notes>
      <ApplyBar draft={draft} doc={doc} />
    </Page>
  );
}

/* ---------------------------------------------------------------- Notifiche */

/** Le sei evidenziazioni di «nuovo», con le etichette di Aurora che coprono ognuna (in ordine di priorita' come nel motore). */
const NEW_ROWS = [
  { key: 'newCQZ', label: 'not2.cqz', name: 'not2.n.cqz', kinds: ['new_zone', 'new_zone_band'], alert: ['alertCqz', 'alertCqzOb'] },
  { key: 'newITUZ', label: 'not2.itu', name: 'not2.n.itu', kinds: ['new_itu', 'new_itu_band'], alert: ['alertItu', 'alertItuOb'] },
  { key: 'newDXCC', label: 'not2.dxcc', name: 'not2.n.dxcc', kinds: ['new_dxcc', 'new_dxcc_band'], alert: ['alertDxcc', 'alertDxccOb'] },
  { key: 'newGrid', label: 'not2.grid', name: 'not2.n.grid', kinds: ['new_grid', 'new_grid_band'], alert: ['alertGrid', 'alertGridOb'] },
  { key: 'newPx', label: 'not2.px', name: 'not2.n.px', kinds: ['new_px', 'new_px_band'], alert: ['alertPx', 'alertPxOb'] },
  { key: 'newCall', label: 'not2.call', name: 'not2.n.call', kinds: ['new_call', 'new_call_band'], alert: null },
] as const satisfies readonly { key: string; label: Key; name: Key; kinds: readonly ColorKey[]; alert: readonly [AlertKey, AlertKey] | null }[];

/** La casella nuda di un avviso di «Play alert sound» (subito, nel browser). */
function AlertCheck({ k, label, locked = false }: { k: AlertKey; label: string; locked?: boolean }) {
  const view = useNotifyView((s) => s.view);
  return <input type="checkbox" aria-label={label} data-alert={k} disabled={locked} checked={view[k]} onChange={(e) => {
        useNotifyView.getState().set({ [k]: e.target.checked });
        if (e.target.checked) useSoundAssign.getState().ensure([eventOfAlert(k)]);
      }}
    />;
}

/** Una cella con la casella di un avviso. */
function AlertBox({ k, label, locked = false }: { k: AlertKey; label: string; locked?: boolean }) {
  return (
    <Field label={label}>
      <AlertCheck k={k} label={label} locked={locked} />
    </Field>
  );
}

/** Avvisi sonori: acceso/spento con il volume e «non suonare mentre trasmetti». Il suono di ogni funzione si sceglie nella sua riga e nel riquadro Suoni. */
function AlertOptions() {
  const t = useT();
  const alerts = useAlertPrefs((s) => s.prefs);
  const change = (next: typeof alerts) => {
    if (next.enabled) armAudio();
    useAlertPrefs.getState().set(next);
  };
  return (
    <>
      <Cell label={t('alert.button')} wide>
        <AlertSwitch value={alerts} onChange={change} />
      </Cell>
      <Check label={t('not2.muteTx')} checked={alerts.muteWhileTx} onChange={(v) => change({ ...alerts, muteWhileTx: v })} />
    </>
  );
}

/** «Anche un suono»: la casella nuda dei suoni dei due tipi di messaggio della riga (nuovo e nuovo su banda/modo). */
function SoundCheck({ kinds, label, disabled }: { kinds: readonly ColorKey[]; label: string; disabled?: boolean }) {
  const sound = useNotify((s) => s.notify.sound);
  return (
    <input
      type="checkbox"
      aria-label={label}
      disabled={disabled}
      checked={!disabled && kinds.every((k) => sound[k])}
      onChange={(e) => {
        if (disabled) return;
        for (const k of kinds) useNotify.getState().setFlag('sound', k, e.target.checked);
        if (e.target.checked) useSoundAssign.getState().ensure(kinds.filter((k): k is Exclude<ColorKey, 'auto'> => k !== 'auto'));
      }}
    />
  );
}

/** Una cella con la casella dei suoni. */
function SoundBox({ kinds, label, disabled }: { kinds: readonly ColorKey[]; label: string; disabled?: boolean }) {
  return (
    <Field label={label}>
      <SoundCheck kinds={kinds} label={label} disabled={disabled} />
    </Field>
  );
}

/** Come si vede un tipo di messaggio nell'anteprima: sfondo e testo, scambiati con «Inverti». */
function sampleStyle(bg: string, view: NotifyView): CSSProperties {
  const ink = inkFor(bg);
  return view.invert ? { background: 'var(--panel)', color: bg, borderColor: bg } : { background: bg, color: ink };
}

/** Le righe dell'anteprima, nell'ordine della schermata del motore: prima i tipi di base, poi le novita' e in fondo «gia' lavorato». */
const PREVIEW_ROWS: readonly ColorKey[] = ['tx_msg', 'cq', 'my_call', 'standard_call', 'new_zone', 'new_zone_band', 'new_itu', 'new_itu_band', 'new_dxcc', 'new_dxcc_band', 'new_grid', 'new_grid_band', 'new_px', 'new_px_band', 'new_call', 'new_call_band', 'worked_call'];

/** Il nominativo mostrato nelle anteprime: quello della stazione, finche' non e' noto un nominativo di prova. */
const SAMPLE_FALLBACK = 'K1ABC';

export function NativeNotifications() {
  const t = useT();
  const { settings, draft } = useDoc();
  const view = useNotifyView((s) => s.view);
  const setView = useNotifyView.getState().set;
  const [colors, setColors] = useState<TagColors>(() => readColors(browserStorage()));
  const myCall = useStation((s) => s.status?.de_call);
  if (settings.isError || !settings.data) {
    return (
      <Page>
        <Waiting error={settings.isError} />
      </Page>
    );
  }
  const doc = settings.data;
  const commit = (next: TagColors) => {
    setColors(next);
    saveColors(browserStorage(), next);
    applyColors(document.documentElement, next);
  };
  // «gia' lavorato» esiste solo se almeno una evidenziazione di «nuovo» e' accesa (come nel motore)
  const anyNew = NEW_ROWS.some((r) => isOn(doc, draft, r.key));
  const look = workedLook(view);
  const sample = myCall && myCall.trim() !== '' ? myCall : SAMPLE_FALLBACK;
  const na = <td className="sx-na" aria-hidden="true">—</td>;
  return (
    <Page>
      <Group title={t('not2.grpTable')} span={2}>
        <div className="sx-tablewrap">
          <table className="sx-table jt-grid">
            <colgroup>
              <col style={{ width: '19%' }} />
              <col />
              <col />
              <col />
              <col />
              <col />
              <col />
            </colgroup>
            <thead>
              <tr>
                <th scope="col">{t('not2.colKind')}</th>
                <th scope="col">{t('not2.colHighlight')}</th>
                <th scope="col"><span className="sx-first-letter">{t('not2.perBand')}</span></th>
                <th scope="col"><span className="sx-first-letter">{t('not2.perMode')}</span></th>
                <th scope="col"><span className="sx-first-letter">{t('not2.alsoSound')}</span></th>
                <th scope="col">{t('not2.colAlert')}</th>
                <th scope="col"><span className="sx-first-letter">{t('not2.perBand')}</span></th>
              </tr>
            </thead>
            <tbody>
              {NEW_ROWS.map((r) => {
                const main = isOn(doc, draft, r.key);
                return (
                  <tr key={r.key}>
                    <th scope="row">{t(r.name)}</th>
                    <td title={t(r.label)}>
                      <span className="sx-cellbox">
                        <EngineCheck k={r.key} label={t(r.label)} doc={doc} draft={draft} />
                      </span>
                    </td>
                    <td>
                      <span className="sx-cellbox">
                        <EngineCheck k={`${r.key}Band`} label={t('not2.perBand')} doc={doc} draft={draft} locked={!main} />
                      </span>
                    </td>
                    <td>
                      <span className="sx-cellbox">
                        <EngineCheck k={`${r.key}BandMode`} label={t('not2.perMode')} doc={doc} draft={draft} locked={!main} />
                      </span>
                    </td>
                    <td>
                      <span className="sx-cellbox">
                        <SoundCheck kinds={r.kinds} label={t('not2.alsoSound')} disabled={!main} />
                      </span>
                    </td>
                    {r.alert ? (
                      <>
                        <td>
                          <span className="sx-cellbox">
                            <AlertCheck k={r.alert[0]} label={t('not2.playAlert')} />
                          </span>
                        </td>
                        <td>
                          <span className="sx-cellbox">
                            <AlertCheck k={r.alert[1]} label={t('not2.perBand')} locked={!view[r.alert[0]]} />
                          </span>
                        </td>
                      </>
                    ) : (
                      <>
                        {na}
                        {na}
                      </>
                    )}
                  </tr>
                );
              })}
              <tr>
                <th scope="row">{t('not2.n.worked')}</th>
                <td title={t('not2.worked')}>
                  <span className="sx-cellbox">
                    <input type="checkbox" aria-label={t('not2.worked')} checked={anyNew && view.workedColor} onChange={(e) => setView({ workedColor: e.target.checked })} />
                  </span>
                </td>
                {na}
                {na}
                {na}
                {na}
                {na}
              </tr>
            </tbody>
          </table>
        </div>
      </Group>

      <Group title={t('not2.grpWorked')}>
        <Cells min={16}>
          <Field label={t('not2.strike')}>
            <input type="checkbox" aria-label={t('not2.strike')} disabled={!anyNew || view.workedUnderlined} checked={anyNew && view.workedStriked} onChange={(e) => setView({ workedStriked: e.target.checked })} />
          </Field>
          <Field label={t('not2.underline')}>
            <input type="checkbox" aria-label={t('not2.underline')} disabled={!anyNew || view.workedStriked} checked={anyNew && view.workedUnderlined} onChange={(e) => setView({ workedUnderlined: e.target.checked })} />
          </Field>
          <Check label={t('not2.hide')} checked={anyNew && view.workedDontShow} onChange={(v) => setView({ workedDontShow: v })} />
          <Check label={t('not2.invert')} checked={view.invert} onChange={(v) => setView({ invert: v })} />
          <EngineBox k="newPotential" label={t('not2.potential')} doc={doc} draft={draft} />
        </Cells>
      </Group>

      <Group title={t('not2.grpColors')} span={2} rows={2}>
        <Cells min={22} className="sx-cells--tight jt-colors">
          {PREVIEW_ROWS.map((key) => {
            const name = t(kindKey(key));
            const style = sampleStyle(colors[key], view);
            const worked = key === 'worked_call';
            // «gia' lavorato»: senza il suo colore e' un messaggio qualunque (sfondo neutro), con barra o sottolineatura
            const bg = worked && !look.color ? { background: 'var(--panel)', color: 'var(--text)' } : style;
            return (
              <Cell key={key} label={name}>
                <SoundPicker event={key as SoundEvent} label={name} />
                <span
                  className={`jt-sample${key.startsWith('new_') ? ' jt-sample--bold' : ''}${worked && look.strike ? ' jt-sample--strike' : ''}${worked && look.underline ? ' jt-sample--underline' : ''}`}
                  style={bg}
                >
                  {sample}
                </span>
                <input type="color" aria-label={name} value={colors[key]} onChange={(e) => commit({ ...colors, [key]: e.target.value })} />
              </Cell>
            );
          })}
          <Cell wide>
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
          </Cell>
        </Cells>
      </Group>

      <Group title={t('not2.grpAlerts')}>
        <Cells min={14}>
          <AlertOptions />
          <AlertBox k="alertMyCall" label={t('not2.alertMy')} />
          <AlertBox k="alertDxCall" label={t('not2.alertDx')} />
          <SoundBox kinds={['my_call']} label={t('not2.soundMy')} />
          <Check
            label={t('not2.soundFirst')}
            checked={view.beepFirst}
            onChange={(v) => {
              setView({ beepFirst: v });
              if (v) useSoundAssign.getState().ensure(['first_message']);
            }}
          />
        </Cells>
      </Group>

      <Group title={t('not2.grpDisplay')}>
        <Cells min={14}>
          <Check label={t('not2.diffBg')} checked={view.differentBackground} onChange={(v) => setView({ differentBackground: v })} />
          <Cell>
            <label className="set-check">
              <input type="radio" name="diff-bg" disabled={!view.differentBackground} checked={!view.diffYellow} onChange={() => setView({ diffYellow: false })} />
              <span>{t('not2.cyan')}</span>
            </label>
            <label className="set-check">
              <input type="radio" name="diff-bg" disabled={!view.differentBackground} checked={view.diffYellow} onChange={() => setView({ diffYellow: true })} />
              <span>{t('not2.yellow')}</span>
            </label>
          </Cell>
          <Check label={t('not2.hlDxCall')} checked={view.highlightDxCall} onChange={(v) => setView({ highlightDxCall: v })} />
          <Check label={t('not2.hlDxGrid')} checked={view.highlightDxGrid} onChange={(v) => setView({ highlightDxGrid: v })} />
        </Cells>
      </Group>

      <Group title={t('not2.markers')}>
        <Cells min={14}>
          <Check label={t('not2.markOther')} checked={view.otherMarker} onChange={(v) => setView({ otherMarker: v })} />
          <Check label={t('not2.markRed')} checked={view.redMarker} onChange={(v) => setView({ redMarker: v })} />
          <Field label={t('not2.markBlue')}>
            <input type="checkbox" aria-label={t('not2.markBlue')} disabled={!view.redMarker} checked={view.redMarker && view.blueMarker} onChange={(e) => setView({ blueMarker: e.target.checked })} />
          </Field>
          <EngineBox k="73RR73Marker" label={t('not2.mark7373')} doc={doc} draft={draft} />
        </Cells>
      </Group>

      <Unavailable count={1}>
        <OffRow label={t('not2.markHint')} reason="ens.r.no_hint" />
      </Unavailable>

      <Notes>
        <Help>{t('not2.help')}</Help>
        <Help>{t('not2.intro')}</Help>
        <Help>{t('not2.previewHelp')}</Help>
        <Help>{t('not2.alertHelp')}</Help>
        <SoundsHelp />
      </Notes>
      <SoundsGroup />
      <ApplyBar draft={draft} doc={doc} />
    </Page>
  );
}
