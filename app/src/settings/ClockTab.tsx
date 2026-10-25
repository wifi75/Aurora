import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, checkClock, correctClock, getClock, putClock, type ClockConfig, type ClockMeasurement } from '../api/client';
import { useT, type T } from '../i18n';
import { clockInTolerance, offsetLabel } from '../lib/clock';
import { Check, Help, Row } from './common';
import { Actions, Cells, Field, Group, Message, Notes, Page } from './layout';
import { useAutosave } from './useAutosave';

interface Draft {
  enabled: boolean;
  servers: string;
  minutes: string;
  warn: string;
  auto: boolean;
}

function toDraft(c: ClockConfig): Draft {
  return { enabled: c.enabled, servers: c.servers.join('\n'), minutes: String(Math.round(c.interval_secs / 60)), warn: String(c.warn_ms), auto: c.auto_correct };
}

/** Il testo della scheda in un'impostazione valida per il core, o `null` se i valori non vanno. */
export function draftToConfig(d: Draft): ClockConfig | null {
  const servers = d.servers.split(/\r?\n/).map((s) => s.trim()).filter((s) => s !== '');
  const minutes = Number(d.minutes);
  const warn = Number(d.warn);
  if (servers.length < 1 || servers.length > 8) return null;
  if (!Number.isInteger(minutes) || minutes < 1 || minutes > 1440) return null;
  if (!Number.isInteger(warn) || warn < 10 || warn > 5000) return null;
  return { enabled: d.enabled, servers, interval_secs: minutes * 60, warn_ms: warn, auto_correct: d.auto };
}

function errorText(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'clock_needs_admin') return t('clock.needsAdmin');
    if (error.code === 'local_only') return t('clock.pcOnly');
    if (error.code === 'bad_clock') return t('clock.invalid');
  }
  return t('clock.failed');
}

function Status({ last, warnMs }: { last: ClockMeasurement | null; warnMs: number }) {
  const t = useT();
  if (!last) return <p className="muted">{t('clock.none')}</p>;
  if (last.offset_ms === null) return <p className="notice">{last.error ?? t('clock.failed')}</p>;
  const good = clockInTolerance(last.offset_ms, warnMs);
  const { ms, behind } = offsetLabel(last.offset_ms);
  return (
    <>
      <p className={good ? 'live-note' : 'notice'} role="status">
        {good ? t('clock.good') : t('clock.bad')}
      </p>
      <Cells min={13}>
        <Row name={t('clock.offset')}>{t(behind ? 'clock.behind' : 'clock.ahead', { ms })}</Row>
        <Row name={t('clock.delay')}>{last.delay_ms === null ? '—' : `${Math.round(last.delay_ms)} ms`}</Row>
        <Row name={t('clock.server')}>{last.server ?? '—'}</Row>
        <Row name={t('clock.answered')}>{t('clock.of', { ok: last.servers_ok, n: last.servers_tried })}</Row>
        <Row name={t('clock.when')}>{new Date(last.at_ms).toLocaleTimeString()}</Row>
      </Cells>
      {last.corrected ? <p className="live-note">{t('clock.corrected')}</p> : null}
    </>
  );
}

/** Scheda Orologio: client NTP tutto configurabile, misura e correzione dell'ora di Windows. */
export function ClockTab() {
  const t = useT();
  const client = useQueryClient();
  const clock = useQuery({ queryKey: ['clock'], queryFn: getClock, retry: false });
  const [draft, setDraft] = useState<Draft | null>(null);
  const [note, setNote] = useState('');

  const shown: Draft | null = draft ?? (clock.data ? toDraft(clock.data.config) : null);
  const parsed = shown ? draftToConfig(shown) : null;

  const save = useMutation({
    mutationFn: (config: ClockConfig) => putClock(config),
    onSuccess: () => {
      setDraft(null);
      setNote(t('clock.saved'));
      void client.invalidateQueries({ queryKey: ['clock'] });
    },
    onError: (e) => setNote(errorText(e, t)),
  });
  const measure = useMutation({
    mutationFn: () => checkClock(),
    onSuccess: () => {
      setNote('');
      void client.invalidateQueries({ queryKey: ['clock'] });
    },
    onError: (e) => setNote(errorText(e, t)),
  });
  const correct = useMutation({
    mutationFn: () => correctClock(),
    onSuccess: () => {
      setNote('');
      void client.invalidateQueries({ queryKey: ['clock'] });
    },
    onError: (e) => setNote(errorText(e, t)),
  });

  useAutosave(
    parsed,
    draft !== null,
    (config) => save.mutate(config),
    (config) => void putClock(config).catch(() => undefined),
  );

  if (clock.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!clock.data || !shown) return <p className="muted">{t('set.loading')}</p>;
  const set = (change: Partial<Draft>) => setDraft({ ...shown, ...change });

  return (
    <Page>
      <Group title={t('clock.state')}>
        <Status last={clock.data.last} warnMs={clock.data.config.warn_ms} />
        <Actions>
          <button type="button" className="glass" title={t('tip.clock.check')} disabled={measure.isPending} onClick={() => measure.mutate()}>
            {t('clock.check')}
          </button>
          <button type="button" className="glass" title={t('tip.clock.correct')} disabled={correct.isPending} onClick={() => correct.mutate()}>
            {t('clock.correct')}
          </button>
        </Actions>
      </Group>

      <Group title={t('lay.settings')} span={2}>
        <Cells min={17}>
          <Check label={t('clock.enabled')} hint={t('tip.clock.enabled')} checked={shown.enabled} onChange={(v) => set({ enabled: v })} />
          <Check label={t('clock.auto')} hint={t('tip.clock.auto')} checked={shown.auto} onChange={(v) => set({ auto: v })} />
          <Field label={t('clock.interval')} hint={t('tip.clock.interval')}>
            <input type="number" className="set-number" min={1} max={1440} value={shown.minutes} onChange={(e) => set({ minutes: e.target.value })} />
          </Field>
          <Field label={t('clock.warn')} hint={t('tip.clock.warn')}>
            <input type="number" className="set-number" min={10} max={5000} value={shown.warn} onChange={(e) => set({ warn: e.target.value })} />
          </Field>
          <Field label={t('clock.servers')} hint={t('tip.clock.servers')} stack wide>
            <textarea rows={4} value={shown.servers} spellCheck={false} onChange={(e) => set({ servers: e.target.value })} />
          </Field>
        </Cells>
        {!parsed ? <Message kind="alert">{t('clock.invalid')}</Message> : <Message kind="muted">{t('clock.autosave')}</Message>}
        {note ? <Message>{note}</Message> : null}
      </Group>

      <Notes>
        <Help>{t('clock.intro')}</Help>
      </Notes>
    </Page>
  );
}
