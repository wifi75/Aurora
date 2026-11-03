import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, checkUpdate, getDiagnostics, getUpdate, installUpdate, openCrashFolder, putUpdate, type UpdateConfigInput } from '../api/client';
import { useT } from '../i18n';
import { Check, Help, Row } from './common';
import { Actions, Cell, Cells, Field, Group, Message, Notes, Page } from './layout';
import { useAutosave } from './useAutosave';

interface Draft {
  enabled: boolean;
  repo: string;
  hours: string;
  prereleases: boolean;
  /** Il token nuovo scritto qui; vuoto = non lo si cambia. */
  token: string;
  removeToken: boolean;
}

/** Il testo della scheda in un'impostazione valida per il core, o `null` se i valori non vanno. */
export function draftToUpdate(d: Draft): UpdateConfigInput | null {
  const repo = d.repo.trim();
  const hours = Number(d.hours);
  if (!/^[A-Za-z0-9_.-]{1,100}\/[A-Za-z0-9_.-]{1,100}$/.test(repo)) return null;
  if (!Number.isInteger(hours) || hours < 1 || hours > 168) return null;
  const token = d.token.trim();
  if (token.length > 255 || /\s/.test(token)) return null;
  const input: UpdateConfigInput = { enabled: d.enabled, repo, interval_hours: hours, prereleases: d.prereleases };
  if (d.removeToken) input.token = '';
  else if (token !== '') input.token = token;
  return input;
}

/** Scheda Aggiornamenti: dove e quando cercare le nuove versioni di Aurora. */
export function UpdatesTab() {
  const t = useT();
  const client = useQueryClient();
  const update = useQuery({ queryKey: ['update'], queryFn: getUpdate, retry: false });
  const diagnostics = useQuery({ queryKey: ['diagnostics'], queryFn: getDiagnostics, retry: false });
  const [crashFailure, setCrashFailure] = useState('');
  const openCrashes = () => {
    setCrashFailure('');
    openCrashFolder().catch((error: unknown) => setCrashFailure(t('diag.openFailed', { message: error instanceof ApiError ? error.code : '—' })));
  };
  const [draft, setDraft] = useState<Draft | null>(null);
  const [note, setNote] = useState('');
  const [installing, setInstalling] = useState(false);
  const [installFailure, setInstallFailure] = useState('');
  const install = async () => {
    setInstalling(true);
    setInstallFailure('');
    try {
      await installUpdate();
    } catch (error) {
      setInstalling(false);
      setInstallFailure(t('upd.installFailed', { message: error instanceof ApiError ? error.code : '—' }));
    }
  };

  const status = update.data;
  const shown: Draft | null = draft ?? (status ? { enabled: status.enabled, repo: status.repo, hours: String(status.interval_hours), prereleases: status.prereleases, token: '', removeToken: false } : null);
  const parsed = shown ? draftToUpdate(shown) : null;

  const save = useMutation({
    mutationFn: (input: UpdateConfigInput) => putUpdate(input),
    onSuccess: () => {
      setDraft(null);
      setNote(t('upd.saved'));
      void client.invalidateQueries({ queryKey: ['update'] });
    },
    onError: () => setNote(t('upd.invalid')),
  });
  const check = useMutation({
    mutationFn: () => checkUpdate(),
    onSuccess: (result) => {
      setNote(result.error ? t('upd.offline', { message: result.error }) : '');
      void client.invalidateQueries({ queryKey: ['update'] });
    },
    onError: () => setNote(t('upd.offline', { message: '—' })),
  });

  useAutosave(
    parsed,
    draft !== null,
    (input) => save.mutate(input),
    (input) => void putUpdate(input).catch(() => undefined),
  );

  if (update.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!status || !shown) return <p className="muted">{t('set.loading')}</p>;
  const set = (change: Partial<Draft>) => setDraft({ ...shown, ...change });

  return (
    <Page>
      <Group title={t('lay.status')}>
        <Cells min={17}>
          <Row name={t('upd.current')}>{status.current}</Row>
          <Row name={t('upd.latest')}>
            {status.latest ? (
              status.available ? (
                // versione nuova: ben visibile, con la stella e «NEW»
                <span className="upd-new">
                  <span className="upd-new-star" aria-hidden="true">★</span>
                  {status.latest.version}
                  <span className="upd-new-tag">NEW</span>
                </span>
              ) : (
                status.latest.version
              )
            ) : (
              '—'
            )}
          </Row>
          <Row name={t('upd.lastCheck')}>{status.checked_at_ms > 0 ? new Date(status.checked_at_ms).toLocaleString() : t('upd.never')}</Row>
        </Cells>
        {status.available ? (
          <p className="live-note sx-message" role="status">
            {t('upd.available', { version: status.available.version })}{' '}
            {status.available.url ? (
              <a href={status.available.url} target="_blank" rel="noreferrer noopener">
                {t('upd.open')}
              </a>
            ) : null}
          </p>
        ) : status.latest ? (
          <Message kind="muted">{t('upd.uptodate')}</Message>
        ) : null}
        {status.error ? <Message kind="alert">{t('upd.offline', { message: status.error })}</Message> : null}
        {installing ? <Message kind="muted">{t('upd.installing')}</Message> : null}
        {installFailure ? <Message kind="alert">{installFailure}</Message> : null}
        <Actions>
          {status.available && status.can_install ? (
            <button type="button" className="glass glass--on" disabled={installing} onClick={() => void install()}>
              {t('upd.install')}
            </button>
          ) : null}
          <button type="button" className="glass" disabled={check.isPending || draft !== null} onClick={() => check.mutate()}>
            {t('upd.check')}
          </button>
        </Actions>
      </Group>

      <Group title={t('lay.settings')} span={2}>
        <Cells min={17}>
          <Check label={t('upd.enabled')} hint={t('tip.upd.enabled')} checked={shown.enabled} onChange={(v) => set({ enabled: v })} />
          {/* repository, versioni di prova e token restano quelli di fabbrica: servono solo a chi sviluppa, non a chi usa Aurora */}
          <Field label={t('upd.interval')} hint={t('tip.upd.interval')}>
            <input type="number" className="set-number" min={1} max={168} value={shown.hours} onChange={(e) => set({ hours: e.target.value })} />
          </Field>
        </Cells>
        {!parsed ? <Message kind="alert">{t('upd.invalid')}</Message> : null}
        {note ? <Message>{note}</Message> : null}
      </Group>

      {diagnostics.data?.crash_folder ? (
        <Group title={t('diag.title')}>
          <Cells min={17}>
            <Cell label={t('diag.crashFolder')} hint={t('tip.diag.crashFolder')}>
              <button type="button" className="glass" onClick={openCrashes}>
                {t('diag.openCrashFolder')}
              </button>
            </Cell>
          </Cells>
          {crashFailure ? <Message kind="alert">{crashFailure}</Message> : null}
        </Group>
      ) : null}

      <Notes>
        <Help>{t('upd.intro')}</Help>
        <Help>{t('tip.upd.token')}</Help>
      </Notes>
    </Page>
  );
}
