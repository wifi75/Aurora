import { useEffect, useRef, useState } from 'react';
import { ApiError, openWefaxFolder, postWefaxFile, putWefax, stopWefaxFile, WEFAX_FILE_MAX_BYTES, wefaxRx, type WefaxSettings, type WefaxSnapshot } from '../../api/client';
import { useT } from '../../i18n';
import '../../styles/wefax-audio.css';

/** MB del WAV di una cartina: 12000 campioni al secondo a 16 bit (1,44 MB al minuto) per le righe massime, piu' toni e pre-roll (circa 30 s). */
function wefaxAudioMb(maxRows: number, lpm: number): number {
  const minutes = maxRows / Math.max(1, lpm) + 0.5;
  return Math.max(1, Math.round(minutes * 60 * 12000 * 2 / 1e6));
}

/** Registrazione dell'audio delle cartine e cartelle: i pulsanti aprono Esplora file solo sul PC dove gira Aurora, altrove si mostra il percorso. */
function Recording({ snap, lpm, save }: { snap: WefaxSnapshot; lpm: number; save: (patch: Partial<WefaxSettings>) => void }) {
  const t = useT();
  const { settings } = snap;
  const [error, setError] = useState<string | null>(null);
  const local = snap.local === true;
  const audioFolder = snap.audio_folder ?? '';
  const open = (which: 'images' | 'audio') => {
    setError(null);
    openWefaxFolder(which).catch((e: unknown) => setError(e instanceof ApiError && e.code === 'local_only' ? t('wefax.folder.remote') : t('wefax.folder.failed')));
  };
  return (
    <div className="wefax-rec">
      <label className="sstv-check" title={t('wefax.rec.tip')}>
        <input type="checkbox" checked={settings.record_audio ?? true} onChange={(e) => save({ record_audio: e.target.checked })} />
        {t('wefax.rec.on')}
        {snap.rx.recording ? <span className="wefax-rec-dot" role="status" aria-label={t('wefax.rec.now')} title={t('wefax.rec.now')} /> : null}
      </label>
      <NumberField label={t('wefax.rec.keep')} value={settings.audio_keep_days ?? 7} min={0} max={3650} onCommit={(v) => save({ audio_keep_days: Math.round(v) })} />
      <span className="sstv-hint">{t('wefax.rec.size', { mb: String(wefaxAudioMb(settings.max_rows, lpm)) })}</span>
      <span className="wefax-rec-folders">
        <button type="button" className="glass sstv-btn" disabled={!local} onClick={() => open('images')}>
          {t('wefax.folder.images')}
        </button>
        <button type="button" className="glass sstv-btn" disabled={!local || !audioFolder} onClick={() => open('audio')}>
          {t('wefax.folder.audio')}
        </button>
      </span>
      {!local ? (
        <span className="wefax-rec-remote">
          <span className="sstv-hint">{t('wefax.folder.remote')}</span>
          <input readOnly className="wefax-rec-path" value={snap.folder} aria-label={t('wefax.folder.images')} onFocus={(e) => e.target.select()} />
          {audioFolder ? <input readOnly className="wefax-rec-path" value={audioFolder} aria-label={t('wefax.folder.audio')} onFocus={(e) => e.target.select()} /> : null}
        </span>
      ) : null}
      {error ? (
        <span className="wefax-file-error" role="alert">
          {error}
        </span>
      ) : null}
    </div>
  );
}

const SLANT_STEPS = [-10, -1, 1, 10] as const;

/** Un campo numerico che si conferma uscendo dal campo o con Invio (non a ogni tasto: ogni conferma e' una scrittura nel motore). */
function NumberField({ label, value, min, max, onCommit }: { label: string; value: number; min: number; max: number; onCommit: (v: number) => void }) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const commit = () => {
    const v = Number(draft);
    if (!Number.isFinite(v)) {
      setDraft(String(value));
      return;
    }
    const clamped = Math.min(max, Math.max(min, v));
    setDraft(String(clamped));
    if (clamped !== value) onCommit(clamped);
  };
  return (
    <label className="sstv-field">
      {label}
      <input type="number" inputMode="numeric" value={draft} min={min} max={max} onChange={(e) => setDraft(e.target.value)} onBlur={commit} onKeyDown={(e) => e.key === 'Enter' && commit()} />
    </label>
  );
}

/** Indicatori e comandi della ricezione WEFAX: ascolto, partenza a mano, chiusura, impostazioni del decodificatore. */
export function Controls({ snap, onChanged, onListen }: { snap: WefaxSnapshot; onChanged: () => void; onListen?: (on: boolean) => void }) {
  const t = useT();
  const { rx, settings } = snap;
  // le impostazioni che si cambiano a raffica (correzione dell'inclinazione) si sommano su quelle appena inviate, non su quelle lette un secondo fa
  const latest = useRef<WefaxSettings>(settings);
  latest.current = settings;
  const save = (patch: Partial<WefaxSettings>) => {
    latest.current = { ...latest.current, ...patch };
    putWefax(latest.current).then(onChanged, onChanged);
  };
  const act = (fn: () => Promise<void>) => () => {
    fn().then(onChanged, onChanged);
  };
  // decodifica di una registrazione: selettore di file, esito dell'invio e avanzamento (dallo stato)
  const fileInput = useRef<HTMLInputElement>(null);
  const [fileError, setFileError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const pickFile = (file: File | undefined) => {
    if (!file) return;
    setFileError(null);
    if (file.size > WEFAX_FILE_MAX_BYTES) {
      setFileError(t('wefax.file.tooBig'));
      return;
    }
    setSending(true);
    postWefaxFile(file)
      .catch((e: unknown) => {
        if (e instanceof ApiError && e.code === 'busy') setFileError(t('wefax.file.busy'));
        else if (e instanceof ApiError && e.status === 413) setFileError(t('wefax.file.tooBig'));
        else if (e instanceof ApiError && e.code === 'bad_audio') setFileError(t('wefax.file.bad'));
        else setFileError(t('wefax.file.failed'));
      })
      .finally(() => {
        setSending(false);
        onChanged();
      });
  };
  const decodingFile = rx.decoding_file === true;
  const filePct = Math.round(Math.max(0, Math.min(100, rx.file_percent ?? 0)));

  const phase = rx.receiving ? t(`wefax.phase.${rx.state === 'apt' || rx.state === 'phasing' ? rx.state : 'image'}`) : t('wefax.phase.idle');
  const syncPct = Math.round(Math.max(0, Math.min(1, rx.sync || 0)) * 100);
  const level01 = Math.max(0, Math.min(100, ((rx.level_db + 60) / 60) * 100));
  const lpmKey = settings.ioc === 288 ? 'lpm_288' : 'lpm_576';
  const lpm = settings[lpmKey];
  const lpmChoices = snap.lpm_values.includes(lpm) ? snap.lpm_values : [...snap.lpm_values, lpm];
  const active = snap.rx.receiving && !decodingFile;

  return (
    <aside className="sstv-side wefax-side">
      <dl className="sstv-readouts">
        <div>
          <dt>{t('wefax.ro.state')}</dt>
          <dd className="sstv-big">{phase}</dd>
        </div>
        <div>
          <dt>{t('wefax.ro.rows')}</dt>
          <dd>
            <b>{rx.receiving || rx.rows > 0 ? `${rx.rows}` : '—'}</b>
            {rx.width > 0 ? <span className="sstv-hint">{`${t('wefax.ro.size')}: ${rx.width} px`}</span> : null}
          </dd>
        </div>
        <div>
          <dt>{t('wefax.ro.ioc')}</dt>
          <dd>
            <b>{rx.ioc || settings.ioc}</b>
            <span className="sstv-hint">{`${t('wefax.ro.lpm')}: ${Math.round(rx.lpm || lpm)}`}</span>
          </dd>
        </div>
        <div>
          <dt>{t('wefax.ro.sync')}</dt>
          <dd>
            <span className={`sstv-meter sstv-meter--${syncPct > 60 ? 'ok' : syncPct > 25 ? 'warn' : 'bad'}`}>
              <span style={{ width: `${syncPct}%` }} />
            </span>
            <b>{rx.receiving ? `${syncPct} %` : '—'}</b>
          </dd>
        </div>
        <div>
          <dt>{t('wefax.ro.level')}</dt>
          <dd>
            <span className="sstv-meter">
              <span style={{ width: `${level01}%` }} />
            </span>
            <b>{rx.listening ? `${Math.round(rx.level_db)} dB` : '—'}</b>
          </dd>
        </div>
      </dl>
      {(rx.dropped_blocks ?? 0) > 0 ? (
        <p className="wefax-dropped" role="status">
          <b>{t('wefax.dropped', { n: String(rx.dropped_blocks) })}</b>
          <span>{t('wefax.droppedTip')}</span>
        </p>
      ) : null}

      <div className="sstv-actions">
        {rx.listening ? (
          <button
            type="button"
            className="glass sstv-btn"
            onClick={act(() => {
              onListen?.(false);
              return wefaxRx('stop');
            })}
          >
            {t('wefax.rx.stop')}
          </button>
        ) : (
          <button
            type="button"
            className="glass sstv-btn sstv-btn--primary"
            onClick={act(() => {
              onListen?.(true);
              return wefaxRx('start');
            })}
          >
            {t('wefax.rx.start')}
          </button>
        )}
        <button
          type="button"
          className="glass sstv-btn"
          disabled={!rx.listening || active || decodingFile}
          title={t('wefax.rx.forceHint')}
          onClick={act(() => {
            onListen?.(true);
            return wefaxRx('force');
          })}
        >
          {t('wefax.rx.force')}
        </button>
        <button type="button" className="glass sstv-btn" disabled={!active} onClick={act(() => wefaxRx('finish'))}>
          {t('wefax.rx.finish')}
        </button>
        <button type="button" className="glass sstv-btn sstv-btn--danger" disabled={!active} onClick={act(() => wefaxRx('cancel'))}>
          {t('wefax.rx.cancel')}
        </button>
      </div>
      <p className="sstv-hint">{t('wefax.rx.forceHint')}</p>

      {/* una registrazione WAV si decodifica come se arrivasse dalla scheda; durante la decodifica l'ascolto dal vivo e' sospeso */}
      <div className="wefax-file">
        <input
          ref={fileInput}
          type="file"
          accept=".wav,audio/wav,audio/x-wav"
          hidden
          onChange={(e) => {
            pickFile(e.target.files?.[0]);
            e.target.value = '';
          }}
        />
        {decodingFile ? (
          <>
            <span className="wefax-file-line">
              <span className="sstv-meter" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={filePct} aria-label={t('wefax.file.progress')}>
                <span style={{ width: `${filePct}%` }} />
              </span>
              <b>{`${filePct} %`}</b>
            </span>
            <button type="button" className="glass sstv-btn sstv-btn--danger" onClick={act(stopWefaxFile)}>
              {t('wefax.file.stop')}
            </button>
            <p className="sstv-hint">{t('wefax.file.running')}</p>
          </>
        ) : (
          <button type="button" className="glass sstv-btn" disabled={sending} title={t('wefax.file.tip')} onClick={() => fileInput.current?.click()}>
            {t('wefax.file.open')}
          </button>
        )}
        {fileError ? (
          <p className="wefax-file-error" role="alert">
            {fileError}
          </p>
        ) : null}
      </div>

      {/* le impostazioni sono visibili ma compatte; con un clic sul titolo si chiudono e la scheda diventa ancora piu' piccola */}
      <details className="wefax-set-details" open>
      <summary>{t('wefax.set.title')}</summary>
      <fieldset className="wefax-set">
        <div className="wefax-grid">
          <label className="sstv-field">
            {t('wefax.set.ioc')}
            <select value={settings.ioc} onChange={(e) => save({ ioc: Number(e.target.value) })}>
              {snap.iocs.map((i) => (
                <option key={i.ioc} value={i.ioc}>
                  {`${i.ioc} (${i.width} px)`}
                </option>
              ))}
            </select>
          </label>
          <label className="sstv-field">
            {t('wefax.set.lpm')}
            <select value={lpm} onChange={(e) => save({ [lpmKey]: Number(e.target.value) } as Partial<WefaxSettings>)}>
              {lpmChoices.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
          </label>
          <label className="sstv-field">
            {t('wefax.set.filter')}
            <select value={settings.filter} onChange={(e) => save({ filter: e.target.value })}>
              {snap.filters.map((f) => (
                <option key={f} value={f}>
                  {f === 'narrow' || f === 'medium' || f === 'wide' ? t(`wefax.set.filter.${f}`) : f}
                </option>
              ))}
            </select>
          </label>
          <NumberField label={t('wefax.set.center')} value={settings.center_hz} min={1000} max={3000} onCommit={(v) => save({ center_hz: v })} />
          <NumberField label={t('wefax.set.shift')} value={settings.shift_hz} min={200} max={1500} onCommit={(v) => save({ shift_hz: v })} />
          <NumberField label={t('wefax.set.afcRange')} value={settings.afc_range_hz ?? 150} min={20} max={400} onCommit={(v) => save({ afc_range_hz: v })} />
        </div>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.manual} onChange={(e) => save({ manual: e.target.checked })} />
          {t('wefax.set.manual')}
        </label>
        <label className="sstv-check" title={t('wefax.set.autoIocTip')}>
          <input type="checkbox" checked={settings.auto_ioc ?? true} onChange={(e) => save({ auto_ioc: e.target.checked })} />
          {t('wefax.set.autoIoc')}
        </label>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.auto_lpm} onChange={(e) => save({ auto_lpm: e.target.checked })} />
          {t('wefax.set.autoLpm')}
        </label>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.auto_center} onChange={(e) => save({ auto_center: e.target.checked })} />
          {t('wefax.set.autoCenter')}
        </label>
        <label className="sstv-check">
          <input type="checkbox" checked={settings.auto_save} onChange={(e) => save({ auto_save: e.target.checked })} />
          {t('wefax.set.autoSave')}
        </label>
        <Recording snap={snap} lpm={lpm} save={save} />
        <label className="sstv-check" title={t('wefax.set.autoSlantTip')}>
          <input type="checkbox" checked={settings.auto_slant ?? true} onChange={(e) => save({ auto_slant: e.target.checked })} />
          {t('wefax.set.autoSlant')}
        </label>
        <div className="sstv-field">
          <span>
            {t('wefax.set.slant')}
            <b>{settings.slant_ppm.toFixed(0)}</b>
          </span>
          <span className="wefax-slant">
            {SLANT_STEPS.map((d) => (
              <button key={d} type="button" className="glass sstv-btn" onClick={() => save({ slant_ppm: Math.round((latest.current.slant_ppm + d) * 10) / 10 })}>
                {d > 0 ? `+${d}` : `${d}`}
              </button>
            ))}
            <button type="button" className="glass sstv-btn" disabled={settings.slant_ppm === 0} onClick={() => save({ slant_ppm: 0 })}>
              {t('wefax.set.slantReset')}
            </button>
          </span>
          <span className="sstv-hint">{t('wefax.set.slantHint')}</span>
          <span title={t('wefax.set.learnedHint')}>
            {t('wefax.set.learned')}
            <b>{Math.round(settings.learned_ppm ?? 0)}</b>
          </span>
          <span className="wefax-slant">
            <button type="button" className="glass sstv-btn" disabled={!settings.learned_ppm} onClick={act(() => wefaxRx('reset_learned'))}>
              {t('wefax.set.slantReset')}
            </button>
          </span>
        </div>
      </fieldset>
      </details>
    </aside>
  );
}
