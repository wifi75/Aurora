import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, connectCat, getAudioDevices, getCat, getQmap, putAudioSettings, putCat, type CatBackend, type CatConfig } from '../api/client';
import { askConfirm } from '../components/ConfirmHost';
import { useT, type Key, type T } from '../i18n';
import { catIssue } from '../lib/cat';
import { coreText } from '../lib/coreText';
import { usePrefs } from '../lib/prefs';
import { TCI_DEFAULT_ADDRESS, TCI_INPUT, TCI_OUTPUT, openInMain, tciEverywhere } from '../lib/sdr';
import { resultLines } from './AudioDevicesPanel';
import { Check } from './common';
import { Help as Tip } from './layout';
import { useAutosave } from './useAutosave';

const BACKEND_KEY: Record<CatBackend, Key> = {
  hamlib: 'cat.backend.hamlib',
  omnirig: 'cat.backend.omnirig',
  tci: 'cat.backend.tci',
  ft991link: 'cat.backend.ft991link',
  none: 'cat.backend.none',
};

const QMAP_SOURCE_KEY = { off: 'qmap.source.off', tci: 'qmap.source.tci', file: 'qmap.source.file' } as const satisfies Record<string, Key>;

function failText(e: unknown, t: T): string {
  const reason = e instanceof ApiError ? (e.detail ? coreText(e.detail, t) : e.code) : e instanceof Error ? e.message : String(e);
  return t('sdr.failed', { reason });
}

/**
 * La scheda SDR: raccoglie in un posto solo cio' che serve a chi ha una SDR, senza impostazioni proprie. L'indirizzo TCI e' quello del
 * CAT (scheda Radio), l'audio TCI quello della scheda Audio, l'interruttore di QMAP la stessa preferenza che c'era in Generali.
 */
export function SdrTab() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 1500 });
  const devices = useQuery({ queryKey: ['audio-devices'], queryFn: getAudioDevices, retry: false });
  const qmap = useQuery({ queryKey: ['qmap'], queryFn: getQmap, retry: false, staleTime: 30_000 });
  const showQmap = usePrefs((s) => s.prefs.showQmap);
  /** L'indirizzo che si sta scrivendo (`null`: quello salvato). */
  const [address, setAddress] = useState<string | null>(null);
  const [lines, setLines] = useState<{ text: string; bad: boolean }[]>([]);
  const [note, setNote] = useState('');

  const config: CatConfig | undefined = snap.data?.config;
  const saved = config?.tci_address ?? '';
  const shown = address ?? saved;
  const dirty = address !== null && address !== saved;
  const bad = config ? catIssue({ ...config, tci_address: shown }) === 'bad_tci_address' : false;

  const refresh = () => {
    void client.invalidateQueries({ queryKey: ['cat'] });
    void client.invalidateQueries({ queryKey: ['audio-devices'] });
    void client.invalidateQueries({ queryKey: ['engine-settings'] });
    void client.invalidateQueries({ queryKey: ['input-level'] });
  };

  // l'indirizzo si salva da solo come nella scheda Radio: dopo una pausa e uscendo dalla scheda
  const saveAddress = useMutation({
    mutationFn: (c: CatConfig) => putCat(c),
    onSuccess: () => {
      setAddress(null);
      setLines([]);
      refresh();
    },
    onError: (e) => setLines([{ text: failText(e, t), bad: true }]),
  });
  useAutosave(dirty && !bad && config ? { ...config, tci_address: shown } : null, dirty, (v) => saveAddress.mutate(v), (v) => void putCat(v).catch(() => undefined), 1500);

  // «Usa il TCI per tutto»: CAT su TCI con l'indirizzo scritto, poi l'audio (il motore offre l'audio TCI solo con il CAT gia' su TCI), poi si collega
  const useAll = useMutation({
    mutationFn: async () => {
      if (!config) throw new Error('cat_not_loaded');
      await putCat({ ...config, backend: 'tci', tci_address: shown.trim() });
      const result = await putAudioSettings({ input: TCI_INPUT, output: TCI_OUTPUT });
      await connectCat().catch(() => undefined);
      return result;
    },
    onSuccess: (result) => {
      setAddress(null);
      setLines([{ text: t('sdr.applied'), bad: false }, ...resultLines(result, t)]);
      refresh();
    },
    onError: (e) => {
      setLines([{ text: failText(e, t), bad: true }]);
      refresh();
    },
  });
  const askUseAll = () => {
    void askConfirm(t('sdr.useAllAsk', { address: shown.trim() || TCI_DEFAULT_ADDRESS })).then((yes) => {
      if (yes) useAll.mutate();
    });
  };
  const open = (hash: string) => {
    openInMain(hash);
    if (location.hash.startsWith('#/finestra/')) setNote(t('sdr.opened'));
  };

  if (snap.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!snap.data || !config) return <p className="muted">{t('set.loading')}</p>;
  const status = snap.data.status;
  const tci = config.backend === 'tci';
  const rig = status.rig;
  const audio = devices.data;
  const done = audio ? tciEverywhere(config, audio.values) : false;
  const qsrc = qmap.data?.config;

  return (
    <div className="sw-page sdr-page">
      <section className="sw-box" aria-label={t('sdr.tci')}>
        <h3>{t('sdr.tci')}</h3>
        <div className="sw-grid">
          <label className="sw-field" title={t('tip.sdr.address')}>
            <span className="sw-label">
              {t('sdr.address')}
              <Tip tip={t('tip.sdr.address')} />
            </span>
            <input
              type="text"
              value={shown}
              spellCheck={false}
              autoComplete="off"
              placeholder={TCI_DEFAULT_ADDRESS}
              aria-invalid={bad}
              onChange={(e) => setAddress(e.target.value.trim())}
            />
          </label>
          <div className="sw-field">
            <span className="sw-label">{t('cat.backend')}</span>
            <span>
              {tci ? (
                <>
                  <span className={`led ${status.state === 'connected' ? 'led--ok' : status.state === 'error' ? 'led--fail' : status.state === 'disconnected' ? '' : 'led--testing'}`} aria-hidden="true" />{' '}
                  {t(`cat.state.${status.state}` as Key)}
                  {status.state === 'connected' && rig?.frequency_hz ? ` · ${(rig.frequency_hz / 1000).toFixed(3)} kHz` : ''}
                </>
              ) : (
                t('sdr.catNow', { name: t(BACKEND_KEY[config.backend]) })
              )}
            </span>
          </div>
          <div className="sw-field">
            <span className="sw-label">{t('sw.tab.audio')}</span>
            <span>{audio ? t('sdr.audioNow', { input: audio.input_in_use ?? (audio.values.input || '—'), output: audio.output_in_use ?? (audio.values.output || '—') }) : '—'}</span>
          </div>
        </div>
        {bad ? <p className="sw-note" role="alert">{t('sdr.badAddress')}</p> : null}
        <div className="sw-row sdr-actions">
          {done ? (
            <span className="sw-note">{t('sdr.allDone')}</span>
          ) : (
            <button type="button" className="glass chip" title={t('tip.sdr.useAll')} disabled={bad || useAll.isPending} onClick={askUseAll}>
              {t('sdr.useAll')}
            </button>
          )}
        </div>
        {lines.length > 0 ? (
          <ul className="sdr-result" role="status">
            {lines.map((l, i) => (
              <li key={i} className={l.bad ? 'sdr-result--bad' : undefined}>
                {l.text}
              </li>
            ))}
          </ul>
        ) : null}
      </section>

      <section className="sw-box" aria-label="QMAP">
        <h3>QMAP</h3>
        <div className="sw-grid">
          <Check label={t('pref.showQmap')} hint={t('tip.pref.showQmap')} checked={showQmap} onChange={(v) => usePrefs.getState().set({ showQmap: v })} />
          <span className="sw-note">
            {qsrc ? t('sdr.qmapSource', { source: `${t(QMAP_SOURCE_KEY[qsrc.source])}${qsrc.source === 'tci' ? ` (${qsrc.tci_address || TCI_DEFAULT_ADDRESS})` : ''}` }) : ''}
          </span>
          <div className="sw-row">
            <button type="button" className="glass chip" onClick={() => open('#/qmap')}>
              {t('sdr.openQmap')}
            </button>
          </div>
        </div>
      </section>

      <section className="sw-box" aria-label={t('sdr.radios')}>
        <h3>{t('sdr.radios')}</h3>
        <p className="sw-note sdr-text">{t('sdr.radiosText')}</p>
        <div className="sw-row sdr-actions">
          <button type="button" className="glass chip" onClick={() => open('#/guida/tci')}>
            {t('sdr.guideTci')}
          </button>
          <button type="button" className="glass chip" onClick={() => open('#/guida/qmap')}>
            {t('sdr.guideQmap')}
          </button>
          {note ? <span className="sw-note">{note}</span> : null}
        </div>
      </section>
    </div>
  );
}
