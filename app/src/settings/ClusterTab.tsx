import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, getClusterCatalog, getClusterSpots, putCluster, sendClusterCommand, type ClusterConfig, type ClusterServer } from '../api/client';
import { useT, type T } from '../i18n';
import { Check, Help } from './common';

const MAX_SERVERS = 8;

interface Draft {
  enabled: boolean;
  callsign: string;
  servers: ClusterServer[];
  login: string;
  maxSpots: string;
  maxAge: string;
}

function toDraft(c: ClusterConfig): Draft {
  return { enabled: c.enabled, callsign: c.callsign, servers: c.servers, login: c.login_commands.join('\n'), maxSpots: String(c.max_spots), maxAge: String(c.max_age_min) };
}

const CALLSIGN = /^[A-Za-z0-9/-]{3,16}$/;
const HOST = /^[A-Za-z0-9.-]{1,253}$/;

/** Le impostazioni della scheda come configurazione per il core, o `null` se qualcosa non va. */
export function draftToConfig(d: Draft): ClusterConfig | null {
  const callsign = d.callsign.trim().toUpperCase();
  if (callsign !== '' && !(CALLSIGN.test(callsign) && /\d/.test(callsign))) return null;
  if (d.servers.length > MAX_SERVERS) return null;
  if (d.servers.some((s) => s.name.trim() === '' || !HOST.test(s.host) || !Number.isInteger(s.port) || s.port < 1 || s.port > 65535)) return null;
  const login = d.login.split(/\r?\n/).map((l) => l.trim()).filter((l) => l !== '');
  const prefixes = ['sh/', 'show/', 'sh ', 'show ', 'set/', 'unset/', 'accept/', 'reject/', 'clear/', 'help', 'bye'];
  if (login.length > 10 || login.some((l) => l.length > 80 || !prefixes.some((p) => l.toLowerCase().startsWith(p)))) return null;
  const maxSpots = Number(d.maxSpots);
  const maxAge = Number(d.maxAge);
  if (!Number.isInteger(maxSpots) || maxSpots < 50 || maxSpots > 2000) return null;
  if (!Number.isInteger(maxAge) || maxAge < 5 || maxAge > 1440) return null;
  return { enabled: d.enabled, callsign, servers: d.servers.map((s) => ({ ...s, name: s.name.trim() })), login_commands: login, max_spots: maxSpots, max_age_min: maxAge };
}

function errorText(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'bad_command') return t('cluster.command.refused');
    if (error.code === 'not_connected') return t('cluster.command.offline');
    if (error.code === 'bad_cluster') return t('cluster.invalid');
  }
  return t('cluster.failed');
}

/** Scheda DX Cluster: server pubblici o propri (telnet), nominativo, comandi, stato dei collegamenti. */
export function ClusterTab() {
  const t = useT();
  const client = useQueryClient();
  const state = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(1), retry: false, refetchInterval: 4000 });
  const catalog = useQuery({ queryKey: ['dx-cluster-catalog'], queryFn: getClusterCatalog, retry: false, staleTime: Infinity });
  const [draft, setDraft] = useState<Draft | null>(null);
  const [note, setNote] = useState('');
  const [pick, setPick] = useState('');
  const [custom, setCustom] = useState({ name: '', host: '', port: '7300' });
  const [command, setCommand] = useState({ server: '', text: '' });

  const shown: Draft | null = draft ?? (state.data ? toDraft(state.data.config) : null);
  const parsed = shown ? draftToConfig(shown) : null;

  const save = useMutation({
    mutationFn: (config: ClusterConfig) => putCluster(config),
    onSuccess: () => {
      setDraft(null);
      setNote(t('cluster.saved'));
      void client.invalidateQueries({ queryKey: ['dx-cluster'] });
    },
    onError: (e) => setNote(errorText(e, t)),
  });
  const send = useMutation({
    mutationFn: () => sendClusterCommand(command.server, command.text.trim()),
    onSuccess: () => {
      setNote(t('cluster.command.sent'));
      setCommand({ ...command, text: '' });
    },
    onError: (e) => setNote(errorText(e, t)),
  });

  if (state.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!state.data || !shown) return <p className="muted">{t('set.loading')}</p>;
  const set = (change: Partial<Draft>) => setDraft({ ...shown, ...change });

  const add = (server: ClusterServer) => {
    if (shown.servers.length >= MAX_SERVERS) return setNote(t('cluster.add.full'));
    if (shown.servers.some((s) => s.host === server.host && s.port === server.port)) return setNote(t('cluster.add.exists'));
    setNote('');
    set({ servers: [...shown.servers, server] });
  };
  const addPublic = () => {
    const entry = catalog.data?.find((c) => `${c.host}:${c.port}` === pick);
    if (entry) add({ name: entry.name, host: entry.host, port: entry.port, enabled: true });
    setPick('');
  };
  const addCustom = () => {
    const port = Number(custom.port);
    add({ name: custom.name.trim() || custom.host, host: custom.host.trim(), port, enabled: true });
    setCustom({ name: '', host: '', port: '7300' });
  };
  const changeServer = (index: number, change: Partial<ClusterServer>) =>
    set({ servers: shown.servers.map((s, i) => (i === index ? { ...s, ...change } : s)) });

  const connected = state.data.servers.filter((s) => s.state === 'connected');

  return (
    <>
      <Help>{t('cluster.intro')}</Help>
      <Check label={t('cluster.enabled')} hint={t('tip.cluster.enabled')} checked={shown.enabled} onChange={(v) => set({ enabled: v })} />
      <label className="auto-field" title={t('tip.cluster.callsign')}>
        {t('cluster.callsign')}
        <input type="text" value={shown.callsign} maxLength={16} onChange={(e) => set({ callsign: e.target.value.toUpperCase() })} />
      </label>

      <h3 className="set-sub">{t('cluster.servers')}</h3>
      <div className="cluster-servers" title={t('tip.cluster.server')}>
        {shown.servers.map((s, i) => (
          <div className="cluster-server" key={`${s.host}:${s.port}`}>
            <Check label={t('cluster.server.on')} checked={s.enabled} onChange={(v) => changeServer(i, { enabled: v })} />
            <strong>{s.name}</strong>
            <span className="data muted">
              {s.host}:{s.port}
            </span>
            <button type="button" className="glass chip" onClick={() => set({ servers: shown.servers.filter((_, k) => k !== i) })}>
              {t('cluster.server.remove')}
            </button>
          </div>
        ))}
      </div>

      <h3 className="set-sub">{t('cluster.add.heading')}</h3>
      <div className="cmd-line" title={t('tip.cluster.catalog')}>
        <select aria-label={t('cluster.add.catalog')} value={pick} onChange={(e) => setPick(e.target.value)}>
          <option value="">{t('cluster.add.choose')}</option>
          {(catalog.data ?? []).map((c) => (
            <option key={`${c.host}:${c.port}`} value={`${c.host}:${c.port}`}>
              {c.name} — {c.software}, {c.place} ({c.host}:{c.port})
            </option>
          ))}
        </select>
        <button type="button" className="glass" disabled={pick === ''} onClick={addPublic}>
          {t('cluster.add.button')}
        </button>
      </div>
      <p className="muted settings-hint">{pick ? (catalog.data?.find((c) => `${c.host}:${c.port}` === pick)?.note ?? '') : ''}</p>
      <div className="cmd-line">
        <strong>{t('cluster.add.custom')}</strong>
        <input type="text" aria-label={t('cluster.add.name')} placeholder={t('cluster.add.name')} value={custom.name} onChange={(e) => setCustom({ ...custom, name: e.target.value })} />
        <input type="text" aria-label={t('cluster.add.host')} placeholder={t('cluster.add.host')} value={custom.host} onChange={(e) => setCustom({ ...custom, host: e.target.value })} />
        <input type="number" className="set-number" aria-label={t('cluster.add.port')} min={1} max={65535} value={custom.port} onChange={(e) => setCustom({ ...custom, port: e.target.value })} />
        <button type="button" className="glass" disabled={!HOST.test(custom.host.trim()) || !(Number(custom.port) >= 1 && Number(custom.port) <= 65535)} onClick={addCustom}>
          {t('cluster.add.button')}
        </button>
      </div>

      <label className="auto-field" title={t('tip.cluster.login')}>
        {t('cluster.login')}
        <textarea rows={3} value={shown.login} spellCheck={false} onChange={(e) => set({ login: e.target.value })} />
      </label>
      <label className="set-check" title={t('tip.cluster.max')}>
        <span>{t('cluster.max')}</span>
        <input type="number" className="set-number" min={50} max={2000} value={shown.maxSpots} onChange={(e) => set({ maxSpots: e.target.value })} />
      </label>
      <label className="set-check" title={t('tip.cluster.age')}>
        <span>{t('cluster.age')}</span>
        <input type="number" className="set-number" min={5} max={1440} value={shown.maxAge} onChange={(e) => set({ maxAge: e.target.value })} />
      </label>
      {!parsed ? <p className="notice">{t('cluster.invalid')}</p> : null}
      <button type="button" className="glass glass--on" disabled={!parsed || save.isPending} onClick={() => parsed && save.mutate(parsed)}>
        {t('cluster.save')}
      </button>
      {note ? <p className="live-note" role="status">{note}</p> : null}

      <h3 className="set-sub">{t('cluster.state')}</h3>
      {state.data.servers.length === 0 ? <p className="muted">{t('cluster.state.none')}</p> : null}
      {state.data.servers.map((s) => (
        <p key={s.key} className={s.state === 'error' ? 'notice' : 'muted'}>
          <strong>{s.name}</strong> — {t(`cluster.state.${s.state}` as 'cluster.state.error', { n: s.spots, detail: s.detail ?? '' })}
        </p>
      ))}

      <h3 className="set-sub">{t('cluster.command')}</h3>
      <div className="cmd-line" title={t('tip.cluster.command')}>
        <select aria-label={t('cluster.command.server')} value={command.server} onChange={(e) => setCommand({ ...command, server: e.target.value })}>
          <option value="">{t('cluster.command.server')}</option>
          {connected.map((s) => (
            <option key={s.key} value={s.key}>
              {s.name}
            </option>
          ))}
        </select>
        <input type="text" aria-label={t('cluster.command')} placeholder="sh/dx 20" maxLength={80} value={command.text} onChange={(e) => setCommand({ ...command, text: e.target.value })} />
        <button type="button" className="glass" disabled={command.server === '' || command.text.trim() === '' || send.isPending} onClick={() => send.mutate()}>
          {t('cluster.command.send')}
        </button>
      </div>
    </>
  );
}
