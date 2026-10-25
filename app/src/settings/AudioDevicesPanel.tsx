import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, getAudioDevices, putAudioSettings, type AudioDevice, type AudioDevices, type AudioPart, type AudioResult, type AudioValues } from '../api/client';
import { useT, type Key, type T } from '../i18n';
import { Help } from './common';
import { Actions, Field, Group, Message, Notes, Page } from './layout';

const IN_CHANNELS = ['Mono', 'Left', 'Right'] as const;
const OUT_CHANNELS = ['Mono', 'Left', 'Right', 'Both'] as const;
const SAVE_MODES = ['0', '1', '2'] as const;

const FIELDS: ReadonlyArray<keyof AudioValues> = ['input', 'output', 'input_channel', 'output_channel', 'save_dir', 'save_mode'];

/** Per l'uscita «predefinito» e vuoto sono la stessa scelta. */
function sameOutput(a: string, b: string, systemDefault: string): boolean {
  const norm = (v: string) => (v.trim() === '' || v.trim().toLowerCase() === systemDefault.toLowerCase() ? '' : v.trim());
  return norm(a) === norm(b);
}

function errorText(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'local_only') return t('aud.err.local');
    if (error.code === 'bad_audio_setting') return t('aud.err.bad');
    if (error.code === 'engine_settings_write') return t('aud.err.write');
    if (error.code === 'audio_unavailable') return t('aud.err.unavailable');
  }
  return t('aud.err.generic');
}

/** Quanti canali ha il dispositivo scelto («predefinito» e' quello predefinito del sistema; sconosciuto: si suppone stereo). */
function channelsOf(list: AudioDevice[], value: string, systemDefault: string): number {
  const v = value.trim();
  const found = v === '' || v.toLowerCase() === systemDefault.toLowerCase() ? list.find((d) => d.is_default) : list.find((d) => d.name.toLowerCase() === v.toLowerCase());
  return found?.channels ?? 2;
}

function deviceLabel(d: AudioDevice, inUse: string | null, t: T): string {
  const ch = d.channels === 1 ? t('aud.mono') : d.channels === 2 ? t('aud.stereo') : t('aud.chMany', { n: d.channels });
  return `${d.name} · ${ch}${inUse !== null && inUse.toLowerCase() === d.name.toLowerCase() ? ` · ${t('aud.inUse')}` : ''}`;
}

function DeviceSelect({
  id,
  label,
  tip,
  list,
  value,
  inUse,
  systemDefault,
  allowUnset,
  onChange,
}: {
  id: string;
  label: string;
  tip: string;
  list: AudioDevice[];
  value: string;
  inUse: string | null;
  systemDefault: string;
  allowUnset: boolean;
  onChange(next: string): void;
}) {
  const t = useT();
  const defaultName = list.find((d) => d.is_default)?.name;
  const isDefault = value.toLowerCase() === systemDefault.toLowerCase();
  const known = isDefault || value === '' || list.some((d) => d.name.toLowerCase() === value.toLowerCase());
  return (
    <Field label={label} hint={tip}>
      <select id={id} value={value} onChange={(e) => onChange(e.target.value)}>
        {allowUnset && value === '' ? <option value="">{t('aud.unset')}</option> : null}
        <option value={systemDefault}>{defaultName ? t('aud.default', { name: defaultName }) : t('aud.defaultPlain')}</option>
        {list.map((d) => (
          <option key={d.name} value={d.name}>
            {deviceLabel(d, inUse, t)}
          </option>
        ))}
        {!known ? <option value={value}>{t('aud.missing', { name: value })}</option> : null}
      </select>
    </Field>
  );
}

function ChannelSelect({ id, label, tip, value, options, stereo, onChange }: { id: string; label: string; tip: string; value: string; options: readonly string[]; stereo: boolean; onChange(next: string): void }) {
  const t = useT();
  return (
    <Field label={label} hint={tip}>
      <select id={id} value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => (
          <option key={o} value={o} disabled={o !== 'Mono' && !stereo}>
            {t(`aud.ch.${o.toLowerCase()}` as Key)}
          </option>
        ))}
      </select>
    </Field>
  );
}

/** Le righe dell'esito: cosa si e' applicato, cosa no e perche', e quale dispositivo resta in uso. */
export function resultLines(result: AudioResult, t: T): { text: string; bad: boolean }[] {
  const lines: { text: string; bad: boolean }[] = [];
  const parts: ReadonlyArray<[AudioPart, Key]> = [
    [result.input, 'aud.part.input'],
    [result.output, 'aud.part.output'],
    [result.save, 'aud.part.save'],
  ];
  for (const [part, key] of parts) {
    const label = t(key);
    const name = part.in_use ?? '';
    if (part.status === 'applied') lines.push({ text: name ? t('aud.res.applied', { part: label, name }) : t('aud.res.appliedPlain', { part: label }), bad: false });
    else if (part.status === 'deferred') lines.push({ text: t('aud.res.deferred', { part: label, name: name || '—' }), bad: false });
    else if (part.status === 'saved_only') lines.push({ text: t('aud.res.savedOnly', { part: label }), bad: false });
    else if (part.status === 'failed') {
      const tail = part.restored && name ? ` ${t('aud.res.restored', { name })}` : part.in_use === null && key !== 'aud.part.save' ? ` ${t('aud.res.lost')}` : '';
      lines.push({ text: `${t('aud.res.failed', { part: label, reason: part.message ?? '—' })}${tail}`, bad: true });
    }
  }
  if (lines.length === 0) lines.push({ text: t('aud.res.nothing'), bad: false });
  else if (result.ok && !lines.some((l) => l.bad)) lines.push({ text: t('aud.res.ok'), bad: false });
  return lines;
}

/** Scheda Audio con il motore proprio: ingresso e uscita a tendina, canali, file audio; l'esito dell'applicazione si legge subito sotto. */
export function AudioDevicesPanel() {
  const t = useT();
  const client = useQueryClient();
  const devices = useQuery({ queryKey: ['audio-devices'], queryFn: getAudioDevices, retry: false });
  const [draft, setDraft] = useState<Partial<AudioValues>>({});
  const [lines, setLines] = useState<{ text: string; bad: boolean }[]>([]);

  const apply = useMutation({
    mutationFn: (data: AudioDevices) => {
      const body: Parameters<typeof putAudioSettings>[0] = {};
      for (const key of FIELDS) {
        const next = draft[key];
        if (next === undefined || next === data.values[key]) continue;
        if (key === 'save_mode') body.save_mode = Number(next);
        else body[key] = next;
      }
      return putAudioSettings(body);
    },
    onSuccess: (result) => {
      setDraft({});
      setLines(resultLines(result, t));
      void client.invalidateQueries({ queryKey: ['audio-devices'] });
      void client.invalidateQueries({ queryKey: ['engine-settings'] });
      void client.invalidateQueries({ queryKey: ['input-level'] });
    },
    onError: (e) => setLines([{ text: errorText(e, t), bad: true }]),
  });

  if (devices.isError) return <p className="notice">{errorText(devices.error, t)}</p>;
  if (!devices.data) return <p className="muted">{t('set.loading')}</p>;
  const data = devices.data;
  const sys = data.system_default;
  const shown: AudioValues = { ...data.values, ...draft };
  const set = (patch: Partial<AudioValues>) => setDraft((previous) => ({ ...previous, ...patch }));
  const inStereo = channelsOf(data.inputs, shown.input, sys) >= 2;
  const outStereo = channelsOf(data.outputs, shown.output, sys) >= 2;
  const outValue = sameOutput(shown.output, '', sys) ? sys : shown.output;
  const pending = FIELDS.filter((k) => draft[k] !== undefined && draft[k] !== data.values[k] && !(k === 'output' && sameOutput(draft[k] ?? '', data.values[k], sys))).length;

  return (
    <Page className="aud-page">
      <Group title={t('aud.input')} span={1}>
        <DeviceSelect
          id="aud-input"
          label={t('aud.input')}
          tip={t('tip.aud.input')}
          list={data.inputs}
          value={shown.input}
          inUse={data.input_in_use}
          systemDefault={sys}
          allowUnset
          onChange={(next) => set({ input: next, ...(channelsOf(data.inputs, next, sys) < 2 && shown.input_channel !== 'Mono' ? { input_channel: 'Mono' } : {}) })}
        />
        <ChannelSelect id="aud-inch" label={t('aud.inch')} tip={t('tip.aud.inch')} value={shown.input_channel} options={IN_CHANNELS} stereo={inStereo} onChange={(next) => set({ input_channel: next })} />
        <Message kind={data.input_in_use ? 'note' : 'alert'}>{data.input_in_use ? t('aud.nowIn', { name: data.input_in_use }) : t('aud.nowInNone')}</Message>
      </Group>

      <Group title={t('aud.output')} span={1}>
        <DeviceSelect
          id="aud-output"
          label={t('aud.output')}
          tip={t('tip.aud.output')}
          list={data.outputs}
          value={outValue}
          inUse={data.output_in_use}
          systemDefault={sys}
          allowUnset={false}
          onChange={(next) => set({ output: next, ...(channelsOf(data.outputs, next, sys) < 2 && shown.output_channel !== 'Mono' ? { output_channel: 'Mono' } : {}) })}
        />
        <ChannelSelect id="aud-outch" label={t('aud.outch')} tip={t('tip.aud.outch')} value={shown.output_channel} options={OUT_CHANNELS} stereo={outStereo} onChange={(next) => set({ output_channel: next })} />
        <Message kind={data.output_in_use ? 'note' : 'muted'}>
          {data.output_in_use ? t('aud.nowOut', { name: data.output_in_use }) : t('aud.nowOutNone')}
          {data.output_pending ? ` ${t('aud.nextOut', { name: data.output_pending })}` : ''}
        </Message>
      </Group>

      <Group title={t('aud.mode')} span={1}>
        <Field label={t('aud.mode')} hint={t('tip.aud.mode')}>
          <select id="aud-mode" value={shown.save_mode} onChange={(e) => set({ save_mode: e.target.value })}>
            {SAVE_MODES.map((m) => (
              <option key={m} value={m}>
                {t(`aud.save.${m}` as Key)}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t('aud.dir')} hint={t('tip.aud.dir')} stack>
          <input id="aud-dir" type="text" className="jx-text" maxLength={260} spellCheck={false} autoComplete="off" value={shown.save_dir} placeholder={t('aud.dirPlaceholder', { dir: data.default_save_dir })} onChange={(e) => set({ save_dir: e.target.value })} />
        </Field>
      </Group>

      <Group title={t('aud.apply')} span={3}>
        <Actions>
          <button type="button" className="glass glass--on" disabled={apply.isPending || pending === 0} onClick={() => apply.mutate(data)}>
            {t('aud.apply')}
          </button>
          <button type="button" className="glass" disabled={apply.isPending || pending === 0} onClick={() => { setDraft({}); setLines([]); }}>
            {t('aud.discard')}
          </button>
          <button type="button" className="glass" title={t('tip.aud.refresh')} disabled={devices.isFetching} onClick={() => void devices.refetch()}>
            {t('aud.refresh')}
          </button>
          {pending > 0 ? <strong>{t('aud.pending', { n: pending })}</strong> : null}
        </Actions>
        {lines.length > 0 ? (
          <div role="status" aria-live="polite">
            {lines.map((line) => (
              <Message key={line.text} kind={line.bad ? 'alert' : 'note'}>
                {line.text}
              </Message>
            ))}
          </div>
        ) : null}
      </Group>

      <Notes span={3}>
        <Help>{t('aud.intro')}</Help>
      </Notes>
    </Page>
  );
}
