import '../styles/first-run.css';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getAudioDevices, getEngineSettings, putAudioSettings, putEngineSettings } from '../api/client';
import { useLang, useT } from '../i18n';
import { LANGS, LANG_NAMES, type Lang } from '../i18n/core';
import { browserStorage } from '../lib/theme';
import { coreText } from '../lib/coreText';

const DONE_KEY = 'aurora-first-run-done';
const CALL_RE = /^[A-Z0-9]{1,3}[0-9][A-Z0-9]{0,4}(\/[A-Z0-9]{1,4})?$/;
const GRID_RE = /^[A-R]{2}[0-9]{2}([A-X]{2})?$/i;
const STEPS = 5;

function readDone(): boolean {
  try {
    return browserStorage()?.getItem(DONE_KEY) === '1';
  } catch {
    return false;
  }
}

/**
 * La guida del primo avvio: compare quando manca il nominativo e porta passo passo a una stazione pronta
 * (lingua, nominativo e locatore, scheda audio, radio). Si puo' chiudere quando si vuole: tutto resta modificabile in Impostazioni.
 */
export function FirstRun() {
  const t = useT();
  const qc = useQueryClient();
  const lang = useLang((s) => s.lang);
  const setLang = useLang((s) => s.setLang);
  const [done, setDone] = useState(readDone);
  const [step, setStep] = useState(0);
  const [call, setCall] = useState('');
  const [grid, setGrid] = useState('');
  const [input, setInput] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const engine = useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false, enabled: !done });
  const audio = useQuery({ queryKey: ['audio-devices'], queryFn: getAudioDevices, retry: false, enabled: !done && step === 2 });

  if (done || !engine.data || (engine.data.values.MyCall ?? '').trim() !== '') return null;

  const finish = () => {
    try {
      browserStorage()?.setItem(DONE_KEY, '1');
    } catch {
      // senza archivio la guida torna al prossimo avvio finche' manca il nominativo
    }
    setDone(true);
    void qc.invalidateQueries();
  };

  const callOk = CALL_RE.test(call.trim().toUpperCase());
  const gridOk = GRID_RE.test(grid.trim());

  const saveStation = async () => {
    setBusy(true);
    setError(null);
    try {
      await putEngineSettings({ MyCall: call.trim().toUpperCase(), MyGrid: grid.trim().slice(0, 2).toUpperCase() + grid.trim().slice(2, 4) + grid.trim().slice(4).toLowerCase() });
      setStep(2);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const saveAudio = async () => {
    const data = audio.data;
    if (!data) return setStep(3);
    const change: { input?: string; output?: string } = {};
    if (input !== null && input !== data.values.input) change.input = input;
    if (output !== null && output !== data.values.output) change.output = output;
    if (!change.input && !change.output) return setStep(3);
    setBusy(true);
    setError(null);
    try {
      const r = await putAudioSettings(change);
      const msg = r.input.message ?? r.output.message;
      if (!r.ok && msg) setError(coreText(msg, t));
      else setStep(3);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const inValue = input ?? audio.data?.values.input ?? '';
  const outValue = output ?? audio.data?.values.output ?? '';
  const sysDefault = audio.data?.system_default ?? '';

  return (
    <div className="first-run" role="dialog" aria-modal="true" aria-labelledby="first-run-title">
      <div className="first-run__card">
        <div className="first-run__glow" aria-hidden="true" />
        <header className="first-run__head">
          <span className="first-run__brand">Aurora</span>
          <ol className="first-run__dots" aria-label={t('fr.progress', { n: step + 1, total: STEPS })}>
            {Array.from({ length: STEPS }, (_, i) => (
              <li key={i} className={i < step ? 'is-done' : i === step ? 'is-now' : ''} />
            ))}
          </ol>
          <button type="button" className="first-run__skip" onClick={finish}>
            {t('fr.skip')}
          </button>
        </header>

        <div className="first-run__body" key={step}>
          {step === 0 ? (
            <>
              <h2 id="first-run-title">{t('fr.welcome.title')}</h2>
              <p className="first-run__lead">{t('fr.welcome.text')}</p>
              <div className="first-run__langs" role="radiogroup" aria-label={t('gen.language')}>
                {LANGS.map((l: Lang) => (
                  <button key={l} type="button" role="radio" aria-checked={lang === l} className={lang === l ? 'is-on' : ''} onClick={() => setLang(l)}>
                    {LANG_NAMES[l]}
                  </button>
                ))}
              </div>
            </>
          ) : null}

          {step === 1 ? (
            <>
              <h2 id="first-run-title">{t('fr.station.title')}</h2>
              <p className="first-run__lead">{t('fr.station.text')}</p>
              <div className="first-run__fields">
                <label>
                  <span>{t('gen2.call')}</span>
                  <input
                    className="first-run__big"
                    value={call}
                    autoFocus
                    spellCheck={false}
                    autoComplete="off"
                    placeholder="IZ0ABC"
                    maxLength={12}
                    onChange={(e) => setCall(e.target.value.toUpperCase().replace(/\s/g, ''))}
                    aria-invalid={call !== '' && !callOk}
                  />
                  {call !== '' && !callOk ? <em>{t('fr.station.badCall')}</em> : null}
                </label>
                <label>
                  <span>{t('gen2.grid')}</span>
                  <input
                    className="first-run__big"
                    value={grid}
                    spellCheck={false}
                    autoComplete="off"
                    placeholder="JN00aa"
                    maxLength={6}
                    onChange={(e) => setGrid(e.target.value.replace(/\s/g, ''))}
                    aria-invalid={grid !== '' && !gridOk}
                  />
                  {grid !== '' && !gridOk ? <em>{t('fr.station.badGrid')}</em> : <small>{t('fr.station.gridHint')}</small>}
                </label>
              </div>
            </>
          ) : null}

          {step === 2 ? (
            <>
              <h2 id="first-run-title">{t('fr.audio.title')}</h2>
              <p className="first-run__lead">{t('fr.audio.text')}</p>
              {audio.isLoading ? <p className="first-run__muted">{t('fr.loading')}</p> : null}
              {audio.data ? (
                <div className="first-run__fields">
                  <label>
                    <span>{t('fr.audio.in')}</span>
                    <select value={inValue} onChange={(e) => setInput(e.target.value)}>
                      <option value={sysDefault}>{t('fr.audio.default')}</option>
                      {audio.data.inputs.map((d) => (
                        <option key={d.name} value={d.name}>
                          {d.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    <span>{t('fr.audio.out')}</span>
                    <select value={outValue} onChange={(e) => setOutput(e.target.value)}>
                      <option value={sysDefault}>{t('fr.audio.default')}</option>
                      {audio.data.outputs.map((d) => (
                        <option key={d.name} value={d.name}>
                          {d.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <small>{t('fr.audio.hint')}</small>
                </div>
              ) : null}
            </>
          ) : null}

          {step === 3 ? (
            <>
              <h2 id="first-run-title">{t('fr.radio.title')}</h2>
              <p className="first-run__lead">{t('fr.radio.text')}</p>
              <div className="first-run__choices">
                <a className="first-run__choice" href="#/impostazioni/radio" onClick={finish}>
                  <strong>{t('fr.radio.cat')}</strong>
                  <span>{t('fr.radio.catText')}</span>
                </a>
                <button type="button" className="first-run__choice" onClick={() => setStep(4)}>
                  <strong>{t('fr.radio.vox')}</strong>
                  <span>{t('fr.radio.voxText')}</span>
                </button>
              </div>
            </>
          ) : null}

          {step === 4 ? (
            <>
              <div className="first-run__check" aria-hidden="true">
                <svg viewBox="0 0 24 24" width="40" height="40">
                  <path d="M5 12.5l4.2 4.2L19 7" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </div>
              <h2 id="first-run-title">{callOk ? t('fr.done.title', { call: call.trim().toUpperCase() }) : t('fr.done.titleNoCall')}</h2>
              <p className="first-run__lead">{t('fr.done.text')}</p>
              {!callOk ? <p className="first-run__muted">{t('fr.done.noCall')}</p> : null}
            </>
          ) : null}

          {error ? (
            <p className="first-run__error" role="alert">
              {error}
            </p>
          ) : null}
        </div>

        <footer className="first-run__foot">
          {step > 0 && step < 4 ? (
            <button type="button" className="first-run__back" onClick={() => setStep(step - 1)} disabled={busy}>
              {t('fr.back')}
            </button>
          ) : (
            <span />
          )}
          {step >= 1 && step <= 3 ? (
            <button type="button" className="first-run__later" onClick={() => setStep(step + 1)} disabled={busy}>
              {t('fr.skipStep')}
            </button>
          ) : null}
          {step === 0 ? (
            <button type="button" className="first-run__next" onClick={() => setStep(1)}>
              {t('fr.start')}
            </button>
          ) : null}
          {step === 1 ? (
            <button type="button" className="first-run__next" onClick={saveStation} disabled={!callOk || !gridOk || busy}>
              {t('fr.next')}
            </button>
          ) : null}
          {step === 2 ? (
            <button type="button" className="first-run__next" onClick={saveAudio} disabled={busy}>
              {t('fr.next')}
            </button>
          ) : null}
          {step === 4 ? (
            <button type="button" className="first-run__next" onClick={finish}>
              {t('fr.go')}
            </button>
          ) : null}
        </footer>
      </div>
    </div>
  );
}
