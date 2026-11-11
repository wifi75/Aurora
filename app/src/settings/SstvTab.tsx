import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useId, useState } from 'react';
import { getSstv, getSstvImages, putSstv, type SstvSettings, type SstvSnapshot } from '../api/client';
import { useT } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { Check } from './common';
import { Actions, Cells, Field, Group, Message, Page } from './layout';
import { useSharedDraft } from './session';
import { useAutosave } from './useAutosave';

/** Le tre schede SSTV delle Impostazioni: ricezione, trasmissione, immagini e cartelle. */
export type SstvPart = 'rx' | 'tx' | 'images';

/** La chiave della bozza SSTV nella bozza unica della finestra: parte con «Salva» in basso, come le altre schede. */
export const SSTV_DRAFT = 'sstv';

/** Le correzioni numeriche restano testo finche' si scrive (cosi' si puo' battere «-»); il resto e' come nel core. */
export interface SstvDraft extends Omit<SstvSettings, 'rx_offset_hz' | 'tx_offset_hz' | 'slant_ppm'> {
  rx_offset_hz: string;
  tx_offset_hz: string;
  slant_ppm: string;
}

function toDraft(s: SstvSettings): SstvDraft {
  return { ...s, tx_folder: s.tx_folder ?? '', rx_offset_hz: String(s.rx_offset_hz), tx_offset_hz: String(s.tx_offset_hz), slant_ppm: String(s.slant_ppm) };
}

/** Un intero fra -lim e +lim, o `null`. */
function whole(v: string, lim: number): number | null {
  const n = Number(v.trim());
  return v.trim() !== '' && Number.isInteger(n) && Math.abs(n) <= lim ? n : null;
}

/** La bozza in impostazioni valide per il core, o `null` se un numero non va. */
export function sstvDraftToSettings(d: SstvDraft): SstvSettings | null {
  const rx = whole(d.rx_offset_hz, 200);
  const tx = whole(d.tx_offset_hz, 200);
  const slant = whole(d.slant_ppm, 5000);
  if (rx === null || tx === null || slant === null) return null;
  return { ...d, rx_offset_hz: rx, tx_offset_hz: tx, slant_ppm: slant };
}

/**
 * Una scheda SSTV. Nella finestra delle Impostazioni le modifiche vanno nella bozza unica e partono con «Salva» in basso (le tre schede
 * SSTV condividono la stessa bozza); fuori dalla finestra (nessuna bozza unica) si salvano da sole poco dopo la modifica.
 */
function SstvForm({ snap, part }: { snap: SstvSnapshot; part: SstvPart }) {
  const t = useT();
  const client = useQueryClient();
  const shared = useSharedDraft();
  const [local, setLocal] = useState<SstvDraft | null>(null);
  const [note, setNote] = useState('');
  const ids = useId();
  const original = toDraft(snap.settings);
  const draft = shared ? ((shared.drafts[SSTV_DRAFT] as SstvDraft | undefined) ?? null) : local;
  const shown = draft ?? original;
  const parsed = sstvDraftToSettings(shown);
  const smooth = usePrefs((s) => s.prefs.sstvSmooth);
  // calibrazione della scheda audio: l'ultima immagine ricevuta bene (da una stazione fidata, per esempio un'immagine di prova o un'emissione di riferimento) dice di quanto va l'orologio
  const images = useQuery({ queryKey: ['sstv-images'], queryFn: getSstvImages, staleTime: 10_000, enabled: part === 'rx' });
  const reference = (images.data ?? []).find((i) => i.direction !== 'tx' && i.complete && (i.sync_quality ?? 0) >= 0.9 && i.clock_ppm !== undefined);

  const save = useMutation({
    mutationFn: (s: SstvSettings) => putSstv(s),
    onSuccess: (s) => {
      client.setQueryData(['sstv'], s);
      setLocal(null);
      setNote(t('sstv.set.saved'));
    },
    onError: () => setNote(t('sstv.set.error')),
  });
  useAutosave(
    parsed,
    shared === null && draft !== null,
    (s) => save.mutate(s),
    (s) => void putSstv(s).then((next) => client.setQueryData(['sstv'], next), () => undefined),
  );

  const set = (change: Partial<SstvDraft>) => {
    setNote('');
    const next = { ...shown, ...change };
    if (shared) shared.setDraftOf(SSTV_DRAFT, JSON.stringify(next) === JSON.stringify(original) ? undefined : next);
    else setLocal(next);
  };
  const modeOptions = snap.modes.map((m) => (
    <option key={m.id} value={m.id}>
      {m.name}
    </option>
  ));
  // come nelle schede del motore: la voce cambiata e non ancora salvata ha il segno giallo
  const ch = (k: keyof SstvDraft) => draft !== null && draft[k] !== original[k];
  const status = (
    <>
      {!parsed ? <Message kind="alert">{t('sstv.set.invalid')}</Message> : null}
      {note ? <Message>{note}</Message> : null}
    </>
  );

  if (part === 'tx') {
    return (
      <Page>
        <Group title={t('sstv.set.grpTx')} span={2}>
          <Cells min={18}>
            <Field label={t('sstv.set.defaultMode')} hint={t('sstv.set.defaultModeHint')} changed={ch('default_mode')}>
              <select value={shown.default_mode} onChange={(e) => set({ default_mode: e.target.value })}>
                {modeOptions}
              </select>
            </Field>
            <Field label={t('sstv.set.txOffset')} hint={t('sstv.set.txOffsetHint')} changed={ch('tx_offset_hz')}>
              <input type="number" min={-200} max={200} step={5} value={shown.tx_offset_hz} onChange={(e) => set({ tx_offset_hz: e.target.value })} />
            </Field>
          </Cells>
          {status}
        </Group>
      </Page>
    );
  }

  if (part === 'images') {
    return (
      <Page>
        <Group title={t('sstv.set.grpSave')} span={2}>
          <Cells min={14}>
            <Check label={t('sstv.set.autoSave')} hint={t('sstv.set.autoSaveHint')} checked={shown.auto_save} onChange={(v) => set({ auto_save: v })} />
            <Check label={t('sstv.set.saveTx')} hint={t('sstv.set.saveTxHint')} checked={shown.save_tx} onChange={(v) => set({ save_tx: v })} />
          </Cells>
        </Group>
        <Group title={t('sstv.set.grpFolders')} span={2}>
          <Cells min={18}>
            {/* la spiegazione sta sotto la cella, fuori dall'etichetta: il nome del campo (e la ricerca) resta corto */}
            <div className="sx-with-below">
              <Field label={t('sstv.set.folder')} stack wide changed={ch('folder')}>
                <input type="text" value={shown.folder} placeholder={snap.folder} spellCheck={false} aria-describedby={`${ids}-folder`} onChange={(e) => set({ folder: e.target.value })} />
              </Field>
              <small className="sx-below" id={`${ids}-folder`}>
                {t('sstv.set.folderHint')}
              </small>
            </div>
            <div className="sx-with-below">
              <Field label={t('sstv.set.txFolder')} stack wide changed={ch('tx_folder')}>
                <input type="text" value={shown.tx_folder} placeholder="C:\Foto\SSTV" spellCheck={false} aria-describedby={`${ids}-txfolder`} onChange={(e) => set({ tx_folder: e.target.value })} />
              </Field>
              <small className="sx-below" id={`${ids}-txfolder`}>
                {t('sstv.set.txFolderHint')}
              </small>
            </div>
          </Cells>
          {status}
        </Group>
      </Page>
    );
  }

  return (
    <Page>
      <Group title={t('sstv.set.grpRx')} span={2}>
        <Cells min={18}>
          <Field label={t('sstv.rx.lock')} hint={t('sstv.set.lockHint')} changed={ch('lock_mode')}>
            <select value={shown.lock_mode ?? ''} onChange={(e) => set({ lock_mode: e.target.value === '' ? null : e.target.value })}>
              <option value="">{t('sstv.rx.lockNone')}</option>
              {modeOptions}
            </select>
          </Field>
          <Field label={t('sstv.set.rxOffset')} hint={t('sstv.set.rxOffsetHint')} changed={ch('rx_offset_hz')}>
            <input type="number" min={-200} max={200} step={5} value={shown.rx_offset_hz} onChange={(e) => set({ rx_offset_hz: e.target.value })} />
          </Field>
          <Check label={t('sstv.set.autoReceive')} hint={t('sstv.set.autoReceiveHint')} checked={shown.auto_receive} onChange={(v) => set({ auto_receive: v })} />
        </Cells>
        {status}
      </Group>
      <Group title={t('sstv.set.grpClock')}>
        <Cells min={14}>
          <Field label={t('sstv.set.slant')} hint={t('sstv.set.slantHint')} changed={ch('slant_ppm')}>
            <input type="number" min={-5000} max={5000} step={10} value={shown.slant_ppm} onChange={(e) => set({ slant_ppm: e.target.value })} />
          </Field>
        </Cells>
        <Actions>
          <button
            type="button"
            className="glass"
            disabled={!reference}
            title={t('sstv.cal.hint')}
            onClick={() => reference?.clock_ppm !== undefined && set({ slant_ppm: String(Math.max(-5000, Math.min(5000, Math.round(reference.clock_ppm)))) })}
          >
            {t('sstv.cal.button')}
          </button>
          <small className="sx-below">{reference?.clock_ppm !== undefined ? t('sstv.cal.found', { ppm: Math.round(reference.clock_ppm) }) : t('sstv.cal.none')}</small>
        </Actions>
        <small className="sx-below">{t('sstv.cal.hint')}</small>
      </Group>
      <Group title={t('sstv.set.grpView')}>
        <Cells min={14}>
          {/* vale subito, anche nella pagina SSTV aperta nella finestra principale */}
          <Check label={t('sstv.rx.smooth')} hint={t('sstv.set.smoothHint')} checked={smooth} onChange={(v) => usePrefs.getState().set({ sstvSmooth: v })} />
        </Cells>
      </Group>
      {/* i riconoscimenti in fondo, discreti */}
      <p className="sx-credits">{t('sstv.credits')}</p>
    </Page>
  );
}

/** La scheda SSTV delle Impostazioni (`part`: quale delle tre). */
export function SstvTab({ part = 'rx' }: { part?: SstvPart }) {
  const t = useT();
  const snap = useQuery({ queryKey: ['sstv'], queryFn: getSstv, retry: false });
  if (snap.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!snap.data) return <p className="muted">{t('set.loading')}</p>;
  return <SstvForm snap={snap.data} part={part} />;
}
