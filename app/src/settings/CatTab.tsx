import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import {
  ApiError,
  catProfile,
  connectCat,
  detectCat,
  disconnectCat,
  getCat,
  getCatModels,
  getCatPorts,
  putCat,
  type CatCandidate,
  type CatConfig,
  type CatDetection,
  type CatHandshake,
  type CatMode,
  type CatPtt,
  type CatSnapshot,
  type CatSplit,
  type CatState,
} from '../api/client';
import { useT, type Key, type T } from '../i18n';
import { BAUDS, applyCandidate, catIssue, filterModels, modelForCandidate, modelLabel, sameCat, visibleFields } from '../lib/cat';
import { Check, Help, Row } from './common';
import { Actions, Cell, Cells, Field, Group, Message, Notes, Page } from './layout';

const STATE_KEY: Record<CatState, Key> = {
  disconnected: 'cat.state.disconnected',
  connecting: 'cat.state.connecting',
  connected: 'cat.state.connected',
  reconnecting: 'cat.state.reconnecting',
  error: 'cat.state.error',
};

const LED: Record<CatState, string> = { disconnected: '', connecting: 'led--testing', connected: 'led--ok', reconnecting: 'led--testing', error: 'led--fail' };

const HANDSHAKES: ReadonlyArray<[CatHandshake, Key]> = [
  ['default', 'cat.default'],
  ['none', 'cat.handshake.none'],
  ['xonxoff', 'cat.handshake.xonxoff'],
  ['hardware', 'cat.handshake.hardware'],
];
const PTTS: ReadonlyArray<[CatPtt, Key]> = [
  ['cat', 'cat.ptt.cat'],
  ['dtr', 'cat.ptt.dtr'],
  ['rts', 'cat.ptt.rts'],
  ['vox', 'cat.ptt.vox'],
];
const SPLITS: ReadonlyArray<[CatSplit, Key]> = [
  ['none', 'cat.split.none'],
  ['rig', 'cat.split.rig'],
  ['emulate', 'cat.split.emulate'],
];
const MODES: ReadonlyArray<[CatMode, Key]> = [
  ['none', 'cat.catmode.none'],
  ['usb', 'cat.catmode.usb'],
  ['data', 'cat.catmode.data'],
];

function errorText(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'local_only') return t('cat.err.local_only');
    if (error.code === 'no_such_profile') return t('cat.err.profile');
    if (error.code === 'bad_name') return t('cat.err.name');
    if (error.code === 'write_failed') return t('cat.err.write');
    if (error.detail) return error.detail;
    if (error.code.startsWith('bad_')) return t('cat.invalid');
  }
  return t('cat.err.generic');
}

function lineValue(v: boolean | null): string {
  return v === null ? 'auto' : v ? 'high' : 'low';
}

function lineFrom(text: string): boolean | null {
  return text === 'auto' ? null : text === 'high';
}

/** Stato del collegamento: led, testo, e se collegata frequenza, modo e PTT letti dalla radio. */
function Status({ snap }: { snap: CatSnapshot }) {
  const t = useT();
  const { status } = snap;
  const rig = status.rig;
  return (
    <>
      <p role="status" className="cat-state sx-message">
        <span className={`led ${LED[status.state]}`} aria-hidden="true" /> <strong>{t(STATE_KEY[status.state])}</strong>
        {status.message ? ` — ${status.message}` : ''}
      </p>
      {status.state === 'connected' && rig ? (
        <Cells min={12}>
          <Row name={t('cat.freq')}>{rig.frequency_hz === null ? '—' : `${(rig.frequency_hz / 1000).toFixed(3)} kHz`}</Row>
          <Row name={t('cat.radiomode')}>{rig.mode ?? '—'}</Row>
          <Row name={t('cat.ptt.state')}>
            {rig.ptt ? t('cat.ptt.on') : t('cat.ptt.off')}
            {rig.ptt_blocked ? ` (${t('cat.ptt.blocked')})` : ''}
          </Row>
          {rig.failures > 0 ? <Message kind="muted">{t('cat.failures', { n: rig.failures })}</Message> : null}
        </Cells>
      ) : null}
    </>
  );
}

/** Le radio rilevate dal sistema (passivo): ognuna si porta nella bozza con «Usa», senza cambiare nulla finche' non si applica. */
function Detected({ detection, onUse }: { detection: CatDetection; onUse(c: CatCandidate): void }) {
  const t = useT();
  if (detection.candidates.length === 0) return <p className="muted">{t('cat.detected.none')}</p>;
  return (
    <ul className="cat-detected" aria-label={t('cat.detected')}>
      {detection.candidates.map((c) => (
        <li key={`${c.device_key}:${c.cat_port}`}>
          <div>
            <strong>{c.rig_label}</strong> — {c.cat_port}
            {c.baud ? `, ${c.baud} baud` : ''} <span className="muted">({t('cat.detected.confidence', { n: c.confidence })})</span>
            <div className="muted">{c.evidence}</div>
            {c.is_radio && c.rig_token === '' ? <div className="muted">{t('cat.detected.manual')}</div> : null}
            {c.audio_input ? <div className="muted">{t('cat.detected.audio', { name: c.audio_input })}</div> : null}
          </div>
          <button type="button" className="glass" onClick={() => onUse(c)}>
            {t('cat.detected.use')}
          </button>
        </li>
      ))}
    </ul>
  );
}

/** I riquadri della scheda Radio: collegamento, PTT, split, lettura, profili, prova, condivisione. Si mettono nella griglia di una pagina. */
export function CatGroups() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 1500 });
  const [draft, setDraft] = useState<CatConfig | null>(null);
  const [search, setSearch] = useState('');
  const [note, setNote] = useState('');
  const [problem, setProblem] = useState('');
  const [profileName, setProfileName] = useState('');
  const [picked, setPicked] = useState('');
  const [detection, setDetection] = useState<CatDetection | null>(null);

  const config: CatConfig | null = draft ?? snap.data?.config ?? null;
  const hamlib = config?.backend === 'hamlib';
  const models = useQuery({ queryKey: ['cat-models'], queryFn: getCatModels, retry: false, staleTime: Infinity, enabled: hamlib });
  const ports = useQuery({ queryKey: ['cat-ports'], queryFn: getCatPorts, retry: false, staleTime: 30_000, enabled: hamlib });

  // il nome del profilo proposto segue quello caricato
  const active = snap.data?.active_profile ?? '';
  useEffect(() => {
    setProfileName(active);
    setPicked(active);
  }, [active]);

  const dirty = draft !== null && snap.data !== undefined && !sameCat(draft, snap.data.config);
  const issue = config ? catIssue(config) : null;

  const done = (message: string) => {
    setProblem('');
    setNote(message);
    void client.invalidateQueries({ queryKey: ['cat'] });
  };
  const failed = (e: unknown) => {
    setNote('');
    setProblem(errorText(e, t));
  };

  const apply = useMutation({
    mutationFn: (c: CatConfig) => putCat(c),
    onSuccess: () => {
      setDraft(null);
      done(t('cat.applied'));
    },
    onError: failed,
  });
  const connect = useMutation({
    // con modifiche in sospeso si applicano prima: si collega sempre cio' che si vede
    mutationFn: async () => {
      if (draft && dirty) await putCat(draft);
      return connectCat();
    },
    onSuccess: () => {
      setDraft(null);
      done('');
    },
    onError: failed,
  });
  const disconnect = useMutation({ mutationFn: () => disconnectCat(), onSuccess: () => done(''), onError: failed });
  const profile = useMutation({
    mutationFn: ({ action, name }: { action: 'save' | 'load' | 'new' | 'delete'; name?: string }) => catProfile(action, name),
    onSuccess: (_, { action }) => {
      if (action !== 'save') setDraft(null);
      done(t(({ save: 'cat.profile.saved', load: 'cat.profile.loaded', new: 'cat.profile.created', delete: 'cat.profile.deleted' } as const)[action]));
    },
    onError: failed,
  });
  const detect = useMutation({
    mutationFn: () => detectCat(),
    onSuccess: (d) => {
      setProblem('');
      setDetection(d);
    },
    onError: failed,
  });

  const modelList = useMemo(() => models.data ?? [], [models.data]);
  const shown = useMemo(() => {
    const found = filterModels(modelList, search).slice(0, 300);
    const current = config ? modelList.find((m) => m.id === config.model) : undefined;
    return current && !found.some((m) => m.id === current.id) ? [current, ...found] : found;
  }, [modelList, search, config]);

  if (snap.isError) return <p className="notice sx-span-3">{t('set.loadError')}</p>;
  if (!snap.data || !config) return <p className="muted sx-span-3">{t('set.loading')}</p>;
  const set = (change: Partial<CatConfig>) => setDraft({ ...config, ...change });
  const fields = visibleFields(config, modelList.find((m) => m.id === config.model));
  const busy = apply.isPending || connect.isPending || disconnect.isPending;
  const state = snap.data.status.state;
  const active_ = state === 'connected' || state === 'connecting' || state === 'reconnecting';
  const useCandidate = (c: CatCandidate) => {
    setDraft(applyCandidate(config, c, modelForCandidate(modelList, c)));
    setNote(t('cat.detected.pick'));
  };

  const link = (
    <Group title={t('cat.grp.link')} span={2}>
      {hamlib && !snap.data.status.rigctld_found ? <Message kind="alert">{t('cat.missing')}</Message> : null}
      <Cells min={17}>
        <Field label={t('cat.backend')} hint={t('tip.cat.backend')}>
          <select value={config.backend} onChange={(e) => set({ backend: e.target.value as CatConfig['backend'] })}>
            <option value="hamlib">{t('cat.backend.hamlib')}</option>
            <option value="omnirig">{t('cat.backend.omnirig')}</option>
          </select>
        </Field>
        {fields.rig ? (
          <>
            <Field label={t('cat.rig.search')}>
              <input type="search" value={search} onChange={(e) => setSearch(e.target.value)} />
            </Field>
            <Field label={t('cat.rig')}>
              <select
                value={config.model}
                onChange={(e) => {
                  const m = modelList.find((x) => x.id === Number(e.target.value));
                  set({ model: m?.id ?? 0, rig_name: m ? modelLabel(m) : '' });
                }}
              >
                <option value={0}>{t('cat.rig.none')}</option>
                {shown.map((m) => (
                  <option key={m.id} value={m.id}>
                    {modelLabel(m)}
                  </option>
                ))}
              </select>
            </Field>
          </>
        ) : null}
        {fields.port ? (
          <Field label={t('cat.port')} hint={t('tip.cat.port')}>
            <input type="text" list="cat-ports" value={config.port} spellCheck={false} onChange={(e) => set({ port: e.target.value.trim() })} />
          </Field>
        ) : null}
        {fields.port && fields.baud ? (
          <Field label={t('cat.baud')}>
            <select value={config.baud} onChange={(e) => set({ baud: Number(e.target.value) })}>
              <option value={0}>{t('cat.default')}</option>
              {BAUDS.map((b) => (
                <option key={b} value={b}>
                  {b}
                </option>
              ))}
            </select>
          </Field>
        ) : null}
        {fields.serial ? (
          <>
            <Field label={t('cat.databits')}>
              <select value={config.data_bits} onChange={(e) => set({ data_bits: Number(e.target.value) })}>
                <option value={0}>{t('cat.default')}</option>
                <option value={7}>7</option>
                <option value={8}>8</option>
              </select>
            </Field>
            <Field label={t('cat.stopbits')}>
              <select value={config.stop_bits} onChange={(e) => set({ stop_bits: Number(e.target.value) })}>
                <option value={0}>{t('cat.default')}</option>
                <option value={1}>1</option>
                <option value={2}>2</option>
              </select>
            </Field>
            <Field label={t('cat.handshake')} hint={t('tip.cat.handshake')}>
              <select value={config.handshake} onChange={(e) => set({ handshake: e.target.value as CatHandshake })}>
                {HANDSHAKES.map(([v, k]) => (
                  <option key={v} value={v}>
                    {t(k)}
                  </option>
                ))}
              </select>
            </Field>
            <Field label={t('cat.dtr')} hint={t('tip.cat.line')}>
              <select value={lineValue(config.force_dtr)} onChange={(e) => set({ force_dtr: lineFrom(e.target.value) })}>
                <option value="auto">{t('cat.line.auto')}</option>
                <option value="high">{t('cat.line.high')}</option>
                <option value="low">{t('cat.line.low')}</option>
              </select>
            </Field>
            <Field label={t('cat.rts')} hint={t('tip.cat.line')}>
              <select value={lineValue(config.force_rts)} onChange={(e) => set({ force_rts: lineFrom(e.target.value) })}>
                <option value="auto">{t('cat.line.auto')}</option>
                <option value="high">{t('cat.line.high')}</option>
                <option value="low">{t('cat.line.low')}</option>
              </select>
            </Field>
            {fields.civ ? (
              <Field label={t('cat.civ')} hint={t('tip.cat.civ')}>
                <input type="number" min={0} max={255} value={config.civ_address} onChange={(e) => set({ civ_address: Number(e.target.value) })} />
              </Field>
            ) : null}
          </>
        ) : null}
        {fields.port ? (
          <Cell>
            <button type="button" className="glass" onClick={() => void ports.refetch()}>
              {t('cat.ports.refresh')}
            </button>
          </Cell>
        ) : null}
      </Cells>
      {fields.rig ? <Message kind="muted">{models.isLoading ? t('cat.rig.loading') : models.isError ? t('cat.rig.failed') : t('cat.rig.count', { n: modelList.length })}</Message> : null}
      {!hamlib ? <Message kind="muted">{t('cat.omnirig.help')}</Message> : null}
      <datalist id="cat-ports">
        {(ports.data ?? []).filter((p) => !p.virtual || p.virtual_kind === 'vspe' || p.virtual_kind === 'com0com').map((p) => (
          <option key={p.port} value={p.port}>
            {p.virtual_kind === 'vspe' ? `${t('cat.port.vspe')} — ${p.name}` : p.virtual ? `${t('cat.port.virtual')} — ${p.name}` : p.name}
          </option>
        ))}
      </datalist>
      <Actions>
        <button type="button" className="glass glass--on" title={t('tip.cat.apply')} disabled={busy || !dirty || issue !== null} onClick={() => apply.mutate(config)}>
          {t('cat.apply')}
        </button>
        {issue ? <span className="notice">{t('cat.invalid')}</span> : dirty ? <span className="muted">{t('cat.dirty')}</span> : null}
        {problem ? (
          <span className="notice" role="alert">
            {problem}
          </span>
        ) : null}
        {note ? (
          <span className="live-note" role="status">
            {note}
          </span>
        ) : null}
      </Actions>
    </Group>
  );

  return (
    <>
      {link}

      <Group title={t('cat.grp.test')}>
        <Status snap={snap.data} />
        <Actions>
          <button type="button" className="glass" title={t('tip.cat.connect')} disabled={busy || issue !== null || (config.backend === 'hamlib' && config.model === 0)} onClick={() => connect.mutate()}>
            {t('cat.connect')}
          </button>
          <button type="button" className="glass" title={t('tip.cat.disconnect')} disabled={busy || (!active_ && state !== 'error')} onClick={() => disconnect.mutate()}>
            {t('cat.disconnect')}
          </button>
          {hamlib ? (
            <button type="button" className="glass" title={t('tip.cat.detect')} disabled={detect.isPending} onClick={() => detect.mutate()}>
              {detect.isPending ? t('cat.detecting') : t('cat.detect')}
            </button>
          ) : null}
        </Actions>
        {detection ? <Detected detection={detection} onUse={useCandidate} /> : null}
      </Group>

      {fields.ptt ? (
        <Group title={t('cat.grp.ptt')}>
          <Cells min={14}>
            <Field label={t('cat.ptt.method')} hint={t('tip.cat.ptt')}>
              <select value={config.ptt} onChange={(e) => set({ ptt: e.target.value as CatPtt })}>
                {PTTS.map(([v, k]) => (
                  <option key={v} value={v}>
                    {t(k)}
                  </option>
                ))}
              </select>
            </Field>
            {fields.pttPort ? (
              <Field label={t('cat.ptt.port')}>
                <input type="text" list="cat-ports" value={config.ptt_port} spellCheck={false} onChange={(e) => set({ ptt_port: e.target.value.trim() })} />
              </Field>
            ) : null}
          </Cells>
        </Group>
      ) : null}

      <Group title={t('cat.grp.split')}>
        <Cells min={14}>
          <Field label={t('cat.split')} hint={t('tip.cat.split')}>
            <select value={config.split} onChange={(e) => set({ split: e.target.value as CatSplit })}>
              {SPLITS.map(([v, k]) => (
                <option key={v} value={v}>
                  {t(k)}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t('cat.catmode')} hint={t('tip.cat.catmode')}>
            <select value={config.cat_mode} onChange={(e) => set({ cat_mode: e.target.value as CatMode })}>
              {MODES.map(([v, k]) => (
                <option key={v} value={v}>
                  {t(k)}
                </option>
              ))}
            </select>
          </Field>
          {fields.poll ? (
            <Field label={t('cat.poll')} hint={t('tip.cat.poll')}>
              <input type="number" min={1} max={99} value={config.poll_secs} onChange={(e) => set({ poll_secs: Number(e.target.value) })} />
            </Field>
          ) : null}
          <Check label={t('cat.auto')} hint={t('tip.cat.auto')} checked={config.auto_connect} onChange={(v) => set({ auto_connect: v })} />
        </Cells>
      </Group>

      <Group title={t('cat.profile')}>
        <Cells min={14}>
          <Field label={t('cat.profile.pick')} hint={t('tip.cat.profile')}>
            <select value={picked} onChange={(e) => { setPicked(e.target.value); if (e.target.value) setProfileName(e.target.value); }}>
              <option value="">{snap.data.profiles.length === 0 ? t('cat.profile.none') : '—'}</option>
              {snap.data.profiles.map((p) => (
                <option key={p.name} value={p.name}>
                  {p.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t('cat.profile.name')} hint={t('tip.cat.profile')}>
            <input type="text" maxLength={80} value={profileName} onChange={(e) => setProfileName(e.target.value)} />
          </Field>
        </Cells>
        <Actions>
          <button type="button" className="glass" disabled={picked === '' || profile.isPending} onClick={() => profile.mutate({ action: 'load', name: picked })}>
            {t('cat.profile.load')}
          </button>
          <button type="button" className="glass" disabled={profileName.trim() === '' || profile.isPending} onClick={() => profile.mutate({ action: 'save', name: profileName })}>
            {t('cat.profile.save')}
          </button>
          <button type="button" className="glass" disabled={profile.isPending} onClick={() => profile.mutate({ action: 'new', name: profileName.trim() === '' || snap.data.profiles.some((p) => p.name === profileName.trim()) ? undefined : profileName })}>
            {t('cat.profile.new')}
          </button>
          <button type="button" className="glass" disabled={picked === '' || profile.isPending} onClick={() => profile.mutate({ action: 'delete', name: picked })}>
            {t('cat.profile.delete')}
          </button>
        </Actions>
      </Group>

      {hamlib ? (
        <Group title={t('cat.share')}>
          <Cells min={14}>
            <Check label={t('cat.share.on')} hint={t('tip.cat.share')} checked={config.share_enabled} onChange={(v) => set({ share_enabled: v })} />
            {config.share_enabled ? (
              <Field label={t('cat.share.port')} hint={t('tip.cat.share.port')}>
                <input type="number" min={1024} max={65535} value={config.share_port} onChange={(e) => set({ share_port: Number(e.target.value) })} />
              </Field>
            ) : null}
          </Cells>
          {snap.data.status.share ? (
            <Message>{t('cat.share.where', { host: snap.data.status.share.host, port: snap.data.status.share.port })}</Message>
          ) : config.share_enabled ? (
            <Message kind="muted">{t('cat.share.later')}</Message>
          ) : null}
        </Group>
      ) : null}

    </>
  );
}

/** Le note della scheda Radio: come funziona il collegamento e la condivisione, in un riquadro comprimibile. */
export function CatNotes() {
  const t = useT();
  return (
    <Notes>
      <Help>{t('cat.intro')}</Help>
      <Help>{t('cat.share.help')}</Help>
      <Help>{t('cat.share.com')}</Help>
    </Notes>
  );
}

/** Scheda Radio CAT (le stesse voci, nei riquadri della scheda Radio). */
export function CatTab() {
  return (
    <Page>
      <CatGroups />
      <CatNotes />
    </Page>
  );
}
