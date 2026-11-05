import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState, type ReactNode } from 'react';
import { ApiError, getEngineSettings, getStationInfo, postUploadTest, putEngineSettings } from '../api/client';
import type { EngineSettings } from '../api/schemas';
import { Help } from './layout';
import { engineHint } from './engineHints';
import { useT, type Key, type T } from '../i18n';
import { ENGINE_GROUPS, type EngineGroup } from '../lib/engineGroups';
import { LINK_GROUPS, type SettingGroup, type SettingRow, type UploadService } from '../lib/engineSettingsLayout';
import { Help as Note } from './common';
import { Cells, Group, Notes, Page } from './layout';
import { useAutosave } from './useAutosave';
import { WavelogStationSelect } from './WavelogStationSelect';
import { IntegrationsPanel } from './IntegrationsPanel';
import { coreText } from '../lib/coreText';

/** Vero quando Aurora gira con il motore proprio: le impostazioni sono sue, non di un altro programma. */
export function useIsNative(): boolean | undefined {
  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  if (info.isError) return false;
  return info.data ? info.data.engine === 'native' : undefined;
}

export function useSettings() {
  return useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false });
}

/** Un rifiuto del salvataggio: la voce coinvolta (se il server la dice) e il testo da mostrare accanto. */
export interface SaveError {
  key: string | null;
  text: string;
}

/** Il messaggio vero del server («UDP2Server: serve un indirizzo…») con il nome della voce come appare nell'interfaccia. */
export function saveErrorOf(error: unknown, t: T): SaveError {
  if (error instanceof ApiError) {
    if (error.code === 'local_only') return { key: null, text: t('ens.err.local') };
    if (error.code === 'bad_setting') {
      const m = error.detail ? /^([A-Za-z0-9_]+)\s*:\s*(.+)$/s.exec(error.detail) : null;
      if (m) {
        const key = m[1]!;
        const name = t(`ens.k.${key}` as Key);
        return { key, text: `${name.startsWith('ens.k.') ? key : name}: ${m[2]}` };
      }
      return { key: null, text: error.detail ?? t('ens.err.bad') };
    }
  }
  return { key: null, text: t('ens.err.failed') };
}

export interface Draft {
  changes: Record<string, string>;
  set(key: string, value: string, current: string | undefined): void;
  /** L'ultimo rifiuto del salvataggio automatico (si toglie al salvataggio riuscito o quando la voce cambia). */
  error?: SaveError | null;
  setError?(e: SaveError | null): void;
}

export function useDraft(): Draft & { clear(): void } {
  const [changes, setChanges] = useState<Record<string, string>>({});
  const [error, setError] = useState<SaveError | null>(null);
  return {
    changes,
    error,
    setError,
    set(key, value, current) {
      setError((e) => (e?.key === key ? null : e));
      setChanges((previous) => {
        const next = { ...previous };
        if (value === (current ?? '')) delete next[key];
        else next[key] = value;
        return next;
      });
    },
    clear: () => setChanges({}),
  };
}

/** Evento: un campo segreto e' stato confermato (uscendo dal campo o con Invio); `detail` e' la voce. */
export const SECRET_COMMITTED = 'aurora-secret-committed';
/** Valore «attuale» di un segreto per la bozza: non coincide mai con un testo scritto, cosi' anche «Togli» (testo vuoto) conta come modifica. */
const NO_SECRET = '\u0000';

/**
 * Un campo segreto (password, chiave). Il server non restituisce mai il segreto, solo se e' salvato: salvato si mostra «Salvata ••••»
 * con «Cambia» e «Togli»; mentre si scrive non si salva niente, si conferma uscendo dal campo o con Invio.
 */
export function SecretInput({ id, label, saved, pending, disabled, onCommit }: { id: string; label: string; saved: boolean; pending: string | undefined; disabled: boolean; onCommit(v: string): void }) {
  const t = useT();
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState('');
  const commit = () => {
    setEditing(false);
    const v = text.trim();
    setText('');
    if (v) onCommit(v);
  };
  if (pending !== undefined && !editing) {
    return (
      <span className="ens-secret" id={id} role="status" aria-label={label}>
        {pending === '' ? t('ens.pw.removing') : t('ens.pw.saving')}
      </span>
    );
  }
  if (saved && !editing) {
    return (
      <span className="ens-secret" id={id} aria-label={label}>
        <span className="ens-secret-state" data-testid={`${id}-saved`}>{t('ens.pw.savedShort')} ••••</span>{' '}
        <button type="button" className="glass" disabled={disabled} title={t('ens.pw.changeTip')} onClick={(e) => { e.preventDefault(); setEditing(true); }}>
          {t('ens.pw.change')}
        </button>{' '}
        <button type="button" className="glass" disabled={disabled} title={t('ens.pw.removeTip')} onClick={(e) => { e.preventDefault(); onCommit(''); }}>
          {t('ens.pw.remove')}
        </button>
      </span>
    );
  }
  return (
    <input
      id={id}
      aria-label={label}
      type="password"
      className="jx-text"
      disabled={disabled}
      maxLength={200}
      value={text}
      autoFocus={editing}
      placeholder={t('ens.pw.type')}
      autoComplete="off"
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') { e.preventDefault(); commit(); }
        if (e.key === 'Escape') { setText(''); setEditing(false); }
      }}
    />
  );
}

/** Una voce: viva (si modifica) o non disponibile (spenta, con il motivo). */
export function SettingField({ row, label, doc, draft, locked = false, className = '', shown }: { row: SettingRow; label: string; doc: EngineSettings; draft: Draft; locked?: boolean; className?: string; shown?: string }) {
  const t = useT();
  const item = doc.items[row.key];
  const live = item?.state === 'live';
  const current = doc.values[row.key];
  const changed = draft.changes[row.key];
  const value = changed ?? current ?? '';
  const id = `ens-${row.key}`;
  const change = (next: string) => {
    draft.set(row.key, next, current);
    // in JTDX le due caselle di ALL.TXT si escludono: accenderne una spegne l'altra
    const other = row.key === 'WriteDecodedALLTXT' ? 'WriteDecodedDebugALLTXT' : row.key === 'WriteDecodedDebugALLTXT' ? 'WriteDecodedALLTXT' : null;
    if (other && next === 'true') draft.set(other, 'false', doc.values[other]);
  };
  let control;
  if (row.kind === 'bool') {
    control = <input id={id} aria-label={label} type="checkbox" disabled={!live || locked} checked={live && value === 'true'} onChange={(e) => change(e.target.checked ? 'true' : 'false')} />;
  } else if (row.kind === 'num') {
    control = <input id={id} aria-label={label} type="number" min={row.min ?? 1} max={row.max ?? 65535} step={row.step} disabled={!live || locked} className="set-number" value={live ? value : ''} onChange={(e) => change(e.target.value)} />;
  } else if (row.kind === 'reply') {
    control = (
      <select id={id} aria-label={label} disabled={!live || locked} value={live ? value : ''} onChange={(e) => change(e.target.value)}>
        {!live ? <option value="" /> : null}
        {['1', '2', '3'].map((o) => (
          <option key={o} value={o}>
            {t(`jx.reply.${o}` as Key)}
          </option>
        ))}
      </select>
    );
  } else if (row.kind === 'choice') {
    control = (
      <select id={id} aria-label={label} disabled={!live || locked} value={live ? value : ''} onChange={(e) => change(e.target.value)}>
        {!live ? <option value="" /> : null}
        {(row.options ?? []).map((o) => (
          <option key={o.value} value={o.value}>
            {t(o.label)}
          </option>
        ))}
      </select>
    );
  } else if (row.key === 'WavelogStation') {
    const url = draft.changes.WavelogUrl ?? doc.values.WavelogUrl ?? '';
    control = <WavelogStationSelect id={id} label={label} url={url} value={live ? value : ''} disabled={!live || locked} onChange={change} />;
  } else if (row.kind === 'secret') {
    control = <SecretInput id={id} label={label} saved={Boolean(doc.secrets?.[row.key])} pending={changed} disabled={!live || locked} onCommit={(v) => { draft.set(row.key, v, NO_SECRET); window.dispatchEvent(new CustomEvent(SECRET_COMMITTED, { detail: row.key })); }} />;
  } else {
    control = <input id={id} aria-label={label} type="text" className="jx-text" disabled={!live || locked} maxLength={200} value={live ? value : ''} autoComplete="off" onChange={(e) => change(e.target.value)} />;
  }
  const note = live ? (item?.apply === 'restart' ? t('ens.restart') : '') : t(`ens.r.${item?.reason ?? 'own_decoder'}` as Key);
  const liveNote = live && item?.note ? t(`ens.n.${item.note}` as Key) : '';
  return (
    <label className={`jx-row${className ? ` ${className}` : ''}${changed !== undefined ? ' jx-row--changed' : ''}${live ? '' : ' jx-row--off'}`} htmlFor={id} title={live ? undefined : note}>
      <span className="jx-name">
        {shown ?? label}
        <Help tip={engineHint(row.key, t)} />
        {note ? <small className="ens-note">{live ? ` (${note})` : ` — ${t('ens.off')}`}</small> : null}
      </span>
      {control}
      {!live ? <span className="ens-reason muted">{note}</span> : null}
      {liveNote ? <span className="ens-reason muted">{liveNote}</span> : null}
      {draft.error?.key === row.key ? (
        <span className="ens-reason live-note ens-error" role="alert">
          {draft.error.text}
        </span>
      ) : null}
    </label>
  );
}

/** Barra in fondo: quante modifiche aspettano, con «Applica» (subito, niente riavvii) e «Annulla». */
export function ApplyBar({ draft, doc }: { draft: Draft & { clear(): void }; doc: EngineSettings }) {
  const t = useT();
  const client = useQueryClient();
  const [note, setNote] = useState('');
  const changes = draft.changes;
  const apply = useMutation({
    mutationFn: () => putEngineSettings(changes),
    onSuccess: (next) => {
      const slow = Object.keys(changes).some((k) => doc.items[k]?.apply === 'restart');
      client.setQueryData(['engine-settings'], next);
      draft.clear();
      draft.setError?.(null);
      setNote(slow ? t('ens.appliedRestart') : t('ens.applied'));
    },
    onError: (e) => {
      const err = saveErrorOf(e, t);
      // con la voce nota l'errore va accanto al suo riquadro; senza, resta qui nella barra
      if (err.key && doc.items[err.key]?.state === 'live' && draft.setError) {
        draft.setError(err);
        setNote('');
      } else setNote(err.text);
    },
  });
  const count = Object.keys(changes).length;
  // le modifiche si applicano da sole, dopo una breve pausa e uscendo dalla scheda
  useAutosave(count > 0 ? changes : null, count > 0, () => apply.mutate(), (v) => void Promise.resolve().then(() => putEngineSettings(v)).catch(() => undefined), 700);
  if (count === 0 && !note) return null;
  return (
    <div className="jx-bar" role="status">
      {count > 0 ? <strong>{t('ens.pending', { n: count })}</strong> : null}
      {count > 0 ? (
        <>
          <button title={t('tip.ui.apply')} type="button" className="glass glass--on" disabled={apply.isPending} onClick={() => apply.mutate()}>
            {t('ens.apply')}
          </button>
          <button title={t('tip.ui.discard')} type="button" className="glass" disabled={apply.isPending} onClick={() => { draft.clear(); setNote(''); }}>
            {t('ens.discard')}
          </button>
        </>
      ) : null}
      {note ? <span className="live-note">{note}</span> : null}
    </div>
  );
}

const STATUS_OF: Record<string, string> = {
  PSKReporter: 'psk',
  EQSLSend: 'eqsl',
  EnableTCPConnection: 'tcp',
  AllowSpotsDXSummit: 'dxsummit',
  WavelogSend: 'wavelog',
  ClublogSend: 'clublog',
  QRZLogSend: 'qrz',
  LoTWSend: 'lotw',
  HrdlogSend: 'hrdlog',
};

/** L'esito dell'ultimo invio di un servizio (ok o errore, e quando). */
function SendStatus({ doc, kind, compact = false }: { doc: EngineSettings; kind: string; compact?: boolean }) {
  const t = useT();
  const st = doc.status?.[kind];
  // riga dei servizi: niente «Ultimo invio: mai»; dopo un invio solo l'esito in breve, il dettaglio nel suggerimento
  if (compact) {
    if (!st || st.ok === null) return null;
    const detail = `${st.ok ? t('ens.st.ok') : t('ens.st.err')}: ${coreText(st.message, t)} (${new Date(st.at_ms).toLocaleTimeString()})`;
    return (
      <small className={`ens-reason ${st.ok ? 'muted' : 'live-note'}`} role="status" title={detail} data-testid={`status-${kind}`}>
        {t('ens.st.title')}: {st.ok ? t('ens.st.ok') : t('ens.st.err')} {new Date(st.at_ms).toLocaleTimeString()}
        {st.pending ? ` — ${t('ens.st.pending', { n: st.pending })}` : ''}
      </small>
    );
  }
  const text = !st || st.ok === null ? t('ens.st.never') : `${st.ok ? t('ens.st.ok') : t('ens.st.err')}: ${coreText(st.message, t)} (${new Date(st.at_ms).toLocaleTimeString()})`;
  const pending = st?.pending ? ` — ${t('ens.st.pending', { n: st.pending })}` : '';
  return (
    <p className={`ens-reason ${st?.ok === false ? 'live-note' : 'muted'}`} role="status" data-testid={`status-${kind}`}>
      {t('ens.st.title')}: {text}
      {pending}
    </p>
  );
}

/** Il pulsante «Prova» di un servizio dell'invio diretto: controlla le impostazioni gia' salvate, senza registrare QSO. */
/** L'esito di una prova. */
export interface TestResult {
  ok: boolean;
  text: string;
}

/** Il colore della spia di un servizio: spento o non configurato, da provare, ultima prova o invio riuscito, fallito. */
export type Led = 'off' | 'untested' | 'ok' | 'err';

export function ledOf(enabled: boolean, result: TestResult | null, status: { ok: boolean | null; message: string } | undefined): { led: Led; why: string } {
  if (!enabled) return { led: 'off', why: '' };
  if (result) return { led: result.ok ? 'ok' : 'err', why: result.text };
  if (status && status.ok !== null) return { led: status.ok ? 'ok' : 'err', why: status.message };
  return { led: 'untested', why: '' };
}

/** La spia nel titolo del riquadro. */
export function StatusLed({ led, why }: { led: Led; why: string }) {
  const t = useT();
  const label = t(`ens.led.${led}` as Key);
  return <span className={`ens-led ens-led--${led}`} role="img" aria-label={label} title={why ? `${label}: ${why}` : label} data-testid="ens-led" data-led={led} />;
}

function UploadTest({ service, waiting, keys, result, setResult, autoStart, tip, compact = false }: { service: UploadService; waiting: boolean; keys: readonly string[]; result: TestResult | null; setResult(r: TestResult): void; autoStart: boolean; tip?: Key; compact?: boolean }) {
  const t = useT();
  const client = useQueryClient();
  // dopo aver confermato una password o una chiave del riquadro, appena e' salvata parte da sola la prova
  // all'apertura della pagina, se il servizio e' acceso, si prova una volta da solo
  const [auto, setAuto] = useState(autoStart);
  const keyList = keys.join(',');
  useEffect(() => {
    const on = (e: Event) => {
      if (keyList.split(',').includes(String((e as CustomEvent).detail))) setAuto(true);
    };
    window.addEventListener(SECRET_COMMITTED, on);
    return () => window.removeEventListener(SECRET_COMMITTED, on);
  }, [keyList]);
  const test = useMutation({
    mutationFn: () => postUploadTest(service),
    onSuccess: (text) => setResult({ ok: true, text }),
    onError: (e) => setResult({ ok: false, text: e instanceof ApiError ? (e.detail ?? (e.code === 'local_only' ? t('ens.err.local') : e.code)) : t('ens.test.err') }),
    // la prova puo' far ripartire un servizio fermo: si rilegge lo stato
    onSettled: () => void client.invalidateQueries({ queryKey: ['engine-settings'] }),
  });
  useEffect(() => {
    if (auto && !waiting && !test.isPending) {
      setAuto(false);
      test.mutate();
    }
  }, [auto, waiting, test]);
  return (
    <div className="ens-test">
      <button type="button" className="glass" disabled={waiting || test.isPending} title={waiting ? t('ens.test.saveFirst') : tip ? t(tip) : undefined} onClick={() => test.mutate()} data-testid={`test-${service}`}>
        {test.isPending ? t('ens.test.running') : t('ens.test')}
      </button>
      {waiting ? <span className="muted"> {t('ens.test.saveFirst')}</span> : null}
      {result && !waiting ? (
        <span className={result.ok ? 'ens-test-ok' : 'live-note'} role="status" title={result.text} data-testid={`test-result-${service}`}>
          {' '}
          {compact ? (result.ok ? t('ens.test.ok') : t('ens.test.err')) : `${result.ok ? t('ens.test.ok') : t('ens.test.err')}: ${result.text}`}
        </span>
      ) : null}
    </div>
  );
}

/** «Prova» e spia per un riquadro fuori dalle voci del motore (dati online, orologio): `led` va nel titolo, `test` sotto le voci. */
export function useServiceTest(service: UploadService, enabled: boolean, tip: Key): { led: ReactNode; test: ReactNode } {
  const [result, setResult] = useState<TestResult | null>(null);
  const { led, why } = ledOf(enabled, result, undefined);
  return {
    led: <StatusLed led={led} why={why} />,
    test: <UploadTest service={service} waiting={false} keys={[]} result={result} setResult={setResult} autoStart={enabled} tip={tip} />,
  };
}

/** I riquadri di voci del motore (una pagina o parte di una): interruttore e campi, «?» con la spiegazione, «Prova» e stato dell'ultimo invio, errore accanto. */
/** La larghezza di un campo nella riga di un servizio: nominativi stretti, indirizzi ed email larghi, chiavi medie. */
function fieldWidth(row: SettingRow): string {
  if (row.kind === 'bool') return 'svc-w-bool';
  if (/Call$|Station$/.test(row.key)) return 'svc-w-s';
  if (/Email|Url|Tqsl|Location/.test(row.key)) return 'svc-w-l';
  return 'svc-w-m';
}

export function SettingGroups({ groups, doc, draft, span = 1, layout = 'boxes' }: { groups: readonly SettingGroup[]; doc: EngineSettings; draft: Draft; span?: 1 | 2 | 3; layout?: 'boxes' | 'rows' }) {
  const t = useT();
  const [results, setResults] = useState<Record<string, TestResult>>({});
  const body = (
    <>
      {groups.map((group) => {
        const rows = group.rows.filter((r) => doc.items[r.key]?.state === 'live');
        if (rows.length === 0) return null;
        const kinds = [...new Set(rows.map((r) => STATUS_OF[r.key]).filter((k): k is string => Boolean(k)))];
        const waiting = group.rows.some((r) => draft.changes[r.key] !== undefined);
        const help = group.upload?.help ?? group.help;
        // acceso = il primo interruttore del riquadro (salvato); i riquadri senza interruttore o senza servizio non hanno la spia
        const toggle = rows.find((r) => r.kind === 'bool');
        const enabled = group.upload?.always === true || (toggle ? doc.values[toggle.key] === 'true' : false);
        const service = group.upload?.service ?? kinds[0];
        const result = service ? (results[service] ?? null) : null;
        const led = service ? ledOf(enabled, result, kinds[0] ? doc.status?.[kinds[0]] : undefined) : null;
        const test = group.upload ? <UploadTest service={group.upload.service} waiting={waiting} keys={group.rows.filter((r) => r.kind === 'secret').map((r) => r.key)} result={result} setResult={(v) => setResults((p) => ({ ...p, [group.upload!.service]: v }))} autoStart={enabled && group.upload.auto !== false} tip={group.upload.tip} compact={layout === 'rows'} /> : null;
        if (layout === 'rows') {
          // un servizio per riga: spia, nome con «?», campi in linea, «Prova» ed esito; su schermi stretti va a capo
          return (
            <div key={group.title} className="svc-row" role="group" aria-label={t(group.title)} data-testid={`svc-${service ?? group.title}`}>
              <div className="svc-head">
                {led ? <StatusLed led={led.led} why={coreText(led.why, t)} /> : null}
                <strong>{t(group.title)}</strong>
                <Help tip={[help ? t(help) : '', group.note ? t(group.note) : ''].filter(Boolean).join(' ') || undefined} />
              </div>
              <div className="svc-fields">
                {rows.map((row) => (
                  <SettingField key={row.key} row={row} label={t(`ens.k.${row.key}` as Key)} shown={row === toggle ? t('ens.svc.on') : undefined} doc={doc} draft={draft} className={`svc-field ${fieldWidth(row)}`} />
                ))}
              </div>
              <div className="svc-side">
                {test}
                {kinds.map((k) => (
                  <SendStatus key={k} doc={doc} kind={k} compact />
                ))}
              </div>
            </div>
          );
        }
        return (
          <Group key={group.title} title={<>{led ? <StatusLed led={led.led} why={coreText(led.why, t)} /> : null}{t(group.title)}<Help tip={help ? t(help) : undefined} /></>} span={span}>
            <Cells min={15}>
              {rows.map((row) => (
                <SettingField key={row.key} row={row} label={t(`ens.k.${row.key}` as Key)} doc={doc} draft={draft} />
              ))}
            </Cells>
            {test}
            {kinds.map((k) => (
              <SendStatus key={k} doc={doc} kind={k} />
            ))}
          </Group>
        );
      })}
    </>
  );
  return layout === 'rows' ? <div className="svc-table sx-span-3">{body}</div> : body;
}

/** Riquadri del motore dentro un'altra pagina (spot in «Servizi online», registrazione del QSO in «Sequenza»), con la loro barra del salvataggio. */
export function EngineGroups({ groups, layout }: { groups: readonly SettingGroup[]; layout?: 'boxes' | 'rows' }) {
  const settings = useSettings();
  const draft = useDraft();
  const native = useIsNative();
  if (!native || !settings.data) return null;
  return (
    <>
      <SettingGroups groups={groups} doc={settings.data} draft={draft} layout={layout} />
      <div className="sx-span-3">
        <ApplyBar draft={draft} doc={settings.data} />
      </div>
    </>
  );
}

/** Una pagina fatta solo di riquadri del motore, con la barra del salvataggio. */
export function NativeGroupsPage({ groups, label, children, layout }: { groups: readonly SettingGroup[]; label: string; children?: (doc: EngineSettings, draft: Draft) => ReactNode; layout?: 'boxes' | 'rows' }) {
  const t = useT();
  const settings = useSettings();
  const draft = useDraft();
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  return (
    <section aria-label={label}>
      <Page>
        <SettingGroups groups={groups} doc={doc} draft={draft} layout={layout} />
        {children?.(doc, draft)}
        <Notes span={2}>
          <Note>{t('ens.note', { file: doc.file })}</Note>
        </Notes>
      </Page>
      <ApplyBar draft={draft} doc={doc} />
    </section>
  );
}

/** Scheda «Programmi collegati»: server UDP primario, ADIF via UDP, TCP (N1MM) e il riquadro Integrazioni. */
export function NativeReports() {
  const t = useT();
  return (
    <NativeGroupsPage groups={LINK_GROUPS} label={t('tab.reports')}>
      {(doc, draft) => <IntegrationsPanel doc={doc} draft={draft} />}
    </NativeGroupsPage>
  );
}

/** Le voci di un gruppo di schede (Generali, Radio, Audio, Sequenza, Notifiche, Avanzate) con il motore proprio: quelle vive si modificano, le altre sono elencate con il motivo. */
export function NativeValues({ group, title }: { group: EngineGroup; title?: string }) {
  const t = useT();
  const settings = useSettings();
  const draft = useDraft();
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  const items = ENGINE_GROUPS[group];
  const live = items.filter((i) => doc.items[i.key]?.state === 'live');
  return (
    <section aria-label={title ?? t('ens.title')}>
      <h3 className="set-sub">{title ?? t('ens.title')}</h3>
      {live.length > 0 ? (
        <>
          <div className="jx-list">
            {live.map((i) => (
              <SettingField key={i.key} row={{ key: i.key, kind: i.kind === 'bool' ? 'bool' : i.kind === 'num' ? 'num' : i.kind === 'text' ? 'text' : 'reply' }} label={t(`jx.${i.key}` as Key)} doc={doc} draft={draft} />
            ))}
          </div>
          <Note>{t('ens.note', { file: doc.file })}</Note>
          <ApplyBar draft={draft} doc={doc} />
        </>
      ) : null}
    </section>
  );
}
