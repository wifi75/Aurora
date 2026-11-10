import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { ApiError, getCatPorts, getRotator, getRotatorModels, putRotator, type RotatorConfig } from '../api/client';
import { useT } from '../i18n';
import { BAUDS, filterModels, modelLabel } from '../lib/cat';
import { Check } from './common';
import { Actions, Cell, Cells, Field, Group, Message, Page } from './layout';

/** Scheda Rotore: come si raggiunge rotctld (lanciato da Aurora o gia' acceso), modello, porta e velocita'. */
export function RotatorTab() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['rotator'], queryFn: getRotator, retry: false, refetchInterval: 1500 });
  const [draft, setDraft] = useState<RotatorConfig | null>(null);
  const [search, setSearch] = useState('');
  const [note, setNote] = useState('');
  const [problem, setProblem] = useState('');
  const config = draft ?? snap.data?.config ?? null;
  const launch = config?.connection === 'launch';
  const models = useQuery({ queryKey: ['rotator-models'], queryFn: getRotatorModels, retry: false, staleTime: Infinity, enabled: launch });
  const ports = useQuery({ queryKey: ['cat-ports'], queryFn: getCatPorts, retry: false, staleTime: 30_000, enabled: launch });

  const save = useMutation({
    mutationFn: (c: RotatorConfig) => putRotator(c),
    onSuccess: (s) => {
      setDraft(null);
      setProblem('');
      setNote(t('rot.set.saved'));
      client.setQueryData(['rotator'], s);
    },
    onError: (e) => {
      setNote('');
      setProblem(e instanceof ApiError && e.detail ? e.detail : t('rot.err.failed'));
    },
  });

  const modelList = useMemo(() => models.data ?? [], [models.data]);
  const shown = useMemo(() => {
    const found = filterModels(modelList, search).slice(0, 300);
    const current = config ? modelList.find((m) => m.id === config.model) : undefined;
    return current && !found.some((m) => m.id === current.id) ? [current, ...found] : found;
  }, [modelList, search, config]);

  if (snap.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!snap.data || !config) return <p className="muted">{t('set.loading')}</p>;
  const set = (change: Partial<RotatorConfig>) => setDraft({ ...config, ...change });
  const status = snap.data.status;
  const stateText = status.state === 'connected' ? (status.moving ? t('rot.state.moving') : t('rot.state.connected')) : t(`rot.state.${status.state}`);

  return (
    <Page>
      <Group title={t('rot.set.grp.link')} span={2}>
        {launch && !status.rotctld_found ? <Message kind="alert">{t('rot.set.missing')}</Message> : null}
        <Cells min={17}>
          <Check label={t('rot.set.enabled')} hint={t('tip.rot.enabled')} checked={config.enabled} onChange={(v) => save.mutate({ ...config, enabled: v })} />
          <Field label={t('rot.set.connection')} hint={t('tip.rot.connection')}>
            <select value={config.connection} onChange={(e) => set({ connection: e.target.value as RotatorConfig['connection'] })}>
              <option value="launch">{t('rot.set.launch')}</option>
              <option value="remote">{t('rot.set.remote')}</option>
            </select>
          </Field>
          {launch ? (
            <>
              <Field label={t('rot.set.search')} hint={t('tip.rot.search')}>
                <input type="search" value={search} onChange={(e) => setSearch(e.target.value)} />
                {search.trim() !== '' ? (
                  <ul className="cat-search-results">
                    {shown.slice(0, 12).map((m) => (
                      <li key={m.id}>
                        <button
                          type="button"
                          className={`glass chip${m.id === config.model ? ' glass--on' : ''}`}
                          aria-pressed={m.id === config.model}
                          onClick={() => {
                            set({ model: m.id, model_name: modelLabel(m) });
                            setSearch('');
                          }}
                        >
                          {modelLabel(m)}
                        </button>
                      </li>
                    ))}
                    {shown.length === 0 ? <li className="muted">—</li> : null}
                  </ul>
                ) : null}
              </Field>
              <Field label={t('rot.set.model')} hint={t('tip.rot.model')}>
                <select
                  value={config.model}
                  onChange={(e) => {
                    const m = modelList.find((x) => x.id === Number(e.target.value));
                    set({ model: m?.id ?? 0, model_name: m ? modelLabel(m) : '' });
                  }}
                >
                  <option value={0}>{t('rot.set.none')}</option>
                  {/* il modello salvato resta visibile anche se l'elenco non e' ancora arrivato */}
                  {config.model !== 0 && !shown.some((m) => m.id === config.model) ? <option value={config.model}>{config.model_name || `#${config.model}`}</option> : null}
                  {shown.map((m) => (
                    <option key={m.id} value={m.id}>
                      {modelLabel(m)}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label={t('rot.set.port')} hint={t('tip.rot.port')}>
                <input type="text" list="rot-ports" value={config.port} spellCheck={false} autoComplete="off" placeholder="COM5" onChange={(e) => set({ port: e.target.value.trim() })} />
                <datalist id="rot-ports">
                  {(ports.data ?? []).map((p) => (
                    <option key={p.port} value={p.port}>
                      {p.name}
                    </option>
                  ))}
                </datalist>
              </Field>
              <Field label={t('rot.set.baud')} hint={t('tip.rot.baud')}>
                <select value={config.baud} onChange={(e) => set({ baud: Number(e.target.value) })}>
                  <option value={0}>{t('rot.set.default')}</option>
                  {BAUDS.map((b) => (
                    <option key={b} value={b}>
                      {b}
                    </option>
                  ))}
                </select>
              </Field>
            </>
          ) : (
            <>
              <Field label={t('rot.set.host')} hint={t('tip.rot.host')}>
                <input type="text" value={config.host} spellCheck={false} autoComplete="off" placeholder="127.0.0.1" onChange={(e) => set({ host: e.target.value.trim() })} />
              </Field>
              <Field label={t('rot.set.tcpPort')} hint={t('tip.rot.tcpPort')}>
                <input type="number" min={1} max={65535} value={config.tcp_port} onChange={(e) => set({ tcp_port: Number(e.target.value) || 0 })} />
              </Field>
            </>
          )}
          <Cell label={t('rot.set.state')} hint={t('tip.rot.state')}>
            <span className="data">{stateText}</span>
          </Cell>
        </Cells>
        {launch ? <Message kind="muted">{models.isLoading ? t('rot.set.models.loading') : models.isError ? t('rot.set.models.failed') : t('rot.set.models.count', { n: modelList.length })}</Message> : null}
        {status.state === 'error' && status.error ? <Message kind="alert">{status.error}</Message> : null}
        {problem ? <Message kind="alert">{problem}</Message> : null}
        {note && !problem ? <Message>{note}</Message> : null}
        <Actions>
          <button type="button" className="glass" disabled={draft === null || save.isPending} onClick={() => draft && save.mutate(draft)}>
            {t('rot.set.apply')}
          </button>
        </Actions>
      </Group>
    </Page>
  );
}
