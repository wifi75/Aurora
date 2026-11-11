import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getSstv, putSstv, type SstvSettings, type SstvSnapshot } from '../api/client';
import { useT } from '../i18n';
import '../styles/sstv.css';

/** Le impostazioni SSTV: nella scheda SSTV delle Impostazioni e, ripiegate, in fondo alla pagina SSTV. */
export function SstvSettingsForm({ snap }: { snap: SstvSnapshot }) {
  const t = useT();
  const client = useQueryClient();
  const [draft, setDraft] = useState<SstvSettings>(snap.settings);
  const [done, setDone] = useState<'saved' | 'error' | null>(null);
  useEffect(() => setDraft(snap.settings), [snap.settings]);
  const save = useMutation({
    mutationFn: () => putSstv(draft),
    onSuccess: (s) => {
      client.setQueryData(['sstv'], s);
      setDone('saved');
    },
    onError: () => setDone('error'),
  });
  const set = <K extends keyof SstvSettings>(k: K, v: SstvSettings[K]) => {
    setDone(null);
    setDraft((d) => ({ ...d, [k]: v }));
  };
  const num = (v: string, lim: number) => Math.max(-lim, Math.min(lim, Number(v) || 0));

  return (
    <form
      className="sstv-settings"
      onSubmit={(e) => {
        e.preventDefault();
        save.mutate();
      }}
    >
      <label className="sstv-field">
        <span>{t('sstv.set.defaultMode')}</span>
        <select value={draft.default_mode} onChange={(e) => set('default_mode', e.target.value)}>
          {snap.modes.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
            </option>
          ))}
        </select>
      </label>
      <label className="sstv-field">
        <span>{t('sstv.rx.lock')}</span>
        <select value={draft.lock_mode ?? ''} onChange={(e) => set('lock_mode', e.target.value === '' ? null : e.target.value)}>
          <option value="">{t('sstv.rx.lockNone')}</option>
          {snap.modes.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
            </option>
          ))}
        </select>
      </label>
      <label className="sstv-check">
        <input type="checkbox" checked={draft.auto_receive} onChange={(e) => set('auto_receive', e.target.checked)} />
        {t('sstv.set.autoReceive')}
      </label>
      <label className="sstv-check">
        <input type="checkbox" checked={draft.auto_save} onChange={(e) => set('auto_save', e.target.checked)} />
        {t('sstv.set.autoSave')}
      </label>
      <label className="sstv-check">
        <input type="checkbox" checked={draft.save_tx} onChange={(e) => set('save_tx', e.target.checked)} />
        {t('sstv.set.saveTx')}
      </label>
      <label className="sstv-field sstv-field--wide">
        <span>{t('sstv.set.folder')}</span>
        <input value={draft.folder} placeholder={snap.folder} onChange={(e) => set('folder', e.target.value)} />
        <small className="sstv-hint">{t('sstv.set.folderHint')}</small>
      </label>
      <label className="sstv-field">
        <span>{t('sstv.set.rxOffset')}</span>
        <input type="number" min={-200} max={200} step={5} value={draft.rx_offset_hz} onChange={(e) => set('rx_offset_hz', num(e.target.value, 200))} />
      </label>
      <label className="sstv-field">
        <span>{t('sstv.set.txOffset')}</span>
        <input type="number" min={-200} max={200} step={5} value={draft.tx_offset_hz} onChange={(e) => set('tx_offset_hz', num(e.target.value, 200))} />
      </label>
      <label className="sstv-field">
        <span>{t('sstv.set.slant')}</span>
        <input type="number" min={-5000} max={5000} step={10} value={draft.slant_ppm} onChange={(e) => set('slant_ppm', num(e.target.value, 5000))} />
        <small className="sstv-hint">{t('sstv.set.slantHint')}</small>
      </label>
      <div className="sstv-actions">
        <button type="submit" className="glass sstv-btn sstv-btn--primary" disabled={save.isPending}>
          {t('sstv.set.save')}
        </button>
        {done === 'saved' ? <span className="sstv-ok">{t('sstv.set.saved')}</span> : null}
        {done === 'error' ? <span className="sstv-bad">{t('sstv.set.error')}</span> : null}
      </div>
      <p className="sstv-hint">{t('sstv.credits')}</p>
    </form>
  );
}

export function SstvTab() {
  const t = useT();
  const snap = useQuery({ queryKey: ['sstv'], queryFn: getSstv, retry: false });
  if (snap.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!snap.data) return null;
  return (
    <div className="sstv-tab">
      <h2>{t('sstv.set.title')}</h2>
      <SstvSettingsForm snap={snap.data} />
    </div>
  );
}
